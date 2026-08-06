use std::collections::HashMap;

use config::EngineConfig;

use crate::params::{Tunable, read_param, write_param};

pub struct State {
    pub values: HashMap<String, i64>,
}

impl State {
    pub fn from_params(params: &HashMap<String, Tunable>) -> Self {
        let config = EngineConfig::default();
        let mut values = HashMap::new();

        for (name, tunable) in params {
            let value = read_param(&config, name).clamp(tunable.min, tunable.max);
            values.insert(name.clone(), value);
        }

        Self { values }
    }

    pub fn to_config(&self, mut config: EngineConfig) -> EngineConfig {
        for (name, value) in &self.values {
            write_param(&mut config, name, *value);
        }
        config
    }
}
