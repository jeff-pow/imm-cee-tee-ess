use std::mem::size_of;

#[derive(Default, Clone, Copy)]
pub struct TableEntry {
    key: u16,
    ptr: i32,
}

pub struct HashTable {
    data: Box<[TableEntry]>,
}

impl HashTable {
    pub fn new(mb: usize) -> Self {
        let num = mb * 1024 * 1024 / size_of::<TableEntry>();
        let data = vec![TableEntry::default(); num].into_boxed_slice();
        Self { data }
    }

    #[allow(dead_code)]
    pub fn probe(&self, hash: u64) -> Option<i32> {
        let idx = index(hash, self.data.len());
        let key = hash as u16;
        let entry = &self.data[idx];
        if entry.key == key {
            return Some(entry.ptr);
        }
        None
    }

    #[allow(dead_code)]
    pub fn insert(&mut self, hash: u64, ptr: i32) {
        let idx = index(hash, self.data.len());
        let key = hash as u16;
        self.data[idx] = TableEntry { key, ptr }
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }
}

fn index(hash: u64, table_capacity: usize) -> usize {
    ((u128::from(hash) * (table_capacity as u128)) >> 64) as usize
}
