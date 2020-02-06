use serde::Deserialize;

#[derive(Deserialize)]
pub struct WeatherLocation {
    pub latitude: f32,
    pub longitude: f32,
}

#[derive(Deserialize)]
pub struct FullWeatherReport {
    pub sys: SunTimeData,
    pub weather: Vec<WeatherDetails>,
    pub main: WeatherMain,
    pub wind: WeatherWind,
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
