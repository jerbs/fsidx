use fastvlq::ReadVu64Ext;
use std::convert::TryFrom;
use std::ffi::OsStr;
use std::fmt::Display;
use std::fs::File;
use std::io::{BufReader, ErrorKind, Read, Result as IOResult};
use std::os::unix::prelude::OsStrExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use crate::{Settings, VolumeInfo, FilterToken, filter};

pub enum LocateEvent<'a> {
    Entry(&'a Path, &'a Metadata),
    Finished,
    Interrupted,
    Searching(&'a Path),
    SearchingFinished(&'a Path),
    SearchingFailed(&'a Path, &'a LocateError),
}

#[derive(Debug)]
pub enum LocateError {
    ExpectedFsdbFile(PathBuf),
    UnexpectedEof(PathBuf),
    ReadingFileFailed(PathBuf, std::io::Error),
    WritingResultFailed(std::io::Error),
    UnsupportedFileFormat(PathBuf),
    Interrupted,
    BrokenPipe,
    GlobPatternError(String, globset::Error),
}

pub struct Metadata {
    pub size: Option<u64>,
}

pub fn locate<F: FnMut(LocateEvent)->IOResult<()>>(volume_info: Vec<VolumeInfo>, filter: Vec<FilterToken>, interrupt: Option<Arc<AtomicBool>>, mut f: F) -> Result<(), LocateError> {
    for vi in &volume_info {
        f(LocateEvent::Searching(&vi.folder)).map_err(|err| LocateError::WritingResultFailed(err))?;
        let res = locate_volume(vi, &filter, &interrupt, &mut f);
        if let Err(ref err) = res {
            match err {
                LocateError::Interrupted => return res,
                LocateError::WritingResultFailed(err) if err.kind() == ErrorKind::BrokenPipe => return Err(LocateError::BrokenPipe),
                err => f(LocateEvent::SearchingFailed(&vi.folder, &err)).map_err(|err| LocateError::WritingResultFailed(err))?,
            }
        }
    }
    Ok(())
}

pub fn locate_volume<F: FnMut(LocateEvent)->IOResult<()>>(volume_info: &VolumeInfo, filter: &Vec<FilterToken>, interrupt: &Option<Arc<AtomicBool>>, f: &mut F) -> Result<(), LocateError> {    
    let mut reader = FileIndexReader::new(&volume_info.database)?;
    let filter = filter::compile(&filter)?;
    loop {
        if interrupt.as_ref().map(|v| v.load(Ordering::Relaxed)).unwrap_or(false) {
            return Err(LocateError::Interrupted);
        }
        match reader.next() {
            Ok(Some((path, metadata))) => {
                let bytes = path.as_os_str().as_bytes();
                let text = String::from_utf8_lossy(bytes);
                if filter::apply(&text, &filter) {
                    f(LocateEvent::Entry(path, &metadata)).map_err(|err| LocateError::WritingResultFailed(err))?;
                }
            },
            Ok(None) => return Ok(()),
            Err(err) => return Err(err),
        }
    };
}

pub struct FileIndexReader {
    database: PathBuf,
    reader: BufReader<File>,
    path: Vec<u8>,
    settings: Settings,
}

impl FileIndexReader {
    pub fn new(database: &Path) -> Result<FileIndexReader, LocateError>
    {
        let file = File::open(database).map_err(|err| LocateError::ReadingFileFailed(database.to_owned(), err))?;
        let mut reader = BufReader::new(file);
        let mut fourcc: [u8; 4] = [0; 4];
        reader.read_exact(&mut fourcc).map_err(|err| LocateError::ReadingFileFailed(database.to_owned(), err))?;
        if fourcc != "fsix".as_bytes() {
            return Err(LocateError::ExpectedFsdbFile(database.to_owned()));
        }
        let mut flags: [u8; 1] = [0; 1];
        reader.read_exact(&mut flags).map_err(|err| LocateError::ReadingFileFailed(database.to_owned(), err))?;
        let settings = Settings::try_from(flags[0])
        .map_err(|_err| LocateError::UnsupportedFileFormat(database.to_owned()))?;
        let path: Vec<u8> = Vec::new();
        let database = database.to_owned();
        Ok(FileIndexReader { database, reader, path, settings } )
    }

    pub fn next(&mut self) -> Result<Option<(&Path, Metadata)>, LocateError> {
        let discard = match self.reader.read_vu64() {
            Ok(val) => val,
            Err(err) => {
                match err.kind() {
                    ErrorKind::UnexpectedEof => return Ok(None),
                    _ => return Err(LocateError::ReadingFileFailed(self.database.clone(), err)),
                }
            },
        };
        let length = self.reader.read_vu64().map_err(|err| LocateError::ReadingFileFailed(self.database.clone(), err))?;
        let mut delta = vec![0u8; length as usize];
        self.reader.read_exact(&mut delta).map_err(|err| LocateError::ReadingFileFailed(self.database.clone(), err))?;
        delta_decode(&mut self.path, discard, &delta);
        let size = if self.settings == Settings::WithFileSizes {
            let size_plus_one = self.reader.read_vu64().map_err(|err| LocateError::ReadingFileFailed(self.database.clone(), err))?;
            if size_plus_one == 0 {
                None
            } else {
                Some(size_plus_one -1)
            }
        } else {
            None
        };
        let path = Path::new(OsStr::from_bytes(self.path.as_slice()));        
        Ok(Some((path, Metadata { size } )))
    }
}

fn delta_decode(path: &mut Vec<u8>, discard: u64, delta: &[u8]) {
    let len = path.len();
    let reuse = len - (discard as usize);
    path.splice(reuse..len, delta.iter().cloned());
}

impl Display for LocateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LocateError::ExpectedFsdbFile(path) => f.write_fmt(format_args!("Expected fsdb file: '{}'", path.to_string_lossy())),
            LocateError::UnexpectedEof(path) => f.write_fmt(format_args!("Unexpected end of database file: '{}'", path.to_string_lossy())),
            LocateError::ReadingFileFailed(path, err) => f.write_fmt(format_args!("Reading database '{}' failed: {}", path.to_string_lossy(), err)),
            LocateError::WritingResultFailed(err) => f.write_fmt(format_args!("Writing results failed: {}", err)),
            LocateError::UnsupportedFileFormat(path) => f.write_fmt(format_args!("Database has unsupported file format: '{}'", path.to_string_lossy())),
            LocateError::Interrupted => f.write_str("Interrupted"),
            LocateError::BrokenPipe => f.write_str("Boken pipe"),
            LocateError::GlobPatternError(glob, err) => f.write_fmt(format_args!("Glob pattern error for `{}`: {}", glob, err)),
        }
    }
}
