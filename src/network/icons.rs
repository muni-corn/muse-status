use super::{NetworkStatus, NetworkType};

#[derive(Default)]
pub struct NetworkIcons {
    wireless: WirelessIcons,
    wired: WiredIcons,
}

impl NetworkIcons {
    pub fn get_wireless_icon(&self, status: &NetworkStatus, strength_percent: i32) -> char {
        self.wireless.get_icon(status, strength_percent)
    }

    pub fn get_from_status(&self, net_type: &NetworkType, status: &NetworkStatus) -> char {
        match net_type {
            NetworkType::Wired => self.wired.get_icon(status),
            NetworkType::Wireless {
                strength_percent, ..
            } => self.wireless.get_icon(status, *strength_percent),
        }
    }
}

pub struct WirelessIcons {
    connection_icons: Vec<char>,
    packet_loss_icons: Vec<char>,
    vpn_icons: Vec<char>,
    disconnected_icon: char,
    disabled_icon: char,
    unknown_icon: char,
}

impl Default for WirelessIcons {
    fn default() -> Self {
        Self {
            connection_icons: vec![
                '\u{F092F}',
                '\u{F091F}',
                '\u{F0922}',
                '\u{F0925}',
                '\u{F0928}',
            ],
            packet_loss_icons: vec![
                '\u{F092B}',
                '\u{F0920}',
                '\u{F0923}',
                '\u{F0926}',
                '\u{F0929}',
            ],
            vpn_icons: vec![
                '\u{F092C}',
                '\u{F0921}',
                '\u{F0924}',
                '\u{F0927}',
                '\u{F092A}',
            ],
            disconnected_icon: '\u{F092F}',
            disabled_icon: '\u{F092E}',
            unknown_icon: '\u{F092B}',
        }
    }
}

impl WirelessIcons {
    pub fn get_icon(&self, status: &NetworkStatus, strength_percent: i32) -> char {
        match status {
            NetworkStatus::Disconnected => self.disconnected_icon,
            NetworkStatus::Disabled => self.disabled_icon,
            NetworkStatus::Unknown => self.unknown_icon,
            _ => {
                // determine which icons we'll use based on
                // packet loss or vpn status
                let icons = if let NetworkStatus::PacketLoss = status {
                    &self.packet_loss_icons
                } else if let NetworkStatus::Vpn = status {
                    &self.vpn_icons
                } else {
                    &self.connection_icons
                };

                // get the icon
                let mut icon_index: usize = (icons.len() as i32 * strength_percent / 100) as usize;

                // constrains index
                icon_index = icon_index.min(icons.len() - 1);

                icons[icon_index]
            }
        }
    }
}

pub struct WiredIcons {
    connection_icon: char,
    packet_loss_icon: char,
    vpn_icon: char,
    disabled_icon: char,
    unknown_icon: char,
}

impl Default for WiredIcons {
    fn default() -> Self {
        Self {
            connection_icon: '\u{F059F}',
            packet_loss_icon: '\u{F0551}',
            vpn_icon: '\u{F0582}',
            disabled_icon: '\u{F0A8E}',
            unknown_icon: '\u{F0A39}',
        }
    }
}

impl WiredIcons {
    pub fn get_icon(&self, status: &NetworkStatus) -> char {
        match status {
            NetworkStatus::Disconnected | NetworkStatus::PacketLoss => self.packet_loss_icon,
            NetworkStatus::Slow | NetworkStatus::Weak | NetworkStatus::Connected => {
                self.connection_icon
            }
            NetworkStatus::Vpn => self.vpn_icon,
            NetworkStatus::Disabled => self.disabled_icon,
            NetworkStatus::SignInRequired | NetworkStatus::Connecting | NetworkStatus::Unknown => {
                self.unknown_icon
            }
        }
    }
}
