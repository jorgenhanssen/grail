use std::mem::size_of;
use std::simd::prelude::SimdPartialEq;
use std::simd::u32x4;

use cozy_chess::{Move, Piece, Square};
use utils::memory::prefetch;

use crate::scores::MATE_SCORE_BOUND;

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
/// Caller should check depth to decide if value/bound are trustworthy for cutoffs.
#[derive(Clone, Copy)]
pub struct ProbeResult {
    /// Score from the previous search, mate-adjusted to the probing ply.
    pub value: i16,
    /// Indicates whether the stored value is exact or a bound
    pub bound: Bound,
    /// Best move found from previous search
    pub best_move: Option<Move>,
    /// Cached static eval (None if never computed).
    pub static_eval: Option<i16>,
    /// Depth of the search that produced this entry.
    pub depth: u8,
    /// Whether this position has been a PV node before.
    pub pv: bool,
}

/// A single TT entry (16 bytes, fits 4 per cache line).
#[derive(Clone, Copy, Default)]
#[repr(C)]
pub struct TTEntry {
    /// Low 32 bits of the Zobrist hash (checksumed on probe).
    pub key: u32,
    /// Score from the search.
    pub value: i16,
    /// Whether value is exact or a bound.
    pub bound: Bound,
    /// Cached static eval (i16::MIN if never computed).
    pub static_eval: i16,
    /// Depth of the search that produced this entry.
    pub depth: u8,
    /// Best move found, packed as: [15:12]=promo, [11:6]=to, [5:0]=from (zero = no move).
    pub best_move_packed: u16,
    /// Generation for age-based replacement.
    pub generation: u8,
    /// Whether this position has been a PV node before.
    pub pv: bool,
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

    /// Returns hash table fill rate in permille (0-1000), sampled over the
    /// first 1000 entries.
    pub fn hashfull(&self) -> u16 {
        const MAX_SAMPLE: usize = 1000;

        let sample_size = self.entries.len().min(MAX_SAMPLE);
        let sample = &self.entries[..sample_size];
        let filled = sample.iter().filter(|e| e.key != 0).count();

        ((filled * 1000) / sample_size) as u16
    }

    pub fn prefetch(&self, hash: u64) {
        let idx = (hash as usize) % self.buckets;
        let base = idx * CLUSTER_SIZE;

        unsafe {
            let ptr = self.entries.as_ptr().add(base) as *const u8;
            prefetch(ptr);
        }
    }

    /// Probes the TT for a matching entry, returning the deepest match.
    /// Caller should check result.depth before using value/bound for cutoffs.
    pub fn probe(&self, hash: u64, ply: u8) -> Option<ProbeResult> {
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

        // Adjust mate scores relative to current ply
        let value = if entry.value.abs() >= MATE_SCORE_BOUND {
            if entry.value > 0 {
                entry.value - ply as i16
            } else {
                entry.value + ply as i16
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
            pv: entry.pv,
        })
    }

    /// Stores a search result using depth/age-based replacement.
    #[allow(clippy::too_many_arguments)]
    pub fn store(
        &mut self,
        hash: u64,
        ply: u8,
        depth: u8,
        value: i16,
        static_eval: Option<i16>,
        alpha: i16,
        beta: i16,
        best_move: Option<Move>,
        pv: bool,
    ) {
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
                value + ply as i16
            } else {
                value - ply as i16
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

        let mut new_entry = TTEntry {
            key: key32,
            value: stored_value,
            bound,
            static_eval: stored_se,
            depth,
            best_move_packed,
            generation: current_gen,
            pv,
        };

        // Exact bounds are more useful than upper bounds at the same depth.
        let depth_bonus = |b: Bound| -> i16 {
            match b {
                Bound::Exact | Bound::Lower => 1,
                Bound::Upper => 0,
            }
        };

        // Same-key hit: only replace if the new entry beats the old one.
        for e in cluster.iter_mut() {
            if e.key == key32 {
                let new_value = depth as i16 + depth_bonus(bound);
                let old_value = e.depth as i16 + depth_bonus(e.bound);
                let should_replace =
                    (bound == Bound::Exact && e.bound != Bound::Exact) || new_value >= old_value;

                if should_replace {
                    if new_entry.best_move_packed == 0 {
                        // In some cases (NMP, RFP etc) the move is not stored
                        // and in such cases we should just keep the old one.
                        new_entry.best_move_packed = e.best_move_packed;
                    }

                    new_entry.pv |= e.pv; // (is/was PV)

                    *e = new_entry;
                }
                return;
            }
        }

        for e in cluster.iter_mut() {
            if e.key == 0 {
                *e = new_entry;
                return;
            }
        }

        // No empty slot - evict the shallowest/oldest entry.
        let mut victim_idx = 0;
        let mut min_score = i16::MAX;

        for (i, entry) in cluster.iter().enumerate() {
            let age = current_gen.wrapping_sub(entry.generation) as i16;
            let entry_depth = entry.depth as i16 + depth_bonus(entry.bound);
            let score = (8 * entry_depth) - age;

            if score < min_score {
                min_score = score;
                victim_idx = i;
            }
        }

        cluster[victim_idx] = new_entry;
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
    fn pack_unpack_roundtrip() {
        let moves: &[(&str, &str, Option<Piece>)] = &[
            ("e2", "e4", None),
            ("a1", "h8", None),
            ("g1", "f3", None),
            ("e7", "e8", Some(Piece::Queen)),
            ("a7", "a8", Some(Piece::Knight)),
            ("h7", "h8", Some(Piece::Rook)),
            ("b7", "b8", Some(Piece::Bishop)),
        ];
        for &(from, to, promotion) in moves {
            let mv = Move {
                from: from.parse().unwrap(),
                to: to.parse().unwrap(),
                promotion,
            };
            assert_eq!(unpack_move(pack_move(Some(mv))), Some(mv), "{from}{to}");
        }

        assert_eq!(pack_move(None), 0);
        assert_eq!(unpack_move(0), None);
    }
}
