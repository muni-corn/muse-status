/// A separate module for the Bit struct.
pub mod bit;

/// The module for all things blocks.
pub mod blocks;

/// The module for all things colors.
pub mod color;

// TODO create a separate module for Banner

use crate::utils;
use crate::format::blocks::output::BlockOutput;
use crate::format::blocks::Block;
use color::{RGBA, Color};
use std::collections::HashMap;
use std::collections::VecDeque;
use std::str::FromStr;
use std::sync::mpsc;

/// Attention provides a way to easily apply colors to a Block, without actually passing any RGBA
/// values.
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

    primary_block_names: Vec<String>,
    secondary_block_names: Vec<String>,
    ternary_block_names: Vec<String>,

    /// Stores outputs of blocks.
    outputs: HashMap<String, blocks::output::BlockOutput>,

    /// A banner queue.
    banners: VecDeque<Banner>,

    /// Sends outputs, if Some.
    output_sender: Option<mpsc::Sender<String>>,
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
    id: String,
    content: BannerOutput,
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
                r: 0x00,
                g: 0xaa,
                b: 0xaa,
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
            text_font: String::from("Roboto 10"),
            icon_font: String::from("Material Design Icons 12"),

            primary_block_names: Vec::new(),
            secondary_block_names: Vec::new(),
            ternary_block_names: Vec::new(),

            outputs: HashMap::new(),

            banners: VecDeque::new(),

            output_sender: None,
        }
    }
}

impl Formatter {
    /// Returns a new Formatter with the block names provided.
    #[deprecated]
    pub fn new(
        primary_block_names: Vec<String>,
        secondary_block_names: Vec<String>,
        ternary_block_names: Vec<String>,
    ) -> Self {
        let mut f: Self = Default::default();
        f.primary_block_names = primary_block_names;
        f.secondary_block_names = secondary_block_names;
        f.ternary_block_names = ternary_block_names;

        f
    }

    /// Chains status bites together, ensuring that there are no awkward spaces between bites, and
    /// outputs a result fit to be parsed by a status bar
    fn output(&self) -> String {
        match self.formatting_mode {
            Mode::JsonProtocol => {
                let mut string_outputs: Vec<String> = Vec::new();
                for block_name in self.ternary_block_names.iter().chain(self.secondary_block_names.iter().chain(self.primary_block_names.iter().rev())) {
                    if let Some(bo) = self.outputs.get(block_name) {
                        if let Some(jps) = bo.as_json_protocol_string(self) {
                            string_outputs.push(jps)
                        }
                    }
                }
                let joined = string_outputs.join(",");
                format!("[{}]", joined)
            }
            Mode::Lemonbar => {
                unimplemented!()
            }
        }
    }

    /// Returns a new Formatter that takes block names from the slices of blocks provided. Favored
    /// over `new`, because this method requires actual blocks that have already been created.
    pub fn from_blocks(
        primary_blocks: &[Box<dyn Block>],
        secondary_blocks: &[Box<dyn Block>],
        ternary_blocks: &[Box<dyn Block>],
    ) -> Self {
        let mut f: Self = Default::default();

        f.set_block_names_from_blocks(primary_blocks, secondary_blocks, ternary_blocks);

        f
    }

    /// Sets the order of blocks and block names.
    pub fn set_block_names_from_blocks(
        &mut self,
        primary_blocks: &[Box<dyn Block>],
        secondary_blocks: &[Box<dyn Block>],
        ternary_blocks: &[Box<dyn Block>],
    ) {
        for b in primary_blocks.iter() {
            self.primary_block_names.push(String::from(b.name()));
        }

        for b in secondary_blocks.iter() {
            self.secondary_block_names.push(String::from(b.name()));
        }

        for b in ternary_blocks.iter() {
            self.ternary_block_names.push(String::from(b.name()));
        }
    }

    /// Sets the formatting mode.
    pub fn set_format_mode(&mut self, m: Mode) {
        self.formatting_mode = m
    }

    /// Returns the formatting mode.
    pub fn get_format_mode(&self) -> &Mode {
        &self.formatting_mode
    }

    /// Updates a block in the Formatter with the value. If the value is None, the block is removed
    /// from the Formatter's output.
    pub fn update(&mut self, output: BlockOutput) {
        self.outputs.insert(output.block_name.clone(), output);
        if let Some(sender) = &self.output_sender {
            sender.send(self.output()).unwrap();
        }
    }

    /// Activates and displays a banner.
    pub fn banner(&mut self, _banner: Banner) {
        unimplemented!()
    }

    /// Replaces the current output channel associated with this Formatter, creates a new channel,
    /// saves the Sender in the struct, and returns the Receiver.
    #[must_use]
    pub fn new_output_channel(&mut self) -> mpsc::Receiver<String> {
        let (tx, rx) = mpsc::channel();
        self.output_sender = Some(tx);

        rx
    }

    /// Sets the text font of the Formatter.
    pub fn set_text_font(&mut self, font: &str) {
        self.text_font = font.to_owned();
    }

    /// Sets the icon font of the Formatter.
    pub fn set_icon_font(&mut self, font: &str) {
        self.icon_font = font.to_owned();
    }

    // // module_separator returns something that separates modules
    // // (spaces in Lemonbar mode, comma in i3 mode)
    // fn module_separator<'a>(&self) -> &'a str {
    //     match self.formatting_mode {
    //         Mode::JsonProtocol => ",",
    //         _ => "    ",
    //     }
    // }

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

    // Returns the original text wrapped in a clickable
    // area
    // fn action(&self, action: &str, original: &str) -> String {
    //     if self.formatting_mode == Mode::JsonProtocol {
    //         return original.to_string();
    //     }
    //     let s = format!("%{{A:{}:}}{}%{{A}}", action, original);

    //     s
    // }

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

}

// /// Returns 4 spaces as a separator between data.
// fn separator<'a>() -> &'a str {
//     "    "
// }
