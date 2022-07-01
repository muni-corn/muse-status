use crate::errors::*;
use crate::format::blocks::*;
use crate::format::Attention;
use crate::format::blocks::output::BlockText;
use chrono::Duration;
use nl80211::Socket;
use std::process::Command;
use std::process::Stdio;

use self::icons::NetworkIcons;

pub mod icons;

pub enum NetworkType {
    Wired,
    Wireless {
        ssid: Option<String>,
        strength_percent: i32,
    },
}

/// A block that transmits wireless interface data.
pub struct NetworkBlock {
    iface_name: String,

    ssid: Option<String>,
    strength_percent: i32,
    status: NetworkStatus,
    icons: NetworkIcons,
}

impl NetworkBlock {
    /// Returns a new NetworkBlock.
    pub fn new(iface_name: &str) -> Result<Self, MuseStatusError> {
        let block = Self {
            iface_name: String::from(iface_name),
            status: NetworkStatus::Unknown,
            icons: NetworkIcons::default(),
            ssid: None,
            strength_percent: 0,
        };

        Ok(block)
    }

    fn packet_loss(&self) -> Result<bool, UpdateError> {
        let ping_cmd_status = Command::new("ping")
            .arg("-c")
            .arg("2")
            .arg("-W")
            .arg("2")
            .arg("-I")
            .arg(&self.iface_name)
            .arg("8.8.8.8")
            .stdout(Stdio::null())
            .status();

        let is_success = ping_cmd_status.map_err(|e| UpdateError {
            block_name: self.name().to_string(),
            message: format!("couldn't execute `ping`: {}", e),
        })?.success();

        Ok(!is_success)
    }

    fn get_ip_link_show(&self) -> Result<String, UpdateError> {
        let mut cmd = Command::new("ip");
        cmd.args(&["link", "show", &self.iface_name]);
        let stdout = cmd.output().map(|o| o.stdout).map_err(|e| UpdateError {
            block_name: self.name().to_string(),
            message: format!("there was a problem executing `ip`: {}", e),
        })?;
        Ok(String::from_utf8_lossy(&stdout).into_owned())
    }

    fn update_status(&mut self) -> Result<(), UpdateError> {
        let ip_output = self.get_ip_link_show()?;
        self.status = if ip_output.contains("state UP") {
            NetworkStatus::Connected
        } else if ip_output.contains("state DOWN") {
            if ip_output.contains("NO-CARRIER") {
                NetworkStatus::Disconnected
            } else {
                NetworkStatus::Disabled
            }
        } else if ip_output.contains("state DORMANT") {
            NetworkStatus::Connecting
        } else {
            NetworkStatus::Unknown
        };

        Ok(())
    }

    fn update_ssid_strength(&mut self) -> Result<(), UpdateError> {
        // get interface
        let iface = get_interface(&self.iface_name).map_err(|e| {
            self.status = NetworkStatus::Unknown;

            UpdateError {
                block_name: self.name().to_string(),
                message: format!("couldn't get interface: {}", e),
            }
        })?;

        // get station
        let station = iface.get_station_info().map_err(|e| UpdateError {
            block_name: self.name().to_string(),
            message: format!("{}", e),
        })?;

        // get ssid
        self.ssid = iface.ssid.map(|ssid| nl80211::parse_string(&ssid));
        if self.ssid.is_none() {
            self.status = NetworkStatus::Disconnected;
        } else {
            // get signal strength
            if let Some(s) = station.signal {
                let dbm = nl80211::parse_i8(&s);
                self.strength_percent = dbm_to_percentage(dbm as i32);
                self.status = NetworkStatus::Connected;
            } else {
                // if no signal, disconnected maybe?
                self.status = NetworkStatus::Disconnected;
            }
        }
        Ok(())
    }
}

impl Block for NetworkBlock {
    // Name returns "network"
    fn name(&self) -> &str {
        "network"
    }

    // Updates the network information
    fn update(&mut self) -> Result<(), UpdateError> {
        self.update_status()?;

        match self.status {
            NetworkStatus::Connected | NetworkStatus::Unknown => {
                // update ssid and strength to confirm connected status. if status at this point is
                // Unknown, this might correct it
                self.update_ssid_strength()?;

                // detect packet loss
                match self.packet_loss() {
                    Ok(p) => {
                        if p {
                            self.status = NetworkStatus::PacketLoss;
                        } else {
                            self.status = NetworkStatus::Connected;
                        }
                    }
                    Err(e) => {
                        // This is probably an error returned by `ping`, which is why we set the status
                        // to PacketLoss here
                        self.status = NetworkStatus::PacketLoss;
                        return Err(e);
                    }
                }
            }
            NetworkStatus::Disconnected | NetworkStatus::Disabled => {
                // ensure that the ssid is set to None if we're disconnected or disabled
                self.ssid = None;
            }
            _ => {}
        }

        Ok(())
    }

    fn next_update(&self) -> Option<NextUpdate> {
        Some(NextUpdate::In(Duration::seconds(UPDATE_INTERVAL_SECONDS)))
    }

    fn output(&self) -> Option<BlockOutput> {
        let icon = self.icons.get_wireless_icon(&self.status, self.strength_percent);
        match &self.status {
            NetworkStatus::Disconnected | NetworkStatus::Unknown | NetworkStatus::Disabled => {
                // 'dim' statuses; disconnected or otherwise
                let text = BlockText::Single(self.status.to_string().unwrap_or_default());
                Some(BlockOutput::new(self.name(), Some(icon), text, Attention::Dim))
            }
            NetworkStatus::Connected | NetworkStatus::PacketLoss => {
                let text = if let Some(ssid) = &self.ssid {
                    if let Some(status) = self.status.to_string() {
                        // we have both ssid and status, so we can do a pair
                        BlockText::Pair(ssid.to_owned(), status)
                    } else {
                        // if no status, we'll just do ssid. it's okay
                        BlockText::Single(ssid.to_owned())
                    }
                } else {
                    // if no ssid, we'll count on `status` to give us something
                    BlockText::Single(self.status.to_string().unwrap_or_default())
                };
                Some(BlockOutput::new(self.name(), Some(icon), text, Attention::Normal))
            }
            _ => None,
        }
    }
}

// only returns one interface that matches the name given
fn get_interface(interface_name: &str) -> Result<nl80211::Interface, BasicError> {
    // get all interfaces
    let interfaces = match Socket::connect() {
        Ok(mut s) => match s.get_interfaces_info() {
            Ok(i) => i,
            Err(e) => {
                return Err(BasicError {
                    message: format!("couldn't create network block (getting interfaces): {}", e),
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
            if nl80211::parse_string(n).as_str().trim_matches('\u{0}') == interface_name {
                return Ok(iface);
            }
        }
    }

    Err(BasicError {
        message: format!("network interface not found: {}", interface_name),
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

    /// The device is connected to the internet through a VPN.
    Vpn,

    /// The access point requires login information.
    SignInRequired,

    /// Wireless interfaces are disabled.
    Disabled,

    /// The connection speed is slow.
    Slow,

    /// The connection signal strength is weak.
    Weak,

    /// The status of the device is unknown.
    Unknown,
}

impl NetworkStatus {
    fn to_string(&self) -> Option<String> {
        match self {
            Self::Disconnected => Some(String::from("Not connected")),
            Self::PacketLoss => Some(String::from("No Internet")),
            Self::Connecting => Some(String::from("Connecting")),
            Self::Connected => None,
            Self::SignInRequired => Some(String::from("Sign-in required")),
            Self::Disabled => Some(String::from("Off")),
            Self::Slow => Some(String::from("Slow")),
            Self::Weak => Some(String::from("Weak")),
            Self::Vpn => Some(String::from("Secured")),
            Self::Unknown => Some(String::from("Status unknown")),
        }
    }
}

const UPDATE_INTERVAL_SECONDS: i64 = 5; // interval to update network information, in seconds
