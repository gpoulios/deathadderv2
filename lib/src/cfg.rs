use std::default::Default;
use serde::{Serialize, Deserialize};
use confy::ConfyError;
use rgb::RGB8;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub same_color: bool,
    pub logo_color: RGB8,
    pub scroll_color: RGB8,
}

impl Config {
    pub fn save(&self) -> Result<(), ConfyError> {
        confy::store("deathadder_v2", None, self)
    }

    pub fn load() -> Option<Self> {
        match confy::load("deathadder_v2", None) {
            Ok(cfg) => Some(cfg),
            Err(_) => None
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            same_color: true,
            logo_color: RGB8::new(0xAA, 0xAA, 0xAA),
            scroll_color: RGB8::new(0xAA, 0xAA, 0xAA),
        }
    }
}