use std::collections::HashMap;

use config::EngineConfig;

use crate::gradient::Gradient;
use crate::params::{Parameters, Tunable};

pub struct State {
    pub values: HashMap<String, i64>,
}

impl State {
    pub fn from_params(params: &Parameters) -> Result<Self, String> {
        let json = serde_json::to_value(EngineConfig::default()).unwrap();

        let mut values = HashMap::new();

        for (name, tunable) in params.iter() {
            let value = json[name].as_i64().unwrap();
            values.insert(name.clone(), validate(name, value, tunable)?);
        }

        Ok(Self { values })
    }

    pub fn to_config(&self, config: EngineConfig) -> EngineConfig {
        let mut json = serde_json::to_value(config).unwrap();

        for (name, value) in &self.values {
            json[name.as_str()] = (*value).into();
        }

        serde_json::from_value(json).unwrap()
    }

    pub fn apply(&self, gradient: &Gradient, params: &Parameters) -> Self {
        let mut values = self.values.clone();

        for (name, delta) in &gradient.deltas {
            let tunable = params.get(name).unwrap();
            let value = values[name] + delta;
            values.insert(name.clone(), value.clamp(tunable.min, tunable.max));
        }

        Self { values }
    }
}

fn validate(name: &str, value: i64, tunable: &Tunable) -> Result<i64, String> {
    if !(tunable.min..=tunable.max).contains(&value) {
        return Err(format!(
            "{name} default value ({value}) outside [{}, {}]",
            tunable.min, tunable.max
        ));
    }
    Ok(value)
}
