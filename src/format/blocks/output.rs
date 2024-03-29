use crate::format::color::RGBA;
use crate::format::{Attention, Formatter};
use crate::utils;
use serde::{Deserialize, Serialize};

/// The output of a Block.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlockOutput {
    /// The name of the original block.
    block_name: String,

    /// The icon of the block.
    icon: Option<char>,

    /// The text to show for the block.
    text: BlockText,

    /// The Attention level of the output, which may give the block a special color.
    attention: Attention,
}

impl BlockOutput {
    /// Creates a new `BlockOutput`.
    pub fn new(
        block_name: &str,
        icon: Option<char>,
        text: BlockText,
        attention: Attention,
    ) -> Self {
        Self {
            block_name: block_name.to_string(),
            icon,
            text,
            attention,
        }
    }

    /// Returns the name of the block this output is from.
    pub fn name(&self) -> String {
        self.block_name.to_owned()
    }

    /// Returns the block content (the text)
    pub fn text(&self) -> &BlockText {
        &self.text
    }

    /// Formats the output as a pango string. The first string returned is the full text including
    /// icon, primary text, and secondary text. The second string is the same but excludes the
    /// secondary text.
    pub fn as_pango_strings(&self, f: &Formatter) -> (String, Option<String>) {
        let (primary_color, secondary_color) = self.attention.colors(f);
        let icon_pango = self.icon.map(|i| {
            utils::make_pango_string(&i.to_string(), Some(primary_color), Some(&f.icon_font))
        });
        let (full, short_opt) = self.text.to_pango_strings(primary_color, secondary_color);
        if let Some(icon) = icon_pango {
            // strings with icons
            let full_with_icon = format!("{icon}  {full}");
            let short_with_icon = if let Some(short) = short_opt {
                // icon with short text
                format!("{icon} {short}") // i accidentally only put one space in here, but idk,
                                          // i kinda like it for shorter texts
            } else {
                // icon only, no short text
                icon
            };
            (full_with_icon, Some(short_with_icon))
        } else {
            // strings without icons
            (full, short_opt)
        }
    }
}

/// Text that is displayed with a block, either a single string or a primary and secondary string.
///
/// When displaying primary and secondary data, the two strings are colored differently. The intent
/// is to color the primary and secondary text differently such that the primary text is more
/// prominent as it's the more important piece of information.
///
/// This struct does not account for icons.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum BlockText {
    /// Only one string of text, primary-colored.
    Single(String),

    /// Two strings of text, primary and secondary.
    Pair(String, String),
}

impl BlockText {
    /// Returns the long and short versions of the `BlockText` in pango markup format.
    ///
    /// If `Single`, the long version is the string and the short version is `None`.
    ///
    /// If `Pair`, the long version is both strings, and the short version is only the primary
    /// string.
    ///
    /// TODO: Add `to_pango_strings_with_icon` to include an icon with the primary text and reduce
    /// the amount of markup we have to do
    fn to_pango_strings(
        &self,
        primary_color: RGBA,
        secondary_color: RGBA,
    ) -> (String, Option<String>) {
        // make the "second half" (i.e. secondary) part of the long string
        let second_half = match self {
            BlockText::Single(s) | BlockText::Pair(_, s) => {
                let color = match self {
                    BlockText::Single(_) => primary_color,
                    BlockText::Pair(_, _) => secondary_color,
                };
                utils::make_pango_string(s, Some(color), None)
            }
        };

        let short = self.to_short_pango_string(primary_color);
        let long = if let Some(ref short_str) = short {
            format!("{}  {}", short_str, second_half)
        } else {
            second_half
        };

        (long, short)
    }

    /// Returns the short version of the pango markup representation of this `BlockText`.
    ///
    /// If `Single`, the short version is `None`.
    ///
    /// If `Pair`, the short version is only the primary string.
    fn to_short_pango_string(&self, primary_color: RGBA) -> Option<String> {
        match self {
            BlockText::Single(_) => None,
            BlockText::Pair(p, _) => Some(utils::make_pango_string(p, Some(primary_color), None)),
        }
    }

    /// Returns the long version of the pango markup representation of this `BlockText`.
    ///
    /// If `Single`, the long version is the single string.
    ///
    /// If `Pair`, the long version is both strings, colored differently to accent the 'primary'
    /// text.
    #[allow(dead_code)]
    fn to_long_pango_string(&self, primary_color: RGBA, secondary_color: RGBA) -> String {
        // why is this implemented this way?
        //
        // this function should return *only* the long version of the text.
        //
        // if we moved the logic of `to_pango_strings` here, `to_pango_strings` would require two
        // calls to `to_short_pango_string`: once to get the short text, and another time to create
        // the long text here.
        //
        // by having the string-building logic in `to_pango_strings`, `to_short_pango_string` only
        // has to be called once if we wanted to create *both* short and long versions of the
        // `BlockText`.
        //
        // `to_short_pango_string` has to be called anyway to create the long string, so there's no
        // harm in having it called in `to_pango_strings`.
        self.to_pango_strings(primary_color, secondary_color).0
    }
}
