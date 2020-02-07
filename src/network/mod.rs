use crate::errors::*;
use crate::format::blocks::output::*;
use crate::format::blocks::*;
use crate::format::Attention;
use chrono::{DateTime, Local};
use nl80211::Socket;

// Block is a block that transmits time and date data
pub struct NetworkBlock {
    iface: nl80211::Interface,

    ssid: String,
    strength_percent: i32,
    status: NetworkStatus,

    dbm: i32,

    next_update_time: DateTime<Local>,
}

impl NetworkBlock {
    pub fn new(iface: &str) -> Result<Self, MuseStatusError> {
        // let client = wifi.New(); // todo

        // get all interfaces
        let interfaces = match Socket::connect() {
            Ok(mut s) => match s.get_interfaces_info() {
                Ok(i) => i,
                Err(e) => {
                    return Err(MuseStatusError::from(BasicError {
                        message: format!(
                            "couldn't create network block (getting interfaces): {}",
                            e
                        ),
                    }))
                }
            },
            Err(e) => {
                return Err(MuseStatusError::from(BasicError {
                    message: format!(
                        "couldn't create network block (connecting to netlink socket): {}",
                        e
                    ),
                }))
            }
        };

        // but only select the one we want
        let iface = if let Some(i) = get_interface(iface, interfaces) {
            i
        } else {
            return Err(MuseStatusError::from(BasicError {
                message: "couldn't create network block, as the specified interface doesn't exist"
                    .to_string(),
            }));
        };

        let now = Local::now();

        Ok(Self {
            iface,
            ssid: String::new(),
            strength_percent: 0,
            status: NetworkStatus::Disconnected,

            dbm: 0,

            next_update_time: now + chrono::Duration::seconds(UPDATE_INTERVAL_SECONDS),
        })
    }

    fn packet_loss(&self) -> Result<bool, UpdateError> {
        let iface_name = if let Some(n) = &self.iface.name {
            match String::from_utf8(n.clone()) {
                Ok(s) => s,
                Err(e) => {
                    return Err(UpdateError {
                        block_name: self.name().to_string(),
                        message: format!("couldn't successfully retrieve interface name: {}", e), // well, maybe we should just store the interface name in the struct XXX
                    });
                }
            }
        } else {
            return Err(UpdateError {
                block_name: self.name().to_string(),
                message: String::from("can't run `ping` without an interface name"), // well, maybe we should just store the interface name in the struct XXX
            });
        };

        let mut ping_cmd = std::process::Command::new("ping");
        ping_cmd.args(&["ping", "-c", "2", "-W", "2", "-I", &iface_name, "8.8.8.8"]);

        let status = match ping_cmd.status() {
            Ok(s) => s,
            Err(e) => {
                return Err(UpdateError {
                    block_name: self.name().to_string(),
                    message: format!("couldn't execute `ping`: {}", e),
                })
            }
        };

        Ok(!status.success())
    }
}

impl Block for NetworkBlock {
    // Name returns "network"
    fn name(&self) -> &str {
        "network"
    }

    // update updates the network information
    fn update(&mut self) -> Result<(), UpdateError> {
        self.next_update_time =
            self.next_update_time + chrono::Duration::seconds(UPDATE_INTERVAL_SECONDS.into());

        // strength
        let station = match self.iface.get_station_info() {
            Ok(i) => i,
            Err(e) => {
                return Err(UpdateError {
                    block_name: self.name().to_string(),
                    message: format!("{}", e),
                })
            }
        };

        // get signal strength
        if let Some(s) = station.signal {
            let dbm = nl80211::parse_i8(&s);
            self.strength_percent = dbm_to_percentage(dbm) as i32;
        } else {
            // if no signal, disconnected maybe?
            self.status = NetworkStatus::Disconnected;
        }

        // detect packet loss
        if self.packet_loss()? {
            self.status = NetworkStatus::PacketLoss;
        } else {
            self.status = NetworkStatus::Connected;
        }

        Ok(())
    }

    fn next_update_time(&self) -> Option<DateTime<Local>> {
        Some(self.next_update_time)
    }

    fn output(&self) -> Option<BlockOutputBody> {
        match &self.status {
            NetworkStatus::Connected => Some(BlockOutputBody::from(NiceOutput {
                attention: Attention::Normal,
                icon: Some(get_icon(self.strength_percent, &self.status)),
                primary_text: self.ssid.clone(),
                secondary_text: self.status.to_string(),
            })),
            _ => None,
        }
    }
}

// only returns one interface that matches the name given
fn get_interface(
    interface_name: &str,
    interfaces: Vec<nl80211::Interface>,
) -> Option<nl80211::Interface> {
    for iface in interfaces {
        if let Some(n) = &iface.name {
            if nl80211::parse_string(&n).as_str().trim_matches('\u{0}') == interface_name {
                return Some(iface);
            }
        }
    }

    None
}

const SIGNAL_MAX_DBM: i32 = -30;
const NOISE_FLOOR_DBM: i32 = -80;

// thank u to i3status and NetworkManager :)
fn dbm_to_percentage(mut dbm: i8) -> i32 {
    dbm = dbm.max(NOISE_FLOOR_DBM as i8).min(SIGNAL_MAX_DBM as i8);
    let dbm_f = dbm as f64;

    (-0.04 * ((dbm_f + 30.0) * (dbm_f + 30.0) + 100.0)) as i32
}

fn get_icon(signal_strength_percent: i32, status: &NetworkStatus) -> char {
    // determine which icons we'll use based on
    // packet_loss
    let icons = if *status == NetworkStatus::PacketLoss {
        PACKET_LOSS_ICONS
    } else {
        CONNECTION_ICONS
    };

    // get the icon
    let mut icon_index: usize = (icons.len() as i32 * signal_strength_percent / 100) as usize;

    // constrains index
    icon_index = icon_index.min(icons.len() - 1);

    icons[icon_index]
}

#[derive(PartialEq)]
pub enum NetworkStatus {
    Disconnected,
    PacketLoss,
    Connecting,
    Connected,
    SignInRequired,
    Airplane,
    Slow,
    Weak,
}

impl NetworkStatus {
    fn to_string(&self) -> Option<String> {
        match self {
            Self::Disconnected => Some(String::from("No connection")),
            Self::PacketLoss => Some(String::from("Connection problems")),
            Self::Connecting => Some(String::from("Connecting")),
            Self::Connected => None,
            Self::SignInRequired => Some(String::from("Sign-in required")),
            Self::Airplane => Some(String::from("Airplane mode")),
            Self::Slow => Some(String::from("Slow connection")),
            Self::Weak => Some(String::from("Weak connection")),
        }
    }
}

const UPDATE_INTERVAL_SECONDS: i64 = 5; // interval to update network information, in seconds

const CONNECTION_ICONS: [char; 5] = ['\u{f92e}', '\u{f91e}', '\u{f921}', '\u{f924}', '\u{f927}'];
const PACKET_LOSS_ICONS: [char; 5] = ['\u{f92a}', '\u{f91f}', '\u{f922}', '\u{f925}', '\u{f928}'];
const DISCONNECTED_ICON: char = '\u{f92e}';
const DISABLED_ICON: char = '\u{f92d}';
