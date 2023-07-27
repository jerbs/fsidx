use fsidx::LocateError;
use rustyline::error::ReadlineError;
use rustyline::Editor;
use signal_hook::iterator::Signals;
use signal_hook::consts::signal::SIGINT;
use std::os::unix::prelude::OsStrExt;
use std::process::Command;
use std::{env::Args, process};
use std::io::{Error, Result as IOResult, stdout, stderr, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
use crate::cli::CliError;
use crate::config::Config;
use crate::expand::{Expand, OpenRule};
use crate::help::{help_shell, help_shell_short};
use crate::locate::locate_shell;
use crate::tokenizer::{Token, tokenize_shell};
use crate::tty::set_tty;
use crate::update::update_shell;
use crate::verbosity::verbosity;


pub(crate) fn shell(config: Config, args: &mut Args) -> Result<(), CliError> {
    if let Some(arg) = args.next() {
        return Err(CliError::InvalidShellArgument(arg));
    } 
    set_tty()
        .map_err(|err: Error| CliError::TtyConfigurationFailed(err))?;
    let interrupt = Arc::new(AtomicBool::new(false));
    let mut signals = Signals::new(&[SIGINT])   // Ctrl-C
        .map_err(CliError::CreatingSignalHandlerFailed)?;
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
    let _ = help_shell_short();
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
                    Err(CliError::LocateError(LocateError::Interrupted)) => {println!("CTRL-C");},
                    Err(CliError::LocateError(LocateError::BrokenPipe))  => {println!("EOF");},
                    Err(err) => { print_error(); eprintln!("{:?}", err);},    // FIXME: Replace debug print
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
    Ok(())
}

fn process_shell_line(config: &Config, line: &str, interrupt: Arc<AtomicBool>, selection: &Option<Vec<PathBuf>>) -> Result<Option<Vec<PathBuf>>, CliError>{
    let token = tokenize_shell(line)?;
    if let Some(Token::Text(command)) = token.first() {
        // Backslash commands:
        if &command[0..1] == "\\" {
            match command.as_str() {
                "\\q" if token.len() == 1 => { process::exit(0); },
                "\\o" => { open_command(config, &token[1..], selection)?; },
                "\\u" if token.len() == 1 => { update_shell(config)?; },
                "\\h" => { let _ = help_shell(); },
                _ => { let _ = help_shell_short(); },
            };
            return Ok(None);
        }
        // Open commands:
        if match command.parse::<OpenRule>() {
            Ok(OpenRule::Index(_)) => true,
            Ok(OpenRule::IndexRange(_, _)) => true,
            Ok(OpenRule::IndexGlob(_, _)) => true,
            _ => false,
        } {
            open_command(config, &token, selection)?;
            return Ok(None);
        }
    }
    // Locate query:
    locate_shell(
        config,
        line,
        Some(interrupt)
    ).map(|v| Some(v))
}

fn open_command(_config: &Config, token: &[Token], selection: &Option<Vec<PathBuf>>) -> Result<(), CliError> {
    if let Some(selection) = selection {
        let mut command = Command::new("open");
        let mut found = false;
        for token in token {
            match token {
                crate::tokenizer::Token::Text(text) => {
                    if let Ok(open_rule) = text.parse::<OpenRule>() {
                        let expand = Expand::new(open_rule, selection);
                        expand.foreach(|path| open_append(&mut command, path, &mut found))?;
                    } else {
                        return Err(CliError::InvalidOpenRule(text.clone()));
                    }
                },
                crate::tokenizer::Token::Option(_) => {},   // TODO: Implement options to configure glob expansion.
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

fn open_append(command: &mut Command, path: &Path, found: &mut bool) -> Result<(), CliError> {
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
        stderr().write_all(b"' not exists. Device not mounted.\n")?;   // FIXME: Improve error.
    }
    Ok(())
}

fn open_spawn(command: &mut Command) -> IOResult<()> {

    let mut child = command.spawn()?;
    let exit_status = child.wait()?;
    if !exit_status.success() {
        print_error();
        eprintln!("Open failed.");
    }
    Ok(())
}

fn print_error() {
    let mut stderr = StandardStream::stderr(ColorChoice::Auto);
    let _ = stderr.set_color(ColorSpec::new().set_fg(Some(Color::Red)));
    let _ = stderr.write_all(b"Error");
    let _ = stderr.set_color(&ColorSpec::new());
    let _ = stderr.write_all(b": ");
}
