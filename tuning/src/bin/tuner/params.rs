use std::collections::HashMap;
use std::path::Path;

use serde::Deserialize;
use settings::{Config, File};

#[derive(Debug, Clone, Deserialize)]
pub struct Tunable {
    pub step: i64,
    pub min: i64,
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

pub struct Parameters {
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
