use fastvlq::ReadVu64Ext;
use std::convert::TryFrom;
use std::ffi::OsStr;
use std::fmt::Display;
use std::fs::File;
use std::io::{BufReader, Error, ErrorKind, Read, Result as IOResult};
use std::os::unix::prelude::OsStrExt;
use std::path::Path;
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
    ExpectedFsidFile,
    UnexpectedEof,
    ReadingFileFailed(std::io::Error),
    WritingResultFailed(std::io::Error),
    Interrupted,
    BrokenPipe,
}

impl From<std::io::Error> for LocateError {
    fn from(err: std::io::Error) -> Self {
        LocateError::ReadingFileFailed(err)
    }
}

pub struct Metadata {
    pub size: Option<u64>,
}

pub fn locate<F: FnMut(LocateEvent)->IOResult<()>>(volume_info: Vec<VolumeInfo>, filter: Vec<FilterToken>, interrupt: Option<Arc<AtomicBool>>, mut f: F) -> Result<(), LocateError> {
    for vi in &volume_info {
        f(LocateEvent::Searching(&vi.folder))?;
        let res = locate_volume(vi, &filter, &interrupt, &mut f);
        if let Err(ref err) = res {
            match err {
                LocateError::Interrupted => return res,
                LocateError::WritingResultFailed(err) if err.kind() == ErrorKind::BrokenPipe => return Err(LocateError::BrokenPipe),
                err => { f(LocateEvent::SearchingFailed(&vi.folder, &err))?; },
            }
        }
    }
    Ok(())
}

pub fn locate_volume<F: FnMut(LocateEvent)->IOResult<()>>(volume_info: &VolumeInfo, filter: &Vec<FilterToken>, interrupt: &Option<Arc<AtomicBool>>, f: &mut F) -> Result<(), LocateError> {    
    let mut reader = FileIndexReader::new(&volume_info.database)?;
    let filter = filter::compile(&filter)
    .map_err(|_| Error::new(ErrorKind::Other, "pattern error"))?;
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
    reader: BufReader<File>,
    path: Vec<u8>,
    settings: Settings,
}

impl FileIndexReader {
    pub fn new(database: &Path) -> Result<FileIndexReader, LocateError>
    {
        let file = File::open(database)?;
        let mut reader = BufReader::new(file);
        let mut fourcc: [u8; 4] = [0; 4];
        reader.read_exact(&mut fourcc)?;
        if fourcc != "fsix".as_bytes() {
            return Err(LocateError::ExpectedFsidFile);
        }
        let mut flags: [u8; 1] = [0; 1];
        reader.read_exact(&mut flags)?;
        let settings = Settings::try_from(flags[0])
        .map_err(|_err| Error::new(ErrorKind::InvalidData, "Unsupported file format"))?;
        let path: Vec<u8> = Vec::new();
        Ok(FileIndexReader { reader, path, settings } )
    }

    pub fn next(&mut self) -> Result<Option<(&Path, Metadata)>, LocateError> {
        let discard = match self.reader.read_vu64() {
            Ok(val) => val,
            Err(err) => {
                match err.kind() {
                    ErrorKind::UnexpectedEof => return Ok(None),
                    _ => return Err(LocateError::ReadingFileFailed(err)),
                }
            },
        };
        let length = self.reader.read_vu64()?;
        let mut delta = vec![0u8; length as usize];
        self.reader.read_exact(&mut delta)?;
        delta_decode(&mut self.path, discard, &delta);
        let size = if self.settings == Settings::WithFileSizes {
            let size_plus_one = self.reader.read_vu64()?;
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
            LocateError::ExpectedFsidFile => f.write_str("Expected fsdb file"),
            LocateError::UnexpectedEof => f.write_str("Unexpected end of file"),
            LocateError::ReadingFileFailed(_) => f.write_fmt(format_args!("")),
            LocateError::WritingResultFailed(_) => f.write_fmt(format_args!("")),
            LocateError::Interrupted => f.write_str("Interrupted"),
            LocateError::BrokenPipe => f.write_str("Boken pipe"),
        }
    }
}
