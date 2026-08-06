use std::collections::HashMap;
use std::ops::Neg;

use rand::RngExt;

use crate::params::Parameters;

pub struct Gradient {
    pub deltas: HashMap<String, i64>,
}

impl Gradient {
    pub fn random(params: &Parameters) -> Self {
        let mut rng = rand::rng();
        let mut deltas = HashMap::new();

        for (name, tunable) in params.iter() {
            let sign = if rng.random_bool(0.5) { 1 } else { -1 };
            deltas.insert(name.clone(), sign * tunable.step);
        }

        Self { deltas }
    }
}

impl Neg for Gradient {
    type Output = Self;

    fn neg(self) -> Self {
        Self {
            deltas: self
                .deltas
                .into_iter()
                .map(|(name, delta)| (name, -delta))
                .collect(),
        }
    }
}
