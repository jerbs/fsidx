use clap::{App, Arg, ArgMatches};
use fsidx::{FilterToken, Settings};
use std::io::{Error, ErrorKind, Result};
use std::path::Path;
use crate::config::{Config, find_and_load, get_volume_info, load_from_path};
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
}

pub fn main() -> i32 {
    let matches = app_cli().get_matches();

    set_verbosity(matches.occurrences_of("verbosity"));

    if matches.is_present("version_info") {
        const VERSION: &str = env!("CARGO_PKG_VERSION");
        const NAME: &str = env!("CARGO_PKG_NAME");
        println!("{}: Version {}", NAME, VERSION);
    }

    let config: Config = if let Some(config_file) = matches.value_of("config_file") {
        if verbosity() {
            println!("Config File: {}", config_file);
        }
        match load_from_path(Path::new(config_file)) {
            Ok(config) => config,
            Err(msg) => {eprintln!("{}", msg); return 1},
        }
    } else {
        match find_and_load() {
            Ok(config) => config,
            Err(msg) => {eprintln!("{}", msg); return 1},
        }
    };

    let result = match matches.subcommand() {
        Some(("locate", sub_matches)) => locate(config, sub_matches),
        Some(("update", sub_matches)) => update(config, sub_matches),
        _ => {
            app_cli().print_help().ok();
            println!("\n");
            Err(Error::new(ErrorKind::Other, "Invalid command"))
        },
    };

    let exit_code = match result {
        Ok(exit_code) => exit_code,
        Err(err) => {
            eprintln!("Error: {}", err);
            1
        },
    };

    exit_code
}

fn locate_cli() -> App<'static> {
    App::new("locate")
    .about("Find matching files in the database")
    .arg(Arg::new("case_sensitive")
        .short('c')
        .multiple_occurrences(true)
        .takes_value(false) )
    .arg(Arg::new("case_insensitive")
        .short('i')
        .multiple_occurrences(true)
        .takes_value(false) )
    .arg(Arg::new("any_order")
        .short('a')
        .multiple_occurrences(true)
        .takes_value(false) )
    .arg(Arg::new("same_order")
        .short('s')
        .multiple_occurrences(true)
        .takes_value(false) )
    .arg(Arg::new("whole_path")
        .short('w')
        .multiple_occurrences(true)
        .takes_value(false) )
    .arg(Arg::new("last_element")
        .short('l')
        .multiple_occurrences(true)
        .takes_value(false) )
    .arg(Arg::new("text")
        // .allow_invalid_utf8(true)
        .multiple_occurrences(true)
    )
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
            filter.push((FilterToken::Text(text.to_string()), idx))
        }
    }
    filter.sort_by(|(_,a), (_,b)| a.cmp(b));
    filter.into_iter().map(|(token,_)| token).collect()
}

fn locate(config: Config, matches: &ArgMatches) -> Result<i32> {
    let filter_token = locate_filter(matches);
    let volume_info = get_volume_info(&config)
    .ok_or(Error::new(ErrorKind::Other, "No database path set"))?;
    fsidx::locate(volume_info, filter_token);
    Ok(0)
}

fn update_cli() -> App<'static> {
    App::new("update")
    .about("Rescan folders and update the database")
}

fn update(config: Config, _matches: &ArgMatches) -> Result<i32> {
    let volume_info = get_volume_info(&config)
    .ok_or(Error::new(ErrorKind::Other, "No database path set"))?;
    fsidx::update(volume_info, Settings::WithFileSizes);
    Ok(0)
}
