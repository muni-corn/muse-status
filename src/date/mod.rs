use crate::{
    errors::*,
    format::{
        blocks::{output::*, *},
        Attention,
    },
};
use chrono::{prelude::*, DateTime, Duration, Local};

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
        let now = Local::now();
        let next_update = next_minute_or_five_seconds();

        Self { now, next_update }
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
        self.now = Local::now();
        self.next_update = get_next_minute();

        Ok(())
    }

    fn name(&self) -> &str {
        "date"
    }

    fn next_update(&self) -> Option<NextUpdate> {
        Some(NextUpdate::At(self.next_update))
    }

    fn output(&self) -> Option<BlockOutput> {
        let icon = {
            let index = self.now.hour() % 12;
            CLOCK_ICONS[index as usize]
        };
        let time = format!("{}", self.now.format(TIME_FORMAT));
        let date = format!("{}", self.now.format(DATE_FORMAT));
        let text = BlockText::Pair(time, date);

        Some(BlockOutput::new(self.name(), Some(icon), text, Attention::Normal))
    }
}

/// Returns a greeting based on the hour of the day
#[allow(dead_code)]
fn get_greeting() -> String {
    let hour = Local::now().hour();

    let greeting = if hour < 12 {
        "Good morning!"
    } else if (12..17).contains(&hour) {
        "Good afternoon!"
    } else {
        "Good evening!"
    };

    String::from(greeting)
}

/// Returns the time of the next minute of the hour.
fn get_next_minute() -> DateTime<Local> {
    let now = Local::now();
    let in_one_minute = now + Duration::minutes(1);
    if let Some(truncated) = in_one_minute.with_second(0) {
        truncated
    } else {
        // default to an un-truncated minute ahead of time (as if that would be valid if
        // `with_second` failed)
        in_one_minute
    }
}

/// Returns a time that is either at the next minute of the hour or in five seconds, whichever
/// comes first.
fn next_minute_or_five_seconds() -> DateTime<Local> {
    let now = Local::now();
    let next_minute = get_next_minute();
    let in_five_seconds = now + Duration::seconds(5);

    next_minute.min(in_five_seconds)
}
