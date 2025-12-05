use std::mem::size_of;
use std::simd::prelude::SimdPartialEq;
use std::simd::u32x4;

use cozy_chess::{Move, Piece, Square};
use utils::memory::prefetch;

use crate::pruning::MATE_SCORE_BOUND;

/// Indicates whether the stored value is exact or a bound.
#[derive(Clone, Copy, PartialEq, Default)]
pub enum Bound {
    /// True minimax value (alpha < value < beta)
    #[default]
    Exact = 0,
    /// Value >= beta (beta cutoff)
    Lower = 1,
    /// Value <= alpha (all moves failed)
    Upper = 2,
}

/// Result from probing the transposition table.
/// Caller should check `depth` to decide if `value`/`bound` are trustworthy for cutoffs.
#[derive(Clone, Copy)]
pub struct ProbeResult {
    /// Score from searching this position (mate-adjusted for current ply)
    pub value: i16,
    /// Indicates whether the stored value is exact or a bound
    pub bound: Bound,
    /// Best move found from previous search
    pub best_move: Option<Move>,
    /// Cached static eval (None if unknown)
    pub static_eval: Option<i16>,
    /// Search depth that produced this result
    pub depth: u8,
}

/// A single TT entry (16 bytes, fits 4 per cache line).
#[derive(Clone, Copy, Default)]
#[repr(C)]
pub struct TTEntry {
    /// Lower 32 bits of Zobrist hash for verification
    pub key: u32,
    /// Score from searching this position
    pub value: i16,
    /// Indicates whether the stored value is exact or a bound
    pub bound: Bound,
    /// Static eval without search, cached to avoid recomputation (i16::MIN = unknown)
    pub static_eval: i16,
    /// Search depth that produced this result
    pub depth: u8,
    /// Best move found, packed as: [15:12]=promo, [11:6]=to, [5:0]=from
    pub best_move_packed: u16,
    /// Age for replacement policy
    pub generation: u8,
}

impl TTEntry {
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

/// Hash table for memoizing search results.
/// Uses 4-entry clusters for cache efficiency and SIMD probing.
/// Replacement considers depth, age, and bound type.
///
/// <https://www.chessprogramming.org/Transposition_Table>
pub struct TranspositionTable {
    entries: Vec<TTEntry>,
    buckets: usize,
    generation: u8,
}

impl TranspositionTable {
    /// Creates a new TT with the given size in megabytes.
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

    pub fn clear(&mut self) {
        // Clear TT entries
        unsafe {
            let ptr = self.entries.as_mut_ptr() as *mut u8;
            let size = self.entries.len() * size_of::<TTEntry>();
            std::ptr::write_bytes(ptr, 0, size);
        }
        self.generation = 0;
    }

    /// Increments generation counter. Called at start of each search.
    pub fn age(&mut self) {
        self.generation = self.generation.wrapping_add(1);
    }

    /// Returns hash table fill rate in permille (0-1000).
    ///
    /// Samples the first 1000 entries to get an approximation of the fill rate.
    /// Looping through the entire table would be too slow.
    pub fn hashfull(&self) -> u16 {
        const MAX_SAMPLE: usize = 1000;

        let sample_size = self.entries.len().min(MAX_SAMPLE);
        let sample = &self.entries[..sample_size];

        // Count non-empty entries (key == 0)
        let filled_count = sample.iter().filter(|e| e.key != 0).count();

        // Convert to permille: (filled / sample_size) * 1000
        let permille = (filled_count * 1000) / sample_size;

        permille as u16
    }

    // Prefetch TT entry into cache
    pub fn prefetch(&self, hash: u64) {
        let idx = (hash as usize) % self.buckets;
        let base = idx * CLUSTER_SIZE;

        unsafe {
            let ptr = self.entries.as_ptr().add(base) as *const u8;
            prefetch(ptr);
        }
    }

    /// Probes the TT for a matching entry, returning the deepest match.
    /// Caller should check `result.depth >= needed_depth` before using value/bound for cutoffs.
    pub fn probe(&self, hash: u64, depth: u8) -> Option<ProbeResult> {
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

        // Find deepest matching entry
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

        let (i, _) = best?;
        let entry = &cluster[i];

        // Adjust mate scores relative to current depth
        let value = if entry.value.abs() >= MATE_SCORE_BOUND {
            if entry.value > 0 {
                entry.value - depth as i16
            } else {
                entry.value + depth as i16
            }
        } else {
            entry.value
        };

        let static_eval = if entry.static_eval == i16::MIN {
            None
        } else {
            Some(entry.static_eval)
        };

        Some(ProbeResult {
            value,
            bound: entry.bound,
            best_move: unpack_move(entry.best_move_packed),
            static_eval,
            depth: entry.depth,
        })
    }

    /// Stores a search result using depth/age-based replacement.
    #[allow(clippy::too_many_arguments)]
    pub fn store(
        &mut self,
        hash: u64,
        depth: u8,
        max_depth: u8,
        value: i16,
        static_eval: Option<i16>,
        alpha: i16,
        beta: i16,
        best_move: Option<Move>,
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

        // Store mate scores relative to root so they remain valid from different plies
        let stored_value = if value.abs() >= MATE_SCORE_BOUND {
            if value > 0 {
                value + depth as i16
            } else {
                value - depth as i16
            }
        } else {
            value
        };
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

        // Exact key hit: Replace only if deeper or better bound
        for e in cluster.iter_mut() {
            if e.key == key32 {
                let new_value = stored_depth as i16 + depth_bonus(bound);
                let old_value = e.depth as i16 + depth_bonus(e.bound);

                // Always replace if new bound is Exact and old isn't.
                // Otherwise, only replace if new entry is deeper or better bound type
                let should_replace =
                    (bound == Bound::Exact && e.bound != Bound::Exact) || new_value >= old_value;

                if should_replace {
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

        // Empty slot
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

        // Prefer replacing: shallow entries, old entries, upper bounds
        let mut victim_idx = 0;
        let mut min_score = i16::MAX;

        for (i, entry) in cluster.iter().enumerate() {
            let age = current_gen.wrapping_sub(entry.generation) as i16;
            let entry_depth = entry.depth as i16 + depth_bonus(entry.bound);

            // Lower score = better candidate for replacement
            let score = (8 * entry_depth) - age;

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

/// Packs a move into 16 bits: [15:12]=promo, [11:6]=to, [5:0]=from
fn pack_move(mv: Option<Move>) -> u16 {
    if let Some(m) = mv {
        let from = m.from as u16; // 0..63
        let to = m.to as u16; // 0..63
        let promo = match m.promotion {
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

/// Unpacks a 16-bit encoded move.
fn unpack_move(code: u16) -> Option<Move> {
    if code == 0 {
        return None;
    }
    let from_idx = (code & 0x3F) as usize;
    let to_idx = ((code >> 6) & 0x3F) as usize;
    let promo_code = ((code >> 12) & 0x0F) as u8;
    let from = Square::index(from_idx);
    let to = Square::index(to_idx);
    let promotion = match promo_code {
        1 => Some(Piece::Knight),
        2 => Some(Piece::Bishop),
        3 => Some(Piece::Rook),
        4 => Some(Piece::Queen),
        _ => None,
    };
    Some(Move {
        from,
        to,
        promotion,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pack_unpack_roundtrip() {
        let test_moves: &[(&str, &str, Option<Piece>)] = &[
            ("e2", "e4", None),                // Simple pawn push
            ("a1", "h8", None),                // Corner to corner
            ("g1", "f3", None),                // Knight move
            ("e7", "e8", Some(Piece::Queen)),  // Queen promotion
            ("a7", "a8", Some(Piece::Knight)), // Knight promotion
            ("h7", "h8", Some(Piece::Rook)),   // Rook promotion
            ("b7", "b8", Some(Piece::Bishop)), // Bishop promotion
        ];

        for &(from, to, promo) in test_moves {
            let mv = Move {
                from: from.parse().unwrap(),
                to: to.parse().unwrap(),
                promotion: promo,
            };
            let packed = pack_move(Some(mv));
            let unpacked = unpack_move(packed);
            assert_eq!(unpacked, Some(mv), "Failed for move {}{}", from, to);
        }
    }

    #[test]
    fn test_pack_unpack_none() {
        assert_eq!(pack_move(None), 0);
        assert_eq!(unpack_move(0), None);
    }
}
