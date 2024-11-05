use std::mem::size_of;

#[derive(Default, Debug, Clone, Copy)]
pub struct TableEntry {
    key: u16,
    eval: f32,
}

#[derive(Debug)]
pub struct HashTable {
    data: Box<[TableEntry]>,
}

impl HashTable {
    pub fn new(mb: f32) -> Self {
        let cap = (mb * 1024. * 1024. / size_of::<TableEntry>() as f32) as usize;
        assert!(cap > 0, "Hash table must have at least 1 element");
        let data = vec![TableEntry::default(); cap].into_boxed_slice();
        Self { data }
    }

    pub fn probe(&self, hash: u64) -> Option<f32> {
        let idx = self.index(hash);
        let key = hash as u16;
        let entry = &self.data[idx];
        if entry.key == key {
            return Some(entry.eval);
        }
        None
    }

    pub fn clear(&mut self) {
        for entry in &mut self.data {
            *entry = TableEntry::default();
        }
    }

    pub fn insert(&mut self, hash: u64, eval: f32) {
        let idx = self.index(hash);
        let key = hash as u16;
        self.data[idx] = TableEntry { key, eval }
    }

    pub const fn len(&self) -> usize {
        self.data.len()
    }

    fn index(&self, hash: u64) -> usize {
        ((u128::from(hash) * (self.data.len() as u128)) >> 64) as usize
    }
}
