use crate::bulb::model::Rgb;
use crate::screen::color::{Hsb, hsb_to_rgb, rgb_to_hsb};

pub const SYNC_INTENSITY_DEFAULT: f32 = 0.0;
pub const SYNC_INTENSITY_MAX: f32 = 100.0;

pub fn normalize_sync_intensity(value: f32) -> f32 {
    if !value.is_finite() {
        return SYNC_INTENSITY_DEFAULT;
    }
    value.round().clamp(0.0, SYNC_INTENSITY_MAX)
}

pub fn intensify(color: Rgb, intensity: f32) -> Rgb {
    let intensity = normalize_sync_intensity(intensity);
    if intensity == SYNC_INTENSITY_DEFAULT {
        return color;
    }
    let hsb = rgb_to_hsb(color);
    if hsb.saturation <= 0.5 {
        return color;
    }
    let saturation = hsb.saturation + (100.0 - hsb.saturation) * (intensity / 100.0);
    hsb_to_rgb(Hsb { saturation, ..hsb })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intensity_is_a_percentage() {
        assert_eq!(normalize_sync_intensity(140.0), 100.0);
        assert_eq!(normalize_sync_intensity(-10.0), 0.0);
        assert_eq!(normalize_sync_intensity(f32::NAN), 0.0);
    }

    #[test]
    fn zero_keeps_the_captured_colour() {
        let color = Rgb::new(180, 120, 120);
        assert_eq!(intensify(color, 0.0), color);
    }

    #[test]
    fn full_intensity_reaches_full_saturation_without_changing_hue_or_value() {
        let color = Rgb::new(180, 120, 120);
        let before = rgb_to_hsb(color);
        let after = rgb_to_hsb(intensify(color, 100.0));
        assert!((after.saturation - 100.0).abs() < 0.1);
        assert!((after.hue - before.hue).abs() < 0.1);
        assert!((after.value - before.value).abs() < 0.1);
    }

    #[test]
    fn achromatic_colours_never_gain_a_hue() {
        let grey = Rgb::new(180, 180, 180);
        assert_eq!(intensify(grey, 100.0), grey);
        assert_eq!(intensify(Rgb::WHITE, 100.0), Rgb::WHITE);
    }
}
