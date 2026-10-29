use cozy_chess::Color;
use nnue::encoding::{NUM_FEATURES, encode_board_bitset};
use nnue::network::FV_SCALE;
use rand::RngExt;
use utils::{Node, bitset};

pub struct Sampler {
    base_chance: f64,
    feature_delta_weight: f64,
    eval_delta_weight: f64,
    disagreement_weight: f64,
    previous: Option<SampleCandidate>,
}

struct SampleCandidate {
    white_features: bitset!(NUM_FEATURES),
    black_features: bitset!(NUM_FEATURES),
    scout_eval: i16,
}

impl Sampler {
    pub fn new(
        base_chance: f64,
        feature_delta_weight: f64,
        eval_delta_weight: f64,
        disagreement_weight: f64,
    ) -> Self {
        Self {
            base_chance,
            feature_delta_weight,
            eval_delta_weight,
            disagreement_weight,
            previous: None,
        }
    }

    pub fn should_sample(&mut self, node: &Node, static_eval: i16, scout_eval: i16) -> bool {
        let candidate = SampleCandidate::new(node, scout_eval);

        let mut sample_chance = self.base_chance;

        if let Some(previous) = &self.previous {
            sample_chance += self.feature_delta_chance(&candidate, previous);
            sample_chance += self.eval_delta_chance(scout_eval, previous.scout_eval);
        } else {
            sample_chance = 1.0;
        }

        sample_chance += self.eval_disagreement_chance(static_eval, scout_eval);

        let selected = rand::rng().random::<f64>() < sample_chance.clamp(0.0, 1.0);

        if selected {
            self.previous = Some(candidate);
        }

        selected
    }

    fn feature_delta_chance(&self, candidate: &SampleCandidate, previous: &SampleCandidate) -> f64 {
        let feature_delta = feature_distance(candidate, previous);

        self.feature_delta_weight * feature_delta.powi(2)
    }

    fn eval_delta_chance(&self, scout_eval: i16, previous_eval: i16) -> f64 {
        let eval_delta = eval_distance(scout_eval, previous_eval);

        self.eval_delta_weight * eval_delta.powi(2)
    }

    fn eval_disagreement_chance(&self, static_eval: i16, scout_eval: i16) -> f64 {
        let disagreement = eval_distance(static_eval, scout_eval);

        self.disagreement_weight * disagreement.powi(2)
    }
}

impl SampleCandidate {
    fn new(node: &Node, scout_eval: i16) -> Self {
        Self {
            white_features: encode_board_bitset(node.board(), node.metrics(), Color::White),
            black_features: encode_board_bitset(node.board(), node.metrics(), Color::Black),
            scout_eval,
        }
    }
}

fn feature_distance(a: &SampleCandidate, b: &SampleCandidate) -> f64 {
    let mut changed = 0;
    let mut active = 0;

    for (a, b) in [
        (&a.white_features, &b.white_features),
        (&a.black_features, &b.black_features),
    ] {
        for (a, b) in a.as_array().iter().zip(b.as_array()) {
            changed += (a ^ b).count_ones();
            active += (a | b).count_ones();
        }
    }

    changed as f64 / active as f64
}

fn eval_distance(a: i16, b: i16) -> f64 {
    let a = 1.0 / (1.0 + (-(a as f64) / FV_SCALE as f64).exp());
    let b = 1.0 / (1.0 + (-(b as f64) / FV_SCALE as f64).exp());
    (a - b).abs()
}
