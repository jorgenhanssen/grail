#[derive(Debug, Clone)]
pub struct UciOption {
    pub name: &'static str,
    pub option_type: UciOptionType,
}

#[derive(Debug, Clone)]
pub enum UciOptionType {
    Spin { min: i32, max: i32 },
    Check,
}

impl UciOptionType {
    pub fn validate(&self, value: &str) -> Result<(), String> {
        match self {
            UciOptionType::Spin { min, max } => {
                let parsed = value
                    .parse::<i32>()
                    .map_err(|e| format!("Invalid integer: {}", e))?;
                if parsed < *min || parsed > *max {
                    return Err(format!("Value {} out of range [{}, {}]", parsed, min, max));
                }
                Ok(())
            }
            UciOptionType::Check => match value.to_lowercase().as_str() {
                "true" | "false" => Ok(()),
                _ => Err("Boolean value must be 'true' or 'false'".to_string()),
            },
        }
    }

    pub fn to_uci<T>(&self, name: &str, current_value: &T) -> String
    where
        T: ToString,
    {
        match self {
            UciOptionType::Spin { min, max } => {
                format!(
                    "option name {} type spin default {} min {} max {}",
                    name,
                    current_value.to_string(),
                    min,
                    max
                )
            }
            UciOptionType::Check => {
                format!(
                    "option name {} type check default {}",
                    name,
                    current_value.to_string()
                )
            }
        }
    }
}
