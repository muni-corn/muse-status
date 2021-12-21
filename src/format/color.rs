use super::Mode;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Color doesn't represent colors as numbers; instead, it is an enum for types of
/// colors encountered in muse-status.
#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum Color {
    /// The primary color.
    Primary,

    /// The secondary color.
    Secondary,

    /// The warning color.
    Warning,

    /// The alarm color.
    Alarm,

    /// A custom rgba color.
    Other(RGBA),
}

/// Represents an RGBA color using bytes, each 0 - 255.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RGBA {
    /// Red.
    pub r: u8,

    /// Green.
    pub g: u8,

    /// Blue.
    pub b: u8,

    /// Alpha.
    pub a: u8,
}

impl FromStr for RGBA {
    type Err = RGBAParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // remove any characters that aren't hex digits (like '#')
        // 50 points to Rust for including char::is_ascii_hexdigit :D:D:D:D
        let raw: String = s.chars().filter(|c| c.is_ascii_hexdigit()).collect();
        match raw.len() {
            6 => {
                let raw_int = i32::from_str_radix(&raw, 16)?;
                Ok(Self {
                    r: (raw_int >> 16 & 0xff) as u8,
                    g: (raw_int >> 8 & 0xff) as u8,
                    b: (raw_int & 0xff) as u8,
                    a: 255,
                })
            }
            8 => {
                let raw_int = i32::from_str_radix(&raw, 16)?;
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

/// Sinister red.
pub const ALARM_COLOR: RGBA = RGBA {
    r: 0xff,
    g: 0,
    b: 0,
    a: 0xff,
};

/// Vibrant orange.
pub const WARNING_COLOR: RGBA = RGBA {
    r: 0xff,
    g: 0xaa,
    b: 0,
    a: 0xff,
};

/// Transparent black.
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
                "{:08x}",
                u32::from_be_bytes([self.a, self.r, self.g, self.b])
            ),
            _ => format!(
                "{:08x}",
                u32::from_be_bytes([self.r, self.g, self.b, self.a])
            ),
        }
    }
}

/// Mixes two colors together. Interpolation determines how much of either `first` (0.0) or
/// `second` (1.0) is in the result.
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

/// Returned when muse-status has an issue parsing an RGBA struct from a hex string, e.g. rrggbbaa
#[derive(Debug)]
pub enum RGBAParseError {
    /// The String has an invalid length to be a color.
    BadString(String),

    /// There was an error parsing a hex string as an integer.
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
