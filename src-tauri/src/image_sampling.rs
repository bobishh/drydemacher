use image::{DynamicImage, GrayImage, Luma, Pixel};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RasterForeground {
    Dark,
    Light,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RasterFitMode {
    Contain,
    Stretch,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct RasterLayout {
    pub width: f64,
    pub depth: f64,
    pub offset_x: f64,
    pub offset_y: f64,
}

/// Fits a raster into its physical authoring box.
/// One dimension preserves aspect directly. Two dimensions contain by default;
/// non-uniform scaling requires explicit stretch mode.
pub(crate) fn resolve_raster_layout(
    source_width: u32,
    source_height: u32,
    width: Option<f64>,
    depth: Option<f64>,
    fit: RasterFitMode,
) -> Result<RasterLayout, &'static str> {
    if source_width == 0 || source_height == 0 {
        return Err("source image dimensions must be greater than zero");
    }
    for value in [width, depth].into_iter().flatten() {
        if !value.is_finite() || value <= 0.0 {
            return Err("`:width` and `:depth` must be finite and greater than zero");
        }
    }
    let source_aspect = f64::from(source_width) / f64::from(source_height);
    let (width, depth, offset_x, offset_y) = match (width, depth) {
        (Some(width), Some(depth)) if fit == RasterFitMode::Stretch => (width, depth, 0.0, 0.0),
        (Some(box_width), Some(box_depth)) => {
            let box_aspect = box_width / box_depth;
            if box_aspect >= source_aspect {
                let width = box_depth * source_aspect;
                (width, box_depth, (box_width - width) * 0.5, 0.0)
            } else {
                let depth = box_width / source_aspect;
                (box_width, depth, 0.0, (box_depth - depth) * 0.5)
            }
        }
        (Some(width), None) => (width, width / source_aspect, 0.0, 0.0),
        (None, Some(depth)) => (depth * source_aspect, depth, 0.0, 0.0),
        (None, None) => return Err("requires at least one of `:width` or `:depth`"),
    };
    Ok(RasterLayout {
        width,
        depth,
        offset_x,
        offset_y,
    })
}

/// Converts RGBA artwork into normalized foreground coverage.
/// Alpha is always coverage: transparent pixels remain empty for both modes.
pub(crate) fn raster_coverage_image(
    image: DynamicImage,
    foreground: RasterForeground,
) -> GrayImage {
    let rgba = image.to_rgba8();
    GrayImage::from_fn(rgba.width(), rgba.height(), |x, y| {
        let pixel = rgba.get_pixel(x, y);
        let alpha = u16::from(pixel[3]);
        let luminance = u16::from(pixel.to_luma()[0]);
        let selected = match foreground {
            RasterForeground::Dark => 255 - luminance,
            RasterForeground::Light => luminance,
        };
        Luma([((alpha * selected + 127) / 255) as u8])
    })
}

/// Deterministic normalized grayscale sample shared by displacement,
/// lithophane, and source-authored raster geometry.
pub(crate) fn bilinear_gray(image: &GrayImage, u: f32, v: f32) -> f32 {
    let width = image.width().max(1);
    let height = image.height().max(1);
    let x = u.clamp(0.0, 1.0) * (width as f32 - 1.0);
    let y = v.clamp(0.0, 1.0) * (height as f32 - 1.0);
    let x0 = x.floor() as u32;
    let y0 = y.floor() as u32;
    let x1 = (x0 + 1).min(width - 1);
    let y1 = (y0 + 1).min(height - 1);
    let tx = x - x0 as f32;
    let ty = y - y0 as f32;
    let p00 = image.get_pixel(x0, y0).0[0] as f32 / 255.0;
    let p10 = image.get_pixel(x1, y0).0[0] as f32 / 255.0;
    let p01 = image.get_pixel(x0, y1).0[0] as f32 / 255.0;
    let p11 = image.get_pixel(x1, y1).0[0] as f32 / 255.0;
    let top = p00 + (p10 - p00) * tx;
    let bottom = p01 + (p11 - p01) * tx;
    top + (bottom - top) * ty
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    #[test]
    fn one_physical_dimension_preserves_raster_aspect_ratio() {
        assert_eq!(
            resolve_raster_layout(100, 50, Some(40.0), None, RasterFitMode::Contain),
            Ok(RasterLayout {
                width: 40.0,
                depth: 20.0,
                offset_x: 0.0,
                offset_y: 0.0,
            })
        );
        assert_eq!(
            resolve_raster_layout(100, 50, None, Some(20.0), RasterFitMode::Contain),
            Ok(RasterLayout {
                width: 40.0,
                depth: 20.0,
                offset_x: 0.0,
                offset_y: 0.0,
            })
        );
    }

    #[test]
    fn explicit_box_contains_by_default_and_stretches_only_when_requested() {
        assert_eq!(
            resolve_raster_layout(100, 50, Some(40.0), Some(30.0), RasterFitMode::Contain,),
            Ok(RasterLayout {
                width: 40.0,
                depth: 20.0,
                offset_x: 0.0,
                offset_y: 5.0,
            })
        );
        assert_eq!(
            resolve_raster_layout(100, 50, Some(40.0), Some(30.0), RasterFitMode::Stretch,),
            Ok(RasterLayout {
                width: 40.0,
                depth: 30.0,
                offset_x: 0.0,
                offset_y: 0.0,
            })
        );
    }

    #[test]
    fn grayscale_sampling_is_bilinear_and_clamped() {
        let image = GrayImage::from_fn(2, 2, |x, y| Luma([((x + y * 2) * 85) as u8]));

        let center = bilinear_gray(&image, 0.5, 0.5);
        assert!((center - 0.5).abs() < 1e-6, "{center}");
        assert_eq!(bilinear_gray(&image, -1.0, -1.0), 0.0);
        assert_eq!(bilinear_gray(&image, 2.0, 2.0), 1.0);
    }

    #[test]
    fn foreground_coverage_never_inverts_alpha() {
        let image = image::RgbaImage::from_fn(4, 1, |x, _| match x {
            0 => Rgba([0, 0, 0, 255]),
            1 => Rgba([255, 255, 255, 255]),
            2 => Rgba([0, 0, 0, 128]),
            _ => Rgba([0, 0, 0, 0]),
        });

        let dark = raster_coverage_image(
            DynamicImage::ImageRgba8(image.clone()),
            RasterForeground::Dark,
        );
        let light = raster_coverage_image(DynamicImage::ImageRgba8(image), RasterForeground::Light);

        assert_eq!(dark.as_raw(), &[255, 0, 128, 0]);
        assert_eq!(light.as_raw(), &[0, 255, 0, 0]);
    }
}
