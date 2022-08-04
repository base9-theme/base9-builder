
use base9_builder::Palette;
use ext_palette::{IntoColor, Lab, rgb::channels::Argb, Srgb};
use rand::prelude::*;

use crate::{palette::PaletteOption, Color};

pub fn get_average_hue(palette_option: &PaletteOption) -> Option<Color> {
    let mut hue_average_lab_components = (0f32, 0f32, 0f32);
    let mut count = 0;
    for c in palette_option.colors {
        if c.is_none() { continue; }
        count += 1;
        let lab: ext_palette::Lab = c.unwrap().into_format().into_color();
        let comp = lab.into_components();
        hue_average_lab_components.0 += comp.0;
        hue_average_lab_components.1 += comp.1;
        hue_average_lab_components.2 += comp.2;
    }
    if count == 0 { return None; }
    let count = count as f32;
    hue_average_lab_components.0 /= count;
    hue_average_lab_components.1 /= count;
    hue_average_lab_components.2 /= count;
    let rtn: ext_palette::Srgb = ext_palette::Lab::from_components(hue_average_lab_components).into_color();
    Some(rtn.into_format())
}

pub fn generate(palette_option: &PaletteOption) -> Palette {
    let mut colors = palette_option.colors.clone();
    let mut rng = rand::thread_rng();
    let average_hue = get_average_hue(palette_option);
    let is_dark: bool = match (colors, average_hue) {
        ([Some(c), ..], _) => is_dark(&c),
        ([_, Some(c), ..], _) => !is_dark(&c),
        (_, Some(c)) => !is_dark(&c),
        _ => rand::random::<bool>(),
    };
    generate_bg(&mut rng, &mut colors, &average_hue, is_dark);
    generate_fg(&mut rng, &mut colors, is_dark);
    generate_hue(&mut rng, &mut colors, &average_hue, is_dark);
    Palette { colors: colors.map(|x| x.unwrap()) }
}

fn get_dark_color(rng: &mut impl rand::Rng) -> Color {
    let lab_0: Lab = Color::from_u32::<Argb>(0xff_10009cu32).into_format().into_color();
    let lab_x: Lab = Color::from_u32::<Argb>(0xff_003000u32).into_format().into_color();
    let lab_y: Lab = Color::from_u32::<Argb>(0xff_5a0000u32).into_format().into_color();
    let mut rand_l: f32 = rng.gen();
    let mut rand_x: f32 = rng.gen();
    let mut rand_y: f32 = rng.gen();
    if rand_x + rand_y > 1. {
        rand_x = 1. - rand_x;
        rand_y = 1. - rand_y;
    }
    let mut rand_0 = 1. - rand_x - rand_y;
    rand_0 *= rand_l;
    rand_x *= rand_l;
    rand_y *= rand_l;

    let mut rtn = Lab::new(0.,0.,0.);
    rtn.l += lab_0.l * rand_0;
    rtn.l += lab_x.l * rand_x;
    rtn.l += lab_y.l * rand_y;

    rtn.a += lab_0.a * rand_0;
    rtn.a += lab_x.a * rand_x;
    rtn.a += lab_y.a * rand_y;

    rtn.b += lab_0.b * rand_0;
    rtn.b += lab_x.b * rand_x;
    rtn.b += lab_y.b * rand_y;

    from_lab(rtn)
}

fn get_light_color(rng: &mut impl rand::Rng) -> Color {
    let lab_w: Lab = Color::from_u32::<Argb>(0xff_ffffffu32).into_format().into_color();
    let lab_0: Lab = Color::from_u32::<Argb>(0xff_00e2ffu32).into_format().into_color();
    let lab_x: Lab = Color::from_u32::<Argb>(0xff_00ef00u32).into_format().into_color();
    let lab_y: Lab = Color::from_u32::<Argb>(0xff_ffadffu32).into_format().into_color();
    let mut rand_l: f32 = rng.gen();
    let mut rand_x: f32 = rng.gen();
    let mut rand_y: f32 = rng.gen();

    let mut rand_0 = 1. - rand_x - rand_y;
    let rand_w = 1. - rand_l;
    rand_0 *= rand_l;
    rand_x *= rand_l;
    rand_y *= rand_l;

    let mut rtn = Lab::new(0.,0.,0.);
    rtn.l += lab_w.l * rand_w;
    rtn.l += lab_0.l * rand_0;
    rtn.l += lab_x.l * rand_x;
    rtn.l += lab_y.l * rand_y;

    rtn.a += lab_0.a * rand_0;
    rtn.a += lab_x.a * rand_x;
    rtn.a += lab_y.a * rand_y;

    rtn.b += lab_0.b * rand_0;
    rtn.b += lab_x.b * rand_x;
    rtn.b += lab_y.b * rand_y;

    from_lab(rtn)
}

fn is_dark(c: &Color) -> bool {
    let lab: ext_palette::Lab = c.into_format().into_color();
    lab.l < 50.0
}

fn generate_bg(rng: &mut impl rand::Rng, colors: &mut [Option<Color>;9], hue_average: &Option<Color>, is_dark: bool) {
    if colors[0].is_some() { return; }
    colors[0] = Some(if is_dark {
        get_dark_color(rng)
    } else {
        get_light_color(rng)
    });
}

fn generate_fg(rng: &mut impl rand::Rng, colors: &mut [Option<Color>;9], is_dark: bool) {
    if colors[1].is_some() { return; }
    colors[1] = Some(if is_dark {
        get_light_color(rng)
    } else {
        get_dark_color(rng)
    });
}

fn to_lab(c: Color) -> Lab {
    c.into_format().into_color()
}

fn from_lab(lab: Lab) -> Color {
    IntoColor::<Srgb>::into_color(lab).into_format()
}

pub fn length_and_angle(lab: &Lab, base: &Lab) -> (f32, f32) {
    let x = lab.a - base.a;
    let y = lab.b - base.b;
    let l = (x*x + y*y).sqrt();
    let angle = f32::atan2(x, y);

    (l, angle)
}

fn interpolate(bg: &Lab, fg: &Lab, l: f32) -> Lab {
    let mut center = Lab::new(l,0.,0.);

    if (bg.l < center.l) ^ (fg.l < center.l) {
        center.a = bg.a * (center.l - bg.l) / (fg.l - bg.l) + fg.a * (center.l - fg.l) / (bg.l - fg.l);
        center.b = bg.b * (center.l - bg.l) / (fg.l - bg.l) + fg.b * (center.l - fg.l) / (bg.l - fg.l);
    }

    center
}

pub fn generate_hue(rng: &mut impl rand::Rng, colors: &mut [Option<Color>;9], hue_average: &Option<Color>, is_dark: bool) {
    let bg: Lab = to_lab(colors[0].unwrap());
    let fg: Lab = to_lab(colors[1].unwrap());

    let mut count = 0;
    let mut max_l = 0f32;
    let mut min_l = f32::MAX;
    let mut hue_distance: f32 = 0.;
    let mut angles: Vec<f32> = Vec::new();

    for c in &colors[2..] {
        if c.is_none() { continue; }
        let lab = to_lab(c.unwrap());
        let center = interpolate(&bg, &fg, lab.l);
        let (d, th) = length_and_angle(&lab, &center);
        hue_distance += d;
        angles.push(th);
        max_l = max_l.max(lab.l);
        min_l = min_l.min(lab.l);
        count += 1;
    }
    if count == 0 {
        max_l = if is_dark {
            fg.l * 0.8
        } else {
            100. * (1. - 0.8) + fg.l*0.8
        };
        min_l = max_l;
        hue_distance = rng.gen_range(17f32..70f32);
    } else {
        hue_distance /= count as f32;
    }
    let mut new_angles = get_new_angles(rng, &angles);
    for c in colors {
        if c.is_some() { continue; }

        let angle = new_angles.pop().unwrap();
        let l = rng.gen_range(min_l..=max_l);
        let mut lab = interpolate(&bg, &fg, l);
        lab.a += angle.cos() * hue_distance;
        lab.b += angle.sin() * hue_distance;

        *c = Some(from_lab(lab));
    }
}

fn angle_to_distance(a: f32) -> f32 {
    a
}

fn distance_to_angle(a: f32) -> f32 {
    a
}

fn get_new_angles(rng: &mut impl Rng, angles: &Vec<f32>) -> Vec<f32> {
    let remaining = 7 - angles.len();
    let mut rtn = Vec::<f32>::with_capacity(remaining);
    if remaining == 0 { return rtn; }
    let mut valid_distances: Vec<f32> = angles.iter().filter(|x| x.is_finite()).map(|x| angle_to_distance(*x)).collect();
    let module = std::f32::consts::PI * 2.;
    valid_distances.sort_by(|a,b| a.partial_cmp(b).unwrap());
    if valid_distances.len() == 0 {
        let start = rng.gen_range(0f32..module);
        rtn.push(distance_to_angle(start));
        for i in 1..remaining {
            rtn.push(distance_to_angle((start + i as f32 * module / remaining as f32) % module));
        }
        rtn.shuffle(rng);
        return rtn;
    }

    let mut gaps = Vec::<(f32, usize)>::with_capacity(valid_distances.len());
    for (i, d) in valid_distances.iter().enumerate() {
        let d2 = valid_distances[(i+1) % valid_distances.len()];
        gaps.push(((d2 + module - d) % module, 0));
    }
    
    for i in 0..remaining {
        let slot = gaps.iter_mut().max_by(|(gap1, c1), (gap2, c2)|
            (gap1 / (c1 + 1) as f32).partial_cmp(&(gap2 / (c2 + 1) as f32)).unwrap()).unwrap();
        slot.1 += 1;
    }

    for (i, d) in valid_distances.iter().enumerate() {
        let (gap, count) = gaps[i];
        for j in 0..count {
            rtn.push(distance_to_angle((d + j as f32 * gap / (count + 1) as f32) % module));
        }
    }

    rtn.shuffle(rng);
    rtn
}