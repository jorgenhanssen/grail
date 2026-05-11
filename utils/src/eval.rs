use cozy_chess::Color;

/// Flip evaluation score between white's perspective and side-to-move's perspective.
pub fn flip_eval_perspective(stm: Color, score: i16) -> i16 {
    if stm == Color::White { score } else { -score }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flip_perspective() {
        assert_eq!(flip_eval_perspective(Color::White, 100), 100);
        assert_eq!(flip_eval_perspective(Color::White, -50), -50);
        assert_eq!(flip_eval_perspective(Color::Black, 100), -100);
        assert_eq!(flip_eval_perspective(Color::Black, -50), 50);
    }
}
