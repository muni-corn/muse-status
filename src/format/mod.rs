pub mod bit;
pub mod blocks;
pub mod color;

use crate::format::blocks::Block;
use crate::utils;
use color::RGBA;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::str::FromStr;

pub enum Attention {
    Dim,
    Normal,
    Warning,
    WarningPulse,
    Alarm,
    AlarmPulse,
}

/// For different types of status modes, for different status bars that parse information
/// differently
#[derive(PartialEq)]
pub enum Mode {
    Lemonbar,
    JsonProtocol,
}

/// Fonts, colors and more for muse-status
pub struct Formatter {
    pub formatting_mode: Mode,
    pub primary_color: RGBA,
    pub secondary_color: RGBA,
    pub text_font: String,
    pub icon_font: String,

    primary_block_names: Vec<String>,
    secondary_block_names: Vec<String>,
    ternary_block_names: Vec<String>,

    /// Stores outputs of blocks.
    outputs: HashMap<String, Option<blocks::output::BlockOutputBody>>,

    /// A banner queue.
    banners: VecDeque<Banner>,
}

pub enum BannerOutput {
    SingleBit(bit::Bit),
    Custom(String),
}

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
            text_font: String::from("Roboto 10"),
            icon_font: String::from("Material Design Icons 12"),

            primary_block_names: Vec::new(),
            secondary_block_names: Vec::new(),
            ternary_block_names: Vec::new(),

            outputs: HashMap::new(),

            banners: VecDeque::new(),
        }
    }
}

impl Formatter {
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
                // let i3s = Vec::new::<JsonBlock>();
                // for b in blocks {
                //     if c, ok = b.(ClassicBlock); ok {
                //         if j = i3_json_of(c); j != nil {
                //             i3s = append(i3s, *j);
                //         }
                //     }
                // }
                // marshaled, _ = json.marshal(i3s);
                // return "," + string(marshaled);
                unimplemented!()
            }
            Mode::Lemonbar => {
                // // huh. increment first until we find a module that
                // // isn't nil or blank (empty for loop)
                // for first = 0; first < len(blocks) && blocks[first] == nil; first++ { }

                // // if everything is blank, return a blank string
                // if first >= len(blocks) {
                //     return ""
                // }

                // result = blocks[first].output(mode);

                // for i = first + 1; i < len(blocks); i++ {
                //     if blocks[i] == nil || blocks[i].hidden() {
                //         continue;
                //     }

                //     v = blocks[i].output(mode);

                //     // trim space at the ends
                //     v = v.trim();
                //     if v != "" {
                //         result += module_separator() + v
                //     }
                // }
                // return result
                unimplemented!()
            }
        }
    }

    pub fn from_blocks(
        primary_blocks: &Vec<Box<dyn Block>>,
        secondary_blocks: &Vec<Box<dyn Block>>,
        ternary_blocks: &Vec<Box<dyn Block>>,
    ) -> Self {
        let mut f: Self = Default::default();

        for b in primary_blocks.iter() {
            f.primary_block_names.push(String::from(b.name()));
        }

        for b in secondary_blocks.iter() {
            f.secondary_block_names.push(String::from(b.name()));
        }

        for b in ternary_blocks.iter() {
            f.ternary_block_names.push(String::from(b.name()));
        }

        f
    }

    /// Sets the order of blocks and block names.
    pub fn set_block_names(
        &mut self,
        primary_block_names: Vec<String>,
        secondary_block_names: Vec<String>,
        ternary_block_names: Vec<String>,
    ) {
        self.primary_block_names = primary_block_names;
        self.secondary_block_names = secondary_block_names;
        self.ternary_block_names = ternary_block_names;
    }

    /// Sets the formatting mode.
    pub fn set_format_mode(&mut self, m: Mode) {
        self.formatting_mode = m
    }

    /// Returns the formatting mode.
    pub fn get_format_mode(&self) -> &Mode {
        &self.formatting_mode
    }

    pub fn update(&mut self, block_name: String, value: Option<blocks::output::BlockOutputBody>) {
        self.outputs.insert(block_name, value);
    }

    pub fn banner(&mut self, _banner: Banner) {
        unimplemented!()
    }

    // set_text_font sets the regular text font
    pub fn set_text_font(&mut self, font: &str) {
        self.text_font = font.to_owned();
    }

    // set_icon_font sets the icon font
    pub fn set_icon_font(&mut self, font: &str) {
        self.icon_font = font.to_owned();
    }

    // module_separator returns something that separates modules
    // (spaces in Lemonbar mode, comma in i3 mode)
    fn module_separator<'a>(&self) -> &'a str {
        match self.formatting_mode {
            Mode::JsonProtocol => ",",
            _ => "    ",
        }
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

    // Returns the original text wrapped in a clickable
    // area
    fn action(&self, action: &str, original: &str) -> String {
        if self.formatting_mode == Mode::JsonProtocol {
            return original.to_string();
        }
        let s = format!("%{{A:{}:}}{}%{{A}}", action, original);

        s
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

    pub fn get_alarm_pulse_color(&self) -> RGBA {
        self.get_pulse_color(&color::ALARM_COLOR, 1.0)
    }

    pub fn get_warn_pulse_color(&self) -> RGBA {
        self.get_pulse_color(&color::WARNING_COLOR, 2.0)
    }

    pub fn get_dim_pulse_color(&self) -> RGBA {
        self.get_pulse_color(&color::TRANSPARENT_COLOR, 3.0)
    }
}

// separator returns 4 spaces as a separator between data
fn separator<'a>() -> &'a str {
    "    "
}

struct JsonBlock {
    name: String,
    full_text: String,
    short_text: String,
    separator: bool,
    markup: String,
}
