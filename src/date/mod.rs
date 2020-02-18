use crate::errors::*;
use crate::format::blocks::output::*;
use crate::format::blocks::*;
use crate::format::Attention;
use chrono::prelude::*;
use chrono::{DateTime, Local};

const TIME_FORMAT: &str = "%-I:%M %P";
const DATE_FORMAT: &str = "%a, %b %-d";
const ICON: char = '\u{f150}';

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

    fn output(&self) -> Option<BlockOutputBody> {
        Some(BlockOutputBody::from(NiceOutput {
            icon: ICON,
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
