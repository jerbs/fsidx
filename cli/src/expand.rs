use std::ffi::{OsStr, OsString};
use std::fmt::Debug;
use std::os::unix::prelude::OsStrExt;
use std::path::Path;
use std::str::FromStr;
use crate::config::Config;
use crate::selection::{Selection, SelectionIter};

// 421.        -- Open single selected file
// 421..       -- Open all selected files in same directory
// 421...      -- Open all files in same directory
// 421..jpg    -- Open all selected files in same directory with suffix
// 421...jpg   -- Open all files in same directory with suffix

pub struct Expand<'a> {
    config: &'a Config,
    match_rule: MatchRule,
    selection: &'a Selection,
}

impl<'a> Expand<'a> {
    pub fn new(config: &'a Config, match_rule: MatchRule, selection: &'a Selection) -> Expand<'a> {
        Expand {
            config,
            match_rule,
            selection,
        }
    }
}

impl<'a> IntoIterator for Expand<'a> {
    type Item = &'a Path;

    type IntoIter = ExpandIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        ExpandIter::new(self)
    }
}

pub struct ExpandIter<'a> {
    implementation: Box<dyn Iterator<Item=&'a Path> + 'a>,
}

impl<'a>  ExpandIter<'a> {
    fn new(expand: Expand<'a>) -> ExpandIter<'a> {
        let implementation: Box<dyn Iterator<Item=&'a Path>> = match expand.match_rule {
            MatchRule::Index(index) => Box::new(SingleItem::new(index, expand.selection)),
            MatchRule::DirectoryOfSelection(index) => Box::new(DirectoryOfSelection::new(index, expand.selection)),
            MatchRule::DirectoryOfFileIndex(index) => Box::new(DirectoryOfFileIndex::new(index, expand.selection, expand.config)),
            MatchRule::DirectoryOfSelectionWithSuffix(index, suffix) => Box::new(DirectoryOfSelectionWithSuffix::new(index, suffix, expand.selection)),
            MatchRule::DirectoryOfFileIndexWithSuffix(index, suffix) => Box::new(DirectoryOfFileIndexWithSuffix::new(index, suffix, expand.selection, expand.config)),
        };
        ExpandIter { implementation }
        // todo!()
    }
}

impl<'a> Iterator for ExpandIter<'a> {
    type Item = &'a Path;

    fn next(&mut self) -> Option<Self::Item> {
        self.implementation.next()
    }
}

// MatchRule::Index(index)
struct SingleItem<'a> {
    path: Option<&'a Path>
}

impl<'a> SingleItem<'a> {
    fn new(index: usize, selection: &'a Selection) -> SingleItem<'a> {
        let path = selection
        .get_path(index)
        .map(|v| v.as_ref());
        SingleItem { path }
    }
}

impl<'a> Iterator for SingleItem<'a> {
    type Item = &'a Path;

    fn next(&mut self) -> Option<Self::Item> {
        self.path.take()
    }
}

// MatchRule::DirectoryOfSelection(index)
struct DirectoryOfSelection<'a> {
    dir: Option<&'a Path>,
    iter: SelectionIter<'a>,
}

impl<'a> DirectoryOfSelection<'a> {
    fn new(index: usize, selection: &Selection) -> DirectoryOfSelection {
        let dir = if let Some(path) = selection.get_path(index) {
            let path: &Path = path.as_ref();
            path.parent()
        } else {
            None
        };
        let iter = selection.iter();
        DirectoryOfSelection {
            dir,
            iter,
        }
    }
}

impl<'a> Iterator for DirectoryOfSelection<'a> {
    type Item = &'a Path;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(dir) = self.dir {
            let dir = dir
            .as_os_str()
            .as_bytes();
            loop {
                if let Some(item) = self.iter.next() {
                    let path = item.path.as_slice();
                    if path.starts_with(dir) {
                        let path = Path::new(OsStr::from_bytes(path));
                        break Some(path);
                    }
                } else {
                    break None;
                }
            }
        } else {
            None
        }
    }
}

// MatchRule::DirectoryOfFileIndex(index)
struct DirectoryOfFileIndex<'a> {
    dir: Option<&'a Path>,
}

impl<'a> DirectoryOfFileIndex<'a> {
    fn new(_index: usize, _selection: &'a Selection, _config: &'a Config) -> DirectoryOfFileIndex<'a> {
        DirectoryOfFileIndex {
            dir: None,
        }
    }
}

impl<'a> Iterator for DirectoryOfFileIndex<'a> {
    type Item = &'a Path;

    fn next(&mut self) -> Option<Self::Item> {
        None
    }
}

// MatchRule::DirectoryOfSelectionWithSuffix(index, suffix)
struct DirectoryOfSelectionWithSuffix<'a> {
    dir: Option<&'a Path>,
    suffix: OsString,
    iter: SelectionIter<'a>,
}

impl<'a> DirectoryOfSelectionWithSuffix<'a> {
    fn new(index: usize, suffix: OsString, selection: &'a Selection) -> DirectoryOfSelectionWithSuffix<'a> {
        let dir = if let Some(path) = selection.get_path(index) {
            let path: &Path = path.as_ref();
            path.parent()
        } else {
            None
        };
        let iter = selection.iter();
        DirectoryOfSelectionWithSuffix {
            dir,
            suffix,
            iter
        }
    }
}

impl<'a> Iterator for DirectoryOfSelectionWithSuffix<'a> {
    type Item = &'a Path;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(dir) = self.dir {
            let dir = dir
            .as_os_str()
            .as_bytes();
            loop {
                if let Some(item) = self.iter.next() {
                    let path = item.path.as_slice();
                    if path.starts_with(dir) {
                        let path = Path::new(OsStr::from_bytes(path));
                        if let Some(ext) = path.extension() {
                            if ext == self.suffix {
                                break Some(path);
                            }    
                        }
                    }
                } else {
                    break None;
                }
            }
        } else {
            None
        }
    }
}

// MatchRule::DirectoryOfFileIndexWithSuffix(index, suffix)
struct DirectoryOfFileIndexWithSuffix<'a> {
    dir: Option<&'a Path>,
}

impl<'a> DirectoryOfFileIndexWithSuffix<'a> {
    fn new(_index: usize, _suffix: OsString, _selection: &'a Selection, _config: &'a Config) -> DirectoryOfFileIndexWithSuffix<'a> {
        DirectoryOfFileIndexWithSuffix {
            dir: None,
        }
    }
}

impl<'a> Iterator for DirectoryOfFileIndexWithSuffix<'a> {
    type Item = &'a Path;

    fn next(&mut self) -> Option<Self::Item> {
        None
    }
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

    fn from_str(s: &str) -> Result<Self, Self::Err> {
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
