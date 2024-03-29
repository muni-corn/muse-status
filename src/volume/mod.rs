use crate::errors::*;
use crate::format::blocks::output::{BlockOutput, BlockText};
use crate::format::blocks::{Block, NextUpdate};
use crate::format::Attention;
use std::process::Command;

/// Enums are great
#[derive(Debug, Eq, PartialEq)]
pub enum Volume {
    /// Volume is unmuted with a value from 0 to 100 (maybe more).
    On(i32),

    /// Volume is muted.
    Off,
}

impl Default for Volume {
    fn default() -> Self {
        Self::Off
    }
}

/// VolumeBlock provides information for the system's audio volume. Requires `amixer`.
#[derive(Default)]
pub struct VolumeBlock {
    volume_sink: Option<String>,
    current_volume: Volume,
}

impl VolumeBlock {
    /// Returns a new VolumeBlock which uses the specified sink.
    pub fn new(volume_sink: &str) -> Self {
        Self {
            volume_sink: Some(volume_sink.to_string()),
            ..Default::default()
        }
    }

    const MAX_WAIT_SECONDS: u64 = 30;

    /// Gets the system volume from the `pamixer` command
    fn volume_from_pamixer(&self) -> Result<Volume, UpdateError> {
        let mut get_mute_command_args = vec!["--get-mute"];
        if let Some(sink) = &self.volume_sink {
            #[cfg(debug_assertions)]
            {
                eprintln!("getting mute from sink '{}'", sink);
            }
            get_mute_command_args.push("--sink");
            get_mute_command_args.push(sink);
        } else {
            #[cfg(debug_assertions)]
            {
                eprintln!("getting mute from default sink");
            }
        }

        let muted = Command::new("pamixer")
            .args(&get_mute_command_args)
            .output()
            .map(|output| {
                String::from_utf8(output.stdout).map(|b| {
                    #[cfg(debug_assertions)]
                    {
                        println!("parsing `{}` as bool", b.trim())
                    }
                    b.trim().parse::<bool>()
                })
            })
            .map_err(|e| UpdateError {
                block_name: "volume".to_string(),
                message: format!("{}", e),
            })?
            .map_err(|e| UpdateError {
                block_name: "volume".to_string(),
                message: format!("{}", e),
            })?
            .map_err(|e| UpdateError {
                block_name: "volume".to_string(),
                message: format!("{}", e),
            })?;

        if muted {
            Ok(Volume::Off)
        } else {
            let get_volume_command_args = if let Some(sink) = &self.volume_sink {
                #[cfg(debug_assertions)]
                {
                    eprintln!("getting volume from sink '{}'", sink);
                }
                vec!["--get-volume", "--sink", sink]
            } else {
                #[cfg(debug_assertions)]
                {
                    eprintln!("getting volume from default sink");
                }
                vec!["--get-volume"]
            };

            let num = Command::new("pamixer")
                .args(&get_volume_command_args)
                .output()
                .map(|output| {
                    String::from_utf8(output.stdout).map(|num| {
                        #[cfg(debug_assertions)]
                        {
                            println!("parsing `{}` as i32", num.trim())
                        }
                        num.trim().parse::<i32>()
                    })
                })
                .map_err(|e| UpdateError {
                    block_name: "volume".to_string(),
                    message: format!("{}", e),
                })?
                .map_err(|e| UpdateError {
                    block_name: "volume".to_string(),
                    message: format!("{}", e),
                })?
                .map_err(|e| UpdateError {
                    block_name: "volume".to_string(),
                    message: format!("{}", e),
                })?;

            Ok(Volume::On(num))
        }
    }

    /// Gets the system volume from the `amixer` command
    fn volume_from_amixer(&self) -> Result<Volume, UpdateError> {
        let raw_string_opt = Command::new("amixer")
            .args(["sget", "Master"])
            .output()
            .map(|output| {
                String::from_utf8(output.stdout)
                    .map(|info| info.lines().last().map(|last_line| last_line.to_string()))
            })
            .map_err(|e| UpdateError {
                block_name: "volume".to_string(),
                message: format!("{}", e),
            })?
            .map_err(|e| UpdateError {
                block_name: "volume".to_string(),
                message: format!("{}", e),
            })?;

        if let Some(raw_string) = raw_string_opt {
            match raw_string.chars().position(|c| c == '[') {
                Some(i) => {
                    let line_end = &raw_string[i..];

                    if line_end.contains("off") {
                        Ok(Volume::Off)
                    } else {
                        // filters out any non-digit characters past the first opening bracket to parse the
                        // volume amount
                        let raw_percent = line_end
                            .chars()
                            .filter(|c| c.is_ascii_digit())
                            .collect::<String>();

                        let current_volume =
                            raw_percent.parse::<i32>().map_err(|e| UpdateError {
                                block_name: String::from("volume"),
                                message: format!(
                                    "couldn't parse volume from `{}`: {}",
                                    raw_percent, e
                                ),
                            })?;

                        Ok(Volume::On(current_volume))
                    }
                }
                None => Err(UpdateError {
                    block_name: String::from("volume"),
                    message: String::from(
                        "couldn't find square bracket delimiter in amixer output",
                    ),
                }),
            }
        } else {
            Err(UpdateError {
                block_name: String::from("volume"),
                message: String::from("couldn't get any output to parse from (amixer)"),
            })
        }
    }

    fn get_icon(&self) -> char {
        match self.current_volume {
            Volume::On(0) => ZERO_ICON,
            Volume::On(x) => {
                let index = (x as usize * VOLUME_ICONS.len() / 100).min(VOLUME_ICONS.len() - 1);

                VOLUME_ICONS[index]
            }
            Volume::Off => MUTE_ICON,
        }
    }

    fn get_text(&self) -> String {
        match self.current_volume {
            Volume::Off | Volume::On(0) => String::from("Muted"),
            Volume::On(x) => format!("{}%", x),
        }
    }

    // vim: foldmethod=marker
}

impl Block for VolumeBlock {
    fn update(&mut self) -> Result<(), UpdateError> {
        let mut wait_time_seconds = 1;
        self.current_volume = loop {
            // try `pamixer` first
            match self.volume_from_pamixer() {
                Ok(vol) => break vol,
                Err(_e) => {
                    #[cfg(debug_assertions)]
                    {
                        eprintln!("{}", _e);
                    }

                    // fallback to `amixer` if there's an error
                    match self.volume_from_amixer() {
                        Ok(vol) => break vol,
                        Err(_e1) => {
                            #[cfg(debug_assertions)]
                            {
                                eprintln!("{}", _e1);
                            }
                        }
                    }
                }
            }

            // exponential falloff
            std::thread::sleep(std::time::Duration::from_secs(wait_time_seconds));
            if wait_time_seconds < Self::MAX_WAIT_SECONDS {
                wait_time_seconds = Self::MAX_WAIT_SECONDS.min(wait_time_seconds * 2);
            }
        };

        Ok(())
    }

    fn name(&self) -> &str {
        "volume"
    }

    fn next_update(&self) -> Option<NextUpdate> {
        None
    }

    fn output(&self) -> Option<BlockOutput> {
        Some(BlockOutput::new(
            self.name(),
            Some(self.get_icon()),
            BlockText::Single(self.get_text()),
            Attention::Dim,
        ))
    }
}

const VOLUME_ICONS: [char; 3] = ['\u{F057F}', '\u{F0580}', '\u{F057E}'];
const MUTE_ICON: char = '\u{F0581}';
const ZERO_ICON: char = '\u{F0E08}';
