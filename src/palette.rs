use std::{fmt, str::FromStr};
use ext_palette::Srgb;
use serde::{Serialize, de::{Visitor, self}, Deserialize, Deserializer};

use crate::Color;

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Palette {
    pub colors: [Color;9]
}

pub struct PaletteOption {
    pub colors: [Option<Color>;9]
}

impl fmt::Display for Palette {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = self.colors.map(|c| format!("{:x}", c)).join("-");
        f.write_str(&s)?;
        Ok(())
    }
}

impl fmt::Display for PaletteOption {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = self.colors.map(|co| co.map_or("".to_string(), |c| format!("{:x}", c))).join("-");
        f.write_str(&s)?;
        Ok(())
    }
}

impl FromStr for Palette {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if !const_regex::match_regex!(r"([0-9a-fA-F]{6}-){8}[0-9a-fA-F]{6}", s.as_bytes()) {
            return Err(format!("color palette in wrong format: {}", s));
        }

        Ok(Palette { colors: s.split('-').map(|s| {
            let r = u8::from_str_radix(&s[0..2], 16).unwrap();
            let g = u8::from_str_radix(&s[2..4], 16).unwrap();
            let b = u8::from_str_radix(&s[4..6], 16).unwrap();
            return Srgb::from_components((r,g,b));
        }).collect::<Vec<Color>>().try_into().unwrap()})
    }
}

impl FromStr for PaletteOption {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if !const_regex::match_regex!(r"(([0-9a-fA-F]{6})?-){8}([0-9a-fA-F]{6})?", s.as_bytes()) {
            return Err(format!("color palette in wrong format: {}", s));
        }

        Ok(PaletteOption { colors: s.split('-').map(|s| {
            if s.len() == 0 {
                None
            } else {
                let r = u8::from_str_radix(&s[0..2], 16).unwrap();
                let g = u8::from_str_radix(&s[2..4], 16).unwrap();
                let b = u8::from_str_radix(&s[4..6], 16).unwrap();
                Some(Srgb::from_components((r,g,b)))
            }
        }).collect::<Vec<Option<Color>>>().try_into().unwrap()})
    }
}

impl Serialize for Palette {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl Serialize for PaletteOption {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

struct PaletteVisitor;

impl<'de> Visitor<'de> for PaletteVisitor {
    type Value = Palette;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("base9 palette code")
    }

    fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Self::Value::from_str(s).map_err(|x| E::custom(x))
    }
}

struct PaletteOptionVisitor;

impl<'de> Visitor<'de> for PaletteOptionVisitor {
    type Value = PaletteOption;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("base9 palette code")
    }

    fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Self::Value::from_str(s).map_err(|x| E::custom(x))
    }
}

impl<'de> Deserialize<'de> for Palette {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(PaletteVisitor)
    }
}

impl<'de> Deserialize<'de> for PaletteOption {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(PaletteOptionVisitor)
    }
}