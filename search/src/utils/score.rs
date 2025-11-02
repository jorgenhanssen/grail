use evaluation::scores::MATE_VALUE;
use uci::commands::Score;

#[inline(always)]
pub fn convert_mate_score(score: i16) -> Score {
    let mate_plies = (MATE_VALUE - score.abs()).max(0);
    let mate_in = (mate_plies + 1) / 2;
    if score > 0 {
        Score::Mate(mate_in)
    } else {
        Score::Mate(-mate_in)
    }
}

#[inline(always)]
pub fn convert_centipawn_score(score: i16) -> Score {
    Score::Centipawns(score)
}

