use crate::{battery::BatteryLevel, errors::BasicError, errors::MuseStatusError, weather::Units};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs::File, path::Path, path::PathBuf};

/// Configuration for all of muse-status.
#[derive(Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct Config {
    /// The TCP address to run and listen on.
    pub daemon_addr: String,

    /// The ordering of primary-level blocks.
    pub primary_order: Vec<String>,

    /// The ordering of secondary-level blocks.
    pub secondary_order: Vec<String>,

    /// The ordering of tertiary-level blocks.
    pub tertiary_order: Vec<String>,

    /// The name of the brightness directory in Linux's /sys/class/backlight directory.
    pub brightness_id: String,

    /// The audio sink to use for the volume block.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volume_sink: Option<String>,

    /// The name of the user's network interface (like `wlan0`).
    pub network_interface_name: String,

    /// Battery config to use for battery blocks.
    pub battery_config: BatteryConfig,

    /// Weather config to use for weather blocks.
    pub weather_config: WeatherConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            daemon_addr: "localhost:2899".to_string(),
            primary_order: vec![
                "date".to_string(),
                "weather".to_string(),
                "mpris".to_string(),
            ],
            secondary_order: vec![
                "brightness".to_string(),
                "volume".to_string(),
                "network".to_string(),
                "battery".to_string(),
            ],
            tertiary_order: vec![],

            brightness_id: String::from("amdgpu_bl0"),
            network_interface_name: String::from("wlan0"),
            volume_sink: None,

            battery_config: Default::default(),
            weather_config: Default::default(),
        }
    }
}

impl Config {
    /// Parses the configuration file at the path.
    pub fn from_file<P: AsRef<Path>>(p: P) -> Result<Config, MuseStatusError> {
        let path = p.as_ref();
        if !path.exists() {
            // if the file path doesn't exist, write the default config to it, then return the
            // default config.
            Self::write_default_config(path)?;
            Ok(Self::default())
        } else {
            // if the path already exists, read and parse
            serde_yaml::from_reader(File::open(path)?).map_err(|e| {
                MuseStatusError::Basic(BasicError {
                    message: format!("couldn't parse the configuration file: {}", e),
                })
            })
        }
    }

    fn write_default_config(path: &Path) -> Result<(), MuseStatusError> {
        Ok(std::fs::write(
            path,
            serde_yaml::to_string(&Self::default()).map_err(|e| {
                MuseStatusError::Basic(BasicError {
                    message: format!("{}", e),
                })
            })?,
        )?)
    }
}

/// Configuration for a battery information struct.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct BatteryConfig {
    /// The name of the battery in Linux's /sys/class/power_supply/ directory.
    pub battery_id: String,

    /// The level at which the battery is getting low.
    pub warning_level: BatteryLevel,

    /// The level at which the battery is considered critically low.
    pub alarm_level: BatteryLevel,
}

impl Default for BatteryConfig {
    fn default() -> Self {
        Self {
            battery_id: "BAT0".to_string(),
            warning_level: BatteryLevel::Percentage(0.30),
            alarm_level: BatteryLevel::Percentage(0.15),
        }
    }
}

/// Configuration for a weather information block.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct WeatherConfig {
    /// API key for OpenWeatherMap, which, ya know, gets weather information.
    pub openweathermap_key: String,

    /// API key for IPStack, which returns a geolocation from a user's public IP.
    pub ipstack_key: String,

    /// Weather icons.
    pub weather_icons: HashMap<String, char>,

    /// The default icon to use if a weather icon isn't available.
    pub default_icon: char,

    /// How often to update weather, in minutes.
    pub update_interval_minutes: u32,

    /// The units to report weather in, either Imperial or Metric.
    pub units: Units,
}

impl Default for WeatherConfig {
    fn default() -> Self {
        let weather_icons = {
            let mut hm = HashMap::<String, char>::new();
            hm.insert(String::from("01d"), '\u{F0599}');
            hm.insert(String::from("01n"), '\u{F0594}');
            hm.insert(String::from("02d"), '\u{F0595}');
            hm.insert(String::from("02n"), '\u{F0F31}');
            hm.insert(String::from("03d"), '\u{F0590}');
            hm.insert(String::from("03n"), '\u{F0590}');
            hm.insert(String::from("04d"), '\u{F0590}');
            hm.insert(String::from("04n"), '\u{F0590}');
            hm.insert(String::from("09d"), '\u{F0597}');
            hm.insert(String::from("09n"), '\u{F0597}');
            hm.insert(String::from("10d"), '\u{F0596}');
            hm.insert(String::from("10n"), '\u{F0596}');
            hm.insert(String::from("11d"), '\u{F0593}');
            hm.insert(String::from("11n"), '\u{F0593}');
            hm.insert(String::from("13d"), '\u{F0598}');
            hm.insert(String::from("13n"), '\u{F0598}');
            hm.insert(String::from("50d"), '\u{F0591}');
            hm.insert(String::from("50n"), '\u{F0591}');

            hm
        };

        Self {
            // users need to supply their own API keys
            openweathermap_key: String::new(),
            ipstack_key: String::new(),

            weather_icons,
            default_icon: '\u{F0590}',
            update_interval_minutes: 20,

            // although i'm in the US, the rest of the world uses metric, so let's appeal to the
            // masses
            units: Units::Metric,
        }
    }
}

/// Returns the default configuration path for muse-status.
pub fn default_config_path() -> Result<PathBuf, MuseStatusError> {
    if let Some(dir) = dirs::config_dir() {
        Ok(dir.join("muse-status").join("daemon.yaml"))
    } else {
        Err(MuseStatusError::Basic(BasicError {
            message: String::from("couldn't figure out your configuration path.\ntry using the `--config` flag instead")
        }))
    }
}
