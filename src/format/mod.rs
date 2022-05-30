/// The module for all things blocks.
pub mod blocks;

/// The module for all things colors.
pub mod color;

// TODO create a separate module for Banner

use crate::daemon::DataPayload;
use crate::errors::{BasicError, MuseStatusError};
use crate::format::blocks::output::BlockOutput;
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

impl Attention {
    /// Returns the colors associated with this `Attention`. The two colors returned are the
    /// primary and secondary colors, respectively.
    pub fn colors(&self, f: &Formatter) -> (RGBA, RGBA) {
        match self {
            Self::Normal => (f.primary_color, f.secondary_color),
            Self::Dim => (f.secondary_color, f.secondary_color),
            Self::Warning => (f.warning_color, f.warning_color),
            Self::Alarm => (f.alarm_color, f.alarm_color),
            Self::WarningPulse => {
                // TODO
                // let c = f.get_warn_pulse_color();
                // (c, c)
                (f.warning_color, f.warning_color)
            }
            Self::AlarmPulse => {
                // TODO
                // let c = f.get_alarm_pulse_color();
                // (c, c)
                (f.alarm_color, f.alarm_color)
            }
        }
    }
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

/// Sets fonts, colors and more for muse-status. Its most important job is converting all data into
/// a format that can be read by status bars (and you!).
pub struct Formatter {
    formatting_mode: Mode,
    primary_color: RGBA,
    secondary_color: RGBA,
    alarm_color: RGBA,
    warning_color: RGBA,
    icon_font: String,

    /// A banner queue.
    #[allow(dead_code)]
    banners: VecDeque<Banner>,
}

/// A banner temporarily hides all blocks on the status bar to bring information front and center
/// for a set duration of time.
#[allow(dead_code)]
pub struct Banner {
    /// A unique identifier, used to update a banner if a twin (with the same id) is sent.
    id: String,

    /// Banner content.
    text: String,

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
                        // starting from tertiary, then to secondary, then to primary (we're
                        // assuming our status bar is right-aligned, like i3 or swaybar, and we'll
                        // arrange the status bar right-to-left in order of importance)
                        //
                        // primary is reversed because it is by default arranged left-to-right.
                        // secondary and tertiary are assumed to be right-to-left.
                        //
                        // we should probably change that.
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
                            .chain(secondary.iter().chain(primary.iter()))
                        {
                            let markup_string = self.block_output_as_markup(block_output);
                            markup_strings.push(markup_string);
                        }
                    }
                    DataPayload::Unranked(outputs) => {
                        for block_output in outputs {
                            let markup_string = self.block_output_as_markup(&block_output);
                            markup_strings.push(markup_string);
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
        // zero to one, calculated by `unix_millis / max_millis`
        let max_millis: u64 = (1000.0 * seconds) as u64;
        let unix_millis: u128 = (now.timestamp_nanos() as u128
            / Duration::from_millis(1).as_nanos() as u128)
            % max_millis as u128;
        let interpolation = utils::cubic_ease_arc((unix_millis / max_millis as u128) as f32);

        color::interpolate_colors(&self.secondary_color, color, interpolation)
    }

    /// A convenience method for giving a standard, pulsing alarm color.
    pub fn get_alarm_pulse_color(&self) -> RGBA {
        self.get_pulse_color(&self.alarm_color, 1.0)
    }

    /// A convenience method for giving a standard, pulsing warning color.
    pub fn get_warn_pulse_color(&self) -> RGBA {
        self.get_pulse_color(&self.warning_color, 2.0)
    }

    #[allow(dead_code)]
    fn color_to_rgba(&self, c: &Color) -> RGBA {
        match c {
            Color::Alarm => self.alarm_color,
            Color::Warning => self.warning_color,
            Color::Primary => self.primary_color,
            Color::Secondary => self.secondary_color,
            Color::Other(rgba) => *rgba,
        }
    }

    /// Formats the BlockOutput for the i3 JSON protocol. None if body is None.
    fn block_output_as_json_protocol_string(&self, block_output: &BlockOutput) -> Option<String> {
        let (full_text, short_text) = block_output.as_pango_strings(self);

        let json = JsonBlock {
            full_text,
            short_text: short_text.unwrap_or_default(), // TODO: Make short_text optional.
                                                        // It is not required as part of
                                                        // the JSON protocol for sway.
            separator: true,
            markup: String::from("pango"),
            name: block_output.name(),
        };

        match serde_json::to_string(&json) {
            Ok(s) => Some(s),
            Err(_) => None,
        }
    }

    /// Formats the BlockOutput for plain markup output.
    fn block_output_as_markup(&self, block_output: &BlockOutput) -> String {
        // return only the long format
        block_output.as_pango_strings(self).0
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
