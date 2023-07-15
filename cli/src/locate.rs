use fsidx::{FilterToken, LocateEvent, Metadata};
use std::collections::VecDeque;
use std::os::unix::prelude::OsStrExt;
use std::env::Args;
use std::io::{Result as IOResult, Write};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
use crate::cli::CliError;
use crate::config::{Config, get_volume_info};
use crate::tokenizer::{TokenIterator, Token};
use crate::verbosity::verbosity;


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

pub(crate) fn locate_cli(config: &Config, args: &mut Args) -> Result<(), CliError> {
    let mut stdout = StandardStream::stdout(ColorChoice::Auto);
    let filter_token = locate_filter(args)?;
    locate_impl(config, filter_token, None, |res| {
        print_locate_result(&mut stdout, &res)
    })?;
    Ok(())
}

pub(crate) fn locate_shell(config: &Config, mut token_it: TokenIterator, interrupt: Option<Arc<AtomicBool>>) -> Result<Vec<PathBuf>, CliError> {
    let mut stdout = StandardStream::stdout(ColorChoice::Auto);
    let mut selection = Vec::new();
    let filter_token = locate_filter_interactive(&mut token_it)?;
    locate_impl(config, filter_token, interrupt, |res| {
        if let LocateEvent::Entry(path, _) = res {
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

fn locate_impl<F: FnMut(LocateEvent)->IOResult<()>>(config: &Config, filter_token: Vec<FilterToken>, interrupt: Option<Arc<AtomicBool>>, f: F) -> Result<(), CliError> {
    let volume_info = get_volume_info(&config)
    .ok_or(CliError::NoDatabaseFound)?;
    match fsidx::locate(volume_info, filter_token, interrupt, f) {
        Ok(_) => Ok(()),
        Err(fsidx::LocateError::BrokenPipe) => Ok(()),     // No error for: fsidx | head -n 5
        Err(err) => Err(CliError::LocateError(err)),
    }
}

fn locate_filter(args: &mut Args) -> Result<Vec<FilterToken>, CliError> {
    let mut token = VecDeque::new();
    for text in args {
        if text.starts_with("--") {
            let long_option = &text[1..];
            token.push_back(Token::Option(long_option.to_string()));
        } else if text.starts_with("-") {
            let mut remainder = &text[1..];
            while !remainder.is_empty() {
                let long_option = &remainder[1..2];
                remainder = &remainder[2..];
                token.push_back(Token::Option(long_option.to_string()));
            }
        } else {
            token.push_back(Token::Text(String::from(text)));
        }
    }
    let token_vec = TokenVec {
        token
    };
    locate_filter_interactive(&mut token_vec.into_iter())
}

fn locate_filter_interactive(token_it: &mut dyn Iterator<Item = Token>) -> Result<Vec<FilterToken>, CliError> {
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
                    return Err(CliError::InvalidLocateFilterOption(text));
                },
            },
        };
        filter.push(filter_token);
    }
    Ok(filter)
}

fn print_size(stdout: &mut StandardStream, size: u64) -> IOResult<()> {
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

fn print_locate_result(stdout: &mut StandardStream, res: &LocateEvent) -> IOResult<()> {
    match *res {
        LocateEvent::Entry(path, Metadata { size: Some(size) } ) => {
            stdout.write_all(path.as_os_str().as_bytes())?;
            stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)))?;
            stdout.write_all(b" (")?;
            print_size(stdout, *size)?;
            stdout.write_all(b")")?;
            stdout.set_color(&ColorSpec::new())?;
            stdout.write_all(b"\n")?;
        },
        LocateEvent::Entry(path, Metadata { size:None } ) => {
            stdout.write_all(path.as_os_str().as_bytes())?;
            stdout.write_all(b"\n")?;
        },
        LocateEvent::Finished => {},
        LocateEvent::Interrupted => {
            stdout.write(b"CTRL-C\n")?;
        },
        LocateEvent::Searching(path) => {
            if verbosity() {
                stdout.write_all(b"Searching: ")?;
                stdout.write_all(path.as_os_str().as_bytes())?;
                stdout.write_all(b"\n")?;
            }
        },
        LocateEvent::SearchingFinished(path) => {
            if verbosity() {
                stdout.write_all(b"Searching  ")?;
                stdout.write_all(path.as_os_str().as_bytes())?;
                stdout.write_all(b" finished\n")?;
            }
        },
        LocateEvent::SearchingFailed(path, error) => {
            stdout.write_all(b"Searching ")?;
            stdout.write_all(path.as_os_str().as_bytes())?;
            stdout.write_fmt(format_args!(" failed: {}\n", error))?;
        },
    }
    Ok(())
}
