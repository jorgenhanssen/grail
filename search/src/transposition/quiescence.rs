use super::main::Bound;
use std::mem::size_of;
use std::simd::prelude::SimdPartialEq;
use std::simd::u32x4;

const CLUSTER_SIZE: usize = 4;
const MIN_BUCKETS: usize = 1024;

#[derive(Clone, Copy, Default)]
#[repr(C)]
struct QSEntry {
    // 0 denotes empty
    key: u32, // position hash with bit 0 toggled when in_check
    value: i16,
    bound: Bound,
}

pub struct QSTable {
    entries: Vec<QSEntry>,
    buckets: usize,
}

impl QSTable {
    #[inline(always)]
    pub fn new(mb: usize) -> Self {
        let bytes = mb.saturating_mul(1024 * 1024);
        let entry_size = size_of::<QSEntry>().max(1);
        let max_entries = (bytes / entry_size).max(CLUSTER_SIZE);

        let buckets = (max_entries / CLUSTER_SIZE).max(MIN_BUCKETS);
        let total_entries = buckets * CLUSTER_SIZE;

        Self {
            entries: vec![QSEntry::default(); total_entries],
            buckets,
        }
    }

    #[inline(always)]
    pub fn clear(&mut self) {
        unsafe {
            let ptr = self.entries.as_mut_ptr() as *mut u8;
            let size = self.entries.len() * size_of::<QSEntry>();
            std::ptr::write_bytes(ptr, 0, size);
        }
    }

    #[inline(always)]
    fn cluster_start(&self, mixed_key: u32) -> usize {
        let idx = (mixed_key as usize) % self.buckets;
        idx * CLUSTER_SIZE
    }

    // Prefetch QS entry into cache
    #[inline(always)]
    pub fn prefetch(&self, hash: u64) {
        // Most cases is not check, so this is a reasonable guesstimate
        let mixed = mix_key(hash, false);
        let start = self.cluster_start(mixed);

        unsafe {
            let ptr = self.entries.as_ptr().add(start) as *const u8;
            crate::utils::memory::prefetch(ptr);
        }
    }

    #[inline(always)]
    pub fn probe(&self, hash: u64, in_check: bool) -> Option<(i16, Bound)> {
        let mixed = mix_key(hash, in_check);
        let start = self.cluster_start(mixed);
        let end = start + CLUSTER_SIZE;
        let cluster = &self.entries[start..end];

        // SIMD compare 4 keys at once
        let keys = u32x4::from_array([
            cluster[0].key,
            cluster[1].key,
            cluster[2].key,
            cluster[3].key,
        ]);
        let target = u32x4::splat(mixed);
        let mask = keys.simd_eq(target);

        for (i, e) in cluster.iter().enumerate() {
            if mask.test(i) {
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

        let mixed = mix_key(hash, in_check);
        let start = self.cluster_start(mixed);
        let end = start + CLUSTER_SIZE;
        let cluster = &mut self.entries[start..end];

        // Exact hit
        for e in cluster.iter_mut() {
            if e.key == mixed {
                e.value = value;
                e.bound = bound;
                return;
            }
        }

        // Empty slot
        for e in cluster.iter_mut() {
            if e.key == 0 {
                e.key = mixed;
                e.value = value;
                e.bound = bound;
                return;
            }
        }

        // Prefer replacing a non-Exact bound; otherwise slot 0
        if let Some((idx, _)) = cluster
            .iter()
            .enumerate()
            .find(|(_, e)| e.bound != Bound::Exact)
        {
            cluster[idx].key = mixed;
            cluster[idx].value = value;
            cluster[idx].bound = bound;
        } else {
            cluster[0].key = mixed;
            cluster[0].value = value;
            cluster[0].bound = bound;
        }
    }
}

#[inline(always)]
fn mix_key(hash: u64, in_check: bool) -> u32 {
    let key32 = hash as u32;
    if in_check {
        key32 ^ 0x1
    } else {
        key32
    }
}
