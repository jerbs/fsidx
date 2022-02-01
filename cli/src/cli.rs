use clap::{App, Arg, ArgMatches};
use fsidx::{FilterToken, Settings, UpdateSink, LocateSink, SelectionInsert};
use rustyline::error::ReadlineError;
use rustyline::Editor;
use signal_hook::iterator::Signals;
use signal_hook::consts::signal::SIGINT;
use std::os::unix::prelude::OsStrExt;
use std::process::Command;
use std::{env, process};
use std::io::{Error, ErrorKind, Result, stdout, stderr, Write};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use crate::config::{Config, find_and_load, get_volume_info, load_from_path};
use crate::selection::{Selection, NoSelection};
use crate::tokenizer::{tokenize, TokenIterator};
use crate::verbosity::{verbosity, set_verbosity};

fn app_cli() -> App<'static> {
    App::new("fsidx")
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
        Some(("update", sub_matches)) => update(config, sub_matches),
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

fn locate_cli() -> App<'static> {
    App::new("locate")
    .about("Find matching files in the database")
    .arg(Arg::new("mt")
        .long("mt")
        .takes_value(false)
        .help("Use multithreaded implementation") ) 
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

fn locate(config: &Config, matches: &ArgMatches, interrupt: Option<Arc<AtomicBool>>) -> Result<i32> {
    let mut selection = NoSelection::new();
    locate_impl(config, matches, interrupt, &mut selection)?;
    Ok(0)
}

fn locate_interactive(config: &Config, matches: &ArgMatches, interrupt: Option<Arc<AtomicBool>>) -> Result<Selection> {
    let mut selection = Selection::new();
    locate_impl(config, matches, interrupt, &mut selection)?;
    Ok(selection)
}

fn locate_impl(config: &Config, matches: &ArgMatches, interrupt: Option<Arc<AtomicBool>>, selection: &mut dyn SelectionInsert) -> Result<()> {
    let filter_token = locate_filter(matches);
    let mt = matches.is_present("mt");
    let volume_info = get_volume_info(&config)
    .ok_or(Error::new(ErrorKind::Other, "No database path set"))?;
    let sink = LocateSink {
        verbosity: verbosity(),
        stdout: &mut stdout(),
        stderr: &mut stderr(),
        selection,
    };
    if mt {
        fsidx::locate_mt(volume_info, filter_token, sink, interrupt);
    } else {
        fsidx::locate(volume_info, filter_token, sink, interrupt);
    }
    Ok(())
}

fn shell_cli() -> App<'static> {
    App::new("shell")
    .about("Open the fsidx shell to enter locate queries")
}

fn shell(config: Config, matches: &ArgMatches, _sub_matches: &ArgMatches) -> Result<i32> {
    let interrupt = Arc::new(AtomicBool::new(false));
    let mut signals = Signals::new(&[SIGINT])?;   // Ctrl-C
    let interrupt_for_signal_handler = interrupt.clone();
    std::thread::spawn(move || {
        let interrupt = interrupt_for_signal_handler;
        for sig in signals.forever() {
            if verbosity() {
                println!("Received signal {:?}", sig);
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
    println!("Ctrl-C: Interrupt printing results");
    println!("Ctrl-D: Terminate application");
    let mut selection: Option<Selection> = None;
    loop {
        let readline = rl.readline("> ");
        match readline {
            Ok(line) => {
                rl.add_history_entry(line.as_str());
                interrupt.store(false, Ordering::Relaxed);
                if let Ok(Some(s)) = process_shell_line(&config, matches, &line, interrupt.clone(), &selection) {
                    selection = Some(s);
                }
            },
            Err(ReadlineError::Interrupted) => {
                println!("CTRL-C");
            },
            Err(ReadlineError::Eof) => {
                println!("CTRL-D");
                break
            },
            Err(err) => {
                println!("Error: {:?}", err);
                break
            }
        }
    }
    if let Some(history) = history {
        rl.save_history(&history).unwrap();
    }
    Ok(0)
}

fn process_shell_line(config: &Config, _matches: &ArgMatches, line: &str, interrupt: Arc<AtomicBool>, selection: &Option<Selection>) -> Result<Option<Selection>>{
    let backslash_commmand = starts_with_backslash(line);
    let token = tokenize(line);
    if backslash_commmand {
        let mut  it = token.into_iter();
        match it.next().as_deref() {
            Some("q") if it.next().is_none() => {process::exit(0);},
            Some("o") => {open(it, selection)?;},
            _ => {help();},
        }
    } else {
        let matches = match locate_cli().setting(clap::AppSettings::NoBinaryName).try_get_matches_from(token) {
            Ok(matches) => matches,
            Err(error) => { eprintln!("Error: {}", error); return Ok(None);},
        };
        return locate_interactive(config, &matches, Some(interrupt)).map(|v| Some(v));
    }
    Ok(None)
}

fn open(token_it: TokenIterator, selection: &Option<Selection>) -> Result<()> {
    if let Some(selection) = selection {
        let mut command = Command::new("open");
        let mut found_files = false;
        for token in token_it {
            if let Ok(index) = token.parse::<usize>() {
                if index > 0 {
                    let index = index - 1;
                    if let Some(path) = selection.get_path(index) {
                        let path = Path::new(path);
                        if path.exists() {
                            command.arg(path);
                            found_files = true;
                            let _ = stdout().write(b"Opening: '");
                            let _ = stdout().write(path.as_os_str().as_bytes());
                            let _ = stdout().write(b"'\n");
                        }
                        else {
                            let _ = stderr().write(b"Error: '");
                            let _ = stderr().write(path.as_os_str().as_bytes());
                            let _ = stderr().write(b"' not exists. Device not mounted.\n");
                        }
                    } else {
                        eprintln!("Error: Invalid index '{}'.", index);    
                    }
                } else {
                    println!("Error: Invalid index '{}'.", index);
                }
            } else {
                eprintln!("Error: Invalid index '{}'.", token);
            }
        }
        if found_files {
            command.spawn()?;
        }
    } else {
        eprintln!("Error: Run a query first.");
    }
    Ok(())
}

fn help() {
    println!("\\q             -- quit application");
    println!("\\o [id ...]    -- open files with id from last selection")
}

fn starts_with_backslash(line: &str) -> bool {
    for ch in line.chars() {
        match ch {
            ' ' | '\t' | '\n' | '\r' => (),
            '\\' => {return true;},
            _ => {return false;},
        }
    }
    false
}

fn update_cli() -> App<'static> {
    App::new("update")
    .about("Rescan folders and update the database")
}

fn update(config: Config, _matches: &ArgMatches) -> Result<i32> {
    let volume_info = get_volume_info(&config)
    .ok_or(Error::new(ErrorKind::Other, "No database path set"))?;
    let sink = UpdateSink {
        stdout: &mut stdout(),
        stderr: &mut stderr(),
    };
    fsidx::update(volume_info, Settings::WithFileSizes, sink);
    Ok(0)
}
