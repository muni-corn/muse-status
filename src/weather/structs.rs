use chrono::NaiveTime;
use serde::Deserialize;

use super::Units;

#[derive(Deserialize)]
pub struct WrappedValue {
    pub value: String,
}

#[derive(Deserialize)]
pub struct CurrentCondition {
    pub humidity: String,
    pub pressure: String,
    pub visibility: String,

    #[serde(rename = "FeelsLikeC")]
    pub feels_like_c: String,

    #[serde(rename = "FeelsLikeF")]
    pub feels_like_f: String,

    #[serde(rename = "cloudcover")]
    pub cloud_cover: String,

    #[serde(rename = "observation_time")]
    pub observation_time: String,

    #[serde(rename = "precipMM")]
    pub precip_mm: String,

    #[serde(rename = "temp_C")]
    pub temp_c: String,

    #[serde(rename = "temp_F")]
    pub temp_f: String,

    #[serde(rename = "uvIndex")]
    pub uv_index: String,

    #[serde(rename = "weatherCode")]
    pub weather_code: String,

    #[serde(rename = "weatherDesc")]
    pub weather_desc: Vec<WrappedValue>,

    #[serde(rename = "weatherIconUrl")]
    pub weather_icon_url: Vec<WrappedValue>,

    #[serde(rename = "winddir16Point")]
    pub wind_dir_16p: String,

    #[serde(rename = "winddirDegree")]
    pub wind_dir_degree: String,

    #[serde(rename = "windspeedKmph")]
    pub wind_speed_kmph: String,

    #[serde(rename = "windspeedMiles")]
    pub windspeed_miles: String,
}

#[derive(Deserialize)]
pub struct Weather {
    pub astronomy: Vec<Astronomy>,
}
#[derive(Deserialize)]
pub struct Astronomy {
    pub sunrise: NaiveTime,
    pub sunset: NaiveTime,
}

#[derive(Deserialize)]
pub struct WttrReport {
    pub current_condition: Vec<CurrentCondition>,
    pub weather: Vec<Weather>,
}

impl WttrReport {
    /// Returns a number with a little circle-thing next to it.
    pub fn temperature_string(&self, units: Units) -> Option<String> {
        self.current_condition.first().map(|c| {
            let value = match units {
                Units::Imperial => c.temp_f.as_str(),
                Units::Metric => c.temp_c.as_str(),
            };

            format!("{}°", value)
        })
    }

    /// Returns the weather description in Sentence case.
    pub fn description(&self) -> Option<&str> {
        self.current_condition
            .first()
            .and_then(|c| c.weather_desc.first().map(|w| w.value.as_str()))
    }

    pub fn weather_code(&self) -> Option<&str> {
        self.current_condition
            .first()
            .map(|c| c.weather_code.as_str())
    }
}
