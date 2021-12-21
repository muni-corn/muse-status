use std::borrow::Cow;

use super::color::Color;
use super::{Formatter, Mode};
use serde::{Deserialize, Serialize};

/// A sort of sub-block for easily creating output.
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Bit {
    /// The text of the Bit.
    pub text: String,

    /// The color of the Bit.
    pub color: Option<Color>,

    /// The font of the Bit.
    pub font: Option<String>,
}

impl Bit {
    /// Formats the Bit for lemonbar ouptut.
    pub fn as_lemonbar_string(&self, f: &Formatter) -> String {
        if let Some(c) = &self.color {
            let rgba = f.color_to_rgba(c);
            format!("%{{F#{}}}{}", rgba.hex_string(Mode::Lemonbar), self.text) // %{F#aabbccdd}text
        } else {
            self.text.clone()
        }
    }

    /// Formats the Bit as a pango string.
    pub fn as_pango_string(&self, f: &Formatter) -> String {
        let escaped = xml_escape(&self.text);
        match &self.color {
            Some(c) => {
                let rgba = f.color_to_rgba(c);
                match &self.font {
                    Some(f) => format!(
                        r##"<span color='#{}' font_desc='{}'>{}</span>"##,
                        rgba.hex_string(Mode::JsonProtocol),
                        f,
                        escaped
                    ), // color and font
                    None => format!(
                        r##"<span color='#{}'>{}</span>"##,
                        rgba.hex_string(Mode::JsonProtocol),
                        escaped
                    ), // color, but no font
                }
            }
            None => match &self.font {
                Some(f) => format!(r##"<span font_desc='{}'>{}</span>"##, f, escaped), // font, but no color
                None => escaped.to_string(), // no color and no font
            },
        }
    }
}

fn xml_escape(s: &str) -> Cow<str> {
    xml::escape::escape_str_attribute(s)
}
