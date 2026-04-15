use cozy_chess::{
    BitBoard, Board, Color, Piece, Square, get_bishop_moves, get_king_moves, get_knight_moves,
    get_pawn_attacks, get_rook_moves,
};
use pyrrhic_rs::{EngineAdapter, TableBases, WdlProbeResult};
use utils::en_passant_square;

use crate::MAX_DEPTH;
use crate::scores::MATE_VALUE;

/// Just below MATE_VALUE - MAX_DEPTH so TB wins don't trigger "score mate X" in UCI output,
/// but above MATE_SCORE_BOUND so the TT's ply normalization handles them correctly.
pub const TB_WIN: i16 = MATE_VALUE - MAX_DEPTH as i16 - 1;

/// Adapter wiring cozy_chess attack generation into pyrrhic-rs.
/// Based on the cozy_chess example from https://docs.rs/pyrrhic-rs/latest/pyrrhic_rs/
#[derive(Clone)]
pub struct CozyAdapter;

impl EngineAdapter for CozyAdapter {
    fn pawn_attacks(color: pyrrhic_rs::Color, sq: u64) -> u64 {
        get_pawn_attacks(
            Square::index(sq as usize),
            if color == pyrrhic_rs::Color::Black {
                Color::Black
            } else {
                Color::White
            },
        )
        .0
    }

    fn knight_attacks(sq: u64) -> u64 {
        get_knight_moves(Square::index(sq as usize)).0
    }

    fn bishop_attacks(sq: u64, occ: u64) -> u64 {
        get_bishop_moves(Square::index(sq as usize), BitBoard(occ)).0
    }

    fn rook_attacks(sq: u64, occ: u64) -> u64 {
        get_rook_moves(Square::index(sq as usize), BitBoard(occ)).0
    }

    fn king_attacks(sq: u64) -> u64 {
        get_king_moves(Square::index(sq as usize)).0
    }

    fn queen_attacks(sq: u64, occ: u64) -> u64 {
        (get_bishop_moves(Square::index(sq as usize), BitBoard(occ))
            | get_rook_moves(Square::index(sq as usize), BitBoard(occ)))
        .0
    }
}

pub fn probe_wdl(tb: &TableBases<CozyAdapter>, board: &Board) -> Option<WdlProbeResult> {
    if board.occupied().len() > tb.max_pieces() {
        return None;
    }
    let w = board.castle_rights(Color::White);
    let b = board.castle_rights(Color::Black);
    if w.short.is_some() || w.long.is_some() || b.short.is_some() || b.long.is_some() {
        return None;
    }

    let white = board.colors(Color::White).0;
    let black = board.colors(Color::Black).0;
    let kings = board.pieces(Piece::King).0;
    let queens = board.pieces(Piece::Queen).0;
    let rooks = board.pieces(Piece::Rook).0;
    let bishops = board.pieces(Piece::Bishop).0;
    let knights = board.pieces(Piece::Knight).0;
    let pawns = board.pieces(Piece::Pawn).0;

    let ep = en_passant_square(board).map(|sq| sq as u32).unwrap_or(0);

    let turn = board.side_to_move() == Color::White;

    tb.probe_wdl(
        white, black, kings, queens, rooks, bishops, knights, pawns, ep, turn,
    )
    .ok()
}

/// Map WDL to a search score. TB_WIN sits in the mate-score range so the TT's
/// ply normalization automatically makes the engine prefer shorter winning paths.
pub fn wdl_to_score(wdl: WdlProbeResult, draw_value: i16) -> i16 {
    match wdl {
        WdlProbeResult::Win => TB_WIN,
        WdlProbeResult::Loss => -TB_WIN,
        WdlProbeResult::Draw | WdlProbeResult::CursedWin | WdlProbeResult::BlessedLoss => {
            draw_value
        }
    }
}
