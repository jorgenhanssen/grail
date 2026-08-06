use std::collections::HashMap;
use std::path::Path;

use config::EngineConfig;
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

pub fn load_params(path: &Path) -> HashMap<String, Tunable> {
    let params: HashMap<String, Tunable> = Config::builder()
        .add_source(File::from(path))
        .build()
        .unwrap()
        .try_deserialize()
        .unwrap();

    for (name, param) in &params {
        param.validate(name);
    }

    params
}

// TODO: Dont unwrap
pub fn read_param(config: &EngineConfig, name: &str) -> i64 {
    serde_json::to_value(config).unwrap()[name]
        .as_i64()
        .unwrap()
}

// TODO: Dont unwrap
pub fn write_param(config: &mut EngineConfig, name: &str, value: i64) {
    let mut v = serde_json::to_value(&*config).unwrap();

    v[name] = value.into();
    *config = serde_json::from_value(v).unwrap();
}
