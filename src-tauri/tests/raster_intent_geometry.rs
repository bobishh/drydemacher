use std::collections::BTreeMap;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use ecky_cad_lib::contracts::DesignParams;
use ecky_cad_lib::models::PathResolver;

struct TempResolver {
    root: PathBuf,
}

impl PathResolver for TempResolver {
    fn app_config_dir(&self) -> PathBuf {
        self.root.join("config")
    }

    fn app_data_dir(&self) -> PathBuf {
        self.root.join("data")
    }

    fn resource_path(&self, _path: &str) -> Option<PathBuf> {
        None
    }
}

fn binary_stl_evidence(path: &Path) -> ([f32; 3], [f32; 3], usize) {
    let mut file = std::fs::File::open(path).expect("open STL");
    file.seek(SeekFrom::Start(80)).expect("seek STL count");
    let mut count = [0_u8; 4];
    file.read_exact(&mut count).expect("read STL count");
    let triangle_count = u32::from_le_bytes(count);
    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];
    let mut edge_uses = BTreeMap::<([u32; 3], [u32; 3]), usize>::new();
    for _ in 0..triangle_count {
        let mut facet = [0_u8; 50];
        file.read_exact(&mut facet).expect("read STL facet");
        let mut triangle = [[0_u32; 3]; 3];
        for (vertex, coordinates) in triangle.iter_mut().enumerate() {
            for (axis, coordinate) in coordinates.iter_mut().enumerate() {
                let offset = 12 + vertex * 12 + axis * 4;
                let value = f32::from_le_bytes(facet[offset..offset + 4].try_into().unwrap());
                *coordinate = value.to_bits();
                min[axis] = min[axis].min(value);
                max[axis] = max[axis].max(value);
            }
        }
        for (left, right) in [(0, 1), (1, 2), (2, 0)] {
            let edge = if triangle[left] <= triangle[right] {
                (triangle[left], triangle[right])
            } else {
                (triangle[right], triangle[left])
            };
            *edge_uses.entry(edge).or_default() += 1;
        }
    }
    let bad_edges = edge_uses.values().filter(|uses| **uses != 2).count();
    (min, max, bad_edges)
}

#[test]
fn raster_extrude_traces_alpha_coverage_without_an_image_rectangle() {
    let root = std::env::temp_dir().join(format!("ecky-raster-extrude-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root).expect("create fixture root");
    let image_path = root.join("logo.png");
    let mut image = image::RgbaImage::from_pixel(8, 8, image::Rgba([0, 0, 0, 0]));
    for y in 1..7 {
        for x in 1..7 {
            image.put_pixel(x, y, image::Rgba([0, 0, 0, 64]));
        }
    }
    for y in 2..6 {
        for x in 2..6 {
            image.put_pixel(x, y, image::Rgba([0, 0, 0, 255]));
        }
    }
    image.save(&image_path).expect("save raster logo");
    let resolver = TempResolver { root: root.clone() };
    let source = format!(
        r#"(model
          (part logo
            (extrude "{}" 3
              :width 8
              :depth 8
              :threshold 0.5
              :foreground dark)))"#,
        image_path.display()
    );

    let bundle = ecky_cad_lib::ecky_ir::render_model(&source, &DesignParams::new(), &resolver)
        .expect("raster extrude should render");
    let (min, max, bad_edges) = binary_stl_evidence(Path::new(&bundle.model_stl_path));
    assert!(min[0] > 1.5 && min[1] > 1.5, "min={min:?}");
    assert!(max[0] < 6.5 && max[1] < 6.5, "max={max:?}");
    assert!((min[2] - 0.0).abs() < 1.0e-4, "min={min:?}");
    assert!((max[2] - 3.0).abs() < 1.0e-4, "max={max:?}");
    assert_eq!(bad_edges, 0, "raster extrusion must stay closed");
    std::fs::remove_dir_all(root).ok();
}

#[test]
fn raster_extrude_with_one_dimension_preserves_source_aspect_ratio() {
    let root = std::env::temp_dir().join(format!(
        "ecky-raster-extrude-aspect-{}",
        uuid::Uuid::new_v4()
    ));
    std::fs::create_dir_all(&root).expect("create fixture root");
    let image_path = root.join("wide-logo.png");
    let mut image = image::RgbaImage::from_pixel(10, 5, image::Rgba([255, 255, 255, 255]));
    for y in 1..4 {
        for x in 1..9 {
            image.put_pixel(x, y, image::Rgba([0, 0, 0, 255]));
        }
    }
    image.save(&image_path).expect("save wide raster logo");
    let resolver = TempResolver { root: root.clone() };
    let width_only = format!(
        r#"(model
          (part logo
            (extrude "{}" 3
              :width 20
              :threshold 0.5
              :foreground dark)))"#,
        image_path.display()
    );
    let explicit_aspect = format!(
        r#"(model
          (part logo
            (extrude "{}" 3
              :width 20
              :depth 10
              :threshold 0.5
              :foreground dark)))"#,
        image_path.display()
    );

    let inferred =
        ecky_cad_lib::ecky_ir::render_model(&width_only, &DesignParams::new(), &resolver)
            .expect("width-only raster extrude should preserve source aspect ratio");
    let explicit =
        ecky_cad_lib::ecky_ir::render_model(&explicit_aspect, &DesignParams::new(), &resolver)
            .expect("explicit-aspect raster extrude should render");
    assert_eq!(
        std::fs::read(inferred.model_stl_path).expect("read inferred STL"),
        std::fs::read(explicit.model_stl_path).expect("read explicit STL")
    );
    std::fs::remove_dir_all(root).ok();
}

#[test]
fn raster_extrude_contain_preserves_and_centers_aspect_inside_explicit_box() {
    let root = std::env::temp_dir().join(format!(
        "ecky-raster-extrude-contain-{}",
        uuid::Uuid::new_v4()
    ));
    std::fs::create_dir_all(&root).expect("create fixture root");
    let image_path = root.join("wide-logo.png");
    let mut image = image::RgbaImage::from_pixel(10, 5, image::Rgba([255, 255, 255, 255]));
    for y in 1..4 {
        for x in 1..9 {
            image.put_pixel(x, y, image::Rgba([0, 0, 0, 255]));
        }
    }
    image.save(&image_path).expect("save wide raster logo");
    let resolver = TempResolver { root: root.clone() };
    let contained = format!(
        r#"(model
          (part logo
            (extrude "{}" 3
              :width 20
              :depth 20
              :fit contain
              :threshold 0.5
              :foreground dark)))"#,
        image_path.display()
    );
    let explicit_aspect = format!(
        r#"(model
          (part logo
            (translate 0 5 0
              (extrude "{}" 3
                :width 20
                :depth 10
                :threshold 0.5
                :foreground dark))))"#,
        image_path.display()
    );

    let contained =
        ecky_cad_lib::ecky_ir::render_model(&contained, &DesignParams::new(), &resolver)
            .expect("contained raster extrude should render");
    let explicit =
        ecky_cad_lib::ecky_ir::render_model(&explicit_aspect, &DesignParams::new(), &resolver)
            .expect("explicit-aspect raster extrude should render");
    assert_eq!(
        std::fs::read(contained.model_stl_path).expect("read contained STL"),
        std::fs::read(explicit.model_stl_path).expect("read explicit STL")
    );
    std::fs::remove_dir_all(root).ok();
}

#[test]
fn raster_extrude_light_foreground_keeps_transparent_dark_pixels_empty() {
    let root = std::env::temp_dir().join(format!(
        "ecky-raster-extrude-light-{}",
        uuid::Uuid::new_v4()
    ));
    std::fs::create_dir_all(&root).expect("create fixture root");
    let image_path = root.join("light-logo.png");
    let mut image = image::RgbaImage::from_pixel(8, 8, image::Rgba([0, 0, 0, 0]));
    for y in 2..6 {
        for x in 2..6 {
            image.put_pixel(x, y, image::Rgba([255, 255, 255, 255]));
        }
    }
    image.save(&image_path).expect("save light raster logo");
    let resolver = TempResolver { root: root.clone() };
    let source = format!(
        r#"(model
          (part logo
            (extrude "{}" 2
              :width 8
              :depth 8
              :threshold 0
              :foreground light)))"#,
        image_path.display()
    );

    let bundle = ecky_cad_lib::ecky_ir::render_model(&source, &DesignParams::new(), &resolver)
        .expect("light raster extrude should render");
    let (min, max, bad_edges) = binary_stl_evidence(Path::new(&bundle.model_stl_path));
    assert!(min[0] > 1.5 && min[1] > 1.5, "min={min:?}");
    assert!(max[0] < 6.5 && max[1] < 6.5, "max={max:?}");
    assert!((max[2] - 2.0).abs() < 1.0e-4, "max={max:?}");
    assert_eq!(bad_edges, 0, "light raster extrusion must stay closed");
    std::fs::remove_dir_all(root).ok();
}

#[test]
fn raster_extrude_crosses_poly_brep_boundary_without_leaking_foreground_symbol() {
    let root = std::env::temp_dir().join(format!(
        "ecky-raster-extrude-hybrid-{}",
        uuid::Uuid::new_v4()
    ));
    std::fs::create_dir_all(&root).expect("create fixture root");
    let image_path = root.join("logo.png");
    let mut image = image::RgbaImage::from_pixel(8, 8, image::Rgba([0, 0, 0, 0]));
    for y in 1..7 {
        for x in 1..7 {
            image.put_pixel(x, y, image::Rgba([0, 0, 0, 255]));
        }
    }
    image.save(&image_path).expect("save raster logo");
    let resolver = TempResolver { root: root.clone() };
    let source = format!(
        r#"(model
          (params (image artwork "{}"))
          (part clipped-logo
            (intersection
              (extrude artwork 3
                :width 8
                :depth 8
                :threshold 0.5
                :foreground dark)
              (translate 4 4 -1 (cylinder 3 5)))))"#,
        image_path.display()
    );

    let bundle = ecky_cad_lib::ecky_ir::render_model(&source, &DesignParams::new(), &resolver)
        .expect("hybrid raster extrusion should render");
    let (_, _, bad_edges) = binary_stl_evidence(Path::new(&bundle.model_stl_path));
    assert_eq!(bad_edges, 0, "hybrid raster extrusion must stay closed");
    std::fs::remove_dir_all(root).ok();
}

#[test]
fn raster_protrude_maps_luminance_above_local_zero_without_public_backing() {
    let root = std::env::temp_dir().join(format!("ecky-raster-protrude-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root).expect("create fixture root");
    let image_path = root.join("relief.png");
    let mut image = image::RgbaImage::from_pixel(3, 3, image::Rgba([255, 255, 255, 255]));
    image.put_pixel(1, 1, image::Rgba([0, 0, 0, 255]));
    image.put_pixel(0, 0, image::Rgba([0, 0, 0, 0]));
    image.save(&image_path).expect("save raster relief");
    let resolver = TempResolver { root: root.clone() };
    let source = format!(
        r#"(model
          (part relief
            (protrude "{}" 4
              :width 12
              :depth 12
              :foreground dark)))"#,
        image_path.display()
    );

    let bundle = ecky_cad_lib::ecky_ir::render_model(&source, &DesignParams::new(), &resolver)
        .expect("raster protrude should render");
    let (min, max, bad_edges) = binary_stl_evidence(Path::new(&bundle.model_stl_path));
    assert!(min[2] < 0.0 && min[2] > -0.01, "internal closure={min:?}");
    assert!((max[2] - 4.0).abs() < 1.0e-4, "max={max:?}");
    assert_eq!(bad_edges, 0, "raster protrusion must stay closed");
    std::fs::remove_dir_all(root).ok();
}

#[test]
fn raster_protrude_with_one_dimension_preserves_source_aspect_ratio() {
    let root = std::env::temp_dir().join(format!(
        "ecky-raster-protrude-aspect-{}",
        uuid::Uuid::new_v4()
    ));
    std::fs::create_dir_all(&root).expect("create fixture root");
    let image_path = root.join("wide-relief.png");
    let image =
        image::RgbaImage::from_fn(10, 5, |x, y| image::Rgba([((x + y) * 16) as u8, 0, 0, 255]));
    image.save(&image_path).expect("save wide raster relief");
    let resolver = TempResolver { root: root.clone() };
    let depth_only = format!(
        r#"(model
          (part relief
            (protrude "{}" 4
              :depth 10
              :foreground dark)))"#,
        image_path.display()
    );
    let explicit_aspect = format!(
        r#"(model
          (part relief
            (protrude "{}" 4
              :width 20
              :depth 10
              :foreground dark)))"#,
        image_path.display()
    );

    let inferred =
        ecky_cad_lib::ecky_ir::render_model(&depth_only, &DesignParams::new(), &resolver)
            .expect("depth-only raster protrude should preserve source aspect ratio");
    let explicit =
        ecky_cad_lib::ecky_ir::render_model(&explicit_aspect, &DesignParams::new(), &resolver)
            .expect("explicit-aspect raster protrude should render");
    assert_eq!(
        std::fs::read(inferred.model_stl_path).expect("read inferred STL"),
        std::fs::read(explicit.model_stl_path).expect("read explicit STL")
    );
    std::fs::remove_dir_all(root).ok();
}

#[test]
fn raster_protrude_contain_centers_source_inside_explicit_box() {
    let root = std::env::temp_dir().join(format!(
        "ecky-raster-protrude-contain-{}",
        uuid::Uuid::new_v4()
    ));
    std::fs::create_dir_all(&root).expect("create fixture root");
    let image_path = root.join("wide-relief.png");
    image::RgbaImage::from_pixel(10, 5, image::Rgba([0, 0, 0, 255]))
        .save(&image_path)
        .expect("save wide raster relief");
    let resolver = TempResolver { root: root.clone() };
    let source = format!(
        r#"(model
          (part relief
            (protrude "{}" 4
              :width 20
              :depth 20
              :fit contain
              :foreground dark)))"#,
        image_path.display()
    );

    let bundle = ecky_cad_lib::ecky_ir::render_model(&source, &DesignParams::new(), &resolver)
        .expect("contained raster protrude should render");
    let (min, max, bad_edges) = binary_stl_evidence(Path::new(&bundle.model_stl_path));
    assert!((min[0] - 0.0).abs() < 1.0e-4, "min={min:?}");
    assert!((max[0] - 20.0).abs() < 1.0e-4, "max={max:?}");
    assert!((min[1] - 5.0).abs() < 1.0e-4, "min={min:?}");
    assert!((max[1] - 15.0).abs() < 1.0e-4, "max={max:?}");
    assert_eq!(bad_edges, 0, "contained protrusion must stay closed");
    std::fs::remove_dir_all(root).ok();
}

#[test]
fn raster_geometry_requires_at_least_one_physical_dimension() {
    let root = std::env::temp_dir().join(format!(
        "ecky-raster-missing-dimensions-{}",
        uuid::Uuid::new_v4()
    ));
    std::fs::create_dir_all(&root).expect("create fixture root");
    let image_path = root.join("logo.png");
    image::RgbaImage::from_pixel(2, 2, image::Rgba([0, 0, 0, 255]))
        .save(&image_path)
        .expect("save raster fixture");
    let resolver = TempResolver { root: root.clone() };

    for operation in ["extrude", "protrude"] {
        let source = format!(
            "(model (part image ({operation} \"{}\" 2 :foreground dark)))",
            image_path.display()
        );
        let error = ecky_cad_lib::ecky_ir::render_model(&source, &DesignParams::new(), &resolver)
            .expect_err("dimensionless raster geometry must fail");
        assert!(
            error
                .to_string()
                .contains("at least one of `:width` or `:depth`"),
            "{operation}: {error:?}"
        );
    }
    std::fs::remove_dir_all(root).ok();
}
