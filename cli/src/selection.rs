use std::io::{Result, stdout, Write};
use std::ffi::OsStr;
use std::os::unix::prelude::OsStrExt;

pub struct Item {
    pub path: Vec<u8>,
    pub size: Option<u64>,
}

impl Item {
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
    items: Vec<Item>,
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
}

impl fsidx::SelectionInsert for Selection {
    fn insert(&mut self, path: &[u8], size: Option<u64>) {
        let buf = path.to_vec();
        let item = Item {path: buf, size};
        let index = self.items.len();
        let _ = item.print(index + 1);
        self.items.push(item);
    }

    fn insert_owned(&mut self, path: Vec<u8>, size: Option<u64>) {
        let item = Item {path, size};
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
