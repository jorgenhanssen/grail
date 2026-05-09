use uci::{UciOption, UciOptionType};

pub(crate) fn uci_option(
    include: bool,
    name: &'static str,
    option_type: UciOptionType,
) -> Option<UciOption> {
    if include {
        Some(UciOption { name, option_type })
    } else {
        None
    }
}

/// Builds EngineConfig from the param tuples below. The include flag picks
/// whether a param is reachable via setoption (often for tuning).
macro_rules! define_config {
    ($(($field:ident: $type:ty, $uci_name:expr, $uci_type:expr, $default:expr, $include:expr)),* $(,)?) => {
        #[derive(Debug, Clone)]
        pub struct EngineConfig {
            $(pub $field: $crate::ConfigParam<$type>,)*
        }

        impl Default for EngineConfig {
            fn default() -> Self {
                Self {
                    $($field: $crate::ConfigParam {
                        value: $default,
                        uci: $crate::macros::uci_option($include, $uci_name, $uci_type),
                    },)*
                }
            }
        }

        impl EngineConfig {
            pub fn update_from_uci(&mut self, uci_name: &str, value: &str) -> Result<(), String> {
                // TODO: Empty name is a workaround for malformed setoption commands.
                // Consider adding InvalidCommand variant to UciInput instead.
                if uci_name.is_empty() {
                    return Err("Invalid setoption command: missing option name".to_string());
                }

                match uci_name {
                    $($uci_name if $include => self.$field.update_from_uci(value),)*
                    _ => Err(format!("Unknown option: {uci_name}")),
                }
            }

            pub fn to_uci(
                &self,
                output: &std::sync::mpsc::Sender<::uci::UciOutput>,
            ) -> Result<(), std::sync::mpsc::SendError<::uci::UciOutput>> {
                $(
                    if self.$field.uci.is_some() {
                        output.send(::uci::UciOutput::Option(self.$field.to_uci()))?;
                    }
                )*
                Ok(())
            }
        }
    };
}

pub(crate) use define_config;
