use std::path::Path;

use config::EngineConfig;
use serde::Deserialize;

use crate::gradient::Gradient;
use crate::matcher::Score;

/// The tunable specification for a single parameter.
#[derive(Debug, Clone, Deserialize)]
pub struct Tunable {
    /// How much the parameter is nudged per gradient.
    pub step: i64,

    /// The minimum value the parameter can be nudged to.
    pub min: i64,

    /// The maximum value the parameter can be nudged to.
    pub max: i64,
}

impl Tunable {
    fn validate(&self, value: f64) -> Result<(), String> {
        if self.step <= 0 {
            return Err("step must be > 0".into());
        }
        if self.min > self.max {
            return Err(format!("min ({}) must be <= max ({})", self.min, self.max));
        }
        if !(self.min as f64..=self.max as f64).contains(&value) {
            return Err(format!(
                "default value ({value}) outside [{}, {}]",
                self.min, self.max
            ));
        }
        Ok(())
    }
}

/// A tunable parameter with its current SPSA value.
#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub value: f64,
    pub tuning: Tunable,
}

impl Param {
    fn clamp(&self, value: f64) -> f64 {
        value.clamp(self.tuning.min as f64, self.tuning.max as f64)
    }
}

/// SPSA tunable parameters
#[derive(Clone)]
pub struct Parameters {
    params: Vec<Param>,
}

impl Parameters {
    pub fn load(path: &Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        let table: toml::Table = toml::from_str(&text).map_err(|e| e.to_string())?;

        if table.is_empty() {
            return Err("params file is empty".into());
        }

        let defaults = serde_json::to_value(EngineConfig::default()).unwrap();
        let mut params = Vec::new();

        for (name, value) in table {
            let tuning: Tunable = value
                .try_into()
                .map_err(|e| format!("param '{name}': {e}"))?;

            let default = defaults[&name]
                .as_i64()
                .ok_or_else(|| format!("'{name}' is not an EngineConfig field"))?
                as f64;

            tuning
                .validate(default)
                .map_err(|e| format!("param '{name}': {e}"))?;

            params.push(Param {
                name,
                value: default,
                tuning,
            });
        }

        Ok(Self { params })
    }

    pub fn params(&self) -> &[Param] {
        &self.params
    }

    pub fn iter(&self) -> impl Iterator<Item = &Param> {
        self.params.iter()
    }

    pub fn get(&self, name: &str) -> &Param {
        self.params.iter().find(|p| p.name == name).unwrap()
    }

    pub fn to_config(&self, config: EngineConfig) -> EngineConfig {
        let mut json = serde_json::to_value(config).unwrap();

        for param in &self.params {
            json[param.name.as_str()] = (param.value.round() as i64).into();
        }

        serde_json::from_value(json).unwrap()
    }

    /// Apply a gradient and return a new candidate parameter set.
    pub fn apply(&self, gradient: &Gradient) -> Self {
        let mut params = self.params.clone();

        for param in &mut params {
            let delta = gradient.deltas[&param.name];
            param.value = param.clamp(param.value + delta as f64);
        }

        Self { params }
    }

    /// SPSA update from a match score.
    ///
    /// In https://www.chessprogramming.org/SPSA we trust!
    pub fn update(&mut self, grad: &Gradient, score: &Score, gain: f64) {
        let result = (score.wins as f64 - score.losses as f64) / score.played() as f64;

        for param in &mut self.params {
            let delta = grad.deltas[&param.name];
            let next = param.value + gain * result / (delta as f64);
            param.value = param.clamp(next);
        }
    }
}
