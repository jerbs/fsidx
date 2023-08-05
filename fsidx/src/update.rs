use super::{Settings, VolumeInfo};
use core::cmp::Ordering;
use fastvlq::WriteVu64Ext;
use nix::sys::stat::stat;
use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::{Error, ErrorKind, Result, Write};
use std::path::Path;
use std::sync::mpsc::{channel, Sender};
use std::thread::{self};
use walkdir::WalkDir;

type GroupedVolumes = Vec<Vec<VolumeInfo>>;

pub struct UpdateSink<'a> {
    pub stdout: &'a mut dyn Write,
    pub stderr: &'a mut dyn Write,
}

enum Msg {
    Info(String),
    Error(String),
}

pub fn update(volume_info: Vec<VolumeInfo>, settings: Settings, sink: UpdateSink) {
    let grouped = group_volumes(volume_info);
    let mut handles = vec![];
    let (tx, rx) = channel();
    for group in grouped {
        let settings = settings.clone();
        let tx = tx.clone();
        let handle = thread::spawn(|| {
            update_volume_group(group, settings, tx);
        });
        handles.push(handle);
    }
    drop(tx);
    loop {
        let recv = rx.recv();
        match recv {
            Ok(Msg::Info(text)) => {
                let _ = writeln!(sink.stdout, "{}", text);
            }
            Ok(Msg::Error(text)) => {
                let _ = writeln!(sink.stderr, "Error: {}", text);
            }
            Err(_) => {
                break;
            }
        };
    }
    for handle in handles {
        handle.join().expect("join failed");
    }
}

fn group_volumes(volume_info: Vec<VolumeInfo>) -> GroupedVolumes {
    let mut map = BTreeMap::<i32, Vec<VolumeInfo>>::new();

    for vi in volume_info {
        let st = stat(&vi.folder);
        if let Ok(f_stat) = st {
            let dev = f_stat.st_dev;
            map.entry(dev).or_default().push(vi);
        }
    }

    map.values().cloned().collect()
}

fn update_volume_group(group: Vec<VolumeInfo>, settings: Settings, tx: Sender<Msg>) {
    for volume_info in group {
        update_volume(volume_info, settings.clone(), &tx);
    }
}

fn update_volume(volume_info: VolumeInfo, settings: Settings, tx: &Sender<Msg>) {
    let _ = tx.send(Msg::Info(format!(
        "Scanning: {}",
        volume_info.folder.display()
    )));

    if let Err(err) = update_volume_impl(&volume_info, settings, &tx) {
        let _ = tx.send(Msg::Error(format!("Error: {}", err)));
        let _ = tx.send(Msg::Error(format!(
            "Scanning failed: {}",
            volume_info.folder.display()
        )));
    } else {
        let _ = tx.send(Msg::Info(format!(
            "Finished: {}",
            volume_info.folder.display()
        )));
    }
}

fn update_volume_impl(
    volume_info: &VolumeInfo,
    settings: Settings,
    tx: &Sender<Msg>,
) -> Result<()> {
    let db_file_name = &volume_info.database;
    let mut tmp_file_name = db_file_name.clone();
    tmp_file_name.set_extension("~");

    let mut file = File::create(&tmp_file_name)?;
    let result = scan_folder(&mut file, &volume_info.folder, settings, &tx);
    drop(file); // close file

    match result {
        Ok(_) => fs::rename(&tmp_file_name, &db_file_name)?,
        Err(_) => fs::remove_file(&tmp_file_name)?,
    }

    result
}

fn scan_folder(
    mut writer: &mut dyn Write,
    folder: &Path,
    settings: Settings,
    tx: &Sender<Msg>,
) -> Result<()> {
    let flags: &[u8] = &[settings.clone() as u8];

    // The written file should be removed when this function returns an Err.
    // Either the device was not mounted (ErrorKind::NotFound) or writing the
    // file failed, i.e. the file content is corrupt.
    writer.write_all("fsix".as_bytes())?;
    writer.write_all(flags)?;
    let mut previous: Vec<u8> = Vec::new();
    for entry in WalkDir::new(folder).sort_by(|a, b| compare(a.file_name(), b.file_name())) {
        match entry {
            Ok(entry) => {
                let bytes = byte_slice(entry.path());
                let (discard, delta) = delta_encode(&previous, bytes);

                // println!("{}: {}", discard, String::from_utf8_lossy(delta));
                // println!("{}: {}", bytes.len(), entry.path().display());

                writer.write_vu64(discard as u64)?;
                writer.write_vu64(delta.len() as u64)?;
                writer.write_all(&delta)?;

                if settings == Settings::WithFileSizes {
                    let size_plus_one = if let Ok(metadata) = entry.metadata() {
                        metadata.len() + 1
                    } else {
                        0
                    };
                    writer.write_vu64(size_plus_one)?;
                }

                previous = bytes.to_vec();
            }
            Err(error) => {
                let depth = error.depth();
                if let Some(io_error) = error.io_error() {
                    // capture.error(&format!("io error: {:?}", io_error.kind()));
                    if io_error.kind() == std::io::ErrorKind::NotFound && depth == 0 {
                        // The toplevel entry directory does not exist.
                        // Assuming that the device is not mounted.
                        // Stop scanning and remove the temporary TPdb file.
                        return Err(Error::new(ErrorKind::NotFound, "Device not mounted"));

                        // Note: I have seen the NotFound error for netatalk mounted directory
                        //       name with non ascii characters.
                    }
                }
                match error.path() {
                    Some(path) => {
                        let _ = tx.send(Msg::Error(format!(
                            "Error: {} on path {}",
                            error,
                            path.display()
                        )));
                    }
                    None => {
                        let _ = tx.send(Msg::Error(format!("Error: {}", error)));
                    }
                }
            }
        }
    }
    Ok(())
}

fn compare(a: &OsStr, b: &OsStr) -> Ordering {
    let a1 = a.to_string_lossy();
    let b1 = b.to_string_lossy();
    natord::compare(&a1, &b1)
}

fn byte_slice(path: &Path) -> &[u8] {
    use std::os::unix::ffi::OsStrExt; // Import OsStrExt trait for OsStr to get as_bytes()
    let os_str = path.as_os_str();
    let bytes: &[u8] = os_str.as_bytes(); // as_bytes() is Unix specific
    bytes
}

fn delta_encode<'a>(a: &'a [u8], b: &'a [u8]) -> (usize, &'a [u8]) {
    let mut idx: usize = 0;
    for (a, b) in a.iter().zip(b.iter()) {
        if a != b {
            break;
        }
        idx = idx + 1;
    }
    let discard = a.len() - idx;
    let delta: &[u8] = &b[idx..];
    (discard, delta)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;

    #[test]
    fn test_compare() {
        assert_eq!(
            compare(&OsString::from("foo"), &OsString::from("foo")),
            Ordering::Equal
        );
        assert_eq!(
            compare(&OsString::from("foo2"), &OsString::from("foo10")),
            Ordering::Less
        );
        // lexical_sort::natural_lexical_cmp panicks with these large numbers:
        assert_eq!(
            compare(
                &OsString::from("foo123456789012345678901234"),
                &OsString::from("foo123456789012345678901234")
            ),
            Ordering::Equal
        );
        assert_eq!(
            compare(
                &OsString::from("foo23456789012345678901234"),
                &OsString::from("foo123456789012345678901234")
            ),
            Ordering::Less
        );
    }
}
