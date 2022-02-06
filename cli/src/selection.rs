use std::fmt::Debug;
use std::io::{Result, stdout, Write};
use std::ffi::OsStr;
use std::os::unix::prelude::OsStrExt;
// use std::path::PathBuf;

#[derive(Debug, PartialEq)]
pub struct SelectionItem {
    pub path: Vec<u8>,
    pub size: Option<u64>,
}

impl SelectionItem {
    // Not implementing Display trait here, to avoid conversion ti UTF-8.
    fn print(&self, index: usize) -> Result<()> {
        stdout().write_fmt(format_args!("{}. ", index))?;
        stdout().write_all(&self.path)?;
        if let Some(size) = self.size {
            stdout().write_fmt(format_args!(" ({})", size))?;
        }
        stdout().write_all(b"\n")?;   
        Ok(())
    }
}

pub struct Selection {
    items: Vec<SelectionItem>,
}

impl Selection {
    pub fn new() -> Selection {
        Selection {
            items: Vec::new(),
        }
    }

    pub fn get_path(&self, index: usize) -> Option<&OsStr> {
        if let Some(item) = self.items.get(index) {
            let path = OsStr::from_bytes(&item.path);
            Some(path)
        } else {
            None
        }
    }

    pub fn iter(&self) -> SelectionIter<'_> {
        SelectionIter {
            iter: self.items.iter(),
        }
    }
}

impl fsidx::SelectionInsert for Selection {
    fn insert(&mut self, path: &[u8], size: Option<u64>) {
        let buf = path.to_vec();
        let item = SelectionItem {path: buf, size};
        let index = self.items.len();
        let _ = item.print(index + 1);
        self.items.push(item);
    }

    fn insert_owned(&mut self, path: Vec<u8>, size: Option<u64>) {
        let item = SelectionItem {path, size};
        let index = self.items.len();
        let _ = item.print(index + 1);
        self.items.push(item);
    }
}

pub struct NoSelection {
}

impl NoSelection {
    pub fn new() -> NoSelection {
        NoSelection {}
    }
}

impl fsidx::SelectionInsert for NoSelection {
    fn insert(&mut self, _path: &[u8], _size: Option<u64>) {
    }

    fn insert_owned(&mut self, _path: Vec<u8>, _size: Option<u64>) {
    }
}

impl<'a> IntoIterator for &'a Selection {       // Implementing the trait for a reference!
    type Item = &'a SelectionItem;

    type IntoIter = SelectionIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

pub struct SelectionIter<'a> {
    iter: std::slice::Iter<'a, SelectionItem>,
}

impl<'a> Iterator for SelectionIter<'a> {
    type Item = &'a SelectionItem;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }
}

#[cfg(test)]
mod tests {
    use fsidx::SelectionInsert;

    use super::*;

    #[test]
    fn test_selection_iterator() {
        let mut selection = Selection::new();
        selection.insert(b"A", Some(1000));
        selection.insert(b"B", Some(1001));
    
        let mut it = selection.iter();
        assert_eq!(it.next(), Some(&SelectionItem { path: b"A".to_vec(), size: Some(1000) }));
        assert_eq!(it.next(), Some(&SelectionItem { path: b"B".to_vec(), size: Some(1001) }));
        assert_eq!(it.next(), None);
    }
}
