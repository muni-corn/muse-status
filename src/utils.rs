use std::fs;
use std::path::Path;
use crate::errors::*;

/// Returns a number in a file
pub fn get_int_from_file(filepath: &Path) -> Result<i32, MuseStatusError> {
    Ok(fs::read_to_string(filepath)?.parse::<i32>()?)
}

/// Returns the reuslt of a concave-down cubic function
pub fn cubic_ease_arc(mut x: f32) -> f32 {
    x *= 2.0;
    x -= 1.0;
    let mut cubic = -(x * x * x).abs();

    cubic += 1.0;

    cubic
}

