use rand::Rng;

/// Select an index from scores using softmax probabilities.
/// Higher scores are more likely to be selected.
///
/// Uses numerically stable softmax: https://jaykmody.com/blog/stable-softmax/
pub fn select_softmax(scores: &[f32], rng: &mut impl Rng) -> usize {
    let max = scores.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let weights: Vec<f32> = scores.iter().map(|&s| (s - max).exp()).collect();
    let total: f32 = weights.iter().sum();

    let mut r = rng.random::<f32>() * total;
    for (i, &w) in weights.iter().enumerate() {
        r -= w;
        if r <= 0.0 {
            return i;
        }
    }

    // Should never happen. but fallback to first score
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;

    fn seeded_rng() -> rand::rngs::StdRng {
        rand::rngs::StdRng::seed_from_u64(69)
    }

    #[test]
    fn single_element_returns_zero() {
        let mut rng = seeded_rng();
        assert_eq!(select_softmax(&[1.0], &mut rng), 0);
        assert_eq!(select_softmax(&[100.0], &mut rng), 0);
        assert_eq!(select_softmax(&[-100.0], &mut rng), 0);
    }

    #[test]
    fn higher_scores_selected_more_often() {
        let mut rng = seeded_rng();
        let scores = [10.0, 0.0, 0.0];

        let mut counts = [0usize; 3];
        for _ in 0..1000 {
            counts[select_softmax(&scores, &mut rng)] += 1;
        }

        assert!(counts[0] > counts[1]);
        assert!(counts[0] > counts[2]);
    }

    #[test]
    fn equal_scores_produce_equal_weights() {
        let scores = [5.0, 5.0, 5.0];

        let max = scores.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        let weights: Vec<f32> = scores.iter().map(|&s| (s - max).exp()).collect();

        // All weights should be exactly 1.0
        for w in weights {
            assert!((w - 1.0).abs() < f32::EPSILON);
        }
    }
}
