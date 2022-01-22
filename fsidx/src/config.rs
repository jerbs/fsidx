use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct VolumeInfo {
    pub folder: PathBuf,
    pub database: PathBuf,
}
