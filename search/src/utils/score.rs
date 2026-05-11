use crate::scores::MATE_VALUE;
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
    fn mate_score_to_mate_in() {
        let cases: &[(i16, i16)] = &[
            (MATE_VALUE - 1, 1),
            (MATE_VALUE - 3, 2),
            (MATE_VALUE - 5, 3),
            (-(MATE_VALUE - 1), -1),
            (-(MATE_VALUE - 3), -2),
        ];
        for &(score, want) in cases {
            assert!(
                matches!(convert_mate_score(score), Score::Mate(m) if m == want),
                "score={score} want=Mate({want})",
            );
        }
    }

    #[test]
    fn centipawn_score_passthrough() {
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
