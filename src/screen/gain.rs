pub const SYNC_GAIN_DEFAULT: f32 = 1.0;
pub const SYNC_GAIN_MIN: f32 = 0.5;
pub const SYNC_GAIN_MAX: f32 = 2.0;

pub fn normalize_sync_gain(gain: f32) -> f32 {
    if !gain.is_finite() {
        return SYNC_GAIN_DEFAULT;
    }
    let rounded = (gain * 10.0).round() / 10.0;
    rounded.clamp(SYNC_GAIN_MIN, SYNC_GAIN_MAX)
}
