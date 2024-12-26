use super::{Settings, VolumeInfo};
use core::cmp::Ordering;
use fastvlq::WriteVu64Ext;
use nix::sys::stat::stat;
use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::{Error, Result as IOResult, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Sender};
use std::thread::{self};
use walkdir::WalkDir;

type GroupedVolumes = Vec<Vec<VolumeInfo>>;

/// UpdateEvent indicate events to a callback function.
#[derive(Debug)]
pub enum UpdateEvent {
    /// Starts scanning a configured folder.
    Scanning(PathBuf),
    /// Finishs scanning a configured folder.
    ScanningFinished(PathBuf),
    /// Scanning failed. Database for this folder was not updated.
    ScanningFailed(PathBuf),
    /// Writing the database file failed.
    DbWriteError(PathBuf, Error),
    /// Moving the temporary database file to its final location failed.
    ReplacingDatabaseFailed(PathBuf, PathBuf, Error),
    /// Removing the temporary database file failed.
    RemovingTemporaryFileFailed(PathBuf, Error),
    /// Creating the temporary database file failed.
    CreatingTemporaryFileFailed(PathBuf, Error),
    /// Scanning the directory tree failed.
    ScanError(PathBuf, walkdir::Error),
}

/// The update function recursively scans multiple folders and updates database
/// files with the retrieved information.
///
/// Settings define which information is written into the databas files.
///
/// The implementations uses multiple threads to scan folders on different
/// physical devices in parallel.
///
/// The provided closure is used to notify the caller about the scanning state
/// and error.
pub fn update<F: FnMut(UpdateEvent) -> IOResult<()>>(
    volume_info: Vec<VolumeInfo>,
    settings: Settings,
    mut f: F,
) {
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
            Ok(event) => {
                let _ = f(event);
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
    let mut map = BTreeMap::<_, Vec<VolumeInfo>>::new();
    for vi in volume_info {
        let st = stat(&vi.folder);
        if let Ok(f_stat) = st {
            let dev = f_stat.st_dev; // MacOS: i32, Linux: u64
            map.entry(dev).or_default().push(vi);
        }
    }
    map.values().cloned().collect()
}

fn update_volume_group(group: Vec<VolumeInfo>, settings: Settings, tx: Sender<UpdateEvent>) {
    for volume_info in group {
        update_volume(volume_info, settings.clone(), &tx);
    }
}

fn update_volume(volume_info: VolumeInfo, settings: Settings, tx: &Sender<UpdateEvent>) {
    let _ = tx.send(UpdateEvent::Scanning(volume_info.folder.clone()));
    if update_volume_impl(&volume_info, settings, tx) {
        // Database file is updated.
        let _ = tx.send(UpdateEvent::ScanningFinished(volume_info.folder.clone()));
    } else {
        // Database file is not updated.
        let _ = tx.send(UpdateEvent::ScanningFailed(volume_info.folder.clone()));
    }
}

fn update_volume_impl(
    volume_info: &VolumeInfo,
    settings: Settings,
    tx: &Sender<UpdateEvent>,
) -> bool {
    let db_file_name = &volume_info.database;
    let mut tmp_file_name = db_file_name.clone();
    tmp_file_name.set_extension("~");

    let mut file = match File::create(&tmp_file_name) {
        Ok(file) => file,
        Err(err) => {
            let _ = tx.send(UpdateEvent::CreatingTemporaryFileFailed(tmp_file_name, err));
            return false;
        }
    };
    let result = scan_folder(&mut file, &volume_info.folder, settings, tx);
    drop(file); // close file

    match result {
        Ok(_) => {
            if let Err(err) = fs::rename(&tmp_file_name, db_file_name) {
                let _ = tx.send(UpdateEvent::ReplacingDatabaseFailed(
                    tmp_file_name,
                    db_file_name.clone(),
                    err,
                ));
                false
            } else {
                true
            }
        }
        Err(err) => {
            let _ = tx.send(UpdateEvent::DbWriteError(volume_info.database.clone(), err));
            if let Err(err) = fs::remove_file(&tmp_file_name) {
                let _ = tx.send(UpdateEvent::RemovingTemporaryFileFailed(tmp_file_name, err));
            }
            false
        }
    }
}

fn scan_folder(
    mut writer: &mut dyn Write,
    folder: &Path,
    settings: Settings,
    tx: &Sender<UpdateEvent>,
) -> IOResult<()> {
    // An Err(_) return value always indicates that writing the database file failed.
    // When scanning the folder fails the error is sent as an event.
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
                writer.write_all(delta)?;

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
                // This function is not called if a folder is not mounted.
                // Unmounted volumes are already filtered ou by group_volumes.
                let _ = tx.send(UpdateEvent::ScanError(folder.to_path_buf(), error));
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
        idx += 1;
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
