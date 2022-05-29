use serde::Deserialize;

#[derive(Deserialize)]
pub struct WeatherLocation {
    pub latitude: f32,
    pub longitude: f32,
}

#[derive(Deserialize)]
pub struct SunTimeData {
    pub sunrise: i64,
    pub sunset: i64,
}

#[derive(Deserialize)]
pub struct WeatherDetails {
    pub description: String,
    pub icon: String,
}

#[derive(Deserialize)]
pub struct WeatherWind {
    pub speed: f32,
    pub deg: f32,
}

#[derive(Deserialize)]
pub struct WeatherMain {
    pub temp: f32,
}

#[derive(Deserialize)]
pub struct FullWeatherReport {
    pub sys: SunTimeData,
    pub weather: Vec<WeatherDetails>,
    pub main: WeatherMain,
    pub wind: WeatherWind,
}

impl FullWeatherReport {
    /// Returns a number with a little circle-thing next to it.
    pub fn temperature_string(&self) -> String {
        format!("{}°", self.main.temp.round() as i32)
    }

    /// Returns a String-ified weather description, in Sentence case.
    pub fn description(&self) -> Option<String> {
        self.weather.first().map(|w| {
            // capitalize the first letter in the description
            let mut chars = w.description.chars();
            if let Some(first_char) = chars.next() {
                first_char.to_ascii_uppercase().to_string() + chars.as_str()
            } else {
                // if there wasn't a first char, there must've been no description
                String::from("No description")
            }
        })
    }
}
