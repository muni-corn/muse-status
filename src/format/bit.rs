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
            let rgba = f.color_to_rgba(&c);
            format!("%{{F#{}}}{}", rgba.hex_string(Mode::Lemonbar), self.text) // %{F#aabbccdd}text
        } else {
            self.text.clone()
        }
    }

    /// Formats the Bit as a pango string.
    pub fn as_pango_string(&self, f: &Formatter) -> String {
        match &self.color {
            Some(c) => {
                let rgba = f.color_to_rgba(&c);
                match &self.font {
                    Some(f) => format!(
                        "<span color=\"#{}\" font_desc=\"{}\">{}</span>",
                        rgba.hex_string(Mode::JsonProtocol),
                        f,
                        self.text
                    ), // color and font
                    None => format!(
                        "<span color=\"#{}\">{}</span>",
                        rgba.hex_string(Mode::JsonProtocol),
                        self.text
                    ), // color, but no font
                }
            }
            None => match &self.font {
                Some(f) => format!("<span font_desc=\"{}\">{}</span>", f, self.text), // font, but no color
                None => self.text.clone(), // no color and no font
            },
        }
    }
}
