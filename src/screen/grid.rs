use crate::bulb::model::Rgb;
use crate::screen::color::{lab_from_oklch, rgb_to_oklch};

const DARK_THRESHOLD: u8 = 8;
const BAR_COVERAGE: f32 = 0.97;
const MINIMUM_SURVIVAL: f32 = 0.25;

#[derive(Clone, Copy, Debug, Default)]
pub struct ContentRect {
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
}

#[derive(Clone, Debug)]
pub struct PreparedFrame {
    pub pixel_count: usize,
    pub lab: Vec<f32>,
    pub chroma: Vec<f32>,
    pub mean_drive: f32,
}

pub fn prepare_frame(width: usize, height: usize, pixels: &[u8]) -> PreparedFrame {
    let rect = detect_content_rect(width, height, pixels);
    let pixel_count = rect.width * rect.height;
    let mut lab = Vec::with_capacity(pixel_count * 3);
    let mut chroma = Vec::with_capacity(pixel_count);

    let mut sum_drive = 0.0;
    for y in rect.y..rect.y + rect.height {
        for x in rect.x..rect.x + rect.width {
            let offset = (y * width + x) * 4;
            let red = pixels.get(offset).copied().unwrap_or(0);
            let green = pixels.get(offset + 1).copied().unwrap_or(0);
            let blue = pixels.get(offset + 2).copied().unwrap_or(0);
            let (l, c, hue) = rgb_to_oklch(Rgb::new(red, green, blue));
            let converted = lab_from_oklch(l, c, hue);
            lab.push(converted.l);
            lab.push(converted.a);
            lab.push(converted.b);
            chroma.push(c);
            sum_drive += f32::from(red.max(green).max(blue)) / 255.0;
        }
    }

    PreparedFrame {
        pixel_count,
        lab,
        chroma,
        mean_drive: sum_drive / pixel_count.max(1) as f32,
    }
}

fn detect_content_rect(width: usize, height: usize, pixels: &[u8]) -> ContentRect {
    let mut top = 0usize;
    let mut bottom = height.saturating_sub(1);
    let mut left = 0usize;
    let mut right = width.saturating_sub(1);

    while top < height && dark_fraction(pixels, width, height, Axis::Row, top) >= BAR_COVERAGE {
        top += 1;
    }
    while bottom >= top && dark_fraction(pixels, width, height, Axis::Row, bottom) >= BAR_COVERAGE {
        if bottom == 0 {
            break;
        }
        bottom -= 1;
    }
    while left < width && dark_fraction(pixels, width, height, Axis::Column, left) >= BAR_COVERAGE {
        left += 1;
    }
    while right >= left && dark_fraction(pixels, width, height, Axis::Column, right) >= BAR_COVERAGE
    {
        if right == 0 {
            break;
        }
        right -= 1;
    }

    let candidate_width = right.saturating_sub(left) + 1;
    let candidate_height = bottom.saturating_sub(top) + 1;
    let candidate_area = (candidate_width * candidate_height) as f32;
    let total_area = (width * height) as f32;
    if candidate_width == 0
        || candidate_height == 0
        || candidate_area < total_area * MINIMUM_SURVIVAL
    {
        return ContentRect {
            x: 0,
            y: 0,
            width,
            height,
        };
    }

    ContentRect {
        x: left,
        y: top,
        width: candidate_width,
        height: candidate_height,
    }
}

#[derive(Clone, Copy)]
enum Axis {
    Row,
    Column,
}

fn dark_fraction(pixels: &[u8], width: usize, height: usize, axis: Axis, index: usize) -> f32 {
    let count = match axis {
        Axis::Row => width,
        Axis::Column => height,
    };
    let mut dark = 0usize;
    for cursor in 0..count {
        let (x, y) = match axis {
            Axis::Row => (cursor, index),
            Axis::Column => (index, cursor),
        };
        if y >= height || x >= width {
            continue;
        }
        let offset = (y * width + x) * 4;
        if pixels.get(offset).copied().unwrap_or(0) <= DARK_THRESHOLD
            && pixels.get(offset + 1).copied().unwrap_or(0) <= DARK_THRESHOLD
            && pixels.get(offset + 2).copied().unwrap_or(0) <= DARK_THRESHOLD
        {
            dark += 1;
        }
    }
    dark as f32 / count.max(1) as f32
}

pub fn percentile(values: &[f32], fraction: f32) -> f32 {
    if values.is_empty() {
        return 0.0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(|left, right| left.total_cmp(right));
    let index = ((sorted.len() - 1) as f32 * fraction.clamp(0.0, 1.0)).floor() as usize;
    sorted
        .get(index.min(sorted.len() - 1))
        .copied()
        .unwrap_or(0.0)
}
