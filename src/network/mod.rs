use crate::{
    errors::*,
    format::{
        blocks::{output::BlockText, Block, BlockOutput, NextUpdate},
        Attention,
    },
};
use chrono::Duration;
use nl80211::Socket;
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use self::icons::NetworkIcons;

/// Module for all sorts of network icons.
pub mod icons;

/// Whether a network interface is wired (Ethernet) or wireless (WiFi).
pub enum NetworkType {
    /// The network interface is wired.
    Wired,

    /// The network interface is wireless.
    Wireless {
        /// The name of the access point.
        ssid: Option<String>,

        /// The wireless connection strength from 0 to 100.
        strength_percent: i32,
    },
}

/// A block that transmits network interface data.
pub struct NetworkBlock {
    iface_name: String,
    iface_type: NetworkType,
    sys_path: PathBuf,
    status: NetworkStatus,
    icons: NetworkIcons,
}

impl NetworkBlock {
    /// Returns a new NetworkBlock.
    pub fn new(iface_name: &str) -> Result<Self, MuseStatusError> {
        // first, make sure the path to this interface exists
        let sys_path = Path::new("/sys/class/net").join(&iface_name);
        if !sys_path.exists() {
            return Err(MuseStatusError::Basic(BasicError {
                message: format!("network interface `{iface_name}` doesn't exist on this system"),
            }));
        }

        // then we can create the block
        let block = Self {
            iface_name: String::from(iface_name),
            iface_type: get_interface_type(iface_name),
            status: NetworkStatus::Unknown,
            icons: NetworkIcons::default(),

            sys_path,
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

        let is_success = ping_cmd_status
            .map_err(|e| UpdateError {
                block_name: self.name().to_string(),
                message: format!("couldn't execute `ping`: {}", e),
            })?
            .success();

        Ok(!is_success)
    }

    /// Returns true if the network is connected to a VPN (wireguard, ppp, tun).
    fn is_network_secured(&self) -> Result<bool, UpdateError> {
        if self.iface_name.starts_with("tun")
            || self.iface_name.starts_with("tap")
            || self.sys_path.join("tun_flags").exists()
        {
            Ok(true)
        } else {
            let uevent_path = self.sys_path.join("uevent");
            let uevent = fs::read_to_string(uevent_path).map_err(|e| UpdateError {
                block_name: self.name().to_owned(),
                message: format!("couldn't get network iface uevent data: {}", e),
            })?;

            Ok(uevent.contains("wireguard") || uevent.contains("ppp"))
        }
    }

    /// Returns true if the file content at `/sys/class/{iface_name}/{file_name}` matches
    /// `up_value`. Special thanks to i3status-rust's source code for guidance here.
    fn is_up_according_to_file(
        &self,
        file_name: &str,
        up_value: &str,
    ) -> Result<bool, UpdateError> {
        let file = self.sys_path.join(file_name);
        if !file.exists() {
            // consider down if file doesn't even exist
            return Ok(false);
        }

        let value = fs::read_to_string(&file).map_err(|e| UpdateError {
            block_name: self.name().to_string(),
            message: format!(
                "couldn't read {}'s {} file: {}",
                self.iface_name, file_name, e
            ),
        })?;

        Ok(value.trim() == up_value.trim())
    }
}

fn get_interface_type<P: AsRef<Path>>(iface_path: P) -> NetworkType {
    if iface_path.as_ref().join("wireless").exists() {
        NetworkType::Wireless {
            ssid: None,
            strength_percent: 0,
        }
    } else {
        NetworkType::Wired
    }
}

impl Block for NetworkBlock {
    // Name returns "network"
    fn name(&self) -> &str {
        "network"
    }

    // Updates the network information
    fn update(&mut self) -> Result<(), UpdateError> {
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

        // if wireless, update ssid and strength
        if let NetworkType::Wireless {
            ssid,
            strength_percent,
        } = &mut self.iface_type
        {
            *ssid = iface.ssid.map(|val| nl80211::parse_string(&val));
            if ssid.is_none() {
                self.status = NetworkStatus::Disconnected;
            } else {
                // get signal strength
                if let Some(s) = station.signal {
                    let dbm = nl80211::parse_i8(&s);
                    *strength_percent = dbm_to_percentage(dbm as i32);
                    self.status = NetworkStatus::Connected;
                } else {
                    // if no signal, disconnected maybe?
                    self.status = NetworkStatus::Disconnected;
                }
            }
        }

        // check for packet loss and/or vpn if we're connected
        if matches!(self.status, NetworkStatus::Connected) && self.packet_loss()? {
            self.status = NetworkStatus::PacketLoss;
        } else if self.is_network_secured()? {
            self.status = NetworkStatus::Vpn;
        }

        Ok(())
    }

    fn next_update(&self) -> Option<NextUpdate> {
        Some(NextUpdate::In(Duration::seconds(UPDATE_INTERVAL_SECONDS)))
    }

    fn output(&self) -> Option<BlockOutput> {
        let icon = self.icons.get_from_status(&self.iface_type, &self.status);
        match &self.status {
            NetworkStatus::Disconnected | NetworkStatus::Unknown | NetworkStatus::Disabled => {
                // 'dim' statuses; disconnected or otherwise
                let text = BlockText::Single(self.status.to_string().unwrap_or_default());
                Some(BlockOutput::new(
                    self.name(),
                    Some(icon),
                    text,
                    Attention::Dim,
                ))
            }
            NetworkStatus::Connected | NetworkStatus::PacketLoss => match &self.iface_type {
                NetworkType::Wired => Some(BlockOutput::new(
                    self.name(),
                    Some(icon),
                    BlockText::Single(self.status.to_string().unwrap_or_default()),
                    Attention::Normal,
                )),
                NetworkType::Wireless { ssid, .. } => {
                    let text = if let Some(ssid) = &ssid {
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
                    Some(BlockOutput::new(
                        self.name(),
                        Some(icon),
                        text,
                        Attention::Normal,
                    ))
                }
            },
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
    /// The interface is enabled, but there is no connection to the internet.
    Disconnected,

    /// The device has a connection, but packets are being lost.
    PacketLoss,

    /// The device is trying to connect to the internet.
    Connecting,

    /// The device is successfully connected to the internet.
    Connected,

    /// The device is connected to the internet through a VPN.
    Vpn,

    /// The access point requires login information (captive portal).
    SignInRequired,

    /// The interface is disabled.
    Disabled,

    /// The connection speed is slow.
    Slow,

    /// The wireless connection signal strength is weak.
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
