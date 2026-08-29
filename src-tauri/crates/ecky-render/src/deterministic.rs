//! Deterministic scalar helpers shared by compiler and geometry backends.

pub fn hash01(x: f64, y: f64, seed: f64) -> f64 {
    let raw = (x * 127.1 + y * 311.7 + seed * 74.7).sin() * 43_758.545_312_3;
    fract01(raw)
}

pub fn hash_signed(x: f64, y: f64, seed: f64) -> f64 {
    hash01(x, y, seed) * 2.0 - 1.0
}

pub fn noise2(x: f64, y: f64, seed: f64) -> f64 {
    let x0 = x.floor();
    let y0 = y.floor();
    let xf = x - x0;
    let yf = y - y0;
    let n00 = hash01(x0, y0, seed);
    let n10 = hash01(x0 + 1.0, y0, seed);
    let n01 = hash01(x0, y0 + 1.0, seed);
    let n11 = hash01(x0 + 1.0, y0 + 1.0, seed);
    let sx = smoothstep01(xf);
    let sy = smoothstep01(yf);
    let ix0 = lerp(n00, n10, sx);
    let ix1 = lerp(n01, n11, sx);
    lerp(ix0, ix1, sy).clamp(0.0, 1.0)
}

pub fn fbm2(x: f64, y: f64, seed: f64, octaves: f64, lacunarity: f64, gain: f64) -> f64 {
    let octaves = octaves.round().clamp(1.0, 12.0) as usize;
    let lacunarity = if lacunarity.is_finite() {
        lacunarity.max(0.000_1)
    } else {
        2.0
    };
    let gain = if gain.is_finite() {
        gain.clamp(0.0, 1.0)
    } else {
        0.5
    };
    let mut amplitude = 0.5;
    let mut frequency = 1.0;
    let mut total = 0.0;
    let mut normalizer = 0.0;
    for octave in 0..octaves {
        total += noise2(x * frequency, y * frequency, seed + octave as f64 * 17.0) * amplitude;
        normalizer += amplitude;
        amplitude *= gain;
        frequency *= lacunarity;
    }
    if normalizer <= f64::EPSILON {
        0.0
    } else {
        (total / normalizer).clamp(0.0, 1.0)
    }
}

pub fn cell_distance2(x: f64, y: f64, seed: f64) -> f64 {
    let cx = x.floor();
    let cy = y.floor();
    let mut best = f64::INFINITY;
    for oy in -1..=1 {
        for ox in -1..=1 {
            let gx = cx + ox as f64;
            let gy = cy + oy as f64;
            let px = gx + hash01(gx, gy, seed);
            let py = gy + hash01(gx + 19.19, gy + 7.73, seed + 31.0);
            let dist = ((x - px).powi(2) + (y - py).powi(2)).sqrt();
            best = best.min(dist);
        }
    }
    (best / std::f64::consts::SQRT_2).clamp(0.0, 1.0)
}

pub fn voronoi2(x: f64, y: f64, seed: f64) -> f64 {
    (1.0 - cell_distance2(x, y, seed)).clamp(0.0, 1.0)
}

/// Builds one bounded, uniformly inset Voronoi cell relative to its selected site.
pub fn voronoi_cell_polygon(
    sites: &[[f64; 2]],
    index: usize,
    width: f64,
    height: f64,
    inset: f64,
) -> Result<Vec<[f64; 2]>, String> {
    const EPSILON: f64 = 1.0e-9;

    if sites.is_empty() {
        return Err("voronoi-cell sites must not be empty".into());
    }
    if index >= sites.len() {
        return Err(format!(
            "voronoi-cell index {index} is outside 0..{}",
            sites.len()
        ));
    }
    if !width.is_finite() || !height.is_finite() || width <= 0.0 || height <= 0.0 {
        return Err("voronoi-cell width and height must be finite and positive".into());
    }
    if !inset.is_finite() || inset < 0.0 {
        return Err("voronoi-cell inset must be finite and non-negative".into());
    }
    if inset * 2.0 >= width || inset * 2.0 >= height {
        return Err("voronoi-cell inset collapses bounds".into());
    }
    if sites
        .iter()
        .flatten()
        .any(|coordinate| !coordinate.is_finite())
    {
        return Err("voronoi-cell sites must contain finite coordinates".into());
    }
    for left in 0..sites.len() {
        for right in left + 1..sites.len() {
            let dx = sites[left][0] - sites[right][0];
            let dy = sites[left][1] - sites[right][1];
            if dx.hypot(dy) <= EPSILON {
                return Err(format!(
                    "voronoi-cell duplicate sites at indices {left} and {right}"
                ));
            }
        }
    }

    let half_width = width * 0.5 - inset;
    let half_height = height * 0.5 - inset;
    let selected = sites[index];
    let mut polygon = vec![
        [-half_width, -half_height],
        [half_width, -half_height],
        [half_width, half_height],
        [-half_width, half_height],
    ];

    for (other_index, other) in sites.iter().enumerate() {
        if other_index == index {
            continue;
        }
        let normal = [other[0] - selected[0], other[1] - selected[1]];
        let limit = (other[0] * other[0] + other[1] * other[1]
            - selected[0] * selected[0]
            - selected[1] * selected[1])
            * 0.5
            - inset * normal[0].hypot(normal[1]);
        polygon = clip_polygon_half_plane(&polygon, normal, limit, EPSILON);
        if polygon.len() < 3 {
            return Err(format!("voronoi-cell index {index} collapses after inset"));
        }
    }

    for point in &mut polygon {
        point[0] -= selected[0];
        point[1] -= selected[1];
        for coordinate in point {
            if coordinate.abs() <= EPSILON {
                *coordinate = 0.0;
            }
        }
    }
    polygon = remove_adjacent_duplicate_points(polygon, EPSILON);
    if polygon.len() < 3 || signed_polygon_area(&polygon).abs() <= EPSILON {
        return Err(format!("voronoi-cell index {index} has zero area"));
    }
    if signed_polygon_area(&polygon) < 0.0 {
        polygon.reverse();
    }
    let first = (0..polygon.len())
        .min_by(|left, right| {
            polygon[*left][0]
                .total_cmp(&polygon[*right][0])
                .then_with(|| polygon[*left][1].total_cmp(&polygon[*right][1]))
        })
        .expect("non-empty polygon");
    polygon.rotate_left(first);
    Ok(polygon)
}

fn clip_polygon_half_plane(
    polygon: &[[f64; 2]],
    normal: [f64; 2],
    limit: f64,
    epsilon: f64,
) -> Vec<[f64; 2]> {
    let mut clipped = Vec::new();
    for edge in 0..polygon.len() {
        let start = polygon[edge];
        let end = polygon[(edge + 1) % polygon.len()];
        let start_distance = normal[0] * start[0] + normal[1] * start[1] - limit;
        let end_distance = normal[0] * end[0] + normal[1] * end[1] - limit;
        let start_inside = start_distance <= epsilon;
        let end_inside = end_distance <= epsilon;
        if start_inside {
            clipped.push(start);
        }
        if start_inside != end_inside {
            let denominator = start_distance - end_distance;
            if denominator.abs() > epsilon {
                let t = start_distance / denominator;
                clipped.push([
                    start[0] + (end[0] - start[0]) * t,
                    start[1] + (end[1] - start[1]) * t,
                ]);
            }
        }
    }
    clipped
}

fn remove_adjacent_duplicate_points(mut polygon: Vec<[f64; 2]>, epsilon: f64) -> Vec<[f64; 2]> {
    polygon.dedup_by(|left, right| (left[0] - right[0]).hypot(left[1] - right[1]) <= epsilon);
    if polygon.len() > 1 {
        let last = polygon.len() - 1;
        if (polygon[0][0] - polygon[last][0]).hypot(polygon[0][1] - polygon[last][1]) <= epsilon {
            polygon.pop();
        }
    }
    polygon
}

fn signed_polygon_area(polygon: &[[f64; 2]]) -> f64 {
    polygon
        .iter()
        .zip(polygon.iter().cycle().skip(1))
        .take(polygon.len())
        .map(|(left, right)| left[0] * right[1] - right[0] * left[1])
        .sum::<f64>()
        * 0.5
}

pub fn logistic_scalar2(x: f64, y: f64, seed: f64) -> f64 {
    let mut value = hash01(x, y, seed).clamp(1e-6, 1.0 - 1e-6);
    let rate = 3.57 + hash01(seed, x + y, 11.0) * 0.42;
    for _ in 0..10 {
        value = rate * value * (1.0 - value);
    }
    value.clamp(0.0, 1.0)
}

pub fn henon_scalar2(x: f64, y: f64, seed: f64) -> f64 {
    let mut px = hash_signed(x, y, seed) * 0.65;
    let mut py = hash_signed(y + 17.0, x - 11.0, seed + 3.0) * 0.35;
    let a = 1.22 + hash01(seed, x, 23.0) * 0.18;
    let b = 0.22 + hash01(seed, y, 29.0) * 0.1;
    for _ in 0..8 {
        let next_x = 1.0 - a * px * px + py + (x * 0.017 + y * 0.011).sin() * 0.04;
        let next_y = b * px;
        px = next_x.tanh();
        py = next_y.tanh();
    }
    (px * 0.5 + 0.5).clamp(0.0, 1.0)
}

pub fn ikeda_scalar2(x: f64, y: f64, seed: f64) -> f64 {
    let mut px = hash_signed(x, y, seed) * 0.75;
    let mut py = hash_signed(y, x, seed + 41.0) * 0.75;
    let u = 0.82 + hash01(seed, x - y, 47.0) * 0.09;
    for _ in 0..7 {
        let radius2 = px * px + py * py;
        let t = 0.4 - 6.0 / (1.0 + radius2);
        let next_x = 1.0 + u * (px * t.cos() - py * t.sin()) + (x * 0.013).sin() * 0.03;
        let next_y = u * (px * t.sin() + py * t.cos()) + (y * 0.019).cos() * 0.03;
        px = (next_x * 0.55).tanh();
        py = (next_y * 0.55).tanh();
    }
    ((px + py) * 0.25 + 0.5).clamp(0.0, 1.0)
}

pub fn schwarz_p_scalar(x: f64, y: f64, z: f64) -> f64 {
    ((x.cos() + y.cos() + z.cos()) / 3.0 * 0.5 + 0.5).clamp(0.0, 1.0)
}

pub fn diamond_minimal_scalar(x: f64, y: f64, z: f64) -> f64 {
    let sx = x.sin();
    let sy = y.sin();
    let sz = z.sin();
    let cx = x.cos();
    let cy = y.cos();
    let cz = z.cos();
    let raw = sx * sy * sz + sx * cy * cz + cx * sy * cz + cx * cy * sz;
    (raw / 4.0 * 0.5 + 0.5).clamp(0.0, 1.0)
}

pub fn neovius_scalar(x: f64, y: f64, z: f64) -> f64 {
    let cx = x.cos();
    let cy = y.cos();
    let cz = z.cos();
    let raw = 3.0 * (cx + cy + cz) + 4.0 * cx * cy * cz;
    (raw / 13.0 * 0.5 + 0.5).clamp(0.0, 1.0)
}

pub fn fract01(value: f64) -> f64 {
    let mut result = value.fract();
    if result < 0.0 {
        result += 1.0;
    }
    result.clamp(0.0, 1.0)
}

pub fn smoothstep01(x: f64) -> f64 {
    let t = x.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

pub fn lerp(start: f64, end: f64, t: f64) -> f64 {
    start + (end - start) * t
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seeded_helpers_are_deterministic_and_bounded() {
        assert_eq!(hash01(2.0, 3.0, 4.0), hash01(2.0, 3.0, 4.0));
        assert_ne!(hash01(2.0, 3.0, 4.0), hash01(2.0, 3.0, 5.0));
        for value in [
            hash01(2.0, 3.0, 4.0),
            noise2(0.2, 0.8, 4.0),
            fbm2(0.2, 0.8, 4.0, 4.0, 2.0, 0.5),
            voronoi2(0.2, 0.8, 4.0),
            cell_distance2(0.2, 0.8, 4.0),
            logistic_scalar2(0.2, 0.8, 4.0),
            henon_scalar2(0.2, 0.8, 4.0),
            ikeda_scalar2(0.2, 0.8, 4.0),
            schwarz_p_scalar(0.2, 0.8, 4.0),
            diamond_minimal_scalar(0.2, 0.8, 4.0),
            neovius_scalar(0.2, 0.8, 4.0),
        ] {
            assert!((0.0..=1.0).contains(&value), "{value}");
        }
    }

    #[test]
    fn voronoi_cell_polygon_clips_and_insets_exactly() {
        let sites = [[-10.0, 0.0], [10.0, 0.0]];
        let cell = voronoi_cell_polygon(&sites, 0, 40.0, 20.0, 2.0).expect("left cell");

        assert_eq!(cell.len(), 4);
        assert!(cell.iter().all(|point| point[0] >= -8.000_001));
        assert!(cell.iter().all(|point| point[0] <= 8.000_001));
        assert!(cell.iter().all(|point| point[1].abs() <= 8.000_001));
        assert_eq!(
            cell,
            voronoi_cell_polygon(&sites, 0, 40.0, 20.0, 2.0).expect("stable cell")
        );
    }

    #[test]
    fn voronoi_cell_polygon_rejects_duplicates_and_invalid_index() {
        let duplicate = [[0.0, 0.0], [0.0, 0.0]];
        assert!(voronoi_cell_polygon(&duplicate, 0, 20.0, 20.0, 1.0)
            .expect_err("duplicates fail")
            .contains("duplicate"));
        assert!(voronoi_cell_polygon(&[[0.0, 0.0]], 1, 20.0, 20.0, 1.0)
            .expect_err("index fails")
            .contains("index"));
        assert!(voronoi_cell_polygon(&[[0.0, 0.0]], 0, 20.0, 20.0, 10.0)
            .expect_err("collapsed bounds fail")
            .contains("collapses"));
    }

    #[test]
    fn voronoi_cell_polygon_has_stable_ccw_square_grid_order() {
        let sites = [[-5.0, -5.0], [5.0, -5.0], [-5.0, 5.0], [5.0, 5.0]];
        let cell = voronoi_cell_polygon(&sites, 0, 20.0, 20.0, 1.0).expect("corner cell");

        assert_eq!(cell[0], [-4.0, -4.0]);
        assert!(signed_polygon_area(&cell) > 0.0);
        assert_eq!(
            cell,
            vec![[-4.0, -4.0], [4.0, -4.0], [4.0, 4.0], [-4.0, 4.0]]
        );
    }
}
