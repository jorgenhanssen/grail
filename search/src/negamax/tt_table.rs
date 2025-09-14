use crate::negamax::utils::MATE_SCORE_BOUND;
use chess::{ChessMove, Piece, Square};
use evaluation::scores::MATE_VALUE;
use std::mem::size_of;
use std::simd::prelude::SimdPartialEq;
use std::simd::u64x4;

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
    pub key: u64,
    pub value: i16,
    pub static_eval: i16, // i16::MIN denotes "unknown"
    pub depth: u8,
    pub bound: Bound,
    pub best_move_packed: u16,
}

impl TTEntry {
    #[inline(always)]
    pub fn set(
        &mut self,
        key: u64,
        depth: u8,
        value: i16,
        static_eval: i16,
        bound: Bound,
        best_move_packed: u16,
    ) {
        self.key = key;
        self.depth = depth;
        self.value = value;
        self.static_eval = static_eval;
        self.bound = bound;
        self.best_move_packed = best_move_packed;
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
        // Clear TT entries
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
    ) -> Option<(i16, Bound, Option<ChessMove>, Option<i16>)> {
        let needed_depth = max_depth - depth;
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
            if key_matches.test(i) && entry.depth >= needed_depth {
                let val = if entry.value.abs() >= MATE_SCORE_BOUND {
                    from_tt_value(entry.value, depth)
                } else {
                    entry.value
                };
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
        let idx = (hash as usize) & self.mask;
        let base = idx * CLUSTER_SIZE;

        let cluster = &self.entries[base..base + 4];
        let keys = u64x4::from_array([
            cluster[0].key,
            cluster[1].key,
            cluster[2].key,
            cluster[3].key,
        ]);
        let target_keys = u64x4::splat(hash);
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

        let bound = if value <= alpha {
            Bound::Upper
        } else if value >= beta {
            Bound::Lower
        } else {
            Bound::Exact
        };

        let stored_value = if value.abs() >= MATE_SCORE_BOUND {
            to_tt_value(value, depth)
        } else {
            value
        };
        let stored_se = static_eval.unwrap_or(i16::MIN);

        let idx = (hash as usize) & self.mask;
        let base = idx * CLUSTER_SIZE;
        let end = base + CLUSTER_SIZE;

        let cluster = &mut self.entries[base..end];

        // 1) Exact key hit: update if new info is at least as deep
        for e in cluster.iter_mut() {
            if e.key == hash {
                if stored_depth >= e.depth {
                    e.set(
                        hash,
                        stored_depth,
                        stored_value,
                        stored_se,
                        bound,
                        best_move_packed,
                    );
                }
                return;
            }
        }

        // 2) Empty slot
        for e in cluster.iter_mut() {
            if e.key == 0 {
                e.set(
                    hash,
                    stored_depth,
                    stored_value,
                    stored_se,
                    bound,
                    best_move_packed,
                );
                return;
            }
        }

        // 3) Simple depth-preferential replacement (proven strategy)
        let mut victim_idx = 0;
        let mut min_depth = cluster[0].depth;

        for (i, entry) in cluster.iter().enumerate().skip(1) {
            if entry.depth < min_depth {
                min_depth = entry.depth;
                victim_idx = i;
            }
        }

        cluster[victim_idx].set(
            hash,
            stored_depth,
            stored_value,
            stored_se,
            bound,
            best_move_packed,
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

#[inline(always)]
fn to_tt_value(value: i16, _ply_from_root: u8) -> i16 {
    if value > 0 {
        // Winning mate: store distance from this position (positive)
        MATE_VALUE - value
    } else {
        // Losing mate: store distance from this position (negative)
        -(MATE_VALUE + value)
    }
}

#[inline(always)]
fn from_tt_value(stored: i16, ply_from_root: u8) -> i16 {
    let ply = ply_from_root as i16;
    if stored > 0 {
        // Positive = winning mate distance from this position
        MATE_VALUE - (stored + ply)
    } else {
        // Negative = losing mate distance from this position
        -(MATE_VALUE - (-stored + ply))
    }
}
