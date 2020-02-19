use crate::errors::*;
use crate::format::blocks::output::*;
use crate::format::blocks::*;
use crate::format::Attention;
use chrono::{DateTime, Duration, Local};
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

    /// The battery is full and charging.
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

const SYS_POWER_SUPPLY_BASE_DIR: &str = "/sys/class/power_supply/";
const MAX_READS: i32 = 40; // used for moving averages
const TIME_FORMAT: &str = "%-I:%M %P";

/// Data block for battery reports and estimates
pub struct SmartBatteryBlock {
    warning_level: i32,
    alarm_level: i32,

    battery: String,
    charge_full: i32,

    charging_reads_since_last_anchor: i32,
    average_charging_rate: f32,

    discharging_reads_since_last_anchor: i32,
    average_discharging_rate: f32,

    current_read: Option<BatteryRead>,
    last_read: Option<BatteryRead>,

    next_update_time: DateTime<Local>,
}

impl SmartBatteryBlock {
    /// Returns a new block with the specified battery, warning level, and alarm level..
    pub fn new(battery_dir: &str, warning_level: i32, alarm_level: i32) -> Self {
        let battery = String::from(battery_dir);
        let next_update_time = Local::now() + Duration::minutes(1);
        Self {
            warning_level,
            alarm_level,

            battery,
            charge_full: 0,

            charging_reads_since_last_anchor: 0,
            average_charging_rate: 0.0,
            discharging_reads_since_last_anchor: 0,
            average_discharging_rate: 0.0,
            current_read: None,
            last_read: None,

            next_update_time,
        }
    }

    fn calculate_new_rate(&mut self, rate_now: f32) {
        if let Some(r) = &self.current_read {
            match &r.status {
                ChargeStatus::Discharging => {
                    if rate_now < 0.0 {
                        self.average_discharging_rate = get_new_average_rate(
                            self.average_discharging_rate,
                            self.discharging_reads_since_last_anchor,
                            rate_now,
                        );

                        if self.discharging_reads_since_last_anchor < MAX_READS {
                            self.discharging_reads_since_last_anchor += 1;
                        }
                    }
                }
                ChargeStatus::Charging => {
                    if rate_now > 0.0 {
                        self.average_charging_rate = get_new_average_rate(
                            self.average_charging_rate,
                            self.charging_reads_since_last_anchor,
                            rate_now,
                        );

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

    fn get_completion_time(&self) -> Option<DateTime<Local>> {
        match &self.current_read {
            Some(r) => {
                let now = Local::now();

                let end = match &r.status {
                    ChargeStatus::Discharging => 0,
                    ChargeStatus::Charging => self.charge_full,
                    _ => 0,
                };

                // charge units left * duration per charge unit
                let rate = match &r.status {
                    ChargeStatus::Charging => self.average_charging_rate,
                    ChargeStatus::Discharging => self.average_discharging_rate,
                    _ => 0.0,
                };

                let nanos_left = (end - r.charge) as f32 * rate;
                let time_left = Duration::nanoseconds(nanos_left as i64); // charge units remaining * nanoseconds / charge unit
                Some(now + time_left)
            }
            None => None,
        }
    }
}

impl Block for SmartBatteryBlock {
    fn name(&self) -> &str {
        "battery"
    }

    fn output(&self) -> Option<BlockOutputBody> {
        match &self.current_read {
            Some(current_read) => {
                let now = Local::now();
                let percent = current_read.charge * 100 / self.charge_full;

                let primary_text = match current_read.status {
                    ChargeStatus::Full => String::from("Full"),
                    _ => format!("{}%", percent)
                };

                let secondary_text = match current_read.status {
                    ChargeStatus::Full => Some(String::from("Plugged in")),
                    _ => {
                        match self.get_completion_time() {
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
                                            completion_time.format(TIME_FORMAT)
                                    ))
                                } 
                            }
                            None => None,
                        }
                    }
                };

                let icon = match &self.current_read {
                    Some(r) => get_battery_icon(&r.status, percent),
                    None => ' ',
                };

                let attention = if let Some(r) = &self.current_read {
                    match &r.status {
                        ChargeStatus::Discharging => {
                            if r.charge < self.alarm_level {
                                Attention::AlarmPulse
                            } else if r.charge < self.warning_level {
                                Attention::WarningPulse
                            } else {
                                Attention::Normal
                            }
                        }
                        _ => Attention::Normal,
                    }
                } else {
                    Attention::Normal
                };

                Some(BlockOutputBody::Nice(NiceOutput {
                    primary_text,
                    secondary_text,
                    icon,
                    attention,
                }))
            },
            None => None
        }
    }

    fn update(&mut self) -> Result<(), UpdateError> {
        let now = Local::now();
        self.next_update_time = now + Duration::seconds(5);

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

        match &self.current_read {
            Some(current_read) => {
                match &self.last_read {
                    Some(last_read) => {
                        if current_read.status != last_read.status || self.last_read.is_none() {
                        } else if current_read.at - last_read.at >= Duration::seconds(5)
                            && current_read.charge - last_read.charge != 0
                                && (current_read.status == ChargeStatus::Charging
                                    || current_read.status == ChargeStatus::Discharging)
                        {
                            let time_diff_ns: i64 =
                                (current_read.at - last_read.at).num_nanoseconds().unwrap();
                            let charge_diff: i64 = (current_read.charge - last_read.charge).into();

                            // calculate new rate in nanoseconds per charge unit
                            let rate_now = time_diff_ns / charge_diff;

                            self.calculate_new_rate(rate_now as f32);

                            self.last_read = self.current_read.clone();
                        }
                    }
                    None => {}
                }
            }
            None => {}
        }

        self.last_read = self.current_read.clone();

        Ok(())
    }

    fn next_update_time(&self) -> Option<chrono::DateTime<chrono::Local>> {
        Some(self.next_update_time)
    }
}

fn get_new_average_rate(avg_rate_now: f32, reads: i32, new_read_rate: f32) -> f32 {
    let reads_f = reads as f32;
    (avg_rate_now * reads_f) / (reads_f + 1.0) + new_read_rate / (reads_f + 1.0)
}

const DISCHARGING_ICONS: [char; 11] = [
    '\u{f08e}', '\u{f07a}', '\u{f07b}', '\u{f07c}', '\u{f07d}', '\u{f07e}', '\u{f07f}', '\u{f080}',
    '\u{f081}', '\u{f082}', '\u{f079}',
];
const CHARGING_ICONS: [char; 11] = [
    '\u{f89e}', '\u{f89b}', '\u{f086}', '\u{f087}', '\u{f088}', '\u{f89c}', '\u{f089}', '\u{f89d}',
    '\u{f08a}', '\u{f08b}', '\u{f085}',
];
const FULL_ICON: char = '\u{f084}';
const UNKNOWN_ICON: char = '\u{f590}';

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
        ChargeStatus::Full => {
            FULL_ICON
        }
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
