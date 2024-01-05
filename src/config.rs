use std::{
    collections::HashMap,
    fs::File,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::{
    battery::BatteryLevel,
    errors::{BasicError, MuseStatusError},
    weather::Units,
};

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

    /// The name of the brightness directory in Linux's /sys/class/backlight
    /// directory.
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
            // if the file path doesn't exist, write the default config to it, then return
            // the default config.
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
    /// Weather icons.
    pub weather_icons: HashMap<String, char>,

    /// Night time weather icons.
    pub night_weather_icons: HashMap<String, char>,

    /// The default icon to use if a weather icon isn't available.
    pub default_icon: char,

    /// How often to update weather, in minutes.
    pub update_interval_minutes: u32,

    /// The units to report weather in, either Imperial or Metric.
    pub units: Units,
}

impl Default for WeatherConfig {
    fn default() -> Self {
        let weather_icons = HashMap::from_iter(
            DEFAULT_WEATHER_ICONS
                .iter()
                .map(|(k, v)| (k.to_string(), *v)),
        );
        let night_weather_icons = HashMap::from_iter(
            DEFAULT_NIGHT_WEATHER_ICONS
                .iter()
                .map(|(k, v)| (k.to_string(), *v)),
        );

        Self {
            weather_icons,
            night_weather_icons,
            default_icon: '\u{F1BF9}',
            update_interval_minutes: 20,

            // although i'm in the US, the rest of the world uses metric, so let's appeal to
            // the masses
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

/// Default weather icons using the Material Design Icons font. Codes are taken
/// from here, according to wttr.in: https://www.worldweatheronline.com/weather-api/api/docs/weather-icons.aspx
const DEFAULT_WEATHER_ICONS: [(&str, char); 48] = [
    // Clear/Sunny
    ("113", '\u{F0599}'),
    // Partly Cloudy
    ("116", '\u{F0595}'),
    // Cloudy
    ("119", '\u{F0163}'),
    // Overcast
    ("122", '\u{F0163}'),
    // Mist
    ("143", '\u{F0F30}'),
    // Patchy rain nearby
    ("176", '\u{F0F33}'),
    // Patchy snow nearby
    ("179", '\u{F0F34}'),
    // Patchy sleet nearby
    ("182", '\u{F0F35}'),
    // Patchy freezing drizzle nearby
    ("185", '\u{F0F33}'),
    // Thundery outbreaks in nearby
    ("200", '\u{F0593}'),
    // Blowing snow
    ("227", '\u{F059E}'),
    // Blizzard
    ("230", '\u{F0F29}'),
    // Fog
    ("248", '\u{F0591}'),
    // Freezing fog
    ("260", '\u{F0591}'),
    // Patchy light drizzle
    ("263", '\u{F0F33}'),
    // Light drizzle
    ("266", '\u{F0597}'),
    // Freezing drizzle
    ("281", '\u{F0597}'),
    // Heavy freezing drizzle
    ("284", '\u{F0596}'),
    // Patchy light rain
    ("293", '\u{F0F33}'),
    // Light rain
    ("296", '\u{F0597}'),
    // Moderate rain at times
    ("299", '\u{F0596}'),
    // Moderate rain
    ("302", '\u{F0596}'),
    // Heavy rain at times
    ("305", '\u{F0596}'),
    // Heavy rain
    ("308", '\u{F0596}'),
    // Light freezing rain
    ("311", '\u{F0597}'),
    // Moderate or Heavy freezing rain
    ("314", '\u{F0596}'),
    // Light sleet
    ("317", '\u{F067F}'),
    // Moderate or heavy sleet
    ("320", '\u{F067F}'),
    // Patchy light snow
    ("323", '\u{F0F34}'),
    // Light snow
    ("326", '\u{F0598}'),
    // Patchy moderate snow
    ("329", '\u{F0598}'),
    // Moderate snow
    ("332", '\u{F0598}'),
    // Patchy heavy snow
    ("335", '\u{F0F36}'),
    // Heavy snow
    ("338", '\u{F0F36}'),
    // Ice pellets
    ("350", '\u{F0592}'),
    // Light rain shower
    ("353", '\u{F0597}'),
    // Moderate or heavy rain shower
    ("356", '\u{F0596}'),
    // Torrential rain shower
    ("359", '\u{F0596}'),
    // Light sleet showers
    ("362", '\u{F067F}'),
    // Moderate or heavy sleet showers
    ("365", '\u{F067F}'),
    // Light snow showers
    ("368", '\u{F0598}'),
    // Moderate or heavy snow showers
    ("371", '\u{F0F36}'),
    // Light showers of ice pellets
    ("374", '\u{F0592}'),
    // Moderate or heavy showers of ice pellets
    ("377", '\u{F0592}'),
    // Patchy light rain in area with thunder
    ("386", '\u{F0F32}'),
    // Moderate or heavy rain in area with thunder
    ("389", '\u{F067E}'),
    // Patchy light snow in area with thunder
    ("392", '\u{F0593}'),
    // Moderate or heavy snow in area with thunder
    ("395", '\u{F0593}'),
];

/// Default weather icons for nighttime weather using the Material Design Icons
/// font. Codes are taken from here, according to wttr.in:
/// https://www.worldweatheronline.com/weather-api/api/docs/weather-icons.aspx
const DEFAULT_NIGHT_WEATHER_ICONS: [(&str, char); 7] = [
    // Clear/Sunny
    ("113", '\u{F0594}'),
    // Partly Cloudy
    ("116", '\u{F0F31}'),
    // Patchy light snow
    ("323", '\u{F0598}'),
    // Patchy moderate snow
    ("329", '\u{F0F36}'),
    // Patchy heavy snow
    ("335", '\u{F0F36}'),
    // Patchy light rain in area with thunder
    ("386", '\u{F0597}'),
    // Patchy light snow in area with thunder
    ("392", '\u{F0597}'),
];
