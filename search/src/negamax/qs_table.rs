use super::tt_table::Bound;
use std::mem::size_of;

const CLUSTER_SIZE: usize = 4;
const MIN_BUCKETS: usize = 1024;

#[derive(Clone, Copy, Default)]
#[repr(C)]
struct QSEntry {
    // 0 denotes empty
    key: u64,
    value: i16,
    bound: Bound,
    in_check: bool,
}

pub struct QSTable {
    entries: Vec<QSEntry>,
    mask: usize,
}

impl QSTable {
    #[inline(always)]
    pub fn new(mb: usize) -> Self {
        let bytes = mb.saturating_mul(1024 * 1024);
        let entry_size = size_of::<QSEntry>().max(1);
        let max_entries = (bytes / entry_size).max(CLUSTER_SIZE);
        let buckets = {
            let b = max_entries.div_ceil(CLUSTER_SIZE);
            let b = b.max(MIN_BUCKETS);
            b.next_power_of_two()
        };
        let total_entries = buckets * CLUSTER_SIZE;

        Self {
            entries: vec![QSEntry::default(); total_entries],
            mask: buckets - 1,
        }
    }

    #[inline(always)]
    pub fn clear(&mut self) {
        self.entries.fill(QSEntry::default());
    }

    #[inline(always)]
    fn cluster_start(&self, hash: u64) -> usize {
        let idx = (hash as usize) & self.mask;
        idx * CLUSTER_SIZE
    }

    #[inline(always)]
    pub fn probe(&self, hash: u64, in_check: bool) -> Option<(i16, Bound)> {
        let start = self.cluster_start(hash);
        let end = start + CLUSTER_SIZE;
        let cluster = &self.entries[start..end];
        for e in cluster {
            if e.key == hash && e.in_check == in_check {
                return Some((e.value, e.bound));
            }
        }
        None
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

        let start = self.cluster_start(hash);
        let end = start + CLUSTER_SIZE;
        let cluster = &mut self.entries[start..end];

        // Exact hit
        for e in cluster.iter_mut() {
            if e.key == hash && e.in_check == in_check {
                e.value = value;
                e.bound = bound;
                return;
            }
        }

        // Empty slot
        for e in cluster.iter_mut() {
            if e.key == 0 {
                e.key = hash;
                e.value = value;
                e.bound = bound;
                e.in_check = in_check;
                return;
            }
        }

        // Replace arbitrary victim (round-robin: slot 0)
        cluster[0].key = hash;
        cluster[0].value = value;
        cluster[0].bound = bound;
        cluster[0].in_check = in_check;
    }
}
