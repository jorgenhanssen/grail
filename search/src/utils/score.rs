use evaluation::scores::MATE_VALUE;
use uci::commands::Score;

pub fn convert_mate_score(score: i16) -> Score {
    let mate_plies = (MATE_VALUE - score.abs()).max(0);
    let mate_in = (mate_plies + 1) / 2;
    if score > 0 {
        Score::Mate(mate_in)
    } else {
        Score::Mate(-mate_in)
    }
}

pub fn convert_centipawn_score(score: i16) -> Score {
    Score::Centipawns(score)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_mate_score_positive() {
        // Mate in 1 = MATE_VALUE - 1 ply
        let score = MATE_VALUE - 1;
        let result = convert_mate_score(score);
        assert!(matches!(result, Score::Mate(m) if m > 0));
    }

    #[test]
    fn test_convert_mate_score_negative() {
        // Getting mated in 1 = -(MATE_VALUE - 1)
        let score = -(MATE_VALUE - 1);
        let result = convert_mate_score(score);
        assert!(matches!(result, Score::Mate(m) if m < 0));
    }

    #[test]
    fn test_convert_mate_in_one() {
        // Mate in 1 ply = mate in 1 move
        let score = MATE_VALUE - 1;
        let result = convert_mate_score(score);
        assert!(matches!(result, Score::Mate(1)));
    }

    #[test]
    fn test_convert_mate_in_two() {
        // Mate in 3 plies = mate in 2 moves (we move, they move, we mate)
        let score = MATE_VALUE - 3;
        let result = convert_mate_score(score);
        assert!(matches!(result, Score::Mate(2)));
    }

    #[test]
    fn test_convert_centipawn_score() {
        assert!(matches!(
            convert_centipawn_score(100),
            Score::Centipawns(100)
        ));
        assert!(matches!(
            convert_centipawn_score(-50),
            Score::Centipawns(-50)
        ));
    }
}
