use crate::cli::CliError;
use std::io::Write;
use std::process::Command;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

pub(crate) fn print_version() {}

pub(crate) fn usage_cli() -> Result<(), CliError> {
    let usage = concat!(
        "Usage: fsidx [-h | -hh | -hhh | --help] [-v | --verbose] [-V | --version]\n",
        "             [-c <path> | --config-file <path>] <command> [<args>]\n",
        "       fsidx [<options>] update\n",
        "       fsidx [<options>] locate [<args>]\n",
        "       fsidx [<options>] shell\n",
        "       fsidx [<options>] help\n",
    );
    pretty_print_usage(usage)
}

pub(crate) fn help_cli_short() -> Result<(), CliError> {
    usage_cli()?;
    Ok(())
}

pub(crate) fn help_cli_long() -> Result<(), CliError> {
    let mut handle = Command::new("man").arg("fsidx").spawn()?;
    let _ = handle.wait();
    Ok(())
}

pub(crate) fn help_toml() -> Result<(), CliError> {
    let mut handle = Command::new("man").arg("fsidx.toml").spawn()?;
    let _ = handle.wait();
    Ok(())
}

pub(crate) fn help_shell_short() -> Result<(), CliError> {
    let help: &str = concat!(
        "Short-Cuts:\n",
        "    Ctrl-C       Interrupt printing results\n",
        "    Ctrl-D       Terminate application\n",
        "\n",
        "Commands:\n",
        "    plain text   Print database entries containing the strings\n",
        "    *.flac       Print database entries matching the glob\n",
        "    \\h           Print detailed help\n",
        "\n",
    );
    pretty_print_help(help)
}

pub(crate) fn help_shell_long() -> Result<(), CliError> {
    let help: &str = concat!(
        "Short-Cuts:\n",
        "    Ctrl-C       Interrupt printing results\n",
        "    Ctrl-D       Terminate application\n",
        "\n",
        "Commands:\n",
        "    plain text          Print database entries containing the strings\n",
        "    *.flac              Print database entries matching the glob\n",
        "    \\h                  Print detailed help\n",
        "    \\q                  Terminate application\n",
        "    \\o nnn.             Open query result\n",
        "    \\o nnn.-mmm.        Open query result\n",
        "    \\o *.jpg            Open matching query results\n",
        "    \\o nnn./path/*.jpg  Open matching quey results\n",
        "    \\u                  Scan folders and update database\n",
        "\n",
        "Options:\n",
        "    -c | --case_sensitive    Case-sensitive matching\n",
        "    -i | --case_insensitive  Case-insensitive matching (default)\n",
        "    -0 | --auto              Argument type is autodetected\n",
        "    -1 | --plain             Arguments are plain text\n",
        "    -2 | --glob              Arguments are glob pattern\n",
        "\n",
        "Options for plain text:\n",
        "    -a | --any_order         Plain text may match in any order (default)\n",
        "    -s | --same_order        Plain text must appear in same order\n",
        "    -w | --whole_path        Pattern is applied on whole path (default)\n",
        "    -l | --last_element      Pattern is applied on last element\n",
        "    -s | --smart_spaces      Space, dash and underscore match each other (default)\n",
        "    -S | --no_smart_spaces   Spaces only match with spaces\n",
        "    -b | --word_boundary     Plain text \n",
        "    -B | --no_word_boundary  (default)\n",
        "\n",
        "Options for glob patterns:\n",
        "    --ls | --literal_separator      Asterisk does not match a slash\n",
        "    --nls | --no_literal_separator  Asterisk matches any character (default)\n",
        "\n",
    );
    pretty_print_help(help)
}

fn pretty_print_usage(usage: &str) -> Result<(), CliError> {
    let mut stdout = StandardStream::stdout(ColorChoice::Auto);
    let mut start = 0;
    let mut pos = 0;
    let mut green = false;
    let mut quote = false;
    for ch in usage.chars() {
        let len = ch.len_utf8();
        if ch == ':' {
            pos += len;
            stdout.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)))?;
            stdout.write_all(&usage[start..pos].as_bytes())?;
            stdout.set_color(&ColorSpec::new())?;
            start = pos;
        } else if (ch.is_alphanumeric() || ch == '-') && !quote {
            if !green {
                stdout.write_all(&usage[start..pos].as_bytes())?;
                start = pos;
                green = true;
            }
            pos += len;
        } else {
            if green {
                stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)))?;
                stdout.write_all(&usage[start..pos].as_bytes())?;
                stdout.set_color(&ColorSpec::new())?;
                start = pos;
                green = false;
            }
            pos += len;
            if ch == '<' {
                quote = true;
            } else if ch == '>' {
                quote = false;
            }
        }
    }
    // No green here. Text is expected to end with newline.
    stdout.write_all(&usage[start..pos].as_bytes())?;
    Ok(())
}

fn pretty_print_help(help: &str) -> Result<(), CliError> {
    let mut stdout = StandardStream::stdout(ColorChoice::Auto);
    for line in help.lines() {
        if line.ends_with(':') {
            stdout.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)))?;
            stdout.write_all(&line.as_bytes())?;
            stdout.set_color(&ColorSpec::new())?;
            stdout.write_all(b"\n")?;
            continue;
        } else {
            if let Some((pos, _)) = line.char_indices().nth(4) {
                if let Some(pos2) = line[pos..].find("  ") {
                    stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)))?;
                    stdout.write_all(&line[0..pos + pos2].as_bytes())?;
                    stdout.set_color(&ColorSpec::new())?;
                    stdout.write_all(&line[pos + pos2..].as_bytes())?;
                    stdout.write_all(b"\n")?;
                    continue;
                }
            }
        }
        stdout.write_all(&line.as_bytes())?;
        stdout.write_all(b"\n")?;
    }
    Ok(())
}
