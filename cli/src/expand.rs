use std::ffi::OsString;
use std::fmt::Debug;
use std::io::{Error, ErrorKind, Result};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use fsidx::FileIndexReader;

use crate::config::{Config, get_volume_info};

// 421.        -- Open single selected file
// 421..       -- Open all selected files in same directory
// 421...      -- Open all files in same directory
// 421..jpg    -- Open all selected files in same directory with suffix
// 421...jpg   -- Open all files in same directory with suffix

pub struct Expand<'a> {
    config: &'a Config,
    match_rule: MatchRule,
    selection: &'a Vec<PathBuf>,
}

impl<'a> Expand<'a> {
    pub fn new(config: &'a Config, match_rule: MatchRule, selection: &'a Vec<PathBuf>) -> Expand<'a> {
        Expand {
            config,
            match_rule,
            selection,
        }
    }

    pub fn foreach<F: FnMut(&Path)->Result<()>>(&self, f: F) -> Result<()> {
        match &self.match_rule {
            MatchRule::Index(index) => expand_single(*index, self.selection, f),
            MatchRule::DirectoryOfSelection(index) => expand_directory_of_selection(*index, self.selection, f),
            MatchRule::DirectoryOfFileIndex(index) => expand_directory_of_file_index(*index, self.selection, self.config, f),
            MatchRule::DirectoryOfSelectionWithSuffix(index, suffix) => expand_directory_of_selection_with_suffix(*index, suffix, self.selection, f),
            MatchRule::DirectoryOfFileIndexWithSuffix(index, suffix) => expand_directory_of_file_index_with_suffix(*index, suffix, self.selection, self.config, f),
        }
    } 
}

// MatchRule::Index(index)
fn expand_single<F: FnMut(&Path)->Result<()>>(index: usize, selection: & Vec<PathBuf>, mut f: F) -> Result<()> {
    let path = selection
    .get(index - 1)
    .map(|v| v.as_ref());
    if let Some(path) = path {
        f(path)?;
    }
    Ok(())
}

// MatchRule::DirectoryOfSelection(index)
fn expand_directory_of_selection<F: FnMut(&Path)->Result<()>>(index: usize, selection: &Vec<PathBuf>, mut f: F) -> Result<()> {
    let sel_path = selection.get(index - 1).ok_or(Error::new(ErrorKind::InvalidData, "Invalid index"))?;
    let sel_dir = sel_path.parent().ok_or(Error::new(ErrorKind::InvalidData, "No parent"))?;
    for path in selection {
        if path.starts_with(sel_dir) {
            f(path)?;
        }
    }
    Ok(())
}

// MatchRule::DirectoryOfFileIndex(index)
fn expand_directory_of_file_index<F: FnMut(&Path)->Result<()>>(index: usize, selection: &Vec<PathBuf>, config: &Config, mut f: F) -> Result<()> {
    if let Some(volume_info) = get_volume_info(config) {
        for volume_info in volume_info {
            let dir = if let Some(path) = selection.get(index - 1) {
                let path: &Path = path.as_ref();
                path.parent()
            } else {
                None
            };
            if let Some(dir) = dir {
                if dir.starts_with(volume_info.folder) {
                    if let Some(mut file_index_reader) = FileIndexReader::new(&volume_info.database).ok() {
                        while let Ok(Some((path, _))) = file_index_reader.next() {
                            if path.starts_with(dir) {
                                f(path)?;
                            }
                        };
                    }
                }
            }
        }
    }
    Ok(())
}

// MatchRule::DirectoryOfSelectionWithSuffix(index, suffix)
fn expand_directory_of_selection_with_suffix<F: FnMut(&Path)->Result<()>>(index: usize, suffix: &OsString, selection: &Vec<PathBuf>, mut f: F) -> Result<()> {
    let sel_path = selection.get(index - 1).ok_or(Error::new(ErrorKind::InvalidData, "Invalid index"))?;
    let sel_dir = sel_path.parent().ok_or(Error::new(ErrorKind::InvalidData, "No parent"))?;
    for path in selection {
        if path.starts_with(sel_dir) {
            if let Some(ext) = path.extension() {
                if ext == suffix {
                    f(path)?;
                }
            }
        }
    }
    Ok(())
}

// MatchRule::DirectoryOfFileIndexWithSuffix(index, suffix)
fn expand_directory_of_file_index_with_suffix<F: FnMut(&Path)->Result<()>>(index: usize, suffix: &OsString, selection: &Vec<PathBuf>, config: &Config, mut f: F) -> Result<()> {
    if let Some(volume_info) = get_volume_info(config) {
        for volume_info in volume_info {
            let dir = if let Some(path) = selection.get(index - 1) {
                let path: &Path = path.as_ref();
                path.parent()
            } else {
                None
            };
            if let Some(dir) = dir {
                if dir.starts_with(volume_info.folder) {
                    if let Some(mut file_index_reader) = FileIndexReader::new(&volume_info.database).ok() {
                        while let Ok(Some((path, _))) = file_index_reader.next() {
                            if path.starts_with(dir) {
                                if let Some(ext) = path.extension() {
                                    if ext == suffix {
                                        f(path)?;
                                    }
                                }
                            }
                        };
                    }
                }
            }
        }
    }
    Ok(())
}

#[derive(PartialEq)]
pub enum MatchRule {
    Index(usize),
    DirectoryOfSelection(usize),
    DirectoryOfFileIndex(usize),
    DirectoryOfSelectionWithSuffix(usize, OsString),
    DirectoryOfFileIndexWithSuffix(usize, OsString),
}

#[derive(PartialEq)]
pub enum ParseError {
    Invalid
}

impl FromStr for MatchRule {
    type Err = ParseError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let mut index = 0;
        let mut dots = 0;
        let mut suffix = String::new();
        #[derive(Clone, Copy)]
        enum State {
            Start,
            Index,
            Dots,
            Suffix,
        }
        let mut state = State::Start;
        for ch in s.chars() {
            match (ch, state) {
                ('0', State::Start | State::Index) => {index = index * 10 + 0; state = State::Index;},
                ('1', State::Start | State::Index) => {index = index * 10 + 1; state = State::Index;},
                ('2', State::Start | State::Index) => {index = index * 10 + 2; state = State::Index;},
                ('3', State::Start | State::Index) => {index = index * 10 + 3; state = State::Index;},
                ('4', State::Start | State::Index) => {index = index * 10 + 4; state = State::Index;},
                ('5', State::Start | State::Index) => {index = index * 10 + 5; state = State::Index;},
                ('6', State::Start | State::Index) => {index = index * 10 + 6; state = State::Index;},
                ('7', State::Start | State::Index) => {index = index * 10 + 7; state = State::Index;},
                ('8', State::Start | State::Index) => {index = index * 10 + 8; state = State::Index;},
                ('9', State::Start | State::Index) => {index = index * 10 + 9; state = State::Index;},
                ('.', State::Index) => {state = State::Dots; dots = 1;},
                (_  , State::Start | State::Index) => {return Err(ParseError::Invalid);},
                ('.', State::Dots) if dots < 3 => {state = State::Dots; dots = dots+1;}
                ('.', _) => {return Err(ParseError::Invalid);},
                (ch, State::Dots) => {suffix.push(ch); state = State::Suffix;},
                (ch, State::Suffix) => {suffix.push(ch);},
            }
        }
        if index == 0 {
            return Err(ParseError::Invalid);
        }
        let match_rule = match (state, dots) {
            (State::Start | State::Index, _) => {return Err(ParseError::Invalid);},
            (State::Dots                , 1) => MatchRule::Index(index),
            (State::Dots                , 2) => MatchRule::DirectoryOfSelection(index),
            (State::Dots                , 3) => MatchRule::DirectoryOfFileIndex(index),
            (State::Suffix              , 2) => MatchRule::DirectoryOfSelectionWithSuffix(index, OsString::from(suffix)),
            (State::Suffix              , 3) => MatchRule::DirectoryOfFileIndexWithSuffix(index, OsString::from(suffix)),
            (_                          , _) => {return Err(ParseError::Invalid);},
        };
        Ok(match_rule)
    }
}

impl Debug for MatchRule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Index(arg0) => f.debug_tuple("Index").field(arg0).finish(),
            Self::DirectoryOfSelection(arg0) => f.debug_tuple("DirectoryOfSelection").field(arg0).finish(),
            Self::DirectoryOfFileIndex(arg0) => f.debug_tuple("DirectoryOfFileIndex").field(arg0).finish(),
            Self::DirectoryOfSelectionWithSuffix(arg0, arg1) => f.debug_tuple("DirectoryOfSelectionWithSuffix").field(arg0).field(arg1).finish(),
            Self::DirectoryOfFileIndexWithSuffix(arg0, arg1) => f.debug_tuple("DirectoryOfFileIndexWithSuffix").field(arg0).field(arg1).finish(),
        }
    }
}

impl Debug for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            _ => {f.write_str("ParseError")?;}
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_dots() {
        assert_eq!("123".parse::<MatchRule>(), Err(ParseError::Invalid));
    }

    #[test]
    fn single_index() {
        assert_eq!("123.".parse(), Ok(MatchRule::Index(123)));
    }

    #[test]
    fn non_zero_index() {
        assert_eq!("0.".parse::<MatchRule>(), Err(ParseError::Invalid));
    }

    #[test]
    fn directory_of_selection() {
        assert_eq!("123..".parse(), Ok(MatchRule::DirectoryOfSelection(123)));
    }

    #[test]
    fn directory_of_file_index() {
        assert_eq!("123...".parse(), Ok(MatchRule::DirectoryOfFileIndex(123)));
    }

    #[test]
    fn too_many_dots() {
        assert_eq!("123....".parse::<MatchRule>(), Err(ParseError::Invalid));
    }

    #[test]
    fn directory_of_selection_with_suffix() {
        assert_eq!("123..jpg".parse(), Ok(MatchRule::DirectoryOfSelectionWithSuffix(123, OsString::from("jpg"))));
    }

    #[test]
    fn directory_of_file_index_with_suffix() {
        assert_eq!("123...jpg".parse(), Ok(MatchRule::DirectoryOfFileIndexWithSuffix(123, OsString::from("jpg"))));
    }

}
