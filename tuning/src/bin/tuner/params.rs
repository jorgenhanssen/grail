use std::collections::HashMap;
use std::path::Path;

use serde::Deserialize;
use settings::{Config, File};

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
    pub fn validate(&self, name: &str) {
        assert!(self.step > 0, "param '{name}': step must be > 0");
        assert!(
            self.min <= self.max,
            "param '{name}': min ({}) must be <= max ({})",
            self.min,
            self.max
        );
    }
}

/// A collection of tunable parameters.
pub struct Parameters {
    /// A map of parameter names to their tunable values.
    ///
    /// Maybe a slightly irritating structure, but the nicest (imo) toml format
    /// parses directly into this
    params: HashMap<String, Tunable>,
}

impl Parameters {
    pub fn load(path: &Path) -> Self {
        let params: HashMap<String, Tunable> = Config::builder()
            .add_source(File::from(path))
            .build()
            .unwrap()
            .try_deserialize()
            .unwrap();

        for (name, param) in &params {
            param.validate(name);
        }

        Self { params }
    }

    pub fn is_empty(&self) -> bool {
        self.params.is_empty()
    }

    pub fn get(&self, name: &str) -> Option<&Tunable> {
        self.params.get(name)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &Tunable)> {
        self.params.iter()
    }
}
