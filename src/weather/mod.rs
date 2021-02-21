mod structs;

use crate::{
    config::WeatherConfig,
    errors::*,
    format::{
        blocks::{output::*, Block},
        Attention,
    },
};
use chrono::Local;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use structs::*;

/// Type of units to use when reporting locale-specific measurements.
#[derive(Clone, Copy, Deserialize, Serialize)]
pub enum Units {
    /// Freedom units.
    Imperial,

    /// Non-US units.
    Metric,
}

impl fmt::Display for Units {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Imperial => write!(f, "imperial"),
            Self::Metric => write!(f, "metric"),
        }
    }
}

/// WeatherBlock returns information about the weather around the user's current location.
/// OpenWeatherMap and IPStack are used for weather and location respectively.
pub struct WeatherBlock {
    openweathermap_key: String,
    ipstack_key: String,
    weather_icons: HashMap<String, char>,
    default_icon: char,

    current_report: Option<FullWeatherReport>,
    location: Option<WeatherLocation>,
    units: Units,

    update_interval_minutes: u32,

    next_update_time: chrono::DateTime<chrono::Local>,
}

impl Default for WeatherBlock {
    fn default() -> Self {
        Self::new(WeatherConfig::default())
    }
}

impl WeatherBlock {
    /// Creates a new weather block.
    pub fn new(config: WeatherConfig) -> Self {
        Self {
            openweathermap_key: config.openweathermap_key,
            ipstack_key: config.ipstack_key,
            weather_icons: config.weather_icons,
            update_interval_minutes: config.update_interval_minutes,
            default_icon: config.default_icon,
            units: config.units,

            current_report: None,
            location: None,
            next_update_time: Local::now()
                + chrono::Duration::minutes(config.update_interval_minutes as i64),
        }
    }

    /// Creates a new weather block, but with a custom location.
    pub fn new_with_location(config: WeatherConfig, location: WeatherLocation) -> Self {
        let mut w = Self::new(config);
        w.current_report = None;
        w.location = Some(location);

        w
    }

    fn get_current_location(&self) -> Result<WeatherLocation, MuseStatusError> {
        let ip = get_external_ip()?;

        let url = format!(
            "http://api.ipstack.com/{}?access_key={}&format=1",
            ip, self.ipstack_key
        );

        let res = reqwest::blocking::get(&url)?;

        match serde_json::from_str::<WeatherLocation>(&res.text()?) {
            Ok(r) => Ok(r),
            Err(e) => Err(MuseStatusError::from(BasicError {
                message: format!("couldn't deserialize current location from ipstack: {}", e),
            })),
        }
    }

    fn get_weather_icon(&self, report: &FullWeatherReport) -> char {
        if let Some(r) = report.weather.get(0) {
            let icon_string = &r.icon;
            self.weather_icons[icon_string]
        } else {
            self.default_icon
        }
    }

    fn update_current_report(&mut self) -> Result<(), UpdateError> {
        if self.location.is_none() {
            let location = self.get_current_location().map_err(|e| UpdateError {
                block_name: self.name().to_owned(),
                message: format!("couldn't get current location: {}", e),
            })?;
            self.location = Some(location);
        }

        self.current_report = match &self.location {
            Some(l) => {
                let req_url = format!(
                    "http://api.openweathermap.org/data/2.5/weather?lat={}&lon={}&appid={}&units={}",
                    l.latitude, l.longitude, self.openweathermap_key, self.units.to_string()
                );

                let text = match reqwest::blocking::get(&req_url) {
                    Ok(res) => match res.text() {
                        Ok(t) => t,
                        Err(e) => {
                            return Err(UpdateError {
                                block_name: self.name().to_string(),
                                message: format!("couldn't retrieve weather data as text: {}", e),
                            })
                        }
                    },
                    Err(e) => {
                        return Err(UpdateError {
                            block_name: self.name().to_string(),
                            message: format!("couldn't retrieve weather data: {}", e),
                        })
                    }
                };

                let report: FullWeatherReport = match serde_json::from_str(&text) {
                    Ok(r) => r,
                    Err(e) => {
                        return Err(UpdateError {
                            block_name: self.name().to_string(),
                            message: format!(
                                "couldn't deserialize response for weather report: {}",
                                e
                            ),
                        })
                    }
                };

                Some(report)
            }
            None => unreachable!(), // because location should be initialized if None at the beginning of this function
        };

        Ok(())
    }

    /// Returns a number with a little circle-thing next to it.
    pub fn get_temperature_string(&self) -> Option<String> {
        if let Some(r) = &self.current_report {
            if r.weather.is_empty() {
                None
            } else {
                Some(format!("{}°", r.main.temp.round() as i32))
            }
        } else {
            None
        }
    }

    /// Returns a String-ified weather description, in Sentence case.
    pub fn get_weather_description(&self) -> Option<String> {
        if let Some(r) = &self.current_report {
            if r.weather.is_empty() {
                None
            } else {
                let mut desc = r.weather[0].description.to_owned();

                // capitalize the first letter in the description
                match desc.chars().next() {
                    Some(c) => {
                        desc = format!("{}{}", c.to_uppercase(), &desc[1..]);
                    }
                    None => return None,
                }

                Some(desc)
            }
        } else {
            None
        }
    }
}

impl Block for WeatherBlock {
    fn update(&mut self) -> Result<(), UpdateError> {
        let mut wait_time_seconds = 1;

        // continually try to update with exponential falloff until we have a successful update
        loop {
            if let Err(e) = self.update_current_report() {
                eprintln!(
                    "couldn't update weather: {}. trying again in {} seconds",
                    e, wait_time_seconds
                )
            } else {
                break;
            }

            std::thread::sleep(std::time::Duration::from_secs(wait_time_seconds));

            if wait_time_seconds < self.update_interval_minutes as u64 * 60 {
                wait_time_seconds =
                    (wait_time_seconds * 2).min(self.update_interval_minutes as u64 * 60);
            }
        }

        self.next_update_time =
            Local::now() + chrono::Duration::minutes(self.update_interval_minutes as i64);

        Ok(())
    }

    fn name(&self) -> &str {
        "weather"
    }

    fn output(&self) -> Option<BlockOutputContent> {
        if let Some(r) = &self.current_report {
            Some(BlockOutputContent::from(NiceOutput {
                attention: Attention::Normal,
                icon: self.get_weather_icon(r),
                primary_text: self
                    .get_temperature_string()
                    .unwrap_or_else(|| "".to_string()),
                secondary_text: self.get_weather_description(),
            }))
        } else {
            None
        }
    }

    fn next_update_time(&self) -> Option<chrono::DateTime<chrono::Local>> {
        Some(self.next_update_time)
    }
}

/// Returns the external, public IP address of this device. The address is used to find the
/// device's current location.
pub fn get_external_ip() -> Result<String, MuseStatusError> {
    Ok(reqwest::blocking::get("http://checkip.amazonaws.com")?.text()?)
}
