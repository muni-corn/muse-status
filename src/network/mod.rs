use crate::errors::*;
use crate::format::blocks::output::*;
use crate::format::blocks::*;
use crate::format::Attention;
use chrono::{DateTime, Local};
use nl80211::Socket;

/// A block that transmits wireless interface data.
pub struct NetworkBlock {
    iface_name: String,

    ssid: Option<String>,
    strength_percent: i32,
    status: NetworkStatus,

    connection_icons: Vec<char>,
    packet_loss_icons: Vec<char>,
    disconnected_icon: char,
    disabled_icon: char,

    next_update_time: DateTime<Local>,
}

impl Default for NetworkBlock {
    fn default() -> Self {
        Self {
            iface_name: String::new(),

            ssid: None,
            strength_percent: 0,
            status: NetworkStatus::Disconnected,

            connection_icons: vec!['\u{f92e}', '\u{f91e}', '\u{f921}', '\u{f924}', '\u{f927}'],
            packet_loss_icons: vec!['\u{f92a}', '\u{f91f}', '\u{f922}', '\u{f925}', '\u{f928}'],
            disconnected_icon: '\u{f92e}',
            disabled_icon: '\u{f92d}',

            next_update_time: Local::now(),
        }
    }
}

impl NetworkBlock {
    /// Returns a new NetworkBlock.
    pub fn new(iface_name: &str) -> Result<Self, MuseStatusError> {
        let mut block: Self = Default::default();

        block.iface_name = String::from(iface_name);
        block.next_update_time = Local::now() + chrono::Duration::seconds(UPDATE_INTERVAL_SECONDS);

        Ok(block)
    }

    fn packet_loss(&self) -> Result<bool, UpdateError> {
        let mut ping_cmd = std::process::Command::new("ping");
        ping_cmd.args(&["ping", "-c", "2", "-W", "2", "-I", &self.iface_name, "8.8.8.8"]);

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

    fn get_icon(&self) -> char {
        match &self.status {
            NetworkStatus::Disconnected => self.disconnected_icon,
            NetworkStatus::Airplane => self.disabled_icon,
            _ => {
                // determine which icons we'll use based on
                // packet_loss
                let icons = if self.status == NetworkStatus::PacketLoss {
                    &self.packet_loss_icons
                } else {
                    &self.connection_icons
                };

                // get the icon
                let mut icon_index: usize = (icons.len() as i32 * self.strength_percent / 100) as usize;

                // constrains index
                icon_index = icon_index.min(icons.len() - 1);

                icons[icon_index]
            }
        }
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
            chrono::Local::now() + chrono::Duration::seconds(UPDATE_INTERVAL_SECONDS);

        // get interface
        let iface = match get_interface(&self.iface_name) {
            Ok(i) => i,
            Err(e) => return Err(UpdateError {
                block_name: self.name().to_string(),
                message: format!("couldn't get interface: {}", e)
            })
        };

        // strength
        let station = match iface.get_station_info() {
            Ok(i) => i,
            Err(e) => {
                return Err(UpdateError {
                    block_name: self.name().to_string(),
                    message: format!("{}", e),
                })
            }
        };

        // get ssid
        self.ssid = if let Some(ssid) = &iface.ssid {
            Some(nl80211::parse_string(&ssid))
        } else {
            None
        };


        // get signal strength
        if let Some(s) = station.signal {
            let dbm = nl80211::parse_i8(&s);
            self.strength_percent = dbm_to_percentage(dbm as i32);
            self.status = NetworkStatus::Connected;
        } else {
            // if no signal, disconnected maybe?
            self.status = NetworkStatus::Disconnected;
        }

        // detect packet loss
        // if self.packet_loss()? {
        //     self.status = NetworkStatus::PacketLoss;
        // } else {
        //     self.status = NetworkStatus::Connected;
        // }

        Ok(())
    }

    fn next_update_time(&self) -> Option<DateTime<Local>> {
        Some(self.next_update_time)
    }

    fn output(&self) -> Option<BlockOutputBody> {
        match &self.status {
            NetworkStatus::Connected => Some(BlockOutputBody::from(NiceOutput {
                attention: Attention::Normal,
                icon: self.get_icon(),
                primary_text: self.ssid.clone().unwrap_or_else(String::new),
                secondary_text: self.status.to_string(),
            })),
            _ => None,
        }
    }
}

// only returns one interface that matches the name given
fn get_interface(
    interface_name: &str,
) -> Result<nl80211::Interface, BasicError> {
    // get all interfaces
    let interfaces = match Socket::connect() {
        Ok(mut s) => match s.get_interfaces_info() {
            Ok(i) => i,
            Err(e) => {
                return Err(BasicError {
                    message: format!(
                                 "couldn't create network block (getting interfaces): {}",
                                 e
                             ),
                })
            }
        },
        Err(e) => {
            return Err(BasicError {
                message: format!(
                             "couldn't create network block (connecting to netlink socket): {}",
                             e
                         ),
            })
        }
    };

    for iface in interfaces {
        if let Some(n) = &iface.name {
            if nl80211::parse_string(&n).as_str().trim_matches('\u{0}') == interface_name {
                return Ok(iface);
            }
        }
    }

    Err(BasicError {
        message: format!("network interface not found: {}", interface_name)
    })
}

const SIGNAL_MAX_DBM: i32 = -30;
const NOISE_FLOOR_DBM: i32 = -80;

// thank u to i3status and NetworkManager :)
fn dbm_to_percentage(mut dbm: i32) -> i32 {
    dbm = dbm.max(NOISE_FLOOR_DBM).min(SIGNAL_MAX_DBM);
    let dbm_f = dbm as f64;

    (-0.04 * (dbm_f + 30.0).powi(2) + 100.0) as i32
}

/// NetworkStatus represents the state of a wireless interface.
#[derive(PartialEq)]
pub enum NetworkStatus {
    /// Wireless interfaces are enabled, but there is no connection to the internet.
    Disconnected,

    /// The device is connected to an access point, but packets are being lost.
    PacketLoss,

    /// The device is trying to connect to the internet.
    Connecting,

    /// The device is successfully connected to the internet.
    Connected,

    /// The access point requires login information.
    SignInRequired,

    /// Wireless interfaces are disabled.
    Airplane,

    /// The connection speed is slow.
    Slow,

    /// The connection signal strength is weak.
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
