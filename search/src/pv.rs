use cozy_chess::Move;

use crate::MAX_DEPTH;
use crate::aspiration::AspirationWindow;

/// One PV line with its score, scored from the searching side. pv_index is
/// the rank in MultiPV (0 = the best line).
#[derive(Clone, Debug, Default)]
pub struct PvLine {
    pub line: Vec<Move>,
    pub score: i16,
    pub pv_index: usize,
}

impl PvLine {
    pub fn new(line: Vec<Move>, score: i16, pv_index: usize) -> Self {
        Self {
            line,
            score,
            pv_index,
        }
    }

    pub fn best_move(&self) -> Option<Move> {
        self.line.first().copied()
    }

    pub fn is_empty(&self) -> bool {
        self.line.is_empty()
    }
}

/// Search context for a single PV rank in multi-PV search.
pub struct PvSearchContext {
    pub result: PvLine,
    pub window: AspirationWindow,
}

impl PvSearchContext {
    pub fn new(window: AspirationWindow) -> Self {
        Self {
            result: PvLine::default(),
            window,
        }
    }
}

pub struct MultiPvSearchContext {
    /// Search context for each PV line
    pub lines: Vec<PvSearchContext>,
    /// Root moves excluded at current depth (moves already found for higher PV ranks)
    excluded: Vec<Move>,
    /// Index of the PV currently being searched
    pub current_pv_index: Option<usize>,
}

impl MultiPvSearchContext {
    pub fn new() -> Self {
        Self {
            lines: Vec::new(),
            excluded: Vec::new(),
            current_pv_index: None,
        }
    }

    pub fn init(
        &mut self,
        count: usize,
        window_size: i16,
        window_widen: i16,
        window_depth: u8,
        score_divisor: i32,
    ) {
        self.lines.clear();
        self.excluded.clear();

        for _ in 0..count {
            let window =
                AspirationWindow::new(window_size, window_widen, window_depth, score_divisor);
            self.lines.push(PvSearchContext::new(window));
        }
    }

    pub fn begin_pv_search(&mut self, pv_index: usize, depth: u8) {
        self.current_pv_index = Some(pv_index);
        let prev_score = self.lines[pv_index].result.score;
        self.lines[pv_index].window.begin_depth(depth, prev_score);
    }

    /// Called at the start of each depth before iterating through the PVs.
    pub fn reset_excluded(&mut self) {
        self.excluded.clear();
        self.current_pv_index = None;
    }

    pub fn add_excluded(&mut self, mv: Move) {
        self.excluded.push(mv);
    }

    pub fn is_excluded(&self, mv: Move) -> bool {
        self.excluded.contains(&mv)
    }

    /// Best move from the previous depth's search of the current PV rank, used
    /// as a hint for move ordering.
    pub fn best_move_hint(&self) -> Option<Move> {
        let pv_index = self.current_pv_index?;
        self.lines.get(pv_index)?.result.best_move()
    }

    pub fn primary(&self) -> Option<&PvLine> {
        self.lines
            .first()
            .map(|pv| &pv.result)
            .filter(|pv| !pv.is_empty())
    }
}

impl Default for MultiPvSearchContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Triangular PV-table: collects the principal variation during search.
/// https://www.chessprogramming.org/Triangular_PV-Table
///
/// Uses the "Quadratic PV-Table" layout for cheaper indexing.
///
/// ply 0: [e2e4, e7e5, g1f3, b8c6, None]
/// ply 1: [e7e5, g1f3, b8c6, None]
/// ply 2: [g1f3, b8c6, None]
/// ply 3: [b8c6, None]
pub struct PvTable {
    pv_array: Vec<Option<Move>>,
}

impl PvTable {
    pub fn new() -> Self {
        Self {
            pv_array: vec![None; MAX_DEPTH * MAX_DEPTH],
        }
    }

    /// Starting index of the PV row for a given ply.
    fn index(ply: usize) -> usize {
        ply * MAX_DEPTH
    }

    /// Mark this ply as having no PV yet.
    pub fn init_ply(&mut self, ply: u8) {
        let p = ply as usize;
        if p < MAX_DEPTH {
            self.pv_array[Self::index(p)] = None;
        }
    }

    /// Write a move at this ply with no continuation (e.g. TT cutoff).
    pub fn set_move(&mut self, ply: u8, mv: Move) {
        let p = ply as usize;
        let pv_index = Self::index(p);
        self.pv_array[pv_index] = Some(mv);
        self.pv_array[pv_index + 1] = None;
    }

    /// Write a move at this ply and append the child's PV from ply + 1.
    pub fn update(&mut self, ply: u8, mv: Move) {
        let p = ply as usize;
        let pv_index = Self::index(p);
        self.pv_array[pv_index] = Some(mv);

        if p + 1 < MAX_DEPTH {
            let pv_next_index = Self::index(p + 1);
            self.pv_array.copy_within(
                pv_next_index..pv_next_index + MAX_DEPTH - p - 1,
                pv_index + 1,
            );
        }
    }

    /// Read the PV at a specific ply
    pub fn get(&self, ply: u8) -> Vec<Move> {
        let pv_index = Self::index(ply as usize);
        self.pv_array[pv_index..pv_index + MAX_DEPTH]
            .iter()
            .take_while(|m| m.is_some())
            .map(|m| m.unwrap())
            .collect()
    }

    pub fn is_empty(&self, ply: u8) -> bool {
        self.pv_array[Self::index(ply as usize)].is_none()
    }
}
