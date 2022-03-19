use crate::{
    config::BatteryConfig,
    errors::*,
    format::{
        blocks::{output::*, *},
        Attention,
    },
};
use chrono::{DateTime, Duration, Local};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// The status of a battery.
#[derive(Clone, PartialEq)]
pub enum ChargeStatus {
    /// The battery is discharging.
    Discharging,

    /// The battery is charging.
    Charging,

    /// The status of the battery isn't known.
    Unknown,

    /// The battery is full and still plugged in.
    Full,
}

impl Default for ChargeStatus {
    fn default() -> Self {
        Self::Unknown
    }
}

impl ChargeStatus {
    fn from_str(s: &str) -> Self {
        match s.trim() {
            "Discharging" => Self::Discharging,
            "Charging" => Self::Charging,
            "Full" => Self::Full,
            _ => Self::Unknown,
        }
    }
}

#[derive(Clone)]
struct BatteryRead {
    at: DateTime<Local>,
    status: ChargeStatus,
    charge: i32,
}

/// A remaining battery level while a battery is discharging, whether measured by percentage or
/// minutes until complete depletion.
#[derive(Clone, Copy, Deserialize, Serialize, PartialOrd, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum BatteryLevel {
    /// A value from 0.0 to 1.0.
    Percentage(f32),

    /// Minutes until the battery is depleted.
    MinutesLeft(i64),
}

const SYS_POWER_SUPPLY_BASE_DIR: &str = "/sys/class/power_supply/";
const MAX_READS: i32 = 15; // used for moving averages

/// Data block for battery reports and estimates
pub struct BatteryBlock {
    warning_level: BatteryLevel,
    alarm_level: BatteryLevel,

    battery: String,
    charge_full: i32,

    charging_reads_since_last_anchor: i32,
    average_charging_rate: Option<f32>,

    discharging_reads_since_last_anchor: i32,
    average_discharging_rate: Option<f32>,

    current_read: Option<BatteryRead>,
    last_read: Option<BatteryRead>,
}

impl BatteryBlock {
    /// Returns a new block with the configuration provided.
    pub fn new(config: BatteryConfig) -> Self {
        let battery = config.battery_id;
        Self {
            warning_level: config.warning_level,
            alarm_level: config.alarm_level,

            battery,
            charge_full: 0,

            charging_reads_since_last_anchor: 0,
            average_charging_rate: None,

            discharging_reads_since_last_anchor: 0,
            average_discharging_rate: None,

            current_read: None,
            last_read: None,
        }
    }

    fn calculate_new_rate(&mut self, rate_now: f32) {
        if let Some(r) = &self.current_read {
            match &r.status {
                ChargeStatus::Discharging => {
                    if rate_now < 0.0 {
                        self.average_discharging_rate = Some(get_new_average_rate(
                            self.average_discharging_rate,
                            self.discharging_reads_since_last_anchor,
                            rate_now,
                        ));

                        if self.discharging_reads_since_last_anchor < MAX_READS {
                            self.discharging_reads_since_last_anchor += 1;
                        }
                    }
                }
                ChargeStatus::Charging => {
                    if rate_now > 0.0 {
                        self.average_charging_rate = Some(get_new_average_rate(
                            self.average_charging_rate,
                            self.charging_reads_since_last_anchor,
                            rate_now,
                        ));

                        if self.charging_reads_since_last_anchor < MAX_READS {
                            self.charging_reads_since_last_anchor += 1;
                        }
                    }
                }
                _ => (), // do nothing on any other status
            }
        }
    }

    fn get_new_read(&self) -> Result<BatteryRead, MuseStatusError> {
        let charge = self.get_battery_charge()?;
        let status = self.get_battery_status()?;
        let at = Local::now();

        Ok(BatteryRead { charge, status, at })
    }

    fn get_battery_charge(&self) -> Result<i32, MuseStatusError> {
        let raw = match std::fs::read_to_string(self.get_base_dir().join("charge_now")) {
            Ok(s) => s,
            // XXX Probably shouldn't ignore this error?
            Err(_) => match std::fs::read_to_string(self.get_base_dir().join("energy_now")) {
                Ok(s) => s,
                Err(e) => return Err(MuseStatusError::from(e)),
            },
        };

        Ok(raw.trim().parse()?)
    }

    // XXX This function is a copy-and-paste of Self::get_batttery_charge. consider writing a
    // function that handles similar functionality
    fn update_battery_charge_max(&mut self) -> Result<(), MuseStatusError> {
        let raw = match std::fs::read_to_string(self.get_base_dir().join("charge_full")) {
            Ok(s) => s,
            // XXX Probably shouldn't ignore this error?
            Err(_) => match std::fs::read_to_string(self.get_base_dir().join("energy_full")) {
                Ok(s) => s,
                Err(e) => return Err(MuseStatusError::from(e)),
            },
        };

        self.charge_full = raw.trim().parse()?;

        Ok(())
    }

    fn get_battery_status(&self) -> Result<ChargeStatus, MuseStatusError> {
        let s = std::fs::read_to_string(self.get_base_dir().join("status"))?;

        Ok(ChargeStatus::from_str(&s))
    }

    fn get_base_dir(&self) -> PathBuf {
        PathBuf::from(SYS_POWER_SUPPLY_BASE_DIR).join(&self.battery)
    }

    /// Returns the amount of nanoseconds left until the battery will be either fully charged or
    /// completely depleted.
    fn get_nanos_left(&self) -> Option<i64> {
        let rate = match &self.current_read.as_ref()?.status {
            ChargeStatus::Charging => self.average_charging_rate?,
            ChargeStatus::Discharging => self.average_discharging_rate?,
            _ => return None,
        };

        let target_percentage = match &self.current_read.as_ref()?.status {
            ChargeStatus::Discharging => 0,
            ChargeStatus::Charging => self.charge_full,
            _ => return None,
        };

        // charge units left * duration per charge unit
        let nanos_left = (target_percentage - self.current_read.as_ref()?.charge) as f32 * rate;
        Some(nanos_left as i64)
    }

    /// Returns the amount of minutes left until the battery will be either fully charged or
    /// completely depleted.
    fn get_minutes_left(&self) -> Option<i64> {
        self.get_nanos_left()
            .map(|n| Duration::nanoseconds(n).num_minutes())
    }

    /// Returns how full the battery is, a value ranging from 0 to 1.
    fn get_percent_left(&self) -> Option<f32> {
        self.current_read
            .clone()
            .map(|current_read| current_read.charge as f32 / self.charge_full as f32)
    }

    /// Returns the time at which the battery will be either fully charged or completely depleted.
    fn get_completion_time(&self) -> Option<DateTime<Local>> {
        self.get_nanos_left()
            .map(|n| Local::now() + Duration::nanoseconds(n as i64))
    }

    /// Returns true if the battery is at or below the warning level. If no current battery reading
    /// is saved, the method returns false.
    fn is_warning(&self) -> bool {
        match self.warning_level {
            BatteryLevel::MinutesLeft(warning_minutes) => match self.get_minutes_left() {
                Some(battery_minutes) => battery_minutes <= warning_minutes,
                None => false,
            },
            BatteryLevel::Percentage(warning_percentage) => match self.get_percent_left() {
                Some(battery_percentage) => battery_percentage <= warning_percentage,
                None => false,
            },
        }
    }

    /// Returns true if the battery is at or below the alarm level. If no current battery reading
    /// is saved, the method returns false.
    fn is_alarm(&self) -> bool {
        match self.alarm_level {
            BatteryLevel::MinutesLeft(alarm_minutes) => match self.get_minutes_left() {
                Some(minutes_left) => minutes_left <= alarm_minutes,
                None => false,
            },
            BatteryLevel::Percentage(alarm_percentage) => match self.get_percent_left() {
                Some(percentage_left) => percentage_left <= alarm_percentage,
                None => false,
            },
        }
    }
}

impl Block for BatteryBlock {
    fn name(&self) -> &str {
        "battery"
    }

    fn output(&self) -> Option<BlockOutputContent> {
        match &self.current_read {
            Some(current_read) => {
                let now = Local::now();
                let percent = (self.get_percent_left().unwrap() * 100.0) as i32;

                let primary_text = match current_read.status {
                    ChargeStatus::Full => String::from("Full"),
                    _ => format!("{}%", percent),
                };

                let secondary_text = match current_read.status {
                    ChargeStatus::Full => Some(String::from("Plugged in")),
                    _ => match self.get_completion_time() {
                        Some(completion_time) => {
                            let minutes_left = (completion_time - now).num_minutes();
                            if minutes_left <= 0 {
                                None
                            } else if minutes_left <= 30 {
                                Some(format!("{} min left", minutes_left))
                            } else {
                                let prefix = match &current_read.status {
                                    ChargeStatus::Charging => "Full at",
                                    ChargeStatus::Discharging => "Until",
                                    _ => "",
                                };

                                Some(format!(
                                    "{} {}",
                                    prefix,
                                    completion_time.format(crate::date::TIME_FORMAT)
                                ))
                            }
                        }
                        None => None,
                    },
                };

                let icon = match &self.current_read {
                    Some(r) => get_battery_icon(&r.status, percent),
                    None => ' ',
                };

                let attention = if let Some(r) = &self.current_read {
                    match &r.status {
                        ChargeStatus::Discharging => {
                            if self.is_alarm() {
                                Attention::AlarmPulse
                            } else if self.is_warning() {
                                Attention::Warning
                            } else {
                                Attention::Normal
                            }
                        }
                        _ => Attention::Normal,
                    }
                } else {
                    Attention::Normal
                };

                Some(BlockOutputContent::Nice(NiceOutput {
                    primary_text,
                    secondary_text,
                    icon,
                    attention,
                }))
            }
            None => None,
        }
    }

    fn update(&mut self) -> Result<(), UpdateError> {
        // update the max charge, if it changes, which I'm pretty sure it does tbh
        // (only update if no error)
        match self.update_battery_charge_max() {
            Ok(r) => r,
            Err(e) => {
                return Err(UpdateError {
                    block_name: self.name().to_owned(),
                    message: format!("couldn't get max battery charge: {}", e),
                })
            }
        };

        self.current_read = match self.get_new_read() {
            Ok(r) => Some(r),
            Err(e) => {
                return Err(UpdateError {
                    block_name: self.name().to_owned(),
                    message: format!("couldn't get new read: {}", e),
                })
            }
        };

        if let Some(current_read) = &self.current_read {
            if let Some(last_read) = &self.last_read {
                if current_read.status == last_read.status
                    && current_read.at - last_read.at >= Duration::seconds(5)
                    && current_read.charge - last_read.charge != 0
                    && (current_read.status == ChargeStatus::Charging
                        || current_read.status == ChargeStatus::Discharging)
                {
                    if let Some(time_diff_ns) = (current_read.at - last_read.at).num_nanoseconds() {
                        let charge_diff: i64 = (current_read.charge - last_read.charge).into();

                        // calculate new rate in nanoseconds per charge unit
                        let rate_now = time_diff_ns / charge_diff;

                        self.calculate_new_rate(rate_now as f32);

                        self.last_read = self.current_read.clone();
                    }
                }
            }
        }

        self.last_read = self.current_read.clone();

        Ok(())
    }

    fn next_update(&self) -> Option<NextUpdate> {
        Some(NextUpdate::In(Duration::seconds(5)))
    }
}

fn get_new_average_rate(
    avg_rate_now: Option<f32>,
    reads_so_far: i32,
    most_recent_read_rate: f32,
) -> f32 {
    let reads = reads_so_far as f32;
    (avg_rate_now.unwrap_or(0.0) * reads) / (reads + 1.0) + most_recent_read_rate / (reads + 1.0)
}

const DISCHARGING_ICONS: [char; 11] = [
    '\u{f008e}',
    '\u{f007a}',
    '\u{f007b}',
    '\u{f007c}',
    '\u{f007d}',
    '\u{f007e}',
    '\u{f007f}',
    '\u{f0080}',
    '\u{f0081}',
    '\u{f0082}',
    '\u{f0079}',
];
const CHARGING_ICONS: [char; 11] = [
    '\u{f089f}',
    '\u{f089c}',
    '\u{f0086}',
    '\u{f0087}',
    '\u{f0088}',
    '\u{f089d}',
    '\u{f0089}',
    '\u{f089e}',
    '\u{f008a}',
    '\u{f008b}',
    '\u{f0085}',
];
const FULL_ICON: char = '\u{f0084}';
const UNKNOWN_ICON: char = '\u{f0091}';

// returns a battery icon
fn get_battery_icon(status: &ChargeStatus, percentage: i32) -> char {
    match status {
        ChargeStatus::Charging => {
            let charging_index = ((percentage * CHARGING_ICONS.len() as i32 / 100) as usize)
                .min(CHARGING_ICONS.len() - 1);
            CHARGING_ICONS[charging_index as usize]
        }
        ChargeStatus::Discharging => {
            let discharging_index = ((percentage * DISCHARGING_ICONS.len() as i32 / 100) as usize)
                .min(DISCHARGING_ICONS.len() - 1);
            DISCHARGING_ICONS[discharging_index as usize]
        }
        ChargeStatus::Full => FULL_ICON,
        _ => UNKNOWN_ICON,
    }
}

/*  DATA FILE FORMAT

data recorded like so:
key %/hour records

where "records" is the amount of times the parameter has been recorded.
used for recording a new average based on the current average and how
many times the parameter has been recorded before
for example:

C 3.14159 200

--- BEGIN FILE EXAMPLE ------------------------------------------------

C			| charging avg
C0			|
C1  		        |
C2			| charging values by percentage
...			|
C9			|

D			| discharging avg
D0			|
D1			|
D2			| discharging avg values by hour of day
...			| (used for predicting nonexistent day-by-day values)
D23			|

S0			| sunday
S1			|
S2			| discharging values by hour by day of week
...			|
S23			|

M0			| monday
...                     |

T0			| thursday
...                     |

W0			| wednesday
...                     |

R0			| thursday
...                     |

F0			| friday
...                     |

A0			| saturday
...                     |
*/
