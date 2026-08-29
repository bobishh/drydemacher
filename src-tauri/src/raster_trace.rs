use crate::contracts::{
    AppError, AppResult, RasterTraceAssetIdentity, RasterTraceContour, RasterTraceProvenance,
    RasterTraceRequest, RasterTraceResponse,
};
use image::ImageReader;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::io::Cursor;

use crate::image_sampling::{raster_coverage_image, RasterForeground};

pub const RASTER_TRACE_EXTRACTOR_VERSION: &str = "raster-trace-v1";
pub const MAX_RASTER_TRACE_PIXELS: u64 = 40_000_000;
const MAX_RASTER_TRACE_FILE_BYTES: u64 = 64 * 1024 * 1024;
const MAX_RASTER_TRACE_CONTOURS: usize = 256;
const DEFAULT_MAX_RASTER_TRACE_CONTOURS: usize = 64;
const MIN_COMPONENT_PIXELS: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct GridPoint {
    x: u32,
    y: u32,
}

#[derive(Debug)]
struct PixelComponent {
    pixels: Vec<(u32, u32)>,
}

#[derive(Debug)]
struct PixelLoop {
    points: Vec<GridPoint>,
    foreground_pixel_count: usize,
}

pub fn extract_raster_contours(request: RasterTraceRequest) -> AppResult<RasterTraceResponse> {
    validate_request(&request)?;
    let path = std::path::Path::new(&request.image_path);
    let metadata = std::fs::metadata(path).map_err(|error| {
        raster_error(
            &request,
            format!("failed to inspect image '{}': {error}", path.display()),
        )
    })?;
    if metadata.len() > MAX_RASTER_TRACE_FILE_BYTES {
        return Err(raster_error(
            &request,
            format!(
                "image file has {} bytes; allowedBytes={MAX_RASTER_TRACE_FILE_BYTES}",
                metadata.len()
            ),
        ));
    }

    let bytes = std::fs::read(path).map_err(|error| {
        raster_error(
            &request,
            format!("failed to read image '{}': {error}", path.display()),
        )
    })?;
    let reader = ImageReader::new(Cursor::new(&bytes))
        .with_guessed_format()
        .map_err(|error| {
            raster_error(&request, format!("image format detection failed: {error}"))
        })?;
    let (width, height) = reader
        .into_dimensions()
        .map_err(|error| raster_error(&request, format!("image dimensions failed: {error}")))?;
    ensure_pixel_budget(width, height).map_err(|detail| raster_error(&request, detail))?;

    let decoded = image::load_from_memory(&bytes)
        .map_err(|error| raster_error(&request, format!("image decode failed: {error}")))?;
    let image = raster_coverage_image(
        decoded,
        if request.invert {
            RasterForeground::Light
        } else {
            RasterForeground::Dark
        },
    );
    let foreground = threshold_pixels(&image, request.threshold);
    let components = connected_components(&foreground, width, height);
    let connected_component_count = components.len();
    let mut loops = components
        .iter()
        .filter(|component| component.pixels.len() >= MIN_COMPONENT_PIXELS)
        .flat_map(|component| trace_component_loops(component, width, height))
        .filter(|pixel_loop| pixel_loop.points.len() >= 3)
        .collect::<Vec<_>>();
    loops.sort_by(|left, right| {
        signed_grid_area(&right.points)
            .abs()
            .total_cmp(&signed_grid_area(&left.points).abs())
            .then_with(|| left.points.cmp(&right.points))
    });

    let max_contours = request
        .max_contours
        .unwrap_or(DEFAULT_MAX_RASTER_TRACE_CONTOURS);
    loops.truncate(max_contours);
    if loops.is_empty() {
        return Err(raster_error(
            &request,
            format!("no closed contour found; connectedComponents={connected_component_count}"),
        ));
    }

    let asset = RasterTraceAssetIdentity {
        image_path: request.image_path.clone(),
        digest: format!("sha256:{:x}", Sha256::digest(&bytes)),
        width_pixels: width,
        height_pixels: height,
    };
    let contours = loops
        .into_iter()
        .enumerate()
        .map(|(index, pixel_loop)| {
            let contour_id = format!("raster-{}-{index}", sketch_view_label(&request.view));
            let points = pixel_loop
                .points
                .iter()
                .map(|point| {
                    [
                        point.x as f64 / width as f64 * request.calibration.physical_width,
                        request.calibration.physical_height
                            - point.y as f64 / height as f64 * request.calibration.physical_height,
                    ]
                })
                .collect::<Vec<_>>();
            let provenance = RasterTraceProvenance {
                kind: "rasterTrace".to_string(),
                asset: asset.clone(),
                view: request.view.clone(),
                calibration: request.calibration.clone(),
                threshold: request.threshold,
                invert: request.invert,
                contour_id: contour_id.clone(),
                extractor_version: RASTER_TRACE_EXTRACTOR_VERSION.to_string(),
            };
            RasterTraceContour {
                contour_id,
                signed_area: signed_area(&points),
                points,
                closed: true,
                foreground_pixel_count: pixel_loop.foreground_pixel_count,
                provenance,
            }
        })
        .collect::<Vec<_>>();

    Ok(RasterTraceResponse {
        asset,
        evidence: vec![
            format!("decoded raster {width}x{height}"),
            format!("threshold={} invert={}", request.threshold, request.invert),
            format!(
                "connectedComponents={connected_component_count} closedContours={}",
                contours.len()
            ),
        ],
        contours,
        connected_component_count,
        extractor_version: RASTER_TRACE_EXTRACTOR_VERSION.to_string(),
    })
}

fn validate_request(request: &RasterTraceRequest) -> AppResult<()> {
    if request.image_path.trim().is_empty() {
        return Err(raster_error(request, "imagePath must not be empty"));
    }
    if !request.calibration.physical_width.is_finite() || request.calibration.physical_width <= 0.0
    {
        return Err(raster_error(
            request,
            "physicalWidth must be finite and greater than zero",
        ));
    }
    if !request.calibration.physical_height.is_finite()
        || request.calibration.physical_height <= 0.0
    {
        return Err(raster_error(
            request,
            "physicalHeight must be finite and greater than zero",
        ));
    }
    if request.max_contours == Some(0)
        || request
            .max_contours
            .is_some_and(|count| count > MAX_RASTER_TRACE_CONTOURS)
    {
        return Err(raster_error(
            request,
            format!("maxContours must be within 1..={MAX_RASTER_TRACE_CONTOURS}"),
        ));
    }
    Ok(())
}

fn ensure_pixel_budget(width: u32, height: u32) -> Result<(), String> {
    let pixels = u64::from(width) * u64::from(height);
    if width == 0 || height == 0 || pixels > MAX_RASTER_TRACE_PIXELS {
        return Err(format!(
            "image dimensions {width}x{height} have observedPixels={pixels}; allowedPixels={MAX_RASTER_TRACE_PIXELS}"
        ));
    }
    Ok(())
}

fn threshold_pixels(image: &image::GrayImage, threshold: u8) -> Vec<bool> {
    image
        .pixels()
        .map(|pixel| pixel[0] > 0 && pixel[0] >= threshold)
        .collect()
}

fn connected_components(foreground: &[bool], width: u32, height: u32) -> Vec<PixelComponent> {
    let mut visited = vec![false; foreground.len()];
    let mut components = Vec::new();
    for y in 0..height {
        for x in 0..width {
            let index = pixel_index(x, y, width);
            if !foreground[index] || visited[index] {
                continue;
            }
            visited[index] = true;
            let mut queue = VecDeque::from([(x, y)]);
            let mut pixels = Vec::new();
            while let Some((current_x, current_y)) = queue.pop_front() {
                pixels.push((current_x, current_y));
                for (next_x, next_y) in orthogonal_neighbors(current_x, current_y, width, height) {
                    let next_index = pixel_index(next_x, next_y, width);
                    if foreground[next_index] && !visited[next_index] {
                        visited[next_index] = true;
                        queue.push_back((next_x, next_y));
                    }
                }
            }
            components.push(PixelComponent { pixels });
        }
    }
    components
}

fn trace_component_loops(component: &PixelComponent, width: u32, height: u32) -> Vec<PixelLoop> {
    let pixels = component.pixels.iter().copied().collect::<BTreeSet<_>>();
    let mut edges = BTreeSet::new();
    for &(x, y) in &component.pixels {
        if y == 0 || !pixels.contains(&(x, y - 1)) {
            edges.insert((GridPoint { x, y }, GridPoint { x: x + 1, y }));
        }
        if x + 1 >= width || !pixels.contains(&(x + 1, y)) {
            edges.insert((GridPoint { x: x + 1, y }, GridPoint { x: x + 1, y: y + 1 }));
        }
        if y + 1 >= height || !pixels.contains(&(x, y + 1)) {
            edges.insert((GridPoint { x: x + 1, y: y + 1 }, GridPoint { x, y: y + 1 }));
        }
        if x == 0 || !pixels.contains(&(x - 1, y)) {
            edges.insert((GridPoint { x, y: y + 1 }, GridPoint { x, y }));
        }
    }

    let mut adjacency = BTreeMap::<GridPoint, BTreeSet<GridPoint>>::new();
    for (start, end) in &edges {
        adjacency.entry(*start).or_default().insert(*end);
    }
    let mut unused = edges;
    let mut loops = Vec::new();
    while let Some(&(start, first_end)) = unused.iter().next() {
        let mut points = vec![start];
        let mut current = start;
        let mut next = first_end;
        let mut closed = false;
        while unused.remove(&(current, next)) {
            current = next;
            if current == start {
                closed = true;
                break;
            }
            points.push(current);
            let Some(candidate) = adjacency
                .get(&current)
                .and_then(|ends| ends.iter().find(|end| unused.contains(&(current, **end))))
                .copied()
            else {
                break;
            };
            next = candidate;
        }
        if closed {
            let simplified = remove_collinear_grid_points(points);
            if simplified.len() >= 3 && signed_grid_area(&simplified).abs() > f64::EPSILON {
                loops.push(PixelLoop {
                    points: simplified,
                    foreground_pixel_count: component.pixels.len(),
                });
            }
        }
    }
    loops
}

fn remove_collinear_grid_points(mut points: Vec<GridPoint>) -> Vec<GridPoint> {
    loop {
        if points.len() <= 3 {
            return points;
        }
        let mut kept = Vec::with_capacity(points.len());
        for index in 0..points.len() {
            let previous = points[(index + points.len() - 1) % points.len()];
            let current = points[index];
            let next = points[(index + 1) % points.len()];
            let cross = (current.x as i64 - previous.x as i64) * (next.y as i64 - current.y as i64)
                - (current.y as i64 - previous.y as i64) * (next.x as i64 - current.x as i64);
            if cross != 0 {
                kept.push(current);
            }
        }
        if kept.len() == points.len() || kept.len() < 3 {
            return points;
        }
        points = kept;
    }
}

fn orthogonal_neighbors(x: u32, y: u32, width: u32, height: u32) -> Vec<(u32, u32)> {
    let mut neighbors = Vec::with_capacity(4);
    if x > 0 {
        neighbors.push((x - 1, y));
    }
    if x + 1 < width {
        neighbors.push((x + 1, y));
    }
    if y > 0 {
        neighbors.push((x, y - 1));
    }
    if y + 1 < height {
        neighbors.push((x, y + 1));
    }
    neighbors
}

fn pixel_index(x: u32, y: u32, width: u32) -> usize {
    (u64::from(y) * u64::from(width) + u64::from(x)) as usize
}

fn signed_grid_area(points: &[GridPoint]) -> f64 {
    points
        .iter()
        .zip(points.iter().cycle().skip(1))
        .take(points.len())
        .map(|(left, right)| left.x as f64 * right.y as f64 - right.x as f64 * left.y as f64)
        .sum::<f64>()
        * 0.5
}

fn signed_area(points: &[[f64; 2]]) -> f64 {
    points
        .iter()
        .zip(points.iter().cycle().skip(1))
        .take(points.len())
        .map(|(left, right)| left[0] * right[1] - right[0] * left[1])
        .sum::<f64>()
        * 0.5
}

fn sketch_view_label(view: &crate::contracts::SketchView) -> &'static str {
    match view {
        crate::contracts::SketchView::Front => "front",
        crate::contracts::SketchView::Top => "top",
        crate::contracts::SketchView::Side => "side",
        crate::contracts::SketchView::Custom => "custom",
    }
}

fn raster_error(request: &RasterTraceRequest, detail: impl AsRef<str>) -> AppError {
    AppError::validation(format!(
        "Raster trace failed: path='{}' view={} threshold={} invert={}: {}",
        request.image_path,
        sketch_view_label(&request.view),
        request.threshold,
        request.invert,
        detail.as_ref()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::{RasterTraceCalibration, SketchView};
    use image::{GrayImage, Luma};
    use std::path::{Path, PathBuf};

    fn fixture_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "ecky-raster-trace-{name}-{}.png",
            uuid::Uuid::new_v4()
        ))
    }

    fn save_two_rectangles(path: &Path) {
        let mut image = GrayImage::from_pixel(32, 24, Luma([255]));
        for y in 3..11 {
            for x in 2..12 {
                image.put_pixel(x, y, Luma([0]));
            }
        }
        for y in 8..21 {
            for x in 20..29 {
                image.put_pixel(x, y, Luma([0]));
            }
        }
        image.save(path).expect("save raster fixture");
    }

    fn request(path: &Path) -> RasterTraceRequest {
        RasterTraceRequest {
            image_path: path.to_string_lossy().to_string(),
            view: SketchView::Front,
            calibration: RasterTraceCalibration {
                physical_width: 160.0,
                physical_height: 120.0,
            },
            threshold: 127,
            invert: false,
            max_contours: Some(8),
        }
    }

    #[test]
    fn extracts_connected_closed_contours_deterministically() {
        let path = fixture_path("rectangles");
        save_two_rectangles(&path);

        let first = extract_raster_contours(request(&path)).expect("extract contours");
        let second = extract_raster_contours(request(&path)).expect("extract contours again");

        assert_eq!(first, second);
        assert_eq!(first.connected_component_count, 2);
        assert_eq!(first.contours.len(), 2);
        assert!(first.contours.iter().all(|contour| contour.closed));
        assert!(first
            .contours
            .iter()
            .all(|contour| contour.points.len() == 4));
        assert!(first.asset.digest.starts_with("sha256:"));
        assert_eq!(first.extractor_version, RASTER_TRACE_EXTRACTOR_VERSION);
        assert_eq!(first.asset.width_pixels, 32);
        assert_eq!(first.asset.height_pixels, 24);
        assert!(first.contours[0].signed_area.abs() > first.contours[1].signed_area.abs());

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn rejects_image_without_closed_contour_with_raw_context() {
        let path = fixture_path("empty");
        GrayImage::from_pixel(8, 8, Luma([255]))
            .save(&path)
            .expect("save empty raster fixture");

        let error = extract_raster_contours(request(&path)).expect_err("empty image must reject");
        let message = error.to_string();
        assert!(message.contains(path.to_string_lossy().as_ref()));
        assert!(message.contains("threshold=127"));
        assert!(message.contains("no closed contour"));

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn rejects_invalid_calibration_before_decode() {
        let path = Path::new("/tmp/missing-raster.png");
        let mut invalid = request(path);
        invalid.calibration.physical_width = 0.0;
        let error = extract_raster_contours(invalid).expect_err("zero width must reject");
        assert!(error.to_string().contains("physicalWidth"));
    }

    #[test]
    fn pixel_budget_reports_observed_and_allowed_counts() {
        let error = ensure_pixel_budget(10_000, 5_000).expect_err("oversized raster must reject");
        assert!(error.contains("observedPixels=50000000"));
        assert!(error.contains("allowedPixels=40000000"));
    }
}
