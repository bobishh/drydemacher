use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use ecky_fem::{
    CanonicalDigest, FemEngineeringEvidenceLedger, FemIdealizationArtifact, FemMeshQuality,
    FemResultSummary, FemSupportReactionResult,
};

use crate::contracts::{AppError, AppResult};
use crate::models::PathResolver;
use crate::services::fem::{FemAcceptanceEvaluation, FemMeshPipelineResult, FemPipelineResult};

static FEM_ARTIFACT_NONCE: AtomicU64 = AtomicU64::new(1);
const FEM_RESULT_ROOT: &str = "fem-results-v3";
const FEM_RESULT_MANIFEST: &str = "manifest.edn";
const FEM_RESULT_ASSET_SCHEMA_VERSION: u32 = 7;
const FEM_MESH_ROOT: &str = "fem-meshes-v1";
const FEM_MESH_ASSET_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FemScalarType {
    Float64Le,
    Uint32Le,
}

impl FemScalarType {
    fn byte_width(self) -> u64 {
        match self {
            Self::Float64Le => 8,
            Self::Uint32Le => 4,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemBinaryArrayAsset {
    pub name: String,
    pub path: PathBuf,
    pub scalar_type: FemScalarType,
    pub shape: Vec<u64>,
    pub byte_length: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemResultAsset {
    pub schema_version: u32,
    pub source_digest: String,
    pub analysis_identity_digest: String,
    pub solution_digest: String,
    pub result_digest: String,
    pub mesh_content_digest: String,
    pub source_boundary_digest: String,
    pub engineering_evidence_digest: String,
    pub engineering_evidence: FemEngineeringEvidenceLedger,
    pub idealization_artifact_digest: String,
    pub idealization_artifact: FemIdealizationArtifact,
    pub decision_ready: bool,
    pub decision_readiness_error: Option<String>,
    pub mesh_quality: FemMeshQuality,
    pub summary: FemResultSummary,
    pub equilibrium_relative_imbalance: f64,
    pub solver_relative_residual: f64,
    pub support_reactions: Vec<FemSupportReactionResult>,
    pub acceptance_evaluations: Vec<FemAcceptanceEvaluation>,
    pub arrays: Vec<FemBinaryArrayAsset>,
    pub manifest_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemMeshAsset {
    pub schema_version: u32,
    pub analysis_identity_digest: String,
    pub mesh_content_digest: String,
    pub source_boundary_digest: String,
    pub mesh_quality: FemMeshQuality,
    pub face_group_count: u32,
    pub arrays: Vec<FemBinaryArrayAsset>,
    pub manifest_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FemTopologyMeshData {
    pub mesh: ecky_fem::FemIndexedTet4Mesh,
    pub boundary_triangles: Vec<[u32; 3]>,
    pub boundary_face_group_indices: Vec<u32>,
    pub face_group_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StoredFemMeshAsset {
    schema_version: u32,
    analysis_identity_digest: String,
    mesh_content_digest: String,
    source_boundary_digest: String,
    mesh_quality: FemMeshQuality,
    face_group_count: u32,
    arrays: Vec<FemBinaryArrayAsset>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StoredFemResultAsset {
    schema_version: u32,
    source_digest: String,
    analysis_identity_digest: String,
    solution_digest: String,
    result_digest: String,
    mesh_content_digest: String,
    source_boundary_digest: String,
    engineering_evidence_digest: String,
    engineering_evidence: FemEngineeringEvidenceLedger,
    idealization_artifact_digest: String,
    idealization_artifact: FemIdealizationArtifact,
    decision_ready: bool,
    decision_readiness_error: Option<String>,
    mesh_quality: FemMeshQuality,
    summary: FemResultSummary,
    equilibrium_relative_imbalance: f64,
    solver_relative_residual: f64,
    support_reactions: Vec<FemSupportReactionResult>,
    acceptance_evaluations: Vec<FemAcceptanceEvaluation>,
    arrays: Vec<FemBinaryArrayAsset>,
}

pub fn publish_fem_mesh_asset(
    app: &dyn PathResolver,
    result: &FemMeshPipelineResult,
    maximum_result_bytes: u64,
) -> AppResult<FemMeshAsset> {
    if maximum_result_bytes == 0 {
        return Err(AppError::validation(
            "FEM mesh artifact byte budget must be positive.",
        ));
    }
    let analysis_digest = result.analysis_identity.canonical_digest();
    let result_root = app.app_data_dir().join(FEM_MESH_ROOT);
    fs::create_dir_all(&result_root).map_err(|error| {
        AppError::persistence(format!(
            "FEM mesh root '{}' could not be created: {error}",
            result_root.display()
        ))
    })?;
    let final_dir = result_root
        .join(digest_component(&analysis_digest)?)
        .join(digest_component(&result.mesh.content_digest)?);
    if final_dir.is_dir() {
        return load_fem_mesh_asset_from_dir(&final_dir, &analysis_digest, maximum_result_bytes);
    }

    let nonce = FEM_ARTIFACT_NONCE.fetch_add(1, Ordering::Relaxed);
    let temporary_dir = result_root.join(format!(".publishing-{}-{nonce}", std::process::id()));
    fs::create_dir(&temporary_dir).map_err(|error| {
        AppError::persistence(format!(
            "FEM temporary mesh directory '{}' could not be created: {error}",
            temporary_dir.display()
        ))
    })?;
    let temporary = TemporaryDirectory::new(temporary_dir.clone());
    fs::create_dir(temporary_dir.join("arrays")).map_err(|error| {
        AppError::persistence(format!(
            "FEM mesh array directory could not be created: {error}"
        ))
    })?;
    let arrays = vec![
        write_f64_array(
            &temporary_dir,
            "nodesMm",
            "arrays/nodes.f64le",
            &[result.mesh.nodes.len() as u64, 3],
            result
                .mesh
                .nodes
                .iter()
                .flat_map(|point| [point.x_mm, point.y_mm, point.z_mm]),
        )?,
        write_u32_array(
            &temporary_dir,
            "tet4Cells",
            "arrays/tet4-cells.u32le",
            &[result.mesh.cells.len() as u64, 4],
            result.mesh.cells.iter().flatten().copied(),
        )?,
        write_u32_array(
            &temporary_dir,
            "boundaryTriangles",
            "arrays/boundary-triangles.u32le",
            &[result.mesh.boundary_triangles.len() as u64, 3],
            result.mesh.boundary_triangles.iter().flatten().copied(),
        )?,
        write_u32_array(
            &temporary_dir,
            "boundaryFaceGroupIndices",
            "arrays/boundary-face-groups.u32le",
            &[result.mesh.boundary_face_group_indices.len() as u64],
            result.mesh.boundary_face_group_indices.iter().copied(),
        )?,
    ];
    enforce_total_budget(&arrays, maximum_result_bytes)?;
    let stored = StoredFemMeshAsset {
        schema_version: FEM_MESH_ASSET_SCHEMA_VERSION,
        analysis_identity_digest: analysis_digest.clone(),
        mesh_content_digest: result.mesh.content_digest.clone(),
        source_boundary_digest: result.mesh.source_boundary_digest.clone(),
        mesh_quality: result.mesh.quality.clone(),
        face_group_count: result.mesh.face_group_count,
        arrays,
    };
    let manifest_bytes = crate::strict_edn::to_vec(&stored).map_err(|error| {
        AppError::internal(format!("FEM mesh manifest serialization failed: {error}"))
    })?;
    if manifest_bytes.len() as u64 > maximum_result_bytes {
        return Err(AppError::validation(format!(
            "FEM mesh manifest exceeds byte budget: observed {}, allowed {maximum_result_bytes}.",
            manifest_bytes.len()
        )));
    }
    write_bytes(&temporary_dir.join(FEM_RESULT_MANIFEST), &manifest_bytes)?;
    fs::create_dir_all(
        final_dir
            .parent()
            .expect("mesh digest directory has parent"),
    )
    .map_err(|error| {
        AppError::persistence(format!(
            "FEM mesh artifact directory could not be created: {error}"
        ))
    })?;
    match fs::rename(&temporary_dir, &final_dir) {
        Ok(()) => temporary.disarm(),
        Err(_error) if final_dir.is_dir() => {
            drop(temporary);
            return load_fem_mesh_asset_from_dir(
                &final_dir,
                &analysis_digest,
                maximum_result_bytes,
            );
        }
        Err(error) => {
            return Err(AppError::persistence(format!(
                "FEM mesh publication '{}' failed atomically: {error}",
                final_dir.display()
            )))
        }
    }
    load_fem_mesh_asset_from_dir(&final_dir, &analysis_digest, maximum_result_bytes)
}

pub fn load_fem_mesh_asset(
    app: &dyn PathResolver,
    analysis_identity_digest: &str,
    mesh_content_digest: &str,
    maximum_result_bytes: u64,
) -> AppResult<FemMeshAsset> {
    let directory = app
        .app_data_dir()
        .join(FEM_MESH_ROOT)
        .join(digest_component(analysis_identity_digest)?)
        .join(digest_component(mesh_content_digest)?);
    load_fem_mesh_asset_from_dir(&directory, analysis_identity_digest, maximum_result_bytes)
}

pub fn decode_fem_topology_mesh(asset: &FemMeshAsset) -> AppResult<FemTopologyMeshData> {
    let directory = asset
        .manifest_path
        .parent()
        .ok_or_else(|| AppError::validation("FEM mesh manifest has no artifact directory."))?;
    let node_values = read_f64_asset(directory, required_array(&asset.arrays, "nodesMm")?)?;
    let cell_values = read_u32_asset(directory, required_array(&asset.arrays, "tet4Cells")?)?;
    let triangle_values = read_u32_asset(
        directory,
        required_array(&asset.arrays, "boundaryTriangles")?,
    )?;
    let boundary_face_group_indices = read_u32_asset(
        directory,
        required_array(&asset.arrays, "boundaryFaceGroupIndices")?,
    )?;
    if node_values.len() % 3 != 0 || cell_values.len() % 4 != 0 || triangle_values.len() % 3 != 0 {
        return Err(AppError::validation(
            "FEM mesh arrays have invalid node/cell/triangle shapes.",
        ));
    }
    let nodes = node_values
        .as_chunks::<3>()
        .0
        .iter()
        .map(|value| ecky_fem::FemPoint3::new(value[0], value[1], value[2]))
        .collect::<Vec<_>>();
    let cells = cell_values
        .as_chunks::<4>()
        .0
        .iter()
        .map(|value| [value[0], value[1], value[2], value[3]])
        .collect::<Vec<_>>();
    let boundary_triangles = triangle_values
        .as_chunks::<3>()
        .0
        .iter()
        .map(|value| [value[0], value[1], value[2]])
        .collect::<Vec<_>>();
    if boundary_face_group_indices.len() != boundary_triangles.len()
        || boundary_face_group_indices
            .iter()
            .any(|group| *group >= asset.face_group_count)
    {
        return Err(AppError::validation(
            "FEM boundary face groups are inconsistent with boundary triangles.",
        ));
    }
    let node_count = nodes.len();
    if cells
        .iter()
        .flatten()
        .chain(boundary_triangles.iter().flatten())
        .any(|index| *index as usize >= node_count)
    {
        return Err(AppError::validation(
            "FEM mesh topology contains out-of-range node indices.",
        ));
    }
    Ok(FemTopologyMeshData {
        mesh: ecky_fem::FemIndexedTet4Mesh {
            schema_version: ecky_fem::FEM_SCHEMA_VERSION,
            nodes,
            cells,
        },
        boundary_triangles,
        boundary_face_group_indices,
        face_group_count: asset.face_group_count,
    })
}

fn load_fem_mesh_asset_from_dir(
    directory: &Path,
    expected_analysis_digest: &str,
    maximum_result_bytes: u64,
) -> AppResult<FemMeshAsset> {
    let manifest_path = directory.join(FEM_RESULT_MANIFEST);
    let manifest_bytes = read_bounded(&manifest_path, maximum_result_bytes)?;
    let stored: StoredFemMeshAsset =
        crate::strict_edn::from_slice(&manifest_bytes).map_err(|error| {
            AppError::validation(format!(
                "FEM mesh manifest '{}' is invalid: {error}",
                manifest_path.display()
            ))
        })?;
    if stored.schema_version != FEM_MESH_ASSET_SCHEMA_VERSION {
        return Err(AppError::validation(format!(
            "FEM mesh schemaVersion {} is unsupported.",
            stored.schema_version
        )));
    }
    if stored.analysis_identity_digest != expected_analysis_digest {
        return Err(AppError::conflict(
            "FEM mesh is stale for the requested analysis identity.",
        ));
    }
    validate_asset_arrays(directory, &stored.arrays, maximum_result_bytes)?;
    Ok(FemMeshAsset {
        schema_version: stored.schema_version,
        analysis_identity_digest: stored.analysis_identity_digest,
        mesh_content_digest: stored.mesh_content_digest,
        source_boundary_digest: stored.source_boundary_digest,
        mesh_quality: stored.mesh_quality,
        face_group_count: stored.face_group_count,
        arrays: stored.arrays,
        manifest_path,
    })
}

pub fn publish_fem_result_asset(
    app: &dyn PathResolver,
    result: &FemPipelineResult,
    source_digest: &str,
    maximum_result_bytes: u64,
) -> AppResult<FemResultAsset> {
    if maximum_result_bytes == 0 {
        return Err(AppError::validation(
            "FEM result artifact byte budget must be positive.",
        ));
    }
    let analysis_digest = result.analysis_identity.canonical_digest();
    let result_root = app.app_data_dir().join(FEM_RESULT_ROOT);
    fs::create_dir_all(&result_root).map_err(|error| {
        AppError::persistence(format!(
            "FEM result root '{}' could not be created: {error}",
            result_root.display()
        ))
    })?;
    let final_dir = result_root
        .join(digest_component(&analysis_digest)?)
        .join(digest_component(&result.solution.solution_digest)?);
    if final_dir.is_dir() {
        return load_fem_result_asset_from_dir(
            &final_dir,
            Some(&analysis_digest),
            Some(&result.solution.solution_digest),
            maximum_result_bytes,
        );
    }

    let nonce = FEM_ARTIFACT_NONCE.fetch_add(1, Ordering::Relaxed);
    let temporary_dir = result_root.join(format!(".publishing-{}-{nonce}", std::process::id()));
    fs::create_dir(&temporary_dir).map_err(|error| {
        AppError::persistence(format!(
            "FEM temporary artifact directory '{}' could not be created: {error}",
            temporary_dir.display()
        ))
    })?;
    let temporary = TemporaryDirectory::new(temporary_dir.clone());
    let arrays_dir = temporary_dir.join("arrays");
    fs::create_dir(&arrays_dir).map_err(|error| {
        AppError::persistence(format!(
            "FEM array directory '{}' could not be created: {error}",
            arrays_dir.display()
        ))
    })?;

    let arrays = vec![
        write_f64_array(
            &temporary_dir,
            "nodesMm",
            "arrays/nodes.f64le",
            &[result.mesh.nodes.len() as u64, 3],
            result
                .mesh
                .nodes
                .iter()
                .flat_map(|point| [point.x_mm, point.y_mm, point.z_mm]),
        )?,
        write_u32_array(
            &temporary_dir,
            "tet4Cells",
            "arrays/tet4-cells.u32le",
            &[result.mesh.cells.len() as u64, 4],
            result.mesh.cells.iter().flatten().copied(),
        )?,
        write_u32_array(
            &temporary_dir,
            "boundaryTriangles",
            "arrays/boundary-triangles.u32le",
            &[result.mesh.boundary_triangles.len() as u64, 3],
            result.mesh.boundary_triangles.iter().flatten().copied(),
        )?,
        write_u32_array(
            &temporary_dir,
            "boundaryFaceGroupIndices",
            "arrays/boundary-face-groups.u32le",
            &[result.mesh.boundary_face_group_indices.len() as u64],
            result.mesh.boundary_face_group_indices.iter().copied(),
        )?,
        write_f64_array(
            &temporary_dir,
            "displacementMm",
            "arrays/displacement.f64le",
            &[result.mesh.nodes.len() as u64, 3],
            result.solution.displacement_dofs_mm.iter().copied(),
        )?,
        write_f64_array(
            &temporary_dir,
            "elementVonMisesMpa",
            "arrays/element-von-mises.f64le",
            &[result.solution.postprocess.elements.len() as u64],
            result
                .solution
                .postprocess
                .elements
                .iter()
                .map(|element| element.von_mises_mpa),
        )?,
        write_f64_array(
            &temporary_dir,
            "nodalDisplayVonMisesMpa",
            "arrays/nodal-von-mises.f64le",
            &[result.solution.postprocess.nodal_display.len() as u64],
            result
                .solution
                .postprocess
                .nodal_display
                .iter()
                .map(|node| node.volume_weighted_von_mises_mpa),
        )?,
    ];
    enforce_total_budget(&arrays, maximum_result_bytes)?;

    let idealization_artifact =
        FemIdealizationArtifact::from_record(&result.engineering_evidence.idealization).map_err(
            |error| AppError::validation(format!("FEM idealization artifact is invalid: {error}")),
        )?;
    let idealization_artifact_digest = idealization_artifact.canonical_digest();
    let stored = StoredFemResultAsset {
        schema_version: FEM_RESULT_ASSET_SCHEMA_VERSION,
        source_digest: source_digest.to_string(),
        analysis_identity_digest: analysis_digest.clone(),
        solution_digest: result.solution.solution_digest.clone(),
        result_digest: result.solution.postprocess.result_digest.clone(),
        mesh_content_digest: result.mesh.content_digest.clone(),
        source_boundary_digest: result.mesh.source_boundary_digest.clone(),
        engineering_evidence_digest: result.engineering_evidence.canonical_digest(),
        engineering_evidence: result.engineering_evidence.clone(),
        idealization_artifact_digest,
        idealization_artifact,
        decision_ready: result.decision_readiness_error.is_none(),
        decision_readiness_error: result.decision_readiness_error.clone(),
        mesh_quality: result.mesh.quality.clone(),
        summary: result.solution.postprocess.summary.clone(),
        equilibrium_relative_imbalance: result.solution.equilibrium.relative_imbalance,
        solver_relative_residual: result.solution.linear_solve.relative_residual,
        support_reactions: result.solution.support_reactions.clone(),
        acceptance_evaluations: result.acceptance_evaluations.clone(),
        arrays,
    };
    let manifest_bytes = crate::strict_edn::to_vec(&stored).map_err(|error| {
        AppError::internal(format!("FEM result manifest serialization failed: {error}"))
    })?;
    if manifest_bytes.len() as u64 > maximum_result_bytes {
        return Err(AppError::validation(format!(
            "FEM result manifest exceeds byte budget: observed {}, allowed {maximum_result_bytes}.",
            manifest_bytes.len()
        )));
    }
    write_bytes(&temporary_dir.join(FEM_RESULT_MANIFEST), &manifest_bytes)?;
    fs::create_dir_all(final_dir.parent().expect("digest directory has parent")).map_err(
        |error| {
            AppError::persistence(format!(
                "FEM analysis artifact directory could not be created: {error}"
            ))
        },
    )?;
    match fs::rename(&temporary_dir, &final_dir) {
        Ok(()) => temporary.disarm(),
        Err(_error) if final_dir.is_dir() => {
            drop(temporary);
            return load_fem_result_asset_from_dir(
                &final_dir,
                Some(&analysis_digest),
                Some(&result.solution.solution_digest),
                maximum_result_bytes,
            );
        }
        Err(error) => {
            return Err(AppError::persistence(format!(
                "FEM result publication '{}' failed atomically: {error}",
                final_dir.display()
            )));
        }
    }
    load_fem_result_asset_from_dir(
        &final_dir,
        Some(&analysis_digest),
        Some(&result.solution.solution_digest),
        maximum_result_bytes,
    )
}

pub fn load_fem_result_asset(
    app: &dyn PathResolver,
    analysis_identity_digest: &str,
    solution_digest: &str,
    maximum_result_bytes: u64,
) -> AppResult<FemResultAsset> {
    let directory = app
        .app_data_dir()
        .join(FEM_RESULT_ROOT)
        .join(digest_component(analysis_identity_digest)?)
        .join(digest_component(solution_digest)?);
    load_fem_result_asset_from_dir(
        &directory,
        Some(analysis_identity_digest),
        Some(solution_digest),
        maximum_result_bytes,
    )
}

pub fn export_fem_result_vtu(
    asset: &FemResultAsset,
    target_path: impl AsRef<Path>,
    maximum_output_bytes: u64,
) -> AppResult<(u64, String)> {
    if maximum_output_bytes == 0 {
        return Err(AppError::validation(
            "FEM VTU output byte budget must be positive.",
        ));
    }
    let target_path = target_path.as_ref();
    if target_path.extension().and_then(|value| value.to_str()) != Some("vtu") {
        return Err(AppError::validation(
            "FEM result export target must end in '.vtu'.",
        ));
    }
    let directory = asset
        .manifest_path
        .parent()
        .ok_or_else(|| AppError::validation("FEM result manifest has no artifact directory."))?;
    validate_asset_arrays(directory, &asset.arrays, maximum_output_bytes)?;
    let nodes = read_f64_asset(directory, required_array(&asset.arrays, "nodesMm")?)?;
    let cells = read_u32_asset(directory, required_array(&asset.arrays, "tet4Cells")?)?;
    let displacement = read_f64_asset(directory, required_array(&asset.arrays, "displacementMm")?)?;
    let nodal_stress = read_f64_asset(
        directory,
        required_array(&asset.arrays, "nodalDisplayVonMisesMpa")?,
    )?;
    let element_stress = read_f64_asset(
        directory,
        required_array(&asset.arrays, "elementVonMisesMpa")?,
    )?;
    let node_count = nodes.len() / 3;
    let cell_count = cells.len() / 4;
    if displacement.len() != nodes.len()
        || nodal_stress.len() != node_count
        || element_stress.len() != cell_count
    {
        return Err(AppError::validation(
            "FEM VTU arrays have inconsistent point/cell cardinality.",
        ));
    }

    let parent = target_path
        .parent()
        .ok_or_else(|| AppError::validation("FEM VTU target must have a parent directory."))?;
    fs::create_dir_all(parent).map_err(|error| {
        AppError::persistence(format!("FEM VTU target directory create failed: {error}"))
    })?;
    let temporary_path = parent.join(format!(
        ".ecky-fem-vtu-{}-{}",
        std::process::id(),
        FEM_ARTIFACT_NONCE.fetch_add(1, Ordering::Relaxed)
    ));
    let file = File::create(&temporary_path).map_err(|error| {
        AppError::persistence(format!("FEM VTU temporary file create failed: {error}"))
    })?;
    let mut writer = BudgetWriter::new(BufWriter::new(file), maximum_output_bytes);
    let write_result = (|| -> std::io::Result<()> {
        writeln!(writer, "<?xml version=\"1.0\"?>")?;
        writeln!(
            writer,
            "<VTKFile type=\"UnstructuredGrid\" version=\"0.1\" byte_order=\"LittleEndian\">"
        )?;
        writeln!(writer, "<UnstructuredGrid><Piece NumberOfPoints=\"{node_count}\" NumberOfCells=\"{cell_count}\">")?;
        writeln!(
            writer,
            "<Points><DataArray type=\"Float64\" NumberOfComponents=\"3\" format=\"ascii\">"
        )?;
        write_values(&mut writer, nodes.iter())?;
        writeln!(writer, "</DataArray></Points>")?;
        writeln!(
            writer,
            "<Cells><DataArray type=\"UInt32\" Name=\"connectivity\" format=\"ascii\">"
        )?;
        write_values(&mut writer, cells.iter())?;
        writeln!(
            writer,
            "</DataArray><DataArray type=\"UInt32\" Name=\"offsets\" format=\"ascii\">"
        )?;
        write_values(&mut writer, (1..=cell_count).map(|index| index * 4))?;
        writeln!(
            writer,
            "</DataArray><DataArray type=\"UInt8\" Name=\"types\" format=\"ascii\">"
        )?;
        write_values(&mut writer, std::iter::repeat_n(10, cell_count))?;
        writeln!(writer, "</DataArray></Cells>")?;
        writeln!(
            writer,
            "<PointData Vectors=\"DisplacementMm\" Scalars=\"NodalDisplayVonMisesMpa\">"
        )?;
        writeln!(writer, "<DataArray type=\"Float64\" Name=\"DisplacementMm\" NumberOfComponents=\"3\" format=\"ascii\">")?;
        write_values(&mut writer, displacement.iter())?;
        writeln!(writer, "</DataArray><DataArray type=\"Float64\" Name=\"NodalDisplayVonMisesMpa\" format=\"ascii\">")?;
        write_values(&mut writer, nodal_stress.iter())?;
        writeln!(writer, "</DataArray></PointData>")?;
        writeln!(writer, "<CellData Scalars=\"ElementVonMisesMpa\"><DataArray type=\"Float64\" Name=\"ElementVonMisesMpa\" format=\"ascii\">")?;
        write_values(&mut writer, element_stress.iter())?;
        writeln!(
            writer,
            "</DataArray></CellData></Piece></UnstructuredGrid></VTKFile>"
        )?;
        writer.flush()
    })();
    if let Err(error) = write_result {
        let _ = fs::remove_file(&temporary_path);
        let message = if error.kind() == std::io::ErrorKind::FileTooLarge {
            format!("FEM VTU output exceeds byte budget {maximum_output_bytes}.")
        } else {
            format!("FEM VTU write failed: {error}")
        };
        return Err(AppError::persistence(message));
    }
    let byte_length = writer.written;
    drop(writer);
    fs::rename(&temporary_path, target_path).map_err(|error| {
        let _ = fs::remove_file(&temporary_path);
        AppError::persistence(format!("FEM VTU atomic publication failed: {error}"))
    })?;
    let digest = sha256_file(target_path)?;
    Ok((byte_length, digest))
}

fn required_array<'a>(
    arrays: &'a [FemBinaryArrayAsset],
    name: &str,
) -> AppResult<&'a FemBinaryArrayAsset> {
    arrays
        .iter()
        .find(|array| array.name == name)
        .ok_or_else(|| AppError::validation(format!("FEM VTU export requires array '{name}'.")))
}

fn read_f64_asset(directory: &Path, array: &FemBinaryArrayAsset) -> AppResult<Vec<f64>> {
    if array.scalar_type != FemScalarType::Float64Le {
        return Err(AppError::validation(format!(
            "FEM VTU array '{}' must use float64Le.",
            array.name
        )));
    }
    let bytes = fs::read(directory.join(&array.path)).map_err(|error| {
        AppError::persistence(format!(
            "FEM VTU array '{}' read failed: {error}",
            array.name
        ))
    })?;
    let values = bytes
        .as_chunks::<8>()
        .0
        .iter()
        .map(|chunk| f64::from_le_bytes(*chunk))
        .collect::<Vec<_>>();
    if values.iter().any(|value| !value.is_finite()) {
        return Err(AppError::validation(format!(
            "FEM VTU array '{}' contains non-finite values.",
            array.name
        )));
    }
    Ok(values)
}

fn read_u32_asset(directory: &Path, array: &FemBinaryArrayAsset) -> AppResult<Vec<u32>> {
    if array.scalar_type != FemScalarType::Uint32Le {
        return Err(AppError::validation(format!(
            "FEM VTU array '{}' must use uint32Le.",
            array.name
        )));
    }
    let bytes = fs::read(directory.join(&array.path)).map_err(|error| {
        AppError::persistence(format!(
            "FEM VTU array '{}' read failed: {error}",
            array.name
        ))
    })?;
    Ok(bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|chunk| u32::from_le_bytes(*chunk))
        .collect())
}

fn write_values<T: std::fmt::Display>(
    writer: &mut impl Write,
    values: impl IntoIterator<Item = T>,
) -> std::io::Result<()> {
    let mut first = true;
    for value in values {
        if !first {
            writer.write_all(b" ")?;
        }
        first = false;
        write!(writer, "{value}")?;
    }
    writeln!(writer)
}

struct BudgetWriter<W> {
    inner: W,
    maximum: u64,
    written: u64,
}

impl<W> BudgetWriter<W> {
    fn new(inner: W, maximum: u64) -> Self {
        Self {
            inner,
            maximum,
            written: 0,
        }
    }
}

impl<W: Write> Write for BudgetWriter<W> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        let next = self.written.saturating_add(bytes.len() as u64);
        if next > self.maximum {
            return Err(std::io::Error::new(
                std::io::ErrorKind::FileTooLarge,
                "VTU byte budget exceeded",
            ));
        }
        let count = self.inner.write(bytes)?;
        self.written += count as u64;
        Ok(count)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

fn load_fem_result_asset_from_dir(
    directory: &Path,
    expected_analysis_digest: Option<&str>,
    expected_solution_digest: Option<&str>,
    maximum_result_bytes: u64,
) -> AppResult<FemResultAsset> {
    let manifest_path = directory.join(FEM_RESULT_MANIFEST);
    let manifest_bytes = read_bounded(&manifest_path, maximum_result_bytes)?;
    let stored: StoredFemResultAsset =
        crate::strict_edn::from_slice(&manifest_bytes).map_err(|error| {
            AppError::validation(format!(
                "FEM result manifest '{}' is invalid: {error}",
                manifest_path.display()
            ))
        })?;
    if stored.schema_version != FEM_RESULT_ASSET_SCHEMA_VERSION {
        return Err(AppError::validation(format!(
            "FEM result schemaVersion {} is unsupported.",
            stored.schema_version
        )));
    }
    if stored.source_digest.trim().is_empty() {
        return Err(AppError::validation(
            "FEM result source identity must be populated.",
        ));
    }
    validate_stored_result_integrity(
        expected_analysis_digest,
        expected_solution_digest,
        &stored.analysis_identity_digest,
        &stored.solution_digest,
        &stored.engineering_evidence_digest,
        &stored.engineering_evidence,
        stored.decision_ready,
        stored.decision_readiness_error.as_deref(),
        &stored.acceptance_evaluations,
        &stored.mesh_content_digest,
        &stored.result_digest,
    )?;
    stored.idealization_artifact.validate().map_err(|error| {
        AppError::validation(format!("FEM idealization artifact is invalid: {error}"))
    })?;
    if stored.idealization_artifact.canonical_digest() != stored.idealization_artifact_digest {
        return Err(AppError::validation(
            "FEM idealization artifact digest does not match its immutable content.",
        ));
    }
    validate_asset_arrays(directory, &stored.arrays, maximum_result_bytes)?;
    Ok(FemResultAsset {
        schema_version: stored.schema_version,
        source_digest: stored.source_digest,
        analysis_identity_digest: stored.analysis_identity_digest,
        solution_digest: stored.solution_digest,
        result_digest: stored.result_digest,
        mesh_content_digest: stored.mesh_content_digest,
        source_boundary_digest: stored.source_boundary_digest,
        engineering_evidence_digest: stored.engineering_evidence_digest,
        engineering_evidence: stored.engineering_evidence,
        idealization_artifact_digest: stored.idealization_artifact_digest,
        idealization_artifact: stored.idealization_artifact,
        decision_ready: stored.decision_ready,
        decision_readiness_error: stored.decision_readiness_error,
        mesh_quality: stored.mesh_quality,
        summary: stored.summary,
        equilibrium_relative_imbalance: stored.equilibrium_relative_imbalance,
        solver_relative_residual: stored.solver_relative_residual,
        support_reactions: stored.support_reactions,
        acceptance_evaluations: stored.acceptance_evaluations,
        arrays: stored.arrays,
        manifest_path,
    })
}

#[allow(clippy::too_many_arguments)]
fn validate_stored_result_integrity(
    expected_analysis_digest: Option<&str>,
    expected_solution_digest: Option<&str>,
    analysis_identity_digest: &str,
    solution_digest: &str,
    engineering_evidence_digest: &str,
    engineering_evidence: &FemEngineeringEvidenceLedger,
    decision_ready: bool,
    decision_readiness_error: Option<&str>,
    acceptance_evaluations: &[FemAcceptanceEvaluation],
    mesh_content_digest: &str,
    result_digest: &str,
) -> AppResult<()> {
    if expected_analysis_digest.is_some_and(|expected| expected != analysis_identity_digest) {
        return Err(AppError::conflict(
            "FEM result is stale for the requested analysis identity.",
        ));
    }
    if expected_solution_digest.is_some_and(|expected| expected != solution_digest) {
        return Err(AppError::conflict(
            "FEM result solution identity differs from requested immutable artifact.",
        ));
    }
    engineering_evidence.validate().map_err(|error| {
        AppError::validation(format!(
            "FEM result engineering evidence is invalid: {error}"
        ))
    })?;
    let observed_evidence_digest = engineering_evidence.canonical_digest();
    if observed_evidence_digest != engineering_evidence_digest {
        return Err(AppError::validation(format!(
            "FEM result engineering evidence digest does not match immutable content: observed '{observed_evidence_digest}', expected '{engineering_evidence_digest}'."
        )));
    }
    for evaluation in acceptance_evaluations {
        if evaluation.analysis_identity_digest != analysis_identity_digest
            || evaluation.mesh_content_digest != mesh_content_digest
            || evaluation.result_digest != result_digest
        {
            return Err(AppError::validation(format!(
                "FEM acceptance evaluation '{}' identity differs from result manifest.",
                evaluation.metric_id
            )));
        }
    }
    let computed_error = acceptance_evaluations
        .iter()
        .find(|evaluation| evaluation.status != "passed")
        .map(|evaluation| evaluation.detail.clone())
        .or_else(|| {
            engineering_evidence
                .validate_decision_readiness()
                .err()
                .map(|error| error.to_string())
        });
    if decision_ready != computed_error.is_none()
        || decision_readiness_error != computed_error.as_deref()
    {
        return Err(AppError::validation(
            "FEM result decision readiness differs from immutable evidence and acceptance evaluations.",
        ));
    }
    Ok(())
}

fn validate_asset_arrays(
    directory: &Path,
    arrays: &[FemBinaryArrayAsset],
    maximum_result_bytes: u64,
) -> AppResult<()> {
    enforce_total_budget(arrays, maximum_result_bytes)?;
    let mut names = std::collections::BTreeSet::new();
    for array in arrays {
        if array.name.trim().is_empty() || !names.insert(array.name.as_str()) {
            return Err(AppError::validation(
                "FEM artifact array names must be non-empty and unique.",
            ));
        }
        validate_relative_path(&array.path)?;
        let expected_length = array
            .shape
            .iter()
            .try_fold(array.scalar_type.byte_width(), |bytes, dimension| {
                bytes.checked_mul(*dimension)
            })
            .ok_or_else(|| {
                AppError::validation("FEM artifact array shape byte count overflowed.")
            })?;
        if array.shape.is_empty()
            || array.shape.contains(&0)
            || expected_length != array.byte_length
        {
            return Err(AppError::validation(format!(
                "FEM artifact array '{}' shape/byteLength mismatch.",
                array.name
            )));
        }
        let path = directory.join(&array.path);
        let metadata = fs::metadata(&path).map_err(|error| {
            AppError::persistence(format!(
                "FEM artifact array '{}' is missing: {error}",
                path.display()
            ))
        })?;
        if !metadata.is_file() || metadata.len() != array.byte_length {
            return Err(AppError::validation(format!(
                "FEM artifact array '{}' byte length differs from manifest.",
                array.name
            )));
        }
        let observed = sha256_file(&path)?;
        if observed != array.sha256 {
            return Err(AppError::validation(format!(
                "FEM artifact array '{}' digest mismatch: observed '{observed}', expected '{}'.",
                array.name, array.sha256
            )));
        }
    }
    Ok(())
}

fn write_f64_array(
    root: &Path,
    name: &str,
    relative_path: &str,
    shape: &[u64],
    values: impl IntoIterator<Item = f64>,
) -> AppResult<FemBinaryArrayAsset> {
    let mut bytes = Vec::new();
    for value in values {
        if !value.is_finite() {
            return Err(AppError::validation(format!(
                "FEM result array '{name}' contains a non-finite value."
            )));
        }
        bytes.extend(value.to_le_bytes());
    }
    write_array(
        root,
        name,
        relative_path,
        FemScalarType::Float64Le,
        shape,
        bytes,
    )
}

fn write_u32_array(
    root: &Path,
    name: &str,
    relative_path: &str,
    shape: &[u64],
    values: impl IntoIterator<Item = u32>,
) -> AppResult<FemBinaryArrayAsset> {
    let mut bytes = Vec::new();
    for value in values {
        bytes.extend(value.to_le_bytes());
    }
    write_array(
        root,
        name,
        relative_path,
        FemScalarType::Uint32Le,
        shape,
        bytes,
    )
}

fn write_array(
    root: &Path,
    name: &str,
    relative_path: &str,
    scalar_type: FemScalarType,
    shape: &[u64],
    bytes: Vec<u8>,
) -> AppResult<FemBinaryArrayAsset> {
    let path = PathBuf::from(relative_path);
    validate_relative_path(&path)?;
    let expected = shape
        .iter()
        .try_fold(scalar_type.byte_width(), |size, dimension| {
            size.checked_mul(*dimension)
        })
        .ok_or_else(|| AppError::validation("FEM result array shape byte count overflowed."))?;
    if shape.is_empty() || shape.contains(&0) || expected != bytes.len() as u64 {
        return Err(AppError::internal(format!(
            "FEM result array '{name}' producer shape mismatch."
        )));
    }
    let sha256 = format!("sha256:{:x}", Sha256::digest(&bytes));
    write_bytes(&root.join(&path), &bytes)?;
    Ok(FemBinaryArrayAsset {
        name: name.to_string(),
        path,
        scalar_type,
        shape: shape.to_vec(),
        byte_length: bytes.len() as u64,
        sha256,
    })
}

fn write_bytes(path: &Path, bytes: &[u8]) -> AppResult<()> {
    let mut file = File::create(path).map_err(|error| {
        AppError::persistence(format!(
            "FEM artifact '{}' create failed: {error}",
            path.display()
        ))
    })?;
    file.write_all(bytes).map_err(|error| {
        AppError::persistence(format!(
            "FEM artifact '{}' write failed: {error}",
            path.display()
        ))
    })?;
    file.sync_all().map_err(|error| {
        AppError::persistence(format!(
            "FEM artifact '{}' sync failed: {error}",
            path.display()
        ))
    })
}

fn read_bounded(path: &Path, maximum_bytes: u64) -> AppResult<Vec<u8>> {
    let metadata = fs::metadata(path).map_err(|error| {
        AppError::persistence(format!(
            "FEM artifact '{}' is missing: {error}",
            path.display()
        ))
    })?;
    if !metadata.is_file() || metadata.len() > maximum_bytes {
        return Err(AppError::validation(format!(
            "FEM artifact '{}' exceeds byte budget or is not a file.",
            path.display()
        )));
    }
    fs::read(path).map_err(|error| {
        AppError::persistence(format!(
            "FEM artifact '{}' read failed: {error}",
            path.display()
        ))
    })
}

fn enforce_total_budget(arrays: &[FemBinaryArrayAsset], maximum_bytes: u64) -> AppResult<()> {
    let total = arrays
        .iter()
        .try_fold(0_u64, |total, array| total.checked_add(array.byte_length));
    let Some(total) = total else {
        return Err(AppError::validation(
            "FEM result array byte count overflowed.",
        ));
    };
    if total > maximum_bytes {
        Err(AppError::validation(format!(
            "FEM result array budget exceeded: observed {total}, allowed {maximum_bytes}."
        )))
    } else {
        Ok(())
    }
}

fn validate_relative_path(path: &Path) -> AppResult<()> {
    if path.as_os_str().is_empty()
        || path.is_absolute()
        || !path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
    {
        Err(AppError::validation(format!(
            "FEM result path '{}' must be confined and relative.",
            path.display()
        )))
    } else {
        Ok(())
    }
}

fn digest_component(digest: &str) -> AppResult<&str> {
    let Some(hex) = digest.strip_prefix("sha256:") else {
        return Err(AppError::validation(
            "FEM artifact identity must use sha256 prefix.",
        ));
    };
    if hex.len() != 64 || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(AppError::validation(
            "FEM artifact identity must contain 64 hexadecimal characters.",
        ));
    }
    Ok(hex)
}

fn sha256_file(path: &Path) -> AppResult<String> {
    let mut reader = BufReader::new(File::open(path).map_err(|error| {
        AppError::persistence(format!(
            "FEM artifact '{}' open failed: {error}",
            path.display()
        ))
    })?);
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = reader.read(&mut buffer).map_err(|error| {
            AppError::persistence(format!(
                "FEM artifact '{}' hash failed: {error}",
                path.display()
            ))
        })?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(format!("sha256:{:x}", hasher.finalize()))
}

struct TemporaryDirectory {
    path: PathBuf,
    armed: std::sync::Mutex<bool>,
}

impl TemporaryDirectory {
    fn new(path: PathBuf) -> Self {
        Self {
            path,
            armed: std::sync::Mutex::new(true),
        }
    }

    fn disarm(&self) {
        if let Ok(mut armed) = self.armed.lock() {
            *armed = false;
        }
    }
}

impl Drop for TemporaryDirectory {
    fn drop(&mut self) {
        if self.armed.lock().map(|armed| *armed).unwrap_or(true) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fem_persistent_manifests_are_strict_edn_only() {
        assert_eq!(FEM_RESULT_MANIFEST, "manifest.edn");
    }

    fn test_extremum(
        field_kind: ecky_fem::FemResultFieldKind,
        value: f64,
        unit: &str,
    ) -> ecky_fem::FemResultExtremum {
        ecky_fem::FemResultExtremum {
            field_kind,
            value,
            unit: unit.to_string(),
            node_id: Some(0),
            element_id: None,
            coordinate_mm: ecky_fem::FemPoint3::new(0.0, 0.0, 0.0),
            mesh_content_digest: "sha256:mesh".to_string(),
            source_boundary_digest: "sha256:boundary".to_string(),
        }
    }

    #[test]
    fn vtu_export_contains_tet4_displacement_and_separate_nodal_and_element_stress() {
        let root = std::env::temp_dir().join(format!(
            "ecky-fem-vtu-test-{}-{}",
            std::process::id(),
            FEM_ARTIFACT_NONCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(root.join("arrays")).unwrap();
        let arrays = vec![
            write_f64_array(
                &root,
                "nodesMm",
                "arrays/nodes.bin",
                &[4, 3],
                [0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0],
            )
            .unwrap(),
            write_u32_array(
                &root,
                "tet4Cells",
                "arrays/cells.bin",
                &[1, 4],
                [0, 1, 2, 3],
            )
            .unwrap(),
            write_f64_array(
                &root,
                "displacementMm",
                "arrays/displacement.bin",
                &[4, 3],
                [0.0; 12],
            )
            .unwrap(),
            write_f64_array(
                &root,
                "nodalDisplayVonMisesMpa",
                "arrays/nodal-stress.bin",
                &[4],
                [1.0, 2.0, 3.0, 4.0],
            )
            .unwrap(),
            write_f64_array(
                &root,
                "elementVonMisesMpa",
                "arrays/element-stress.bin",
                &[1],
                [5.0],
            )
            .unwrap(),
        ];
        let asset = FemResultAsset {
            schema_version: FEM_RESULT_ASSET_SCHEMA_VERSION,
            source_digest: "sha256:source".to_string(),
            analysis_identity_digest: "sha256:analysis".to_string(),
            solution_digest: "sha256:solution".to_string(),
            result_digest: "sha256:result".to_string(),
            mesh_content_digest: "sha256:mesh".to_string(),
            source_boundary_digest: "sha256:boundary".to_string(),
            engineering_evidence_digest: "sha256:evidence".to_string(),
            engineering_evidence: serde_json::from_value(serde_json::json!({
                "schemaVersion": ecky_fem::FEM_SCHEMA_VERSION,
                "question": {"questionId":"q", "statement":"question", "decision":"decide", "acceptanceMetricIds":["metric"]},
                "acceptanceCriteria": [{"metricId":"metric", "field":"maximumDisplacement", "comparison":"lessThanOrEqual", "limit":1.0, "unit":"mm", "requiresConvergence":false}],
                "idealization": {"sourceGeometryDigest":"sha256:geometry", "analysisGeometryDigest":"sha256:geometry", "affectedTopologyIds":[], "justification":"exact solid", "expectedInfluencePercent":0.0, "acceptedByUser":true},
                "evidence": [], "inputBindings": [], "assumptions": [], "applicabilityChecks": [],
                "sensitivity": null, "validationEvidence": []
            })).expect("test engineering evidence ledger"),
            idealization_artifact_digest: ecky_fem::FemIdealizationArtifact::from_record(
                &ecky_fem::FemIdealizationRecord {
                    source_geometry_digest: "sha256:geometry".into(),
                    analysis_geometry_digest: "sha256:geometry".into(),
                    affected_topology_ids: vec![],
                    justification: "exact solid".into(),
                    expected_influence_percent: 0.0,
                    accepted_by_user: true,
                },
            )
            .unwrap()
            .canonical_digest(),
            idealization_artifact: ecky_fem::FemIdealizationArtifact::from_record(
                &ecky_fem::FemIdealizationRecord {
                    source_geometry_digest: "sha256:geometry".into(),
                    analysis_geometry_digest: "sha256:geometry".into(),
                    affected_topology_ids: vec![],
                    justification: "exact solid".into(),
                    expected_influence_percent: 0.0,
                    accepted_by_user: true,
                },
            )
            .unwrap(),
            decision_ready: false,
            decision_readiness_error: Some("physical validation missing".to_string()),
            mesh_quality: ecky_fem::FemMeshQuality {
                minimum_signed_volume_mm3: 1.0 / 6.0,
                maximum_signed_volume_mm3: 1.0 / 6.0,
                minimum_scaled_jacobian: 0.2,
                minimum_radius_ratio: 0.1,
                worst_cell_index: 0,
                worst_cell_centroid_mm: ecky_fem::FemPoint3::new(0.25, 0.25, 0.25),
                connected_component_count: 1,
                boundary_area_mm2_by_group: vec![0.5],
            },
            summary: ecky_fem::FemResultSummary {
                maximum_displacement: test_extremum(
                    ecky_fem::FemResultFieldKind::DisplacementMagnitude,
                    0.0,
                    "mm",
                ),
                maximum_von_mises: test_extremum(
                    ecky_fem::FemResultFieldKind::VonMisesStress,
                    5.0,
                    "MPa",
                ),
                maximum_principal_stress: test_extremum(
                    ecky_fem::FemResultFieldKind::PrincipalStressMaximum,
                    5.0,
                    "MPa",
                ),
                volume_mm3: 1.0 / 6.0,
                mass_kg: 1.0,
                minimum_yield_safety_factor: ecky_fem::FemSafetyFactor::Finite { value: 2.0 },
            },
            equilibrium_relative_imbalance: 0.0,
            solver_relative_residual: 0.0,
            support_reactions: vec![],
            acceptance_evaluations: vec![],
            arrays,
            manifest_path: root.join(FEM_RESULT_MANIFEST),
        };
        let target = root.join("result.vtu");
        let (bytes, digest) = export_fem_result_vtu(&asset, &target, 1024 * 1024).unwrap();
        let text = fs::read_to_string(&target).unwrap();
        assert!(bytes > 0);
        assert!(digest.starts_with("sha256:"));
        assert!(text.contains("NumberOfPoints=\"4\" NumberOfCells=\"1\""));
        assert!(text.contains("Name=\"DisplacementMm\""));
        assert!(text.contains("Name=\"NodalDisplayVonMisesMpa\""));
        assert!(text.contains("Name=\"ElementVonMisesMpa\""));
        assert!(export_fem_result_vtu(&asset, root.join("too-small.vtu"), 64).is_err());
        assert!(!root.join("too-small.vtu").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn binary_array_manifest_rejects_corruption_truncation_and_budget_excess() {
        let root = std::env::temp_dir().join(format!(
            "ecky-fem-artifact-test-{}-{}",
            std::process::id(),
            FEM_ARTIFACT_NONCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(root.join("arrays")).unwrap();
        let array = write_u32_array(
            &root,
            "tet4Cells",
            "arrays/tet4-cells.u32le",
            &[1, 4],
            [0, 1, 2, 3],
        )
        .unwrap();
        validate_asset_arrays(&root, std::slice::from_ref(&array), 16).unwrap();
        assert!(enforce_total_budget(std::slice::from_ref(&array), 15)
            .unwrap_err()
            .message
            .contains("observed 16, allowed 15"));

        let path = root.join(&array.path);
        let mut corrupted = fs::read(&path).unwrap();
        corrupted[0] ^= 0xff;
        fs::write(&path, &corrupted).unwrap();
        assert!(
            validate_asset_arrays(&root, std::slice::from_ref(&array), 16)
                .unwrap_err()
                .message
                .contains("digest mismatch")
        );

        fs::write(&path, &corrupted[..12]).unwrap();
        assert!(
            validate_asset_arrays(&root, std::slice::from_ref(&array), 16)
                .unwrap_err()
                .message
                .contains("byte length differs")
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn immutable_result_manifest_rejects_identity_and_readiness_tampering() {
        let ledger: FemEngineeringEvidenceLedger = serde_json::from_value(serde_json::json!({
            "schemaVersion": ecky_fem::FEM_SCHEMA_VERSION,
            "question": {"questionId":"q", "statement":"question", "decision":"decide", "acceptanceMetricIds":["metric"]},
            "acceptanceCriteria": [{"metricId":"metric", "field":"maximumDisplacement", "comparison":"lessThanOrEqual", "limit":1.0, "unit":"mm", "requiresConvergence":false}],
            "idealization": {"sourceGeometryDigest":"sha256:geometry", "analysisGeometryDigest":"sha256:geometry", "affectedTopologyIds":[], "justification":"exact solid", "expectedInfluencePercent":0.0, "acceptedByUser":true},
            "evidence": [], "inputBindings": [], "assumptions": [],
            "applicabilityChecks": [{"checkId":"one-solid", "kind":"oneSolidScope", "status":"pass", "observed":1.0, "limit":1.0, "unit":"solid", "evidenceIds":[], "detail":"One connected solid."}],
            "sensitivity": null, "validationEvidence": []
        }))
        .unwrap();
        let readiness_error = ledger
            .validate_decision_readiness()
            .expect_err("fixture is red")
            .to_string();
        let evidence_digest = ledger.canonical_digest();

        validate_stored_result_integrity(
            Some("sha256:analysis"),
            Some("sha256:solution"),
            "sha256:analysis",
            "sha256:solution",
            &evidence_digest,
            &ledger,
            false,
            Some(readiness_error.as_str()),
            &[],
            "sha256:mesh",
            "sha256:result",
        )
        .expect("matching red evidence remains readable");

        let error = validate_stored_result_integrity(
            Some("sha256:analysis"),
            Some("sha256:solution"),
            "sha256:analysis",
            "sha256:tampered",
            &evidence_digest,
            &ledger,
            false,
            Some(readiness_error.as_str()),
            &[],
            "sha256:mesh",
            "sha256:result",
        )
        .expect_err("solution identity tamper");
        assert!(error.message.contains("solution identity"), "{error:?}");

        let error = validate_stored_result_integrity(
            Some("sha256:analysis"),
            Some("sha256:solution"),
            "sha256:analysis",
            "sha256:solution",
            "sha256:tampered-evidence",
            &ledger,
            false,
            Some(readiness_error.as_str()),
            &[],
            "sha256:mesh",
            "sha256:result",
        )
        .expect_err("evidence digest tamper");
        assert!(error.message.contains("evidence digest"), "{error:?}");

        let error = validate_stored_result_integrity(
            Some("sha256:analysis"),
            Some("sha256:solution"),
            "sha256:analysis",
            "sha256:solution",
            &evidence_digest,
            &ledger,
            true,
            None,
            &[],
            "sha256:mesh",
            "sha256:result",
        )
        .expect_err("red ledger cannot be flipped green");
        assert!(error.message.contains("decision readiness"), "{error:?}");
    }

    #[test]
    fn binary_array_shape_and_path_are_strict() {
        let root = std::env::temp_dir().join(format!(
            "ecky-fem-artifact-shape-test-{}-{}",
            std::process::id(),
            FEM_ARTIFACT_NONCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).unwrap();
        assert!(write_array(
            &root,
            "nodesMm",
            "../escape.f64le",
            FemScalarType::Float64Le,
            &[1, 3],
            vec![0; 24],
        )
        .is_err());
        assert!(write_array(
            &root,
            "nodesMm",
            "nodes.f64le",
            FemScalarType::Float64Le,
            &[2, 3],
            vec![0; 24],
        )
        .unwrap_err()
        .message
        .contains("shape mismatch"));
        fs::remove_dir_all(root).unwrap();
    }
}
