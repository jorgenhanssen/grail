use std::collections::HashMap;

use config::EngineConfig;

use crate::game::Score;
use crate::gradient::Gradient;
use crate::params::{Parameters, Tunable};

pub struct State {
    pub values: HashMap<String, f64>,
}

impl State {
    pub fn from_params(params: &Parameters) -> Result<Self, String> {
        let json = serde_json::to_value(EngineConfig::default()).unwrap();

        let mut values = HashMap::new();

        for (name, tunable) in params.iter() {
            let value = json[name].as_i64().unwrap() as f64;
            values.insert(name.clone(), validate(name, value, tunable)?);
        }

        Ok(Self { values })
    }

    pub fn to_config(&self, config: EngineConfig) -> EngineConfig {
        let mut json = serde_json::to_value(config).unwrap();

        for (name, value) in &self.values {
            json[name.as_str()] = (value.round() as i64).into();
        }

        serde_json::from_value(json).unwrap()
    }

    pub fn apply(&self, gradient: &Gradient, params: &Parameters) -> Self {
        let mut values = self.values.clone();

        for (name, delta) in &gradient.deltas {
            let tunable = params.get(name).unwrap();
            let value = values[name] + *delta as f64;
            values.insert(
                name.clone(),
                value.clamp(tunable.min as f64, tunable.max as f64),
            );
        }

        Self { values }
    }

    /// In https://www.chessprogramming.org/SPSA we trust!
    pub fn update(&mut self, grad: &Gradient, score: &Score, params: &Parameters, ak: f64) {
        let result = (score.wins as f64 - score.losses as f64) / score.played() as f64;

        for (name, delta) in &grad.deltas {
            let tunable = params.get(name).unwrap();
            let next = self.values[name] + ak * result / (*delta as f64);
            self.values.insert(
                name.clone(),
                next.clamp(tunable.min as f64, tunable.max as f64),
            );
        }
    }
}

fn validate(name: &str, value: f64, tunable: &Tunable) -> Result<f64, String> {
    let min = tunable.min as f64;
    let max = tunable.max as f64;
    if !(min..=max).contains(&value) {
        return Err(format!(
            "{name} default value ({value}) outside [{}, {}]",
            tunable.min, tunable.max
        ));
    }
    Ok(value)
}
