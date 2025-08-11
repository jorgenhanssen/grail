use ahash::AHashMap;

pub struct QSTable {
    map: AHashMap<u64, i16>,
}

impl QSTable {
    #[inline(always)]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            map: AHashMap::with_capacity(capacity),
        }
    }

    #[inline(always)]
    pub fn clear(&mut self) {
        self.map.clear();
    }

    #[inline(always)]
    pub fn probe(&self, hash: u64) -> Option<i16> {
        self.map.get(&hash).copied()
    }

    #[inline(always)]
    pub fn store(&mut self, hash: u64, value: i16) {
        self.map.insert(hash, value);
    }
}
