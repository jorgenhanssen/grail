use chess::{BitBoard, Board, Color};
use std::cell::OnceCell;

use crate::attacks::Attacks;

pub struct Position<'a> {
    pub board: &'a Board,
    attacks: OnceCell<Attacks>,
}

impl<'a> Position<'a> {
    #[inline(always)]
    pub fn new(board: &'a Board) -> Self {
        Self {
            board,
            attacks: OnceCell::new(),
        }
    }

    /// Get or compute the attack map (computed once, cached for reuse)
    #[inline(always)]
    fn attacks(&self) -> &Attacks {
        self.attacks.get_or_init(|| Attacks::new(self.board))
    }

    #[inline(always)]
    pub fn space_for(&self, color: Color) -> i16 {
        self.attacks().space[color.to_index()]
    }

    #[inline(always)]
    pub fn attacks_for(&self, color: Color) -> BitBoard {
        self.attacks().attacks[color.to_index()]
    }

    #[inline(always)]
    pub fn threats_for(&self, color: Color) -> BitBoard {
        self.attacks().threats[color.to_index()]
    }

    #[inline(always)]
    pub fn support_for(&self, color: Color) -> BitBoard {
        self.attacks().support[color.to_index()]
    }
}
