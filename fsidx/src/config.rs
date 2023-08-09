use num_enum::TryFromPrimitive;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// VolumeInfo holds the root folder of a scanned directory tree and the location of the corresponding database file.
#[derive(Debug, Clone)]
pub struct VolumeInfo {
    /// Root folder of a scanned directory tree.
    pub folder: PathBuf,
    /// Location of the corresponding database file.
    pub database: PathBuf,
}

/// Settings about what information will be stored in the database.
#[derive(Debug, Clone, PartialEq, TryFromPrimitive)]
#[repr(u8)]
pub enum Settings {
    /// Store file names.
    FileNamesOnly = 0,
    /// Store file names and sizes.
    WithFileSizes = 1,
}

/// Default configuration for locate queries.
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(deny_unknown_fields)]
pub struct LocateConfig {
    /// Case-sensitivity.
    #[serde(default = "default_case_sensitive")]
    pub case_sensitive: bool,
    /// In which order plain text must appear.
    #[serde(default)]
    pub order: Order,
    /// What parts of the pathname are searched.
    #[serde(default)]
    pub what: What,
    /// If space, minus and underscore in plain text match each other and no character.
    #[serde(default = "default_smart_spaces")]
    pub smart_spaces: bool,
    /// If start and end of plain text must match on a word boundary.
    #[serde(default = "default_word_boundaries")]
    pub word_boundaries: bool,
    /// If an asterisk (*) in a glob expression is not matching a path separator (/).
    #[serde(default = "default_literal_separator")]
    pub literal_separator: bool,
    /// Distinguish between glob patterns and plain text.
    #[serde(default)]
    pub mode: Mode,
}

fn default_case_sensitive() -> bool {
    false
}

fn default_smart_spaces() -> bool {
    true
}

fn default_word_boundaries() -> bool {
    false
}

fn default_literal_separator() -> bool {
    false
}

/// Defines in which order plain text must appear in the pathname.
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Default)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "snake_case")]
pub enum Order {
    /// Plain text may appear in any order.
    #[default]
    AnyOrder,
    /// Plain text must appear in the same order.
    SameOrder,
}

/// Defines which parts of the pathname are used to match plain text and glob patterns.
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Default)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "snake_case")]
pub enum What {
    /// The whole path is used.
    #[default]
    WholePath,
    /// The last path element is used only.
    LastElement,
}

/// Defines how subsequent [FilterToken::Text](crate::filter::FilterToken#variant.Text)
/// filter elements are used.
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Copy, Default)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    /// Glob patterns are autodetected.
    #[default]
    Auto,
    /// [Text](crate::filter::FilterToken#variant.Text) elements are used
    /// as plain text.
    Plain,

    /// [Text](crate::filter::FilterToken#variant.Text) elements are used
    /// as glob patterns.
    Glob,
}

impl Default for LocateConfig {
    fn default() -> Self {
        LocateConfig {
            case_sensitive: default_case_sensitive(),
            order: Order::default(),
            what: What::default(),
            smart_spaces: default_smart_spaces(),
            word_boundaries: default_word_boundaries(),
            literal_separator: default_literal_separator(),
            mode: Mode::default(),
        }
    }
}
