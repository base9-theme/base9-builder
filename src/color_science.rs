use itertools::Itertools;
use ext_palette::{Srgb, Xyz, Lab, convert::IntoColorUnclamped, IntoColor, Lch};

pub type Rgb = Srgb<u8>;

pub fn mix1d(a: f32, b: f32, w: f32) -> f32 {
    return a*(1.-w)+b*w;
}

pub fn mix(c1: &Rgb, c2: &Rgb, w: f32) -> Rgb {
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

pub fn closest_order<'a, 'b>(colors: &'a[Rgb], target: &'b [Rgb]) -> Vec<&'a Rgb> {
        let average = {
          let mut mut_average: (f32, f32, f32) = (0.,0.,0.);
            for c in colors {
                let lab: Lab = c.into_format().into_color();
                mut_average.0 += lab.l;
                mut_average.1 += lab.a;
                mut_average.2 += lab.b;
            }
            mut_average.0 /= colors.len() as f32;
            mut_average.1 /= colors.len() as f32;
            mut_average.2 /= colors.len() as f32;
            mut_average
        };

        let result = colors.into_iter().permutations(target.len()).min_by_key(|perm| {
            let mut sum: f32 = 0.;
            for (ca, cr) in target.iter().zip_eq(perm) {
                let mut ca: Lab = ca.into_format().into_color();
                let mut cr: Lab = cr.into_format().into_color();
                cr.a-=average.1;
                cr.b-=average.2;
                let ca: Lch = ca.into_color();
                let cr: Lch = cr.into_color();
                let ds = ((ca.l - cr.l), (ca.chroma - cr.chroma), (ca.hue - cr.hue).to_degrees());
                let weight = (0., 1., 6.);
                let tmp_sum = (ds.0 * ds.0 * weight.0 + ds.1*ds.1 * weight.1 + ds.2*ds.2*weight.2).sqrt();
                sum += tmp_sum;
            }

            (sum * 1000.) as i64
        }).unwrap();
        
        result.iter().map(|x| *x).collect()
}