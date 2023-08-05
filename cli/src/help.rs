use crate::cli::CliError;
use std::io::{Result as IOResult, Write};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

pub(crate) fn print_version() {}

pub(crate) fn usage_cli() -> Result<(), CliError> {
    println!("Usage...");
    Ok(())
}

pub(crate) fn help_cli() -> Result<(), CliError> {
    println!("Help...");
    Ok(())
}

pub(crate) fn help_shell_short() -> IOResult<()> {
    let indent = 20;
    let mut stdout = StandardStream::stdout(ColorChoice::Auto);
    write_flags(
        &mut stdout,
        &[r#"Ctrl-C"#],
        indent,
        "Interrupt printing results",
    )?;
    write_flags(&mut stdout, &[r#"Ctrl-D"#], indent, "Terminate application")?;
    write_section(&mut stdout, "Commands:")?;
    write_flags(&mut stdout, &[r#"\h"#], indent, "print detailed help")?;
    Ok(())
}

pub(crate) fn help_shell() -> Result<(), CliError> {
    help_shell_short()?;
    let indent = 20;
    let mut stdout = StandardStream::stdout(ColorChoice::Auto);
    write_flags(&mut stdout, &[r#"\q"#], indent, "quit application")?;
    write_flags(
        &mut stdout,
        &[r#"\o [id ...]"#],
        indent,
        "open files with id from last selection",
    )?;
    write_flags(&mut stdout, &[r#"\u"#], indent, "update database")?;

    write_section(&mut stdout, "Modes:")?;
    write_flags(
        &mut stdout,
        &["-m0", "-auto"],
        indent,
        "Auto detect mode (default)",
    )?;
    write_flags(
        &mut stdout,
        &["-m1", "-smart"],
        indent,
        "Enter smart searching mode",
    )?;
    write_flags(
        &mut stdout,
        &["-m2", "-glob"],
        indent,
        "Enter glob pattern mode",
    )?;

    write_section(&mut stdout, "Smart searching (-m1):")?;
    write_flags(
        &mut stdout,
        &[r#"text"#],
        indent,
        "Search for any path containing 'text'",
    )?;
    write_flags(
        &mut stdout,
        &[r#"foo bar"#],
        indent,
        "Path must contains all strings in any order",
    )?;
    write_flags(
        &mut stdout,
        &[r#""foo bar""#],
        indent,
        "Match text with spaces",
    )?;
    write_flags(
        &mut stdout,
        &[r#""ab cb""#],
        indent,
        r#"Match "ab cd", "ab-cd", "ab_cd" and "abcd""#,
    )?;
    write_flags(
        &mut stdout,
        &[r#"ab\ cb"#],
        indent,
        r#"Match "ab cd", "ab-cd", "ab_cd" and "abcd""#,
    )?;
    write_flags(
        &mut stdout,
        &[r#"ab-cb"#],
        indent,
        r#"Match "ab cd", "ab-cd", "ab_cd" and "abcd""#,
    )?;
    write_flags(
        &mut stdout,
        &[r#"ab_cb"#],
        indent,
        r#"Match "ab cd", "ab-cd", "ab_cd" and "abcd""#,
    )?;
    write_flags(
        &mut stdout,
        &[r#""\"""#, r#"""\""#],
        indent,
        "Match double quote",
    )?;
    write_flags(&mut stdout, &[r#""-""#, r#"""-"#], indent, "Match dash")?;
    write_flags(
        &mut stdout,
        &[r#""\\""#, r#"""\\"#],
        indent,
        "Match backslash",
    )?;

    write_section(&mut stdout, "Glob pattern searching (-m2):")?;
    write_flags(
        &mut stdout,
        &[r#"*.???"#],
        indent,
        "? matches any single character.",
    )?;
    write_flags(
        &mut stdout,
        &[r#"*.jpg"#],
        indent,
        "* matches any sequence of characters",
    )?;
    write_flags(
        &mut stdout,
        &[r#"/**/*.jpg"#],
        indent,
        "** matches any subdirectory",
    )?;
    write_flags(
        &mut stdout,
        &[r#"[abc]"#],
        indent,
        "[...] matches one of the characters inside the brackets",
    )?;
    write_flags(
        &mut stdout,
        &[r#"[a-zA-Z]"#],
        indent,
        "[.-.] matches one of the characters in the sequence",
    )?;
    write_flags(&mut stdout, &[r#"[!abc]"#], indent, "negation of [...]")?;
    write_flags(&mut stdout, &[r#"[?]"#], indent, "matches a ?")?;
    write_flags(&mut stdout, &[r#"[*]"#], indent, "matches a *")?;
    write_flags(&mut stdout, &[r#"[[]]"#], indent, "matches a [")?;
    write_flags(&mut stdout, &[r#"[]]"#], indent, "matches a ]")?;
    write_flags(
        &mut stdout,
        &[r#"-ls, -ls1, -require_literal_separator"#],
        indent + 25,
        "* does not match /",
    )?;
    write_flags(
        &mut stdout,
        &[r#"-ls0, -unrequire_literal_separator"#],
        indent + 25,
        "* does match /",
    )?;
    write_flags(
        &mut stdout,
        &[r#"-ld, -ld1, -require_literal_leading_dot"#],
        indent + 25,
        "* does not match a leading dot",
    )?;
    write_flags(
        &mut stdout,
        &[r#"-ld0, -unrequire_literal_leading_dot"#],
        indent + 25,
        "* does match a leading dot",
    )?;

    let indent = 30;
    write_section(&mut stdout, "Search options:")?;
    write_flags(
        &mut stdout,
        &["-c", "-case_sensitive"],
        indent,
        "Subsequent [text] arguments are matched case sensitive",
    )?;
    write_flags(
        &mut stdout,
        &["-i", "-case_insensitive"],
        indent,
        "Subsequent [text] arguments are matched case insensitive",
    )?;
    write_flags(
        &mut stdout,
        &["-a", "-any_order"],
        indent,
        "Subsequent [text] arguments may  match in any order",
    )?;
    write_flags(
        &mut stdout,
        &["-s", "-same_order"],
        indent,
        "Subsequent [text] arguments must match in the same order",
    )?;
    write_flags(
        &mut stdout,
        &["-w", "-whole_path"],
        indent,
        "Subsequent [text] arguments may  appear in the whole path",
    )?;
    write_flags(
        &mut stdout,
        &["-l", "-last_element"],
        indent,
        "Subsequent [text] arguments must appear in the last element only",
    )?;
    write_flags(
        &mut stdout,
        &["-", ""],
        indent,
        "Not an option. This matches a dash.",
    )?;

    let indent = 20;
    write_section(&mut stdout, "Open search results with index commands:")?;
    write_flags(
        &mut stdout,
        &[r#"12."#],
        indent,
        "Open single selected file",
    )?;
    write_flags(
        &mut stdout,
        &[r#"12.."#],
        indent,
        "Open all selected files in same directory",
    )?;
    write_flags(
        &mut stdout,
        &[r#"12..."#],
        indent,
        "Open all files in same directory",
    )?;
    write_flags(
        &mut stdout,
        &[r#"12.."#],
        indent,
        "Open all selected files in same directory with suffix",
    )?;
    write_flags(
        &mut stdout,
        &[r#"12...jpg"#],
        indent,
        "Open all files in same directory with suffix",
    )?;
    Ok(())
}

fn write_section(stdout: &mut StandardStream, text: &str) -> IOResult<()> {
    stdout.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)))?;
    writeln!(stdout, "\n{}", text)?;
    stdout.set_color(&ColorSpec::new())?;
    Ok(())
}

fn write_flags(
    stdout: &mut StandardStream,
    flags: &[&str],
    indent: usize,
    description: &str,
) -> IOResult<()> {
    let mut pos = 4;
    write!(stdout, "    ")?;
    for (index, flag) in flags.iter().enumerate() {
        stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)))?;
        write!(stdout, "{}", flag)?;
        pos += flag.chars().count();
        stdout.set_color(&ColorSpec::new())?;
        if index + 1 != flags.len() {
            write!(stdout, ", ")?;
            pos += 2;
        }
    }
    while pos < indent {
        write!(stdout, " ")?;
        pos += 1;
    }
    writeln!(stdout, "{}", description)?;
    Ok(())
}
