use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use ecky_fem::{
    topology_result_digest, topology_state_digest, FemIndexedTet4Mesh, FemLinearSolverIdentity,
    FemMma87State, FemTopologyIteration, FemTopologyResult, FemTopologyState,
    FemTopologyTermination,
};
use sha2::{Digest, Sha256};

use crate::{
    contracts::{AppError, AppResult},
    models::PathResolver,
};

const ROOT: &str = "fem-topology-v2";
const MAGIC: &[u8; 8] = b"ECKYTOP5";
const STATE_MAGIC: &[u8; 8] = b"ECKYSTA5";
static NONCE: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone)]
pub struct FemTopologyArtifact {
    pub checkpoint_path: PathBuf,
    pub density_path: PathBuf,
    pub preview_vtu_path: PathBuf,
    pub state: FemTopologyState,
    pub result: FemTopologyResult,
}

#[derive(Debug, Clone)]
pub struct FemTopologyStateArtifact {
    pub checkpoint_path: PathBuf,
    pub state: FemTopologyState,
}

struct CheckpointEnvelope {
    state: FemTopologyState,
    result: FemTopologyResult,
    density_sha256: String,
    sensitivity_sha256: String,
    preview_sha256: String,
}

pub fn publish_fem_topology_artifact(
    app: &dyn PathResolver,
    mesh: &FemIndexedTet4Mesh,
    state: &FemTopologyState,
    result: &FemTopologyResult,
    maximum_result_bytes: u64,
) -> AppResult<FemTopologyArtifact> {
    if maximum_result_bytes == 0 {
        return Err(AppError::validation(
            "FEM topology artifact byte budget must be positive.",
        ));
    }
    let root = app.app_data_dir().join(ROOT);
    let final_dir = root
        .join(digest_component(&state.input_digest)?)
        .join(digest_component(&state.state_digest)?);
    if final_dir.is_dir() {
        if final_dir.join("checkpoint.bin").is_file() {
            return load_from_dir(
                &final_dir,
                &state.input_digest,
                &state.state_digest,
                maximum_result_bytes,
            );
        }
        let existing = load_fem_topology_state(
            app,
            &state.input_digest,
            &state.state_digest,
            maximum_result_bytes,
        )?;
        if existing.state != *state {
            return Err(AppError::conflict(
                "FEM topology state-only artifact differs from full publication state.",
            ));
        }
        let (checkpoint, density, sensitivity, preview) =
            topology_artifact_bytes(mesh, state, result, maximum_result_bytes)?;
        write_atomic(&final_dir.join("density.f64le"), &density)?;
        write_atomic(
            &final_dir.join("compliance-sensitivity.f64le"),
            &sensitivity,
        )?;
        write_atomic(&final_dir.join("density-preview.vtu"), &preview)?;
        // checkpoint.bin is the commit marker. State-only readers remain valid
        // if publication stops before this final atomic rename.
        write_atomic(&final_dir.join("checkpoint.bin"), &checkpoint)?;
        return load_from_dir(
            &final_dir,
            &state.input_digest,
            &state.state_digest,
            maximum_result_bytes,
        );
    }
    fs::create_dir_all(&root).map_err(|error| {
        AppError::persistence(format!("FEM topology root create failed: {error}"))
    })?;
    let temporary_dir = root.join(format!(
        ".publishing-{}-{}",
        std::process::id(),
        NONCE.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&temporary_dir).map_err(|error| {
        AppError::persistence(format!(
            "FEM topology temporary directory create failed: {error}"
        ))
    })?;

    let published = (|| {
        let (checkpoint_bytes, density_bytes, sensitivity_bytes, preview_bytes) =
            topology_artifact_bytes(mesh, state, result, maximum_result_bytes)?;
        write_atomic(&temporary_dir.join("checkpoint.bin"), &checkpoint_bytes)?;
        write_atomic(&temporary_dir.join("density.f64le"), &density_bytes)?;
        write_atomic(
            &temporary_dir.join("compliance-sensitivity.f64le"),
            &sensitivity_bytes,
        )?;
        write_atomic(&temporary_dir.join("density-preview.vtu"), &preview_bytes)?;
        fs::create_dir_all(final_dir.parent().expect("topology digest has parent")).map_err(
            |error| {
                AppError::persistence(format!("FEM topology digest root create failed: {error}"))
            },
        )?;
        fs::rename(&temporary_dir, &final_dir).map_err(|error| {
            AppError::persistence(format!("FEM topology atomic publication failed: {error}"))
        })?;
        load_from_dir(
            &final_dir,
            &state.input_digest,
            &state.state_digest,
            maximum_result_bytes,
        )
    })();
    if published.is_err() && temporary_dir.exists() {
        let _ = fs::remove_dir_all(&temporary_dir);
    }
    published
}

fn topology_artifact_bytes(
    mesh: &FemIndexedTet4Mesh,
    state: &FemTopologyState,
    result: &FemTopologyResult,
    maximum_result_bytes: u64,
) -> AppResult<(Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>)> {
    let density = f64_bytes(&result.densities);
    let sensitivity = f64_bytes(&result.compliance_sensitivity);
    let preview = topology_vtu(mesh, &result.densities)?;
    let envelope = CheckpointEnvelope {
        state: state.clone(),
        result: result.clone(),
        density_sha256: sha256_bytes(&density),
        sensitivity_sha256: sha256_bytes(&sensitivity),
        preview_sha256: sha256_bytes(&preview),
    };
    let checkpoint = encode_checkpoint(&envelope)?;
    let total_bytes = checkpoint
        .len()
        .saturating_add(density.len())
        .saturating_add(sensitivity.len())
        .saturating_add(preview.len()) as u64;
    if total_bytes > maximum_result_bytes {
        return Err(AppError::validation(format!(
            "FEM topology artifacts exceed byte budget: observed {total_bytes}, allowed {maximum_result_bytes}."
        )));
    }
    Ok((checkpoint, density, sensitivity, preview))
}

pub fn load_fem_topology_artifact(
    app: &dyn PathResolver,
    input_digest: &str,
    state_digest: &str,
    maximum_result_bytes: u64,
) -> AppResult<FemTopologyArtifact> {
    let directory = app
        .app_data_dir()
        .join(ROOT)
        .join(digest_component(input_digest)?)
        .join(digest_component(state_digest)?);
    load_from_dir(&directory, input_digest, state_digest, maximum_result_bytes)
}

pub fn publish_fem_topology_state_checkpoint(
    app: &dyn PathResolver,
    state: &FemTopologyState,
    maximum_result_bytes: u64,
) -> AppResult<FemTopologyStateArtifact> {
    if maximum_result_bytes == 0 {
        return Err(AppError::validation(
            "FEM topology checkpoint byte budget must be positive.",
        ));
    }
    if topology_state_digest(state) != state.state_digest {
        return Err(AppError::conflict(
            "FEM topology state canonical digest is invalid.",
        ));
    }
    let root = app.app_data_dir().join(ROOT);
    let final_dir = root
        .join(digest_component(&state.input_digest)?)
        .join(digest_component(&state.state_digest)?);
    if final_dir.is_dir() {
        return load_fem_topology_state(
            app,
            &state.input_digest,
            &state.state_digest,
            maximum_result_bytes,
        );
    }
    fs::create_dir_all(&root).map_err(|error| {
        AppError::persistence(format!("FEM topology root create failed: {error}"))
    })?;
    let temporary_dir = root.join(format!(
        ".publishing-state-{}-{}",
        std::process::id(),
        NONCE.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&temporary_dir).map_err(|error| {
        AppError::persistence(format!(
            "FEM topology state temporary directory create failed: {error}"
        ))
    })?;
    let published = (|| {
        let checkpoint = encode_state_checkpoint(state)?;
        if checkpoint.len() as u64 > maximum_result_bytes {
            return Err(AppError::validation(
                "FEM topology state checkpoint exceeds byte budget.",
            ));
        }
        write_atomic(&temporary_dir.join("state-checkpoint.bin"), &checkpoint)?;
        fs::create_dir_all(
            final_dir
                .parent()
                .expect("topology state digest has parent"),
        )
        .map_err(|error| {
            AppError::persistence(format!(
                "FEM topology state digest root create failed: {error}"
            ))
        })?;
        fs::rename(&temporary_dir, &final_dir).map_err(|error| {
            AppError::persistence(format!(
                "FEM topology state atomic publication failed: {error}"
            ))
        })?;
        load_fem_topology_state(
            app,
            &state.input_digest,
            &state.state_digest,
            maximum_result_bytes,
        )
    })();
    if published.is_err() && temporary_dir.exists() {
        let _ = fs::remove_dir_all(&temporary_dir);
    }
    published
}

pub fn load_fem_topology_state(
    app: &dyn PathResolver,
    input_digest: &str,
    state_digest: &str,
    maximum_result_bytes: u64,
) -> AppResult<FemTopologyStateArtifact> {
    let directory = app
        .app_data_dir()
        .join(ROOT)
        .join(digest_component(input_digest)?)
        .join(digest_component(state_digest)?);
    let state_path = directory.join("state-checkpoint.bin");
    if state_path.is_file() {
        let state = decode_state_checkpoint(&read_bounded(&state_path, maximum_result_bytes)?)?;
        if state.input_digest != input_digest
            || state.state_digest != state_digest
            || topology_state_digest(&state) != state.state_digest
        {
            return Err(AppError::conflict(
                "FEM topology state checkpoint identity or digest is invalid.",
            ));
        }
        return Ok(FemTopologyStateArtifact {
            checkpoint_path: state_path,
            state,
        });
    }
    let artifact = load_from_dir(&directory, input_digest, state_digest, maximum_result_bytes)?;
    Ok(FemTopologyStateArtifact {
        checkpoint_path: artifact.checkpoint_path,
        state: artifact.state,
    })
}

fn load_from_dir(
    directory: &Path,
    input_digest: &str,
    state_digest: &str,
    maximum_result_bytes: u64,
) -> AppResult<FemTopologyArtifact> {
    let checkpoint_path = directory.join("checkpoint.bin");
    let density_path = directory.join("density.f64le");
    let sensitivity_path = directory.join("compliance-sensitivity.f64le");
    let preview_vtu_path = directory.join("density-preview.vtu");
    let checkpoint = read_bounded(&checkpoint_path, maximum_result_bytes)?;
    let density = read_bounded(&density_path, maximum_result_bytes)?;
    let sensitivity = read_bounded(&sensitivity_path, maximum_result_bytes)?;
    let preview = read_bounded(&preview_vtu_path, maximum_result_bytes)?;
    let total = checkpoint
        .len()
        .saturating_add(density.len())
        .saturating_add(sensitivity.len())
        .saturating_add(preview.len()) as u64;
    if total > maximum_result_bytes {
        return Err(AppError::validation(
            "FEM topology artifact set exceeds read byte budget.",
        ));
    }
    let mut envelope = decode_checkpoint(&checkpoint)?;
    if envelope.state.input_digest != input_digest
        || envelope.state.state_digest != state_digest
        || envelope.result.result_digest.is_empty()
        || envelope.result.exact_brep
        || envelope.result.production_step
        || envelope.result.engineering_accepted
        || sha256_bytes(&density) != envelope.density_sha256
        || sha256_bytes(&sensitivity) != envelope.sensitivity_sha256
        || sha256_bytes(&preview) != envelope.preview_sha256
    {
        return Err(AppError::conflict(
            "FEM topology checkpoint identity, evidence boundary, or array digest is invalid.",
        ));
    }
    envelope.result.densities = decode_f64_bytes(&density)?;
    envelope.result.compliance_sensitivity = decode_f64_bytes(&sensitivity)?;
    if envelope.state.design_densities.len() != envelope.result.densities.len()
        || envelope.result.compliance_sensitivity.len() != envelope.result.densities.len()
    {
        return Err(AppError::validation(
            "FEM topology density arrays have inconsistent lengths.",
        ));
    }
    if topology_state_digest(&envelope.state) != envelope.state.state_digest
        || topology_result_digest(&envelope.result) != envelope.result.result_digest
    {
        return Err(AppError::conflict(
            "FEM topology checkpoint canonical state or result digest is invalid.",
        ));
    }
    Ok(FemTopologyArtifact {
        checkpoint_path,
        density_path,
        preview_vtu_path,
        state: envelope.state,
        result: envelope.result,
    })
}

fn encode_checkpoint(envelope: &CheckpointEnvelope) -> AppResult<Vec<u8>> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(MAGIC);
    put_u32(&mut bytes, ecky_fem::FEM_SCHEMA_VERSION);
    put_string(&mut bytes, &envelope.state.input_digest)?;
    put_string(&mut bytes, &envelope.state.state_digest)?;
    put_string(&mut bytes, &envelope.result.result_digest)?;
    put_string(&mut bytes, &envelope.density_sha256)?;
    put_string(&mut bytes, &envelope.sensitivity_sha256)?;
    put_string(&mut bytes, &envelope.preview_sha256)?;
    put_f64_vec(&mut bytes, &envelope.state.design_densities)?;
    put_mma87_state(&mut bytes, &envelope.state.mma87)?;
    match envelope.state.initial_compliance {
        Some(value) => {
            bytes.push(1);
            put_f64(&mut bytes, value);
        }
        None => bytes.push(0),
    }
    put_iterations(&mut bytes, &envelope.state.iterations)?;
    put_solver_identity(&mut bytes, envelope.state.solver_identity.as_ref())?;
    put_f64(&mut bytes, envelope.result.initial_compliance);
    put_f64(&mut bytes, envelope.result.final_compliance);
    put_f64(&mut bytes, envelope.result.final_volume_fraction);
    put_f64(&mut bytes, envelope.result.filter_radius_mm);
    put_f64(&mut bytes, envelope.result.passive_solid_volume_fraction);
    put_f64(&mut bytes, envelope.result.passive_void_volume_fraction);
    put_solver_identity(&mut bytes, envelope.result.solver_identity.as_ref())?;
    bytes.push(termination_code(envelope.result.termination));
    Ok(bytes)
}

fn encode_state_checkpoint(state: &FemTopologyState) -> AppResult<Vec<u8>> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(STATE_MAGIC);
    put_u32(&mut bytes, ecky_fem::FEM_SCHEMA_VERSION);
    put_string(&mut bytes, &state.input_digest)?;
    put_string(&mut bytes, &state.state_digest)?;
    put_f64_vec(&mut bytes, &state.design_densities)?;
    put_mma87_state(&mut bytes, &state.mma87)?;
    match state.initial_compliance {
        Some(value) => {
            bytes.push(1);
            put_f64(&mut bytes, value);
        }
        None => bytes.push(0),
    }
    put_iterations(&mut bytes, &state.iterations)?;
    put_solver_identity(&mut bytes, state.solver_identity.as_ref())?;
    Ok(bytes)
}

fn decode_state_checkpoint(bytes: &[u8]) -> AppResult<FemTopologyState> {
    let mut cursor = Cursor::new(bytes);
    if cursor.take(8)? != STATE_MAGIC {
        return Err(AppError::validation(
            "FEM topology state checkpoint magic is invalid.",
        ));
    }
    if cursor.u32()? != ecky_fem::FEM_SCHEMA_VERSION {
        return Err(AppError::validation(
            "FEM topology state checkpoint schema is unsupported.",
        ));
    }
    let input_digest = cursor.string()?;
    let state_digest = cursor.string()?;
    let design_densities = cursor.f64_vec()?;
    let mma87 = cursor.mma87_state()?;
    let initial_compliance = match cursor.u8()? {
        0 => None,
        1 => Some(cursor.f64()?),
        _ => {
            return Err(AppError::validation(
                "FEM topology state checkpoint option is invalid.",
            ));
        }
    };
    let iterations = cursor.iterations()?;
    let solver_identity = cursor.solver_identity()?;
    if !cursor.finished() {
        return Err(AppError::validation(
            "FEM topology state checkpoint contains trailing bytes.",
        ));
    }
    Ok(FemTopologyState {
        schema_version: ecky_fem::FEM_SCHEMA_VERSION,
        input_digest,
        design_densities,
        mma87,
        initial_compliance,
        solver_identity,
        iterations,
        state_digest,
    })
}

fn decode_checkpoint(bytes: &[u8]) -> AppResult<CheckpointEnvelope> {
    let mut cursor = Cursor::new(bytes);
    if cursor.take(8)? != MAGIC {
        return Err(AppError::validation(
            "FEM topology checkpoint magic is invalid.",
        ));
    }
    if cursor.u32()? != ecky_fem::FEM_SCHEMA_VERSION {
        return Err(AppError::validation(
            "FEM topology checkpoint schema is unsupported.",
        ));
    }
    let input_digest = cursor.string()?;
    let state_digest = cursor.string()?;
    let result_digest = cursor.string()?;
    let density_sha256 = cursor.string()?;
    let sensitivity_sha256 = cursor.string()?;
    let preview_sha256 = cursor.string()?;
    let design_densities = cursor.f64_vec()?;
    let mma87 = cursor.mma87_state()?;
    let initial_compliance = match cursor.u8()? {
        0 => None,
        1 => Some(cursor.f64()?),
        _ => {
            return Err(AppError::validation(
                "FEM topology checkpoint option is invalid.",
            ))
        }
    };
    let iterations = cursor.iterations()?;
    let state_solver_identity = cursor.solver_identity()?;
    let result_initial_compliance = cursor.f64()?;
    let final_compliance = cursor.f64()?;
    let final_volume_fraction = cursor.f64()?;
    let filter_radius_mm = cursor.f64()?;
    let passive_solid_volume_fraction = cursor.f64()?;
    let passive_void_volume_fraction = cursor.f64()?;
    let result_solver_identity = cursor.solver_identity()?;
    let termination = decode_termination(cursor.u8()?)?;
    if !cursor.finished() {
        return Err(AppError::validation(
            "FEM topology checkpoint contains trailing bytes.",
        ));
    }
    let state = FemTopologyState {
        schema_version: ecky_fem::FEM_SCHEMA_VERSION,
        input_digest,
        design_densities,
        mma87,
        initial_compliance,
        solver_identity: state_solver_identity,
        iterations: iterations.clone(),
        state_digest,
    };
    let result = FemTopologyResult {
        schema_version: ecky_fem::FEM_SCHEMA_VERSION,
        initial_compliance: result_initial_compliance,
        final_compliance,
        final_volume_fraction,
        filter_radius_mm,
        passive_solid_volume_fraction,
        passive_void_volume_fraction,
        densities: Vec::new(),
        compliance_sensitivity: Vec::new(),
        iterations,
        solver_identity: result_solver_identity,
        termination,
        exact_brep: false,
        production_step: false,
        engineering_accepted: false,
        result_digest,
    };
    Ok(CheckpointEnvelope {
        state,
        result,
        density_sha256,
        sensitivity_sha256,
        preview_sha256,
    })
}

fn topology_vtu(mesh: &FemIndexedTet4Mesh, densities: &[f64]) -> AppResult<Vec<u8>> {
    if densities.len() != mesh.cells.len() {
        return Err(AppError::validation(
            "FEM topology density count must equal Tet4 cell count.",
        ));
    }
    let mut text = String::from("<?xml version=\"1.0\"?>\n<VTKFile type=\"UnstructuredGrid\" version=\"0.1\" byte_order=\"LittleEndian\">\n<UnstructuredGrid>\n");
    text.push_str(&format!(
        "<Piece NumberOfPoints=\"{}\" NumberOfCells=\"{}\">\n",
        mesh.nodes.len(),
        mesh.cells.len()
    ));
    text.push_str(
        "<Points><DataArray type=\"Float64\" NumberOfComponents=\"3\" format=\"ascii\">\n",
    );
    for point in &mesh.nodes {
        text.push_str(&format!("{} {} {} ", point.x_mm, point.y_mm, point.z_mm));
    }
    text.push_str("\n</DataArray></Points>\n<Cells>\n<DataArray type=\"Int32\" Name=\"connectivity\" format=\"ascii\">\n");
    for cell in &mesh.cells {
        text.push_str(&format!("{} {} {} {} ", cell[0], cell[1], cell[2], cell[3]));
    }
    text.push_str("\n</DataArray>\n<DataArray type=\"Int32\" Name=\"offsets\" format=\"ascii\">\n");
    for index in 1..=mesh.cells.len() {
        text.push_str(&format!("{} ", index * 4));
    }
    text.push_str("\n</DataArray>\n<DataArray type=\"UInt8\" Name=\"types\" format=\"ascii\">\n");
    for _ in &mesh.cells {
        text.push_str("10 ");
    }
    text.push_str("\n</DataArray>\n</Cells>\n<CellData Scalars=\"density\"><DataArray type=\"Float64\" Name=\"density\" format=\"ascii\">\n");
    for density in densities {
        text.push_str(&format!("{density} "));
    }
    text.push_str("\n</DataArray></CellData>\n</Piece></UnstructuredGrid></VTKFile>\n");
    Ok(text.into_bytes())
}

fn termination_code(value: FemTopologyTermination) -> u8 {
    match value {
        FemTopologyTermination::Paused => 0,
        FemTopologyTermination::Cancelled => 1,
        FemTopologyTermination::Converged => 2,
        FemTopologyTermination::MaximumIterations => 3,
        FemTopologyTermination::MaximumWallTime => 4,
    }
}

fn decode_termination(value: u8) -> AppResult<FemTopologyTermination> {
    match value {
        0 => Ok(FemTopologyTermination::Paused),
        1 => Ok(FemTopologyTermination::Cancelled),
        2 => Ok(FemTopologyTermination::Converged),
        3 => Ok(FemTopologyTermination::MaximumIterations),
        4 => Ok(FemTopologyTermination::MaximumWallTime),
        _ => Err(AppError::validation(
            "FEM topology checkpoint termination is invalid.",
        )),
    }
}

fn put_iterations(bytes: &mut Vec<u8>, values: &[FemTopologyIteration]) -> AppResult<()> {
    put_u64(bytes, bounded_len(values.len())?);
    for value in values {
        put_u64(bytes, value.iteration as u64);
        put_f64(bytes, value.compliance);
        put_f64(bytes, value.volume_fraction);
        put_f64(bytes, value.maximum_density_change);
        put_f64(bytes, value.maximum_physical_density_change);
        put_f64(bytes, value.kkt_residual);
        put_u64(bytes, value.conservative_inner_attempts as u64);
    }
    Ok(())
}

fn put_mma87_state(bytes: &mut Vec<u8>, state: &FemMma87State) -> AppResult<()> {
    put_f64_vec(bytes, &state.previous_design_densities)?;
    put_f64_vec(bytes, &state.previous_previous_design_densities)?;
    put_f64_vec(bytes, &state.asymptote_widths)?;
    put_f64(bytes, state.dual);
    put_f64(bytes, state.objective_lift);
    put_f64(bytes, state.constraint_lift);
    Ok(())
}

fn put_solver_identity(
    bytes: &mut Vec<u8>,
    value: Option<&FemLinearSolverIdentity>,
) -> AppResult<()> {
    let Some(value) = value else {
        bytes.push(0);
        return Ok(());
    };
    bytes.push(1);
    put_string(bytes, &value.backend)?;
    put_string(bytes, &value.backend_version)?;
    put_string(bytes, &value.factorization)?;
    put_string(bytes, &value.ordering)?;
    put_string(bytes, &value.scalar_type)?;
    put_string(bytes, &value.parallelism)?;
    put_u64(bytes, value.thread_count as u64);
    for time in [value.factor_time_ms, value.solve_time_ms] {
        match time {
            Some(time) => {
                bytes.push(1);
                put_f64(bytes, time);
            }
            None => bytes.push(0),
        }
    }
    put_f64(bytes, value.relative_tolerance);
    Ok(())
}

fn put_f64_vec(bytes: &mut Vec<u8>, values: &[f64]) -> AppResult<()> {
    put_u64(bytes, bounded_len(values.len())?);
    for value in values {
        put_f64(bytes, *value);
    }
    Ok(())
}

fn put_string(bytes: &mut Vec<u8>, value: &str) -> AppResult<()> {
    put_u32(
        bytes,
        u32::try_from(value.len())
            .map_err(|_| AppError::validation("FEM topology checkpoint string is too long."))?,
    );
    bytes.extend_from_slice(value.as_bytes());
    Ok(())
}

fn bounded_len(value: usize) -> AppResult<u64> {
    u64::try_from(value)
        .map_err(|_| AppError::validation("FEM topology checkpoint vector is too long."))
}

fn put_u32(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn put_u64(bytes: &mut Vec<u8>, value: u64) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn put_f64(bytes: &mut Vec<u8>, value: f64) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

struct Cursor<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Cursor<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn take(&mut self, count: usize) -> AppResult<&'a [u8]> {
        let end = self
            .offset
            .checked_add(count)
            .ok_or_else(|| AppError::validation("FEM topology checkpoint offset overflowed."))?;
        let value = self
            .bytes
            .get(self.offset..end)
            .ok_or_else(|| AppError::validation("FEM topology checkpoint is truncated."))?;
        self.offset = end;
        Ok(value)
    }

    fn u8(&mut self) -> AppResult<u8> {
        Ok(self.take(1)?[0])
    }

    fn u32(&mut self) -> AppResult<u32> {
        Ok(u32::from_le_bytes(
            self.take(4)?.try_into().expect("4 bytes"),
        ))
    }

    fn u64(&mut self) -> AppResult<u64> {
        Ok(u64::from_le_bytes(
            self.take(8)?.try_into().expect("8 bytes"),
        ))
    }

    fn f64(&mut self) -> AppResult<f64> {
        let value = f64::from_le_bytes(self.take(8)?.try_into().expect("8 bytes"));
        if !value.is_finite() {
            return Err(AppError::validation(
                "FEM topology checkpoint contains non-finite float.",
            ));
        }
        Ok(value)
    }

    fn string(&mut self) -> AppResult<String> {
        let count = self.u32()? as usize;
        String::from_utf8(self.take(count)?.to_vec())
            .map_err(|_| AppError::validation("FEM topology checkpoint string is not UTF-8."))
    }

    fn f64_vec(&mut self) -> AppResult<Vec<f64>> {
        let count = self.bounded_count(8, "vector")?;
        (0..count).map(|_| self.f64()).collect()
    }

    fn iterations(&mut self) -> AppResult<Vec<FemTopologyIteration>> {
        let count = self.bounded_count(8 + 8 * 6, "trace")?;
        (0..count)
            .map(|_| {
                Ok(FemTopologyIteration {
                    iteration: usize::try_from(self.u64()?).map_err(|_| {
                        AppError::validation("FEM topology iteration exceeds platform bounds.")
                    })?,
                    compliance: self.f64()?,
                    volume_fraction: self.f64()?,
                    maximum_density_change: self.f64()?,
                    maximum_physical_density_change: self.f64()?,
                    kkt_residual: self.f64()?,
                    conservative_inner_attempts: usize::try_from(self.u64()?).map_err(|_| {
                        AppError::validation(
                            "FEM topology inner-attempt count exceeds platform bounds.",
                        )
                    })?,
                })
            })
            .collect()
    }

    fn mma87_state(&mut self) -> AppResult<FemMma87State> {
        Ok(FemMma87State {
            previous_design_densities: self.f64_vec()?,
            previous_previous_design_densities: self.f64_vec()?,
            asymptote_widths: self.f64_vec()?,
            dual: self.f64()?,
            objective_lift: self.f64()?,
            constraint_lift: self.f64()?,
        })
    }

    fn solver_identity(&mut self) -> AppResult<Option<FemLinearSolverIdentity>> {
        if self.u8()? == 0 {
            return Ok(None);
        }
        let backend = self.string()?;
        let backend_version = self.string()?;
        let factorization = self.string()?;
        let ordering = self.string()?;
        let scalar_type = self.string()?;
        let parallelism = self.string()?;
        let thread_count = usize::try_from(self.u64()?).map_err(|_| {
            AppError::validation("FEM topology solver thread count exceeds platform bounds.")
        })?;
        let mut time = || -> AppResult<Option<f64>> {
            match self.u8()? {
                0 => Ok(None),
                1 => Ok(Some(self.f64()?)),
                _ => Err(AppError::validation(
                    "FEM topology solver timing option is invalid.",
                )),
            }
        };
        let factor_time_ms = time()?;
        let solve_time_ms = time()?;
        Ok(Some(FemLinearSolverIdentity {
            backend,
            backend_version,
            factorization,
            ordering,
            scalar_type,
            parallelism,
            thread_count,
            factor_time_ms,
            solve_time_ms,
            relative_tolerance: self.f64()?,
        }))
    }

    fn bounded_count(&mut self, encoded_item_bytes: usize, label: &str) -> AppResult<usize> {
        let declared = usize::try_from(self.u64()?).map_err(|_| {
            AppError::validation(format!(
                "FEM topology checkpoint {label} exceeds platform bounds."
            ))
        })?;
        let remaining = self.bytes.len().saturating_sub(self.offset);
        if declared > remaining / encoded_item_bytes {
            return Err(AppError::validation(format!(
                "FEM topology checkpoint {label} declared length exceeds remaining bytes."
            )));
        }
        Ok(declared)
    }

    fn finished(&self) -> bool {
        self.offset == self.bytes.len()
    }
}

fn digest_component(digest: &str) -> AppResult<&str> {
    let value = digest
        .strip_prefix("sha256:")
        .ok_or_else(|| AppError::validation("FEM topology identity must use sha256 prefix."))?;
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(AppError::validation(
            "FEM topology identity must contain 64 hexadecimal characters.",
        ));
    }
    Ok(value)
}

fn read_bounded(path: &Path, maximum: u64) -> AppResult<Vec<u8>> {
    let metadata = fs::metadata(path).map_err(|error| {
        AppError::persistence(format!(
            "FEM topology artifact metadata failed for {}: {error}",
            path.display()
        ))
    })?;
    if metadata.len() > maximum {
        return Err(AppError::validation(
            "FEM topology artifact exceeds read byte budget.",
        ));
    }
    fs::read(path).map_err(|error| {
        AppError::persistence(format!(
            "FEM topology artifact read failed for {}: {error}",
            path.display()
        ))
    })
}

fn write_atomic(path: &Path, bytes: &[u8]) -> AppResult<()> {
    let temporary = path.with_extension("tmp");
    let mut file = fs::File::create(&temporary).map_err(|error| {
        AppError::persistence(format!(
            "FEM topology temporary file create failed: {error}"
        ))
    })?;
    file.write_all(bytes).map_err(|error| {
        AppError::persistence(format!("FEM topology temporary file write failed: {error}"))
    })?;
    file.sync_all().map_err(|error| {
        AppError::persistence(format!("FEM topology temporary file sync failed: {error}"))
    })?;
    fs::rename(&temporary, path).map_err(|error| {
        AppError::persistence(format!("FEM topology file publication failed: {error}"))
    })
}

fn sha256_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("sha256:{:x}", hasher.finalize())
}

fn f64_bytes(values: &[f64]) -> Vec<u8> {
    values
        .iter()
        .flat_map(|value| value.to_le_bytes())
        .collect()
}

fn decode_f64_bytes(bytes: &[u8]) -> AppResult<Vec<f64>> {
    if bytes.len() % 8 != 0 {
        return Err(AppError::validation(
            "FEM topology float64 array has invalid byte length.",
        ));
    }
    let values = bytes
        .as_chunks::<8>()
        .0
        .iter()
        .map(|chunk| f64::from_le_bytes(*chunk))
        .collect::<Vec<_>>();
    if values.iter().any(|value| !value.is_finite()) {
        return Err(AppError::validation(
            "FEM topology float64 array contains non-finite values.",
        ));
    }
    Ok(values)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestResolver(PathBuf);

    impl PathResolver for TestResolver {
        fn app_config_dir(&self) -> PathBuf {
            self.0.join("config")
        }

        fn app_data_dir(&self) -> PathBuf {
            self.0.join("data")
        }

        fn resource_path(&self, _path: &str) -> Option<PathBuf> {
            None
        }
    }

    #[test]
    fn topology_checkpoint_round_trips_with_digest_bound_evidence() {
        let root = std::env::temp_dir().join(format!(
            "ecky-topology-artifact-{}-{}",
            std::process::id(),
            NONCE.fetch_add(1, Ordering::Relaxed)
        ));
        let resolver = TestResolver(root.clone());
        let mut state = FemTopologyState {
            schema_version: ecky_fem::FEM_SCHEMA_VERSION,
            input_digest: digest('a'),
            design_densities: vec![0.25],
            mma87: FemMma87State {
                previous_design_densities: vec![0.25],
                previous_previous_design_densities: vec![0.25],
                asymptote_widths: vec![0.2],
                dual: 0.0,
                objective_lift: 0.0,
                constraint_lift: 0.0,
            },
            initial_compliance: Some(10.0),
            solver_identity: None,
            iterations: vec![],
            state_digest: String::new(),
        };
        state.state_digest = topology_state_digest(&state);
        let mut result = FemTopologyResult {
            schema_version: ecky_fem::FEM_SCHEMA_VERSION,
            initial_compliance: 10.0,
            final_compliance: 8.0,
            final_volume_fraction: 0.25,
            filter_radius_mm: 2.0,
            passive_solid_volume_fraction: 0.0,
            passive_void_volume_fraction: 0.0,
            densities: vec![0.25],
            compliance_sensitivity: vec![-4.0],
            iterations: vec![],
            solver_identity: None,
            termination: FemTopologyTermination::Paused,
            exact_brep: false,
            production_step: false,
            engineering_accepted: false,
            result_digest: String::new(),
        };
        result.result_digest = topology_result_digest(&result);
        let mesh = FemIndexedTet4Mesh {
            schema_version: ecky_fem::FEM_SCHEMA_VERSION,
            nodes: vec![
                ecky_fem::FemPoint3::new(0.0, 0.0, 0.0),
                ecky_fem::FemPoint3::new(1.0, 0.0, 0.0),
                ecky_fem::FemPoint3::new(0.0, 1.0, 0.0),
                ecky_fem::FemPoint3::new(0.0, 0.0, 1.0),
            ],
            cells: vec![[0, 1, 2, 3]],
        };
        let asset =
            publish_fem_topology_artifact(&resolver, &mesh, &state, &result, 1_000_000).unwrap();
        assert_eq!(&fs::read(&asset.checkpoint_path).unwrap()[..8], MAGIC);
        let loaded = load_fem_topology_artifact(
            &resolver,
            &state.input_digest,
            &state.state_digest,
            1_000_000,
        )
        .unwrap();
        assert_eq!(loaded.state.design_densities, vec![0.25]);
        assert_eq!(loaded.result.densities, vec![0.25]);
        assert_eq!(loaded.result.compliance_sensitivity, vec![-4.0]);
        assert!(!loaded.result.exact_brep);
        assert!(!loaded.result.production_step);
        assert!(!loaded.result.engineering_accepted);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn checkpoint_declared_vector_count_is_bounded_before_allocation() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(MAGIC);
        put_u32(&mut bytes, ecky_fem::FEM_SCHEMA_VERSION);
        for _ in 0..6 {
            put_string(&mut bytes, "").unwrap();
        }
        put_u64(&mut bytes, u64::MAX);

        let error = match decode_checkpoint(&bytes) {
            Ok(_) => panic!("declared vector larger than remaining envelope must fail"),
            Err(error) => error,
        };
        assert!(error
            .message
            .contains("declared length exceeds remaining bytes"));
    }

    #[test]
    fn stopped_state_checkpoint_round_trips_without_running_final_analysis() {
        let root = std::env::temp_dir().join(format!(
            "ecky-topology-stopped-state-{}-{}",
            std::process::id(),
            NONCE.fetch_add(1, Ordering::Relaxed)
        ));
        let resolver = TestResolver(root.clone());
        let mut state = FemTopologyState {
            schema_version: ecky_fem::FEM_SCHEMA_VERSION,
            input_digest: digest('d'),
            design_densities: vec![0.5, 0.5],
            mma87: FemMma87State {
                previous_design_densities: vec![0.5, 0.5],
                previous_previous_design_densities: vec![0.5, 0.5],
                asymptote_widths: vec![0.2, 0.2],
                dual: 0.0,
                objective_lift: 0.0,
                constraint_lift: 0.0,
            },
            initial_compliance: None,
            solver_identity: None,
            iterations: vec![],
            state_digest: String::new(),
        };
        state.state_digest = topology_state_digest(&state);

        let published = publish_fem_topology_state_checkpoint(&resolver, &state, 1_000_000)
            .expect("publish stopped state");
        assert!(published.checkpoint_path.is_file());
        assert!(!published
            .checkpoint_path
            .parent()
            .unwrap()
            .join("density.f64le")
            .exists());
        let loaded = load_fem_topology_state(
            &resolver,
            &state.input_digest,
            &state.state_digest,
            1_000_000,
        )
        .expect("load stopped state");
        assert_eq!(loaded.state, state);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn full_publication_upgrades_matching_state_only_checkpoint() {
        let root = std::env::temp_dir().join(format!(
            "ecky-topology-state-upgrade-{}-{}",
            std::process::id(),
            NONCE.fetch_add(1, Ordering::Relaxed)
        ));
        let resolver = TestResolver(root.clone());
        let mut state = FemTopologyState {
            schema_version: ecky_fem::FEM_SCHEMA_VERSION,
            input_digest: digest('e'),
            design_densities: vec![0.5],
            mma87: FemMma87State {
                previous_design_densities: vec![0.5],
                previous_previous_design_densities: vec![0.5],
                asymptote_widths: vec![0.2],
                dual: 0.0,
                objective_lift: 0.0,
                constraint_lift: 0.0,
            },
            initial_compliance: Some(10.0),
            solver_identity: None,
            iterations: vec![],
            state_digest: String::new(),
        };
        state.state_digest = topology_state_digest(&state);
        let mut result = FemTopologyResult {
            schema_version: ecky_fem::FEM_SCHEMA_VERSION,
            initial_compliance: 10.0,
            final_compliance: 8.0,
            final_volume_fraction: 0.5,
            filter_radius_mm: 2.0,
            passive_solid_volume_fraction: 0.0,
            passive_void_volume_fraction: 0.0,
            densities: vec![0.5],
            compliance_sensitivity: vec![-4.0],
            iterations: vec![],
            solver_identity: None,
            termination: FemTopologyTermination::Paused,
            exact_brep: false,
            production_step: false,
            engineering_accepted: false,
            result_digest: String::new(),
        };
        result.result_digest = topology_result_digest(&result);
        let mesh = FemIndexedTet4Mesh {
            schema_version: ecky_fem::FEM_SCHEMA_VERSION,
            nodes: vec![
                ecky_fem::FemPoint3::new(0.0, 0.0, 0.0),
                ecky_fem::FemPoint3::new(1.0, 0.0, 0.0),
                ecky_fem::FemPoint3::new(0.0, 1.0, 0.0),
                ecky_fem::FemPoint3::new(0.0, 0.0, 1.0),
            ],
            cells: vec![[0, 1, 2, 3]],
        };

        publish_fem_topology_state_checkpoint(&resolver, &state, 1_000_000)
            .expect("publish stopped state");
        let asset = publish_fem_topology_artifact(&resolver, &mesh, &state, &result, 1_000_000)
            .expect("upgrade stopped state to full artifact");

        assert!(asset.checkpoint_path.is_file());
        assert!(asset.density_path.is_file());
        assert!(asset.preview_vtu_path.is_file());
        assert_eq!(asset.state, state);
        assert_eq!(asset.result, result);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    #[ignore = "diagnostic: requires ECKY_TOPOLOGY_DATA_DIR, ECKY_TOPOLOGY_INPUT_DIGEST, and ECKY_TOPOLOGY_STATE_DIGEST"]
    fn inspect_immutable_topology_checkpoint_as_edn() {
        struct DirectResolver(PathBuf);
        impl PathResolver for DirectResolver {
            fn app_config_dir(&self) -> PathBuf {
                self.0.join("config")
            }
            fn app_data_dir(&self) -> PathBuf {
                self.0.clone()
            }
            fn resource_path(&self, _path: &str) -> Option<PathBuf> {
                None
            }
        }
        let data_dir = PathBuf::from(std::env::var("ECKY_TOPOLOGY_DATA_DIR").unwrap());
        let input = std::env::var("ECKY_TOPOLOGY_INPUT_DIGEST").unwrap();
        let state = std::env::var("ECKY_TOPOLOGY_STATE_DIGEST").unwrap();
        let resolver = DirectResolver(data_dir);
        let state_dir = resolver
            .app_data_dir()
            .join(ROOT)
            .join(digest_component(&input).unwrap())
            .join(digest_component(&state).unwrap());
        if !state_dir.join("checkpoint.bin").is_file() {
            let artifact =
                load_fem_topology_state(&resolver, &input, &state, 512 * 1024 * 1024).unwrap();
            let tail = artifact
                .state
                .iterations
                .iter()
                .rev()
                .take(10)
                .collect::<Vec<_>>();
            println!(
                "{{:iterations {} :artifact-kind :state-only :trace-tail [",
                artifact.state.iterations.len()
            );
            for iteration in tail.into_iter().rev() {
                println!(
                    "  {{:iteration {} :compliance {:.17} :volume-fraction {:.17} :maximum-density-change {:.17} :maximum-physical-density-change {:.17} :kkt-residual {:.17} :inner-attempts {}}}",
                    iteration.iteration,
                    iteration.compliance,
                    iteration.volume_fraction,
                    iteration.maximum_density_change,
                    iteration.maximum_physical_density_change,
                    iteration.kkt_residual,
                    iteration.conservative_inner_attempts,
                );
            }
            println!("]}}");
            return;
        }
        let artifact =
            load_fem_topology_artifact(&resolver, &input, &state, 512 * 1024 * 1024).unwrap();
        let densities = &artifact.result.densities;
        let count_above = |threshold: f64| {
            densities
                .iter()
                .filter(|value| **value >= threshold)
                .count()
        };
        let mut density_changes = artifact
            .state
            .design_densities
            .iter()
            .zip(&artifact.state.mma87.previous_design_densities)
            .map(|(current, previous)| (current - previous).abs())
            .collect::<Vec<_>>();
        density_changes.sort_by(f64::total_cmp);
        let change_count_above = |threshold: f64| {
            density_changes
                .iter()
                .filter(|value| **value >= threshold)
                .count()
        };
        let change_mean = density_changes.iter().sum::<f64>() / density_changes.len() as f64;
        let change_rms = (density_changes
            .iter()
            .map(|value| value * value)
            .sum::<f64>()
            / density_changes.len() as f64)
            .sqrt();
        let change_p99 = density_changes[density_changes.len() * 99 / 100];
        let tail = artifact
            .result
            .iterations
            .iter()
            .rev()
            .take(10)
            .collect::<Vec<_>>();
        println!(
            "{{:iterations {} :termination \"{:?}\" :last-change {:.17} :last-physical-change {:.17} :last-kkt-residual {:.17} :density-change {{:mean {:.17} :rms {:.17} :p99 {:.17} :above-0.01 {} :above-0.05 {} :above-0.1 {} :above-0.19 {}}} :density {{:count {} :min {:.17} :max {:.17} :above-0.2 {} :above-0.5 {} :above-0.8 {}}} :trace-tail [",
            artifact.result.iterations.len(),
            artifact.result.termination,
            artifact.result.iterations.last().unwrap().maximum_density_change,
            artifact
                .result
                .iterations
                .last()
                .unwrap()
                .maximum_physical_density_change,
            artifact.result.iterations.last().unwrap().kkt_residual,
            change_mean,
            change_rms,
            change_p99,
            change_count_above(0.01),
            change_count_above(0.05),
            change_count_above(0.1),
            change_count_above(0.19),
            densities.len(),
            densities.iter().copied().fold(f64::INFINITY, f64::min),
            densities.iter().copied().fold(f64::NEG_INFINITY, f64::max),
            count_above(0.2),
            count_above(0.5),
            count_above(0.8),
        );
        for iteration in tail.into_iter().rev() {
            println!(
                "  {{:iteration {} :compliance {:.17} :volume-fraction {:.17} :maximum-density-change {:.17} :maximum-physical-density-change {:.17} :kkt-residual {:.17} :inner-attempts {}}}",
                iteration.iteration,
                iteration.compliance,
                iteration.volume_fraction,
                iteration.maximum_density_change,
                iteration.maximum_physical_density_change,
                iteration.kkt_residual,
                iteration.conservative_inner_attempts,
            );
        }
        println!("]}}");
    }

    fn digest(value: char) -> String {
        format!("sha256:{}", value.to_string().repeat(64))
    }
}
