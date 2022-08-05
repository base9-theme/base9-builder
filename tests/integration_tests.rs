use std::{fs, str::FromStr};
use itertools::Itertools;

use base9_builder;

#[test]
fn correct_absolute_color() {
    let contents = fs::read_to_string("tests/palette_with_correct_absolute_order").unwrap();
    for palette_str in contents.split('\n') {
        if palette_str.len() == 0 || palette_str.chars().next().unwrap() == '#' {
            continue;
        }
        let palette = base9_builder::Palette::from_str(palette_str).unwrap();
        let data = base9_builder::to_mustache_data(&palette);
        let template = mustache::compile_str(include_str!("../templates/absolute.mustache")).unwrap();
        let actual = template.render_data_to_string(&data).unwrap();
        assert_eq!(actual, palette_str[((6+1)*2)..((6+1)*(6+2)-1)], "wrong order: https://coolors.co/{}.", palette_str);
    }
}