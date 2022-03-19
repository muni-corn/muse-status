use crate::errors::*;
use crate::format::blocks::output::*;
use crate::format::blocks::*;
use crate::format::Attention;
use crate::utils;
use std::path::PathBuf;

const BASE_DIR: &str = "/sys/class/backlight/";
const BRIGHTNESS_ICONS: [char; 6] = [
    '\u{F00DB}',
    '\u{F00DC}',
    '\u{F00DD}',
    '\u{F00DE}',
    '\u{F00DF}',
    '\u{F00E0}',
];

/// BrightnessBlock is a block that contains device brightness information
pub struct BrightnessBlock {
    card: String,
    current_brightness: u32,
    max_brightness: u32,
}

impl BrightnessBlock {
    /// Returns a new BrightnessBlock that reads from the `card` specified
    pub fn new(card: &str) -> Self {
        let mut b = Self {
            card: card.to_owned(),
            current_brightness: 0,
            max_brightness: 0,
        };

        // ignore errors
        let _ = b.update_current_brightness();
        let _ = b.update_max_brightness();

        b
    }

    fn update_max_brightness(&mut self) -> Result<(), UpdateError> {
        let path = &PathBuf::from(BASE_DIR)
            .join(&self.card)
            .join("max_brightness");
        self.max_brightness = match utils::get_int_from_file(path) {
            Ok(b) => b as u32,
            Err(e) => {
                return Err(UpdateError {
                    block_name: self.name().to_string(),
                    message: format!("couldn't get max brightness: {}", e),
                })
            }
        };

        Ok(())
    }

    fn update_current_brightness(&mut self) -> Result<(), UpdateError> {
        self.current_brightness = match utils::get_int_from_file(
            &PathBuf::from(BASE_DIR).join(&self.card).join("brightness"),
        ) {
            Ok(b) => b as u32,
            Err(e) => {
                return Err(UpdateError {
                    block_name: self.name().to_string(),
                    message: format!("couldn't get max brightness: {}", e),
                })
            }
        };

        Ok(())
    }
}

impl Block for BrightnessBlock {
    fn name(&self) -> &str {
        "brightness"
    }

    fn update(&mut self) -> Result<(), UpdateError> {
        self.update_current_brightness()?;
        self.update_max_brightness()?; // because why not

        Ok(())
    }

    fn next_update(&self) -> Option<NextUpdate> {
        None
    }

    fn output(&self) -> Option<BlockOutputContent> {
        if self.max_brightness == 0 {
            return None;
        }

        let percent = self.current_brightness * 100 / self.max_brightness;
        let icon = get_icon(percent as u32);
        Some(BlockOutputContent::Nice(NiceOutput {
            icon,
            primary_text: format!("{}%", percent),
            secondary_text: None,
            attention: Attention::Dim,
        }))
    }
}

fn get_icon(percentage: u32) -> char {
    let index =
        (percentage as usize * BRIGHTNESS_ICONS.len() / 100).min(BRIGHTNESS_ICONS.len() - 1);

    BRIGHTNESS_ICONS[index]
}
