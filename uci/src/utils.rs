use cozy_chess::{Board, Move, util::display_uci_move};

pub fn move_to_uci(board: &Board, mv: Move, chess960: bool) -> String {
    if chess960 {
        // cozy-chess uses Chess960 castling notation by default
        return mv.to_string();
    }
    display_uci_move(board, mv).to_string()
}

pub fn pv_to_uci(starting_board: &Board, pv: &[Move], chess960: bool) -> Vec<String> {
    let mut result = Vec::with_capacity(pv.len());
    let mut board = starting_board.clone();

    for &mv in pv {
        result.push(move_to_uci(&board, mv, chess960));
        board.play_unchecked(mv);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn castling_notation() {
        let standard: Board = "4k3/8/8/8/8/8/8/R3K2R w HA - 0 1".parse().unwrap();
        let castle = "e1h1".parse::<Move>().unwrap();
        assert_eq!(move_to_uci(&standard, castle, false), "e1g1");
        assert_eq!(move_to_uci(&standard, castle, true), "e1h1");

        let stationary_king: Board = "4k3/8/8/8/8/8/8/6KR w H - 0 1".parse().unwrap();
        let castle = "g1h1".parse::<Move>().unwrap();
        assert_eq!(move_to_uci(&stationary_king, castle, true), "g1h1");
    }
}
