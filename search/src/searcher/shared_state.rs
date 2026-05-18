use std::cell::UnsafeCell;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
};

use config::EngineConfig;
use pyrrhic_rs::TableBases;

use crate::correction::Correction;
use crate::tablebase::CozyAdapter;
use crate::transposition::TranspositionTable;

/// Shared mutable state for searchers.
///
/// So... thread safety is not CRUCIAL for some of these fields. Two threads hitting
/// the exact same entry at the same time is rare, and even when it happens:
/// - tt: just a cache. Worst case is a super super rare mixed entry (from different writes).
/// - correction: statistical hint, so one wrong value out of thousands doesn't matter
/// - tb: configured outside search, then read by workers
pub struct SharedSearcherState {
    tt: UnsafeCell<TranspositionTable>,
    correction: UnsafeCell<Correction>,
    tb: UnsafeCell<Option<TableBases<CozyAdapter>>>,
    stop: Arc<AtomicBool>,
    total_nodes: AtomicU64,
}

/// UnsafeCell opts out of Sync, so we manually allow it (hehehe).
unsafe impl Sync for SharedSearcherState {}

impl SharedSearcherState {
    pub fn new(config: &EngineConfig, stop: Arc<AtomicBool>) -> Self {
        Self {
            tt: UnsafeCell::new(TranspositionTable::new(config.hash_size.value as usize)),
            correction: UnsafeCell::new(Correction::new(config)),
            tb: UnsafeCell::new(None),
            stop,
            total_nodes: AtomicU64::new(0),
        }
    }

    #[allow(clippy::mut_from_ref)]
    pub fn tt(&self) -> &mut TranspositionTable {
        unsafe { &mut *self.tt.get() }
    }

    #[allow(clippy::mut_from_ref)]
    pub fn correction(&self) -> &mut Correction {
        unsafe { &mut *self.correction.get() }
    }

    pub fn is_stopped(&self) -> bool {
        self.stop.load(Ordering::Relaxed)
    }

    pub fn set_stop(&self, value: bool) {
        self.stop.store(value, Ordering::Relaxed);
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

    pub fn init_tablebases(&self, path: &str) {
        let path = path.replace(';', ":");
        match TableBases::<CozyAdapter>::new(path) {
            Ok(tb) => {
                log::info!("Syzygy tablebases loaded: up to {} pieces", tb.max_pieces());
                unsafe { *self.tb.get() = Some(tb) }
            }
            Err(e) => {
                log::warn!("Failed to load Syzygy tablebases: {:?}", e);
                unsafe { *self.tb.get() = None }
            }
        }
    }

    pub fn clear_tablebases(&self) {
        unsafe { *self.tb.get() = None }
    }

    /// Inject an already-loaded handle (data gen workers share one instance via clone).
    pub fn set_tablebases(&self, tb: TableBases<CozyAdapter>) {
        unsafe { *self.tb.get() = Some(tb) }
    }

    pub fn tb(&self) -> Option<&TableBases<CozyAdapter>> {
        unsafe { (*self.tb.get()).as_ref() }
    }
}
