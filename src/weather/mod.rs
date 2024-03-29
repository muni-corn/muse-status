mod structs;

use crate::{
    config::WeatherConfig,
    errors::*,
    format::{
        blocks::{output::*, Block, NextUpdate},
        Attention,
    },
};
use chrono::Duration;
use serde::{Deserialize, Serialize};
use structs::*;

/// Type of units to use when reporting locale-specific measurements.
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Units {
    /// Freedom units.
    Imperial,

    /// Non-US units.
    Metric,
}

impl Units {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Imperial => "imperial",
            Self::Metric => "metric",
        }
    }
}

/// WeatherBlock returns information about the weather around the user's current location.
/// OpenWeatherMap and IPStack are used for weather and location respectively.
pub struct WeatherBlock {
    config: WeatherConfig,

    current_report: Option<FullWeatherReport>,
    location: Option<WeatherLocation>,
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
            config,

            current_report: None,
            location: None,
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
            ip, self.config.ipstack_key
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
        report
            .weather
            .first()
            .map(|r| {
                let icon_string = &r.icon;
                self.config.weather_icons[icon_string]
            })
            .unwrap_or(self.config.default_icon)
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
                    l.latitude, l.longitude, self.config.openweathermap_key, self.config.units.as_str()
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
}

impl Block for WeatherBlock {
    fn update(&mut self) -> Result<(), UpdateError> {
        let mut wait_time_seconds = 1;

        // continually try to update with exponential falloff until we have a successful update
        while let Err(e) = self.update_current_report() {
            eprintln!(
                "couldn't update weather: {}. trying again in {} seconds",
                e, wait_time_seconds
            );

            std::thread::sleep(std::time::Duration::from_secs(wait_time_seconds));

            if wait_time_seconds < self.config.update_interval_minutes as u64 * 60 {
                wait_time_seconds =
                    (wait_time_seconds * 2).min(self.config.update_interval_minutes as u64 * 60);
            }
        }

        Ok(())
    }

    fn name(&self) -> &str {
        "weather"
    }

    fn output(&self) -> Option<BlockOutput> {
        self.current_report.as_ref().map(|r| {
            let temp_string = r.temperature_string();

            let text = if let Some(desc) = r.description() {
                BlockText::Pair(temp_string, desc)
            } else {
                BlockText::Single(temp_string)
            };
            BlockOutput::new(
                self.name(),
                Some(self.get_weather_icon(r)),
                text,
                Attention::Normal,
            )
        })
    }

    fn next_update(&self) -> Option<NextUpdate> {
        Some(NextUpdate::In(Duration::minutes(
            self.config.update_interval_minutes.into(),
        )))
    }
}

/// Returns the external, public IP address of this device. The address is used to find the
/// device's current location.
pub fn get_external_ip() -> Result<String, MuseStatusError> {
    Ok(reqwest::blocking::get("http://ifconfig.me")?.text()?)
}
