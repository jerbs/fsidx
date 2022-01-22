use fastvlq::ReadVu64Ext;
use std::fs::File;
use std::io::{BufReader, Error, ErrorKind, Read, Result, stdout, Write};
use super::FilterToken;
use super::VolumeInfo;

pub fn locate(volume_info: Vec<VolumeInfo>, filter_token: Vec<FilterToken>) {
    for vi in &volume_info {
        println!("Searching: {}", vi.folder.display());
        if let Err(error) = locate_volume(vi, &filter_token) {
            eprintln!("Searching '{}' failed: {}", vi.folder.display(), error);
        }
    }
}

fn locate_volume(volume_info: &VolumeInfo, _filter: &Vec<FilterToken>) -> Result<()> {    
    let file = File::open(&volume_info.database)?;
    let mut reader = BufReader::new(file);
    let mut fourcc: [u8; 4] = [0; 4];
    reader.read_exact(&mut fourcc)?;
    if fourcc != "fsix".as_bytes() {
        return Err(Error::new(ErrorKind::InvalidData, "Expected fsidx file."));
    }
    let mut path: Vec<u8> = Vec::new();
    loop {
        let discard = match reader.read_vu64() {
            Ok(val) => val,
            Err(err) => {
                match err.kind() {
                    ErrorKind::UnexpectedEof => break,
                    _ => return Err(err),
                }
            },
        };
        let length = reader.read_vu64()?;
        let mut delta = vec![0u8; length as usize];
        reader.read_exact(&mut delta)?;
        delta_decode(&mut path, discard, &delta);
        let stdout = stdout();
        let mut stdout = stdout.lock();
        stdout.write_all(&path)?;
        stdout.write_all(b"\n")?;
    }
    Ok(())
}

fn delta_decode(path: &mut Vec<u8>, discard: u64, delta: &[u8]) {
    let len = path.len();
    let reuse = len - (discard as usize);
    path.splice(reuse..len, delta.iter().cloned());
}
