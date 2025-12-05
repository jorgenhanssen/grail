use cozy_chess::Color;

/// Flip evaluation score between White's perspective and side-to-move's perspective.
///
/// Evaluators return scores from White's perspective (positive = White better).
/// Search algorithms need scores from the side-to-move's perspective (positive = good for STM).
pub fn flip_eval_perspective(stm: Color, score: i16) -> i16 {
    if stm == Color::White {
        score
    } else {
        -score
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flip_white_to_move() {
        assert_eq!(flip_eval_perspective(Color::White, 100), 100);
        assert_eq!(flip_eval_perspective(Color::White, -50), -50);
    }

    #[test]
    fn test_flip_black_to_move() {
        assert_eq!(flip_eval_perspective(Color::Black, 100), -100);
        assert_eq!(flip_eval_perspective(Color::Black, -50), 50);
    }
}
