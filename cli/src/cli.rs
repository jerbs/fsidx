use std::{env::{Args, args}, process};
use std::io::{Error, stdout, Write};
use std::path::PathBuf;
use crate::help::{help_cli, print_version, usage_cli};
use crate::config::{Config, ConfigError, find_and_load, load_from_path};
use crate::locate::locate;
use crate::shell::shell;
use crate::update::update;
use crate::verbosity::{verbosity, set_verbosity};


struct MainOptions {
    config_file: Option<PathBuf>,
    help: bool,
    verbose: u8,
    version: bool,
}

#[derive(Debug)]
pub(crate) enum CliError {
    MissingValue(String),
    InvalidOption(String),
    InvalidSubCommand(String),
    ConfigError(ConfigError),
    LocateError(fsidx::LocateError),
    NoDatabaseFound,
    TtyConfigurationFailed(std::io::Error),
    CreatingSignalHandlerFailed(std::io::Error),
    StdoutWriteFailed(std::io::Error),
    InvalidLocateFilterOption(String),
    InvalidShellArgument(String),
    InvalidUpdateArgument(String),
}

impl From<Error> for CliError {
    fn from(value: Error) -> Self {
        CliError::StdoutWriteFailed(value)
    }
}

// FIXME: Implement more From traits to avoid map_err.

impl Default for MainOptions {
    fn default() -> Self {
        Self {
            config_file: None,
            help: false,
            verbose: 0,
            version: false,
        }
    }
}

pub fn main() -> i32 {
    if let Err(err) = process_main_command() {
        eprintln!("{:?}", err);
        process::exit(1);
    }
    0
}

fn process_main_command() -> Result<(), CliError> {
    let mut args = args();
    let _ = args.next();
    let (main_options, sub_command) = parse_main_command(&mut args)?;
    set_verbosity(main_options.verbose);
    if main_options.help {
        let _ = help_cli();
        process::exit(0);
    }
    if main_options.version {
        print_version();
        process::exit(0);
    }
    let config: Config = if let Some(config_file) = main_options.config_file {
        if verbosity() {
            let _ = writeln!(stdout().lock(), "Config File: {}", config_file.to_string_lossy());
        }
        match load_from_path(&config_file) {
            Ok(config) => config,
            Err(err) => {return Err(CliError::ConfigError(err))},
        }
    } else {
        match find_and_load() {
            Ok(config) => config,
            Err(err) => {return Err(CliError::ConfigError(err))},
        }
    };

    if let Some(sub_command) = sub_command {
        match sub_command.as_str() {
            "shell"  => { shell(config, &mut args) },
            "locate" => { locate(&config, &mut args) },
            "update" => { update(&config, &mut args) },
            "help"   => { help_cli() },
            _        => { Err(CliError::InvalidSubCommand(sub_command)) }
        }
    } else {
        usage_cli()
    }
}

fn parse_main_command(args: &mut Args) -> Result<(MainOptions, Option<String>), CliError>  {
    let mut main_options = MainOptions::default();
    let sub_command = loop {
        if let Some(item) = args.next() {
            if item.starts_with("--") {
                let long_option = &item[2..];
                main_options.parse(long_option, args)?;
            } else if item.starts_with("-") {
                let mut remainder = &item[1..];
                while !remainder.is_empty() {
                    let short_option = &remainder[0..1];
                    remainder = &remainder[1..];
                    main_options.parse(short_option, args)?;
                }
            } else {
                break Some(item);
            };
        } else {
            break None;
        }
    };
    Ok((main_options, sub_command))
}

impl MainOptions {
    fn parse(&mut self, option: &str, args: &mut Args) -> Result<(), CliError> {
        match option {
            "c" | "config"  => { self.config_file = Some(get_path_buf(args)
                                        .ok_or_else(|| CliError::MissingValue(option.to_string()))?); },
            "h" | "help"    => { self.help = true; },
            "v" | "verbose" => { self.verbose += 1; },
            "V" | "version" => { self.version = true; },
            val => { return Err(CliError::InvalidOption(val.to_string())); },
        }
        Ok(())
    }
}

fn get_path_buf(args: &mut Args) -> Option<PathBuf>  {
    if let Some(text) = args.next() {
        Some(PathBuf::from(text))
    } else {
        None
    }
}
