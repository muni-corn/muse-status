use crate::errors::*;
use crate::format::blocks::output::{BlockOutputBody, NiceOutput};
use crate::format::blocks::Block;
use crate::format::Attention;
use std::process;

/// VolumeBlock provides information for the system's audio volume. Requires `amixer`.
#[derive(Default)]
pub struct VolumeBlock {
    current_volume: i32,
    muted: bool,
}

impl VolumeBlock {
    /// Returns a new VolumeBlock. By default, it gets info for the Master bus via `amixer`.
    pub fn new() -> Self {
        Default::default()
    }

    // returns the current volume percentage as an i32, or zero
    // if muted
    fn update_current_volume(&mut self) -> Result<(), UpdateError> {
        let output = process::Command::new("amixer")
            .args(&["sget", "Master"])
            .output()
            .unwrap();
        let info = String::from_utf8(output.stdout).unwrap();
        let last_line = info.lines().last().unwrap();

        match last_line.chars().position(|c| c == '[') {
            Some(i) => {
                let line_end = &last_line[i..];

                // first, are we muted?
                self.muted = if line_end.contains("on") {
                    false
                } else if line_end.contains("off") {
                    true
                } else {
                    return Err(UpdateError {
                        block_name: String::from("volume"),
                        message: String::from(
                            "couldn't parse if volume is definitely muted or not",
                        ),
                    });
                };

                if !self.muted {
                    // filters out any non-digit characters past the first opening bracket to parse the
                    // volume amount
                    let raw_percent = line_end
                        .chars()
                        .filter(|c| c.is_digit(10))
                        .collect::<String>();

                    self.current_volume = match raw_percent.parse::<i32>() {
                        Ok(p) => p,
                        Err(e) => {
                            return Err(UpdateError {
                                block_name: String::from("volume"),
                                message: format!(
                                    "couldn't parse volume from `{}`: {}",
                                    raw_percent, e
                                ),
                            })
                        }
                    };
                }

                Ok(())
            }
            None => Err(UpdateError {
                block_name: String::from("volume"),
                message: String::from("couldn't parse amixer output"),
            }),
        }
    }

    fn get_icon(&self) -> char {
        if self.muted || self.current_volume == 0 {
            MUTE_ICON
        } else {
            let index = (self.current_volume as usize * VOLUME_ICONS.len() / 100)
                .min(VOLUME_ICONS.len() - 1);

            VOLUME_ICONS[index]
        }
    }

    // vim: foldmethod=marker
}

impl Block for VolumeBlock {
    fn update(&mut self) -> Result<(), UpdateError> {
        self.update_current_volume()
    }

    fn name(&self) -> &str {
        "volume"
    }

    fn next_update_time(&self) -> Option<chrono::DateTime<chrono::Local>> {
        None
    }

    fn output(&self) -> Option<BlockOutputBody> {
        Some(BlockOutputBody::Nice(NiceOutput {
            icon: self.get_icon(),
            primary_text: if self.muted || self.current_volume == 0 {
                String::from("Muted")
            } else {
                format!("{}%", self.current_volume)
            },
            secondary_text: None,
            attention: Attention::Dim,
        }))
    }
}

const VOLUME_ICONS: [char; 3] = ['', '', ''];
const MUTE_ICON: char = '';
