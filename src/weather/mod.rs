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
    current_report: Option<WttrReport>,
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
        }
    }

    fn get_weather_icon(&self, report: &WttrReport) -> char {
        *self
            .config
            .weather_icons
            .get(&report.weather_code)
            .unwrap_or(&self.config.default_icon)
    }

    fn update_current_report(&mut self) -> Result<(), UpdateError> {
        self.current_report = reqwest::blocking::get("https://wttr.in/?format=j1")
            .and_then(|res| res.json::<WttrReport>())
            .map_err(|e| UpdateError {
                block_name: self.name().to_string(),
                message: format!("couldn't retrieve weather data: {}", e),
            })
            .map(Option::Some)?;

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
            let temp_string = r.temperature_string(self.config.units);

            let text = if let Some(desc) = r.description() {
                BlockText::Pair(temp_string, desc.to_string())
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
