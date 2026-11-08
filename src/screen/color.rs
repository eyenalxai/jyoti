use palette::{FromColor, Hsv, IntoColor, Oklch, RgbHue, Srgb};

use crate::bulb::model::Rgb;

#[derive(Clone, Copy, Debug, Default)]
pub struct Lab {
    pub l: f32,
    pub a: f32,
    pub b: f32,
}

pub fn lab_from_oklch(l: f32, chroma: f32, hue: f32) -> Lab {
    let radians = hue.to_radians();
    Lab {
        l,
        a: chroma * radians.cos(),
        b: chroma * radians.sin(),
    }
}

pub fn oklch_from_lab(lab: Lab) -> (f32, f32, f32) {
    let chroma = (lab.a * lab.a + lab.b * lab.b).sqrt();
    let hue = if chroma < 0.0001 {
        0.0
    } else {
        lab.b.atan2(lab.a).to_degrees().rem_euclid(360.0)
    };
    (lab.l, chroma, hue)
}

pub fn delta_e_ok(left: Lab, right: Lab) -> f32 {
    ((left.l - right.l).powi(2) + (left.a - right.a).powi(2) + (left.b - right.b).powi(2)).sqrt()
}

pub fn rgb_to_oklch(rgb: Rgb) -> (f32, f32, f32) {
    let srgb: Srgb = Srgb::new(
        f32::from(rgb.red) / 255.0,
        f32::from(rgb.green) / 255.0,
        f32::from(rgb.blue) / 255.0,
    );
    let oklch: Oklch = srgb.into_color();
    (oklch.l, oklch.chroma, oklch.hue.into_degrees())
}

pub fn oklch_to_rgb(l: f32, chroma: f32, hue: f32) -> Rgb {
    let (l, chroma, hue) = fit_to_gamut(l, chroma, hue);
    let srgb: Srgb = Srgb::from_color(Oklch::new(l, chroma, hue));
    Rgb::new(
        round_channel(srgb.red * 255.0),
        round_channel(srgb.green * 255.0),
        round_channel(srgb.blue * 255.0),
    )
}

pub fn kelvin_to_rgb(kelvin: u16) -> Rgb {
    let temp = f32::from(kelvin) / 100.0;
    let red = if temp <= 66.0 {
        255.0
    } else {
        329.698_73 * (temp - 60.0).powf(-0.133_204_76)
    };
    let green = if temp <= 66.0 {
        99.470_8 * temp.ln() - 161.119_57
    } else {
        288.122_16 * (temp - 60.0).powf(-0.075_514_85)
    };
    let blue = if temp >= 66.0 {
        255.0
    } else if temp <= 19.0 {
        0.0
    } else {
        138.517_73 * (temp - 10.0).ln() - 305.044_8
    };
    Rgb::new(
        round_channel(red),
        round_channel(green),
        round_channel(blue),
    )
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Hsb {
    pub hue: f32,
    pub saturation: f32,
    pub value: f32,
}

pub fn rgb_to_hsb(rgb: Rgb) -> Hsb {
    let srgb: Srgb = Srgb::new(
        f32::from(rgb.red) / 255.0,
        f32::from(rgb.green) / 255.0,
        f32::from(rgb.blue) / 255.0,
    );
    let hsv: Hsv = srgb.into_color();
    Hsb {
        hue: hsv.hue.into_degrees(),
        saturation: hsv.saturation * 100.0,
        value: hsv.value * 100.0,
    }
}

pub fn hsb_to_rgb(hsb: Hsb) -> Rgb {
    let hsv = Hsv::new(
        RgbHue::from_degrees(hsb.hue.rem_euclid(360.0)),
        (hsb.saturation / 100.0).clamp(0.0, 1.0),
        (hsb.value / 100.0).clamp(0.0, 1.0),
    );
    let srgb: Srgb = Srgb::from_color(hsv);
    Rgb::new(
        round_channel(srgb.red * 255.0),
        round_channel(srgb.green * 255.0),
        round_channel(srgb.blue * 255.0),
    )
}

fn fit_to_gamut(l: f32, chroma: f32, hue: f32) -> (f32, f32, f32) {
    let l = l.clamp(0.0, 1.0);
    let hue = hue.clamp(0.0, 360.0);
    let mut chroma = chroma.clamp(0.0, 0.37);
    if in_gamut(l, chroma, hue) {
        return (l, chroma, hue);
    }

    let mut low = 0.0;
    let mut high = chroma;
    for _ in 0..18 {
        let middle = (low + high) / 2.0;
        if in_gamut(l, middle, hue) {
            low = middle;
        } else {
            high = middle;
        }
    }
    chroma = low;
    (l, chroma, hue)
}

fn in_gamut(l: f32, chroma: f32, hue: f32) -> bool {
    const EPSILON: f32 = 0.000_01;
    let srgb: Srgb = Srgb::from_color(Oklch::new(l, chroma, hue));
    srgb.red >= -EPSILON
        && srgb.red <= 1.0 + EPSILON
        && srgb.green >= -EPSILON
        && srgb.green <= 1.0 + EPSILON
        && srgb.blue >= -EPSILON
        && srgb.blue <= 1.0 + EPSILON
}

fn round_channel(value: f32) -> u8 {
    value.round().clamp(0.0, 255.0) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hsb_round_trips_an_encoded_colour() {
        let color = Rgb::new(255, 96, 96);
        assert_eq!(hsb_to_rgb(rgb_to_hsb(color)), color);
    }

    #[test]
    fn oklch_round_trips_an_encoded_colour() {
        let color = Rgb::new(128, 128, 128);
        let (l, chroma, hue) = rgb_to_oklch(color);
        assert_eq!(oklch_to_rgb(l, chroma, hue), color);
    }
}
