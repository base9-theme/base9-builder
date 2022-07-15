use std::{collections::HashMap, fmt::{self}};
use palette::Srgb;
use regex::Regex;
use serde::{Serialize, Deserialize, de::{Visitor, self}, Deserializer};

pub type Rgb = Srgb<u8>;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Config {
    pub palette: Palette,
    pub shades: HashMap<String, f32>,
    pub colors: HashMap<String, ColorNames>,
}

pub fn parse_palette(s: &str) -> Result<Palette, serde::de::value::Error>  {
    PaletteVisitor{}.visit_str(s)
}

impl Config {
    pub fn tmp() -> Config {
        Config {
            palette: parse_palette("001153-cad1ea-f958a8-e3c0ae-97bda5-00b8dc-00abff-968dff-ee8394").unwrap(),
            shades: HashMap::from([
                ("p10".to_string(), 1.),
            ]),
            colors: HashMap::from([
                ("x".into(), ColorNames::Mapping(HashMap::new())),
                ("y".into(), ColorNames::Reference(Reference{ string: "color".to_string() })),
            ]),
        }
    }
}


#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Shades {
    pub p10: f32,
    pub p25: f32,
    pub p50: f32,
    pub p75: f32,
    pub p100: f32,
    pub p125: f32,
}

#[derive(Debug, PartialEq)]
pub enum ColorNames {
    BuiltIn,
    Reference(Reference),
    Mapping(HashMap<String, ColorNames>),
}

impl Serialize for ColorNames {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            ColorNames::BuiltIn => serializer.serialize_str("BUILT_IN"),
            ColorNames::Reference(s) => serializer.serialize_str(&s.string),
            ColorNames::Mapping(hash) => {
                use serde::ser::SerializeMap;
                let mut map = serializer.serialize_map(Some(hash.len()))?;
                for (k, v) in hash {
                    map.serialize_entry(k, v)?;
                }
                map.end()
            }
        }
    }
}

struct ColorNamesVisitor;

impl<'de> Visitor<'de> for ColorNamesVisitor {
    type Value = ColorNames;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("string or map")
    }

    fn visit_str<E>(self, s: &str) -> Result<ColorNames, E>
    where
        E: de::Error,
    {
        if s == "BUILT_IN" {
            return Ok(ColorNames::BuiltIn);
        }
        let re = Regex::new(r"[a-z][0-9a-z_]*(\.[a-z][0-9a-z_]*)*").unwrap();
        if re.is_match(s) {
            return Ok(ColorNames::Reference(Reference {string: s.to_string()}));
        }
        Err(E::custom("invalid color name"))
    }

    fn visit_map<V>(self, mut visitor: V) -> Result<ColorNames, V::Error>
    where
        V: de::MapAccess<'de>,
    {
        let mut values = HashMap::new();

        while let Some((key, value)) = visitor.next_entry()? {
            values.insert(key, value);
        }

        Ok(ColorNames::Mapping(values))
    }
}

impl<'de> Deserialize<'de> for ColorNames {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(ColorNamesVisitor)
    }
}

#[derive(Debug, PartialEq)]
pub struct Palette {
    pub colors: [Rgb;9]
}

impl Serialize for Palette {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let s = self.colors.map(|c| format!("#{:x}", c)).join("-");
        serializer.serialize_str(&s)
    }
}

struct PaletteVisitor;

impl<'de> Visitor<'de> for PaletteVisitor {
    type Value = Palette;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("base9 palette code")
    }

    fn visit_str<E>(self, s: &str) -> Result<Palette, E>
    where
        E: de::Error,
    {
        let re = Regex::new(r"([0-9a-fA-F]{6}-){8}[0-9a-fA-F]{6}").unwrap();
        if !re.is_match(s) {
            return Err(E::custom("color palette in wrong format"));
        }

        Ok(Palette { colors: s.split('-').map(|s| {
            let r = u8::from_str_radix(&s[0..2], 16).unwrap();
            let g = u8::from_str_radix(&s[2..4], 16).unwrap();
            let b = u8::from_str_radix(&s[4..6], 16).unwrap();
            return Srgb::from_components((r,g,b));
        }).collect::<Vec<Rgb>>().try_into().unwrap()})
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

#[derive(Debug, PartialEq)]
pub struct Reference {
    string: String,
}

impl Reference {
    pub fn key_iter<'a>(&'a self) -> std::str::Split<&str>
    {
        self.string.split(".")
    }
}