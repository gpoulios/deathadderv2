pub mod core {
    use std::{num::ParseIntError, fmt, error, default::Default};
    use serde::{Serialize, Deserialize};
    use confy::{ConfyError};
    use rgb::{RGB8, FromSlice};

    #[derive(Debug, Serialize, Deserialize)]
    pub struct Config {
        pub color: RGB8,
        pub wheel_color: Option<RGB8>,
    }

    impl Config {
        pub fn save(&self) -> Result<(), ConfyError> {
            confy::store("deathadder", None, self)
        }

        pub fn load() -> Option<Self> {
            match confy::load("deathadder", None) {
                Ok(cfg) => Some(cfg),
                Err(_) => None
            }
        }
    }

    impl Default for Config {
        fn default() -> Self { 
            Self { 
                color: RGB8::new(0xAA, 0xAA, 0xAA), 
                wheel_color: None 
            }
        }
    }

    #[derive(Debug)]
    pub enum ParseRGBError {
        WrongLength(usize),
        ParseHex(ParseIntError),
    }
    
    impl fmt::Display for ParseRGBError {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            match *self {
                ParseRGBError::WrongLength(len) =>
                    write!(f, "excluding pre/suffixes, \
                        string can only be of length 3 or 6 ({} given)", len),
                ParseRGBError::ParseHex(ref pie) =>
                    write!(f, "{}", pie),
            }
        }
    }
    
    impl error::Error for ParseRGBError {
        fn source(&self) -> Option<&(dyn error::Error + 'static)> {
            match *self {
                ParseRGBError::WrongLength(_) => None,
                ParseRGBError::ParseHex(ref pie) => Some(pie),
            }
        }
    }
    
    impl From<ParseIntError> for ParseRGBError {
        fn from(err: ParseIntError) -> ParseRGBError {
            ParseRGBError::ParseHex(err)
        }
    }
    
    pub fn rgb_from_hex(input: &str) -> Result<RGB8, ParseRGBError> {
        let s = input
            .trim_start_matches("0x")
            .trim_start_matches("#")
            .trim_end_matches("h");
    
        match s.len() {
            3 => {
                match s.chars()
                    .map(|c| u8::from_str_radix(format!("{}{}", c, c).as_str(), 16))
                    .collect::<Result<Vec<u8>, ParseIntError>>() {
                    Ok(res) => Ok(res.as_rgb()[0]),
                    Err(pie) => Err(ParseRGBError::from(pie))
                }
            },
            6 => {
                match (0..s.len())
                    .step_by(2)
                    .map(|i| u8::from_str_radix(&s[i..i + 2], 16))
                    .collect::<Result<Vec<u8>, ParseIntError>>() {
                    Ok(res) => Ok(res.as_rgb()[0]),
                    Err(pie) => Err(ParseRGBError::from(pie))
                }
            },
            _ => {
                Err(ParseRGBError::WrongLength(s.len()))
            }
        }
    }
}

pub mod v2 {
    use std::{time::Duration};
    use rusb::{
        Context, UsbContext,
    };
    use rgb::{
        RGB8
    };
    use crate::core::Config;

    pub fn preview_color(color: RGB8, wheel_color: Option<RGB8>) -> Result<String, String> {
        _set_color(color, wheel_color, false)
    }

    pub fn set_color(color: RGB8, wheel_color: Option<RGB8>) -> Result<String, String> {
        _set_color(color, wheel_color, true)
    }

    fn _set_color(color: RGB8, wheel_color: Option<RGB8>, save: bool) -> Result<String, String> {
        let vid = 0x1532;
        let pid = 0x0084;
    
        let timeout = Duration::from_secs(1);

        // save regardless of USB result and fail silently
        if save {
            _ = Config {color, wheel_color}.save();
        }
    
        match Context::new() {
            Ok(context) => match context.open_device_with_vid_pid(vid, pid) {
                Some(handle) => {
    
                    let mut packet: Vec<u8> = vec![
                        // the start (no idea what they are)
                        0x00, 0x1f, 0x00, 0x00, 0x00, 0x0b, 0x0f, 0x03, 0x00, 0x00, 0x00, 0x00, 0x01,

                        // wheel RGB (3B) | body RGB (3B)
                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff,

                        // the trailer (no idea what they are either)
                        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x06, 0x00
                    ];
                    
                    packet.splice(16..19, color.iter());
                    packet.splice(13..16, wheel_color.unwrap_or(color).iter());
                    
                    match handle.write_control(0x21, 9, 0x300, 0,
                        &packet, timeout) {
                            Ok(len) => Ok(format!("written {} bytes", len)),
                            Err(e) => Err(format!("could not write ctrl transfer: {}", e))
                    }
                }
                None => Err(format!("could not find device {:04x}:{:04x}", vid, pid)),
            },
            Err(e) => Err(format!("could not initialize libusb: {}", e)),
        }
    }
}