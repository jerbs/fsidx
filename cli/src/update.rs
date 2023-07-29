use std::env::Args;
use fsidx::{Settings, UpdateSink};
use std::io::{stdout, stderr};
use crate::cli::CliError;
use crate::config::{Config, get_volume_info};


pub(crate) fn update_cli(config: &Config, args: &mut Args) -> Result<(), CliError> {
    if let Some(arg) = args.next() {
        return Err(CliError::InvalidUpdateArgument(arg));
    }
    update_shell(config)
}

pub(crate) fn update_shell(config: &Config) -> Result<(), CliError> {
    let volume_info = get_volume_info(&config)
    .ok_or(CliError::NoDatabasePath)?;
    let sink = UpdateSink {
        stdout: &mut stdout(),
        stderr: &mut stderr(),
    };
    fsidx::update(volume_info, Settings::WithFileSizes, sink);
    Ok(())
}
