use nix::sys::termios::{self, LocalFlags, SetArg};
use std::io::Result;

pub fn set_tty() -> Result<()> {
    let original_mode = termios::tcgetattr(std::io::stdin())?;
    let mut raw = original_mode.clone();

    // Disable ECHO.
    // Without ECHO CTRL-C does not print '^C':
    raw.local_flags &= !(LocalFlags::ECHO);

    // Disable flush after abort or quit.
    // With flush enabled CTRL-C clear stdout buffer, but application still writes into the buffer.
    // I.e. some characters/lines are missing and output is corrupted.
    raw.local_flags |= LocalFlags::NOFLSH;

    termios::tcsetattr(std::io::stdin(), SetArg::TCSADRAIN, &raw)?;

    Ok(())
}
