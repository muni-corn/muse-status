mod structs;

use crate::errors::*;
use crate::format::blocks::output::*;
use crate::format::blocks::Block;
use crate::format::Attention;
use chrono::Local;
use std::collections::HashMap;
use structs::*;
use std::fmt;

const IP_STACK_KEY: &str = "9c237911bdacce2e8c9a021d9b4c1317";

enum Units {
    Imperial,
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
    current_report: Option<FullWeatherReport>,
    location: Option<WeatherLocation>,
    units: Units,

    update_interval_minutes: i32,

    weather_icons: HashMap<String, char>,
    default_icon: char,

    next_update_time: chrono::DateTime<chrono::Local>,
}

impl Default for WeatherBlock {
    fn default() -> Self {
        let mut weather_icons = HashMap::<String, char>::new();
        weather_icons.insert(String::from("01d"), '');
        weather_icons.insert(String::from("01n"), '');
        weather_icons.insert(String::from("02d"), '');
        weather_icons.insert(String::from("02n"), '');
        weather_icons.insert(String::from("03d"), '');
        weather_icons.insert(String::from("03n"), '');
        weather_icons.insert(String::from("04d"), '');
        weather_icons.insert(String::from("04n"), '');
        weather_icons.insert(String::from("09d"), '');
        weather_icons.insert(String::from("09n"), '');
        weather_icons.insert(String::from("10d"), '');
        weather_icons.insert(String::from("10n"), '');
        weather_icons.insert(String::from("11d"), '');
        weather_icons.insert(String::from("11n"), '');
        weather_icons.insert(String::from("13d"), '');
        weather_icons.insert(String::from("13n"), '');
        weather_icons.insert(String::from("50d"), '');
        weather_icons.insert(String::from("50n"), '');

        let default_icon = '\u{f50f}';
        let openweathermap_key = "d179cc80ed41e8080f9e86356b604ee3"; // TODO
        let units = Units::Imperial;

        Self {
            current_report: None,
            location: None,
            weather_icons,
            default_icon,
            openweathermap_key: openweathermap_key.to_string(),
            update_interval_minutes: 20,
            units,
            next_update_time: Local::now() + chrono::Duration::minutes(20),
        }
    }
}

impl WeatherBlock {
    /// Creates a new weather block.
    pub fn new() -> Self {
        Default::default()
    }

    /// Creates a new weather block, but with a custom location.
    pub fn new_with_location(location: WeatherLocation) -> Self {
        let mut w = Self::new();
        w.current_report = None;
        w.location = Some(location);

        w
    }

    fn get_current_location() -> Result<WeatherLocation, MuseStatusError> {
        let ip = get_external_ip()?;

        let url = format!(
            "http://api.ipstack.com/{}?access_key={}&format=1",
            ip, IP_STACK_KEY
        );

        let res = reqwest::blocking::get(&url)?;

        match serde_json::from_str::<WeatherLocation>(&res.text()?) {
            Ok(r) => Ok(r),
            Err(e) => Err(MuseStatusError::from(BasicError {
                message: format!("could deserialize current location from ipstack: {}", e),
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
            self.location = match Self::get_current_location() {
                Ok(l) => Some(l),
                Err(e) => {
                    return Err(UpdateError {
                        block_name: self.name().to_owned(),
                        message: format!("couldn't get current location: {}", e),
                    })
                }
            };
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
        self.next_update_time =
            Local::now() + chrono::Duration::minutes(self.update_interval_minutes as i64);
        self.update_current_report()?;

        Ok(())
    }

    fn name(&self) -> &str {
        "weather"
    }

    fn output(&self) -> Option<BlockOutputBody> {
        if let Some(r) = &self.current_report {
            Some(BlockOutputBody::from(NiceOutput {
                attention: Attention::Normal,
                icon: self.get_weather_icon(r),
                primary_text: self.get_temperature_string().unwrap_or_else(|| "".to_string()),
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
