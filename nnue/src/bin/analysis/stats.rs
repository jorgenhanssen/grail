use candle_nn::Linear;
use cozy_chess::{Color, Piece};
use nnue::encoding::{
    PIECE_FEATURES_END, PIECE_FEATURES_START, THEM_SPACE_END, THEM_SUPPORT_END, THEM_THREATS_END,
    US_SPACE_START, US_SUPPORT_START, US_THREATS_START,
};
use nnue::network::model::OutputStack;
use std::error::Error;

use crate::math;

/// Weights with |w| below this are considered dead.
pub const DEAD_WEIGHT_THRESHOLD: f32 = 1e-4;

/// Output neurons whose row L2 norm exceeds this are considered active.
pub const ACTIVE_NEURON_THRESHOLD: f32 = 1.0;

/// Piece slots per square: 6 us + 6 them, in (P, N, B, R, Q, K) order.
/// Matches the layout used by `encoding::piece_index`.
pub const PIECES_PER_SQUARE: usize = Piece::NUM * Color::NUM;

/// Feature group in the input layer. Used to split up column norms.
pub struct FeatureGroup {
    pub name: &'static str,
    pub start: usize,
    pub end: usize,
}

impl FeatureGroup {
    pub fn len(&self) -> usize {
        self.end - self.start
    }
}

pub const FEATURE_GROUPS: &[FeatureGroup] = &[
    FeatureGroup {
        name: "pieces",
        start: PIECE_FEATURES_START,
        end: PIECE_FEATURES_END,
    },
    FeatureGroup {
        name: "support",
        start: US_SUPPORT_START,
        end: THEM_SUPPORT_END,
    },
    FeatureGroup {
        name: "space",
        start: US_SPACE_START,
        end: THEM_SPACE_END,
    },
    FeatureGroup {
        name: "threats",
        start: US_THREATS_START,
        end: THEM_THREATS_END,
    },
];

/// Per-piece-type offsets inside the 12-slot piece block of a square.
/// Each row is (name, us_offset, them_offset).
pub const PIECE_TYPES: &[(&str, usize, usize)] = &[
    ("pawn", 0, 6),
    ("knight", 1, 7),
    ("bishop", 2, 8),
    ("rook", 3, 9),
    ("queen", 4, 10),
    ("king", 5, 11),
];

/// Aggregate statistics for a single linear layer.
pub struct LayerStats {
    pub weight_mean_abs: f32,
    pub weight_median_abs: f32,
    pub weight_std: f32,
    pub weight_max_abs: f32,
    pub bias_mean_signed: f32,
    pub bias_mean_abs: f32,
    pub bias_max_abs: f32,
    /// mean|W| normalized by the Kaiming-uniform init scale (1.00x = unchanged from init).
    pub scale: f32,
    /// std / mean|W|: scale-invariant distribution shape.
    pub cov: f32,
    /// Fraction of weights with |W| below DEAD_WEIGHT_THRESHOLD.
    pub dead_fraction: f32,
    /// mean|W| relative to a reference mean. None for the embedding.
    pub rel_to_ref: Option<f32>,
    pub row_norm_min: f32,
    pub row_norm_mean: f32,
    pub row_norm_max: f32,
    pub active_neurons: usize,
    pub total_neurons: usize,
}

impl LayerStats {
    pub fn from_linear(linear: &Linear) -> Result<Self, Box<dyn Error>> {
        let weights = linear.weight().flatten_all()?.to_vec1::<f32>()?;
        let biases = linear.bias().unwrap().to_vec1::<f32>()?;
        let fan_in = linear.weight().dim(1)?;

        let weight_mean_abs = math::mean_abs(&weights);
        let weight_std = math::std_dev(&weights);
        // Kaiming uniform draws from U(-1/sqrt(fan_in), 1/sqrt(fan_in)),
        // so E[|W|] at init is 1/(2*sqrt(fan_in)). Scale is mean|W| / E[|W|_init].
        let scale = weight_mean_abs * 2.0 * (fan_in as f32).sqrt();
        let cov = if weight_mean_abs > 0.0 {
            weight_std / weight_mean_abs
        } else {
            0.0
        };
        let dead_fraction = weights
            .iter()
            .filter(|w| w.abs() < DEAD_WEIGHT_THRESHOLD)
            .count() as f32
            / weights.len() as f32;

        let norms = math::row_norms(&weights, fan_in);
        let active_neurons = norms
            .iter()
            .filter(|&&n| n > ACTIVE_NEURON_THRESHOLD)
            .count();

        Ok(Self {
            weight_mean_abs,
            weight_median_abs: math::median_abs(&weights),
            weight_std,
            weight_max_abs: math::max_abs(&weights),
            bias_mean_signed: math::mean(&biases),
            bias_mean_abs: math::mean_abs(&biases),
            bias_max_abs: math::max_abs(&biases),
            scale,
            cov,
            dead_fraction,
            rel_to_ref: None,
            row_norm_min: math::min_of(&norms),
            row_norm_mean: math::mean(&norms),
            row_norm_max: math::max_of(&norms),
            active_neurons,
            total_neurons: norms.len(),
        })
    }
}

/// Stats for every refinement layer in one bucket.
pub struct BucketStats {
    pub h1: LayerStats,
    pub h2: LayerStats,
    pub output: LayerStats,
}

impl BucketStats {
    pub fn from_stack(stack: &OutputStack) -> Result<Self, Box<dyn Error>> {
        let mut h1 = LayerStats::from_linear(&stack.hidden1)?;
        let mut h2 = LayerStats::from_linear(&stack.hidden2)?;
        let mut output = LayerStats::from_linear(&stack.output)?;

        // Refinement layers report mean|W| relative to their bucket's output mean|W|.
        let ref_mean = output.weight_mean_abs;
        if ref_mean > 0.0 {
            h1.rel_to_ref = Some(h1.weight_mean_abs / ref_mean);
            h2.rel_to_ref = Some(h2.weight_mean_abs / ref_mean);
            output.rel_to_ref = Some(1.0);
        }

        Ok(Self { h1, h2, output })
    }
}
