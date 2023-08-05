use crate::cli::CliError;
use globset::GlobBuilder;
use nom::IResult;
use std::fmt::Debug;
use std::path::{Path, PathBuf};
use std::str::FromStr;

// Selection refers to the indexed list with the last query result.
// idx.           -- Open single file from selection
// idx.-idx.      -- Opens range of files from selection
// glob           -- Opens all matching files from selection
// idx./path/glob -- Opens all matching files from selection

pub struct Expand<'a> {
    open_rule: OpenRule,
    selection: &'a Vec<PathBuf>,
}

impl<'a> Expand<'a> {
    pub fn new(open_rule: OpenRule, selection: &'a Vec<PathBuf>) -> Expand<'a> {
        Expand {
            open_rule,
            selection,
        }
    }

    pub(crate) fn foreach<F: FnMut(&Path) -> Result<(), CliError>>(
        &self,
        mut f: F,
    ) -> Result<(), CliError> {
        match &self.open_rule {
            OpenRule::Glob(glob) => expand_glob(glob, self.selection, &mut f),
            OpenRule::Index(index) => expand_index(*index, self.selection, &mut f),
            OpenRule::IndexRange(start, end) => {
                expand_index_range(*start, *end, self.selection, &mut f)
            }
            OpenRule::IndexGlob(index, glob) => {
                expand_index_with_glob(*index, glob, self.selection, &mut f)
            }
        }
    }
}

// idx.           -- Open single file from selection
fn expand_index<F: FnMut(&Path) -> Result<(), CliError>>(
    index: usize,
    selection: &Vec<PathBuf>,
    f: &mut F,
) -> Result<(), CliError> {
    let path = selection
        .get(index - 1)
        .ok_or(CliError::InvalidOpenIndex(index))?;
    f(path)
}

// idx.-idx.      -- Opens range of files from selection
fn expand_index_range<F: FnMut(&Path) -> Result<(), CliError>>(
    start: usize,
    end: usize,
    selection: &Vec<PathBuf>,
    f: &mut F,
) -> Result<(), CliError> {
    for index in start..=end {
        expand_index(index, selection, f)?;
    }
    Ok(())
}

// glob           -- Opens all matching files from selection
fn expand_glob<F: FnMut(&Path) -> Result<(), CliError>>(
    glob: &str,
    selection: &Vec<PathBuf>,
    f: &mut F,
) -> Result<(), CliError> {
    let glob_set = GlobBuilder::new(glob)
        .case_insensitive(true) // FIXME: Make this configurable.
        .literal_separator(false) // FIXME: Make this configurable.
        .backslash_escape(true)
        .empty_alternates(true)
        .build()
        .map_err(|err| CliError::GlobPatternError(glob.to_string(), err))?
        .compile_matcher();
    for path in selection {
        if glob_set.is_match(path) {
            f(path)?;
        }
    }
    Ok(())
}

// idx./path/glob -- Opens all matching files from selection
fn expand_index_with_glob<F: FnMut(&Path) -> Result<(), CliError>>(
    index: usize,
    glob: &str,
    selection: &Vec<PathBuf>,
    f: &mut F,
) -> Result<(), CliError> {
    let Some(path) = selection.get(index) else {
        return Err(CliError::InvalidOpenIndex(index));
    };
    let Some(path) = path.to_str() else {
        return Err(CliError::NotImplementedForNonUtf8Path(path.to_path_buf()));
    };
    let mut glob2 = String::from(path);
    glob2.push_str("/");
    glob2.push_str(glob);
    let glob2 = normalize(glob2);
    expand_glob(glob2.as_str(), selection, f)?;
    Ok(())
}

#[derive(PartialEq)]
pub enum OpenRule {
    Glob(String),
    Index(usize),
    IndexRange(usize, usize),
    IndexGlob(usize, String),
}

#[derive(PartialEq)]
pub enum ParseError {
    Invalid(String),
}

impl From<nom::error::Error<&str>> for ParseError {
    fn from(err: nom::error::Error<&str>) -> Self {
        ParseError::Invalid(err.to_string())
    }
}

impl FromStr for OpenRule {
    type Err = ParseError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        use nom::Finish;
        let (_rest, open_rule) = parse_open_rule(s).finish()?;
        Ok(open_rule)
    }
}

impl Debug for OpenRule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Index(index) => f.debug_tuple("Index").field(index).finish(),
            Self::IndexRange(start, end) => {
                f.debug_tuple("IndexRange").field(start).field(end).finish()
            }
            Self::Glob(glob) => f.debug_tuple("Glob").field(glob).finish(),
            Self::IndexGlob(index, glob) => f
                .debug_tuple("IndexWithGlob")
                .field(index)
                .field(glob)
                .finish(),
        }
    }
}

impl Debug for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            _ => {
                f.write_str("ParseError")?;
            }
        }
        Ok(())
    }
}

fn parse_open_rule(input: &str) -> IResult<&str, OpenRule> {
    use nom::branch::alt;
    use nom::bytes::complete::tag;
    use nom::character::complete::u64;
    use nom::combinator::{all_consuming, map, rest};
    use nom::sequence::tuple;
    all_consuming(alt((
        map(
            tuple((u64::<&str, _>, tag("./"), rest)),
            |(idx, _, glob)| OpenRule::IndexGlob(idx as usize, glob.to_string()),
        ),
        map(
            tuple((u64, tag(".-"), u64, tag("."))),
            |(start, _, end, _)| OpenRule::IndexRange(start as usize, end as usize),
        ),
        map(tuple((u64, tag("."))), |(idx, _)| {
            OpenRule::Index(idx as usize)
        }),
        map(rest::<&str, _>, |glob| OpenRule::Glob(glob.to_string())),
    )))(input)
}

fn normalize(mut glob: String) -> String {
    loop {
        if let Some(pos2) = glob.find("/../") {
            if let Some(pos1) = glob[0..pos2].rfind("/") {
                glob.replace_range(pos1 + 1..pos2 + 4, "");
            } else {
                break;
            }
        } else {
            break;
        }
    }
    glob
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glob() {
        assert_eq!("*.jpg".parse(), Ok(OpenRule::Glob("*.jpg".to_string())))
    }

    #[test]
    fn index() {
        assert_eq!("123.".parse(), Ok(OpenRule::Index(123)));
    }

    #[test]
    fn index_range() {
        assert_eq!("123.-456.".parse(), Ok(OpenRule::IndexRange(123, 456)));
    }

    #[test]
    fn invalid_index() {
        assert_eq!(
            "123".parse::<OpenRule>(),
            Ok(OpenRule::Glob("123".to_string()))
        );
    }

    #[test]
    fn index_and_glob() {
        assert_eq!(
            "423./../*.flac".parse::<OpenRule>(),
            Ok(OpenRule::IndexGlob(423, "../*.flac".to_string()))
        );
    }

    #[test]
    fn test_normalize() {
        let path = String::from("/abc/../foo/bar/baz/../../*.jpg");
        let path = normalize(path);
        assert_eq!(path.as_str(), "/foo/*.jpg");
    }
}
