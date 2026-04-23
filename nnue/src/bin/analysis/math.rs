pub fn mean(values: &[f32]) -> f32 {
    values.iter().sum::<f32>() / values.len() as f32
}

pub fn mean_abs(values: &[f32]) -> f32 {
    values.iter().map(|v| v.abs()).sum::<f32>() / values.len() as f32
}

pub fn median_abs(values: &[f32]) -> f32 {
    let mut abs: Vec<f32> = values.iter().map(|v| v.abs()).collect();
    // total_cmp so a stray NaN doesn't poison the sort.
    abs.sort_by(f32::total_cmp);
    abs[abs.len() / 2]
}

pub fn std_dev(values: &[f32]) -> f32 {
    let m = mean(values);
    let var = values.iter().map(|v| (v - m).powi(2)).sum::<f32>() / values.len() as f32;
    var.sqrt()
}

pub fn max_abs(values: &[f32]) -> f32 {
    values.iter().map(|v| v.abs()).fold(0.0, f32::max)
}

pub fn min_of(values: &[f32]) -> f32 {
    values.iter().copied().fold(f32::INFINITY, f32::min)
}

pub fn max_of(values: &[f32]) -> f32 {
    values.iter().copied().fold(f32::NEG_INFINITY, f32::max)
}

/// L2 norm of each input column (fan_in columns). Useful for feature importance.
pub fn col_norms(weights: &[f32], fan_in: usize) -> Vec<f32> {
    let mut sq_sums = vec![0.0f32; fan_in];
    for row in weights.chunks_exact(fan_in) {
        for (s, &w) in sq_sums.iter_mut().zip(row) {
            *s += w * w;
        }
    }
    for s in &mut sq_sums {
        *s = s.sqrt();
    }
    sq_sums
}

/// L2 norm of each output row (one per output neuron).
pub fn row_norms(weights: &[f32], fan_in: usize) -> Vec<f32> {
    weights
        .chunks_exact(fan_in)
        .map(|row| row.iter().map(|w| w * w).sum::<f32>().sqrt())
        .collect()
}

pub fn cosine(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let na = a.iter().map(|v| v * v).sum::<f32>().sqrt();
    let nb = b.iter().map(|v| v * v).sum::<f32>().sqrt();
    if na == 0.0 || nb == 0.0 {
        0.0
    } else {
        dot / (na * nb)
    }
}
