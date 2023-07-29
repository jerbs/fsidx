use num_enum::TryFromPrimitive;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct VolumeInfo {
    pub folder: PathBuf,
    pub database: PathBuf,
}

#[derive(Debug, Clone, PartialEq, TryFromPrimitive)]
#[repr(u8)]
pub enum Settings {
    FileNamesOnly = 0,
    WithFileSizes = 1,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(deny_unknown_fields)]
pub struct LocateConfig {
    #[serde(default)]
    pub case: Case,
    #[serde(default)]
    pub order: Order,
    #[serde(default)]
    pub what: What,
    #[serde(default)]
    pub smart_spaces: bool,
    #[serde(default)]
    pub word_boundaries: bool,
    #[serde(default)]
    pub literal_separator: bool,
    #[serde(default)]
    pub mode: Mode,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "snake_case")]
pub enum Case {
    MatchCase,
    IgnoreCase,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "snake_case")]
pub enum Order {
    AnyOrder,
    SameOrder,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "snake_case")]
pub enum What {
    WholePath,
    LastElement,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Copy)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    Auto,
    Plain,
    Glob,
}

impl Default for LocateConfig {
    fn default() -> Self {
        LocateConfig {
            case: Case::IgnoreCase,
            order: Order::AnyOrder,
            what: What::WholePath,
            smart_spaces: true,
            word_boundaries: false,
            literal_separator: false,
            mode: Mode::Auto,
        }
    }
}

impl Default for Case {
    fn default() -> Self {
        Case::IgnoreCase
    }
}

impl Default for Order {
    fn default() -> Self {
        Order::AnyOrder
    }
}

impl Default for What {
    fn default() -> Self {
        What::WholePath
    }
}

impl Default for Mode {
    fn default() -> Self {
        Mode::Auto
    }
}
