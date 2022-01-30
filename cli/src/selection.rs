use std::io::{Result, stdout, Write};

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
