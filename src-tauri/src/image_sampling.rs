use image::GrayImage;

/// Deterministic normalized grayscale sample shared by displacement,
/// lithophane, and source-authored heightfield geometry.
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
    use image::Luma;

    #[test]
    fn grayscale_sampling_is_bilinear_and_clamped() {
        let image = GrayImage::from_fn(2, 2, |x, y| Luma([((x + y * 2) * 85) as u8]));

        let center = bilinear_gray(&image, 0.5, 0.5);
        assert!((center - 0.5).abs() < 1e-6, "{center}");
        assert_eq!(bilinear_gray(&image, -1.0, -1.0), 0.0);
        assert_eq!(bilinear_gray(&image, 2.0, 2.0), 1.0);
    }
}
