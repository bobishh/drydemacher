use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use ecky_fem::{
    FemElementKind, FemFaceTarget, FemMeshControl, FemMeshingEvidence, FemPoint3,
    FemRuntimeIdentity, FemVolumeMesh, FemVolumeMeshInput, FEM_SCHEMA_VERSION,
};
use sha2::{Digest, Sha256};

use crate::contracts::{AppError, AppResult};
use crate::ecky_cad_host::analysis_boundary::AnalysisBoundarySurface;
use crate::netgen_mesher::{run_netgen_exact_brep, NetgenRuntimeIdentity};

const GMSH_ADAPTER_PROTOCOL_VERSION: u32 = 1;
const MAX_GMSH_THREADS: u32 = 64;
const DIAGNOSTIC_LIMIT_BYTES: u64 = 64 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GmshRuntimeIdentity {
    pub executable_path: PathBuf,
    pub version: String,
    pub executable_sha256: String,
    pub platform: String,
    pub architecture: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExactBrepMesherRuntime {
    pub gmsh: GmshRuntimeIdentity,
    pub netgen: Option<NetgenRuntimeIdentity>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GmshBrepFaceSignature {
    pub face_index: u32,
    pub area_mm2: f64,
    pub center_mm: [f64; 3],
}

#[derive(Debug, Clone, PartialEq)]
pub struct GmshMesherControl {
    pub global_size_mm: f64,
    pub minimum_scaled_jacobian: f64,
    pub maximum_face_area_relative_error: f64,
    pub maximum_face_centroid_deviation_mm: f64,
    pub thread_count: u32,
    pub maximum_nodes: u64,
    pub maximum_tet4_cells: u64,
    pub maximum_boundary_triangles: u64,
    pub maximum_result_bytes: u64,
    pub maximum_runtime_ms: u64,
    pub local_refinements: Vec<GmshLocalRefinement>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GmshLocalRefinement {
    pub face_group_indices: Vec<u32>,
    pub target_size_mm: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GmshBrepMeshRequest {
    pub schema_version: u32,
    pub request_id: String,
    pub step_path: PathBuf,
    pub step_sha256: String,
    pub source_geometry_digest: String,
    pub source_boundary_digest: String,
    pub face_signatures: Vec<GmshBrepFaceSignature>,
    pub face_group_targets: Vec<FemFaceTarget>,
    pub required_face_group_indices: Vec<u32>,
    pub control: GmshMesherControl,
}

impl GmshBrepMeshRequest {
    #[allow(clippy::too_many_arguments)]
    pub fn from_analysis_boundary(
        request_id: impl Into<String>,
        step_path: PathBuf,
        boundary: &AnalysisBoundarySurface,
        mesh_control: &FemMeshControl,
        minimum_scaled_jacobian: f64,
        maximum_runtime_ms: u64,
        thread_count: u32,
        required_face_targets: &[FemFaceTarget],
    ) -> AppResult<Self> {
        mesh_control
            .validate()
            .map_err(|error| AppError::validation(format!("FEM mesh control rejected: {error}")))?;
        if mesh_control.element_kind != FemElementKind::Tet4 {
            return Err(AppError::validation(
                "Gmsh HXT adapter accepts only Tet4 mesh controls.",
            ));
        }
        if !boundary.evidence.closed
            || !boundary.evidence.manifold
            || !boundary.evidence.positive_volume
            || boundary.evidence.component_count != 1
        {
            return Err(AppError::validation(format!(
                "Gmsh HXT requires one closed manifold positive-volume exact BRep; closed={}, manifold={}, positiveVolume={}, components={}.",
                boundary.evidence.closed,
                boundary.evidence.manifold,
                boundary.evidence.positive_volume,
                boundary.evidence.component_count
            )));
        }
        let face_group_targets = boundary
            .face_groups
            .iter()
            .enumerate()
            .map(|(index, group)| {
                Ok(FemFaceTarget {
                    schema_version: FEM_SCHEMA_VERSION,
                    part_id: group.part_id.clone(),
                    canonical_target_id: group.canonical_target_id.clone(),
                    durable_target_id: group.durable_target_id.clone().ok_or_else(|| {
                        AppError::validation(format!(
                            "Gmsh boundary face group {index} lacks durableTargetId."
                        ))
                    })?,
                    source_geometry_digest: boundary.source_geometry_digest.clone(),
                })
            })
            .collect::<AppResult<Vec<_>>>()?;
        let mut face_centers = vec![[0.0; 3]; boundary.face_groups.len()];
        let mut tessellated_areas = vec![0.0; boundary.face_groups.len()];
        for (triangle, group) in boundary
            .triangles
            .iter()
            .zip(&boundary.triangle_face_group_indices)
        {
            let points = triangle.map(|node| {
                FemPoint3::new(
                    boundary.vertices[node as usize][0],
                    boundary.vertices[node as usize][1],
                    boundary.vertices[node as usize][2],
                )
            });
            let area = triangle_area(points);
            let group = *group as usize;
            tessellated_areas[group] += area;
            for axis in 0..3 {
                face_centers[group][axis] +=
                    area * (points[0].get(axis) + points[1].get(axis) + points[2].get(axis)) / 3.0;
            }
        }
        let face_signatures = boundary
            .face_groups
            .iter()
            .enumerate()
            .map(|(index, group)| {
                if tessellated_areas[index] <= 0.0 {
                    return Err(AppError::validation(format!(
                        "Gmsh source face group {index} has no exact-boundary triangles."
                    )));
                }
                Ok(GmshBrepFaceSignature {
                    face_index: index as u32,
                    area_mm2: group.area,
                    center_mm: face_centers[index].map(|value| value / tessellated_areas[index]),
                })
            })
            .collect::<AppResult<Vec<_>>>()?;
        let resolve_target = |target: &FemFaceTarget, purpose: &str| -> AppResult<u32> {
            let matches = face_group_targets
                .iter()
                .enumerate()
                .filter(|(_, candidate)| *candidate == target)
                .map(|(index, _)| index)
                .collect::<Vec<_>>();
            if matches.len() != 1 {
                return Err(AppError::validation(format!(
                    "Gmsh {purpose} face '{}' resolved to {} groups; expected one.",
                    target.durable_target_id,
                    matches.len()
                )));
            }
            u32::try_from(matches[0])
                .map_err(|_| AppError::validation("Gmsh face-group index exceeds u32 range."))
        };
        let mut required_face_group_indices = required_face_targets
            .iter()
            .map(|target| resolve_target(target, "required"))
            .collect::<AppResult<BTreeSet<_>>>()?;
        let mut local_refinements = Vec::new();
        for (index, refinement) in mesh_control.local_refinements.iter().enumerate() {
            let mut groups = BTreeSet::new();
            for face in &refinement.faces {
                let group = resolve_target(face, &format!("local refinement {index}"))?;
                groups.insert(group);
                required_face_group_indices.insert(group);
            }
            local_refinements.push(GmshLocalRefinement {
                face_group_indices: groups.into_iter().collect(),
                target_size_mm: refinement.size_mm,
            });
        }
        local_refinements.sort_by(|left, right| {
            left.target_size_mm
                .total_cmp(&right.target_size_mm)
                .then(left.face_group_indices.cmp(&right.face_group_indices))
        });
        let maximum_nodes_from_dofs = mesh_control.budgets.dofs / 3;
        let step_sha256 = sha256_file(&step_path)?;
        let request = Self {
            schema_version: FEM_SCHEMA_VERSION,
            request_id: request_id.into(),
            step_path,
            step_sha256,
            source_geometry_digest: boundary.source_geometry_digest.clone(),
            source_boundary_digest: boundary.content_digest.clone(),
            face_signatures,
            face_group_targets,
            required_face_group_indices: required_face_group_indices.into_iter().collect(),
            control: GmshMesherControl {
                global_size_mm: mesh_control.global_size_mm,
                minimum_scaled_jacobian,
                maximum_face_area_relative_error: 0.05,
                maximum_face_centroid_deviation_mm: mesh_control
                    .global_size_mm
                    .max(boundary.tessellation_policy.linear_deflection_mm * 4.0),
                thread_count,
                maximum_nodes: mesh_control.budgets.nodes.min(maximum_nodes_from_dofs),
                maximum_tet4_cells: mesh_control.budgets.tet4_cells,
                maximum_boundary_triangles: mesh_control.budgets.boundary_triangles,
                maximum_result_bytes: mesh_control.budgets.result_bytes,
                maximum_runtime_ms,
                local_refinements,
            },
        };
        validate_request(&request)?;
        Ok(request)
    }
}

#[derive(Debug)]
pub(crate) struct ParsedMsh2 {
    pub(crate) nodes: Vec<FemPoint3>,
    pub(crate) cells: Vec<[u32; 4]>,
    pub(crate) boundary_triangles: Vec<[u32; 3]>,
    pub(crate) boundary_face_group_indices: Vec<u32>,
}

pub fn probe_gmsh_runtime(executable: &Path) -> AppResult<GmshRuntimeIdentity> {
    let executable_path = resolve_executable(executable)?;
    let output = Command::new(&executable_path)
        .arg("--version")
        .output()
        .map_err(|error| {
            AppError::validation(format!(
                "Gmsh executable '{}' could not run: {error}",
                executable_path.display()
            ))
        })?;
    if !output.status.success() {
        return Err(AppError::validation(format!(
            "Gmsh --version failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let major = version
        .split(|character: char| !character.is_ascii_digit())
        .find(|token| !token.is_empty())
        .and_then(|token| token.parse::<u32>().ok())
        .ok_or_else(|| AppError::validation(format!("Gmsh version '{version}' is invalid.")))?;
    if major < 4 {
        return Err(AppError::validation(format!(
            "Gmsh {version} is unsupported; HXT runtime requires major version 4 or newer."
        )));
    }
    Ok(GmshRuntimeIdentity {
        executable_sha256: sha256_file(&executable_path)?,
        executable_path,
        version,
        platform: std::env::consts::OS.to_string(),
        architecture: std::env::consts::ARCH.to_string(),
    })
}

pub fn run_gmsh_hxt(
    runtime: &GmshRuntimeIdentity,
    request: &GmshBrepMeshRequest,
    scratch_dir: &Path,
    cancelled: &AtomicBool,
) -> AppResult<FemVolumeMesh> {
    validate_request(request)?;
    let observed_digest = sha256_file(&runtime.executable_path)?;
    if observed_digest != runtime.executable_sha256 {
        return Err(AppError::conflict(format!(
            "Gmsh executable changed after probe: observed {observed_digest}, expected {}.",
            runtime.executable_sha256
        )));
    }
    let observed_step_digest = sha256_file(&request.step_path)?;
    if observed_step_digest != request.step_sha256 {
        return Err(AppError::conflict(format!(
            "Exact STEP changed after request resolution: observed {observed_step_digest}, expected {}.",
            request.step_sha256
        )));
    }
    fs::create_dir_all(scratch_dir).map_err(|error| {
        AppError::persistence(format!(
            "Gmsh scratch '{}' could not be created: {error}",
            scratch_dir.display()
        ))
    })?;
    let safe_id = request
        .request_id
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
                character
            } else {
                '_'
            }
        })
        .collect::<String>();
    let mesh_path = scratch_dir.join(format!("{safe_id}.msh"));
    let geo_path = scratch_dir.join(format!("{safe_id}.geo"));
    let stdout_path = scratch_dir.join(format!("{safe_id}.stdout"));
    let stderr_path = scratch_dir.join(format!("{safe_id}.stderr"));
    let _scratch_files = ScratchFiles([
        mesh_path.clone(),
        geo_path.clone(),
        stdout_path.clone(),
        stderr_path.clone(),
    ]);
    let stdout = File::create(&stdout_path).map_err(|error| {
        AppError::persistence(format!("Gmsh stdout file could not be created: {error}"))
    })?;
    let stderr = File::create(&stderr_path).map_err(|error| {
        AppError::persistence(format!("Gmsh stderr file could not be created: {error}"))
    })?;
    let minimum_size_mm = request
        .control
        .local_refinements
        .iter()
        .map(|refinement| refinement.target_size_mm)
        .fold(request.control.global_size_mm, f64::min);
    let mut geo = format!(
        "SetFactory(\"OpenCASCADE\");\nMerge \"{}\";\nMesh.Algorithm3D = 10;\nGeneral.NumThreads = {};\nMesh.MaxNumThreads3D = {};\nMesh.MeshSizeMin = {};\nMesh.MeshSizeMax = {};\nMesh.MeshSizeFromCurvature = 0;\nMesh.Binary = 0;\nMesh.MshFileVersion = 2.2;\nMesh.SaveAll = 1;\n",
        escape_gmsh_string(&request.step_path),
        request.control.thread_count,
        request.control.thread_count,
        minimum_size_mm,
        request.control.global_size_mm,
    );
    for signature in &request.face_signatures {
        let entity = signature.face_index + 1;
        geo.push_str(&format!(
            "eckyFaceCenter{entity}[] = CenterOfMass Surface {{{entity}}};\nPrintf(\"ECKY_FACE {entity} %.17g %.17g %.17g %.17g\", Mass Surface {{{entity}}}, eckyFaceCenter{entity}[0], eckyFaceCenter{entity}[1], eckyFaceCenter{entity}[2]);\n"
        ));
    }
    for refinement in &request.control.local_refinements {
        let surfaces = refinement
            .face_group_indices
            .iter()
            .map(|index| (index + 1).to_string())
            .collect::<Vec<_>>()
            .join(",");
        geo.push_str(&format!(
            "MeshSize {{ PointsOf{{ Surface{{{surfaces}}}; }} }} = {};\n",
            refinement.target_size_mm
        ));
    }
    fs::write(&geo_path, geo).map_err(|error| {
        AppError::persistence(format!("Gmsh control file could not be created: {error}"))
    })?;
    let thread_count = request.control.thread_count.to_string();
    let mut child = Command::new(&runtime.executable_path)
        .arg(&geo_path)
        .args(["-nt", thread_count.as_str()])
        .args(["-3", "-format", "msh2", "-save_all", "-o"])
        .arg(&mesh_path)
        .args(["-v", "3"])
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr))
        .spawn()
        .map_err(|error| {
            AppError::validation(format!(
                "Gmsh HXT executable '{}' could not start: {error}",
                runtime.executable_path.display()
            ))
        })?;
    let started = Instant::now();
    let timeout = Duration::from_millis(request.control.maximum_runtime_ms);
    let status = loop {
        if cancelled.load(Ordering::Relaxed) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(AppError::conflict(format!(
                "Gmsh HXT request '{}' was cancelled.",
                request.request_id
            )));
        }
        if started.elapsed() > timeout {
            let _ = child.kill();
            let _ = child.wait();
            return Err(AppError::validation(format!(
                "Gmsh HXT request '{}' exceeded {} ms runtime budget.",
                request.request_id, request.control.maximum_runtime_ms
            )));
        }
        match child.try_wait().map_err(|error| {
            AppError::validation(format!("Gmsh HXT status check failed: {error}"))
        })? {
            Some(status) => break status,
            None => thread::sleep(Duration::from_millis(20)),
        }
    };
    if !status.success() {
        let stderr = read_bounded_text(&stderr_path, DIAGNOSTIC_LIMIT_BYTES)?;
        let stdout = read_bounded_text(&stdout_path, DIAGNOSTIC_LIMIT_BYTES)?;
        return Err(AppError::validation(format!(
            "Gmsh HXT failed with {status}: stderr='{}'; stdout='{}'.",
            stderr.trim(),
            stdout.trim()
        )));
    }
    let stdout = read_bounded_text(&stdout_path, DIAGNOSTIC_LIMIT_BYTES)?;
    let exact_face_signatures =
        parse_exact_face_signatures(&stdout, request.face_signatures.len())?;
    validate_exact_face_mapping(&exact_face_signatures, request)?;
    let result_size = fs::metadata(&mesh_path)
        .map_err(|error| AppError::persistence(format!("Gmsh mesh metadata failed: {error}")))?
        .len();
    if result_size > request.control.maximum_result_bytes {
        return Err(AppError::validation(format!(
            "Gmsh mesh result uses {result_size} bytes; budget is {}.",
            request.control.maximum_result_bytes
        )));
    }
    let parsed = parse_msh2(&mesh_path, request.face_signatures.len(), &request.control)?;
    validate_budgets(&parsed, &request.control)?;
    validate_face_mapping(&parsed, request)?;
    let boundary_triangle_count = parsed.boundary_triangles.len() as u64;
    let face_group_count = u32::try_from(request.face_group_targets.len())
        .map_err(|_| AppError::validation("Gmsh face-group count exceeds u32 range."))?;
    let notice_digest = sha256_bytes(
        b"Gmsh external runtime; GPL-2.0-or-later; not linked or redistributed; https://gmsh.info/",
    );
    FemVolumeMesh::validate_and_canonicalize(FemVolumeMeshInput {
        schema_version: request.schema_version,
        nodes: parsed.nodes,
        cells: parsed.cells,
        boundary_triangles: parsed.boundary_triangles,
        boundary_face_group_indices: parsed.boundary_face_group_indices,
        face_group_count,
        face_group_targets: request.face_group_targets.clone(),
        source_boundary_digest: request.source_boundary_digest.clone(),
        mesher_identity: FemRuntimeIdentity {
            schema_version: FEM_SCHEMA_VERSION,
            platform: runtime.platform.clone(),
            architecture: runtime.architecture.clone(),
            library_name: "Gmsh HXT".to_string(),
            library_version: runtime.version.clone(),
            library_digest: runtime.executable_sha256.clone(),
            adapter_protocol_version: GMSH_ADAPTER_PROTOCOL_VERSION,
            supported_capabilities: vec![
                "exactBrepStep".to_string(),
                "hxt".to_string(),
                "parallelTet4".to_string(),
                "durableSurfaceEntities".to_string(),
                "isolatedProcess".to_string(),
            ],
            notice_digest,
        },
        meshing_evidence: FemMeshingEvidence {
            schema_version: FEM_SCHEMA_VERSION,
            source_triangle_count: boundary_triangle_count,
            inserted_source_triangle_count: boundary_triangle_count,
            tagged_boundary_triangle_count: boundary_triangle_count,
            maximum_boundary_deviation_mm: 0.0,
            discarded_tet4_component_count: 0,
            discarded_tet4_cell_count: 0,
            discarded_low_quality_tet4_cell_count: 0,
            deterministic_thread_count: request.control.thread_count,
        },
        minimum_scaled_jacobian: request.control.minimum_scaled_jacobian,
    })
    .map_err(|error| AppError::validation(format!("Gmsh HXT volume mesh rejected: {error}")))
}

pub fn run_exact_brep_mesher(
    runtime: &ExactBrepMesherRuntime,
    request: &GmshBrepMeshRequest,
    scratch_dir: &Path,
    cancelled: &AtomicBool,
) -> AppResult<FemVolumeMesh> {
    validate_request(request)?;
    let observed_gmsh_digest = sha256_file(&runtime.gmsh.executable_path)?;
    if observed_gmsh_digest != runtime.gmsh.executable_sha256 {
        return Err(AppError::conflict(format!(
            "Gmsh executable changed after probe: observed {observed_gmsh_digest}, expected {}.",
            runtime.gmsh.executable_sha256
        )));
    }
    let observed_step_digest = sha256_file(&request.step_path)?;
    if observed_step_digest != request.step_sha256 {
        return Err(AppError::conflict(format!(
            "Exact STEP changed after request resolution: observed {observed_step_digest}, expected {}.",
            request.step_sha256
        )));
    }
    let started = Instant::now();
    match run_gmsh_hxt(&runtime.gmsh, request, scratch_dir, cancelled) {
        Ok(mesh) => Ok(mesh),
        Err(hxt_error) => {
            if cancelled.load(Ordering::Relaxed) {
                return Err(hxt_error);
            }
            let Some(netgen) = runtime.netgen.as_ref() else {
                return Err(AppError::validation(format!(
                    "Gmsh HXT failed and Netgen exact-BRep fallback is unavailable: {hxt_error}"
                )));
            };
            let elapsed_ms = started.elapsed().as_millis().min(u64::MAX as u128) as u64;
            let remaining_ms = request
                .control
                .maximum_runtime_ms
                .saturating_sub(elapsed_ms);
            if remaining_ms == 0 {
                return Err(AppError::validation(format!(
                    "Gmsh HXT exhausted the meshing runtime budget before Netgen fallback: {hxt_error}"
                )));
            }
            let mut fallback_request = request.clone();
            fallback_request.control.maximum_runtime_ms = remaining_ms;
            run_netgen_exact_brep(netgen, &fallback_request, scratch_dir, cancelled).map_err(
                |netgen_error| {
                    AppError::validation(format!(
                        "Exact-BRep meshing failed in both backends. HXT: {hxt_error}. Netgen: {netgen_error}"
                    ))
                },
            )
        }
    }
}

struct ScratchFiles([PathBuf; 4]);

impl Drop for ScratchFiles {
    fn drop(&mut self) {
        for path in &self.0 {
            let _ = fs::remove_file(path);
        }
    }
}

pub(crate) fn validate_request(request: &GmshBrepMeshRequest) -> AppResult<()> {
    if request.schema_version != FEM_SCHEMA_VERSION {
        return Err(AppError::validation(
            "Gmsh request schemaVersion is unsupported.",
        ));
    }
    if request.request_id.trim().is_empty()
        || request.source_geometry_digest.trim().is_empty()
        || request.source_boundary_digest.trim().is_empty()
        || request.step_sha256.trim().is_empty()
    {
        return Err(AppError::validation(
            "Gmsh request requires requestId and source digests.",
        ));
    }
    if !request.step_path.is_file() {
        return Err(AppError::validation(format!(
            "Gmsh exact STEP '{}' does not exist.",
            request.step_path.display()
        )));
    }
    if request.face_signatures.is_empty()
        || request.face_signatures.len() != request.face_group_targets.len()
    {
        return Err(AppError::validation(
            "Gmsh face signatures and durable targets must have equal positive cardinality.",
        ));
    }
    for (index, signature) in request.face_signatures.iter().enumerate() {
        if signature.face_index != index as u32
            || !signature.area_mm2.is_finite()
            || signature.area_mm2 <= 0.0
            || signature.center_mm.iter().any(|value| !value.is_finite())
        {
            return Err(AppError::validation(format!(
                "Gmsh face signature {index} is invalid or out of order."
            )));
        }
    }
    if request.required_face_group_indices.is_empty()
        || request
            .required_face_group_indices
            .iter()
            .any(|index| *index as usize >= request.face_signatures.len())
    {
        return Err(AppError::validation(
            "Gmsh required face groups are empty or out of range.",
        ));
    }
    let control = &request.control;
    if !control.global_size_mm.is_finite()
        || control.global_size_mm <= 0.0
        || !control.minimum_scaled_jacobian.is_finite()
        || control.minimum_scaled_jacobian <= 0.0
        || !control.maximum_face_area_relative_error.is_finite()
        || !(0.0..=0.25).contains(&control.maximum_face_area_relative_error)
        || !control.maximum_face_centroid_deviation_mm.is_finite()
        || control.maximum_face_centroid_deviation_mm <= 0.0
        || !(1..=MAX_GMSH_THREADS).contains(&control.thread_count)
        || control.maximum_nodes == 0
        || control.maximum_tet4_cells == 0
        || control.maximum_boundary_triangles == 0
        || control.maximum_result_bytes == 0
        || control.maximum_runtime_ms == 0
    {
        return Err(AppError::validation("Gmsh mesher controls are invalid."));
    }
    for (index, refinement) in control.local_refinements.iter().enumerate() {
        if refinement.face_group_indices.is_empty()
            || refinement
                .face_group_indices
                .iter()
                .any(|group| *group as usize >= request.face_signatures.len())
            || !refinement.target_size_mm.is_finite()
            || refinement.target_size_mm <= 0.0
            || refinement.target_size_mm >= control.global_size_mm
        {
            return Err(AppError::validation(format!(
                "Gmsh local refinement {index} is invalid."
            )));
        }
    }
    Ok(())
}

fn escape_gmsh_string(path: &Path) -> String {
    path.to_string_lossy()
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
}

pub(crate) fn validate_budgets(parsed: &ParsedMsh2, control: &GmshMesherControl) -> AppResult<()> {
    for (label, observed, maximum) in [
        ("nodes", parsed.nodes.len() as u64, control.maximum_nodes),
        (
            "Tet4 cells",
            parsed.cells.len() as u64,
            control.maximum_tet4_cells,
        ),
        (
            "boundary triangles",
            parsed.boundary_triangles.len() as u64,
            control.maximum_boundary_triangles,
        ),
    ] {
        if observed > maximum {
            return Err(AppError::validation(format!(
                "Gmsh {label} budget exceeded: observed {observed}, allowed {maximum}."
            )));
        }
    }
    Ok(())
}

pub(crate) fn validate_face_mapping(
    parsed: &ParsedMsh2,
    request: &GmshBrepMeshRequest,
) -> AppResult<()> {
    let mut seen = BTreeSet::new();
    for (_triangle, group) in parsed
        .boundary_triangles
        .iter()
        .zip(&parsed.boundary_face_group_indices)
    {
        let group = *group as usize;
        seen.insert(group as u32);
    }
    for required in &request.required_face_group_indices {
        if !seen.contains(required) {
            return Err(AppError::validation(format!(
                "Gmsh omitted required durable face group {required}."
            )));
        }
    }
    for index in 0..request.face_signatures.len() {
        if !seen.contains(&(index as u32)) {
            return Err(AppError::validation(format!(
                "Gmsh OCC surface {} has no boundary triangles.",
                index + 1
            )));
        }
    }
    Ok(())
}

pub(crate) fn validate_exact_face_mapping(
    exact: &[GmshBrepFaceSignature],
    request: &GmshBrepMeshRequest,
) -> AppResult<()> {
    if exact.len() != request.face_signatures.len() {
        return Err(AppError::validation(format!(
            "Gmsh exact OCC face count {} differs from durable face count {}.",
            exact.len(),
            request.face_signatures.len()
        )));
    }
    for (index, (observed, signature)) in exact.iter().zip(&request.face_signatures).enumerate() {
        if observed.face_index != signature.face_index {
            return Err(AppError::validation(format!(
                "Gmsh OCC surface order mismatch at durable face {index}."
            )));
        }
        let relative_area_error =
            (observed.area_mm2 - signature.area_mm2).abs() / signature.area_mm2.max(f64::EPSILON);
        let center_deviation = distance(observed.center_mm, signature.center_mm);
        if relative_area_error > request.control.maximum_face_area_relative_error
            || center_deviation > request.control.maximum_face_centroid_deviation_mm
        {
            return Err(AppError::validation(format!(
                "Gmsh OCC surface {} does not match durable face {}: area error={}, center deviation={} mm.",
                index + 1,
                signature.face_index,
                relative_area_error,
                center_deviation
            )));
        }
    }
    Ok(())
}

pub(crate) fn parse_exact_face_signatures(
    stdout: &str,
    expected_count: usize,
) -> AppResult<Vec<GmshBrepFaceSignature>> {
    let mut signatures = Vec::new();
    for line in stdout.lines() {
        let Some(marker) = line.find("ECKY_FACE ") else {
            continue;
        };
        let fields = line[marker..].split_whitespace().collect::<Vec<_>>();
        if fields.len() < 6 {
            return Err(AppError::validation(format!(
                "Gmsh exact OCC face diagnostic is malformed: '{line}'."
            )));
        }
        let entity = parse_u64(fields[1], "exact OCC surface entity")?;
        if entity == 0 || entity as usize > expected_count {
            return Err(AppError::validation(format!(
                "Gmsh exact OCC surface entity {entity} is outside expected count {expected_count}."
            )));
        }
        signatures.push(GmshBrepFaceSignature {
            face_index: (entity - 1) as u32,
            area_mm2: parse_f64(fields[2], "exact OCC face area")?,
            center_mm: [
                parse_f64(fields[3], "exact OCC face center x")?,
                parse_f64(fields[4], "exact OCC face center y")?,
                parse_f64(fields[5], "exact OCC face center z")?,
            ],
        });
    }
    signatures.sort_by_key(|signature| signature.face_index);
    if signatures.len() != expected_count
        || signatures
            .iter()
            .enumerate()
            .any(|(index, signature)| signature.face_index != index as u32)
    {
        return Err(AppError::validation(format!(
            "Gmsh emitted {} unique exact OCC face signatures; expected {expected_count}.",
            signatures.len()
        )));
    }
    Ok(signatures)
}

pub(crate) fn parse_msh2(
    path: &Path,
    face_group_count: usize,
    control: &GmshMesherControl,
) -> AppResult<ParsedMsh2> {
    let file = File::open(path)
        .map_err(|error| AppError::persistence(format!("Gmsh mesh could not be read: {error}")))?;
    let mut lines = BufReader::new(file).lines();
    let mut nodes_by_tag = BTreeMap::<u64, FemPoint3>::new();
    let mut raw_cells = Vec::<[u64; 4]>::new();
    let mut raw_triangles = Vec::<([u64; 3], u32)>::new();
    while let Some(line) = next_line(&mut lines)? {
        match line.as_str() {
            "$MeshFormat" => {
                let format = next_required(&mut lines, "mesh format")?;
                let fields = format.split_whitespace().collect::<Vec<_>>();
                if fields.len() != 3 || !fields[0].starts_with("2.") || fields[1] != "0" {
                    return Err(AppError::validation(format!(
                        "Gmsh output must be ASCII MSH2, got '{format}'."
                    )));
                }
            }
            "$Nodes" => {
                let count = parse_count(&next_required(&mut lines, "node count")?, "node")?;
                if count as u64 > control.maximum_nodes {
                    return Err(AppError::validation(format!(
                        "Gmsh nodes budget exceeded before allocation: observed {count}, allowed {}.",
                        control.maximum_nodes
                    )));
                }
                for _ in 0..count {
                    let row = next_required(&mut lines, "node row")?;
                    let fields = row.split_whitespace().collect::<Vec<_>>();
                    if fields.len() != 4 {
                        return Err(AppError::validation("Gmsh node row is malformed."));
                    }
                    let tag = parse_u64(fields[0], "node tag")?;
                    let point = FemPoint3::new(
                        parse_f64(fields[1], "node x")?,
                        parse_f64(fields[2], "node y")?,
                        parse_f64(fields[3], "node z")?,
                    );
                    if nodes_by_tag.insert(tag, point).is_some() {
                        return Err(AppError::validation("Gmsh node tag is duplicate."));
                    }
                }
            }
            "$Elements" => {
                let count = parse_count(&next_required(&mut lines, "element count")?, "element")?;
                for _ in 0..count {
                    let row = next_required(&mut lines, "element row")?;
                    let fields = row.split_whitespace().collect::<Vec<_>>();
                    if fields.len() < 4 {
                        return Err(AppError::validation("Gmsh element row is malformed."));
                    }
                    let element_type = parse_u64(fields[1], "element type")?;
                    let tag_count = parse_count(fields[2], "element tag")?;
                    let node_offset = 3usize.checked_add(tag_count).ok_or_else(|| {
                        AppError::validation("Gmsh element tag count overflowed.")
                    })?;
                    match element_type {
                        2 => {
                            if tag_count < 2 || fields.len() != node_offset + 3 {
                                return Err(AppError::validation(
                                    "Gmsh boundary triangle lacks geometric entity tag.",
                                ));
                            }
                            let entity = parse_u64(fields[4], "surface entity")?;
                            if entity == 0 || entity as usize > face_group_count {
                                return Err(AppError::validation(format!(
                                    "Gmsh surface entity {entity} is outside durable face count {face_group_count}."
                                )));
                            }
                            raw_triangles.push((
                                [
                                    parse_u64(fields[node_offset], "triangle node")?,
                                    parse_u64(fields[node_offset + 1], "triangle node")?,
                                    parse_u64(fields[node_offset + 2], "triangle node")?,
                                ],
                                (entity - 1) as u32,
                            ));
                            if raw_triangles.len() as u64 > control.maximum_boundary_triangles {
                                return Err(AppError::validation(format!(
                                    "Gmsh boundary triangles budget exceeded while parsing: allowed {}.",
                                    control.maximum_boundary_triangles
                                )));
                            }
                        }
                        4 => {
                            if fields.len() != node_offset + 4 {
                                return Err(AppError::validation("Gmsh Tet4 row is malformed."));
                            }
                            raw_cells.push([
                                parse_u64(fields[node_offset], "Tet4 node")?,
                                parse_u64(fields[node_offset + 1], "Tet4 node")?,
                                parse_u64(fields[node_offset + 2], "Tet4 node")?,
                                parse_u64(fields[node_offset + 3], "Tet4 node")?,
                            ]);
                            if raw_cells.len() as u64 > control.maximum_tet4_cells {
                                return Err(AppError::validation(format!(
                                    "Gmsh Tet4 cells budget exceeded while parsing: allowed {}.",
                                    control.maximum_tet4_cells
                                )));
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
    if nodes_by_tag.is_empty() || raw_cells.is_empty() || raw_triangles.is_empty() {
        return Err(AppError::validation(
            "Gmsh output is missing nodes, Tet4 cells, or boundary triangles.",
        ));
    }
    let mut node_remap = BTreeMap::new();
    let mut nodes = Vec::with_capacity(nodes_by_tag.len());
    for (index, (tag, point)) in nodes_by_tag.into_iter().enumerate() {
        let index = u32::try_from(index)
            .map_err(|_| AppError::validation("Gmsh node count exceeds u32 range."))?;
        node_remap.insert(tag, index);
        nodes.push(point);
    }
    let remap = |tag: u64| {
        node_remap.get(&tag).copied().ok_or_else(|| {
            AppError::validation(format!("Gmsh element references missing node {tag}."))
        })
    };
    let cells = raw_cells
        .into_iter()
        .map(|cell| {
            Ok([
                remap(cell[0])?,
                remap(cell[1])?,
                remap(cell[2])?,
                remap(cell[3])?,
            ])
        })
        .collect::<AppResult<Vec<_>>>()?;
    let mut boundary_triangles = Vec::with_capacity(raw_triangles.len());
    let mut boundary_face_group_indices = Vec::with_capacity(raw_triangles.len());
    for (triangle, group) in raw_triangles {
        boundary_triangles.push([
            remap(triangle[0])?,
            remap(triangle[1])?,
            remap(triangle[2])?,
        ]);
        boundary_face_group_indices.push(group);
    }
    Ok(ParsedMsh2 {
        nodes,
        cells,
        boundary_triangles,
        boundary_face_group_indices,
    })
}

fn next_line<I>(lines: &mut I) -> AppResult<Option<String>>
where
    I: Iterator<Item = std::io::Result<String>>,
{
    lines
        .next()
        .transpose()
        .map_err(|error| AppError::persistence(format!("Gmsh mesh read failed: {error}")))
}

fn next_required<I>(lines: &mut I, label: &str) -> AppResult<String>
where
    I: Iterator<Item = std::io::Result<String>>,
{
    next_line(lines)?
        .ok_or_else(|| AppError::validation(format!("Gmsh output ended before {label}.")))
}

fn parse_count(value: &str, label: &str) -> AppResult<usize> {
    value
        .parse::<usize>()
        .map_err(|_| AppError::validation(format!("Gmsh {label} count '{value}' is invalid.")))
}

fn parse_u64(value: &str, label: &str) -> AppResult<u64> {
    value
        .parse::<u64>()
        .map_err(|_| AppError::validation(format!("Gmsh {label} '{value}' is invalid.")))
}

fn parse_f64(value: &str, label: &str) -> AppResult<f64> {
    let parsed = value
        .parse::<f64>()
        .map_err(|_| AppError::validation(format!("Gmsh {label} '{value}' is invalid.")))?;
    if !parsed.is_finite() {
        return Err(AppError::validation(format!("Gmsh {label} is non-finite.")));
    }
    Ok(parsed)
}

fn triangle_area(points: [FemPoint3; 3]) -> f64 {
    let a = [
        points[1].x_mm - points[0].x_mm,
        points[1].y_mm - points[0].y_mm,
        points[1].z_mm - points[0].z_mm,
    ];
    let b = [
        points[2].x_mm - points[0].x_mm,
        points[2].y_mm - points[0].y_mm,
        points[2].z_mm - points[0].z_mm,
    ];
    let cross = [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ];
    0.5 * cross.iter().map(|value| value * value).sum::<f64>().sqrt()
}

fn distance(left: [f64; 3], right: [f64; 3]) -> f64 {
    left.iter()
        .zip(right)
        .map(|(left, right)| (left - right) * (left - right))
        .sum::<f64>()
        .sqrt()
}

fn resolve_executable(executable: &Path) -> AppResult<PathBuf> {
    if executable.components().count() > 1 || executable.is_absolute() {
        return executable.canonicalize().map_err(|error| {
            AppError::validation(format!(
                "Gmsh executable '{}' could not be resolved: {error}",
                executable.display()
            ))
        });
    }
    let path = std::env::var_os("PATH").ok_or_else(|| {
        AppError::validation("Gmsh executable lookup failed because PATH is unset.")
    })?;
    std::env::split_paths(&path)
        .map(|directory| directory.join(executable))
        .find(|candidate| candidate.is_file())
        .and_then(|candidate| candidate.canonicalize().ok())
        .ok_or_else(|| {
            AppError::validation(format!(
                "Gmsh executable '{}' was not found on PATH.",
                executable.display()
            ))
        })
}

pub fn sha256_file(path: &Path) -> AppResult<String> {
    let mut file = File::open(path).map_err(|error| {
        AppError::persistence(format!(
            "Gmsh executable '{}' could not be hashed: {error}",
            path.display()
        ))
    })?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let count = file.read(&mut buffer).map_err(|error| {
            AppError::persistence(format!("Gmsh executable hash read failed: {error}"))
        })?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(format!("sha256:{:x}", hasher.finalize()))
}

pub(crate) fn sha256_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("sha256:{:x}", hasher.finalize())
}

pub(crate) fn read_bounded_text(path: &Path, maximum_bytes: u64) -> AppResult<String> {
    let file = File::open(path).map_err(|error| {
        AppError::persistence(format!("Gmsh diagnostic could not be read: {error}"))
    })?;
    let mut bytes = Vec::new();
    file.take(maximum_bytes)
        .read_to_end(&mut bytes)
        .map_err(|error| AppError::persistence(format!("Gmsh diagnostic read failed: {error}")))?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

trait PointAxis {
    fn get(&self, axis: usize) -> f64;
}

impl PointAxis for FemPoint3 {
    fn get(&self, axis: usize) -> f64 {
        match axis {
            0 => self.x_mm,
            1 => self.y_mm,
            _ => self.z_mm,
        }
    }
}
