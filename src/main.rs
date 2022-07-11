
use palette::convert::IntoColorUnclamped;
use palette::rgb::channels::Argb;
use clap::{arg, command, ArgAction, Command, ArgMatches};
use itertools::Itertools;
use palette::{
    Srgb,
    Xyz,
    Lab, IntoColor,
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

const N: usize = 9;
type Rgb = Srgb<u8>;
type Palette = [Rgb;N];

fn default_config() -> Result<serde_yaml::Value> {
    let yaml_str = include_str!("default_config.yaml");
    let config = serde_yaml::from_str(yaml_str)?;
    Ok(config)
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
                .arg(arg!(<PALETTE> "The palette code. Example: 282936-E9E9F4-FF5555-FFB86C-F1FA8C-50FA7B-8BE9FD-BD93F9-FF79C6"))
                .arg(arg!(<TEMPLATE> "path to template file"))
        )
        .subcommand(
            Command::new("preview")
                .about("prints a table of all generated colors to preview")
                .arg(arg!(<PALETTE> "The palette code. Example: 282936-E9E9F4-FF5555-FFB86C-F1FA8C-50FA7B-8BE9FD-BD93F9-FF79C6"))
        )
        // .subcommand(
        //     Command::new("list-variables")
        //         .about("prints all variables used by templates")
        //         .arg(arg!(<PALETTE> "The palette code"))
        // )
}

fn matches_to_formatted_variables(matches: &ArgMatches) -> Result<serde_yaml::Value> {
    let palette_arg = matches.get_one::<String>("PALETTE").ok_or(anyhow!("missing palette!"))?;

    let re = Regex::new(r"([0-9a-fA-F]{6}-){8}[0-9a-fA-F]{6}").unwrap();
    if !re.is_match(palette_arg) {
        bail!("color palette in wrong format.");
    }

    // Add config
    let mut config = default_config()?;
    {
        let config_map = config.as_mapping_mut().ok_or(anyhow!("config not an object"))?;
        config_map.insert("palette".to_string().into(), palette_arg.to_string().into());
    }

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
            template.render(&mut io::stdout(), &formatted_variables)?;
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

fn new_color_shade_map(color: &Rgb, bg: &Rgb, config: &serde_yaml::Value) -> Result<Rc<RefCell<ColorMap>>> {
    let mut map = ColorMap::new_map();
    let shades = config.as_mapping().unwrap().get(&"shades".into()).unwrap().as_mapping().unwrap();
    for (key, value) in shades {
        let ratio = value.as_f64().unwrap() as f32;
        map.insert_color(key.as_str().unwrap().to_string(), mix(bg, color, ratio))?;
    }

    Ok(Rc::new(RefCell::new(map)))
}

fn add_colors(config: &serde_yaml::Value, current_map: Rc<RefCell<ColorMap>>, color_map: Rc<RefCell<ColorMap>>) -> Result<()> {
    if !config.is_mapping() {
        bail!("config not a map");
    }
    let config_map = config.as_mapping().unwrap();
    for (key, value) in config_map {
        match value {
            serde_yaml::Value::String(reference) => {
                // println!("begin");
                // match &value {
                //     serde_yaml::Value::String(s) => println!("{}", s),
                //     _ => println!("xxx"),
                // };
                if reference.eq("BUILT_IN") {
                    continue;
                }
                let mut ptr: Rc<RefCell<ColorMap>> = color_map.clone();
                for v in reference.split('.') {
                    let tmp = match &*ptr.deref().borrow() {
                        ColorMap::Color(_) => bail!("..."),
                        ColorMap::Map(map) => map.get(v).unwrap().clone(),
                    };
                    ptr = tmp;
                }
                (&mut *current_map.deref().borrow_mut()).insert(key.as_str().unwrap().to_string(), ptr.clone())?;
            },
            serde_yaml::Value::Mapping(_) => {
                let map2 = Rc::new(RefCell::new(ColorMap::new_map()));
                add_colors(value, map2.clone(), color_map.clone())?;
                (&mut *current_map.deref().borrow_mut()).insert(key.as_str().unwrap().to_string(), map2)?;
            },
            _ => bail!("other value types"),
        }
        if value.is_string() {
        }
    }
    Ok(())
    
}

fn distance(c1: &Rgb, c2: &Rgb) -> i64 {
    let ds = (c1.red as i64 - c2.red as i64, c1.green as i64 - c2.green as i64, c1.blue as i64 - c2.blue as i64);
    ds.0*ds.0 + ds.1*ds.1 + ds.2*ds.2
}

fn get_variables(config: &serde_yaml::Value) -> Result<Rc<RefCell<ColorMap>>> {
    let config_mapping = config.as_mapping().ok_or(anyhow!("config not an object"))?;
    let palette = config_mapping.get(&"palette".into()).ok_or(anyhow!("missing palette"))?
        .as_str().ok_or(anyhow!("palette not a string"))?;
    let palette = parse_palette(palette)?;

    let variables_rc = Rc::new(RefCell::new(ColorMap::new_map()));
    {
        let variables = &mut *variables_rc.deref().borrow_mut();

        let bg = palette[0];
        variables.insert_color("background".into(), bg)?;

        let fg = palette[1];
        variables.insert("foreground".into(), new_color_shade_map(&fg, &bg, config)?)?;

        // c1...c7
        let colors = &palette[2..9];
        for (i, c) in colors.iter().enumerate() {
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

        let relative_colors = colors.into_iter().permutations(6).min_by_key(|perm| {
            let mut sum: i64 = 0;
            for ((_, c1), c2) in absolute_colors.iter().zip_eq(perm) {
                sum += distance(&c1, c2);
            }
            sum
        }).unwrap();

        for (color, (name, _)) in relative_colors.into_iter().zip_eq(absolute_colors) {
            variables.insert(name.to_string(), new_color_shade_map(color, &bg, config)?)?;
        }

        


    }
    add_colors(config.as_mapping().unwrap().get(&"colors".into()).unwrap(), variables_rc.clone(), variables_rc.clone())?;
    Ok(variables_rc)
}

fn format_varialbes_helper(color_map: &Rc<RefCell<ColorMap>>, formats: &Vec<(&str, fn(&Rgb) -> String)> ) -> Data {
    match &*color_map.deref().borrow() {
        ColorMap::Color(c) => {
            let mut data_map: HashMap<String, Data> = HashMap::new();
            for (name, f) in formats {
                data_map.insert(name.to_string(), Data::String(f(c)));
            }
            Data::Map(data_map)
        }
        ColorMap::Map(map) => {
            let mut data_map: HashMap<String, Data> = HashMap::new();

            for (key, value) in map {
                data_map.insert(key.clone(), format_varialbes_helper(value, formats));
            }

            Data::Map(data_map)
        }
    }
}

fn format_varialbes_helper2(color_map: &Rc<RefCell<ColorMap>>, formats: &Vec<(&str, fn(&Rgb) -> String)> ) -> serde_yaml::Value {
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
                data_map.insert(key.clone().into(), format_varialbes_helper2(value, formats));
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

fn format_variables(_config: &serde_yaml::Value, color_map: &Rc<RefCell<ColorMap>>) -> serde_yaml::Value {
    let formats: Vec<(&str, fn(&Rgb) -> String)> = vec![
        ("hex", |x: &Rgb| format!("{x:x}")),
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
    format_varialbes_helper2(color_map, &formats)
}

fn mix1d(a: f32, b: f32, w: f32) -> f32 {
    return a*(1.-w)+b*w;
}

fn mix(c1: &Rgb, c2: &Rgb, w: f32) -> Rgb {
    let c1_xyz: Xyz = c1.into_format().into_color_unclamped();
    let c2_xyz: Xyz = c2.into_format().into_color_unclamped();
    let c1_lab: Lab = c1_xyz.into_color_unclamped();
    let c2_lab: Lab = c2_xyz.into_color();
    let c1_contrast = (c1_xyz.y + 0.05).ln();
    let c2_contrast = (c2_xyz.y + 0.05).ln();
    let c3_contrast = mix1d(c1_contrast, c2_contrast, w);
    let c3y = c3_contrast.exp() - 0.05;
    let c3l_lab: Lab = Xyz::new(0., c3y, 0.).into_color_unclamped();
    let c3 = Lab::new(c3l_lab.l, mix1d(c1_lab.a, c2_lab.a, w), mix1d(c1_lab.b, c2_lab.b, w));

    let c3: Srgb = c3.into_color_unclamped();
    c3.into_format()
}

