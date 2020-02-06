use crate::format::blocks::*;
use crate::format::blocks::output::BlockOutputBody;
use std::process;
use crate::errors::*;

/// VolumeBlock provides information for the system's audio volume. Requires `amixer`.
#[derive(Default)]
pub struct VolumeBlock {
    current_volume: i32,
}

impl VolumeBlock {
    /// Returns a new VolumeBlock. By default, it gets info for the Master bus via `amixer`.
    pub fn new() -> Self {
        Default::default()
    }
}

impl Block for VolumeBlock {
    fn update(&mut self) -> Result<(), UpdateError> {
        self.current_volume = match get_current_volume() {
            Ok(v) => v,
            Err(e) => return Err(UpdateError {
                block_name: self.name().to_string(),
                message: format!("{}", e)
            })
        };
        Ok(())
    }

    fn name(&self) -> &str {
        "volume"
    }

    fn next_update_time(&self) -> Option<chrono::DateTime<chrono::Local>> {
        None
    }
    
    fn output(&self) -> Option<BlockOutputBody> {
        Some(BlockOutputBody::Custom(format!("{}  {}%", get_icon(self.current_volume), self.current_volume)))
    }
}

const VOLUME_ICONS: [char; 3] = ['', '', ''];
const MUTE_ICON: char = '';

// returns the current volume percentage as an i32, or zero
// if muted
fn get_current_volume() -> Result<i32, UpdateError> {
    let output = process::Command::new("amixer").args(&["sget", "Master"]).output().unwrap();
    let info = String::from_utf8(output.stdout).unwrap();

    match info.lines().last().unwrap().chars().position(|c| c == '[') {
        Some(i) => {
            // filters out any non-digit characters past the first opening bracket to parse the
            // volume amount
            Ok(info[i..].chars().filter(|c| c.is_digit(10)).collect::<String>().parse::<i32>().unwrap())
        },
        None => {
            Err(UpdateError {
                block_name: String::from("volume"),
                message: String::from("couldn't parse amixer output"),
            })
        }
    }
}

fn get_icon(percentage: i32) -> char {
    if percentage <= 0 {
        MUTE_ICON
    } else {
        let mut index = percentage * VOLUME_ICONS.len() as i32 / 100;

        // constrain index (should never go below zero)
        if index >= VOLUME_ICONS.len() as i32 {
            index = VOLUME_ICONS.len() as i32 - 1;
        }

        VOLUME_ICONS[index as usize]
    }
}

// vim: foldmethod=marker
