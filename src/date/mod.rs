use crate::errors::*;
use crate::format::blocks::output::*;
use crate::format::blocks::*;
use crate::format::Attention;
use chrono::prelude::*;
use chrono::{DateTime, Local};
use std::io::Cursor;
use rodio::{Device, Source};

// const TIME_FORMAT: &str = "%-I:%M %P";
const TIME_FORMAT: &str = "%-H:%M";
const DATE_FORMAT: &str = "%a, %b %-d";
const CLOCK_ICONS: [char; 12] = ['\u{F1456}', '\u{F144B}', '\u{F144C}', '\u{F144D}', '\u{F144E}', '\u{F144F}', '\u{F1450}', '\u{F1451}', '\u{F1452}', '\u{F1453}', '\u{F1454}', '\u{F1455}'];

/// Transmits time and date data.
pub struct DateBlock {
    now: DateTime<Local>,
    next_update: DateTime<Local>,
    last_hour: Option<u8>,

    audio_device: Device,
}

impl Default for DateBlock {
    fn default() -> Self {
        let now = chrono::Local::now();

        let audio_device = rodio::default_output_device().unwrap();

        Self {
            now,
            next_update: (now + chrono::Duration::minutes(1)).with_second(0).unwrap(), // don't hate me
            last_hour: None,
            audio_device,
        }
    }
}

impl DateBlock {
    /// Returns a new DateBlock.
    pub fn new() -> Self {
        Default::default()
    }

    fn play_new_hour_sound(&self) {
        let cursor = Cursor::new(include_bytes!("../new_hour.wav").as_ref());
        let source = rodio::Decoder::new(cursor).unwrap();
        rodio::play_raw(&self.audio_device, source.convert_samples());
    }
}

impl Block for DateBlock {
    /// Updates the clock
    fn update(&mut self) -> Result<(), UpdateError> {
        self.now = chrono::Local::now();
        self.next_update = (self.now + chrono::Duration::minutes(1))
            .with_second(0)
            .unwrap();

        if let Some(last_hour) = &self.last_hour {
            if *last_hour != self.now.hour() as u8 {
                if self.now.minute() == 0 {
                    self.play_new_hour_sound();
                }
                self.last_hour = Some(self.now.hour() as u8);
            }
        } else {
            self.last_hour = Some(self.now.hour() as u8);
        }

        Ok(())
    }

    fn name(&self) -> &str {
        "date"
    }

    fn next_update_time(&self) -> Option<DateTime<Local>> {
        Some(self.next_update)
    }

    fn output(&self) -> Option<BlockOutputContent> {
        let icon_index = self.now.hour() % 12;
        Some(BlockOutputContent::from(NiceOutput {
            icon: CLOCK_ICONS[icon_index as usize],
            primary_text: format!("{}", self.now.format(TIME_FORMAT)),
            secondary_text: Some(format!("{}", self.now.format(DATE_FORMAT))),
            attention: Attention::Normal,
        }))
    }
}

/// Returns a greeting based on the hour of the day
fn get_greeting() -> String {
    let hour = chrono::Local::now().hour();

    let greeting = if hour < 12 {
        "Good morning!"
    } else if hour >= 12 && hour < 17 {
        "Good afternoon!"
    } else {
        "Good evening!"
    };

    String::from(greeting)
}
