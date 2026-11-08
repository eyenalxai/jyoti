pub const SYNC_GAIN_DEFAULT: f32 = 1.0;
pub const SYNC_GAIN_MIN: f32 = 0.5;
pub const SYNC_GAIN_MAX: f32 = 4.0;

pub fn normalize_sync_gain(gain: f32) -> f32 {
    if !gain.is_finite() {
        return SYNC_GAIN_DEFAULT;
    }
    let rounded = (gain * 10.0).round() / 10.0;
    rounded.clamp(SYNC_GAIN_MIN, SYNC_GAIN_MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_gain_envelope_accepts_four_times_brightness() {
        assert_eq!(normalize_sync_gain(9.0), 4.0);
        assert_eq!(normalize_sync_gain(0.1), 0.5);
    }
}
