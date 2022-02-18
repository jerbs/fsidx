use clap::{self, Arg, ArgMatches};
use fsidx::{FilterToken, Settings, UpdateSink, LocateResult, Metadata};
use rustyline::error::ReadlineError;
use rustyline::Editor;
use signal_hook::iterator::Signals;
use signal_hook::consts::signal::SIGINT;
use std::os::unix::prelude::OsStrExt;
use std::process::Command;
use std::{env, process};
use std::io::{Error, ErrorKind, Result, stdout, stderr, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use crate::config::{Config, find_and_load, get_volume_info, load_from_path};
use crate::expand::{Expand, MatchRule};
use crate::tokenizer::{tokenize, TokenIterator, Token};
use crate::tty::set_tty;
use crate::verbosity::{verbosity, set_verbosity};

fn app_cli() -> clap::Command<'static> {
    clap::Command::new("fsidx")
    .author("Joachim Erbs, joachim.erbs@gmx.de")
    .version(env!("CARGO_PKG_VERSION"))
    .about("Finding file names quickly with a database.")
    .arg(Arg::new("config_file")
        .short('c')
        .long("config")
        .value_name("FILE")
        .help("Set a configuration file")
        .takes_value(true)  )
    .arg(Arg::new("verbosity")
        .short('v')
        .long("verbose")
        .multiple_occurrences(true)
        .help("Set verbosity level") )
    .arg(Arg::new("version_info")
        .long("version")
        .help("Print version info and exit"))
    .subcommand(locate_cli())
    .subcommand(update_cli())
    .subcommand(shell_cli())
}

pub fn main() -> i32 {
    let matches = app_cli().get_matches();

    set_verbosity(matches.occurrences_of("verbosity"));

    if matches.is_present("version_info") {
        const VERSION: &str = env!("CARGO_PKG_VERSION");
        const NAME: &str = env!("CARGO_PKG_NAME");
        let _ = writeln!(stdout().lock(), "{}: Version {}", NAME, VERSION);
    }

    let config: Config = if let Some(config_file) = matches.value_of("config_file") {
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
            let _ = writeln!(stderr().lock(), "Error: {}", err);
            1
        },
    };

    exit_code
}

fn locate_cli() -> clap::Command<'static> {
    clap::Command::new("locate")
    .about("Find matching files in the database")
    // .arg(Arg::new("mt")
    //     .long("mt")
    //     .takes_value(false)
    //     .help("Use multithreaded implementation") )
    .arg(Arg::new("case_sensitive")
        .short('c')
        .multiple_occurrences(true)
        .takes_value(false)
        .display_order(100)
        .help_heading("MATCHING OPTIONS")
        .help("Subsequent [text] arguments are matched case sensitive") )
    .arg(Arg::new("case_insensitive")
        .short('i')
        .multiple_occurrences(true)
        .takes_value(false)
        .display_order(101)
        .help_heading("MATCHING OPTIONS")
        .help("Subsequent [text] arguments are matched case insensitive") )
    .arg(Arg::new("any_order")
        .short('a')
        .multiple_occurrences(true)
        .takes_value(false)
        .display_order(102)
        .help_heading("MATCHING OPTIONS")
        .help("Subsequent [text] arguments may  match in any order") )
    .arg(Arg::new("same_order")
        .short('s')
        .multiple_occurrences(true)
        .takes_value(false)
        .display_order(103)
        .help_heading("MATCHING OPTIONS")
        .help("Subsequent [text] arguments must match in the same order") )
    .arg(Arg::new("whole_path")
        .short('w')
        .multiple_occurrences(true)
        .takes_value(false)
        .display_order(104)
        .help_heading("MATCHING OPTIONS")
        .help("Subsequent [text] arguments may  appear in the whole path") )
    .arg(Arg::new("last_element")
        .short('l')
        .multiple_occurrences(true)
        .takes_value(false)
        .display_order(105)
        .help_heading("MATCHING OPTIONS")
        .help("Subsequent [text] arguments must appear in the last element only") )
    .arg(Arg::new("text")
        // .allow_invalid_utf8(true)
        .multiple_occurrences(true)
        .help("") )
}

fn locate_filter(matches: &ArgMatches) -> Vec<FilterToken> {
    let mut filter: Vec<(FilterToken, usize)> = Vec::new();
    if let Some(indices) = matches.indices_of("case_sensitive") {for idx in indices {filter.push((FilterToken::CaseSensitive, idx)) } };
    if let Some(indices) = matches.indices_of("case_insensitive") {for idx in indices {filter.push((FilterToken::CaseInSensitive, idx)) } };
    if let Some(indices) = matches.indices_of("any_order") {for idx in indices {filter.push((FilterToken::AnyOrder, idx)) } };
    if let Some(indices) = matches.indices_of("same_order") {for idx in indices {filter.push((FilterToken::SameOrder, idx)) } };
    if let Some(indices) = matches.indices_of("whole_path") {for idx in indices {filter.push((FilterToken::WholePath, idx)) } };
    if let Some(indices) = matches.indices_of("last_element") {for idx in indices {filter.push((FilterToken::LastElement, idx)) } };
    if let (Some(indices), Some(texts)) = (matches.indices_of("text"), matches.values_of("text")) {
        for (idx, text) in indices.zip(texts) {
            filter.push((FilterToken::Text(text.to_string()), idx));
        }
    }
    filter.sort_by(|(_,a), (_,b)| a.cmp(b));
    filter.into_iter().map(|(token,_)| token).collect()
}

fn locate_filter_interactive(mut token_it: TokenIterator) -> Vec<FilterToken> {
    let mut filter: Vec<FilterToken> = Vec::new();
    while let Some(token) = token_it.next() {
        if let Some(filter_token) = match token {
            Token::Text(text) => Some(FilterToken::Text(text)),
            Token::Backslash(text) => Some(FilterToken::Text(text)),
            Token::Option(text) => match text.as_str() {
                "case_sensitive"   | "c" => Some(FilterToken::CaseSensitive),
                "case_insensitive" | "i" => Some(FilterToken::CaseInSensitive),
                "any_order"        | "a" => Some(FilterToken::AnyOrder),
                "same_order"       | "s" => Some(FilterToken::SameOrder),
                "whole_path"       | "w" => Some(FilterToken::WholePath),
                "last_element"     | "l" => Some(FilterToken::LastElement),
                _  => {eprintln!("Error: Invalid option `{}`", text); None},
            },
         } {
            filter.push(filter_token);
        }
    }
    filter
}

fn print_locate_result(res: &LocateResult) -> Result<()> {
    match *res {
        LocateResult::Entry(path, Metadata { size: Some(size) } ) => {
            stdout().write_all(path.as_os_str().as_bytes())?;
            stdout().write_fmt(format_args!(" ({})", size))?;
            stdout().write_all(b"\n")?;
        },
        LocateResult::Entry(path, Metadata { size:None } ) => {
            stdout().write_all(path.as_os_str().as_bytes())?;
            stdout().write_all(b"\n")?;
        },
        LocateResult::Finished => {},
        LocateResult::Interrupted => {
            stdout().write(b"CTRL-C\n")?;
        },
        LocateResult::Searching(path) => {
            if verbosity() {
                stdout().write_all(b"Searching: ")?;
                stdout().write_all(path.as_os_str().as_bytes())?;
                stdout().write_all(b"\n")?;
            }
        },
        LocateResult::SearchingFinished(path) => {
            if verbosity() {
                stdout().write_all(b"Searching  ")?;
                stdout().write_all(path.as_os_str().as_bytes())?;
                stdout().write_all(b" finished\n")?;
            }
        },
        LocateResult::SearchingFailed(path, error) => {
            stdout().write_all(b"Searching ")?;
            stdout().write_all(path.as_os_str().as_bytes())?;
            stdout().write_fmt(format_args!(" failed: {}\n", error))?;
        },
    }
    Ok(())
}

fn locate(config: &Config, matches: &ArgMatches, interrupt: Option<Arc<AtomicBool>>) -> Result<i32> {
    let filter_token = locate_filter(matches);
    locate_impl(config, filter_token, interrupt, |res| {
        print_locate_result(&res)
    })?;
    Ok(0)
}

fn locate_interactive(config: &Config, token_it: TokenIterator, interrupt: Option<Arc<AtomicBool>>) -> Result<Vec<PathBuf>> {
    let mut selection = Vec::new();
    let filter_token = locate_filter_interactive(token_it);
    locate_impl(config, filter_token, interrupt, |res| {
        if let LocateResult::Entry(path, _) = res {
            let pb = path.to_path_buf();
            selection.push(pb);
            let index = selection.len();
            stdout().write_fmt(format_args!("{}. ", index))?;
        }
        print_locate_result(&res)
    })?;
    Ok(selection)
}

fn locate_impl<F: FnMut(LocateResult)->Result<()>>(config: &Config, filter_token: Vec<FilterToken>, interrupt: Option<Arc<AtomicBool>>, f: F) -> Result<()> {
    let volume_info = get_volume_info(&config)
    .ok_or(Error::new(ErrorKind::Other, "No database path set"))?;
    fsidx::locate(volume_info, filter_token, interrupt, f)
}

fn shell_cli() -> clap::Command<'static> {
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
                eprintln!("Error: Reading '{}' failed: {}", history.display(), err);
            }
        }
        Some(history)
    } else {
        None
    };
    short_help();
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
                            _ => {println!("Error: {}", err);},
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
                println!("Error: {}", err);
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
            "h" => {help();},
            _ => {short_help();},
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
                                eprintln!("Error: Invalid index '{}'.", index);
                            }
                        } else {
                            println!("Error: Invalid index '{}'.", index);
                        }
                    } else {
                        eprintln!("Error: Invalid index '{}'.", text);
                    }
                },
                crate::tokenizer::Token::Backslash(text) => {
                    eprintln!("Error: No backslash command '\\{}' expected.", text);
                },
                crate::tokenizer::Token::Option(text) => {
                    eprintln!("Error: Invalid option '-{}'.", text);
                },
            }
        }
        if found {
            open_spawn(&mut command)?;
        }
    } else {
        eprintln!("Error: Run a query first.");
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
        eprintln!("Error: Run a query first.");
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
        stderr().write(b"Error: '")?;
        stderr().write(path.as_os_str().as_bytes())?;
        stderr().write(b"' not exists. Device not mounted.\n")?;
    }
    Ok(())
}

fn open_spawn(command: &mut Command) -> Result<()> {

    let mut child = command.spawn()?;
    let exit_status = child.wait()?;
    if !exit_status.success() {
        eprintln!("Error: Open failed.")
    }
    Ok(())
}

fn short_help() {
    println!("Ctrl-C: Interrupt printing results");
    println!("Ctrl-D: Terminate application");
    println!("\\h             -- print detailed help")
}

fn help() {
    short_help();
    println!("\\q             -- quit application");
    println!("\\o [id ...]    -- open files with id from last selection");
    println!("\\u             -- update database");
    println!("Open search results with index commands:");
    println!("  12.           -- Open single selected file");
    println!("  12..          -- Open all selected files in same directory");
    println!("  12...         -- Open all files in same directory");
    println!("  12..jpg       -- Open all selected files in same directory with suffix");
    println!("  12...jpg      -- Open all files in same directory with suffix");
    println!("Quoting:");
    println!("  \"some text\"   -- Search text with space");
}

fn update_cli() -> clap::Command<'static> {
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
