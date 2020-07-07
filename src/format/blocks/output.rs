use crate::format::bit::Bit;
use crate::format::color::Color;
use crate::format::{Attention, Formatter};
use serde::{Deserialize, Serialize};

/// The output of a Block.
#[derive(Serialize, Deserialize)]
pub struct BlockOutput {
    /// The name of the original block.
    pub block_name: String,

    /// The body of the BlockOutput, optional. If None, the output is removed.
    pub body: Option<BlockOutputBody>,
}

impl BlockOutput {
    /// Returns a new BlockOutput.
    pub fn new(block_name: &str, body: Option<BlockOutputBody>) -> Self {
        Self {
            block_name: String::from(block_name),
            body,
        }
    }

    /// Formats the BlockOutput for the i3 JSON protocol. None if body is None.
    pub fn as_json_protocol_string(&self, f: &Formatter) -> Option<String> {
        if let Some(body) = &self.body {
            let (full_text, short_text) = match body {
                BlockOutputBody::Nice(n) => n.as_pango_strings(f),
                BlockOutputBody::SingleBit(b) => {
                    let pango = b.as_pango_string(f);
                    (pango.clone(), pango)
                }
                BlockOutputBody::Custom(c) => (c.clone(), c.clone()),
            };

            let json = JsonBlock {
                full_text,
                short_text,
                separator: true,
                markup: String::from("pango"),
                name: self.block_name.clone(),
            };

            match serde_json::to_string(&json) {
                Ok(s) => Some(s),
                Err(_) => None,
            }
        } else {
            None
        }
    }
}

/// The body of a BlockOutput.
#[derive(Debug, Serialize, Deserialize)]
pub enum BlockOutputBody {
    /// Sexy block output.
    Nice(NiceOutput),

    /// An output of a single Bit.
    SingleBit(Bit),

    /// Custom output.
    Custom(String),
}

impl From<NiceOutput> for BlockOutputBody {
    fn from(n: NiceOutput) -> Self {
        Self::Nice(n)
    }
}

impl From<Bit> for BlockOutputBody {
    fn from(b: Bit) -> Self {
        Self::SingleBit(b)
    }
}

impl From<String> for BlockOutputBody {
    fn from(s: String) -> Self {
        Self::Custom(s)
    }
}

impl From<&str> for BlockOutputBody {
    fn from(s: &str) -> Self {
        Self::Custom(String::from(s))
    }
}

/// A struct for creating nice-looking block outputs.
#[derive(Serialize, Deserialize, Debug)]
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
    /// Formats the output as a pango string.
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

#[derive(Serialize, Deserialize, Debug)]
struct JsonBlock {
    name: String,
    full_text: String,
    short_text: String,
    separator: bool,
    markup: String,
}
