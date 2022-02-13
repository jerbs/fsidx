use fastvlq::ReadVu64Ext;
use std::convert::TryFrom;
use std::ffi::OsStr;
use std::fs::File;
use std::io::{BufReader, Error, ErrorKind, Read, Result};
use std::os::unix::prelude::OsStrExt;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use crate::{Settings, VolumeInfo, FilterToken, filter};

// use nix::sys::termios::LocalFlags;
// use std::io::Write;
// let _ = writeln!(sink.stdout, "Searching: {}", vi.folder.display());
// let _ = sink.stderr.write_fmt(format_args!("Searching '{}' failed: {}", vi.folder.display(), error));
// let _ = sink.stdout.write(b"CTRL-C\n");

pub enum LocateResult<'a> {
    Entry(&'a Path, &'a Metadata),
    Finished,
    Interrupted,
    Searching(&'a Path),
    SearchingFinished(&'a Path),
    SearchingFailed(&'a Path, &'a Error),
}

pub struct Metadata {
    pub size: Option<u64>,
}

pub fn locate<F: FnMut(LocateResult)->Result<()>>(volume_info: Vec<VolumeInfo>, filter: Vec<FilterToken>, interrupt: Option<Arc<AtomicBool>>, mut f: F) -> Result<()> {
    for vi in &volume_info {
        f(LocateResult::Searching(&vi.folder))?;
        if let Err(error) = locate_volume(vi, &filter, &interrupt, &mut f) {
            match error.kind() {
                ErrorKind::Interrupted => {return Err(error);},
                ErrorKind::BrokenPipe => {return Err(error);},
                _ => {f(LocateResult::SearchingFailed(&vi.folder, &error))?;},
            } 
        }
    }
    Ok(())
}

pub fn locate_volume<F: FnMut(LocateResult)->Result<()>>(volume_info: &VolumeInfo, filter: &Vec<FilterToken>, interrupt: &Option<Arc<AtomicBool>>, f: &mut F) -> Result<()> {    
    let mut reader = FileIndexReader::new(&volume_info.database)?;
    let filter = filter::compile(&filter);
    loop {
        if interrupt.as_ref().map(|v| v.load(Ordering::Relaxed)).unwrap_or(false) {
            return Err(Error::new(ErrorKind::Interrupted, "interrupted"));
        }
        match reader.next() {
            Ok(Some((path, metadata))) => {
                let bytes = path.as_os_str().as_bytes();
                let text = String::from_utf8_lossy(bytes);
                if filter::apply(&text, &filter) {
                    f(LocateResult::Entry(path, &metadata))?;
                }
            },
            Ok(None) => return Ok(()),
            Err(err) => return Err(err),
        }
    };
}

struct FileIndexReader {
    reader: BufReader<File>,
    path: Vec<u8>,
    settings: Settings,
}

impl FileIndexReader {
    fn new(database: &Path) -> Result<FileIndexReader>
    {
        let file = File::open(database)?;
        let mut reader = BufReader::new(file);
        let mut fourcc: [u8; 4] = [0; 4];
        reader.read_exact(&mut fourcc)?;
        if fourcc != "fsix".as_bytes() {
            return Err(Error::new(ErrorKind::InvalidData, "Expected fsidx file."));
        }
        let mut flags: [u8; 1] = [0; 1];
        reader.read_exact(&mut flags)?;
        let settings = Settings::try_from(flags[0])
        .map_err(|_err| Error::new(ErrorKind::InvalidData, "Unsupported file format"))?;
        let path: Vec<u8> = Vec::new();
        Ok(FileIndexReader { reader, path, settings } )
    }

    fn next(&mut self) -> Result<Option<(&Path, Metadata)>> {
        let discard = match self.reader.read_vu64() {
            Ok(val) => val,
            Err(err) => {
                match err.kind() {
                    ErrorKind::UnexpectedEof => return Ok(None),
                    _ => return Err(err),
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
