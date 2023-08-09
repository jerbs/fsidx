use crate::config::LocateConfig;
use crate::filter::CompiledFilter;
use crate::{filter, FilterToken, Settings, VolumeInfo};
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

/// LocateEvent indicates events to a callback function.
pub enum LocateEvent<'a> {
    /// A database entry that matches the query.
    Entry(&'a Path, &'a Metadata),
    /// Query is processed completely.
    Finished,
    /// Starts evaluating a query against a database file.
    Searching(&'a Path),
    /// All entries in a database file are evaluated against the query.
    SearchingFinished(&'a Path),
}

/// LocateError reports errors related to processing a query.
#[derive(Debug)]
pub enum LocateError {
    /// File is not a database file.
    ExpectedFsdbFile(PathBuf),
    /// Unexpected end of database file.
    UnexpectedEof(PathBuf),
    /// Reading the database file failed.
    ReadingFileFailed(PathBuf, std::io::Error),
    /// Writing results failed. Actually, the callback failed to write.
    WritingResultFailed(std::io::Error),
    /// Database file was written with an incompatible (e.g. newer) fsidx version.
    UnsupportedFileFormat(PathBuf),
    /// Query was aborted.
    Aborted,
    /// Writing failed due to a broken pipe. This error is reported when the
    /// cli frontend is piping its output to another program which is
    /// terminated before reading the complete input.
    BrokenPipe,
    /// Failed to compile a glob pattern.
    GlobPatternError(String, globset::Error),
    /// Reports a trivial search query that will by definition not match any
    /// database entry.
    Trivial,
}

/// Metadata of a single locate query result.
pub struct Metadata {
    /// File size. The field is optional, since the database file may not
    /// contain the file sizes.
    pub size: Option<u64>,
}

/// The locate function runs a query on all configured database files.
///
/// The matching entries are reported with a callback function. The abort
/// parameter may be used by a frontend to abort a query.
///
/// Design decision: The locate function is using a callback interface. This
/// allows to use references. With an iterator interface this is not possible
/// due to lifetime restrictions of Rust. The pathname is only available until
/// the next database entry is validated against the search query. Providing
/// an Iterator interface would require to return owned data. Allocating
/// memory on the heap for every query result would be less efficient.
pub fn locate<F: FnMut(LocateEvent) -> IOResult<()>>(
    volume_info: Vec<VolumeInfo>,
    filter: Vec<FilterToken>,
    config: &LocateConfig,
    abort: Option<Arc<AtomicBool>>,
    mut f: F,
) -> Result<(), LocateError> {
    let filter = filter::compile(&filter, config);
    if matches!(filter, Err(LocateError::Trivial)) {
        return Ok(());
    }
    let filter = filter?;
    for vi in &volume_info {
        f(LocateEvent::Searching(&vi.folder)).map_err(LocateError::WritingResultFailed)?;
        let res = locate_volume(vi, &filter, &abort, &mut f);
        if let Err(ref err) = res {
            match err {
                LocateError::WritingResultFailed(err) if err.kind() == ErrorKind::BrokenPipe => {
                    return Err(LocateError::BrokenPipe)
                }
                _ => return res,
            }
        }
    }
    Ok(())
}

fn locate_volume<F: FnMut(LocateEvent) -> IOResult<()>>(
    volume_info: &VolumeInfo,
    filter: &CompiledFilter,
    abort: &Option<Arc<AtomicBool>>,
    f: &mut F,
) -> Result<(), LocateError> {
    let mut reader = FileIndexReader::new(&volume_info.database)?;
    loop {
        if abort
            .as_ref()
            .map(|v| v.load(Ordering::Relaxed))
            .unwrap_or(false)
        {
            return Err(LocateError::Aborted);
        }
        match reader.next_entry() {
            Ok(Some((path, metadata))) => {
                let bytes = path.as_os_str().as_bytes();
                let text = String::from_utf8_lossy(bytes);
                if filter::apply(&text, filter) {
                    f(LocateEvent::Entry(path, &metadata))
                        .map_err(LocateError::WritingResultFailed)?;
                }
            }
            Ok(None) => return Ok(()),
            Err(err) => return Err(err),
        }
    }
}

struct FileIndexReader {
    database: PathBuf,
    reader: BufReader<File>,
    path: Vec<u8>,
    settings: Settings,
}

impl FileIndexReader {
    fn new(database: &Path) -> Result<FileIndexReader, LocateError> {
        let file = File::open(database)
            .map_err(|err| LocateError::ReadingFileFailed(database.to_owned(), err))?;
        let mut reader = BufReader::new(file);
        let mut fourcc: [u8; 4] = [0; 4];
        reader
            .read_exact(&mut fourcc)
            .map_err(|err| LocateError::ReadingFileFailed(database.to_owned(), err))?;
        if fourcc != "fsix".as_bytes() {
            return Err(LocateError::ExpectedFsdbFile(database.to_owned()));
        }
        let mut flags: [u8; 1] = [0; 1];
        reader
            .read_exact(&mut flags)
            .map_err(|err| LocateError::ReadingFileFailed(database.to_owned(), err))?;
        let settings = Settings::try_from(flags[0])
            .map_err(|_err| LocateError::UnsupportedFileFormat(database.to_owned()))?;
        let path: Vec<u8> = Vec::new();
        let database = database.to_owned();
        Ok(FileIndexReader {
            database,
            reader,
            path,
            settings,
        })
    }

    fn next_entry(&mut self) -> Result<Option<(&Path, Metadata)>, LocateError> {
        let discard = match self.reader.read_vu64() {
            Ok(val) => val,
            Err(err) => match err.kind() {
                ErrorKind::UnexpectedEof => return Ok(None),
                _ => return Err(LocateError::ReadingFileFailed(self.database.clone(), err)),
            },
        };
        let length = self
            .reader
            .read_vu64()
            .map_err(|err| LocateError::ReadingFileFailed(self.database.clone(), err))?;
        let mut delta = vec![0u8; length as usize];
        self.reader
            .read_exact(&mut delta)
            .map_err(|err| LocateError::ReadingFileFailed(self.database.clone(), err))?;
        delta_decode(&mut self.path, discard, &delta);
        let size = if self.settings == Settings::WithFileSizes {
            let size_plus_one = self
                .reader
                .read_vu64()
                .map_err(|err| LocateError::ReadingFileFailed(self.database.clone(), err))?;
            if size_plus_one == 0 {
                None
            } else {
                Some(size_plus_one - 1)
            }
        } else {
            None
        };
        let path = Path::new(OsStr::from_bytes(self.path.as_slice()));
        Ok(Some((path, Metadata { size })))
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
            LocateError::ExpectedFsdbFile(path) => f.write_fmt(format_args!(
                "Expected fsdb file: '{}'",
                path.to_string_lossy()
            )),
            LocateError::UnexpectedEof(path) => f.write_fmt(format_args!(
                "Unexpected end of database file: '{}'",
                path.to_string_lossy()
            )),
            LocateError::ReadingFileFailed(path, err) => f.write_fmt(format_args!(
                "Reading database '{}' failed: {}",
                path.to_string_lossy(),
                err
            )),
            LocateError::WritingResultFailed(err) => {
                f.write_fmt(format_args!("Writing results failed: {}", err))
            }
            LocateError::UnsupportedFileFormat(path) => f.write_fmt(format_args!(
                "Database has unsupported file format: '{}'",
                path.to_string_lossy()
            )),
            LocateError::Aborted => f.write_str("Aborted"),
            LocateError::BrokenPipe => f.write_str("Boken pipe"),
            LocateError::GlobPatternError(glob, err) => {
                f.write_fmt(format_args!("Glob pattern error for `{}`: {}", glob, err))
            }
            LocateError::Trivial => f.write_str("Trivial"),
        }
    }
}
