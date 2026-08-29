use std::path::Path;

use csgrs::traits::CSG;

use crate::contracts::{AppError, AppErrorCode, AppResult};
use crate::image_sampling::{
    raster_coverage_image, resolve_raster_layout, RasterFitMode, RasterForeground,
};

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

    let decoded = image::open(path).map_err(|error| {
        heightfield_error(format!("failed to decode '{}': {error}", path.display()))
    })?;
    let image = composite_alpha_on_white(decoded);
    build_sampled_surface(
        "heightfield",
        image,
        width,
        depth,
        relief_height,
        0.0,
        base_thickness,
        invert,
    )
}

pub(super) fn build_protrusion(
    image_path: &str,
    width: Option<f64>,
    depth: Option<f64>,
    height: f64,
    foreground: RasterForeground,
    fit: RasterFitMode,
) -> AppResult<IrMesh> {
    if !height.is_finite() || height <= 0.0 {
        return Err(protrude_error(format!(
            "`:height` must be finite and greater than zero, got {height}"
        )));
    }
    if image_path.trim().is_empty() {
        return Err(protrude_error(
            "image path is empty; image selection remains pending",
        ));
    }
    let path = Path::new(image_path);
    let reader = image::ImageReader::open(path)
        .map_err(|error| protrude_error(format!("failed to open '{}': {error}", path.display())))?
        .with_guessed_format()
        .map_err(|error| {
            protrude_error(format!("failed to identify '{}': {error}", path.display()))
        })?;
    let (source_width, source_height) = reader.into_dimensions().map_err(|error| {
        protrude_error(format!("failed to inspect '{}': {error}", path.display()))
    })?;
    let source_pixels = u64::from(source_width) * u64::from(source_height);
    if source_pixels > MAX_SOURCE_PIXELS {
        return Err(protrude_error(format!(
            "source pixel count {source_pixels} exceeds allowed count {MAX_SOURCE_PIXELS}"
        )));
    }
    let layout = resolve_raster_layout(source_width, source_height, width, depth, fit)
        .map_err(protrude_error)?;
    let decoded = image::open(path).map_err(|error| {
        protrude_error(format!("failed to decode '{}': {error}", path.display()))
    })?;
    Ok(build_sampled_surface(
        "protrude",
        raster_coverage_image(decoded, foreground),
        layout.width,
        layout.depth,
        height,
        -0.001,
        0.0,
        false,
    )?
    .translate(layout.offset_x, layout.offset_y, 0.0))
}

#[allow(clippy::too_many_arguments)]
fn build_sampled_surface(
    operation: &str,
    image: image::GrayImage,
    width: f64,
    depth: f64,
    relief_height: f64,
    bottom_z: f64,
    top_base_z: f64,
    invert: bool,
) -> AppResult<IrMesh> {
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
                top_base_z + relief_height * relief,
            ]);
        }
    }
    for y in 0..grid_height {
        for x in 0..grid_width {
            vertices.push([
                width * f64::from(x) / f64::from(grid_width - 1),
                depth * f64::from(y) / f64::from(grid_height - 1),
                bottom_z,
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
        return Err(operation_error(
            operation,
            format!(
                "triangle count {} exceeds allowed count {}",
                triangles.len(),
                MAX_MESH_LITERAL_TRIANGLES
            ),
        ));
    }
    build_mesh_literal(operation, vertices, triangles, true)
}

fn composite_alpha_on_white(image: image::DynamicImage) -> image::GrayImage {
    let mut rgba = image.to_rgba8();
    for pixel in rgba.pixels_mut() {
        let alpha = u16::from(pixel[3]);
        let inverse_alpha = 255 - alpha;
        for channel in &mut pixel.0[..3] {
            let source = u16::from(*channel);
            *channel = ((source * alpha + 255 * inverse_alpha + 127) / 255) as u8;
        }
        pixel[3] = 255;
    }
    image::DynamicImage::ImageRgba8(rgba).to_luma8()
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
    operation_error("heightfield", details)
}

fn protrude_error(details: impl Into<String>) -> AppError {
    operation_error("protrude", details)
}

fn operation_error(operation: &str, details: impl Into<String>) -> AppError {
    AppError::with_details(
        AppErrorCode::Validation,
        format!("Invalid `{operation}` geometry."),
        details,
    )
    .with_operation(operation)
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
