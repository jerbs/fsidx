use crate::cli::CliError;
use crate::config::{get_volume_info, Config};
use fsidx::Settings;
use std::env::Args;
use std::io::{stderr, stdout, Write};
use std::os::unix::prelude::OsStrExt;

pub(crate) fn update_cli(config: &Config, args: &mut Args) -> Result<(), CliError> {
    if let Some(arg) = args.next() {
        return Err(CliError::InvalidUpdateArgument(arg));
    }
    update_shell(config)
}

pub(crate) fn update_shell(config: &Config) -> Result<(), CliError> {
    let volume_info = get_volume_info(config).ok_or(CliError::NoDatabasePath)?;
    fsidx::update(volume_info, Settings::WithFileSizes, |event| {
        match event {
            fsidx::UpdateEvent::Scanning(path) => {
                stdout().write_all(b"Scanning: ")?;
                stdout().write_all(path.as_os_str().as_bytes())?;
                stdout().write_all(b"\n")?;
            }
            fsidx::UpdateEvent::ScanningFinished(path) => {
                stdout().write_all(b"Finished: ")?;
                stdout().write_all(path.as_os_str().as_bytes())?;
                stdout().write_all(b"\n")?;
            }
            fsidx::UpdateEvent::ScanningFailed(path) => {
                stderr().write_all(b"Error: Scanning failed: ")?;
                stderr().write_all(path.as_os_str().as_bytes())?;
                stderr().write_all(b"\n")?;
            }
            fsidx::UpdateEvent::DbWriteError(path, error) => {
                stderr().write_all(b"Error: Writing database \'")?;
                stderr().write_all(path.as_os_str().as_bytes())?;
                stderr().write_fmt(format_args!("\' failed: {}\n", error))?;
            }
            fsidx::UpdateEvent::ReplacingDatabaseFailed(tmp_path, path, error) => {
                stderr().write_all(b"Error: Replacing database \'")?;
                stderr().write_all(path.as_os_str().as_bytes())?;
                stderr().write_fmt(format_args!("\' with \'"))?;
                stderr().write_all(tmp_path.as_os_str().as_bytes())?;
                stderr().write_fmt(format_args!("\' failed: {}\n", error))?;
            }
            fsidx::UpdateEvent::RemovingTemporaryFileFailed(path, error) => {
                stderr().write_all(b"Error: Removing temporary file \'")?;
                stderr().write_all(path.as_os_str().as_bytes())?;
                stderr().write_fmt(format_args!("\' failed: {}\n", error))?;
            }
            fsidx::UpdateEvent::CreatingTemporaryFileFailed(path, error) => {
                stderr().write_all(b"Error: Creating temporary file \'")?;
                stderr().write_all(path.as_os_str().as_bytes())?;
                stderr().write_fmt(format_args!("\' failed: {}\n", error))?;
            }
            fsidx::UpdateEvent::ScanError(path, walk_dir_error) => {
                let depth = walk_dir_error.depth();
                stderr().write_all(b"Error: Scanning directory failed \'")?;
                stderr().write_all(path.as_os_str().as_bytes())?;
                stderr().write_fmt(format_args!("\' failed at depth {}", depth))?;
                if let Some(io_error) = walk_dir_error.io_error() {
                    stderr().write_fmt(format_args!("\': {}\n", io_error))?;
                } else {
                    stderr().write_all(b"\'.\n")?;
                }
                if let Some(associated_path) = walk_dir_error.path() {
                    stderr().write_all(b"       Associated path: \'")?;
                    stderr().write_all(associated_path.as_os_str().as_bytes())?;
                    stderr().write_all(b"\'\n")?;
                }
                if let Some(cycle_path) = walk_dir_error.loop_ancestor() {
                    stderr().write_all(b"       Cycle detected at path: \'")?;
                    stderr().write_all(cycle_path.as_os_str().as_bytes())?;
                    stderr().write_all(b"\'\n")?;
                }
            }
        };
        Ok(())
    });
    Ok(())
}
