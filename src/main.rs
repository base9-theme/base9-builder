
use config::Config;
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
use regex::Regex;
use serde_yaml::{self, Mapping};

mod utils;
mod color_science;
mod config;
use color_science::Rgb;

const N: usize = 9;
type Palette = [Rgb;N];

fn default_config() -> Config {
    serde_yaml::from_str(include_str!("default_config.yml")).unwrap()
}

fn read_stdin() -> Result<String> {
    let mut buf = String::new();
    let mut stdin = io::stdin();
    stdin.read_to_string(&mut buf)?;
    Ok(buf)
}

fn cli() -> Command<'static> {
    Command::new("git")
        .about("base9 builder CLI")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("render")
                .about("renders theme template")
                //TODO(CONTRIB): make sure to use the term "palette code" everywhere.
                .arg(arg!(<PALETTE> "the palette code. use `-` for default palette."))
                .arg(arg!(<TEMPLATE> "path to template file. Use `-` to read from stdin."))
                .arg(
                    arg!([DEST] "path to write output to.")
                    .value_parser(clap::value_parser!(std::path::PathBuf)))
        )
        .subcommand(
            Command::new("preview")
                .about("prints a table of all generated colors to preview")
                .arg(arg!(<PALETTE> "the palette code. use `-` for default palette."))
        )
        // .subcommand(
        //     Command::new("list-variables")
        //         .about("prints all variables used by templates")
        //         .arg(arg!(<PALETTE> "The palette code"))
        // )
}

fn matches_to_formatted_variables(matches: &ArgMatches) -> Result<serde_yaml::Value> {
    let mut config = default_config();
    let mut palette_arg: &str = matches.get_one::<String>("PALETTE").ok_or(anyhow!("missing palette!"))?;

    if palette_arg != "-" {
        config.palette = config::parse_palette(palette_arg)?;
    }

    // Add config

    let variables = get_variables(&config)?;
    Ok(format_variables(&config, &variables))
}

fn main() -> Result<()> {
    let matches = cli().get_matches();
    match matches.subcommand() {
        Some(("render", sub_matches)) => {
            let formatted_variables = matches_to_formatted_variables(&sub_matches)?;
            let template_arg = sub_matches.get_one::<String>("TEMPLATE").unwrap();

            let template = if template_arg == "-" {
                let template_str = read_stdin()?;
                compile_str(&template_str)
            } else {
                compile_path(template_arg)
            }?;
            match sub_matches.get_one::<PathBuf>("DEST") {
                None => template.render(&mut io::stdout(), &formatted_variables)?,
                Some(dest) => {
                    let mut dest_file = utils::get_write(&dest)?;
                    template.render(&mut dest_file, &formatted_variables)?
                },
            };

            return Ok(());
        }
        Some(("preview", sub_matches)) => {
            let formatted_variables = matches_to_formatted_variables(&sub_matches)?;
            compile_str(include_str!("preview.mustache"))?.render(&mut io::stdout(), &formatted_variables)?;
        }
        Some(("list-variables", sub_matches)) => {
            let formatted_variables = matches_to_formatted_variables(&sub_matches)?;
            println!("{}", serde_yaml::to_string(&formatted_variables)?);
        }
        _ => unreachable!()
    }

    return Ok(());
}

fn parse_palette(palette: &str) -> Result<Palette> {
    let re = Regex::new(r"([0-9a-fA-F]{6}-){8}[0-9a-fA-F]{6}").unwrap();
    if !re.is_match(palette) {
        bail!("color palette in wrong format");
    }

    palette.split('-').map(|s| {
        let r = u8::from_str_radix(&s[0..2], 16).unwrap();
        let g = u8::from_str_radix(&s[2..4], 16).unwrap();
        let b = u8::from_str_radix(&s[4..6], 16).unwrap();
        return Srgb::from_components((r,g,b));
    }).collect::<Vec<Rgb>>().try_into().or(Err(anyhow!("color parsing failed.")))
}

#[derive(Debug)]
enum ColorMap {
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

fn get_variables(config: &Config) -> Result<Rc<RefCell<ColorMap>>> {

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

fn format_varialbes_helper2(list: &mut Vec<serde_yaml::Value>, prefix: &mut Vec<String>, color_map: &Rc<RefCell<ColorMap>>, formats: &Vec<(&str, fn(&Rgb) -> String)> ) {
    match &*color_map.deref().borrow() {
        ColorMap::Color(c) => {
            let mut mapping: Mapping = Mapping::new();
            mapping.insert("path".into(), prefix_to_path(prefix));
            mapping.insert("color".into(), color_to_formats(c, formats));
            list.push(serde_yaml::Value::Mapping(mapping));
        }
        ColorMap::Map(map) => {
            let mut mapping: Mapping = Mapping::new();
            mapping.insert("path".into(), prefix_to_path(prefix));
            list.push(serde_yaml::Value::Mapping(mapping));
            for (key, value) in map {
                prefix.push(key.clone());
                format_varialbes_helper2(list, prefix, value, formats);
                prefix.pop();
            }
        }
    }
}

fn format_varialbes_helper(color_map: &Rc<RefCell<ColorMap>>, formats: &Vec<(&str, fn(&Rgb) -> String)> ) -> serde_yaml::Value {
    match &*color_map.deref().borrow() {
        ColorMap::Color(c) => {
            let mut data_map: Mapping = Mapping::new();
            for (name, f) in formats {
                data_map.insert(name.to_string().into(), f(c).into());
            }
            serde_yaml::Value::Mapping(data_map)
        }
        ColorMap::Map(map) => {
            let mut data_map: Mapping = Mapping::new();

            for (key, value) in map {
                data_map.insert(key.clone().into(), format_varialbes_helper(value, formats));
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

fn format_variables(config: &Config, color_map: &Rc<RefCell<ColorMap>>) -> serde_yaml::Value {
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
    let mut colors = format_varialbes_helper(color_map, &formats);
    let mut list = Vec::<serde_yaml::Value>::new();
    format_varialbes_helper2(&mut list, &mut Vec::<String>::new(), color_map, &formats);
    let mapping = colors.as_mapping_mut().unwrap();
    let list = Vec::from(&list[1..]);
    mapping.insert("PROGRAMMABLE".into(), serde_yaml::Value::Sequence(list));

    mapping.insert("PALETTE".into(), config.palette.colors.map(|x| format!("{:x}", x)).join("-").into());
    colors
}


fn tmp_print(perm: &[&Rgb]) {
    let mut r_s = String::new();
    for i in 0..6 {
        let r = perm[i];
        r_s += &format!("{:x}-", r);
    }
    println!("{}", r_s);
}
