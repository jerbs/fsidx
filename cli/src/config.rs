use fsidx::{LocateConfig, VolumeInfo};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub index: Index,
    pub locate: LocateConfig,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(deny_unknown_fields)]
pub struct Index {
    pub folder: Vec<PathBuf>,
    pub db_path: Option<PathBuf>,
}

#[derive(Debug)]
pub enum ConfigError {
    FileReadError(PathBuf, std::io::Error),
    ParseError(PathBuf, toml::de::Error),
    TomlFileExpected(PathBuf),
    ConfigFileNotFound,
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::FileReadError(path, err) => f.write_fmt(format_args!(
                "Reading '{}' failed: {}",
                path.to_string_lossy(),
                err
            )),
            ConfigError::ParseError(path, err) => f.write_fmt(format_args!(
                "Parsing '{}' failed: {}",
                path.to_string_lossy(),
                err
            )),
            ConfigError::TomlFileExpected(path) => f.write_fmt(format_args!(
                "Expected a toml file instead of: '{}'",
                path.to_string_lossy()
            )),
            ConfigError::ConfigFileNotFound => f.write_str("Configuration file not found."),
        }
    }
}

pub fn find_and_load() -> Result<Config, ConfigError> {
    if let Ok(home) = env::var("HOME") {
        let path = Path::new(&home);
        let config_file_path = path.join(Path::new(".fsidx")).join(Path::new("fsidx.toml"));
        if config_file_path.exists() {
            return load_from_path(&config_file_path);
        }
    }
    let config_file_path = Path::new("/etc/fsidx/fsidx.toml");
    if config_file_path.exists() {
        return load_from_path(&config_file_path);
    }
    Err(ConfigError::ConfigFileNotFound)
}

pub fn load_from_path(file_name: &Path) -> Result<Config, ConfigError> {
    if file_name
        .extension()
        .ok_or(ConfigError::TomlFileExpected(file_name.to_owned()))?
        .to_str()
        .ok_or(ConfigError::TomlFileExpected(file_name.to_owned()))?
        != "toml"
    {
        return Err(ConfigError::TomlFileExpected(file_name.to_owned()));
    };
    let contents = fs::read_to_string(file_name)
        .map_err(|err: std::io::Error| ConfigError::FileReadError(file_name.to_owned(), err))?;
    let mut config = parse_content(&contents)
        .map_err(|err| ConfigError::ParseError(file_name.to_owned(), err))?;
    set_db_path(&mut config, file_name);
    Ok(config)
}

fn parse_content(contents: &str) -> Result<Config, toml::de::Error> {
    let mut config: Config = toml::from_str(&contents)?;
    resolve_leading_tilde(&mut config);
    Ok(config)
}

fn resolve_leading_tilde(config: &mut Config) {
    let tilde = Path::new("~");
    if let Ok(home) = env::var("HOME") {
        let home = Path::new(&home);
        for folder in &mut config.index.folder {
            if folder.starts_with(tilde) {
                match folder.strip_prefix(tilde) {
                    Ok(f) => *folder = home.join(f),
                    Err(_) => (),
                }
            }
        }
    }
}

fn set_db_path(config: &mut Config, config_file_path: &Path) {
    if None == config.index.db_path {
        config.index.db_path = match config_file_path.parent() {
            Some(path) => Some(path.to_path_buf()),
            None => None,
        }
    }
}

pub fn get_volume_info(config: &Config) -> Option<Vec<VolumeInfo>> {
    let volume_info = config
        .index
        .folder
        .iter()
        .filter_map(|folder| {
            let database = get_db_file_path(config, folder)?;
            let folder = folder.clone();
            Some(VolumeInfo { folder, database })
        })
        .collect();
    Some(volume_info)
}

pub fn get_db_file_path(config: &Config, folder: &Path) -> Option<PathBuf> {
    if let Some(db_path) = config.index.db_path.as_deref() {
        let s: &str = folder.to_str().unwrap();
        let mut file_name = s.replace("/", "_");
        file_name.push_str(".fsdb");
        Some(db_path.join(Path::new(&file_name)))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fsidx::{Mode, Order, What};
    use indoc::indoc;

    #[test]
    fn toml_parsing() {
        let home = env::var("HOME").unwrap();

        let data = indoc! {
        r#"[index]
            folder = [
                "~/Music",
                "/Volumes/Music"
            ]

            [locate]
            case_sensitive = false
            order = "any_order"
            what = "whole_path"
            smart_spaces = true
            word_boundaries = false
            literal_separator = false
            mode = "auto"
            "#};
        let config: Config = parse_content(data).unwrap();
        assert_eq!(
            config,
            Config {
                index: Index {
                    folder: vec![
                        PathBuf::from(format!("{}/Music", home)),
                        PathBuf::from("/Volumes/Music")
                    ],
                    db_path: None
                },
                locate: LocateConfig {
                    case_sensitive: false,
                    order: Order::AnyOrder,
                    what: What::WholePath,
                    smart_spaces: true,
                    word_boundaries: false,
                    literal_separator: false,
                    mode: Mode::Auto,
                },
            }
        );
    }

    #[test]
    fn encode_toml() {
        let config = Config {
            index: Index {
                folder: vec![PathBuf::from("~/Music"), PathBuf::from("/Volumes/Music")],
                db_path: None,
            },
            locate: LocateConfig {
                case_sensitive: true,
                order: Order::AnyOrder,
                what: What::WholePath,
                smart_spaces: true,
                word_boundaries: false,
                literal_separator: false,
                mode: Mode::Auto,
            },
        };
        let toml = toml::to_string(&config).unwrap();
        let expected = indoc! {
        r#"[index]
            folder = ["~/Music", "/Volumes/Music"]

            [locate]
            case_sensitive = true
            order = "any_order"
            what = "whole_path"
            smart_spaces = true
            word_boundaries = false
            literal_separator = false
            mode = "auto"
            "#};
        assert_eq!(toml, expected);
        // println!("{}", toml);
    }
}
