use std::path::Path;

use crate::contracts::{AppError, AppErrorCode, AppResult};

use super::mesh_literal::{
    build_mesh_literal, MAX_MESH_LITERAL_TRIANGLES, MAX_MESH_LITERAL_VERTICES,
};
use super::shared::IrMesh;

const MAX_SOURCE_PIXELS: u64 = 40_000_000;

pub(super) fn build_heightfield(
    image_path: &str,
    width: f64,
    depth: f64,
    relief_height: f64,
    base_thickness: f64,
    invert: bool,
) -> AppResult<IrMesh> {
    for (name, value) in [
        ("width", width),
        ("depth", depth),
        ("relief-height", relief_height),
        ("base-thickness", base_thickness),
    ] {
        if !value.is_finite() || value <= 0.0 {
            return Err(heightfield_error(format!(
                "`:{name}` must be finite and greater than zero, got {value}"
            )));
        }
    }
    if image_path.trim().is_empty() {
        return Err(heightfield_error(
            "image path is empty; image selection remains pending",
        ));
    }
    let path = Path::new(image_path);
    let reader = image::ImageReader::open(path)
        .map_err(|error| {
            heightfield_error(format!("failed to open '{}': {error}", path.display()))
        })?
        .with_guessed_format()
        .map_err(|error| {
            heightfield_error(format!("failed to identify '{}': {error}", path.display()))
        })?;
    let (source_width, source_height) = reader.into_dimensions().map_err(|error| {
        heightfield_error(format!("failed to inspect '{}': {error}", path.display()))
    })?;
    let source_pixels = u64::from(source_width) * u64::from(source_height);
    if source_pixels > MAX_SOURCE_PIXELS {
        return Err(heightfield_error(format!(
            "source pixel count {source_pixels} exceeds allowed count {MAX_SOURCE_PIXELS}"
        )));
    }

    let image = image::open(path)
        .map_err(|error| {
            heightfield_error(format!("failed to decode '{}': {error}", path.display()))
        })?
        .to_luma8();
    let max_grid_points = (MAX_MESH_LITERAL_VERTICES / 2).max(4);
    let (grid_width, grid_height) =
        bounded_grid_dimensions(image.width(), image.height(), max_grid_points);
    let grid_count = grid_width as usize * grid_height as usize;
    let mut vertices = Vec::with_capacity(grid_count * 2);
    for y in 0..grid_height {
        for x in 0..grid_width {
            let u = x as f32 / (grid_width - 1) as f32;
            let v = y as f32 / (grid_height - 1) as f32;
            let sample = f64::from(crate::image_sampling::bilinear_gray(&image, u, v));
            let relief = if invert { 1.0 - sample } else { sample };
            vertices.push([
                width * f64::from(x) / f64::from(grid_width - 1),
                depth * f64::from(y) / f64::from(grid_height - 1),
                base_thickness + relief_height * relief,
            ]);
        }
    }
    for y in 0..grid_height {
        for x in 0..grid_width {
            vertices.push([
                width * f64::from(x) / f64::from(grid_width - 1),
                depth * f64::from(y) / f64::from(grid_height - 1),
                0.0,
            ]);
        }
    }

    let mut triangles = Vec::with_capacity(
        ((grid_width - 1) * (grid_height - 1) * 4 + (grid_width - 1) * 4 + (grid_height - 1) * 4)
            as usize,
    );
    let top = |x: u32, y: u32| (y * grid_width + x) as usize;
    let bottom = |x: u32, y: u32| grid_count + top(x, y);
    for y in 0..(grid_height - 1) {
        for x in 0..(grid_width - 1) {
            let a = top(x, y);
            let b = top(x + 1, y);
            let c = top(x, y + 1);
            let d = top(x + 1, y + 1);
            triangles.extend([[a, b, d], [a, d, c]]);

            let ba = bottom(x, y);
            let bb = bottom(x + 1, y);
            let bc = bottom(x, y + 1);
            let bd = bottom(x + 1, y + 1);
            triangles.extend([[ba, bd, bb], [ba, bc, bd]]);
        }
    }

    for x in 0..(grid_width - 1) {
        let t0 = top(x, 0);
        let t1 = top(x + 1, 0);
        let b0 = bottom(x, 0);
        let b1 = bottom(x + 1, 0);
        triangles.extend([[b0, b1, t1], [b0, t1, t0]]);

        let t0 = top(x, grid_height - 1);
        let t1 = top(x + 1, grid_height - 1);
        let b0 = bottom(x, grid_height - 1);
        let b1 = bottom(x + 1, grid_height - 1);
        triangles.extend([[b0, t1, b1], [b0, t0, t1]]);
    }
    for y in 0..(grid_height - 1) {
        let t0 = top(0, y);
        let t1 = top(0, y + 1);
        let b0 = bottom(0, y);
        let b1 = bottom(0, y + 1);
        triangles.extend([[b0, t1, b1], [b0, t0, t1]]);

        let t0 = top(grid_width - 1, y);
        let t1 = top(grid_width - 1, y + 1);
        let b0 = bottom(grid_width - 1, y);
        let b1 = bottom(grid_width - 1, y + 1);
        triangles.extend([[b0, b1, t1], [b0, t1, t0]]);
    }

    if triangles.len() > MAX_MESH_LITERAL_TRIANGLES {
        return Err(heightfield_error(format!(
            "triangle count {} exceeds allowed count {}",
            triangles.len(),
            MAX_MESH_LITERAL_TRIANGLES
        )));
    }
    build_mesh_literal("heightfield", vertices, triangles, true)
}

fn bounded_grid_dimensions(width: u32, height: u32, max_points: usize) -> (u32, u32) {
    let width = width.max(2);
    let height = height.max(2);
    let points = width as usize * height as usize;
    if points <= max_points {
        return (width, height);
    }
    let scale = (max_points as f64 / points as f64).sqrt();
    let mut bounded_width = ((width as f64 * scale).floor() as u32).max(2);
    let mut bounded_height = ((height as f64 * scale).floor() as u32).max(2);
    while bounded_width as usize * bounded_height as usize > max_points {
        if bounded_width >= bounded_height && bounded_width > 2 {
            bounded_width -= 1;
        } else if bounded_height > 2 {
            bounded_height -= 1;
        } else {
            break;
        }
    }
    (bounded_width, bounded_height)
}

fn heightfield_error(details: impl Into<String>) -> AppError {
    AppError::with_details(
        AppErrorCode::Validation,
        "Invalid `heightfield` geometry.",
        details,
    )
    .with_operation("heightfield")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_grid_never_exceeds_mesh_vertex_budget() {
        let (width, height) = bounded_grid_dimensions(12_000, 8_000, 50_000);
        assert!(width as usize * height as usize <= 50_000);
        assert!(width >= 2 && height >= 2);
    }
}
