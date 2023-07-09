use clap::{self, Arg, ArgMatches};
use fsidx::{FilterToken, Settings, UpdateSink, LocateResult, Metadata};
use indoc::indoc;
use rustyline::error::ReadlineError;
use rustyline::Editor;
use signal_hook::iterator::Signals;
use signal_hook::consts::signal::SIGINT;
use std::collections::VecDeque;
use std::os::unix::prelude::OsStrExt;
use std::process::Command;
use std::{env, process};
use std::io::{Error, ErrorKind, Result, stdout, stderr, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
use crate::config::{Config, find_and_load, get_volume_info, load_from_path};
use crate::expand::{Expand, MatchRule};
use crate::tokenizer::{tokenize, TokenIterator, Token};
use crate::tty::set_tty;
use crate::verbosity::{verbosity, set_verbosity};

fn app_cli() -> clap::Command {
    clap::Command::new("fsidx")
    .author("Joachim Erbs, joachim.erbs@gmx.de")
    .version(env!("CARGO_PKG_VERSION"))
    .about("Finding file names quickly with a database.")
    .arg(Arg::new("config_file")
        .short('c')
        .long("config")
        .value_name("FILE")
        .help("Set a configuration file")
        .num_args(1)  )
    .arg(Arg::new("verbosity")
        .short('v')
        .long("verbose")
        .action(clap::ArgAction::Count)
        .help("Set verbosity level") )
    .subcommand(locate_cli())
    .subcommand(update_cli())
    .subcommand(shell_cli())
}

pub fn main() -> i32 {
    let matches = app_cli().get_matches();

    set_verbosity(matches.get_count("verbosity"));

    let config: Config = if let Some(config_file) = matches.get_one::<String>("config_file") {
        if verbosity() {
            let _ = writeln!(stdout().lock(), "Config File: {}", config_file);
        }
        match load_from_path(Path::new(config_file)) {
            Ok(config) => config,
            Err(msg) => {let _ = writeln!(stderr().lock(), "{}", msg); return 1},
        }
    } else {
        match find_and_load() {
            Ok(config) => config,
            Err(msg) => {let _ = writeln!(stderr().lock(), "{}", msg); return 1},
        }
    };

    let result = match matches.subcommand() {
        Some(("shell", sub_matches)) => shell(config, &matches, sub_matches),
        Some(("locate", sub_matches)) => locate(&config, sub_matches, None),
        Some(("update", _sub_matches)) => update(&config),
        _ => {
            app_cli().print_help().ok();
            let _ = writeln!(stdout().lock(), "\n");
            Err(Error::new(ErrorKind::Other, "Invalid command"))
        },
    };

    let exit_code = match result {
        Ok(exit_code) => exit_code,
        Err(err) => {
            print_error();
            eprintln!("{}", err);
            1
        },
    };

    exit_code
}

fn locate_cli() -> clap::Command {
    clap::Command::new("locate")
    .about("Find matching files in the database")
    .arg(Arg::new("filter")
        .trailing_var_arg(true)
        .allow_hyphen_values(true)
        .num_args(0..)
        .display_order(100)
        .help_heading("Filter")
        .help(indoc! {"
            Any filter flag, text or glob in any order.
            Use the \\h command available in the fsidx shell
            for more details.
            "})
    )
}

struct TokenVec {
    token: VecDeque<Token>,
}

struct TokenIter {
    remainder: VecDeque<Token>,
}

impl IntoIterator for TokenVec {
    type Item = Token;
    type IntoIter = TokenIter;

    fn into_iter(self) -> Self::IntoIter {
        TokenIter {
            remainder: self.token,
        }
    }
}

impl Iterator for TokenIter {
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        self.remainder.pop_front()
    }
}

fn locate_filter(matches: &ArgMatches) -> Result<Vec<FilterToken>> {
    let mut token = VecDeque::new();
    if let Some(values) = matches.get_raw("filter") {
        for value in values {
            let text = value.to_string_lossy();
            if text.starts_with("-") {
                token.push_back(Token::Option(String::from(&text[1..])));
            } else {
                token.push_back(Token::Text(String::from(text)));
            }
        }
    }
    let token_vec = TokenVec {
        token
    };
    locate_filter_interactive(&mut token_vec.into_iter())
}

fn locate_filter_interactive(token_it: &mut dyn Iterator<Item = Token>) -> Result<Vec<FilterToken>> {
    let mut filter: Vec<FilterToken> = Vec::new();
    while let Some(token) = token_it.next() {
        let filter_token= match token {
            Token::Text(text) => FilterToken::Text(text),
            Token::Backslash(text) => FilterToken::Text(text),
            Token::Option(text) => match text.as_str() {
                "case_sensitive"   | "c" => FilterToken::CaseSensitive,
                "case_insensitive" | "i" => FilterToken::CaseInSensitive,
                "any_order"        | "a" => FilterToken::AnyOrder,
                "same_order"       | "s" => FilterToken::SameOrder,
                "whole_path"       | "w" => FilterToken::WholePath,
                "last_element"     | "l" => FilterToken::LastElement,
                "require_literal_separator"   | "ls" | "ls1" => FilterToken::RequireLiteralSeparator(true),
                "unrequire_literal_separator"        | "ls0" => FilterToken::RequireLiteralSeparator(false),
                "require_literal_leading_dot" | "ld" | "ld1" => FilterToken::RequireLiteralLeadingDot(true),
                "unrequire_literal_leading_dot"      | "ld0" => FilterToken::RequireLiteralLeadingDot(false),
                "auto"  | "m0" => FilterToken::Auto,
                "smart" | "m1" => FilterToken::Smart,
                "glob"  | "m2" => FilterToken::Glob,
                _  => {
                    let msg = format!("Invalid option: -{}", text);
                    return Err(Error::new(ErrorKind::InvalidInput, msg));
                },
            },
        };
        filter.push(filter_token);
    }
    Ok(filter)
}

fn print_size(stdout: &mut StandardStream, size: u64) -> Result<()> {
    let text = size.to_string();
    let bytes = text.bytes();
    let len = bytes.len();
    for (i, ch) in bytes.into_iter().enumerate() {
        if i > 0 {
            match (len - i) % 3 {
                0 => {stdout.write_all(b".")?;}
                _ => {}
            }
        }
        stdout.write(&[ch])?;
    }
    Ok(())
}

fn print_locate_result(stdout: &mut StandardStream, res: &LocateResult) -> Result<()> {
    match *res {
        LocateResult::Entry(path, Metadata { size: Some(size) } ) => {
            stdout.write_all(path.as_os_str().as_bytes())?;
            stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)))?;
            stdout.write_all(b" (")?;
            print_size(stdout, *size)?;
            stdout.write_all(b")")?;
            stdout.set_color(&ColorSpec::new())?;
            stdout.write_all(b"\n")?;
        },
        LocateResult::Entry(path, Metadata { size:None } ) => {
            stdout.write_all(path.as_os_str().as_bytes())?;
            stdout.write_all(b"\n")?;
        },
        LocateResult::Finished => {},
        LocateResult::Interrupted => {
            stdout.write(b"CTRL-C\n")?;
        },
        LocateResult::Searching(path) => {
            if verbosity() {
                stdout.write_all(b"Searching: ")?;
                stdout.write_all(path.as_os_str().as_bytes())?;
                stdout.write_all(b"\n")?;
            }
        },
        LocateResult::SearchingFinished(path) => {
            if verbosity() {
                stdout.write_all(b"Searching  ")?;
                stdout.write_all(path.as_os_str().as_bytes())?;
                stdout.write_all(b" finished\n")?;
            }
        },
        LocateResult::SearchingFailed(path, error) => {
            stdout.write_all(b"Searching ")?;
            stdout.write_all(path.as_os_str().as_bytes())?;
            stdout.write_fmt(format_args!(" failed: {}\n", error))?;
        },
    }
    Ok(())
}

fn locate(config: &Config, matches: &ArgMatches, interrupt: Option<Arc<AtomicBool>>) -> Result<i32> {
    let mut stdout = StandardStream::stdout(ColorChoice::Auto);
    let filter_token = locate_filter(matches)?;
    locate_impl(config, filter_token, interrupt, |res| {
        print_locate_result(&mut stdout, &res)
    })?;
    Ok(0)
}

fn locate_interactive(config: &Config, mut token_it: TokenIterator, interrupt: Option<Arc<AtomicBool>>) -> Result<Vec<PathBuf>> {
    let mut stdout = StandardStream::stdout(ColorChoice::Auto);
    let mut selection = Vec::new();
    let filter_token = locate_filter_interactive(&mut token_it)?;
    locate_impl(config, filter_token, interrupt, |res| {
        if let LocateResult::Entry(path, _) = res {
            let pb = path.to_path_buf();
            selection.push(pb);
            let index = selection.len();
            stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)))?;
            stdout.write_fmt(format_args!("{}. ", index))?;
            stdout.set_color(&ColorSpec::new())?;
        }
        print_locate_result(&mut stdout, &res)
    })?;    
    Ok(selection)
}

fn locate_impl<F: FnMut(LocateResult)->Result<()>>(config: &Config, filter_token: Vec<FilterToken>, interrupt: Option<Arc<AtomicBool>>, f: F) -> Result<()> {
    let volume_info = get_volume_info(&config)
    .ok_or(Error::new(ErrorKind::Other, "No database path set"))?;
    fsidx::locate(volume_info, filter_token, interrupt, f)
}

fn shell_cli() -> clap::Command {
    clap::Command::new("shell")
    .about("Open the fsidx shell to enter locate queries")
}

fn shell(config: Config, _matches: &ArgMatches, _sub_matches: &ArgMatches) -> Result<i32> {
    crate::cli::set_tty()?;
    let interrupt = Arc::new(AtomicBool::new(false));
    let mut signals = Signals::new(&[SIGINT])?;   // Ctrl-C
    let interrupt_for_signal_handler = interrupt.clone();
    std::thread::spawn(move || {
        let interrupt = interrupt_for_signal_handler;
        for sig in signals.forever() {
            if verbosity() {
                println!("Received signal {}", sig);
            }
            if sig == SIGINT {
                interrupt.store(true, Ordering::Relaxed);
            }
        }
    });
    let mut rl = Editor::<()>::new();
    let history = if let Some(db_path) = &config.db_path {
        let history = Path::new(&db_path).join("history.txt");
        if let Err(err) = rl.load_history(&history) {
            if matches!(err, ReadlineError::Errno(nix::errno::Errno::ENOENT)) {
                print_error();
                eprintln!("Reading '{}' failed: {}", history.display(), err);
            }
        }
        Some(history)
    } else {
        None
    };
    let _ = short_help();
    let mut selection: Option<Vec<PathBuf>> = None;
    loop {
        let readline = rl.readline("> ");
        match readline {
            Ok(line) => {
                rl.add_history_entry(line.as_str());
                interrupt.store(false, Ordering::Relaxed);
                match process_shell_line(&config, &line, interrupt.clone(), &selection) {
                    Ok(Some(s)) => {selection = Some(s);},
                    Ok(None) => {},
                    Err(err) => {
                        match err.kind() {
                            ErrorKind::Interrupted => {println!("CTRL-C");},
                            ErrorKind::BrokenPipe => {println!("EOF");},
                            _ => {print_error(); eprintln!("{}", err);},
                        }
                    },
                };
            },
            Err(ReadlineError::Interrupted) => {
                println!("CTRL-C");
            },
            Err(ReadlineError::Eof) => {
                println!("CTRL-D");
                break
            },
            Err(err) => {
                print_error();
                eprintln!("{}", err);
                break
            }
        }
    }
    if let Some(history) = history {
        rl.save_history(&history).unwrap();
    }
    Ok(0)
}

fn process_shell_line(config: &Config, line: &str, interrupt: Arc<AtomicBool>, selection: &Option<Vec<PathBuf>>) -> Result<Option<Vec<PathBuf>>>{
    let mut token_it = tokenize(line).into_iter();
    if let Some(Token::Backslash(command)) = token_it.next() {
        match command.as_str() {
            "q" if token_it.next().is_none() => {process::exit(0);},
            "o" => {open_backslash_command(token_it, selection)?;},
            "u" if token_it.next().is_none() => {update(config)?;},
            "h" => {let _ = help();},
            _ => {let _ = short_help();},
        };
        return Ok(None);
    }
    let mut token_it = tokenize(line).into_iter();
    if let Some(Token::Text(first)) = token_it.next() {
        if let Ok(_) = first.parse::<MatchRule>() {
            let token_it = tokenize(line).into_iter();
            open_index_command(config, token_it, selection)?;
            return Ok(None);
        }
    }
    if tokenize(line).into_iter().next().is_some() {
        return locate_interactive(
            config,
            tokenize(line).into_iter(),
            Some(interrupt)).map(|v| Some(v));    
    } else {
        return Ok(None);
    }
}

fn open_backslash_command(token_it: TokenIterator, selection: &Option<Vec<PathBuf>>) -> Result<()> {
    if let Some(selection) = selection {
        let mut command = Command::new("open");
        let mut found = false;
        for token in token_it {
            match token {
                crate::tokenizer::Token::Text(text) => {
                    if let Ok(index) = text.parse::<usize>() {
                        if index > 0 {
                            let index = index - 1;
                            if let Some(path) = selection.get(index) {
                                let path = Path::new(path);
                                open_append(&mut command, path, &mut found)?;
                            } else {
                                print_error();
                                eprintln!("Invalid index '{}'.", index);
                            }
                        } else {
                            print_error();
                            println!("Invalid index '{}'.", index);
                        }
                    } else {
                        print_error();
                        eprintln!("Invalid index '{}'.", text);
                    }
                },
                crate::tokenizer::Token::Backslash(text) => {
                    print_error();
                    eprintln!("No backslash command '\\{}' expected.", text);
                },
                crate::tokenizer::Token::Option(text) => {
                    print_error();
                    eprintln!("Invalid option '-{}'.", text);
                },
            }
        }
        if found {
            open_spawn(&mut command)?;
        }
    } else {
        print_error();
        eprintln!("Run a query first.");
    }
    Ok(())
}

fn open_index_command(config: &Config, token_it: TokenIterator, selection: &Option<Vec<PathBuf>>) -> Result<()> {
    if let Some(selection) = selection {
        let mut command = Command::new("open");
        let mut found = false;
        for token in token_it {
            match token {
                crate::tokenizer::Token::Text(text) => {
                    if let Ok(match_rule) = text.parse::<MatchRule>() {
                        let expand = Expand::new(config, match_rule, selection);
                        expand.foreach(|path| open_append(&mut command, path, &mut found))?;
                    }
                },
                crate::tokenizer::Token::Backslash(_) => {},
                crate::tokenizer::Token::Option(_) => {},
            };
        }
        if found {
            open_spawn(&mut command)?;
        }
    } else {
        print_error();
        eprintln!("Run a query first.");
    }
    Ok(())
}

fn open_append(command: &mut Command, path: &Path, found: &mut bool) -> Result<()> {
    if path.exists() {
        command.arg(path);
        *found = true;
        stdout().write(b"Opening: '")?;
        stdout().write(path.as_os_str().as_bytes())?;
        stdout().write(b"'\n")?;
    }
    else {
        print_error();
        stderr().write_all(b"'")?;
        stderr().write_all(path.as_os_str().as_bytes())?;
        stderr().write_all(b"' not exists. Device not mounted.\n")?;
    }
    Ok(())
}

fn open_spawn(command: &mut Command) -> Result<()> {

    let mut child = command.spawn()?;
    let exit_status = child.wait()?;
    if !exit_status.success() {
        print_error();
        eprintln!("Open failed.");
    }
    Ok(())
}

fn short_help() -> Result<()> {
    let indent = 20;
    let mut stdout = StandardStream::stdout(ColorChoice::Auto);
    write_flags(&mut stdout, &[r#"Ctrl-C"#], indent, "Interrupt printing results")?;
    write_flags(&mut stdout, &[r#"Ctrl-D"#], indent, "Terminate application")?;
    write_section(&mut stdout, "Commands:")?;
    write_flags(&mut stdout, &[r#"\h"#], indent, "print detailed help")?;
    Ok(())
}

fn help() -> Result<()>{
    short_help()?;
    let indent = 20;
    let mut stdout = StandardStream::stdout(ColorChoice::Auto);
    write_flags(&mut stdout, &[r#"\q"#], indent, "quit application")?;
    write_flags(&mut stdout, &[r#"\o [id ...]"#], indent, "open files with id from last selection")?;
    write_flags(&mut stdout, &[r#"\u"#], indent, "update database")?;

    write_section(&mut stdout, "Modes:")?;
    write_flags(&mut stdout, &["-m0", "-auto"], indent, "Auto detect mode (default)")?;
    write_flags(&mut stdout, &["-m1", "-smart"], indent, "Enter smart searching mode")?;
    write_flags(&mut stdout, &["-m2", "-glob"], indent, "Enter glob pattern mode")?;

    write_section(&mut stdout, "Smart searching (-m1):")?;
    write_flags(&mut stdout, &[r#"text"#], indent, "Search for any path containing 'text'")?;
    write_flags(&mut stdout, &[r#"foo bar"#], indent, "Path must contains all strings in any order")?;
    write_flags(&mut stdout, &[r#""foo bar""#], indent, "Match text with spaces")?;
    write_flags(&mut stdout, &[r#""ab cb""#], indent, r#"Match "ab cd", "ab-cd", "ab_cd" and "abcd""#)?;
    write_flags(&mut stdout, &[r#"ab\ cb"#], indent, r#"Match "ab cd", "ab-cd", "ab_cd" and "abcd""#)?;
    write_flags(&mut stdout, &[r#"ab-cb"#], indent, r#"Match "ab cd", "ab-cd", "ab_cd" and "abcd""#)?;
    write_flags(&mut stdout, &[r#"ab_cb"#], indent, r#"Match "ab cd", "ab-cd", "ab_cd" and "abcd""#)?;
    write_flags(&mut stdout, &[r#""\"""#, r#"""\""#], indent, "Match double quote")?;
    write_flags(&mut stdout, &[r#""-""#, r#"""-"#], indent, "Match dash")?;
    write_flags(&mut stdout, &[r#""\\""#, r#"""\\"#], indent, "Match backslash")?;

    write_section(&mut stdout, "Glob pattern searching (-m2):")?;
    write_flags(&mut stdout, &[r#"*.???"#], indent, "? matches any single character.")?;
    write_flags(&mut stdout, &[r#"*.jpg"#], indent, "* matches any sequence of characters")?;
    write_flags(&mut stdout, &[r#"/**/*.jpg"#], indent, "** matches any subdirectory")?;
    write_flags(&mut stdout, &[r#"[abc]"#], indent, "[...] matches one of the characters inside the brackets")?;
    write_flags(&mut stdout, &[r#"[a-zA-Z]"#], indent, "[.-.] matches one of the characters in the sequence")?;
    write_flags(&mut stdout, &[r#"[!abc]"#], indent, "negation of [...]")?;
    write_flags(&mut stdout, &[r#"[?]"#], indent, "matches a ?")?;
    write_flags(&mut stdout, &[r#"[*]"#], indent, "matches a *")?;
    write_flags(&mut stdout, &[r#"[[]]"#], indent, "matches a [")?;
    write_flags(&mut stdout, &[r#"[]]"#], indent, "matches a ]")?;
    write_flags(&mut stdout, &[r#"-ls, -ls1, -require_literal_separator"#], indent + 25, "* does not match /")?;
    write_flags(&mut stdout, &[r#"-ls0, -unrequire_literal_separator"#], indent + 25, "* does match /")?;
    write_flags(&mut stdout, &[r#"-ld, -ld1, -require_literal_leading_dot"#], indent + 25, "* does not match a leading dot")?;
    write_flags(&mut stdout, &[r#"-ld0, -unrequire_literal_leading_dot"#], indent + 25, "* does match a leading dot")?;

    let indent = 30;
    write_section(&mut stdout, "Search options:")?;
    write_flags(&mut stdout, &["-c", "-case_sensitive"], indent, "Subsequent [text] arguments are matched case sensitive")?;
    write_flags(&mut stdout, &["-i", "-case_insensitive"], indent, "Subsequent [text] arguments are matched case insensitive")?;
    write_flags(&mut stdout, &["-a", "-any_order"], indent, "Subsequent [text] arguments may  match in any order")?;
    write_flags(&mut stdout, &["-s", "-same_order"], indent, "Subsequent [text] arguments must match in the same order")?;
    write_flags(&mut stdout, &["-w", "-whole_path"], indent, "Subsequent [text] arguments may  appear in the whole path")?;
    write_flags(&mut stdout, &["-l", "-last_element"], indent, "Subsequent [text] arguments must appear in the last element only")?;
    write_flags(&mut stdout, &["-", ""], indent, "Not an option. This matches a dash.")?;

    let indent = 20;
    write_section(&mut stdout, "Open search results with index commands:")?;
    write_flags(&mut stdout, &[r#"12."#], indent, "Open single selected file")?;
    write_flags(&mut stdout, &[r#"12.."#], indent, "Open all selected files in same directory")?;
    write_flags(&mut stdout, &[r#"12..."#], indent, "Open all files in same directory")?;
    write_flags(&mut stdout, &[r#"12.."#], indent, "Open all selected files in same directory with suffix")?;
    write_flags(&mut stdout, &[r#"12...jpg"#], indent, "Open all files in same directory with suffix")?;
    Ok(())
}

fn write_section(stdout: &mut StandardStream, text: &str) -> Result<()> {
    stdout.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)))?;
    writeln!(stdout, "\n{}", text)?;
    stdout.set_color(&ColorSpec::new())?;
    Ok(())
}

fn write_flags(stdout: &mut StandardStream, flags: &[&str], indent: usize, description: &str) -> Result<()> {
    let mut pos = 4;
    write!(stdout, "    ")?;
    for (index, flag) in flags.iter().enumerate() {
        stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)))?;
        write!(stdout, "{}", flag)?;
        pos = pos + flag.chars().count();
        stdout.set_color(&ColorSpec::new())?;
        if index + 1 != flags.len() {
            write!(stdout, ", ")?;
            pos = pos + 2;
        }
    }
    while pos < indent {write!(stdout, " ")?; pos = pos + 1;}
    writeln!(stdout, "{}", description)?;
    Ok(())
}

fn print_error() {
    let mut stderr = StandardStream::stderr(ColorChoice::Auto);
    let _ = stderr.set_color(ColorSpec::new().set_fg(Some(Color::Red)));
    let _ = stderr.write_all(b"Error");
    let _ = stderr.set_color(&ColorSpec::new());
    let _ = stderr.write_all(b": ");
}

fn update_cli() -> clap::Command {
    clap::Command::new("update")
    .about("Rescan folders and update the database")
}

fn update(config: &Config) -> Result<i32> {
    let volume_info = get_volume_info(&config)
    .ok_or(Error::new(ErrorKind::Other, "No database path set"))?;
    let sink = UpdateSink {
        stdout: &mut stdout(),
        stderr: &mut stderr(),
    };
    fsidx::update(volume_info, Settings::WithFileSizes, sink);
    Ok(0)
}
