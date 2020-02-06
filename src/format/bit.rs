use super::color::Color;

pub struct Bit {
    pub text: String,
    pub color: Option<Color>,
    pub font: Option<String>,
}

// impl Bit {
//     pub fn as_lemonbar_string(&self) -> String {
//         let mut s = self.text;

//         if let Some(c) = self.color {
//             s = format!("%{{F#{}}}{}", c.hex_string(Mode::Lemonbar), self.text);
//         }

//         s
//     }

//     pub fn as_json_protocol_string(&self) -> String {
//         match self.color {
//             Some(c) => match self.font {
//                 Some(f) => format!("<span color=\"#{}\" font=\"{}\">{}</span>", c.hex_string(Mode::JsonProtocol), f, self.text), // color and font
//                 None => format!("<span color=\"#{}\">{}</span>", c.hex_string(Mode::JsonProtocol), self.text), // color, but no font
//             },
//             None => match self.font {
//                 Some(f) => format!("<span font=\"{}\">{}</span>", f, self.text), // font, but no color
//                 None => self.text.clone(), // no color and no font
//             },
//         }
//     }

//     pub fn from_mode(&self, mode: Mode) -> String {
//         match mode {
//             Mode::JsonProtocol => self.as_json_protocol_string(),
//             Mode::Lemonbar => self.as_lemonbar_string(),
//         }
//     }
// }
