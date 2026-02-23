use std::cell::UnsafeCell;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
};

use crate::EngineConfig;
use crate::history::CorrectionHistory;
use crate::transposition::TranspositionTable;

/// Shared mutable state for searchers.
///
/// So... thread safety is not CRUCIAL for some of these fields. Two threads hitting
/// the exact same entry at the same time is rare, and even when it happens:
/// - tt: just a cache. Worst case is a super super rare mixed entry (from different writes).
/// - correction: statistical hint, so one wrong value out of thousands doesn't matter
pub struct SharedSearcherState {
    tt: UnsafeCell<TranspositionTable>,
    correction: UnsafeCell<CorrectionHistory>,
    stop: Arc<AtomicBool>,
    total_nodes: AtomicU64,
}

/// UnsafeCell opts out of Sync, so we manually allow it (hehehe).
unsafe impl Sync for SharedSearcherState {}

impl SharedSearcherState {
    pub fn new(config: &EngineConfig, stop: Arc<AtomicBool>) -> Self {
        let tt = TranspositionTable::new(config.hash_size.value as usize);
        let correction = CorrectionHistory::new(
            config.correction_table_size.value,
            config.correction_history_max_value.value,
            config.correction_pawn_weight.value,
            config.correction_minor_weight.value,
            config.correction_nonpawn_weight.value,
            config.correction_combined_divisor.value,
            config.correction_minor_update_weight.value,
            config.correction_nonpawn_update_weight.value,
        );

        Self {
            tt: UnsafeCell::new(tt),
            correction: UnsafeCell::new(correction),
            stop,
            total_nodes: AtomicU64::new(0),
        }
    }

    #[allow(clippy::mut_from_ref)]
    pub fn tt(&self) -> &mut TranspositionTable {
        unsafe { &mut *self.tt.get() }
    }

    #[allow(clippy::mut_from_ref)]
    pub fn correction(&self) -> &mut CorrectionHistory {
        unsafe { &mut *self.correction.get() }
    }

    pub fn is_stopped(&self) -> bool {
        self.stop.load(Ordering::Relaxed)
    }

    pub fn set_stop(&self, value: bool) {
        self.stop.store(value, Ordering::Relaxed);
    }

    pub fn stop_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.stop)
    }

    pub fn add_nodes(&self, count: u64) {
        self.total_nodes.fetch_add(count, Ordering::Relaxed);
    }

    pub fn total_nodes(&self) -> u64 {
        self.total_nodes.load(Ordering::Relaxed)
    }

    pub fn reset_nodes(&self) {
        self.total_nodes.store(0, Ordering::Relaxed);
    }
}
