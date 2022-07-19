
use palette::convert::IntoColorUnclamped;
use palette::rgb::channels::Argb;
use clap::{arg, command, ArgAction, Command, ArgMatches};
use itertools::Itertools;
use palette::{
    Srgb,
    Xyz,
    Lab, IntoColor, Hsl, Lch,
};
use std::cell::RefCell;
use std::io::{self, Read};
use std::ops::Deref;
use std::path::PathBuf;
use std::rc::Rc;
use std::{
    collections::HashMap,
    env,
};
use anyhow::{Result, bail, anyhow};
use mustache::{Data, compile_path, compile_str};
use serde_yaml::{self, Mapping};

use crate::color_science::{Rgb, self};
use crate::config::{Config, self};

#[derive(Debug)]
pub(crate) enum ColorMap {
    Color(Rgb),
    Map(HashMap<String, Rc<RefCell<ColorMap>>>),
}

impl ColorMap {
    fn new_map() -> ColorMap {
        ColorMap::Map(HashMap::new())
    }

    fn insert_color(&mut self, key: String, value: Rgb) -> Result<()> {
        match self {
            ColorMap::Color(_) => bail!("can't insert key into color"),
            ColorMap::Map(map) => {
                map.insert(key, Rc::new(RefCell::new(ColorMap::Color(value))));
                Ok(())
            },
        }
    }
    fn insert(&mut self, key: String, value: Rc<RefCell<ColorMap>>) -> Result<()> {
        match self {
            ColorMap::Color(_) => bail!("can't insert key into color"),
            ColorMap::Map(map) => {
                map.insert(key, value);
                Ok(())
            },
        }
    }
}

fn new_color_shade_map(color: &Rgb, bg: &Rgb, config: &Config) -> Result<Rc<RefCell<ColorMap>>> {
    let mut map = ColorMap::new_map();
    for (key, value) in &config.shades {
        map.insert_color(key.clone(), color_science::mix(bg, color, *value))?;
    }

    Ok(Rc::new(RefCell::new(map)))
}

fn add_colors(aliases: &HashMap<String, config::ColorNames>, current_map: Rc<RefCell<ColorMap>>, color_map: Rc<RefCell<ColorMap>>) -> Result<()> {
    for (key, value) in aliases {
        match value {
            config::ColorNames::BuiltIn => {},
            config::ColorNames::Reference(reference) => {
                let mut ptr: Rc<RefCell<ColorMap>> = color_map.clone();
                for k in reference.key_iter() {
                    let tmp = match &*ptr.deref().borrow() {
                        ColorMap::Color(_) => bail!("..."),
                        ColorMap::Map(map) => map.get(k).unwrap().clone(),
                    };
                    ptr = tmp;
                }
                (&mut *current_map.deref().borrow_mut()).insert(key.clone(), ptr.clone())?;
            },
            config::ColorNames::Mapping(tmp) => {
                let map2 = Rc::new(RefCell::new(ColorMap::new_map()));
                add_colors(tmp, map2.clone(), color_map.clone())?;
                (&mut *current_map.deref().borrow_mut()).insert(key.clone(), map2)?;
            },
            _ => bail!("other value types"),
        }
    }
    Ok(())
}

pub(crate) fn get_variables(config: &Config) -> Result<Rc<RefCell<ColorMap>>> {

    let variables_rc = Rc::new(RefCell::new(ColorMap::new_map()));
    {
        let variables = &mut *variables_rc.deref().borrow_mut();

        let bg = config.palette.colors[0];
        variables.insert_color("background".into(), bg)?;

        let fg = config.palette.colors[1];
        variables.insert("foreground".into(), new_color_shade_map(&fg, &bg, config)?)?;

        // c1...c7
        let hues = &config.palette.colors[2..9];
        for (i, c) in hues.iter().enumerate() {
            let name = format!("c{}", i+1);
            variables.insert(name, new_color_shade_map(c, &bg, config)?)?;
        }

        let absolute_colors = [
            ("red", Srgb::from_u32::<Argb>(0xff0000)),
            ("yellow", Srgb::from_u32::<Argb>(0xffff00)),
            ("green", Srgb::from_u32::<Argb>(0x00ff00)),
            ("cyan", Srgb::from_u32::<Argb>(0x00ffff)),
            ("blue", Srgb::from_u32::<Argb>(0x0000ff)),
            ("magenta", Srgb::from_u32::<Argb>(0xff00ff)),
        ];

        let relative_colors = color_science::closest_order(hues, &absolute_colors.iter().map(|x| x.1).collect_vec());

        for (color, (name, _)) in relative_colors.into_iter().zip_eq(absolute_colors) {
            variables.insert(name.to_string(), new_color_shade_map(&color, &bg, config)?)?;
        }
    }
    add_colors(&config.colors, variables_rc.clone(), variables_rc.clone())?;
    Ok(variables_rc)
}

fn color_to_formats(c: &Rgb, formats: &Vec<(&str, fn(&Rgb) -> String)> ) -> serde_yaml::Value {
    let mut data_map: Mapping = Mapping::new();
    for (name, f) in formats {
        data_map.insert(name.to_string().into(), f(c).into());
    }
    serde_yaml::Value::Mapping(data_map)
}

fn prefix_to_path(prefix: &Vec<String>) -> serde_yaml::Value {
    let mut path: Mapping = Mapping::new();
    path.insert("dotted".into(), prefix.join(".").into());
    path.insert("indent".into(), " ".repeat(prefix.len()).into());
    path.insert("last".into(), prefix.last().unwrap_or(&"".to_string()).to_string().into());
    // let mut list = Vec::<serde_yaml::Value>::new();
    // for (i, key) in prefix.iter().enumerate() {
    //     let mut tmp = Mapping::new();
    //     tmp.insert("key".into(), key.clone().into());
    //     tmp.insert("last".into(), (i == prefix.len() - 1).into());
    //     list.push(tmp.into());
    // }
    // path.insert("list".into(), list.into());
    serde_yaml::Value::Mapping(path)
}

fn list_color_map(list: &mut Vec<serde_yaml::Value>, prefix: &mut Vec<String>, color_map: &Rc<RefCell<ColorMap>>, f: ColorFn) {
    match &*color_map.deref().borrow() {
        ColorMap::Color(c) => {
            let mut mapping: Mapping = Mapping::new();
            mapping.insert("path".into(), prefix_to_path(prefix));
            mapping.insert("color".into(), f(c));
            list.push(serde_yaml::Value::Mapping(mapping));
        }
        ColorMap::Map(map) => {
            let mut mapping: Mapping = Mapping::new();
            mapping.insert("path".into(), prefix_to_path(prefix));
            mapping.insert("begin".into(), true.into());
            list.push(serde_yaml::Value::Mapping(mapping));
            for (key, value) in map {
                prefix.push(key.clone());
                list_color_map(list, prefix, value, f);
                prefix.pop();
            }
            let mut mapping: Mapping = Mapping::new();
            mapping.insert("path".into(), prefix_to_path(prefix));
            mapping.insert("end".into(), true.into());
            list.push(serde_yaml::Value::Mapping(mapping));
        }
    }
}

type ColorFn = fn(&Rgb) -> serde_yaml::Value;

pub(crate) fn map_color_map(color_map: &Rc<RefCell<ColorMap>>, f: ColorFn ) -> serde_yaml::Value {
    match &*color_map.deref().borrow() {
        ColorMap::Color(c) => {
            f(c)
        }
        ColorMap::Map(map) => {
            let mut data_map: Mapping = Mapping::new();

            for (key, value) in map {
                data_map.insert(key.clone().into(), map_color_map(value, f));
            }

            serde_yaml::Value::Mapping(data_map)
        }
    }
}

// fn color_map_to_data(color_map: &ColorMap, f: fn(&Rgb) -> String ) -> Data {
//     match color_map {
//         ColorMap::Color(c) => Data::String(f(c)),
//         ColorMap::Map(map) => {
//             let mut data_map: HashMap<String, Data> = HashMap::new();
//             for (key, value) in map {
//                 // let tmp: Ref<ColorMap> = value.borrow();
//                 // data_map.insert(key.clone(), color_map_to_data(value.borrow().borrow(), f));
//             }
//             Data::Map(data_map)
//         }
//     }
// }

pub fn color_to_format(c: &Rgb) -> serde_yaml::Value {
        let formats: Vec<(&str, fn(&Rgb) -> String)> = vec![
            ("hex", |x: &Rgb| format!("{:x}", x)),
            ("hex_r", |x: &Rgb| format!("{:x}", x.red)),
            ("hex_g", |x: &Rgb| format!("{:x}", x.green)),
            ("hex_b", |x: &Rgb| format!("{:x}", x.blue)),
            ("int_r", |x: &Rgb| format!("{}", x.red)),
            ("int_g", |x: &Rgb| format!("{}", x.green)),
            ("int_b", |x: &Rgb| format!("{}", x.blue)),
            ("dec_r", |x: &Rgb| format!("{}", x.red as f64 / 255.)),
            ("dec_g", |x: &Rgb| format!("{}", x.green as f64 / 255.)),
            ("dec_b", |x: &Rgb| format!("{}", x.blue as f64 / 255.)),
        ];
        let mut data_map: Mapping = Mapping::new();
        for (name, f) in formats {
            data_map.insert(name.to_string().into(), f(c).into());
        }
        serde_yaml::Value::Mapping(data_map)
}

pub(crate) fn format_variables(config: &Config, color_map: &Rc<RefCell<ColorMap>>) -> serde_yaml::Value {
    let mut colors = map_color_map(color_map, color_to_format);
    let mut list = Vec::<serde_yaml::Value>::new();
    list_color_map(&mut list, &mut Vec::<String>::new(), color_map, color_to_format);
    let mapping = colors.as_mapping_mut().unwrap();
    let list = Vec::from(&list[1..(list.len()-1)]);
    mapping.insert("PROGRAMMABLE".into(), serde_yaml::Value::Sequence(list));

    mapping.insert("PALETTE".into(), config.palette.colors.map(|x| format!("{:x}", x)).join("-").into());
    colors
}