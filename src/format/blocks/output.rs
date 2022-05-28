use crate::format::bit::Bit;
use crate::format::color::Color;
use crate::format::{Attention, Formatter};
use serde::{Deserialize, Serialize};

/// The output of a Block.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlockOutput {
    /// The name of the original block.
    block_name: String,

    /// The icon of the block.
    icon: Option<char>,

    /// The primary text of the block, to be displayed with a primary color.
    primary_text: String,

    /// The secondary text of the block, to be displayed with a secondary color.
    secondary_text: Option<String>,

    /// The Attention level of the output, which may give the block a special color.
    attention: Attention,
}

impl BlockOutput {
    /// Returns the name of the block this output is from.
    pub fn name(&self) -> String {
        self.block_name.to_owned()
    }

    /// Formats the output as a pango string. The first string returned is the full text including
    /// icon, primary text, and secondary text. The second string is the same but excludes the
    /// secondary text.
    pub fn as_pango_strings(&self, f: &Formatter) -> (String, String) {
        let (primary_color, secondary_color) = self.attention.colors(f);

        let icon_bit_opt = self.icon.map(|icon| Bit {
            text: icon.to_string(),
            color: Some(Color::Other(primary_color.clone())),
            font: Some(f.icon_font.clone()),
        });

        let primary_bit = Bit {
            text: self.primary_text.clone(),
            color: Some(Color::Other(primary_color)),
            font: None,
        };

        let secondary_bit_opt = self.secondary_text.as_ref().map(|t| Bit {
            text: t.clone(),
            color: Some(Color::Other(secondary_color)),
            font: None,
        });

        let short_text = if let Some(icon_bit) = icon_bit_opt {
            format!(
                "{}  {}",
                icon_bit.as_pango_string(f),
                primary_bit.as_pango_string(f)
            )
        } else {
            primary_bit.as_pango_string(f)
        };

        if let Some(secondary_bit) = secondary_bit_opt {
            let full_text = format!("{}  {}", short_text, secondary_bit.as_pango_string(f));
            (full_text, short_text)
        } else {
            (short_text.clone(), short_text)
        }
    }
}
