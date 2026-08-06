use std::fmt::Display;

use config::EngineConfig;

#[derive(Debug, Clone, Copy)]
struct UciOption {
    name: &'static str,
    option_type: UciOptionType,
}

#[derive(Debug, Clone, Copy)]
enum UciOptionType {
    Spin { min: i32, max: i32 },
    String,
}

impl UciOption {
    fn to_uci(self, current_value: impl Display) -> String {
        let current_value = current_value.to_string();

        match self.option_type {
            UciOptionType::Spin { min, max } => format!(
                "option name {} type spin default {} min {} max {}",
                self.name, current_value, min, max
            ),
            UciOptionType::String => {
                format!(
                    "option name {} type string default {}",
                    self.name, current_value
                )
            }
        }
    }
}

fn parse_spin(option: &UciOption, value: &str) -> Result<i32, String> {
    let UciOptionType::Spin { min, max } = option.option_type else {
        return Err(format!("Option {} is not a spin option", option.name));
    };

    let parsed = value
        .parse::<i32>()
        .map_err(|error| format!("Invalid integer for {}: {error}", option.name))?;

    if !(min..=max).contains(&parsed) {
        return Err(format!(
            "Value {parsed} for {} is out of range [{min}, {max}]",
            option.name
        ));
    }

    Ok(parsed)
}

const UCI_NAME_HASH: &str = "Hash";
const UCI_NAME_THREADS: &str = "Threads";
const UCI_NAME_MOVE_OVERHEAD: &str = "Move Overhead";
const UCI_NAME_MULTI_PV: &str = "MultiPV";
const UCI_NAME_SYZYGY_PATH: &str = "SyzygyPath";
const UCI_NAME_SYZYGY_PROBE_DEPTH: &str = "SyzygyProbeDepth";

const HASH: UciOption = UciOption {
    name: UCI_NAME_HASH,
    option_type: UciOptionType::Spin {
        min: 1,
        max: 16_384,
    },
};
const THREADS: UciOption = UciOption {
    name: UCI_NAME_THREADS,
    option_type: UciOptionType::Spin { min: 1, max: 256 },
};
const MOVE_OVERHEAD: UciOption = UciOption {
    name: UCI_NAME_MOVE_OVERHEAD,
    option_type: UciOptionType::Spin { min: 0, max: 5_000 },
};
const MULTI_PV: UciOption = UciOption {
    name: UCI_NAME_MULTI_PV,
    option_type: UciOptionType::Spin { min: 1, max: 64 },
};
const SYZYGY_PATH: UciOption = UciOption {
    name: UCI_NAME_SYZYGY_PATH,
    option_type: UciOptionType::String,
};
const SYZYGY_PROBE_DEPTH: UciOption = UciOption {
    name: UCI_NAME_SYZYGY_PROBE_DEPTH,
    option_type: UciOptionType::Spin { min: 1, max: 100 },
};

/// Update the config with the value of a UCI setoption command.
pub fn set_uci_option(config: &mut EngineConfig, name: &str, value: &str) -> Result<(), String> {
    match name {
        UCI_NAME_HASH => {
            config.hash_size = parse_spin(&HASH, value)?;
        }
        UCI_NAME_THREADS => {
            config.threads = parse_spin(&THREADS, value)? as usize;
        }
        UCI_NAME_MOVE_OVERHEAD => {
            config.move_overhead = parse_spin(&MOVE_OVERHEAD, value)?;
        }
        UCI_NAME_MULTI_PV => {
            config.multi_pv = parse_spin(&MULTI_PV, value)? as u8;
        }
        UCI_NAME_SYZYGY_PATH => {
            config.syzygy_path = value.to_owned();
        }
        UCI_NAME_SYZYGY_PROBE_DEPTH => {
            config.syzygy_probe_depth = parse_spin(&SYZYGY_PROBE_DEPTH, value)? as u8;
        }
        "" => {
            return Err("Invalid setoption command: missing option name".to_owned());
        }
        _ => {
            return Err(format!("Unknown option: {name}"));
        }
    }

    Ok(())
}

/// List supported UCI options.
pub fn list_uci_options(config: &EngineConfig) -> Vec<String> {
    vec![
        HASH.to_uci(config.hash_size),
        THREADS.to_uci(config.threads),
        MOVE_OVERHEAD.to_uci(config.move_overhead),
        MULTI_PV.to_uci(config.multi_pv),
        SYZYGY_PATH.to_uci(&config.syzygy_path),
        SYZYGY_PROBE_DEPTH.to_uci(config.syzygy_probe_depth),
    ]
}
