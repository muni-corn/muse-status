/// A separate module for the Bit struct.
pub mod bit;

/// The module for all things blocks.
pub mod blocks;

/// The module for all things colors.
pub mod color;

// TODO create a separate module for Banner

use crate::daemon::DataPayload;
use crate::errors::{BasicError, MuseStatusError};
use crate::format::blocks::output::{BlockOutput, BlockOutputContent};
use crate::utils;
use color::{Color, RGBA};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::str::FromStr;

/// Eight spaces.
const MARKUP_SEPARATOR: &str = "        ";

/// Attention provides a way to easily apply colors to a Block, without actually passing any RGBA
/// values.
#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum Attention {
    /// Static dim color.
    Dim,

    /// Default static color.
    Normal,

    /// Static warning color.
    Warning,

    /// Flashing warning color.
    WarningPulse,

    /// Static alarm color.
    Alarm,

    /// Flashing alarm color.
    AlarmPulse,
}

/// For different types of status modes, for different status bars that parse information
/// differently
#[derive(PartialEq)]
pub enum Mode {
    /// Lemonbar-compatabile output.
    Lemonbar,

    /// i3-compatabile output.
    JsonProtocol,

    /// Plain markup output.
    Markup,
}

impl Default for Mode {
    fn default() -> Self {
        Self::JsonProtocol
    }
}

impl FromStr for Mode {
    type Err = MuseStatusError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "i3" => Ok(Self::JsonProtocol),
            "lemon" => Ok(Self::Lemonbar),
            "plain" | "markup" => Ok(Self::Markup),
            _ => Err(MuseStatusError::from(BasicError {
                message: format!("this format isn't recognized: `{}`", s),
            })),
        }
    }
}

/// Fonts, colors and more for muse-status
pub struct Formatter {
    formatting_mode: Mode,
    primary_color: RGBA,
    secondary_color: RGBA,
    alarm_color: RGBA,
    warning_color: RGBA,
    text_font: String,
    icon_font: String,

    /// A banner queue.
    banners: VecDeque<Banner>,
}

/// The output of a Banner.
pub enum BannerOutput {
    /// A single bit as output.
    SingleBit(bit::Bit),

    /// Custom output.
    Custom(String),
}

/// A banner temporarily hides all blocks on the status bar to bring information front and center
/// for a set duration of time.
pub struct Banner {
    /// A unique identifier, used to update a banner if a twin (with the same id) is sent.
    id: String,

    /// Banner content.
    content: BannerOutput,

    /// How long the banner should remain visible.
    seconds: f32,
}

impl Default for Formatter {
    fn default() -> Self {
        Self {
            formatting_mode: Mode::JsonProtocol,
            primary_color: RGBA {
                r: 0xff,
                g: 0xff,
                b: 0xff,
                a: 0xff,
            },
            secondary_color: RGBA {
                r: 0xc0,
                g: 0xc0,
                b: 0xc0,
                a: 0xff,
            },
            alarm_color: RGBA {
                r: 0xff,
                g: 0x00,
                b: 0x00,
                a: 0xff,
            },
            warning_color: RGBA {
                r: 0xff,
                g: 0xaa,
                b: 0x00,
                a: 0xff,
            },
            text_font: String::from("sans 10"),
            icon_font: String::from("Material Design Icons 12"),

            banners: VecDeque::new(),
        }
    }
}

impl Formatter {
    /// Returns a new Formatter with the block names provided.
    pub fn new() -> Self {
        Default::default()
    }

    /// Creates and returns a new Formatter from command line arguments.
    pub fn from_env() -> Result<Self, MuseStatusError> {
        let mut args = std::env::args().skip(1);

        let mut formatter: Self = Default::default();

        while let Some(arg) = args.next() {
            if let Some(value) = args.next() {
                match arg.as_str() {
                    "-p" | "--primary-color" => formatter.set_primary_color(&value)?,
                    "-s" | "--secondary-color" => formatter.set_secondary_color(&value)?,
                    "-f" | "--font" => formatter.set_text_font(&value),
                    "-i" | "--icon-font" => formatter.set_icon_font(&value),
                    "-m" | "--mode" => formatter.set_format_mode(value.parse()?),
                    _ => (),
                }
            } else {
                return Err(MuseStatusError::from(BasicError {
                    message: format!("flag requires a value: {}", arg),
                }));
            }
        }

        Ok(formatter)
    }

    /// Chains status bites together, ensuring that there are no awkward spaces between bites, and
    /// outputs a result fit to be parsed by a status bar. The string can safely be printed as-is
    /// without additional formatting or newlines.
    pub fn format_data(&self, data: DataPayload) -> String {
        match self.formatting_mode {
            Mode::JsonProtocol => {
                let mut json_strings = Vec::new();
                match data {
                    DataPayload::Ranked {
                        primary,
                        secondary,
                        tertiary,
                    } => {
                        for block_output in tertiary
                            .iter()
                            .chain(secondary.iter().chain(primary.iter().rev()))
                        {
                            if let Some(jps) =
                                self.block_output_as_json_protocol_string(block_output)
                            {
                                json_strings.push(jps)
                            }
                        }
                    }
                    DataPayload::Unranked(outputs) => {
                        for block_output in outputs {
                            if let Some(jps) =
                                self.block_output_as_json_protocol_string(&block_output)
                            {
                                json_strings.push(jps)
                            }
                        }
                    }
                }
                let joined = json_strings.join(",");
                format!(",[{}]", joined)
            }
            Mode::Lemonbar => unimplemented!(),
            Mode::Markup => {
                let mut markup_strings = Vec::new();
                match data {
                    DataPayload::Ranked {
                        primary,
                        secondary,
                        tertiary,
                    } => {
                        for block_output in tertiary
                            .iter()
                            .chain(secondary.iter().chain(primary.iter().rev()))
                        {
                            if let Some(jps) = self.block_output_as_markup(block_output) {
                                markup_strings.push(jps)
                            }
                        }
                    }
                    DataPayload::Unranked(outputs) => {
                        for block_output in outputs {
                            if let Some(jps) = self.block_output_as_markup(&block_output) {
                                markup_strings.push(jps)
                            }
                        }
                    }
                }

                markup_strings.join(MARKUP_SEPARATOR)
            }
        }
    }

    // TODO
    // /// Formats an error in a format that can be parsed and displayed by a status bar. No
    // /// additional formatting is required.
    // pub fn format_error<E: std::error::Error>(&self, _: E) -> String {
    //     match self.formatting_mode {
    //         Mode::JsonProtocol => unimplemented!(),
    //         Mode::Lemonbar => unimplemented!(),
    //         Mode::Markup => unimplemented!(),
    //     }
    // }

    /// Sets the formatting mode.
    pub fn set_format_mode(&mut self, m: Mode) {
        self.formatting_mode = m
    }

    /// Returns the formatting mode.
    pub fn get_format_mode(&self) -> &Mode {
        &self.formatting_mode
    }

    /// Activates and displays a banner.
    pub fn banner(&mut self, _banner: Banner) {
        unimplemented!()
    }

    /// Sets the text font of the Formatter.
    pub fn set_text_font(&mut self, font: &str) {
        self.text_font = font.to_owned();
    }

    /// Sets the icon font of the Formatter.
    pub fn set_icon_font(&mut self, font: &str) {
        self.icon_font = font.to_owned();
    }

    /// Sets the primary color of the Formatting
    pub fn set_primary_color(&mut self, color: &str) -> Result<(), color::RGBAParseError> {
        Self::set_color(&mut self.primary_color, color)
    }

    /// Sets the secondary (dim) color of the Formatting
    pub fn set_secondary_color(&mut self, color: &str) -> Result<(), color::RGBAParseError> {
        Self::set_color(&mut self.secondary_color, color)
    }

    fn set_color(c: &mut RGBA, s: &str) -> Result<(), color::RGBAParseError> {
        *c = RGBA::from_str(s)?;

        Ok(())
    }

    fn get_pulse_color(&self, color: &RGBA, seconds: f32) -> RGBA {
        use std::time::Duration;

        let now = chrono::Local::now();

        // get alpha byte value. interpolation is a value from
        // zero to one, calculated by unixMillis/maxMillis
        let max_millis: u64 = (1000.0 * seconds) as u64;
        let unix_millis: u128 = (now.timestamp_nanos() as u128
            / Duration::from_millis(1).as_nanos() as u128)
            % max_millis as u128;
        let interpolation = utils::cubic_ease_arc((unix_millis / max_millis as u128) as f32);

        color::interpolate_colors(&self.secondary_color, &color, interpolation)
    }

    /// A convenience method for giving an alarm pulse color.
    pub fn get_alarm_pulse_color(&self) -> RGBA {
        self.get_pulse_color(&self.alarm_color, 1.0)
    }

    /// A convenience method for giving a warning pulse color.
    pub fn get_warn_pulse_color(&self) -> RGBA {
        self.get_pulse_color(&self.warning_color, 2.0)
    }

    fn color_to_rgba(&self, c: &Color) -> RGBA {
        match c {
            Color::Alarm => self.alarm_color.clone(),
            Color::Warning => self.warning_color.clone(),
            Color::Primary => self.primary_color.clone(),
            Color::Secondary => self.secondary_color.clone(),
            Color::Other(rgba) => rgba.clone(),
        }
    }

    /// Formats the BlockOutput for the i3 JSON protocol. None if body is None.
    fn block_output_as_json_protocol_string(&self, b: &BlockOutput) -> Option<String> {
        if let Some(body) = &b.body {
            let (full_text, short_text) = match body {
                BlockOutputContent::Nice(n) => n.as_pango_strings(self),
                BlockOutputContent::SingleBit(b) => {
                    let pango = b.as_pango_string(self);
                    (pango.clone(), pango)
                }
                BlockOutputContent::Custom(c) => (c.clone(), c.clone()),
            };

            let json = JsonBlock {
                full_text,
                short_text,
                separator: true,
                markup: String::from("pango"),
                name: b.block_name.clone(),
            };

            match serde_json::to_string(&json) {
                Ok(s) => Some(s),
                Err(_) => None,
            }
        } else {
            None
        }
    }

    /// Formats the BlockOutput for plain markup output.
    fn block_output_as_markup(&self, b: &BlockOutput) -> Option<String> {
        if let Some(body) = &b.body {
            Some(match body {
                BlockOutputContent::Nice(n) => n.as_pango_strings(self).0,
                BlockOutputContent::SingleBit(b) => b.as_pango_string(self),
                BlockOutputContent::Custom(c) => c.clone(),
            })
        } else {
            None
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct JsonBlock {
    name: String,
    full_text: String,
    short_text: String,
    separator: bool,
    markup: String,
}
