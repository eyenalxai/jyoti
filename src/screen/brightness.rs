const SYNC_BRIGHTNESS_FLOOR: f32 = 8.0;
const SYNC_BRIGHTNESS_CEILING: f32 = 100.0;
const SYNC_BRIGHTNESS_SPAN: f32 = SYNC_BRIGHTNESS_CEILING - SYNC_BRIGHTNESS_FLOOR;

pub fn sync_brightness_for(scene_brightness: f32, gain: f32) -> u8 {
    let envelope = (SYNC_BRIGHTNESS_FLOOR
        + SYNC_BRIGHTNESS_SPAN * scene_brightness.clamp(0.0, 1.0))
    .clamp(SYNC_BRIGHTNESS_FLOOR, SYNC_BRIGHTNESS_CEILING)
    .round();
    (envelope * gain)
        .clamp(SYNC_BRIGHTNESS_FLOOR, SYNC_BRIGHTNESS_CEILING)
        .round() as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_envelope_maps_the_scene_to_the_bulb_range() {
        assert_eq!(sync_brightness_for(0.0, 1.0), 8);
        assert_eq!(sync_brightness_for(1.0, 1.0), 100);
        assert_eq!(sync_brightness_for(0.5, 1.0), 54);
        assert_eq!(sync_brightness_for(0.5, 2.0), 100);
        assert_eq!(sync_brightness_for(1.0, 0.5), 50);
    }
}
