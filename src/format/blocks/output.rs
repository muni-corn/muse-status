use crate::format::bit::Bit;
use crate::format::color::Color;
use crate::format::{Attention, Formatter};
use serde::{Deserialize, Serialize};

/// The output of a Block.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlockOutput {
    /// The name of the original block.
    pub block_name: String,

    /// The body of the BlockOutput, optional. If None, the display of this output should be
    /// removed.
    pub body: Option<BlockOutputContent>,
}

impl BlockOutput {
    /// Returns a new BlockOutput.
    pub fn new(block_name: &str, body: Option<BlockOutputContent>) -> Self {
        Self {
            block_name: String::from(block_name),
            body,
        }
    }
}

/// The body of a BlockOutput.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum BlockOutputContent {
    /// Sexy block output.
    Nice(NiceOutput),

    /// An output of a single Bit.
    SingleBit(Bit),

    /// Custom output.
    Custom(String),
}

impl From<NiceOutput> for BlockOutputContent {
    fn from(n: NiceOutput) -> Self {
        Self::Nice(n)
    }
}

impl From<Bit> for BlockOutputContent {
    fn from(b: Bit) -> Self {
        Self::SingleBit(b)
    }
}

impl From<String> for BlockOutputContent {
    fn from(s: String) -> Self {
        Self::Custom(s)
    }
}

impl From<&str> for BlockOutputContent {
    fn from(s: &str) -> Self {
        Self::Custom(String::from(s))
    }
}

/// A struct for creating nice-looking block outputs.
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct NiceOutput {
    /// The icon of the block.
    pub icon: char,

    /// The primary text of the block, to be displayed with a primary color.
    pub primary_text: String,

    /// The secondary text of the block, to be displayed with a secondary color.
    pub secondary_text: Option<String>,

    /// The Attention level of the output, used to color the output.
    pub attention: Attention,
}

impl NiceOutput {
    /// Formats the output as a pango string. The first string returned is the full text including
    /// icon, primary text, and secondary text. The second string is the same but excludes the
    /// secondary text.
    pub fn as_pango_strings(&self, f: &Formatter) -> (String, String) {
        let (primary_color, secondary_color) = match &self.attention {
            Attention::Normal => (f.primary_color.clone(), f.secondary_color.clone()),
            Attention::Dim => (f.secondary_color.clone(), f.secondary_color.clone()),
            Attention::Warning => (f.warning_color.clone(), f.warning_color.clone()),
            Attention::Alarm => (f.alarm_color.clone(), f.alarm_color.clone()),
            Attention::WarningPulse => {
                let c = f.get_warn_pulse_color();
                (c.clone(), c)
            }
            Attention::AlarmPulse => {
                let c = f.get_alarm_pulse_color();
                (c.clone(), c)
            }
        };

        let icon_bit = Bit {
            text: self.icon.to_string(),
            color: Some(Color::Other(primary_color.clone())),
            font: Some(f.icon_font.clone()),
        };

        let primary_bit = Bit {
            text: self.primary_text.clone(),
            color: Some(Color::Other(primary_color)),
            font: None,
        };

        let secondary_bit_opt = match &self.secondary_text {
            Some(t) => Some(Bit {
                text: t.clone(),
                color: Some(Color::Other(secondary_color)),
                font: None,
            }),
            None => None,
        };

        let short_text = format!(
            "{}  {}",
            icon_bit.as_pango_string(f),
            primary_bit.as_pango_string(f)
        );

        if let Some(secondary_bit) = secondary_bit_opt {
            let full_text = format!("{}  {}", short_text, secondary_bit.as_pango_string(f));
            (full_text, short_text)
        } else {
            (short_text.clone(), short_text)
        }
    }
}
