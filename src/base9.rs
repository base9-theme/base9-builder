
use ext_palette::rgb::channels::Argb;
use itertools::Itertools;
use ext_palette::{
    Srgb, IntoColor,
};
use std::cell::RefCell;
use std::ops::Deref;
use std::rc::Rc;
use std::{
    collections::HashMap,
};
use anyhow::{Result, bail};
use serde_json::{self, Map, Value};

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
        }
    }
    Ok(())
}

pub(crate) fn get_variables(config: &Config) -> Result<Rc<RefCell<ColorMap>>> {

    let variables_rc = Rc::new(RefCell::new(ColorMap::new_map()));
    {
        let variables = &mut *variables_rc.borrow_mut();

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

fn prefix_to_path(prefix: &Vec<String>) -> Value {
    let mut path: Map<String, Value> = Map::new();
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
    Value::Object(path)
}

fn list_color_map(list: &mut Vec<Value>, prefix: &mut Vec<String>, color_map: &Rc<RefCell<ColorMap>>, f: ColorFn) {
    match &*color_map.borrow() {
        ColorMap::Color(c) => {
            let mut mapping: Map<String, Value> = Map::new();
            mapping.insert("path".into(), prefix_to_path(prefix));
            mapping.insert("color".into(), f(&c));
            list.push(Value::Object(mapping));
        }
        ColorMap::Map(map) => {
            let mut mapping: Map<String, Value> = Map::new();
            mapping.insert("path".into(), prefix_to_path(prefix));
            mapping.insert("begin".into(), true.into());
            list.push(Value::Object(mapping));
            for (key, value) in map {
                prefix.push(key.clone());
                list_color_map(list, prefix, &value, f);
                prefix.pop();
            }
            let mut mapping: Map<String, Value> = Map::new();
            mapping.insert("path".into(), prefix_to_path(prefix));
            mapping.insert("end".into(), true.into());
            list.push(Value::Object(mapping));
        }
    }
}

pub type ColorFn = fn(&Rgb) -> Value;

pub(crate) fn map_color_map(color_map: &Rc<RefCell<ColorMap>>, f: ColorFn ) -> Value {
    match &*color_map.deref().borrow() {
        ColorMap::Color(c) => {
            f(c)
        }
        ColorMap::Map(map) => {
            let mut data_map: Map<String, Value> = Map::new();

            for (key, value) in map {
                data_map.insert(key.clone().into(), map_color_map(value, f));
            }

            Value::Object(data_map)
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

pub fn color_to_format(c: &Rgb) -> Value {
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
        let mut data_map: Map<String, Value> = Map::new();
        for (name, f) in formats {
            data_map.insert(name.to_string().into(), f(c).into());
        }
        Value::Object(data_map)
}
pub(crate) fn is_dark(config: &Config) -> bool {
    let bg: ext_palette::Lab = config.palette.colors[0].into_format().into_color();
    let fg: ext_palette::Lab = config.palette.colors[1].into_format().into_color();
    bg.l < fg.l
}

pub(crate) fn format_variables(config: &Config, color_map: &Rc<RefCell<ColorMap>>) -> Value {
    let mut colors = map_color_map(color_map, color_to_format);
    let mut list = Vec::<Value>::new();
    list_color_map(&mut list, &mut Vec::<String>::new(), color_map, color_to_format);
    let mapping = colors.as_object_mut().unwrap();
    let list = Vec::from(&list[1..(list.len()-1)]);
    mapping.insert("PROGRAMMABLE".into(), Value::Array(list));

    mapping.insert("PALETTE".into(), config.palette.colors.map(|x| format!("{:x}", x)).join("-").into());
    colors
}