use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use ecky_fem::{
    FemMeshingEvidence, FemRuntimeIdentity, FemVolumeMesh, FemVolumeMeshInput, FEM_SCHEMA_VERSION,
};

use crate::contracts::{AppError, AppResult};
use crate::gmsh_mesher::{
    parse_exact_face_signatures, parse_msh2, read_bounded_text, sha256_bytes, sha256_file,
    validate_budgets, validate_exact_face_mapping, validate_face_mapping, validate_request,
    GmshBrepMeshRequest,
};

const NETGEN_ADAPTER_PROTOCOL_VERSION: u32 = 1;
const DIAGNOSTIC_LIMIT_BYTES: u64 = 64 * 1024;

const NETGEN_PROBE: &str = r#"
import netgen
import netgen.libngpy
print(netgen.__version__)
print(netgen.libngpy.__file__)
"#;

const NETGEN_MESH_SCRIPT: &str = r#"
import sys
from netgen.occ import OCCGeometry
from netgen.meshing import MeshingParameters

step_path, mesh_path = sys.argv[1], sys.argv[2]
minimum_size, maximum_size = float(sys.argv[3]), float(sys.argv[4])
expected_faces = int(sys.argv[5])
geometry = OCCGeometry(step_path)
faces = geometry.shape.faces
if len(faces) != expected_faces:
    raise RuntimeError(f"exact OCC face count {len(faces)} differs from expected {expected_faces}")
for index, face in enumerate(faces):
    center = face.center
    print(f"ECKY_FACE {index + 1} {face.mass:.17g} {center[0]:.17g} {center[1]:.17g} {center[2]:.17g}")
for refinement in sys.argv[6:]:
    encoded_indices, encoded_size = refinement.split(":", 1)
    target_size = float(encoded_size)
    for encoded_index in encoded_indices.split(","):
        faces[int(encoded_index)].maxh = target_size
parameters = MeshingParameters(
    maxh=maximum_size,
    minh=minimum_size,
    grading=0.3,
    secondorder=False,
)
mesh = geometry.GenerateMesh(parameters)
mesh.Export(mesh_path, "Gmsh2 Format")
"#;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetgenRuntimeIdentity {
    pub python_path: PathBuf,
    pub python_sha256: String,
    pub module_path: PathBuf,
    pub module_sha256: String,
    pub runtime_digest: String,
    pub version: String,
    pub platform: String,
    pub architecture: String,
}

pub fn probe_default_netgen_runtime() -> AppResult<NetgenRuntimeIdentity> {
    if let Some(python) = std::env::var_os("ECKY_NETGEN_PYTHON") {
        return probe_netgen_runtime(Path::new(&python));
    }
    let launcher = resolve_on_path(Path::new("netgen"))?;
    let mut first_line = String::new();
    File::open(&launcher)
        .and_then(|mut file| file.read_to_string(&mut first_line))
        .map_err(|error| {
            AppError::validation(format!(
                "Netgen launcher '{}' could not be read: {error}",
                launcher.display()
            ))
        })?;
    let shebang = first_line.lines().next().unwrap_or_default();
    let python = shebang.strip_prefix("#!").ok_or_else(|| {
        AppError::validation(format!(
            "Netgen launcher '{}' has no absolute Python shebang.",
            launcher.display()
        ))
    })?;
    probe_netgen_runtime(Path::new(python.trim()))
}

pub fn probe_netgen_runtime(python: &Path) -> AppResult<NetgenRuntimeIdentity> {
    let python_path = if python.is_absolute() {
        python.to_path_buf()
    } else {
        resolve_on_path(python)?
    };
    if !python_path.is_file() {
        return Err(AppError::validation(format!(
            "Netgen Python '{}' is not a file.",
            python_path.display()
        )));
    }
    let output = Command::new(&python_path)
        .args(["-c", NETGEN_PROBE])
        .output()
        .map_err(|error| {
            AppError::validation(format!(
                "Netgen Python '{}' could not run: {error}",
                python_path.display()
            ))
        })?;
    if !output.status.success() {
        return Err(AppError::validation(format!(
            "Netgen probe failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut lines = stdout.lines();
    let version = lines.next().unwrap_or_default().trim().to_string();
    let module_path = PathBuf::from(lines.next().unwrap_or_default().trim())
        .canonicalize()
        .map_err(|error| AppError::validation(format!("Netgen module path is invalid: {error}")))?;
    if version.is_empty() {
        return Err(AppError::validation("Netgen probe returned no version."));
    }
    let python_sha256 = sha256_file(&python_path)?;
    let module_sha256 = sha256_file(&module_path)?;
    let runtime_digest = sha256_bytes(
        format!(
            "{}\n{}\n{}\n{}",
            python_sha256,
            module_sha256,
            version,
            sha256_bytes(NETGEN_MESH_SCRIPT.as_bytes())
        )
        .as_bytes(),
    );
    Ok(NetgenRuntimeIdentity {
        python_path,
        python_sha256,
        module_path,
        module_sha256,
        runtime_digest,
        version,
        platform: std::env::consts::OS.into(),
        architecture: std::env::consts::ARCH.into(),
    })
}

pub fn run_netgen_exact_brep(
    runtime: &NetgenRuntimeIdentity,
    request: &GmshBrepMeshRequest,
    scratch_dir: &Path,
    cancelled: &AtomicBool,
) -> AppResult<FemVolumeMesh> {
    validate_request(request)?;
    if sha256_file(&runtime.python_path)? != runtime.python_sha256
        || sha256_file(&runtime.module_path)? != runtime.module_sha256
    {
        return Err(AppError::conflict(
            "Netgen runtime changed after probe; refusing non-immutable fallback.",
        ));
    }
    if sha256_file(&request.step_path)? != request.step_sha256 {
        return Err(AppError::conflict(
            "Exact STEP changed after request resolution before Netgen fallback.",
        ));
    }
    fs::create_dir_all(scratch_dir).map_err(|error| {
        AppError::persistence(format!(
            "Netgen scratch '{}' could not be created: {error}",
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
    let mesh_path = scratch_dir.join(format!("{safe_id}-netgen.msh"));
    let script_path = scratch_dir.join(format!("{safe_id}-netgen.py"));
    let stdout_path = scratch_dir.join(format!("{safe_id}-netgen.stdout"));
    let stderr_path = scratch_dir.join(format!("{safe_id}-netgen.stderr"));
    let _scratch = ScratchFiles([
        mesh_path.clone(),
        script_path.clone(),
        stdout_path.clone(),
        stderr_path.clone(),
    ]);
    fs::write(&script_path, NETGEN_MESH_SCRIPT).map_err(|error| {
        AppError::persistence(format!(
            "Netgen adapter script could not be written: {error}"
        ))
    })?;
    let minimum_size = request
        .control
        .local_refinements
        .iter()
        .map(|refinement| refinement.target_size_mm)
        .fold(request.control.global_size_mm, f64::min);
    let mut arguments = vec![
        script_path.to_string_lossy().into_owned(),
        request.step_path.to_string_lossy().into_owned(),
        mesh_path.to_string_lossy().into_owned(),
        minimum_size.to_string(),
        request.control.global_size_mm.to_string(),
        request.face_signatures.len().to_string(),
    ];
    for refinement in &request.control.local_refinements {
        arguments.push(format!(
            "{}:{}",
            refinement
                .face_group_indices
                .iter()
                .map(u32::to_string)
                .collect::<Vec<_>>()
                .join(","),
            refinement.target_size_mm
        ));
    }
    let stdout = File::create(&stdout_path).map_err(|error| {
        AppError::persistence(format!("Netgen stdout file could not be created: {error}"))
    })?;
    let stderr = File::create(&stderr_path).map_err(|error| {
        AppError::persistence(format!("Netgen stderr file could not be created: {error}"))
    })?;
    let thread_count = request.control.thread_count.to_string();
    let mut child = Command::new(&runtime.python_path)
        .args(&arguments)
        .env("NETGEN_NUM_THREADS", &thread_count)
        .env("OMP_NUM_THREADS", &thread_count)
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr))
        .spawn()
        .map_err(|error| {
            AppError::validation(format!(
                "Netgen runtime '{}' could not start: {error}",
                runtime.python_path.display()
            ))
        })?;
    let started = Instant::now();
    let timeout = Duration::from_millis(request.control.maximum_runtime_ms);
    let status = loop {
        if cancelled.load(Ordering::Relaxed) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(AppError::conflict(format!(
                "Netgen request '{}' was cancelled.",
                request.request_id
            )));
        }
        if started.elapsed() > timeout {
            let _ = child.kill();
            let _ = child.wait();
            return Err(AppError::validation(format!(
                "Netgen request '{}' exceeded {} ms runtime budget.",
                request.request_id, request.control.maximum_runtime_ms
            )));
        }
        match child
            .try_wait()
            .map_err(|error| AppError::validation(format!("Netgen status check failed: {error}")))?
        {
            Some(status) => break status,
            None => thread::sleep(Duration::from_millis(20)),
        }
    };
    if !status.success() {
        return Err(AppError::validation(format!(
            "Netgen exact-BRep fallback failed with {status}: stderr='{}'; stdout='{}'.",
            read_bounded_text(&stderr_path, DIAGNOSTIC_LIMIT_BYTES)?.trim(),
            read_bounded_text(&stdout_path, DIAGNOSTIC_LIMIT_BYTES)?.trim()
        )));
    }
    let exact_faces = parse_exact_face_signatures(
        &read_bounded_text(&stdout_path, DIAGNOSTIC_LIMIT_BYTES)?,
        request.face_signatures.len(),
    )?;
    validate_exact_face_mapping(&exact_faces, request)?;
    let result_size = fs::metadata(&mesh_path)
        .map_err(|error| AppError::persistence(format!("Netgen mesh metadata failed: {error}")))?
        .len();
    if result_size > request.control.maximum_result_bytes {
        return Err(AppError::validation(format!(
            "Netgen mesh result uses {result_size} bytes; budget is {}.",
            request.control.maximum_result_bytes
        )));
    }
    let parsed = parse_msh2(&mesh_path, request.face_signatures.len(), &request.control)?;
    validate_budgets(&parsed, &request.control)?;
    validate_face_mapping(&parsed, request)?;
    let boundary_triangle_count = parsed.boundary_triangles.len() as u64;
    let face_group_count = u32::try_from(request.face_group_targets.len())
        .map_err(|_| AppError::validation("Netgen face-group count exceeds u32 range."))?;
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
            library_name: "Netgen OCC".into(),
            library_version: runtime.version.clone(),
            library_digest: runtime.runtime_digest.clone(),
            adapter_protocol_version: NETGEN_ADAPTER_PROTOCOL_VERSION,
            supported_capabilities: vec![
                "exactBrepStep".into(),
                "durableSurfaceEntities".into(),
                "faceLocalRefinement".into(),
                "isolatedProcess".into(),
            ],
            notice_digest: sha256_bytes(
                b"Netgen external runtime; LGPL-2.1-only; not linked or redistributed; https://ngsolve.org/",
            ),
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
    .map_err(|error| AppError::validation(format!("Netgen volume mesh rejected: {error}")))
}

fn resolve_on_path(executable: &Path) -> AppResult<PathBuf> {
    let path = std::env::var_os("PATH")
        .ok_or_else(|| AppError::validation("Netgen lookup failed because PATH is unset."))?;
    std::env::split_paths(&path)
        .map(|directory| directory.join(executable))
        .find(|candidate| candidate.is_file())
        .ok_or_else(|| AppError::validation("Netgen executable was not found on PATH."))
}

struct ScratchFiles([PathBuf; 4]);

impl Drop for ScratchFiles {
    fn drop(&mut self) {
        for path in &self.0 {
            let _ = fs::remove_file(path);
        }
    }
}
