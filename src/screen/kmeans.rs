use crate::screen::color::Lab;

const CLUSTERS: usize = 5;
const MAX_ITERATIONS: usize = 14;
const CONVERGENCE: f32 = 0.000_01;
const SEED: u32 = 2_654_435_761;
const MULTIPLIER: u32 = 1_664_525;
const INCREMENT: u32 = 1_013_904_223;
const MODULUS: f32 = 4_294_967_296.0;

#[derive(Clone, Debug)]
pub struct Clustering {
    pub centroids: Vec<Lab>,
    pub assignments: Vec<i32>,
}

pub fn cluster(lab: &[f32], pixel_count: usize, previous: Option<&[Lab]>) -> Clustering {
    if let Some(previous) = previous
        && previous.len() == CLUSTERS
    {
        return iterate(lab, pixel_count, previous.to_vec());
    }
    let mut random = make_random(SEED);
    iterate(
        lab,
        pixel_count,
        init_centroids(lab, pixel_count, &mut random),
    )
}

fn make_random(seed: u32) -> impl FnMut() -> f32 {
    let mut state = seed;
    move || {
        state = state.wrapping_mul(MULTIPLIER).wrapping_add(INCREMENT);
        state as f32 / MODULUS
    }
}

fn lab_at(lab: &[f32], index: usize) -> Lab {
    Lab {
        l: lab.get(index * 3).copied().unwrap_or(0.0),
        a: lab.get(index * 3 + 1).copied().unwrap_or(0.0),
        b: lab.get(index * 3 + 2).copied().unwrap_or(0.0),
    }
}

fn distance_squared(lab: &[f32], index: usize, centroid: Lab) -> f32 {
    let dl = lab.get(index * 3).copied().unwrap_or(0.0) - centroid.l;
    let da = lab.get(index * 3 + 1).copied().unwrap_or(0.0) - centroid.a;
    let db = lab.get(index * 3 + 2).copied().unwrap_or(0.0) - centroid.b;
    dl * dl + da * da + db * db
}

fn init_centroids(lab: &[f32], pixel_count: usize, random: &mut impl FnMut() -> f32) -> Vec<Lab> {
    let first_index = ((random() * pixel_count as f32) as usize).min(pixel_count.saturating_sub(1));
    let first = lab_at(lab, first_index);
    let mut centroids = vec![first];
    let mut distances = vec![0.0f32; pixel_count];
    for (cursor, distance) in distances.iter_mut().enumerate() {
        *distance = distance_squared(lab, cursor, first);
    }

    while centroids.len() < CLUSTERS {
        let total: f32 = distances.iter().sum();
        if total <= 0.0 {
            let index =
                ((random() * pixel_count as f32) as usize).min(pixel_count.saturating_sub(1));
            centroids.push(lab_at(lab, index));
            continue;
        }
        let mut target = random() * total;
        let mut chosen = pixel_count.saturating_sub(1);
        for (cursor, distance) in distances.iter().enumerate() {
            target -= *distance;
            if target <= 0.0 {
                chosen = cursor;
                break;
            }
        }
        let centroid = lab_at(lab, chosen);
        centroids.push(centroid);
        for (cursor, distance) in distances.iter_mut().enumerate() {
            let next = distance_squared(lab, cursor, centroid);
            if next < *distance {
                *distance = next;
            }
        }
    }

    centroids
}

fn iterate(lab: &[f32], pixel_count: usize, initial: Vec<Lab>) -> Clustering {
    let mut assignments = vec![0i32; pixel_count];
    let mut current = initial;

    for _ in 0..MAX_ITERATIONS {
        let mut sum_l = vec![0.0f64; current.len()];
        let mut sum_a = vec![0.0f64; current.len()];
        let mut sum_b = vec![0.0f64; current.len()];
        let mut count = vec![0usize; current.len()];

        for (cursor, assignment) in assignments.iter_mut().enumerate() {
            let mut best = 0usize;
            let mut best_distance = f32::INFINITY;
            for (cluster, centroid) in current.iter().enumerate() {
                let distance = distance_squared(lab, cursor, *centroid);
                if distance < best_distance {
                    best_distance = distance;
                    best = cluster;
                }
            }
            *assignment = best as i32;
            if let Some(slot) = sum_l.get_mut(best) {
                *slot += f64::from(lab.get(cursor * 3).copied().unwrap_or(0.0));
            }
            if let Some(slot) = sum_a.get_mut(best) {
                *slot += f64::from(lab.get(cursor * 3 + 1).copied().unwrap_or(0.0));
            }
            if let Some(slot) = sum_b.get_mut(best) {
                *slot += f64::from(lab.get(cursor * 3 + 2).copied().unwrap_or(0.0));
            }
            if let Some(slot) = count.get_mut(best) {
                *slot += 1;
            }
        }

        let mut shift = 0.0f32;
        let next: Vec<Lab> = current
            .iter()
            .enumerate()
            .map(|(cluster, centroid)| {
                let size = count.get(cluster).copied().unwrap_or(0);
                if size == 0 {
                    return *centroid;
                }
                let updated = Lab {
                    l: (sum_l.get(cluster).copied().unwrap_or(0.0) / size as f64) as f32,
                    a: (sum_a.get(cluster).copied().unwrap_or(0.0) / size as f64) as f32,
                    b: (sum_b.get(cluster).copied().unwrap_or(0.0) / size as f64) as f32,
                };
                let dl = updated.l - centroid.l;
                let da = updated.a - centroid.a;
                let db = updated.b - centroid.b;
                shift = shift.max((dl * dl + da * da + db * db).sqrt());
                updated
            })
            .collect();

        current = next;
        if shift < CONVERGENCE {
            break;
        }
    }

    Clustering {
        centroids: current,
        assignments,
    }
}
