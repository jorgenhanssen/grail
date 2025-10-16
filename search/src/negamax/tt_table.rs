use crate::negamax::utils::MATE_SCORE_BOUND;
use chess::{ChessMove, Piece, Square};
use std::mem::size_of;
use std::simd::prelude::SimdPartialEq;
use std::simd::u32x4;

#[derive(Clone, Copy, PartialEq, Default)]
pub enum Bound {
    #[default]
    Exact = 0,
    Lower = 1,
    Upper = 2,
}

#[derive(Clone, Copy, Default)]
#[repr(C)]
pub struct TTEntry {
    // 0 = empty slot
    pub key: u32,
    pub value: i16,
    pub static_eval: i16, // i16::MIN denotes "unknown"
    pub depth: u8,
    pub bound: Bound,
    pub best_move_packed: u16,
    pub generation: u8, // Tracks freshness (increments per search)
}

impl TTEntry {
    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    pub fn set(
        &mut self,
        key: u32,
        depth: u8,
        value: i16,
        static_eval: i16,
        bound: Bound,
        best_move_packed: u16,
        generation: u8,
    ) {
        self.key = key;
        self.depth = depth;
        self.value = value;
        self.static_eval = static_eval;
        self.bound = bound;
        self.best_move_packed = best_move_packed;
        self.generation = generation;
    }
}

const CLUSTER_SIZE: usize = 4;
const MIN_BUCKETS: usize = 1024;

pub struct TranspositionTable {
    entries: Vec<TTEntry>,
    buckets: usize,
    generation: u8,
}

impl TranspositionTable {
    #[inline(always)]
    pub fn new(mb: usize) -> Self {
        let bytes = mb.saturating_mul(1024 * 1024);
        let entry_size = size_of::<TTEntry>().max(1);
        let max_entries = (bytes / entry_size).max(CLUSTER_SIZE);

        let buckets = (max_entries / CLUSTER_SIZE).max(MIN_BUCKETS);
        let total_entries = buckets * CLUSTER_SIZE;

        Self {
            entries: vec![TTEntry::default(); total_entries],
            buckets,
            generation: 0,
        }
    }

    #[inline(always)]
    pub fn clear(&mut self) {
        // Clear TT entries
        unsafe {
            let ptr = self.entries.as_mut_ptr() as *mut u8;
            let size = self.entries.len() * size_of::<TTEntry>();
            std::ptr::write_bytes(ptr, 0, size);
        }
        self.generation = 0;
    }

    #[inline(always)]
    pub fn age(&mut self) {
        self.generation = self.generation.wrapping_add(1);
    }

    // Prefetch TT entry into cache
    #[inline(always)]
    pub fn prefetch(&self, hash: u64) {
        let idx = (hash as usize) % self.buckets;
        let base = idx * CLUSTER_SIZE;

        unsafe {
            let ptr = self.entries.as_ptr().add(base) as *const u8;
            crate::utils::memory::prefetch(ptr);
        }
    }

    #[inline(always)]
    pub fn probe(
        &self,
        hash: u64,
        depth: u8,
        max_depth: u8,
    ) -> Option<(i16, Bound, Option<ChessMove>, Option<i16>)> {
        let needed_depth = max_depth - depth;
        let idx = (hash as usize) % self.buckets;
        let base = idx * CLUSTER_SIZE;
        let key32 = hash as u32;

        // Compare entries (4 at a time with SIMD)
        let cluster = &self.entries[base..base + 4];
        let keys = u32x4::from_array([
            cluster[0].key,
            cluster[1].key,
            cluster[2].key,
            cluster[3].key,
        ]);
        let target_keys = u32x4::splat(key32);
        let key_matches = keys.simd_eq(target_keys);

        // Check each match with depth requirement (most likely first)
        for (i, entry) in cluster.iter().enumerate() {
            if key_matches.test(i) && entry.depth >= needed_depth {
                let val = entry.value;
                let se_opt = if entry.static_eval == i16::MIN {
                    None
                } else {
                    Some(entry.static_eval)
                };
                return Some((
                    val,
                    entry.bound,
                    unpack_move(entry.best_move_packed),
                    se_opt,
                ));
            }
        }
        None
    }

    #[inline(always)]
    pub fn probe_hint(&self, hash: u64) -> Option<(Option<ChessMove>, Option<i16>)> {
        let idx = (hash as usize) % self.buckets;
        let base = idx * CLUSTER_SIZE;
        let key32 = hash as u32;

        let cluster = &self.entries[base..base + 4];
        let keys = u32x4::from_array([
            cluster[0].key,
            cluster[1].key,
            cluster[2].key,
            cluster[3].key,
        ]);
        let target_keys = u32x4::splat(key32);
        let key_matches = keys.simd_eq(target_keys);

        // Prefer deepest entry as hint
        let mut best: Option<(usize, u8)> = None;
        for (i, entry) in cluster.iter().enumerate() {
            if key_matches.test(i) {
                if let Some((_, d)) = best {
                    if entry.depth > d {
                        best = Some((i, entry.depth));
                    }
                } else {
                    best = Some((i, entry.depth));
                }
            }
        }
        if let Some((i, _)) = best {
            let entry = &cluster[i];
            let se = if entry.static_eval == i16::MIN {
                None
            } else {
                Some(entry.static_eval)
            };
            return Some((unpack_move(entry.best_move_packed), se));
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
        static_eval: Option<i16>,
        alpha: i16,
        beta: i16,
        best_move: Option<ChessMove>,
    ) {
        let stored_depth = max_depth - depth;
        let best_move_packed = pack_move(best_move);
        let key32 = hash as u32;

        let bound = if value <= alpha {
            Bound::Upper
        } else if value >= beta {
            Bound::Lower
        } else {
            Bound::Exact
        };

        if value.abs() >= MATE_SCORE_BOUND {
            return;
        }

        let stored_value = value;
        let stored_se = static_eval.unwrap_or(i16::MIN);

        let idx = (hash as usize) % self.buckets;
        let base = idx * CLUSTER_SIZE;
        let end = base + CLUSTER_SIZE;

        let cluster = &mut self.entries[base..end];
        let current_gen = self.generation;

        // Depth bonus for valuable bound types (exact/lower more useful than upper)
        let depth_bonus = |b: Bound| -> i16 {
            match b {
                Bound::Exact | Bound::Lower => 1,
                Bound::Upper => 0,
            }
        };

        // 1) Exact key hit: Replace only if deeper or better bound (depth-preferential)
        for e in cluster.iter_mut() {
            if e.key == key32 {
                let new_value = stored_depth as i16 + depth_bonus(bound);
                let old_value = e.depth as i16 + depth_bonus(e.bound);

                // Only replace if new entry is better (deeper or better bound type)
                if new_value >= old_value {
                    e.set(
                        key32,
                        stored_depth,
                        stored_value,
                        stored_se,
                        bound,
                        best_move_packed,
                        current_gen,
                    );
                }
                return;
            }
        }

        // 2) Empty slot
        for e in cluster.iter_mut() {
            if e.key == 0 {
                e.set(
                    key32,
                    stored_depth,
                    stored_value,
                    stored_se,
                    bound,
                    best_move_packed,
                    current_gen,
                );
                return;
            }
        }

        // 3) Find victim: depth-preferential with age and bound considerations
        // Prefer replacing: shallow entries, old entries, upper bounds
        let mut victim_idx = 0;
        let mut min_score = i16::MAX;

        for (i, entry) in cluster.iter().enumerate() {
            let age = current_gen.wrapping_sub(entry.generation) as i16;
            let entry_depth = entry.depth as i16 + depth_bonus(entry.bound);

            // Score = depth value - age penalty
            // Lower score = better candidate for replacement
            // Age penalty divided by 8 to be gentle (only matters in tournaments)
            let score = entry_depth - (age / 8);

            if score < min_score {
                min_score = score;
                victim_idx = i;
            }
        }

        cluster[victim_idx].set(
            key32,
            stored_depth,
            stored_value,
            stored_se,
            bound,
            best_move_packed,
            current_gen,
        );
    }
}

#[inline(always)]
fn pack_move(mv: Option<ChessMove>) -> u16 {
    // Layout: [15..12]=promo (0=None,1=N,2=B,3=R,4=Q), [11..6]=to, [5..0]=from
    if let Some(m) = mv {
        let from = m.get_source().to_index() as u16; // 0..63
        let to = m.get_dest().to_index() as u16; // 0..63
        let promo = match m.get_promotion() {
            Some(Piece::Knight) => 1u16,
            Some(Piece::Bishop) => 2u16,
            Some(Piece::Rook) => 3u16,
            Some(Piece::Queen) => 4u16,
            _ => 0u16,
        };
        (from & 0x3F) | ((to & 0x3F) << 6) | ((promo & 0x0F) << 12)
    } else {
        0
    }
}

#[inline(always)]
fn unpack_move(code: u16) -> Option<ChessMove> {
    if code == 0 {
        return None;
    }
    let from_idx = (code & 0x3F) as u8;
    let to_idx = ((code >> 6) & 0x3F) as u8;
    let promo_code = ((code >> 12) & 0x0F) as u8;
    let from = unsafe { Square::new(from_idx) };
    let to = unsafe { Square::new(to_idx) };
    let promo = match promo_code {
        1 => Some(Piece::Knight),
        2 => Some(Piece::Bishop),
        3 => Some(Piece::Rook),
        4 => Some(Piece::Queen),
        _ => None,
    };
    Some(ChessMove::new(from, to, promo))
}
