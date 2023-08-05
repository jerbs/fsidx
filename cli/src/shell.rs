use fsidx::LocateError;
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;
use signal_hook::iterator::Signals;
use signal_hook::consts::signal::SIGINT;
use std::os::unix::prelude::OsStrExt;
use std::process::Command;
use std::env::Args;
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

impl From<ReadlineError> for CliError {
    fn from(err: ReadlineError) -> Self {
        let description = err.to_string();
        CliError::ReadlineError(description)
    }
}

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
    let mut rl = DefaultEditor::new()?;
    let history = if let Some(db_path) = &config.index.db_path {
        let history = Path::new(&db_path).join("history.txt");
        if let Err(err) = rl.load_history(&history) {
            if matches!(err, ReadlineError::Errno(nix::Error::ENOENT)) {
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
                rl.add_history_entry(line.as_str())?;
                interrupt.store(false, Ordering::Relaxed);
                match process_shell_line(&config, &line, interrupt.clone(), &selection) {
                    Ok(ShellAction::Found(s)) => {
                        if !s.is_empty() {
                            selection = Some(s);
                        }
                    },
                    Ok(ShellAction::Quit) => {
                        // Don't store \q in history.
                        break;
                    },
                    Ok(ShellAction::None) => {
                    },
                    Err(CliError::LocateError(LocateError::Interrupted)) => {
                        println!("CTRL-C");
                    },
                    Err(CliError::LocateError(LocateError::BrokenPipe))  => {
                        println!("EOF");
                    },
                    Err(err) => {
                        print_error(); eprintln!("{}", err);
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
        if let Some(history) = &history {
            rl.save_history(history).unwrap();
        }
    }
    Ok(())
}

enum ShellAction {
    Found(Vec<PathBuf>),
    None,
    Quit,
}

fn process_shell_line(config: &Config, line: &str, interrupt: Arc<AtomicBool>, selection: &Option<Vec<PathBuf>>) -> Result<ShellAction, CliError> {
    let token = tokenize_shell(line)?;
    if let Some(Token::Text(command)) = token.first() {
        // Backslash commands:
        if command.starts_with('\\') {
            match command.as_str() {
                "\\q" if token.len() == 1 => { return Ok(ShellAction::Quit); },
                "\\o" => { open_command(config, &token[1..], selection)?; },
                "\\u" if token.len() == 1 => { update_shell(config)?; },
                "\\h" => { let _ = help_shell(); },
                _ => { let _ = help_shell_short(); },
            };
            return Ok(ShellAction::None);
        }
        // Open commands:
        if match command.parse::<OpenRule>() {
            Ok(OpenRule::Index(_)) => true,
            Ok(OpenRule::IndexRange(_, _)) => true,
            Ok(OpenRule::IndexGlob(_, _)) => true,
            _ => false,
        } {
            open_command(config, &token, selection)?;
            return Ok(ShellAction::None);
        }
    }
    // Locate query:
    match locate_shell(
        config,
        line,
        Some(interrupt)
    ) {
        Ok(paths) => Ok(ShellAction::Found(paths)),
        Err(err) => Err(err),
    }
}

fn open_command(config: &Config, token: &[Token], selection: &Option<Vec<PathBuf>>) -> Result<(), CliError> {
    if let Some(selection) = selection {
        let mut command = Command::new("open");
        let mut found = false;
        for token in token {
            match token {
                crate::tokenizer::Token::Text(text) => {
                    if let Ok(open_rule) = text.parse::<OpenRule>() {
                        let expand = Expand::new(open_rule, selection);
                        expand.foreach(|path| open_append(&mut command, path, &mut found, config))?;
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

fn open_append(command: &mut Command, path: &Path, found: &mut bool, config: &Config) -> Result<(), CliError> {
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
        stderr().write_all(b"' not exists.")?;
        for base in &config.index.folder {
            if path.starts_with(base) {
                if !base.exists() {
                    stderr().write_all( b" Device not mounted.")?;
                    break;
                }
            }
        }
        stderr().write_all(b"\n")?;
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

pub fn print_error() {
    let mut stderr = StandardStream::stderr(ColorChoice::Auto);
    let _ = stderr.set_color(ColorSpec::new().set_fg(Some(Color::Red)));
    let _ = stderr.write_all(b"Error");
    let _ = stderr.set_color(&ColorSpec::new());
    let _ = stderr.write_all(b": ");
}
