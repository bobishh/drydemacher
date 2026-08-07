use crate::contracts::{AppError, AppResult, CaptureFrameManifest, CaptureMeshPreview};
use crate::ecky_ir::mesh_asset::{IndexedMeshAsset, MeshAsset, MeshAssetSource};
use sha2::{Digest, Sha256};
use std::future::Future;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use tokio::sync::Notify;

const APPLE_HELPER_SOURCE: &str = include_str!("../native/capture_object.swift");

pub type ReconstructionFuture<'a> =
    Pin<Box<dyn Future<Output = AppResult<ReconstructionResult>> + Send + 'a>>;
pub type ProgressCallback = Arc<dyn Fn(f32) + Send + Sync>;

#[derive(Debug, Clone)]
pub struct ReconstructionInput {
    pub session_id: String,
    pub manifest: CaptureFrameManifest,
    pub source_dir: PathBuf,
    pub output_dir: PathBuf,
    pub tool_cache_dir: PathBuf,
}

#[derive(Debug, Clone)]
pub struct ReconstructionResult {
    pub preview: CaptureMeshPreview,
}

#[derive(Clone, Default)]
pub struct ReconstructionCancellation {
    cancelled: Arc<AtomicBool>,
    notify: Arc<Notify>,
}

impl ReconstructionCancellation {
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
        self.notify.notify_waiters();
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
}

pub trait ReconstructionProvider: Send + Sync {
    fn id(&self) -> &'static str;
    fn availability(&self, tool_cache_dir: &Path) -> AppResult<()>;
    fn reconstruct<'a>(
        &'a self,
        input: &'a ReconstructionInput,
        progress: ProgressCallback,
        cancellation: ReconstructionCancellation,
    ) -> ReconstructionFuture<'a>;
}

#[derive(Debug, Clone, Default)]
pub struct AppleObjectCaptureProvider;

fn helper_binary(tool_cache_dir: &Path) -> AppResult<PathBuf> {
    #[cfg(not(target_os = "macos"))]
    {
        let _ = tool_cache_dir;
        return Err(AppError::provider("Apple Object Capture requires macOS."));
    }
    #[cfg(target_os = "macos")]
    {
        let digest = format!("{:x}", Sha256::digest(APPLE_HELPER_SOURCE.as_bytes()));
        let root = tool_cache_dir
            .join("apple-object-capture")
            .join(&digest[..16]);
        let source_path = root.join("capture_object.swift");
        let binary_path = root.join("capture-object");
        if binary_path.is_file() {
            return Ok(binary_path);
        }
        std::fs::create_dir_all(&root).map_err(|error| {
            AppError::persistence(format!("Object Capture tool directory failed: {error}"))
        })?;
        std::fs::write(&source_path, APPLE_HELPER_SOURCE).map_err(|error| {
            AppError::persistence(format!(
                "Object Capture helper source write failed: {error}"
            ))
        })?;
        let output = std::process::Command::new("xcrun")
            .args(["swiftc", "-parse-as-library", "-framework", "RealityKit"])
            .arg(&source_path)
            .arg("-o")
            .arg(&binary_path)
            .output()
            .map_err(|error| {
                AppError::provider(format!("Failed to launch Swift compiler: {error}"))
            })?;
        if !output.status.success() {
            return Err(AppError::provider(format!(
                "Apple Object Capture helper compile failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            )));
        }
        Ok(binary_path)
    }
}

impl ReconstructionProvider for AppleObjectCaptureProvider {
    fn id(&self) -> &'static str {
        "apple-object-capture"
    }

    fn availability(&self, tool_cache_dir: &Path) -> AppResult<()> {
        let binary = helper_binary(tool_cache_dir)?;
        let output = std::process::Command::new(binary)
            .arg("--check")
            .output()
            .map_err(|error| {
                AppError::provider(format!("Object Capture availability failed: {error}"))
            })?;
        if output.status.success() {
            Ok(())
        } else {
            Err(AppError::provider(
                String::from_utf8_lossy(&output.stderr).trim().to_string(),
            ))
        }
    }

    fn reconstruct<'a>(
        &'a self,
        input: &'a ReconstructionInput,
        progress: ProgressCallback,
        cancellation: ReconstructionCancellation,
    ) -> ReconstructionFuture<'a> {
        Box::pin(async move {
            if cancellation.is_cancelled() {
                return Err(AppError::conflict("Reconstruction cancelled."));
            }
            let binary = helper_binary(&input.tool_cache_dir)?;
            let object_dir = input.output_dir.join("object");
            tokio::fs::create_dir_all(&input.output_dir)
                .await
                .map_err(|error| AppError::persistence(error.to_string()))?;
            if object_dir.exists() {
                tokio::fs::remove_dir_all(&object_dir)
                    .await
                    .map_err(|error| AppError::persistence(error.to_string()))?;
            }
            let mut child = tokio::process::Command::new(binary)
                .arg(&input.source_dir)
                .arg(&object_dir)
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .map_err(|error| {
                    AppError::provider(format!("Object Capture launch failed: {error}"))
                })?;
            let mut stdout = child
                .stdout
                .take()
                .ok_or_else(|| AppError::internal("Object Capture stdout unavailable."))?;
            let stderr = child
                .stderr
                .take()
                .ok_or_else(|| AppError::internal("Object Capture stderr unavailable."))?;
            let progress_reader = progress.clone();
            let stderr_task = tokio::spawn(async move {
                let mut lines = BufReader::new(stderr).lines();
                let mut raw = Vec::new();
                while let Ok(Some(line)) = lines.next_line().await {
                    if let Some(value) = line.strip_prefix("PROGRESS ") {
                        if let Ok(value) = value.parse::<f32>() {
                            progress_reader(value.clamp(0.0, 1.0));
                        }
                    } else {
                        raw.push(line);
                    }
                }
                raw.join("\n")
            });
            let mut stdout_bytes = Vec::new();
            let stdout_task = tokio::spawn(async move {
                let _ = stdout.read_to_end(&mut stdout_bytes).await;
                stdout_bytes
            });
            let status = tokio::select! {
                status = child.wait() => status.map_err(|error| AppError::provider(error.to_string()))?,
                _ = cancellation.notify.notified() => {
                    let _ = child.kill().await;
                    return Err(AppError::conflict("Reconstruction cancelled."));
                }
            };
            let raw_stderr = stderr_task.await.unwrap_or_else(|error| error.to_string());
            let _stdout = stdout_task.await.unwrap_or_default();
            if !status.success() {
                return Err(AppError::provider(if raw_stderr.trim().is_empty() {
                    format!("Apple Object Capture exited with {status}.")
                } else {
                    raw_stderr
                }));
            }
            let obj_path = find_first_extension(&object_dir, "obj")?
                .ok_or_else(|| AppError::provider("Apple Object Capture produced no OBJ mesh."))?;
            let stl_path = input.output_dir.join("preview.stl");
            let conversion = convert_obj_to_stl(&obj_path, &stl_path, 1_000.0)?;
            let source = MeshAssetSource::Generated {
                provider: self.id().to_string(),
                model: Some(input.session_id.clone()),
            };
            let indexed = IndexedMeshAsset::from_stl(source, &stl_path)?;
            let _mesh_asset =
                MeshAsset::generated(self.id(), Some(input.session_id.clone()), &stl_path)?;
            let topology = indexed.topology();
            let mut warnings = Vec::new();
            if !topology.closed {
                warnings.push(format!(
                    "Mesh is open: {} boundary edges, {} non-manifold edges.",
                    topology.boundary_edge_count, topology.non_manifold_edge_count
                ));
            }
            warnings.push("Photogrammetry dimensions remain approximate; verify critical sizes with calipers.".into());
            progress(1.0);
            Ok(ReconstructionResult {
                preview: CaptureMeshPreview {
                    stl_path: stl_path.to_string_lossy().into_owned(),
                    triangle_count: indexed.triangles().len() as u64,
                    bounds_mm: conversion.bounds_mm,
                    scale_label: "Object Capture meters converted to millimeters".into(),
                    warnings,
                },
            })
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
struct ObjConversion {
    bounds_mm: [f64; 3],
}

fn find_first_extension(root: &Path, extension: &str) -> AppResult<Option<PathBuf>> {
    if !root.exists() {
        return Ok(None);
    }
    for entry in
        std::fs::read_dir(root).map_err(|error| AppError::persistence(error.to_string()))?
    {
        let path = entry
            .map_err(|error| AppError::persistence(error.to_string()))?
            .path();
        if path.is_dir() {
            if let Some(found) = find_first_extension(&path, extension)? {
                return Ok(Some(found));
            }
        } else if path
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|value| value.eq_ignore_ascii_case(extension))
        {
            return Ok(Some(path));
        }
    }
    Ok(None)
}

fn convert_obj_to_stl(obj_path: &Path, stl_path: &Path, scale: f64) -> AppResult<ObjConversion> {
    let source = std::fs::read_to_string(obj_path)
        .map_err(|error| AppError::validation(format!("OBJ read failed: {error}")))?;
    let mut vertices = Vec::<[f64; 3]>::new();
    let mut triangles = Vec::<[[f64; 3]; 3]>::new();
    for (line_number, line) in source.lines().enumerate() {
        let mut fields = line.split_whitespace();
        match fields.next() {
            Some("v") => {
                let values = fields
                    .take(3)
                    .map(|value| value.parse::<f64>())
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|_| {
                        AppError::validation(format!(
                            "OBJ vertex parse failed at line {}.",
                            line_number + 1
                        ))
                    })?;
                if values.len() != 3 || values.iter().any(|value| !value.is_finite()) {
                    return Err(AppError::validation(format!(
                        "OBJ vertex invalid at line {}.",
                        line_number + 1
                    )));
                }
                vertices.push([values[0] * scale, values[1] * scale, values[2] * scale]);
            }
            Some("f") => {
                let indices = fields
                    .map(|field| field.split('/').next().unwrap_or_default())
                    .map(|value| value.parse::<isize>())
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|_| {
                        AppError::validation(format!(
                            "OBJ face parse failed at line {}.",
                            line_number + 1
                        ))
                    })?;
                if indices.len() < 3 {
                    return Err(AppError::validation(format!(
                        "OBJ face has fewer than 3 vertices at line {}.",
                        line_number + 1
                    )));
                }
                let resolve = |index: isize| -> AppResult<[f64; 3]> {
                    let resolved = if index > 0 {
                        index - 1
                    } else {
                        vertices.len() as isize + index
                    };
                    vertices.get(resolved as usize).copied().ok_or_else(|| {
                        AppError::validation(format!(
                            "OBJ face index out of bounds at line {}.",
                            line_number + 1
                        ))
                    })
                };
                for corner in 1..indices.len() - 1 {
                    triangles.push([
                        resolve(indices[0])?,
                        resolve(indices[corner])?,
                        resolve(indices[corner + 1])?,
                    ]);
                }
            }
            _ => {}
        }
    }
    if triangles.is_empty() {
        return Err(AppError::validation("OBJ contains no triangle faces."));
    }
    let mut output = std::fs::File::create(stl_path)
        .map_err(|error| AppError::persistence(format!("STL create failed: {error}")))?;
    output
        .write_all(&[0; 80])
        .map_err(|error| AppError::persistence(error.to_string()))?;
    output
        .write_all(&(triangles.len() as u32).to_le_bytes())
        .map_err(|error| AppError::persistence(error.to_string()))?;
    let mut minimum = [f64::INFINITY; 3];
    let mut maximum = [f64::NEG_INFINITY; 3];
    for triangle in &triangles {
        for value in [0.0_f32; 3] {
            output
                .write_all(&value.to_le_bytes())
                .map_err(|error| AppError::persistence(error.to_string()))?;
        }
        for vertex in triangle {
            for axis in 0..3 {
                minimum[axis] = minimum[axis].min(vertex[axis]);
                maximum[axis] = maximum[axis].max(vertex[axis]);
                output
                    .write_all(&(vertex[axis] as f32).to_le_bytes())
                    .map_err(|error| AppError::persistence(error.to_string()))?;
            }
        }
        output
            .write_all(&0_u16.to_le_bytes())
            .map_err(|error| AppError::persistence(error.to_string()))?;
    }
    Ok(ObjConversion {
        bounds_mm: [
            maximum[0] - minimum[0],
            maximum[1] - minimum[1],
            maximum[2] - minimum[2],
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn obj_conversion_scales_to_mm_and_validates_through_mesh_asset() {
        let root =
            std::env::temp_dir().join(format!("ecky-reconstruction-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let obj = root.join("model.obj");
        let stl = root.join("model.stl");
        std::fs::write(&obj, "v 0 0 0\nv 1 0 0\nv 0 2 0\nf 1 2 3\n").unwrap();
        let result = convert_obj_to_stl(&obj, &stl, 1_000.0).unwrap();
        assert_eq!(result.bounds_mm, [1_000.0, 2_000.0, 0.0]);
        let asset = MeshAsset::generated("test", Some("model"), &stl).unwrap();
        assert_eq!(asset.stl_path(), stl);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn cancellation_is_observable_before_provider_work() {
        let cancellation = ReconstructionCancellation::default();
        assert!(!cancellation.is_cancelled());
        cancellation.cancel();
        assert!(cancellation.is_cancelled());
    }
}
