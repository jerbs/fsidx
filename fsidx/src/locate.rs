use fastvlq::ReadVu64Ext;
use std::ffi::OsStr;
use std::fs::File;
use std::io::{BufReader, Error, ErrorKind, Read, Result, stdout, Write};
use std::os::unix::prelude::OsStrExt;
use std::path::Path;
use super::{filter, FilterToken};
use super::VolumeInfo;

pub fn locate(volume_info: Vec<VolumeInfo>, filter: Vec<FilterToken>) {
    for vi in &volume_info {
        println!("Searching: {}", vi.folder.display());
        if let Err(error) = locate_volume(vi, &filter) {
            eprintln!("Searching '{}' failed: {}", vi.folder.display(), error);
        }
    }
}

fn locate_volume(volume_info: &VolumeInfo, filter: &Vec<FilterToken> ) -> Result<()> {    
    let mut reader = FileIndexReader::new(&volume_info.database)?;
    let filter = filter::compile(&filter);
    loop {
        match reader.next() {
            Ok(Some(path)) => {
                let bytes = path.as_os_str().as_bytes();
                let text = String::from_utf8_lossy(bytes);
                if filter::apply(&text, &filter) {
                    let stdout = stdout();
                    let mut stdout = stdout.lock();
                    stdout.write_all(bytes)?;
                    stdout.write_all(b"\n")?;   
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
        let path: Vec<u8> = Vec::new();
        Ok(FileIndexReader { reader, path } )
    }

    fn next(&mut self) -> Result<Option<&Path>> {
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
        let path = Path::new(OsStr::from_bytes(self.path.as_slice()));        
        Ok(Some(path))
    }
}

fn delta_decode(path: &mut Vec<u8>, discard: u64, delta: &[u8]) {
    let len = path.len();
    let reuse = len - (discard as usize);
    path.splice(reuse..len, delta.iter().cloned());
}
