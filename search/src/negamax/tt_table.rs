use chess::ChessMove;
use std::mem::size_of;
use std::simd::prelude::SimdPartialEq;
use std::simd::{u64x4, u8x4};

#[derive(Clone, Copy, PartialEq, Default)]
pub enum Bound {
    #[default]
    Exact = 0,
    Lower = 1,
    Upper = 2,
}

#[derive(Clone, Copy, Default)]
pub struct TTEntry {
    // 0 = empty slot
    pub key: u64,
    pub best_move: Option<ChessMove>,
    pub value: i16,
    // Remaining plies searched from this position (depth quality)
    pub plies: u8,
    pub bound: Bound,
}

impl TTEntry {
    #[inline(always)]
    pub fn set(
        &mut self,
        key: u64,
        plies: u8,
        value: i16,
        bound: Bound,
        best_move: Option<ChessMove>,
    ) {
        self.key = key;
        self.plies = plies;
        self.value = value;
        self.bound = bound;
        self.best_move = best_move;
    }
}

const CLUSTER_SIZE: usize = 4;
const MIN_BUCKETS: usize = 1024;

pub struct TranspositionTable {
    entries: Vec<TTEntry>,
    mask: usize,
}

impl TranspositionTable {
    #[inline(always)]
    pub fn new(mb: usize) -> Self {
        let bytes = mb.saturating_mul(1024 * 1024);
        let entry_size = size_of::<TTEntry>().max(1);
        let max_entries = (bytes / entry_size).max(CLUSTER_SIZE);
        let buckets = {
            let b = max_entries.div_ceil(CLUSTER_SIZE);
            let b = b.max(MIN_BUCKETS);
            b.next_power_of_two()
        };
        let total_entries = buckets * CLUSTER_SIZE;

        Self {
            entries: vec![TTEntry::default(); total_entries],
            mask: buckets - 1,
        }
    }

    #[inline(always)]
    pub fn clear(&mut self) {
        // Just nuke the entire array to 0 to reset the TT.
        unsafe {
            let ptr = self.entries.as_mut_ptr() as *mut u8;
            let size = self.entries.len() * size_of::<TTEntry>();
            std::ptr::write_bytes(ptr, 0, size);
        }
    }

    #[inline(always)]
    pub fn probe(
        &self,
        hash: u64,
        depth: u8,
        max_depth: u8,
    ) -> Option<(i16, Bound, Option<ChessMove>)> {
        let plies_needed = max_depth - depth;
        let idx = (hash as usize) & self.mask;
        let base = idx * CLUSTER_SIZE;

        // Compare entries (4 at a time with SIMD)
        let cluster = &self.entries[base..base + 4];
        let keys = u64x4::from_array([
            cluster[0].key,
            cluster[1].key,
            cluster[2].key,
            cluster[3].key,
        ]);
        let target_keys = u64x4::splat(hash);
        let key_matches = keys.simd_eq(target_keys);

        // Check each match with depth requirement (most likely first)
        for (i, entry) in cluster.iter().enumerate() {
            if key_matches.test(i) && entry.plies >= plies_needed {
                return Some((entry.value, entry.bound, entry.best_move));
            }
        }
        None
    }

    #[allow(clippy::too_many_arguments)]
    #[inline(always)]
    pub fn store(
        &mut self,
        hash: u64,
        depth: u8,
        max_depth: u8,
        value: i16,
        alpha: i16,
        beta: i16,
        best_move: Option<ChessMove>,
    ) {
        let plies = max_depth - depth;

        let bound = if value <= alpha {
            Bound::Upper
        } else if value >= beta {
            Bound::Lower
        } else {
            Bound::Exact
        };

        let idx = (hash as usize) & self.mask;
        let base = idx * CLUSTER_SIZE;
        let end = base + CLUSTER_SIZE;

        let cluster = &mut self.entries[base..end];

        // 1) Exact key hit: update if new info is at least as deep
        for e in cluster.iter_mut() {
            if e.key == hash {
                if plies >= e.plies {
                    e.set(hash, plies, value, bound, best_move);
                }
                return;
            }
        }

        // 2) Empty slot
        for e in cluster.iter_mut() {
            if e.key == 0 {
                e.set(hash, plies, value, bound, best_move);
                return;
            }
        }

        // 3) SIMD depth comparison for replacement
        let depths = u8x4::from_array([
            cluster[0].plies,
            cluster[1].plies,
            cluster[2].plies,
            cluster[3].plies,
        ]);

        // Find minimum depth
        let depths_array = depths.as_array();
        let victim_idx = depths_array
            .iter()
            .enumerate()
            .min_by_key(|(_, &depth)| depth)
            .map(|(idx, _)| idx)
            .unwrap();

        cluster[victim_idx].set(hash, plies, value, bound, best_move);
    }
}
