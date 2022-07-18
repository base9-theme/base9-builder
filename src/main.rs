
use base9::{get_variables, format_variables};
use config::{Config, default_config};
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
mod base9;
mod config;
use color_science::Rgb;

const N: usize = 9;
type Palette = [Rgb;N];




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
    let palette_arg: &str = matches.get_one::<String>("PALETTE").ok_or(anyhow!("missing palette!"))?;

    if palette_arg != "-" {
        config.palette = config::Palette::from_str(palette_arg).map_err(|x| anyhow!("{}", x))?;
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
