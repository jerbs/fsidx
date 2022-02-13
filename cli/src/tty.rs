use nix::sys::termios::{self, LocalFlags, SetArg};
use std::os::unix::io::RawFd;
use std::io::Result;

const STDIN_FILENO: RawFd = libc::STDIN_FILENO;

pub fn set_tty() -> Result<()> {
    let original_mode = termios::tcgetattr(STDIN_FILENO)?;
    let mut raw = original_mode.clone();

    // Disable ECHO.
    // Without ECHO CTRL-C does not print '^C':
    raw.local_flags &= !(LocalFlags::ECHO);

    // Disable flush after interrupt or quit.
    // With flush enabled CTRL-C clear stdout buffer, but application still writes into the buffer.
    // I.e. some characters/lines are missing and output is corrupted.
    raw.local_flags |= LocalFlags::NOFLSH;

    termios::tcsetattr(STDIN_FILENO, SetArg::TCSADRAIN, &raw)?;

    Ok(())
}
