#[derive(Clone, Debug)]
pub struct MonitorInfo {
    pub name: String,
    pub description: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Copy, Debug)]
pub struct Scene {
    pub color: crate::bulb::model::Rgb,
    pub brightness: f32,
}
