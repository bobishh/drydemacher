use std::io::Write;
use std::path::Path;

use crate::contracts::{AppError, AppResult};
use crate::ecky_ir::mesh_asset::{IndexedMeshAsset, MeshAssetSource};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MeshCropBounds {
    pub min: [f64; 3],
    pub max: [f64; 3],
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeshCropReport {
    pub input_triangle_count: usize,
    pub output_triangle_count: usize,
}

pub fn clip_triangles_to_box(
    triangles: &[[[f64; 3]; 3]],
    bounds: MeshCropBounds,
) -> AppResult<Vec<[[f64; 3]; 3]>> {
    validate_bounds(bounds)?;
    let mut output = Vec::new();
    for triangle in triangles {
        let mut polygon = triangle.to_vec();
        for axis in 0..3 {
            polygon = clip_polygon(polygon, axis, bounds.min[axis], true);
            polygon = clip_polygon(polygon, axis, bounds.max[axis], false);
            if polygon.len() < 3 {
                break;
            }
        }
        if polygon.len() < 3 {
            continue;
        }
        for index in 1..polygon.len() - 1 {
            let clipped = [polygon[0], polygon[index], polygon[index + 1]];
            if triangle_area(clipped) > 1.0e-12 {
                output.push(clipped);
            }
        }
    }
    if output.is_empty() {
        return Err(AppError::validation(
            "Box crop excludes the complete capture mesh.",
        ));
    }
    Ok(output)
}

pub fn write_capture_box_crop(
    source: &Path,
    output: &Path,
    bounds: MeshCropBounds,
) -> AppResult<MeshCropReport> {
    let mesh = IndexedMeshAsset::from_stl(MeshAssetSource::Imported, source)?;
    let triangles = mesh
        .triangles()
        .iter()
        .map(|triangle| triangle.map(|index| mesh.vertices()[index as usize]))
        .collect::<Vec<_>>();
    let cropped = clip_triangles_to_box(&triangles, bounds)?;
    let temporary = output.with_extension("stl.tmp");
    write_binary_stl(&temporary, &cropped)?;
    std::fs::rename(&temporary, output).map_err(|error| {
        AppError::persistence(format!("Failed to publish cropped capture STL: {error}"))
    })?;
    Ok(MeshCropReport {
        input_triangle_count: triangles.len(),
        output_triangle_count: cropped.len(),
    })
}

fn validate_bounds(bounds: MeshCropBounds) -> AppResult<()> {
    for axis in 0..3 {
        if !bounds.min[axis].is_finite()
            || !bounds.max[axis].is_finite()
            || bounds.min[axis] >= bounds.max[axis]
        {
            return Err(AppError::validation(format!(
                "Box crop axis {axis} requires finite min smaller than max.",
            )));
        }
    }
    Ok(())
}

fn clip_polygon(
    polygon: Vec<[f64; 3]>,
    axis: usize,
    limit: f64,
    keep_greater: bool,
) -> Vec<[f64; 3]> {
    if polygon.is_empty() {
        return polygon;
    }
    let inside = |point: [f64; 3]| {
        if keep_greater {
            point[axis] >= limit
        } else {
            point[axis] <= limit
        }
    };
    let mut output = Vec::new();
    let mut previous = *polygon.last().unwrap();
    let mut previous_inside = inside(previous);
    for current in polygon {
        let current_inside = inside(current);
        if current_inside != previous_inside {
            let denominator = current[axis] - previous[axis];
            if denominator.abs() > f64::EPSILON {
                let t = (limit - previous[axis]) / denominator;
                output.push([
                    previous[0] + (current[0] - previous[0]) * t,
                    previous[1] + (current[1] - previous[1]) * t,
                    previous[2] + (current[2] - previous[2]) * t,
                ]);
            }
        }
        if current_inside {
            output.push(current);
        }
        previous = current;
        previous_inside = current_inside;
    }
    output
}

fn triangle_area(triangle: [[f64; 3]; 3]) -> f64 {
    let ab = subtract(triangle[1], triangle[0]);
    let ac = subtract(triangle[2], triangle[0]);
    let normal = cross(ab, ac);
    dot(normal, normal).sqrt() * 0.5
}

fn write_binary_stl(path: &Path, triangles: &[[[f64; 3]; 3]]) -> AppResult<()> {
    let mut output = std::fs::File::create(path).map_err(|error| {
        AppError::persistence(format!("Failed to create cropped capture STL: {error}"))
    })?;
    output
        .write_all(&[0; 80])
        .map_err(|error| AppError::persistence(error.to_string()))?;
    let count = u32::try_from(triangles.len())
        .map_err(|_| AppError::validation("Cropped capture STL exceeds triangle count limit."))?;
    output
        .write_all(&count.to_le_bytes())
        .map_err(|error| AppError::persistence(error.to_string()))?;
    for triangle in triangles {
        let raw_normal = cross(
            subtract(triangle[1], triangle[0]),
            subtract(triangle[2], triangle[0]),
        );
        let length = dot(raw_normal, raw_normal).sqrt();
        let normal = if length > f64::EPSILON {
            raw_normal.map(|value| value / length)
        } else {
            [0.0; 3]
        };
        for vertex in [normal, triangle[0], triangle[1], triangle[2]] {
            for coordinate in vertex {
                output
                    .write_all(&(coordinate as f32).to_le_bytes())
                    .map_err(|error| AppError::persistence(error.to_string()))?;
            }
        }
        output
            .write_all(&0_u16.to_le_bytes())
            .map_err(|error| AppError::persistence(error.to_string()))?;
    }
    Ok(())
}

fn subtract(left: [f64; 3], right: [f64; 3]) -> [f64; 3] {
    [left[0] - right[0], left[1] - right[1], left[2] - right[2]]
}

fn dot(left: [f64; 3], right: [f64; 3]) -> f64 {
    left[0] * right[0] + left[1] * right[1] + left[2] * right[2]
}

fn cross(left: [f64; 3], right: [f64; 3]) -> [f64; 3] {
    [
        left[1] * right[2] - left[2] * right[1],
        left[2] * right[0] - left[0] * right[2],
        left[0] * right[1] - left[1] * right[0],
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn box_crop_keeps_inside_geometry_and_clips_crossing_triangles() {
        let triangles = vec![
            [[-5.0, -5.0, 0.0], [5.0, -5.0, 0.0], [0.0, 5.0, 0.0]],
            [[20.0, 20.0, 20.0], [21.0, 20.0, 20.0], [20.0, 21.0, 20.0]],
        ];
        let bounds = MeshCropBounds {
            min: [-1.0, -1.0, -1.0],
            max: [1.0, 1.0, 1.0],
        };

        let cropped = clip_triangles_to_box(&triangles, bounds).unwrap();

        assert!(!cropped.is_empty());
        assert!(cropped.iter().flatten().all(|vertex| (0..3)
            .all(|axis| vertex[axis] >= bounds.min[axis] && vertex[axis] <= bounds.max[axis])));
        assert!(cropped.iter().flatten().all(|vertex| vertex[0] < 20.0));
    }
}
