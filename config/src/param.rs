use std::str::FromStr;

use uci::UciOption;

/// A typed value paired with optional UCI metadata. When the metadata is
/// missing the value is internal and not configurable via UCI setoption.
#[derive(Debug, Clone)]
pub struct ConfigParam<T> {
    pub value: T,
    pub uci: Option<UciOption>,
}

impl<T> ConfigParam<T>
where
    T: FromStr + ToString + Clone,
    T::Err: std::fmt::Display,
{
    pub fn update_from_uci(&mut self, value: &str) -> Result<(), String> {
        if let Some(uci_meta) = &self.uci {
            uci_meta.option_type.validate(value)?;
        }

        self.value = value
            .parse::<T>()
            .map_err(|e| format!("Parse error: {e}"))?;
        Ok(())
    }

    pub fn to_uci(&self) -> String {
        let uci_meta = self
            .uci
            .as_ref()
            .expect("UCI metadata required for UCI output");
        uci_meta.option_type.to_uci(uci_meta.name, &self.value)
    }
}
