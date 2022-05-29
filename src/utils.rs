use crate::{
    errors::*,
    format::{color::RGBA, Mode},
};
use std::{fs, borrow::Cow};
use std::path::Path;

/// Returns a number in a file
pub fn get_int_from_file(filepath: &Path) -> Result<i32, MuseStatusError> {
    Ok(fs::read_to_string(filepath)?.trim().parse::<i32>()?)
}

/// Returns the reuslt of a concave-down cubic function
pub fn cubic_ease_arc(mut x: f32) -> f32 {
    x *= 2.0;
    x -= 1.0;
    let mut cubic = -(x * x * x).abs();

    cubic += 1.0;

    cubic
}

pub fn make_pango_string(
    text: &str,
    rgba: Option<RGBA>,
    font: Option<&str>,
) -> String {
    let escaped = xml_escape(text);
    match rgba {
        Some(c) => {
            match font {
                Some(f) => format!(
                    r##"<span color='#{}' font_desc='{}'>{}</span>"##,
                    c.hex_string(Mode::JsonProtocol),
                    f,
                    escaped
                ), // color and font
                None => format!(
                    r##"<span color='#{}'>{}</span>"##,
                    c.hex_string(Mode::JsonProtocol),
                    escaped
                ), // color, but no font
            }
        }
        None => match font {
            Some(f) => format!(r##"<span font_desc='{}'>{}</span>"##, f, escaped), // font, but no color
            None => escaped.to_string(), // no color and no font
        },
    }
}

fn xml_escape(s: &str) -> Cow<str> {
    xml::escape::escape_str_attribute(s)
}
