use super::tt_table::Bound;
use ahash::AHashMap;

#[derive(Clone, Copy)]
struct QSEntry {
    value: i16,
    bound: Bound,
    in_check: bool,
}

pub struct QSTable {
    map: AHashMap<u64, QSEntry>,
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
    pub fn probe(&self, hash: u64, in_check: bool) -> Option<(i16, Bound)> {
        self.map
            .get(&hash)
            .filter(|e| e.in_check == in_check)
            .map(|e| (e.value, e.bound))
    }

    #[inline(always)]
    pub fn store(&mut self, hash: u64, value: i16, alpha: i16, beta: i16, in_check: bool) {
        let bound = if value <= alpha {
            Bound::Upper
        } else if value >= beta {
            Bound::Lower
        } else {
            Bound::Exact
        };
        self.map.insert(
            hash,
            QSEntry {
                value,
                bound,
                in_check,
            },
        );
    }
}
