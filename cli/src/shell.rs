use crate::cli::CliError;
use crate::config::Config;
use crate::expand::{Expand, OpenRule};
use crate::help::{help_shell_long, help_shell_short};
use crate::locate::locate_shell;
use crate::tokenizer::{tokenize_shell, Token};
use crate::tty::set_tty;
use crate::update::update_shell;
use crate::verbosity::verbosity;
use fsidx::LocateError;
use rustyline::completion::Completer;
use rustyline::config::Config as RlConfig;
use rustyline::error::ReadlineError;
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::Editor;
use rustyline::{Helper, Validator};
use signal_hook::consts::signal::SIGINT;
use signal_hook::iterator::Signals;
use std::borrow::Cow;
use std::env::Args;
use std::io::{stderr, stdout, Result as IOResult, Write};
use std::os::unix::prelude::OsStrExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

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
    set_tty().map_err(CliError::TtyConfigurationFailed)?;
    let abort = Arc::new(AtomicBool::new(false));
    let mut signals = Signals::new([SIGINT]) // Ctrl-C
        .map_err(CliError::CreatingSignalHandlerFailed)?;
    let abort_for_signal_handler = abort.clone();
    std::thread::spawn(move || {
        let abort = abort_for_signal_handler;
        for sig in signals.forever() {
            if verbosity() {
                println!("Received signal {}", sig);
            }
            if sig == SIGINT {
                abort.store(true, Ordering::Relaxed);
            }
        }
    });
    let rl_config = RlConfig::builder()
        .max_history_size(100)?
        .history_ignore_dups(true)?
        .history_ignore_space(true)
        .completion_type(rustyline::CompletionType::List)
        .completion_prompt_limit(20)
        .edit_mode(rustyline::EditMode::Emacs)
        .auto_add_history(true)
        .bell_style(rustyline::config::BellStyle::None)
        .color_mode(rustyline::ColorMode::Enabled)
        .build();
    let helper = ShellHelper {};
    let mut rl = Editor::<ShellHelper, _>::with_config(rl_config)?;
    rl.set_helper(Some(helper));
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
                abort.store(false, Ordering::Relaxed);
                match process_shell_line(&config, &line, abort.clone(), &selection) {
                    Ok(ShellAction::Found(s)) => {
                        if !s.is_empty() {
                            selection = Some(s);
                        }
                    }
                    Ok(ShellAction::Quit) => {
                        // Don't store \q in history.
                        break;
                    }
                    Ok(ShellAction::None) => {}
                    Err(CliError::LocateError(LocateError::Aborted)) => {
                        println!("CTRL-C");
                    }
                    Err(CliError::LocateError(LocateError::BrokenPipe)) => {
                        println!("EOF");
                    }
                    Err(err) => {
                        print_error();
                        eprintln!("{}", err);
                    }
                };
            }
            Err(ReadlineError::Interrupted) => {
                println!("CTRL-C");
            }
            Err(ReadlineError::Eof) => {
                println!("CTRL-D");
                break;
            }
            Err(err) => {
                print_error();
                eprintln!("{}", err);
                break;
            }
        }
        if let Some(history) = &history {
            rl.save_history(history).unwrap();
        }
    }
    Ok(())
}

#[derive(Helper, Validator)]
struct ShellHelper {}

const LONG_OPTIONS: [&str; 15] = [
    "--case-sensitive ",
    "--case-insensitive ",
    "--plain ",
    "--glob ",
    "--auto ",
    "--same-order ",
    "--any-order ",
    "--last-element ",
    "--whole-path ",
    "--no-smart-spaces ",
    "--smart-spaces ",
    "--word-boundary ",
    "--no-word-boundary ",
    "--literal-separator ",
    "--no-literal-separator ",
];

impl Hinter for ShellHelper {
    type Hint = String;

    fn hint(&self, line: &str, pos: usize, _ctx: &rustyline::Context<'_>) -> Option<Self::Hint> {
        let start = start_position(line, pos);
        let partial = &line[start..pos];
        if partial.is_empty() {
            return None;
        }
        if let Some(first) = LONG_OPTIONS
            .into_iter()
            .find(|cand| cand.starts_with(partial))
        {
            let hint = first[pos - start..].to_string();
            Some(hint)
        } else {
            None
        }
    }
}

impl Completer for ShellHelper {
    type Candidate = &'static str;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &rustyline::Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Self::Candidate>)> {
        let start = start_position(line, pos);
        let partial = &line[start..pos];
        if partial.is_empty() {
            Ok((0, Vec::with_capacity(0)))
        } else {
            let candidates = LONG_OPTIONS
                .into_iter()
                .filter(|cand| cand.starts_with(partial))
                .collect();
            Ok((start, candidates))
        }
    }

    fn update(
        &self,
        line: &mut rustyline::line_buffer::LineBuffer,
        start: usize,
        elected: &str,
        cl: &mut rustyline::Changeset,
    ) {
        let end = line.pos();
        line.replace(start..end, elected, cl);
    }
}

impl Highlighter for ShellHelper {
    fn highlight_hint<'h>(&self, hint: &'h str) -> Cow<'h, str> {
        let mut highlighted = String::from("\x1B[2m");
        highlighted.push_str(hint);
        highlighted.push_str("\x1B[0m");
        Cow::Owned(highlighted)
    }
}

fn start_position(line: &str, pos: usize) -> usize {
    if let Some(pos_last_whitespace) = line[0..pos].rfind(|ch: char| ch.is_whitespace()) {
        pos_last_whitespace
            + line[pos_last_whitespace..]
                .chars()
                .next()
                .unwrap()
                .len_utf8()
    } else {
        0
    }
}

enum ShellAction {
    Found(Vec<PathBuf>),
    None,
    Quit,
}

fn process_shell_line(
    config: &Config,
    line: &str,
    abort: Arc<AtomicBool>,
    selection: &Option<Vec<PathBuf>>,
) -> Result<ShellAction, CliError> {
    let token = tokenize_shell(line)?;
    if let Some(Token::Text(command)) = token.first() {
        // Backslash commands:
        if command.starts_with('\\') {
            match command.as_str() {
                "\\q" if token.len() == 1 => {
                    return Ok(ShellAction::Quit);
                }
                "\\o" => {
                    open_command(config, &token[1..], selection)?;
                }
                "\\u" if token.len() == 1 => {
                    update_shell(config)?;
                }
                "\\h" => {
                    let _ = help_shell_long();
                }
                _ => {
                    let _ = help_shell_short();
                }
            };
            return Ok(ShellAction::None);
        }
        // Open commands:
        if matches!(
            command.parse::<OpenRule>(),
            Ok(OpenRule::Index(_)) | Ok(OpenRule::IndexRange(_, _)) | Ok(OpenRule::IndexGlob(_, _))
        ) {
            open_command(config, &token, selection)?;
            return Ok(ShellAction::None);
        }
    }
    // Locate query:
    match locate_shell(config, line, Some(abort)) {
        Ok(paths) => Ok(ShellAction::Found(paths)),
        Err(err) => Err(err),
    }
}

fn open_command(
    config: &Config,
    token: &[Token],
    selection: &Option<Vec<PathBuf>>,
) -> Result<(), CliError> {
    if let Some(selection) = selection {
        let mut command = Command::new("open");
        let mut found = false;
        for token in token {
            match token {
                crate::tokenizer::Token::Text(text) => {
                    if let Ok(open_rule) = text.parse::<OpenRule>() {
                        let expand = Expand::new(open_rule, selection);
                        expand
                            .foreach(|path| open_append(&mut command, path, &mut found, config))?;
                    } else {
                        return Err(CliError::InvalidOpenRule(text.clone()));
                    }
                }
                crate::tokenizer::Token::Option(_) => {} // TODO: Implement options to configure glob expansion.
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

fn open_append(
    command: &mut Command,
    path: &Path,
    found: &mut bool,
    config: &Config,
) -> Result<(), CliError> {
    if path.exists() {
        command.arg(path);
        *found = true;
        stdout().write_all(b"Opening: '")?;
        stdout().write_all(path.as_os_str().as_bytes())?;
        stdout().write_all(b"'\n")?;
    } else {
        print_error();
        stderr().write_all(b"'")?;
        stderr().write_all(path.as_os_str().as_bytes())?;
        stderr().write_all(b"' not exists.")?;
        for base in &config.index.folder {
            if path.starts_with(base) && !base.exists() {
                stderr().write_all(b" Device not mounted.")?;
                break;
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
