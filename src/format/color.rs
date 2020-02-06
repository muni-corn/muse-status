use super::Mode;
use std::fmt;
use std::str::FromStr;

pub enum Color {
    Primary,
    Secondary,
    Warning,
    Alarm,
    Other(RGBA),
}

pub struct RGBA {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl FromStr for RGBA {
    type Err = RGBAParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.len() {
            6 => {
                let raw_int = i32::from_str_radix(s, 16)?;
                Ok(Self {
                    r: (raw_int >> 16 & 0xff) as u8,
                    g: (raw_int >> 8 & 0xff) as u8,
                    b: (raw_int & 0xff) as u8,
                    a: 255,
                })
            }
            8 => {
                let raw_int = i32::from_str_radix(s, 16)?;
                Ok(Self {
                    r: (raw_int >> 24 & 0xff) as u8,
                    g: (raw_int >> 16 & 0xff) as u8,
                    b: (raw_int >> 8 & 0xff) as u8,
                    a: (raw_int & 0xff) as u8,
                })
            }
            _ => Err(RGBAParseError::new(s)),
        }
    }
}

pub const ALARM_COLOR: RGBA = RGBA {
    r: 0xff,
    g: 0,
    b: 0,
    a: 0xff,
};

pub const WARNING_COLOR: RGBA = RGBA {
    r: 0xff,
    g: 0xaa,
    b: 0,
    a: 0xff,
};

pub const TRANSPARENT_COLOR: RGBA = RGBA {
    r: 0,
    g: 0,
    b: 0,
    a: 0,
};

impl Default for RGBA {
    fn default() -> Self {
        Self {
            r: 0,
            g: 0,
            b: 0,
            a: 255,
        }
    }
}

impl RGBA {
    /// Returns a hex representation of the string (no #), depending on
    /// the mode that muse-status is in
    pub fn hex_string(&self, mode: Mode) -> String {
        match mode {
            Mode::Lemonbar => format!(
                "#{:x}",
                u32::from_be_bytes([self.a, self.r, self.g, self.b])
            ),
            Mode::JsonProtocol => format!(
                "#{:x}",
                u32::from_be_bytes([self.r, self.g, self.b, self.a])
            ),
        }
    }
}

pub fn interpolate_colors(first: &RGBA, second: &RGBA, interpolation: f32) -> RGBA {
    let a = (first.a as f32) * (1.0 - interpolation) + (second.a as f32) * interpolation;
    let r = (first.r as f32) * (1.0 - interpolation) + (second.r as f32) * interpolation;
    let g = (first.g as f32) * (1.0 - interpolation) + (second.g as f32) * interpolation;
    let b = (first.b as f32) * (1.0 - interpolation) + (second.b as f32) * interpolation;

    RGBA {
        a: a as u8,
        r: r as u8,
        g: g as u8,
        b: b as u8,
    }
}

pub enum RGBAParseError {
    BadString(String),
    IntParse(std::num::ParseIntError),
}

impl RGBAParseError {
    fn new(rgba: &str) -> Self {
        Self::BadString(String::from(rgba))
    }
}

impl From<std::num::ParseIntError> for RGBAParseError {
    fn from(e: std::num::ParseIntError) -> Self {
        Self::IntParse(e)
    }
}

impl fmt::Display for RGBAParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::BadString(rgba) => write!(f, "`{}` is not a valid hex color", rgba),
            Self::IntParse(e) => e.fmt(f),
        }
    }
}
