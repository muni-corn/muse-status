use crate::errors::*;
use crate::format::blocks::output::*;
use crate::format::blocks::*;
use crate::format::Attention;
use chrono::prelude::*;
use chrono::{DateTime, Local};

/// The format with which to format time strings.
pub const TIME_FORMAT: &str = "%-I:%M %P";
const DATE_FORMAT: &str = "%a, %b %-d";
const CLOCK_ICONS: [char; 12] = [
    '\u{F1456}',
    '\u{F144B}',
    '\u{F144C}',
    '\u{F144D}',
    '\u{F144E}',
    '\u{F144F}',
    '\u{F1450}',
    '\u{F1451}',
    '\u{F1452}',
    '\u{F1453}',
    '\u{F1454}',
    '\u{F1455}',
];

/// Transmits time and date data.
pub struct DateBlock {
    now: DateTime<Local>,
    next_update: DateTime<Local>,
}

impl Default for DateBlock {
    fn default() -> Self {
        let now = chrono::Local::now();

        Self {
            now,
            next_update: (now + chrono::Duration::minutes(1)).with_second(0).unwrap(), // don't hate me
        }
    }
}

impl DateBlock {
    /// Returns a new DateBlock.
    pub fn new() -> Self {
        Default::default()
    }
}

impl Block for DateBlock {
    /// Updates the clock
    fn update(&mut self) -> Result<(), UpdateError> {
        self.now = chrono::Local::now();
        self.next_update = (self.now + chrono::Duration::minutes(1))
            .with_second(0)
            .unwrap();

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
#[allow(dead_code)]
fn get_greeting() -> String {
    let hour = chrono::Local::now().hour();

    let greeting = if hour < 12 {
        "Good morning!"
    } else if (12..17).contains(&hour) {
        "Good afternoon!"
    } else {
        "Good evening!"
    };

    String::from(greeting)
}
