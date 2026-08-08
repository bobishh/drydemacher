use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::{BufReader, Read};
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::contracts::{AppError, AppResult};
use crate::ecky_cad_host::analysis_boundary::AnalysisBoundarySurface;
use ecky_fem::{
    FemElementKind, FemMeshControl, FemMeshingEvidence, FemPoint3, FemRuntimeIdentity,
    FemVolumeMesh, FemVolumeMeshInput, FEM_SCHEMA_VERSION,
};

pub const FTETWILD_RUNTIME_SCHEMA_VERSION: u32 = 1;
pub const FTETWILD_WORKER_PROTOCOL: &str = "ecky-ftetwild-worker-v1";
const FTETWILD_RUNTIME_NAME: &str = "fTetWild";
static WORKER_RUN_NONCE: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FTetWildRuntimeRequirement {
    pub runtime_version: String,
    pub source_revision: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FTetWildRuntimeCapabilities {
    pub structured_arrays: bool,
    pub tet4: bool,
    pub wide_surface_tags: bool,
    pub isolated_worker: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FTetWildRuntimeIdentity {
    pub runtime_name: String,
    pub runtime_version: String,
    pub source_revision: String,
    pub platform: String,
    pub arch: String,
    pub worker_protocol: String,
    pub executable_path: PathBuf,
    pub runtime_library_paths: Vec<PathBuf>,
    pub executable_sha256: String,
    pub source_sha256: String,
    pub license_sha256: String,
    pub notice_sha256: String,
    pub transitive_license_inventory_sha256: String,
    pub capabilities: FTetWildRuntimeCapabilities,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct FTetWildRuntimeManifest {
    schema_version: u32,
    runtime_name: String,
    runtime_version: String,
    source_revision: String,
    platform: String,
    arch: String,
    worker_protocol: String,
    executable: ManifestFile,
    source_archive: ManifestFile,
    license: ManifestFile,
    notice: ManifestFile,
    transitive_license_inventory: ManifestFile,
    runtime_libraries: Vec<ManifestFile>,
    capabilities: FTetWildRuntimeCapabilities,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ManifestFile {
    path: PathBuf,
    sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FTetWildWorkerControl {
    pub element_order: u8,
    pub target_edge_length_mm: f64,
    pub envelope_mm: f64,
    pub minimum_scaled_jacobian: f64,
    pub deterministic_thread_count: u32,
    pub allow_hole_filling: bool,
    pub maximum_boundary_triangles: u64,
    pub maximum_nodes: u64,
    pub maximum_tet4_cells: u64,
    pub maximum_result_bytes: u64,
    pub maximum_runtime_ms: u64,
    pub local_refinements: Vec<FTetWildWorkerLocalRefinement>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FTetWildWorkerLocalRefinement {
    pub face_group_indices: Vec<u32>,
    pub target_edge_length_mm: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FTetWildWorkerRequest {
    pub schema_version: u32,
    pub worker_protocol: String,
    pub request_id: String,
    pub source_boundary_digest: String,
    pub vertices_mm: Vec<f64>,
    pub triangles: Vec<u32>,
    pub triangle_face_group_indices: Vec<u32>,
    pub face_group_count: u32,
    pub face_group_targets: Vec<ecky_fem::FemFaceTarget>,
    pub control: FTetWildWorkerControl,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct FTetWildWorkerResponse {
    schema_version: u32,
    worker_protocol: String,
    request_id: String,
    nodes_mm: Vec<f64>,
    tet4_cells: Vec<u32>,
    boundary_triangles: Vec<u32>,
    boundary_face_group_indices: Vec<u32>,
    face_group_count: u32,
    insertion_count: u64,
    maximum_boundary_deviation_mm: f64,
    thread_count: u32,
}

impl FTetWildWorkerRequest {
    pub fn from_analysis_boundary(
        request_id: impl Into<String>,
        boundary: &AnalysisBoundarySurface,
        mesh_control: &FemMeshControl,
        envelope_mm: f64,
        minimum_scaled_jacobian: f64,
        maximum_runtime_ms: u64,
    ) -> AppResult<Self> {
        mesh_control
            .validate()
            .map_err(|error| AppError::validation(format!("FEM mesh control rejected: {error}")))?;
        if mesh_control.element_kind != FemElementKind::Tet4 {
            return Err(AppError::validation(
                "fTetWild adapter accepts only Tet4 mesh controls.",
            ));
        }
        if !boundary.evidence.closed
            || !boundary.evidence.manifold
            || !boundary.evidence.positive_volume
            || boundary.evidence.component_count != 1
        {
            return Err(AppError::validation(format!(
                "fTetWild adapter requires one closed manifold positive-volume analysis boundary; closed={}, manifold={}, positiveVolume={}, components={}.",
                boundary.evidence.closed,
                boundary.evidence.manifold,
                boundary.evidence.positive_volume,
                boundary.evidence.component_count
            )));
        }
        if boundary.content_digest.trim().is_empty() {
            return Err(AppError::validation(
                "fTetWild adapter analysis boundary is missing contentDigest.",
            ));
        }
        let face_group_count = u32::try_from(boundary.face_groups.len()).map_err(|_| {
            AppError::validation("fTetWild adapter face-group count exceeds u32 range.")
        })?;
        let face_group_targets = boundary
            .face_groups
            .iter()
            .enumerate()
            .map(|(index, group)| {
                let durable_target_id = group.durable_target_id.clone().ok_or_else(|| {
                    AppError::validation(format!(
                        "fTetWild boundary face group {index} '{}' is missing durableTargetId.",
                        group.canonical_target_id
                    ))
                })?;
                Ok(ecky_fem::FemFaceTarget {
                    schema_version: FEM_SCHEMA_VERSION,
                    part_id: group.part_id.clone(),
                    canonical_target_id: group.canonical_target_id.clone(),
                    durable_target_id,
                    source_geometry_digest: boundary.source_geometry_digest.clone(),
                })
            })
            .collect::<AppResult<Vec<_>>>()?;
        let mut local_refinements = Vec::with_capacity(mesh_control.local_refinements.len());
        for (refinement_index, refinement) in mesh_control.local_refinements.iter().enumerate() {
            let mut face_group_indices = BTreeSet::new();
            for target in &refinement.faces {
                if target.part_id != boundary.part_id {
                    return Err(AppError::validation(format!(
                        "fTetWild local refinement {refinement_index} target part '{}' does not match boundary part '{}'.",
                        target.part_id, boundary.part_id
                    )));
                }
                if target.source_geometry_digest != boundary.source_geometry_digest {
                    return Err(AppError::validation(format!(
                        "fTetWild local refinement {refinement_index} target sourceGeometryDigest is stale."
                    )));
                }
                let matches = boundary
                    .face_groups
                    .iter()
                    .enumerate()
                    .filter(|(_, group)| {
                        group.canonical_target_id == target.canonical_target_id
                            && group.durable_target_id.as_deref()
                                == Some(target.durable_target_id.as_str())
                    })
                    .map(|(index, _)| index)
                    .collect::<Vec<_>>();
                if matches.len() != 1 {
                    return Err(AppError::validation(format!(
                        "fTetWild local refinement {refinement_index} target '{}'/'{}' resolved to {} boundary groups; expected exactly one.",
                        target.canonical_target_id,
                        target.durable_target_id,
                        matches.len()
                    )));
                }
                face_group_indices.insert(u32::try_from(matches[0]).map_err(|_| {
                    AppError::validation("fTetWild local refinement group index exceeds u32 range.")
                })?);
            }
            local_refinements.push(FTetWildWorkerLocalRefinement {
                face_group_indices: face_group_indices.into_iter().collect(),
                target_edge_length_mm: refinement.size_mm,
            });
        }
        local_refinements.sort_by(|left, right| {
            left.target_edge_length_mm
                .total_cmp(&right.target_edge_length_mm)
                .then(left.face_group_indices.cmp(&right.face_group_indices))
        });
        let maximum_nodes_from_dofs = mesh_control.budgets.dofs / 3;
        let request = Self {
            schema_version: FEM_SCHEMA_VERSION,
            worker_protocol: FTETWILD_WORKER_PROTOCOL.to_string(),
            request_id: request_id.into(),
            source_boundary_digest: boundary.content_digest.clone(),
            vertices_mm: boundary
                .vertices
                .iter()
                .flat_map(|vertex| vertex.iter().copied())
                .collect(),
            triangles: boundary
                .triangles
                .iter()
                .flat_map(|triangle| triangle.iter().copied())
                .collect(),
            triangle_face_group_indices: boundary.triangle_face_group_indices.clone(),
            face_group_count,
            face_group_targets,
            control: FTetWildWorkerControl {
                element_order: 1,
                target_edge_length_mm: mesh_control.global_size_mm,
                envelope_mm,
                minimum_scaled_jacobian,
                deterministic_thread_count: 1,
                allow_hole_filling: false,
                maximum_boundary_triangles: mesh_control.budgets.boundary_triangles,
                maximum_nodes: mesh_control.budgets.nodes.min(maximum_nodes_from_dofs),
                maximum_tet4_cells: mesh_control.budgets.tet4_cells,
                maximum_result_bytes: mesh_control.budgets.result_bytes,
                maximum_runtime_ms,
                local_refinements,
            },
        };
        request.validate()?;
        Ok(request)
    }

    pub fn validate(&self) -> AppResult<()> {
        if self.schema_version != FEM_SCHEMA_VERSION {
            return Err(AppError::validation(format!(
                "fTetWild request schemaVersion {} is unsupported.",
                self.schema_version
            )));
        }
        require_equal(
            "worker protocol",
            &self.worker_protocol,
            FTETWILD_WORKER_PROTOCOL,
        )?;
        if self.request_id.trim().is_empty() || self.source_boundary_digest.trim().is_empty() {
            return Err(AppError::validation(
                "fTetWild request requires requestId and sourceBoundaryDigest.",
            ));
        }
        if self.vertices_mm.len() < 12 || self.vertices_mm.len() % 3 != 0 {
            return Err(AppError::validation(format!(
                "fTetWild request verticesMm length {} must be a multiple of 3 with at least four vertices.",
                self.vertices_mm.len()
            )));
        }
        if self.vertices_mm.iter().any(|value| !value.is_finite()) {
            return Err(AppError::validation(
                "fTetWild request verticesMm contains a non-finite coordinate.",
            ));
        }
        if self.triangles.len() < 12 || self.triangles.len() % 3 != 0 {
            return Err(AppError::validation(format!(
                "fTetWild request triangles length {} must be a multiple of 3 with at least four facets.",
                self.triangles.len()
            )));
        }
        let triangle_count = self.triangles.len() / 3;
        if self.triangle_face_group_indices.len() != triangle_count {
            return Err(AppError::validation(format!(
                "fTetWild request triangle/group cardinality mismatch: triangles={triangle_count}, groups={}.",
                self.triangle_face_group_indices.len()
            )));
        }
        if self.face_group_count == 0 {
            return Err(AppError::validation(
                "fTetWild request faceGroupCount must be positive.",
            ));
        }
        if self.face_group_targets.len() != self.face_group_count as usize {
            return Err(AppError::validation(format!(
                "fTetWild request faceGroupTargets cardinality {} differs from faceGroupCount {}.",
                self.face_group_targets.len(),
                self.face_group_count
            )));
        }
        for target in &self.face_group_targets {
            target.validate().map_err(|error| {
                AppError::validation(format!(
                    "fTetWild request face-group target rejected: {error}"
                ))
            })?;
        }
        validate_worker_control(&self.control)?;
        for (refinement_index, refinement) in self.control.local_refinements.iter().enumerate() {
            if let Some(group) = refinement
                .face_group_indices
                .iter()
                .find(|group| **group >= self.face_group_count)
            {
                return Err(AppError::validation(format!(
                    "fTetWild local refinement {refinement_index} group {group} exceeds faceGroupCount {}.",
                    self.face_group_count
                )));
            }
        }
        if triangle_count as u64 > self.control.maximum_boundary_triangles {
            return Err(AppError::validation(format!(
                "fTetWild request boundary triangle budget exceeded: observed {triangle_count}, allowed {}.",
                self.control.maximum_boundary_triangles
            )));
        }
        let vertex_count = self.vertices_mm.len() / 3;
        if vertex_count as u64 > self.control.maximum_nodes {
            return Err(AppError::validation(format!(
                "fTetWild request node budget exceeded before meshing: observed {vertex_count}, allowed {}.",
                self.control.maximum_nodes
            )));
        }
        let mut seen_groups = BTreeSet::new();
        let mut edges: BTreeMap<(u32, u32), Vec<bool>> = BTreeMap::new();
        let mut seen_triangles = BTreeSet::new();
        for (triangle_index, (triangle, group)) in self
            .triangles
            .chunks_exact(3)
            .zip(&self.triangle_face_group_indices)
            .enumerate()
        {
            let triangle = [triangle[0], triangle[1], triangle[2]];
            if triangle.iter().any(|index| *index as usize >= vertex_count) {
                return Err(AppError::validation(format!(
                    "fTetWild request triangle {triangle_index} contains an out-of-range vertex index."
                )));
            }
            let mut key = triangle;
            key.sort_unstable();
            if key.windows(2).any(|pair| pair[0] == pair[1]) {
                return Err(AppError::validation(format!(
                    "fTetWild request triangle {triangle_index} repeats a vertex."
                )));
            }
            if !seen_triangles.insert(key) {
                return Err(AppError::validation(format!(
                    "fTetWild request triangle {triangle_index} duplicates an earlier facet."
                )));
            }
            if *group >= self.face_group_count {
                return Err(AppError::validation(format!(
                    "fTetWild request triangle {triangle_index} group {group} exceeds faceGroupCount {}.",
                    self.face_group_count
                )));
            }
            seen_groups.insert(*group);
            for (from, to) in [
                (triangle[0], triangle[1]),
                (triangle[1], triangle[2]),
                (triangle[2], triangle[0]),
            ] {
                let edge = (from.min(to), from.max(to));
                edges.entry(edge).or_default().push(from == edge.0);
            }
        }
        if seen_groups.len() != self.face_group_count as usize {
            return Err(AppError::validation(format!(
                "fTetWild request face-group coverage mismatch: observed {}, declared {}.",
                seen_groups.len(),
                self.face_group_count
            )));
        }
        let boundary_edges = edges.values().filter(|owners| owners.len() == 1).count();
        let non_manifold_edges = edges.values().filter(|owners| owners.len() > 2).count();
        let winding_mismatches = edges
            .values()
            .filter(|owners| owners.len() == 2 && owners[0] == owners[1])
            .count();
        if boundary_edges != 0 || non_manifold_edges != 0 || winding_mismatches != 0 {
            return Err(AppError::validation(format!(
                "fTetWild request surface is not a closed oriented manifold: boundaryEdges={boundary_edges}, nonManifoldEdges={non_manifold_edges}, windingMismatches={winding_mismatches}."
            )));
        }
        Ok(())
    }
}

fn validate_worker_control(control: &FTetWildWorkerControl) -> AppResult<()> {
    if control.element_order != 1 {
        return Err(AppError::validation(format!(
            "fTetWild request elementOrder {} is unsupported; only Tet4 order 1 is allowed.",
            control.element_order
        )));
    }
    for (field, value) in [
        ("targetEdgeLengthMm", control.target_edge_length_mm),
        ("envelopeMm", control.envelope_mm),
        ("minimumScaledJacobian", control.minimum_scaled_jacobian),
    ] {
        if !value.is_finite() || value <= 0.0 {
            return Err(AppError::validation(format!(
                "fTetWild request {field} must be finite and positive."
            )));
        }
    }
    if control.minimum_scaled_jacobian > 1.0 {
        return Err(AppError::validation(
            "fTetWild request minimumScaledJacobian must not exceed 1.",
        ));
    }
    if control.deterministic_thread_count != 1 {
        return Err(AppError::validation(
            "fTetWild request deterministicThreadCount must be exactly 1 for reproducible MVP meshing.",
        ));
    }
    if control.allow_hole_filling {
        return Err(AppError::validation(
            "fTetWild request allowHoleFilling must remain false; open domains are rejected.",
        ));
    }
    if control.maximum_boundary_triangles == 0
        || control.maximum_nodes == 0
        || control.maximum_tet4_cells == 0
        || control.maximum_result_bytes == 0
        || control.maximum_runtime_ms == 0
    {
        return Err(AppError::validation(
            "fTetWild worker budgets and timeout must be positive.",
        ));
    }
    for (index, refinement) in control.local_refinements.iter().enumerate() {
        if refinement.face_group_indices.is_empty() {
            return Err(AppError::validation(format!(
                "fTetWild local refinement {index} must target at least one face group."
            )));
        }
        if !refinement.target_edge_length_mm.is_finite()
            || refinement.target_edge_length_mm <= 0.0
            || refinement.target_edge_length_mm > control.target_edge_length_mm
        {
            return Err(AppError::validation(format!(
                "fTetWild local refinement {index} targetEdgeLengthMm must be positive and no larger than global target."
            )));
        }
        if refinement
            .face_group_indices
            .windows(2)
            .any(|pair| pair[0] >= pair[1])
        {
            return Err(AppError::validation(format!(
                "fTetWild local refinement {index} faceGroupIndices must be sorted and unique."
            )));
        }
    }
    Ok(())
}

pub fn run_ftetwild_worker(
    identity: &FTetWildRuntimeIdentity,
    request: &FTetWildWorkerRequest,
    scratch_dir: impl AsRef<Path>,
    cancelled: &AtomicBool,
) -> AppResult<FemVolumeMesh> {
    request.validate()?;
    validate_worker_runtime_identity(identity)?;
    let request_bytes = serde_json::to_vec(request).map_err(|error| {
        AppError::internal(format!("fTetWild request could not be serialized: {error}"))
    })?;
    let scratch_dir = scratch_dir.as_ref();
    fs::create_dir_all(scratch_dir).map_err(|error| {
        AppError::persistence(format!(
            "fTetWild scratch directory '{}' could not be created: {error}",
            scratch_dir.display()
        ))
    })?;
    let files = WorkerScratchFiles::new(scratch_dir)?;
    fs::write(&files.request, request_bytes).map_err(|error| {
        AppError::persistence(format!(
            "fTetWild worker request '{}' could not be written: {error}",
            files.request.display()
        ))
    })?;
    let stdout = File::create(&files.stdout).map_err(worker_file_error("stdout", &files.stdout))?;
    let stderr = File::create(&files.stderr).map_err(worker_file_error("stderr", &files.stderr))?;
    let mut child = Command::new(&identity.executable_path)
        .arg("--protocol")
        .arg(FTETWILD_WORKER_PROTOCOL)
        .arg("--request")
        .arg(&files.request)
        .arg("--response")
        .arg(&files.response)
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr))
        .spawn()
        .map_err(|error| {
            AppError::render(format!(
                "fTetWild worker '{}' could not start: {error}",
                identity.executable_path.display()
            ))
        })?;
    let started = Instant::now();
    let timeout = Duration::from_millis(request.control.maximum_runtime_ms);
    let status = loop {
        if cancelled.load(Ordering::Acquire) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(AppError::conflict(format!(
                "fTetWild worker request '{}' was cancelled.",
                request.request_id
            )));
        }
        if started.elapsed() > timeout {
            let _ = child.kill();
            let _ = child.wait();
            return Err(AppError::render(format!(
                "fTetWild worker request '{}' exceeded runtime budget {} ms.",
                request.request_id, request.control.maximum_runtime_ms
            )));
        }
        match child
            .try_wait()
            .map_err(|error| AppError::render(format!("fTetWild worker status failed: {error}")))?
        {
            Some(status) => break status,
            None => thread::sleep(Duration::from_millis(10)),
        }
    };
    let raw_stdout = read_bounded_text(&files.stdout, 64 * 1024)?;
    let raw_stderr = read_bounded_text(&files.stderr, 64 * 1024)?;
    if !status.success() {
        let exit = status
            .code()
            .map(|code| code.to_string())
            .unwrap_or_else(|| "terminated-by-signal".to_string());
        return Err(AppError::render(format!(
            "fTetWild worker failed with exit code {exit}. stderr: {raw_stderr} stdout: {raw_stdout}"
        )));
    }
    let response_size = fs::metadata(&files.response)
        .map_err(|error| {
            AppError::render(format!(
                "fTetWild worker produced no readable response '{}': {error}. stderr: {raw_stderr}",
                files.response.display()
            ))
        })?
        .len();
    if response_size > request.control.maximum_result_bytes {
        return Err(AppError::validation(format!(
            "fTetWild worker result byte budget exceeded: observed {response_size}, allowed {}.",
            request.control.maximum_result_bytes
        )));
    }
    let response_bytes = fs::read(&files.response).map_err(|error| {
        AppError::persistence(format!(
            "fTetWild worker response '{}' could not be read: {error}",
            files.response.display()
        ))
    })?;
    let response: FTetWildWorkerResponse = serde_json::from_slice(&response_bytes).map_err(|error| {
        AppError::validation(format!(
            "fTetWild worker response is invalid: {error}. stderr: {raw_stderr} stdout: {raw_stdout}"
        ))
    })?;
    validate_and_convert_worker_response(identity, request, response)
}

fn validate_worker_runtime_identity(identity: &FTetWildRuntimeIdentity) -> AppResult<()> {
    require_equal(
        "runtime name",
        &identity.runtime_name,
        FTETWILD_RUNTIME_NAME,
    )?;
    require_equal("platform", &identity.platform, std::env::consts::OS)?;
    require_equal("architecture", &identity.arch, std::env::consts::ARCH)?;
    require_equal(
        "worker protocol",
        &identity.worker_protocol,
        FTETWILD_WORKER_PROTOCOL,
    )?;
    validate_capabilities(&identity.capabilities)?;
    if !identity.executable_path.is_file() {
        return Err(AppError::validation(format!(
            "fTetWild worker executable '{}' is missing.",
            identity.executable_path.display()
        )));
    }
    Ok(())
}

fn validate_and_convert_worker_response(
    identity: &FTetWildRuntimeIdentity,
    request: &FTetWildWorkerRequest,
    response: FTetWildWorkerResponse,
) -> AppResult<FemVolumeMesh> {
    if response.schema_version != FEM_SCHEMA_VERSION {
        return Err(AppError::validation(format!(
            "fTetWild worker response schemaVersion {} is unsupported.",
            response.schema_version
        )));
    }
    require_equal(
        "response protocol",
        &response.worker_protocol,
        FTETWILD_WORKER_PROTOCOL,
    )?;
    if response.request_id != request.request_id {
        return Err(AppError::validation(format!(
            "fTetWild worker response requestId mismatch: observed '{}', expected '{}'.",
            response.request_id, request.request_id
        )));
    }
    if response.nodes_mm.len() % 3 != 0
        || response.tet4_cells.len() % 4 != 0
        || response.boundary_triangles.len() % 3 != 0
    {
        return Err(AppError::validation(
            "fTetWild worker response has malformed typed-array cardinality.",
        ));
    }
    let node_count = response.nodes_mm.len() / 3;
    let cell_count = response.tet4_cells.len() / 4;
    let boundary_count = response.boundary_triangles.len() / 3;
    for (label, observed, allowed) in [
        ("nodes", node_count as u64, request.control.maximum_nodes),
        (
            "Tet4 cells",
            cell_count as u64,
            request.control.maximum_tet4_cells,
        ),
        (
            "boundary triangles",
            boundary_count as u64,
            request.control.maximum_boundary_triangles,
        ),
    ] {
        if observed > allowed {
            return Err(AppError::validation(format!(
                "fTetWild worker {label} budget exceeded: observed {observed}, allowed {allowed}."
            )));
        }
    }
    if response.boundary_face_group_indices.len() != boundary_count {
        return Err(AppError::validation(
            "fTetWild worker boundary group cardinality differs from boundary triangles.",
        ));
    }
    if response.face_group_count != request.face_group_count {
        return Err(AppError::validation(format!(
            "fTetWild worker faceGroupCount mismatch: observed {}, expected {}.",
            response.face_group_count, request.face_group_count
        )));
    }
    if response.thread_count != request.control.deterministic_thread_count {
        return Err(AppError::validation(format!(
            "fTetWild worker thread policy mismatch: observed {}, expected {}.",
            response.thread_count, request.control.deterministic_thread_count
        )));
    }
    if !response.maximum_boundary_deviation_mm.is_finite()
        || response.maximum_boundary_deviation_mm < 0.0
        || response.maximum_boundary_deviation_mm > request.control.envelope_mm
    {
        return Err(AppError::validation(format!(
            "fTetWild worker boundary deviation {} exceeds envelope {} mm.",
            response.maximum_boundary_deviation_mm, request.control.envelope_mm
        )));
    }
    let source_triangle_count = request.triangles.len() as u64 / 3;
    if response.insertion_count > source_triangle_count {
        return Err(AppError::validation(format!(
            "fTetWild worker insertionCount {} exceeds source triangle count {source_triangle_count}.",
            response.insertion_count,
        )));
    }
    let nodes = response
        .nodes_mm
        .chunks_exact(3)
        .map(|point| FemPoint3::new(point[0], point[1], point[2]))
        .collect();
    let cells = response
        .tet4_cells
        .chunks_exact(4)
        .map(|cell| [cell[0], cell[1], cell[2], cell[3]])
        .collect();
    let boundary_triangles = response
        .boundary_triangles
        .chunks_exact(3)
        .map(|triangle| [triangle[0], triangle[1], triangle[2]])
        .collect();
    FemVolumeMesh::validate_and_canonicalize(FemVolumeMeshInput {
        schema_version: response.schema_version,
        nodes,
        cells,
        boundary_triangles,
        boundary_face_group_indices: response.boundary_face_group_indices,
        face_group_count: response.face_group_count,
        face_group_targets: request.face_group_targets.clone(),
        source_boundary_digest: request.source_boundary_digest.clone(),
        mesher_identity: FemRuntimeIdentity {
            schema_version: FEM_SCHEMA_VERSION,
            platform: identity.platform.clone(),
            architecture: identity.arch.clone(),
            library_name: identity.runtime_name.clone(),
            library_version: identity.runtime_version.clone(),
            library_digest: identity.executable_sha256.clone(),
            adapter_protocol_version: 1,
            supported_capabilities: vec![
                "isolatedWorker".to_string(),
                "structuredArrays".to_string(),
                "tet4".to_string(),
                "wideSurfaceTags".to_string(),
            ],
            notice_digest: identity.notice_sha256.clone(),
        },
        meshing_evidence: FemMeshingEvidence {
            schema_version: FEM_SCHEMA_VERSION,
            source_triangle_count,
            inserted_source_triangle_count: response.insertion_count,
            tagged_boundary_triangle_count: boundary_count as u64,
            maximum_boundary_deviation_mm: response.maximum_boundary_deviation_mm,
            deterministic_thread_count: response.thread_count,
        },
        minimum_scaled_jacobian: request.control.minimum_scaled_jacobian,
    })
    .map_err(|error| AppError::validation(format!("fTetWild volume mesh rejected: {error}")))
}

fn worker_file_error<'a>(
    label: &'static str,
    path: &'a Path,
) -> impl FnOnce(std::io::Error) -> AppError + 'a {
    move |error| {
        AppError::persistence(format!(
            "fTetWild worker {label} file '{}' could not be created: {error}",
            path.display()
        ))
    }
}

fn read_bounded_text(path: &Path, maximum_bytes: u64) -> AppResult<String> {
    let size = fs::metadata(path)
        .map_err(|error| {
            AppError::persistence(format!(
                "fTetWild worker diagnostic '{}' metadata failed: {error}",
                path.display()
            ))
        })?
        .len();
    if size > maximum_bytes {
        return Err(AppError::validation(format!(
            "fTetWild worker diagnostic '{}' exceeds {} bytes.",
            path.display(),
            maximum_bytes
        )));
    }
    fs::read_to_string(path).map_err(|error| {
        AppError::persistence(format!(
            "fTetWild worker diagnostic '{}' could not be read: {error}",
            path.display()
        ))
    })
}

struct WorkerScratchFiles {
    request: PathBuf,
    response: PathBuf,
    stdout: PathBuf,
    stderr: PathBuf,
}

impl WorkerScratchFiles {
    fn new(scratch_dir: &Path) -> AppResult<Self> {
        let nonce = WORKER_RUN_NONCE.fetch_add(1, Ordering::Relaxed);
        let prefix = format!("ftetwild-worker-{}-{nonce}", std::process::id());
        let files = Self {
            request: scratch_dir.join(format!("{prefix}.request.json")),
            response: scratch_dir.join(format!("{prefix}.response.json")),
            stdout: scratch_dir.join(format!("{prefix}.stdout.txt")),
            stderr: scratch_dir.join(format!("{prefix}.stderr.txt")),
        };
        for path in [
            &files.request,
            &files.response,
            &files.stdout,
            &files.stderr,
        ] {
            if path.exists() {
                return Err(AppError::conflict(format!(
                    "fTetWild worker scratch path '{}' already exists.",
                    path.display()
                )));
            }
        }
        Ok(files)
    }
}

impl Drop for WorkerScratchFiles {
    fn drop(&mut self) {
        for path in [&self.request, &self.response, &self.stdout, &self.stderr] {
            let _ = fs::remove_file(path);
        }
    }
}

pub fn probe_ftetwild_runtime(
    runtime_root: impl AsRef<Path>,
    requirement: &FTetWildRuntimeRequirement,
) -> AppResult<FTetWildRuntimeIdentity> {
    validate_requirement(requirement)?;
    let runtime_root = runtime_root.as_ref();
    let manifest_path = runtime_root.join("runtime-manifest.json");
    let manifest_bytes = fs::read(&manifest_path).map_err(|error| {
        AppError::persistence(format!(
            "fTetWild runtime manifest '{}' could not be read: {error}",
            manifest_path.display()
        ))
    })?;
    let manifest: FTetWildRuntimeManifest =
        serde_json::from_slice(&manifest_bytes).map_err(|error| {
            AppError::validation(format!(
                "fTetWild runtime manifest '{}' is invalid: {error}",
                manifest_path.display()
            ))
        })?;

    validate_manifest_identity(&manifest, requirement)?;
    validate_capabilities(&manifest.capabilities)?;

    let executable = validate_manifest_file(runtime_root, "executable", &manifest.executable)?;
    validate_executable(&executable)?;
    validate_manifest_file(runtime_root, "source archive", &manifest.source_archive)?;
    validate_manifest_file(runtime_root, "license", &manifest.license)?;
    validate_manifest_file(runtime_root, "notice", &manifest.notice)?;
    let inventory_path = validate_manifest_file(
        runtime_root,
        "transitive license inventory",
        &manifest.transitive_license_inventory,
    )?;
    validate_transitive_inventory(&inventory_path)?;
    let runtime_library_paths = manifest
        .runtime_libraries
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            validate_manifest_file(runtime_root, &format!("runtime library {index}"), entry)
        })
        .collect::<AppResult<Vec<_>>>()?;
    if runtime_library_paths.is_empty() {
        return Err(AppError::validation(
            "fTetWild runtime manifest must list its non-system runtime libraries.",
        ));
    }

    Ok(FTetWildRuntimeIdentity {
        runtime_name: manifest.runtime_name,
        runtime_version: manifest.runtime_version,
        source_revision: manifest.source_revision,
        platform: manifest.platform,
        arch: manifest.arch,
        worker_protocol: manifest.worker_protocol,
        executable_path: executable,
        runtime_library_paths,
        executable_sha256: manifest.executable.sha256,
        source_sha256: manifest.source_archive.sha256,
        license_sha256: manifest.license.sha256,
        notice_sha256: manifest.notice.sha256,
        transitive_license_inventory_sha256: manifest.transitive_license_inventory.sha256,
        capabilities: manifest.capabilities,
    })
}

fn validate_requirement(requirement: &FTetWildRuntimeRequirement) -> AppResult<()> {
    if requirement.runtime_version.trim().is_empty() {
        return Err(AppError::validation(
            "fTetWild runtime requirement has an empty runtime version.",
        ));
    }
    if requirement.source_revision.len() != 40
        || !requirement
            .source_revision
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(AppError::validation(
            "fTetWild runtime requirement source revision must be a 40-character Git commit.",
        ));
    }
    Ok(())
}

fn validate_manifest_identity(
    manifest: &FTetWildRuntimeManifest,
    requirement: &FTetWildRuntimeRequirement,
) -> AppResult<()> {
    require_equal(
        "schema version",
        &manifest.schema_version.to_string(),
        &FTETWILD_RUNTIME_SCHEMA_VERSION.to_string(),
    )?;
    require_equal(
        "runtime name",
        &manifest.runtime_name,
        FTETWILD_RUNTIME_NAME,
    )?;
    require_equal(
        "runtime version",
        &manifest.runtime_version,
        &requirement.runtime_version,
    )?;
    require_equal(
        "source revision",
        &manifest.source_revision,
        &requirement.source_revision,
    )?;
    require_equal("platform", &manifest.platform, std::env::consts::OS)?;
    require_equal("architecture", &manifest.arch, std::env::consts::ARCH)?;
    require_equal(
        "worker protocol",
        &manifest.worker_protocol,
        FTETWILD_WORKER_PROTOCOL,
    )?;
    Ok(())
}

fn require_equal(label: &str, observed: &str, expected: &str) -> AppResult<()> {
    if observed == expected {
        return Ok(());
    }
    Err(AppError::validation(format!(
        "fTetWild runtime {label} mismatch: observed '{observed}', expected '{expected}'."
    )))
}

fn validate_capabilities(capabilities: &FTetWildRuntimeCapabilities) -> AppResult<()> {
    let missing = [
        ("structuredArrays", capabilities.structured_arrays),
        ("tet4", capabilities.tet4),
        ("wideSurfaceTags", capabilities.wide_surface_tags),
        ("isolatedWorker", capabilities.isolated_worker),
    ]
    .into_iter()
    .filter_map(|(name, present)| (!present).then_some(name))
    .collect::<Vec<_>>();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(AppError::validation(format!(
            "fTetWild runtime is missing required capabilities: {}.",
            missing.join(", ")
        )))
    }
}

fn validate_manifest_file(
    runtime_root: &Path,
    label: &str,
    entry: &ManifestFile,
) -> AppResult<PathBuf> {
    validate_relative_path(label, &entry.path)?;
    validate_sha256(label, &entry.sha256)?;
    let canonical_root = runtime_root.canonicalize().map_err(|error| {
        AppError::persistence(format!(
            "fTetWild runtime root '{}' could not be resolved: {error}",
            runtime_root.display()
        ))
    })?;
    let unresolved_path = runtime_root.join(&entry.path);
    let canonical_path = unresolved_path.canonicalize().map_err(|error| {
        AppError::persistence(format!(
            "fTetWild runtime {label} '{}' could not be resolved: {error}",
            unresolved_path.display()
        ))
    })?;
    if !canonical_path.starts_with(&canonical_root) {
        return Err(AppError::validation(format!(
            "fTetWild runtime {label} path '{}' escapes runtime root.",
            entry.path.display()
        )));
    }
    if !canonical_path.is_file() {
        return Err(AppError::validation(format!(
            "fTetWild runtime {label} '{}' is not a regular file.",
            canonical_path.display()
        )));
    }
    let observed = sha256_file(&canonical_path).map_err(|error| {
        AppError::persistence(format!(
            "fTetWild runtime {label} '{}' could not be hashed: {error}",
            canonical_path.display()
        ))
    })?;
    if observed != entry.sha256 {
        return Err(AppError::validation(format!(
            "fTetWild runtime {label} digest mismatch: observed '{observed}', expected '{}'.",
            entry.sha256
        )));
    }
    Ok(canonical_path)
}

fn validate_relative_path(label: &str, path: &Path) -> AppResult<()> {
    let valid = !path.as_os_str().is_empty()
        && !path.is_absolute()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_)));
    if valid {
        Ok(())
    } else {
        Err(AppError::validation(format!(
            "fTetWild runtime {label} path '{}' must be a confined relative path.",
            path.display()
        )))
    }
}

fn validate_sha256(label: &str, digest: &str) -> AppResult<()> {
    let Some(hex) = digest.strip_prefix("sha256:") else {
        return Err(AppError::validation(format!(
            "fTetWild runtime {label} digest must use sha256: prefix."
        )));
    };
    if hex.len() == 64 && hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err(AppError::validation(format!(
            "fTetWild runtime {label} digest must contain 64 hexadecimal characters."
        )))
    }
}

fn validate_executable(path: &Path) -> AppResult<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(path)
            .map_err(|error| {
                AppError::persistence(format!(
                    "fTetWild runtime executable '{}' metadata could not be read: {error}",
                    path.display()
                ))
            })?
            .permissions()
            .mode();
        if mode & 0o111 == 0 {
            return Err(AppError::validation(format!(
                "fTetWild runtime executable '{}' has no executable permission bit.",
                path.display()
            )));
        }
    }
    Ok(())
}

fn validate_transitive_inventory(path: &Path) -> AppResult<()> {
    let bytes = fs::read(path).map_err(|error| {
        AppError::persistence(format!(
            "fTetWild transitive license inventory '{}' could not be read: {error}",
            path.display()
        ))
    })?;
    let inventory: serde_json::Value = serde_json::from_slice(&bytes).map_err(|error| {
        AppError::validation(format!(
            "fTetWild transitive license inventory '{}' is invalid JSON: {error}",
            path.display()
        ))
    })?;
    let Some(records) = inventory.as_array() else {
        return Err(AppError::validation(
            "fTetWild transitive license inventory must be a JSON array.",
        ));
    };
    if records.is_empty() {
        return Err(AppError::validation(
            "fTetWild transitive license inventory must list bundled dependencies.",
        ));
    }
    for (index, record) in records.iter().enumerate() {
        let Some(record) = record.as_object() else {
            return Err(AppError::validation(format!(
                "fTetWild transitive license record {index} must be an object."
            )));
        };
        for field in ["name", "version", "license", "sourceUrl"] {
            if record
                .get(field)
                .and_then(serde_json::Value::as_str)
                .is_none_or(|value| value.trim().is_empty())
            {
                return Err(AppError::validation(format!(
                    "fTetWild transitive license record {index} is missing non-empty '{field}'."
                )));
            }
        }
    }
    Ok(())
}

fn sha256_file(path: &Path) -> std::io::Result<String> {
    let mut reader = BufReader::new(File::open(path)?);
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(format!("sha256:{:x}", hasher.finalize()))
}
