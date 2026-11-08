use crate::bulb::model::Rgb;
use crate::screen::color::{Lab, delta_e_ok, lab_from_oklch, oklch_from_lab, oklch_to_rgb};
use crate::screen::grid::{PreparedFrame, percentile};
use crate::screen::kmeans::cluster;

const CHROMA_QUANTILE: f32 = 0.75;
const CLUSTER_DROP: f32 = 0.03;
const SCORE_CHROMA_SCALE: f32 = 0.15;
const DEADBAND: f32 = 0.02;

#[derive(Clone, Debug, Default)]
pub struct ReducerState {
    pub centroids: Option<Vec<Lab>>,
}

#[derive(Clone, Copy, Debug)]
pub struct DerivedColor {
    pub rgb: Rgb,
    pub lab: Lab,
    pub scene_brightness: f32,
}

#[derive(Clone, Debug)]
pub struct Reduction {
    pub derived: DerivedColor,
    pub state: ReducerState,
}

pub fn reduce_prepared(prepared: &PreparedFrame, state: &ReducerState) -> Reduction {
    let clustering = cluster(
        &prepared.lab,
        prepared.pixel_count,
        state.centroids.as_deref(),
    );
    let target = choose_target(
        prepared,
        &clustering.assignments,
        clustering.centroids.len(),
    );
    let (l, chroma, hue) = oklch_from_lab(target);
    let rgb = oklch_to_rgb(l, chroma, hue);

    Reduction {
        derived: DerivedColor {
            rgb,
            lab: target,
            scene_brightness: prepared.mean_drive,
        },
        state: ReducerState {
            centroids: Some(clustering.centroids),
        },
    }
}

fn choose_target(prepared: &PreparedFrame, assignments: &[i32], centroid_count: usize) -> Lab {
    let mut counts = vec![0usize; centroid_count];
    let mut chroma_sums = vec![0.0f64; centroid_count];
    for cursor in 0..prepared.pixel_count {
        let cluster = assignments.get(cursor).copied().unwrap_or(0) as usize;
        if cluster >= centroid_count {
            continue;
        }
        if let Some(count) = counts.get_mut(cluster) {
            *count += 1;
        }
        if let Some(sum) = chroma_sums.get_mut(cluster) {
            *sum += f64::from(prepared.chroma.get(cursor).copied().unwrap_or(0.0));
        }
    }

    let winner = pick_winner(&counts, &chroma_sums, prepared.pixel_count);
    let mut chromas = Vec::with_capacity(counts.get(winner.index).copied().unwrap_or(0));
    let mut sum_l = 0.0f64;
    let mut sum_a = 0.0f64;
    let mut sum_b = 0.0f64;

    for cursor in 0..prepared.pixel_count {
        if assignments.get(cursor).copied().unwrap_or(0) as usize != winner.index {
            continue;
        }
        sum_l += f64::from(prepared.lab.get(cursor * 3).copied().unwrap_or(0.0));
        sum_a += f64::from(prepared.lab.get(cursor * 3 + 1).copied().unwrap_or(0.0));
        sum_b += f64::from(prepared.lab.get(cursor * 3 + 2).copied().unwrap_or(0.0));
        chromas.push(prepared.chroma.get(cursor).copied().unwrap_or(0.0));
    }

    let size = chromas.len().max(1) as f64;
    let scale = if winner.chroma > 0.0 {
        percentile(&chromas, CHROMA_QUANTILE) / winner.chroma
    } else {
        0.0
    };
    Lab {
        l: (sum_l / size) as f32,
        a: (sum_a / size) as f32 * scale,
        b: (sum_b / size) as f32 * scale,
    }
}

struct Winner {
    index: usize,
    chroma: f32,
}

fn pick_winner(counts: &[usize], chroma_sums: &[f64], pixel_count: usize) -> Winner {
    let mut winner = None;
    let mut winner_score = -1.0f32;
    let mut largest = 0usize;
    let mut largest_population = -1.0f32;

    for (index, count) in counts.iter().enumerate() {
        let population = *count as f32 / pixel_count.max(1) as f32;
        if population > largest_population {
            largest_population = population;
            largest = index;
        }
        if population < CLUSTER_DROP {
            continue;
        }
        let sum = chroma_sums.get(index).copied().unwrap_or(0.0);
        let chroma = (sum / (*count).max(1) as f64) as f32;
        let score = population.powf(0.8) * (0.2 + 0.8 * (chroma / SCORE_CHROMA_SCALE).min(1.0));
        if score > winner_score {
            winner_score = score;
            winner = Some(index);
        }
    }

    let index = winner.unwrap_or(largest);
    let sum = chroma_sums.get(index).copied().unwrap_or(0.0);
    let count = counts.get(index).copied().unwrap_or(1).max(1) as f64;
    Winner {
        index,
        chroma: (sum / count) as f32,
    }
}

pub fn should_publish(next: DerivedColor, last: Option<Lab>) -> bool {
    match last {
        Some(last) => delta_e_ok(next.lab, last) > DEADBAND,
        None => true,
    }
}

pub fn lab_for_color(rgb: Rgb) -> Lab {
    let (l, chroma, hue) = crate::screen::color::rgb_to_oklch(rgb);
    lab_from_oklch(l, chroma, hue)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::screen::grid::prepare_frame;

    #[test]
    fn a_solid_red_frame_derives_red() {
        let (width, height) = (64, 36);
        let mut pixels = vec![0u8; width * height * 4];
        for pixel in pixels.as_chunks_mut::<4>().0 {
            *pixel = [255, 0, 0, 255];
        }
        let prepared = prepare_frame(width, height, &pixels);
        let reduction = reduce_prepared(&prepared, &ReducerState::default());
        assert!(reduction.derived.rgb.red > 200);
        assert!(reduction.derived.rgb.green < 40);
        assert!(reduction.derived.rgb.blue < 40);
    }
}
