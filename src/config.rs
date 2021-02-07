use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::{battery::BatteryLevel, weather::Units};

#[derive(Deserialize, Serialize)]
#[serde(default)]
pub struct Config {
    pub daemon_addr: String,
    pub primary_order: Vec<String>,
    pub secondary_order: Vec<String>,
    pub tertiary_order: Vec<String>,

    pub brightness_id: Option<String>,
    pub network_interface_name: Option<String>,
    pub battery_config: Option<BatteryConfig>,
    pub weather_config: Option<WeatherConfig>,
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

            brightness_id: None,
            network_interface_name: None,
            battery_config: None,
            weather_config: None,
        }
    }
}

#[derive(Deserialize, Serialize)]
pub struct BatteryConfig {
    pub battery_id: String,
    pub warning_level: BatteryLevel,
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

#[derive(Deserialize, Serialize)]
pub struct WeatherConfig {
    pub openweathermap_key: String,
    pub ipstack_key: String,
    pub weather_icons: HashMap<String, char>,
    pub default_icon: char,
    pub update_interval_minutes: u32,
    pub units: Units,
}

impl Default for WeatherConfig {
    fn default() -> Self {
        let mut weather_icons = {
            let hm = HashMap::<String, char>::new();
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
