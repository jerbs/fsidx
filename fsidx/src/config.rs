use num_enum::TryFromPrimitive;
use std::path::PathBuf;

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
