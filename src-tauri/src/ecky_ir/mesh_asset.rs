use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::contracts::{AppError, AppResult, GeometryRepresentation};
use crate::ecky_core_ir::NodeId;

use super::shared::IrMesh;

/// Topology-only seam weld for evaluated CAD meshes. One nanometre in the
/// millimetre authoring unit removes floating-point transform drift without
/// changing printable dimensions.
const INDEXED_MESH_WELD_TOLERANCE_MM: f64 = 1.0e-6;

/// Provenance only. Hybrid consumers depend on [`MeshAsset`], never on the
/// engine or provider that produced it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MeshAssetSource {
    EckyMeshPhase {
        part_id: String,
        node_id: NodeId,
    },
    Imported,
    Generated {
        provider: String,
        model: Option<String>,
    },
}

impl MeshAssetSource {
    /// Stable provenance tag that contributes to deterministic multipart
    /// bundle identity without leaking engine internals to consumers.
    fn provenance_tag(&self) -> String {
        match self {
            MeshAssetSource::EckyMeshPhase { part_id, node_id } => {
                format!("ecky-mesh-phase:{part_id}:{}", node_id.raw())
            }
            MeshAssetSource::Imported => "imported".to_string(),
            MeshAssetSource::Generated { provider, model } => {
                format!("generated:{provider}:{}", model.as_deref().unwrap_or("-"))
            }
        }
    }
}

/// Engine-independent triangle-mesh handoff into the poly-BRep bridge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeshAsset {
    source: MeshAssetSource,
    stl_path: PathBuf,
}

/// Engine-independent indexed triangle mesh. This is the canonical mesh handoff;
/// STL remains an export/import format only.
#[derive(Debug, Clone, PartialEq)]
pub struct IndexedMeshAsset {
    source: MeshAssetSource,
    vertices: Vec<[f64; 3]>,
    triangles: Vec<[u32; 3]>,
    topology: IndexedMeshTopology,
    content_digest: String,
}

/// Validation facts preserved for Boolean-kernel admission and provenance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexedMeshTopology {
    pub boundary_edge_count: usize,
    pub non_manifold_edge_count: usize,
    pub winding_mismatch_count: usize,
    pub component_count: usize,
    pub closed: bool,
}

/// Imported STL preparation policy. No legacy path changes without explicit
/// bounds; no prep unless the caller passes targets.
#[derive(Debug, Clone, PartialEq)]
pub struct IndexedMeshPreparationPolicy {
    pub target_triangles: Option<usize>,
    pub max_error_mm: f64,
    pub preserve_boundaries: bool,
    pub protected_vertices: BTreeSet<u32>,
}

impl IndexedMeshPreparationPolicy {
    pub fn new(
        target_triangles: Option<usize>,
        max_error_mm: f64,
        preserve_boundaries: bool,
    ) -> AppResult<Self> {
        if let Some(target_triangles) = target_triangles {
            if target_triangles < 4 {
                return Err(AppError::validation(
                    "Imported mesh preparation targetTriangles must be at least four.",
                ));
            }
        }
        if !max_error_mm.is_finite() || max_error_mm <= 0.0 {
            return Err(AppError::validation(
                "Imported mesh preparation maxErrorMm must be finite and greater than zero.",
            ));
        }
        Ok(Self {
            target_triangles,
            max_error_mm,
            preserve_boundaries,
            protected_vertices: BTreeSet::new(),
        })
    }

    pub fn with_protected_vertices(
        mut self,
        protected_vertices: impl IntoIterator<Item = u32>,
    ) -> Self {
        self.protected_vertices = protected_vertices.into_iter().collect();
        self
    }
}

/// Typed import-preparation warning. One path: target not reached.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IndexedMeshPreparationWarning {
    TargetNotReached {
        requested_triangle_count: usize,
        achieved_triangle_count: usize,
        hard_error_block_count: usize,
        topology_block_count: usize,
    },
}

/// Deterministic provenance for an import-preparation pass.
#[derive(Debug, Clone, PartialEq)]
pub struct IndexedMeshPreparationProvenance {
    pub raw_source_digest: String,
    pub raw_source_byte_count: usize,
    pub raw_triangle_count: usize,
    pub duplicate_triangle_count_removed: usize,
    pub prepared_vertex_count: usize,
    pub prepared_triangle_count: usize,
    pub prepared_content_digest: String,
    pub max_error_mm: f64,
    pub rms_error_mm: f64,
    pub hard_error_block_count: usize,
    pub topology_block_count: usize,
    pub target_triangle_count: Option<usize>,
    pub preserve_boundaries: bool,
    pub protected_vertex_count: usize,
    pub algorithm_version: String,
}

/// Imported STL preparation result. Carries the canonical indexed mesh and the
/// typed provenance/warning payload. No derivative STL is written.
#[derive(Debug, Clone, PartialEq)]
pub struct IndexedMeshPreparationResult {
    asset: IndexedMeshAsset,
    warnings: Vec<IndexedMeshPreparationWarning>,
    provenance: IndexedMeshPreparationProvenance,
}

impl IndexedMeshPreparationResult {
    pub fn asset(&self) -> &IndexedMeshAsset {
        &self.asset
    }

    pub fn warnings(&self) -> &[IndexedMeshPreparationWarning] {
        &self.warnings
    }

    pub fn provenance(&self) -> &IndexedMeshPreparationProvenance {
        &self.provenance
    }
}

/// Serialized mirror of [`MeshAssetSource`] stored in the indexed-mesh
/// sidecar. The DTO is required (no `Option`, no default) so a cache that
/// predates provenance storage fails deserialization honestly and is
/// regenerated rather than silently reconstructed as a guessed source.
///
/// `NodeId` is a non-serializable opaque id, so it is stored as its raw
/// `u64` and rebuilt via `NodeId::new`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", tag = "kind")]
enum IndexedMeshCacheSource {
    EckyMeshPhase {
        part_id: String,
        node_id: u64,
    },
    Imported,
    Generated {
        provider: String,
        model: Option<String>,
    },
}

impl IndexedMeshCacheSource {
    fn from_source(source: &MeshAssetSource) -> Self {
        match source {
            MeshAssetSource::EckyMeshPhase { part_id, node_id } => Self::EckyMeshPhase {
                part_id: part_id.clone(),
                node_id: node_id.raw(),
            },
            MeshAssetSource::Imported => Self::Imported,
            MeshAssetSource::Generated { provider, model } => Self::Generated {
                provider: provider.clone(),
                model: model.clone(),
            },
        }
    }

    fn to_source(&self) -> MeshAssetSource {
        match self {
            Self::EckyMeshPhase { part_id, node_id } => MeshAssetSource::EckyMeshPhase {
                part_id: part_id.clone(),
                node_id: NodeId::new(*node_id),
            },
            Self::Imported => MeshAssetSource::Imported,
            Self::Generated { provider, model } => MeshAssetSource::Generated {
                provider: provider.clone(),
                model: model.clone(),
            },
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IndexedMeshCacheArtifact {
    schema_version: u32,
    source: IndexedMeshCacheSource,
    vertex_bits: Vec<[u64; 3]>,
    triangles: Vec<[u32; 3]>,
    content_digest: String,
}

impl IndexedMeshAsset {
    #[allow(dead_code)]
    pub(crate) fn from_ir_mesh(source: MeshAssetSource, mesh: &IrMesh) -> AppResult<Self> {
        let triangles = mesh.triangulate();
        let mut vertices = Vec::new();
        let mut vertex_indices = BTreeMap::<[i64; 3], Vec<u32>>::new();
        let mut indexed_triangles = Vec::with_capacity(triangles.polygons.len());
        let mut canonical_triangles = BTreeMap::<[u32; 3], ()>::new();

        for (face_index, polygon) in triangles.polygons.iter().enumerate() {
            if polygon.vertices.len() != 3 {
                return Err(AppError::validation(format!(
                    "Indexed mesh conversion produced non-triangle face {face_index}."
                )));
            }
            let mut triangle = [0; 3];
            for (corner, vertex) in polygon.vertices.iter().enumerate() {
                let position = [vertex.pos.x, vertex.pos.y, vertex.pos.z];
                triangle[corner] = index_vertex(&mut vertices, &mut vertex_indices, position)?;
            }
            if triangle[0] == triangle[1]
                || triangle[1] == triangle[2]
                || triangle[2] == triangle[0]
            {
                continue;
            }
            let a = vertices[triangle[0] as usize];
            let b = vertices[triangle[1] as usize];
            let c = vertices[triangle[2] as usize];
            if triangle_is_degenerate(a, b, c) {
                continue;
            }
            let mut canonical = triangle;
            canonical.sort_unstable();
            if canonical_triangles.insert(canonical, ()).is_some() {
                continue;
            }
            indexed_triangles.push(triangle);
        }

        let (vertices, indexed_triangles) = compact_indexed_geometry(vertices, indexed_triangles);
        Self::new(source, vertices, indexed_triangles)
    }

    /// Decode a standalone imported STL asset (binary or ASCII) into the
    /// canonical indexed mesh. STL is the import boundary only; the canonical
    /// handoff and cache representation remains the indexed sidecar, never STL.
    ///
    /// Authored vertex coordinates are preserved exactly: shared STL vertices
    /// merge only when their stored IEEE-754 representations are identical, so
    /// imported geometry is never silently welded. This differs from
    /// [`IndexedMeshAsset::from_ir_mesh`], which applies the named 1e-6 mm seam
    /// weld for evaluated CAD meshes.
    pub(crate) fn from_stl(source: MeshAssetSource, path: &Path) -> AppResult<Self> {
        let bytes = std::fs::read(path).map_err(|err| {
            AppError::validation(format!("Failed to read STL '{}': {err}", path.display()))
        })?;
        Self::from_stl_bytes(source, &bytes, path)
    }

    fn from_stl_bytes(source: MeshAssetSource, bytes: &[u8], path: &Path) -> AppResult<Self> {
        let raw_triangles = import_decode::decode_stl_triangles(&bytes, path)?;
        if raw_triangles
            .iter()
            .flatten()
            .flatten()
            .any(|coordinate| !coordinate.is_finite())
        {
            return Err(AppError::validation(format!(
                "STL '{}' contains a non-finite vertex coordinate.",
                path.display()
            )));
        }
        let (vertices, triangles) =
            import_decode::index_stl_triangles_preserving_authored_coordinates(&raw_triangles)?;
        if triangles.is_empty() {
            return Err(AppError::validation(format!(
                "STL '{}' contains no triangles.",
                path.display()
            )));
        }
        Self::new(source, vertices, triangles)
    }

    pub(crate) fn prepare_imported_file(
        source: MeshAssetSource,
        path: &Path,
        policy: &IndexedMeshPreparationPolicy,
    ) -> AppResult<IndexedMeshPreparationResult> {
        let bytes = std::fs::read(path).map_err(|err| {
            AppError::validation(format!("Failed to read STL '{}': {err}", path.display()))
        })?;
        let raw_source_digest = digest_bytes(&bytes);
        let raw_triangles = import_decode::decode_stl_triangles(&bytes, path)?;
        let raw_triangle_count = raw_triangles.len();
        if raw_triangles
            .iter()
            .flatten()
            .flatten()
            .any(|coordinate| !coordinate.is_finite())
        {
            return Err(AppError::validation(format!(
                "STL '{}' contains a non-finite vertex coordinate.",
                path.display()
            )));
        }
        let (vertices, triangles) =
            import_decode::index_stl_triangles_preserving_authored_coordinates(&raw_triangles)?;
        let (triangles, duplicate_triangle_count_removed) = deduplicate_triangle_indices(triangles);
        if triangles.is_empty() {
            return Err(AppError::validation(format!(
                "STL '{}' contains no triangles.",
                path.display()
            )));
        }
        let mesh = Self::new(source, vertices, triangles)?;
        let (
            asset,
            warnings,
            max_error_mm,
            rms_error_mm,
            hard_error_block_count,
            topology_block_count,
        ) = prepare_indexed_mesh(mesh, policy)?;
        Ok(IndexedMeshPreparationResult {
            provenance: IndexedMeshPreparationProvenance {
                raw_source_digest,
                raw_source_byte_count: bytes.len(),
                raw_triangle_count,
                duplicate_triangle_count_removed,
                prepared_vertex_count: asset.vertices().len(),
                prepared_triangle_count: asset.triangles().len(),
                prepared_content_digest: asset.content_digest().to_string(),
                max_error_mm,
                rms_error_mm,
                hard_error_block_count,
                topology_block_count,
                target_triangle_count: policy.target_triangles,
                preserve_boundaries: policy.preserve_boundaries,
                protected_vertex_count: policy.protected_vertices.len(),
                algorithm_version: preparation_algorithm_version(policy),
            },
            asset,
            warnings,
        })
    }

    /// Decode a standalone imported 3MF core asset into the canonical indexed
    /// mesh. 3MF is already an explicit indexed format, so authored vertex
    /// coordinates and authored triangle indices are retained as supplied; no
    /// seam weld is applied. Each `<mesh>` block is aggregated with a per-block
    /// vertex base so multi-object packages preserve their authored indexing.
    pub(crate) fn from_3mf(source: MeshAssetSource, path: &Path) -> AppResult<Self> {
        let model_xml = import_decode::read_3mf_model_xml(path)?;
        let (vertices, triangles) = import_decode::parse_3mf_core_mesh(&model_xml, path)?;
        if triangles.is_empty() {
            return Err(AppError::validation(format!(
                "3MF '{}' contains no triangles.",
                path.display()
            )));
        }
        Self::new(source, vertices, triangles)
    }

    /// Decode a standalone imported mesh asset into the canonical indexed mesh,
    /// dispatching by file extension. This is the single authored-decode entry
    /// point for standalone imports, so runtime sidecar generation and runner
    /// consumption stay consistent and no welded-vs-authored split can open
    /// between them.
    ///
    /// Both admitted formats preserve authored coordinates verbatim: STL
    /// vertices merge only by exact IEEE-754 bit-equality and 3MF retains its
    /// explicit indexing; neither applies the evaluated-CAD seam weld. An
    /// unsupported format is a hard rejection, never a silent fallback to a
    /// welded or faceted path.
    pub(crate) fn from_imported_file(source: MeshAssetSource, path: &Path) -> AppResult<Self> {
        match path.extension().and_then(|extension| extension.to_str()) {
            Some(extension) if extension.eq_ignore_ascii_case("stl") => {
                Self::from_stl(source, path)
            }
            Some(extension) if extension.eq_ignore_ascii_case("3mf") => {
                Self::from_3mf(source, path)
            }
            _ => Err(AppError::validation(format!(
                "Imported mesh '{}' uses an unsupported standalone format; only STL and 3MF are admitted to the indexed mesh path.",
                path.display()
            ))),
        }
    }

    pub fn new(
        source: MeshAssetSource,
        vertices: Vec<[f64; 3]>,
        triangles: Vec<[u32; 3]>,
    ) -> AppResult<Self> {
        validate_vertices(&vertices)?;
        let topology = validate_topology(&vertices, &triangles)?;
        let content_digest = indexed_mesh_digest(&vertices, &triangles);
        Ok(Self {
            source,
            vertices,
            triangles,
            topology,
            content_digest,
        })
    }

    pub fn source(&self) -> &MeshAssetSource {
        &self.source
    }

    pub fn vertices(&self) -> &[[f64; 3]] {
        &self.vertices
    }

    pub fn triangles(&self) -> &[[u32; 3]] {
        &self.triangles
    }

    pub fn topology(&self) -> &IndexedMeshTopology {
        &self.topology
    }

    pub fn content_digest(&self) -> &str {
        &self.content_digest
    }

    pub(crate) fn write_cache(&self, path: &Path) -> AppResult<()> {
        let artifact = IndexedMeshCacheArtifact {
            schema_version: 2,
            source: IndexedMeshCacheSource::from_source(&self.source),
            vertex_bits: self
                .vertices
                .iter()
                .map(|vertex| vertex.map(canonical_float_bits))
                .collect(),
            triangles: self.triangles.clone(),
            content_digest: self.content_digest.clone(),
        };
        let bytes = serde_json::to_vec(&artifact).map_err(|err| {
            AppError::persistence(format!("Failed to encode indexed mesh cache: {err}"))
        })?;
        std::fs::write(path, bytes).map_err(|err| {
            AppError::persistence(format!(
                "Failed to write indexed mesh cache '{}': {err}",
                path.display()
            ))
        })
    }

    pub(crate) fn read_cache(path: &Path) -> AppResult<Self> {
        let bytes = std::fs::read(path).map_err(|err| {
            AppError::persistence(format!(
                "Failed to read indexed mesh cache '{}': {err}",
                path.display()
            ))
        })?;
        let artifact: IndexedMeshCacheArtifact = serde_json::from_slice(&bytes).map_err(|err| {
            AppError::validation(format!(
                "Indexed mesh cache '{}' is invalid: {err}",
                path.display()
            ))
        })?;
        if artifact.schema_version != 2 {
            return Err(AppError::validation(format!(
                "Indexed mesh cache '{}' uses unsupported schemaVersion {}.",
                path.display(),
                artifact.schema_version
            )));
        }
        let source = artifact.source.to_source();
        let vertices = artifact
            .vertex_bits
            .into_iter()
            .map(|vertex| vertex.map(f64::from_bits))
            .collect();
        let asset = Self::new(source, vertices, artifact.triangles)?;
        if asset.content_digest != artifact.content_digest {
            return Err(AppError::validation(format!(
                "Indexed mesh cache '{}' content digest mismatch.",
                path.display()
            )));
        }
        Ok(asset)
    }

    /// Explicit Boolean-kernel manifold-status gate. Construction preserves
    /// topology facts for provenance; mesh Boolean callers must invoke this gate.
    /// A Boolean adapter must additionally apply its self-intersection policy.
    pub fn validate_for_boolean(&self) -> AppResult<()> {
        let topology = &self.topology;
        if topology.boundary_edge_count > 0
            || topology.non_manifold_edge_count > 0
            || topology.winding_mismatch_count > 0
            || topology.component_count == 0
        {
            return Err(AppError::validation(format!(
                "Indexed mesh is not Boolean-ready: boundary edges: {}; non-manifold edges: {}; winding mismatches: {}; connected components: {}.",
                topology.boundary_edge_count,
                topology.non_manifold_edge_count,
                topology.winding_mismatch_count,
                topology.component_count,
            )));
        }
        Ok(())
    }
}

/// One authored component inside a mesh-native multipart bundle. The canonical
/// geometry is the [`IndexedMeshAsset`]; identity is deterministic (authored
/// index plus content digest) and provenance is the original
/// [`MeshAssetSource`]. No STEP is ever attached.
#[derive(Debug, Clone, PartialEq)]
pub struct MultipartMeshComponent {
    component_id: String,
    label: String,
    asset: IndexedMeshAsset,
}

impl MultipartMeshComponent {
    pub fn new(index: usize, label: impl Into<String>, asset: IndexedMeshAsset) -> Self {
        let digest = asset.content_digest();
        let hex = digest.strip_prefix("sha256:").unwrap_or(digest);
        let prefix = &hex[..hex.len().min(12)];
        Self {
            component_id: format!("component-{index}-{prefix}"),
            label: label.into(),
            asset,
        }
    }

    pub fn component_id(&self) -> &str {
        &self.component_id
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn asset(&self) -> &IndexedMeshAsset {
        &self.asset
    }

    pub fn content_digest(&self) -> &str {
        self.asset.content_digest()
    }

    pub fn source(&self) -> &MeshAssetSource {
        self.asset.source()
    }
}

/// Mesh-native multipart bundle: one canonical indexed-mesh artifact per
/// authored component, each with deterministic identity and provenance, under
/// one deterministic, order-sensitive bundle identity. This is the
/// representation-preserving export contract for mesh islands targeting STL or
/// 3MF; the indexed manifold mesh is retained and no faceted-BRep conversion
/// or fabricated STEP occurs.
#[derive(Debug, Clone, PartialEq)]
pub struct MultipartMeshNativeBundle {
    components: Vec<MultipartMeshComponent>,
    bundle_digest: String,
}

impl MultipartMeshNativeBundle {
    pub fn new(components: Vec<MultipartMeshComponent>) -> AppResult<Self> {
        if components.is_empty() {
            return Err(AppError::validation(
                "Multipart mesh-native bundle requires at least one component.",
            ));
        }
        let bundle_digest = multipart_mesh_bundle_digest(&components);
        Ok(Self {
            components,
            bundle_digest,
        })
    }

    pub fn components(&self) -> &[MultipartMeshComponent] {
        &self.components
    }

    pub fn bundle_digest(&self) -> &str {
        &self.bundle_digest
    }

    /// Representation marker for this export contract. Always mesh-native: the
    /// canonical indexed mesh is preserved and no faceted-BRep conversion runs.
    pub fn representation(&self) -> GeometryRepresentation {
        GeometryRepresentation::MeshNative
    }

    /// No-fabricated-STEP proof hook. A mesh-native bundle never emits STEP; if
    /// a STEP contract is required the caller must use the OCCT path instead.
    pub fn has_step_artifact(&self) -> bool {
        false
    }
}

fn multipart_mesh_bundle_digest(components: &[MultipartMeshComponent]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"ecky-multipart-mesh-native-v1\0");
    hasher.update((components.len() as u64).to_le_bytes());
    for component in components {
        hasher.update(component.component_id().as_bytes());
        hasher.update(b"\0");
        hasher.update(component.content_digest().as_bytes());
        hasher.update(b"\0");
        hasher.update(component.source().provenance_tag().as_bytes());
        hasher.update(b"\0");
    }
    format!("sha256:{:x}", hasher.finalize())
}

fn compact_indexed_geometry(
    vertices: Vec<[f64; 3]>,
    mut triangles: Vec<[u32; 3]>,
) -> (Vec<[f64; 3]>, Vec<[u32; 3]>) {
    let mut remap = vec![None; vertices.len()];
    let mut compact_vertices = Vec::new();
    for triangle in &mut triangles {
        for index in triangle {
            let old_index = *index as usize;
            *index = match remap[old_index] {
                Some(index) => index,
                None => {
                    let index = compact_vertices.len() as u32;
                    compact_vertices.push(vertices[old_index]);
                    remap[old_index] = Some(index);
                    index
                }
            };
        }
    }
    (compact_vertices, triangles)
}

fn deduplicate_triangle_indices(triangles: Vec<[u32; 3]>) -> (Vec<[u32; 3]>, usize) {
    let original_len = triangles.len();
    let mut retained = Vec::<Option<[u32; 3]>>::new();
    let mut seen = BTreeMap::<[u32; 3], (usize, bool)>::new();
    for triangle in triangles {
        let mut canonical = triangle;
        canonical.sort_unstable();
        let orientation = triangle_orientation_matches_sorted(triangle, canonical);
        if let Some((retained_index, retained_orientation)) = seen.get(&canonical).copied() {
            if orientation != retained_orientation {
                retained[retained_index] = None;
                seen.remove(&canonical);
            }
            continue;
        }
        let retained_index = retained.len();
        retained.push(Some(triangle));
        seen.insert(canonical, (retained_index, orientation));
    }
    let retained = retained.into_iter().flatten().collect::<Vec<_>>();
    let removed = original_len - retained.len();
    (retained, removed)
}

fn triangle_orientation_matches_sorted(triangle: [u32; 3], sorted: [u32; 3]) -> bool {
    matches!(
        triangle,
        [a, b, c]
            if [a, b, c] == sorted
                || [b, c, a] == sorted
                || [c, a, b] == sorted
    )
}

fn prepare_indexed_mesh(
    mesh: IndexedMeshAsset,
    policy: &IndexedMeshPreparationPolicy,
) -> AppResult<(
    IndexedMeshAsset,
    Vec<IndexedMeshPreparationWarning>,
    f64,
    f64,
    usize,
    usize,
)> {
    let original_topology = mesh.topology().clone();
    let original_vertices = mesh.vertices().to_vec();
    let original_triangles = mesh.triangles().to_vec();
    let target_triangles = policy.target_triangles.unwrap_or(mesh.triangles().len());
    if target_triangles >= original_triangles.len() {
        return Ok((mesh, Vec::new(), 0.0, 0.0, 0, 0));
    }

    let packed_positions = original_vertices
        .iter()
        .flat_map(|vertex| vertex.iter().map(|value| *value as f32))
        .flat_map(f32::to_ne_bytes)
        .collect::<Vec<_>>();
    let vertex_adapter =
        meshopt::VertexDataAdapter::new(&packed_positions, std::mem::size_of::<f32>() * 3, 0)
            .map_err(|err| {
                AppError::validation(format!(
                    "Imported mesh preparation could not adapt indexed positions: {err}"
                ))
            })?;
    for &index in &policy.protected_vertices {
        if index as usize >= original_vertices.len() {
            return Err(AppError::validation(format!(
                "Imported mesh preparation protected vertex {index} is out of bounds for {} vertices.",
                original_vertices.len()
            )));
        }
    }
    let mut vertex_locks = vec![false; original_vertices.len()];
    for &index in &policy.protected_vertices {
        vertex_locks[index as usize] = true;
    }
    let original_indices = original_triangles
        .iter()
        .flat_map(|triangle| triangle.iter().copied())
        .collect::<Vec<_>>();
    let mut options = meshopt::SimplifyOptions::ErrorAbsolute;
    if policy.preserve_boundaries {
        options |= meshopt::SimplifyOptions::LockBorder;
    }
    let mut certified_error_mm = 0.0_f32;
    let prepared_indices = if policy.protected_vertices.is_empty() {
        meshopt::simplify(
            &original_indices,
            &vertex_adapter,
            target_triangles.saturating_mul(3),
            policy.max_error_mm as f32,
            options,
            Some(&mut certified_error_mm),
        )
    } else {
        meshopt::simplify_with_locks(
            &original_indices,
            &vertex_adapter,
            &vertex_locks,
            target_triangles.saturating_mul(3),
            policy.max_error_mm as f32,
            options,
            Some(&mut certified_error_mm),
        )
    };
    let prepared_triangles = prepared_indices
        .chunks_exact(3)
        .map(|triangle| [triangle[0], triangle[1], triangle[2]])
        .collect::<Vec<_>>();
    let (prepared_triangles, _) = deduplicate_triangle_indices(prepared_triangles);
    let (prepared_vertices, prepared_triangles) =
        compact_indexed_geometry(original_vertices.clone(), prepared_triangles);
    let candidate =
        IndexedMeshAsset::new(mesh.source().clone(), prepared_vertices, prepared_triangles)?;
    let topology_matches = candidate.topology().boundary_edge_count
        == original_topology.boundary_edge_count
        && candidate.topology().non_manifold_edge_count
            == original_topology.non_manifold_edge_count
        && candidate.topology().winding_mismatch_count == original_topology.winding_mismatch_count
        && candidate.topology().component_count == original_topology.component_count
        && candidate.topology().closed == original_topology.closed;
    let candidate_max_error_mm = certified_error_mm as f64;
    let candidate_rms_error_mm = mesh_rms_deviation_sample(
        &original_vertices,
        &original_triangles,
        candidate.vertices(),
        candidate.triangles(),
    )?;
    let error_matches =
        certified_error_mm.is_finite() && candidate_max_error_mm <= policy.max_error_mm;
    let (current, max_error_mm, rms_error_mm, hard_error_block_count, topology_block_count) =
        if topology_matches && error_matches {
            (
                candidate,
                candidate_max_error_mm,
                candidate_rms_error_mm,
                0,
                0,
            )
        } else {
            (
                mesh,
                0.0,
                0.0,
                usize::from(!error_matches),
                usize::from(!topology_matches),
            )
        };

    let mut warnings = Vec::new();
    if current.triangles().len() > target_triangles {
        warnings.push(IndexedMeshPreparationWarning::TargetNotReached {
            requested_triangle_count: target_triangles,
            achieved_triangle_count: current.triangles().len(),
            hard_error_block_count,
            topology_block_count,
        });
    }
    Ok((
        current,
        warnings,
        max_error_mm,
        rms_error_mm,
        hard_error_block_count,
        topology_block_count,
    ))
}

fn preparation_algorithm_version(policy: &IndexedMeshPreparationPolicy) -> String {
    format!(
        "meshopt-0.6.2:error-absolute:{}",
        if policy.preserve_boundaries {
            "lock-border"
        } else {
            "free-border"
        }
    )
}

fn mesh_rms_deviation_sample(
    reference_vertices: &[[f64; 3]],
    reference_triangles: &[[u32; 3]],
    candidate_vertices: &[[f64; 3]],
    candidate_triangles: &[[u32; 3]],
) -> AppResult<f64> {
    use csgrs::float_types::parry3d::math::Point;
    use csgrs::float_types::parry3d::shape::{TriMesh, TriMeshFlags};

    let reference = TriMesh::with_flags(
        reference_vertices
            .iter()
            .map(|vertex| Point::new(vertex[0], vertex[1], vertex[2]))
            .collect(),
        reference_triangles.to_vec(),
        TriMeshFlags::ORIENTED | TriMeshFlags::CONNECTED_COMPONENTS,
    )
    .map_err(|err| {
        AppError::validation(format!(
            "Imported mesh preparation could not project reference mesh: {err:?}"
        ))
    })?;
    let candidate = TriMesh::with_flags(
        candidate_vertices
            .iter()
            .map(|vertex| Point::new(vertex[0], vertex[1], vertex[2]))
            .collect(),
        candidate_triangles.to_vec(),
        TriMeshFlags::ORIENTED | TriMeshFlags::CONNECTED_COMPONENTS,
    )
    .map_err(|err| {
        AppError::validation(format!(
            "Imported mesh preparation could not project candidate mesh: {err:?}"
        ))
    })?;
    const SAMPLE_LIMIT_PER_DIRECTION: usize = 4096;
    let reference_step = reference_vertices
        .len()
        .div_ceil(SAMPLE_LIMIT_PER_DIRECTION)
        .max(1);
    let candidate_step = candidate_vertices
        .len()
        .div_ceil(SAMPLE_LIMIT_PER_DIRECTION)
        .max(1);
    let mut residuals = Vec::with_capacity(SAMPLE_LIMIT_PER_DIRECTION * 2);
    residuals.extend(
        reference_vertices
            .iter()
            .step_by(reference_step)
            .map(|point| point_mesh_distance(&candidate, *point)),
    );
    residuals.extend(
        candidate_vertices
            .iter()
            .step_by(candidate_step)
            .map(|point| point_mesh_distance(&reference, *point)),
    );
    if residuals.is_empty() {
        return Ok(0.0);
    }
    let rms_error_mm =
        (residuals.iter().map(|value| value * value).sum::<f64>() / residuals.len() as f64).sqrt();
    Ok(rms_error_mm)
}

fn point_mesh_distance(mesh: &csgrs::float_types::parry3d::shape::TriMesh, point: [f64; 3]) -> f64 {
    use csgrs::float_types::parry3d::math::Point;
    use csgrs::float_types::parry3d::query::PointQuery;
    let point = Point::new(point[0], point[1], point[2]);
    let projection = mesh.project_local_point(&point, false);
    (projection.point - point).norm()
}

fn digest_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"ecky-imported-stl-bytes-v1\0");
    hasher.update((bytes.len() as u64).to_le_bytes());
    hasher.update(bytes);
    format!("sha256:{:x}", hasher.finalize())
}

// --- Standalone STL/3MF indexed decoders (task 6) ----------------------------
// Canonical authored decoders for imported assets. `from_imported_file`
// dispatches to these by extension from the runner admission path so authored
// coordinates are effective at runtime, not an additive dead API.
mod import_decode {
    use std::collections::BTreeMap;
    use std::io::Read;
    use std::path::Path;

    use crate::contracts::{AppError, AppResult};

    use super::canonical_float_bits;

    const STL_FACET_SIZE: usize = 50;
    const STL_BINARY_HEADER_SIZE: usize = 84;

    pub(super) fn decode_stl_triangles(bytes: &[u8], path: &Path) -> AppResult<Vec<[[f64; 3]; 3]>> {
        if bytes.len() >= STL_BINARY_HEADER_SIZE {
            let count = u32::from_le_bytes([bytes[80], bytes[81], bytes[82], bytes[83]]) as usize;
            if STL_BINARY_HEADER_SIZE.checked_add(count.saturating_mul(STL_FACET_SIZE))
                == Some(bytes.len())
            {
                return decode_binary_stl_triangles(bytes, count, path);
            }
        }
        decode_ascii_stl_triangles(bytes, path)
    }

    fn decode_binary_stl_triangles(
        bytes: &[u8],
        count: usize,
        path: &Path,
    ) -> AppResult<Vec<[[f64; 3]; 3]>> {
        let mut triangles = Vec::with_capacity(count);
        for index in 0..count {
            let facet_base = STL_BINARY_HEADER_SIZE + index * STL_FACET_SIZE;
            // Skip the 12-byte facet normal at facet_base..facet_base+12.
            let mut triangle = [[0.0_f64; 3]; 3];
            for vertex in 0..3 {
                for coordinate in 0..3 {
                    let offset = facet_base + 12 + (vertex * 3 + coordinate) * 4;
                    let value = f32::from_le_bytes(
                        bytes[offset..offset + 4]
                            .try_into()
                            .expect("checked binary STL facet layout"),
                    ) as f64;
                    triangle[vertex][coordinate] = value;
                }
            }
            triangles.push(triangle);
        }
        let _ = path;
        Ok(triangles)
    }

    fn decode_ascii_stl_triangles(bytes: &[u8], path: &Path) -> AppResult<Vec<[[f64; 3]; 3]>> {
        let text = std::str::from_utf8(bytes).map_err(|err| {
            AppError::validation(format!("STL '{}' is invalid: {err}", path.display()))
        })?;
        let mut flat_vertices = Vec::new();
        for line in text.lines() {
            let mut parts = line.split_whitespace();
            if !parts
                .next()
                .is_some_and(|token| token.eq_ignore_ascii_case("vertex"))
            {
                continue;
            }
            let parsed = [
                parts.next().and_then(|value| value.parse::<f64>().ok()),
                parts.next().and_then(|value| value.parse::<f64>().ok()),
                parts.next().and_then(|value| value.parse::<f64>().ok()),
            ];
            let [Some(x), Some(y), Some(z)] = parsed else {
                return Err(AppError::validation(format!(
                    "STL '{}' contains an invalid vertex.",
                    path.display()
                )));
            };
            flat_vertices.push([x, y, z]);
        }
        if flat_vertices.len() % 3 != 0 {
            return Err(AppError::validation(format!(
                "STL '{}' contains an incomplete facet.",
                path.display()
            )));
        }
        Ok(flat_vertices
            .chunks_exact(3)
            .map(|chunk| [chunk[0], chunk[1], chunk[2]])
            .collect())
    }

    /// Index unindexed STL triangle soup by exact IEEE-754 bit-equality so authored
    /// coordinates are preserved verbatim. Vertices merge only when their stored
    /// representations are identical; no tolerance weld is applied.
    pub(super) fn index_stl_triangles_preserving_authored_coordinates(
        raw_triangles: &[[[f64; 3]; 3]],
    ) -> AppResult<(Vec<[f64; 3]>, Vec<[u32; 3]>)> {
        let mut vertices: Vec<[f64; 3]> = Vec::new();
        let mut index_by_bits: BTreeMap<[u64; 3], u32> = BTreeMap::new();
        let mut triangles: Vec<[u32; 3]> = Vec::with_capacity(raw_triangles.len());
        for raw_triangle in raw_triangles {
            let mut triangle = [0_u32; 3];
            for (slot, position) in raw_triangle.iter().enumerate() {
                let key = position.map(canonical_float_bits);
                let entry = match index_by_bits.get(&key) {
                    Some(existing) => *existing,
                    None => {
                        let next = u32::try_from(vertices.len()).map_err(|_| {
                            AppError::validation("STL import exceeds the u32 vertex-index limit.")
                        })?;
                        vertices.push(*position);
                        index_by_bits.insert(key, next);
                        next
                    }
                };
                triangle[slot] = entry;
            }
            triangles.push(triangle);
        }
        Ok((vertices, triangles))
    }

    pub(super) fn read_3mf_model_xml(path: &Path) -> AppResult<String> {
        let file = std::fs::File::open(path).map_err(|err| {
            AppError::validation(format!("Failed to open 3MF '{}': {err}", path.display()))
        })?;
        let mut archive = zip::ZipArchive::new(file).map_err(|err| {
            AppError::validation(format!(
                "3MF '{}' is not a valid package: {err}",
                path.display()
            ))
        })?;
        for index in 0..archive.len() {
            let mut entry = archive.by_index(index).map_err(|err| {
                AppError::validation(format!(
                    "3MF '{}' could not read package entry: {err}",
                    path.display()
                ))
            })?;
            if entry.name() == "3D/3dmodel.model" {
                let mut model = String::new();
                entry.read_to_string(&mut model).map_err(|err| {
                    AppError::validation(format!(
                        "3MF '{}' model is invalid: {err}",
                        path.display()
                    ))
                })?;
                return Ok(model);
            }
        }
        Err(AppError::validation(format!(
            "3MF '{}' is missing the 3D/3dmodel.model part.",
            path.display()
        )))
    }

    /// Parse the 3MF core `<mesh>` blocks into one aggregated indexed mesh. Authored
    /// vertex coordinates and triangle indices are retained; each `<mesh>` block
    /// offsets its triangle indices by the running vertex base so multi-object
    /// packages preserve their authored indexing.
    pub(super) fn parse_3mf_core_mesh(
        model_xml: &str,
        path: &Path,
    ) -> AppResult<(Vec<[f64; 3]>, Vec<[u32; 3]>)> {
        let mesh_block_re = regex::Regex::new(r"(?s)<mesh\b.*?</mesh>").expect("static mesh regex");
        let vertex_tag_re = regex::Regex::new(r"<vertex\b[^>]*>").expect("static vertex regex");
        let triangle_tag_re =
            regex::Regex::new(r"<triangle\b[^>]*>").expect("static triangle regex");
        let attribute_re =
            regex::Regex::new(r#"(\w+)\s*=\s*["']([^"']*)["']"#).expect("static attribute regex");

        let mut vertices: Vec<[f64; 3]> = Vec::new();
        let mut triangles: Vec<[u32; 3]> = Vec::new();

        for mesh_block in mesh_block_re.find_iter(model_xml) {
            let block = mesh_block.as_str();
            let vertex_base = vertices.len();
            for tag in vertex_tag_re.find_iter(block) {
                let attributes = capture_xml_attributes(tag.as_str(), &attribute_re);
                let x = require_3mf_scalar(&attributes, "x", path)?;
                let y = require_3mf_scalar(&attributes, "y", path)?;
                let z = require_3mf_scalar(&attributes, "z", path)?;
                vertices.push([x, y, z]);
            }
            for tag in triangle_tag_re.find_iter(block) {
                let attributes = capture_xml_attributes(tag.as_str(), &attribute_re);
                let v1 = require_3mf_index(&attributes, "v1", vertex_base, path)?;
                let v2 = require_3mf_index(&attributes, "v2", vertex_base, path)?;
                let v3 = require_3mf_index(&attributes, "v3", vertex_base, path)?;
                triangles.push([v1, v2, v3]);
            }
        }

        Ok((vertices, triangles))
    }

    fn capture_xml_attributes(tag: &str, attribute_re: &regex::Regex) -> BTreeMap<String, String> {
        let mut attributes = BTreeMap::new();
        for capture in attribute_re.captures_iter(tag) {
            if let (Some(name), Some(value)) = (capture.get(1), capture.get(2)) {
                attributes.insert(name.as_str().to_string(), value.as_str().to_string());
            }
        }
        attributes
    }

    fn require_3mf_scalar(
        attributes: &BTreeMap<String, String>,
        name: &str,
        path: &Path,
    ) -> AppResult<f64> {
        let raw = attributes.get(name).ok_or_else(|| {
            AppError::validation(format!(
                "3MF '{}' is missing a required mesh attribute '{name}'.",
                path.display()
            ))
        })?;
        let value = raw.parse::<f64>().map_err(|_| {
            AppError::validation(format!(
                "3MF '{}' contains an invalid mesh attribute '{name}'.",
                path.display()
            ))
        })?;
        if !value.is_finite() {
            return Err(AppError::validation(format!(
                "3MF '{}' contains a non-finite mesh attribute '{name}'.",
                path.display()
            )));
        }
        Ok(value)
    }

    fn require_3mf_index(
        attributes: &BTreeMap<String, String>,
        name: &str,
        vertex_base: usize,
        path: &Path,
    ) -> AppResult<u32> {
        let raw = attributes.get(name).ok_or_else(|| {
            AppError::validation(format!(
                "3MF '{}' is missing a required mesh attribute '{name}'.",
                path.display()
            ))
        })?;
        let local = raw.parse::<u32>().map_err(|_| {
            AppError::validation(format!(
                "3MF '{}' contains an invalid mesh attribute '{name}'.",
                path.display()
            ))
        })?;
        vertex_base
            .checked_add(local as usize)
            .and_then(|absolute| u32::try_from(absolute).ok())
            .ok_or_else(|| {
                AppError::validation(format!(
                    "3MF '{}' triangle vertex index is out of range.",
                    path.display()
                ))
            })
    }
} // end import_decode

#[allow(dead_code)]
fn index_vertex(
    vertices: &mut Vec<[f64; 3]>,
    vertex_indices: &mut BTreeMap<[i64; 3], Vec<u32>>,
    position: [f64; 3],
) -> AppResult<u32> {
    if !position.iter().all(|value| value.is_finite()) {
        return Err(AppError::validation(
            "Indexed mesh contains a non-finite vertex coordinate.",
        ));
    }
    let position = position.map(canonicalize_zero);
    let cell =
        position.map(|coordinate| (coordinate / INDEXED_MESH_WELD_TOLERANCE_MM).floor() as i64);
    for dx in -1..=1 {
        for dy in -1..=1 {
            for dz in -1..=1 {
                let neighbour = [cell[0] + dx, cell[1] + dy, cell[2] + dz];
                for index in vertex_indices.get(&neighbour).into_iter().flatten() {
                    let existing = vertices[*index as usize];
                    let distance_squared = existing
                        .iter()
                        .zip(position)
                        .map(|(left, right)| {
                            let delta = left - right;
                            delta * delta
                        })
                        .sum::<f64>();
                    if distance_squared
                        <= INDEXED_MESH_WELD_TOLERANCE_MM * INDEXED_MESH_WELD_TOLERANCE_MM
                    {
                        return Ok(*index);
                    }
                }
            }
        }
    }
    let index = u32::try_from(vertices.len())
        .map_err(|_| AppError::validation("Indexed mesh exceeds the u32 vertex-index limit."))?;
    vertices.push(position);
    vertex_indices.entry(cell).or_default().push(index);
    Ok(index)
}

fn validate_vertices(vertices: &[[f64; 3]]) -> AppResult<()> {
    if vertices
        .iter()
        .flatten()
        .any(|coordinate| !coordinate.is_finite())
    {
        return Err(AppError::validation(
            "Indexed mesh contains a non-finite vertex coordinate.",
        ));
    }
    Ok(())
}

fn validate_topology(
    vertices: &[[f64; 3]],
    triangles: &[[u32; 3]],
) -> AppResult<IndexedMeshTopology> {
    let mut edges: BTreeMap<(u32, u32), Vec<(usize, bool)>> = BTreeMap::new();
    let mut canonical_triangles = BTreeMap::<[u32; 3], usize>::new();
    for (face_index, triangle) in triangles.iter().enumerate() {
        for index in triangle {
            if (*index as usize) >= vertices.len() {
                return Err(AppError::validation(format!(
                    "Indexed mesh triangle {face_index} has out-of-bounds vertex index {index}."
                )));
            }
        }
        if triangle[0] == triangle[1] || triangle[1] == triangle[2] || triangle[2] == triangle[0] {
            return Err(AppError::validation(format!(
                "Indexed mesh triangle {face_index} is degenerate (repeated vertex index)."
            )));
        }
        let a = vertices[triangle[0] as usize];
        let b = vertices[triangle[1] as usize];
        let c = vertices[triangle[2] as usize];
        if triangle_is_degenerate(a, b, c) {
            return Err(AppError::validation(format!(
                "Indexed mesh triangle {face_index} is degenerate (zero area)."
            )));
        }
        let mut canonical = *triangle;
        canonical.sort_unstable();
        if let Some(original_face) = canonical_triangles.insert(canonical, face_index) {
            return Err(AppError::validation(format!(
                "Indexed mesh triangle {face_index} duplicates triangle {original_face}."
            )));
        }
        for (from, to) in [
            (triangle[0], triangle[1]),
            (triangle[1], triangle[2]),
            (triangle[2], triangle[0]),
        ] {
            let key = (from.min(to), from.max(to));
            edges
                .entry(key)
                .or_default()
                .push((face_index, from == key.0));
        }
    }

    let mut boundary_edge_count = 0;
    let mut non_manifold_edge_count = 0;
    let mut winding_mismatch_count = 0;
    let mut neighbours = vec![Vec::new(); triangles.len()];
    for adjacent_faces in edges.values() {
        match adjacent_faces.as_slice() {
            [_] => boundary_edge_count += 1,
            [(_, first_forward), (_, second_forward)] => {
                if first_forward == second_forward {
                    winding_mismatch_count += 1;
                }
            }
            _ => non_manifold_edge_count += 1,
        }
        for left in 0..adjacent_faces.len() {
            for right in (left + 1)..adjacent_faces.len() {
                let first_face = adjacent_faces[left].0;
                let second_face = adjacent_faces[right].0;
                neighbours[first_face].push(second_face);
                neighbours[second_face].push(first_face);
            }
        }
    }
    let component_count = connected_component_count(&neighbours);
    Ok(IndexedMeshTopology {
        boundary_edge_count,
        non_manifold_edge_count,
        winding_mismatch_count,
        component_count,
        closed: !triangles.is_empty()
            && boundary_edge_count == 0
            && non_manifold_edge_count == 0
            && winding_mismatch_count == 0,
    })
}

fn connected_component_count(neighbours: &[Vec<usize>]) -> usize {
    let mut seen = vec![false; neighbours.len()];
    let mut count = 0;
    for start in 0..neighbours.len() {
        if seen[start] {
            continue;
        }
        count += 1;
        seen[start] = true;
        let mut queue = VecDeque::from([start]);
        while let Some(face) = queue.pop_front() {
            for &neighbour in &neighbours[face] {
                if !seen[neighbour] {
                    seen[neighbour] = true;
                    queue.push_back(neighbour);
                }
            }
        }
    }
    count
}

fn triangle_is_degenerate(a: [f64; 3], b: [f64; 3], c: [f64; 3]) -> bool {
    let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let ac = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
    let scale = ab
        .into_iter()
        .chain(ac)
        .map(f64::abs)
        .fold(0.0_f64, f64::max);
    if scale == 0.0 {
        return true;
    }
    let ab = ab.map(|value| value / scale);
    let ac = ac.map(|value| value / scale);
    let cross = [
        ab[1] * ac[2] - ab[2] * ac[1],
        ab[2] * ac[0] - ab[0] * ac[2],
        ab[0] * ac[1] - ab[1] * ac[0],
    ];
    let normalized_double_area_squared =
        cross[0].mul_add(cross[0], cross[1].mul_add(cross[1], cross[2] * cross[2]));
    normalized_double_area_squared <= f64::EPSILON
}

fn indexed_mesh_digest(vertices: &[[f64; 3]], triangles: &[[u32; 3]]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"ecky-indexed-mesh-v1\0");
    hasher.update((vertices.len() as u64).to_le_bytes());
    for vertex in vertices {
        for coordinate in vertex {
            hasher.update(canonical_float_bits(*coordinate).to_le_bytes());
        }
    }
    hasher.update((triangles.len() as u64).to_le_bytes());
    for triangle in triangles {
        for index in triangle {
            hasher.update(index.to_le_bytes());
        }
    }
    format!("sha256:{:x}", hasher.finalize())
}

fn canonicalize_zero(value: f64) -> f64 {
    if value == 0.0 {
        0.0
    } else {
        value
    }
}

fn canonical_float_bits(value: f64) -> u64 {
    canonicalize_zero(value).to_bits()
}

impl MeshAsset {
    pub fn ecky_mesh_phase(
        part_id: impl Into<String>,
        node_id: NodeId,
        stl_path: impl AsRef<Path>,
    ) -> AppResult<Self> {
        Self::validated(
            MeshAssetSource::EckyMeshPhase {
                part_id: part_id.into(),
                node_id,
            },
            stl_path,
        )
    }

    pub fn imported(stl_path: impl AsRef<Path>) -> AppResult<Self> {
        Self::validated(MeshAssetSource::Imported, stl_path)
    }

    pub fn generated(
        provider: impl Into<String>,
        model: Option<impl Into<String>>,
        stl_path: impl AsRef<Path>,
    ) -> AppResult<Self> {
        Self::validated(
            MeshAssetSource::Generated {
                provider: provider.into(),
                model: model.map(Into::into),
            },
            stl_path,
        )
    }

    fn validated(source: MeshAssetSource, stl_path: impl AsRef<Path>) -> AppResult<Self> {
        let stl_path = stl_path.as_ref();
        let metadata = std::fs::metadata(stl_path).map_err(|err| {
            AppError::validation(format!(
                "Mesh asset '{}' is not readable: {err}",
                stl_path.display()
            ))
        })?;
        if !metadata.is_file() || metadata.len() == 0 {
            return Err(AppError::validation(format!(
                "Mesh asset '{}' must be a non-empty STL file.",
                stl_path.display()
            )));
        }
        if !stl_path
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|value| value.eq_ignore_ascii_case("stl"))
        {
            return Err(AppError::validation(format!(
                "Mesh asset '{}' must use STL format before entering the OCCT bridge.",
                stl_path.display()
            )));
        }
        Ok(Self {
            source,
            stl_path: stl_path.to_path_buf(),
        })
    }

    pub fn source(&self) -> &MeshAssetSource {
        &self.source
    }

    pub fn stl_path(&self) -> &Path {
        &self.stl_path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecky_ir::shared::IrMesh;
    use csgrs::float_types::parry3d::na::{Point3, Vector3};
    use csgrs::mesh::polygon::Polygon as IrPolygon;
    use csgrs::mesh::vertex::Vertex as IrVertex;

    #[test]
    fn generated_mesh_uses_same_validated_asset_contract_as_internal_mesh() {
        let root =
            std::env::temp_dir().join(format!("ecky-generated-mesh-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).expect("temp dir");
        let path = root.join("meshy-output.stl");
        std::fs::write(&path, b"solid generated\nendsolid generated\n").expect("fixture");

        let asset =
            MeshAsset::generated("meshy", Some("model-42"), &path).expect("generated mesh asset");

        assert_eq!(asset.stl_path(), path.as_path());
        assert!(matches!(
            asset.source(),
            MeshAssetSource::Generated { provider, model }
                if provider == "meshy" && model.as_deref() == Some("model-42")
        ));

        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn indexed_mesh_asset_deduplicates_ir_triangles_and_records_closed_manifold_provenance() {
        let mesh = IrMesh::cuboid(2.0, 2.0, 2.0, None);

        let asset = IndexedMeshAsset::from_ir_mesh(MeshAssetSource::Imported, &mesh)
            .expect("indexed asset");

        assert_eq!(asset.vertices().len(), 8);
        assert_eq!(asset.triangles().len(), 12);
        assert_eq!(asset.topology().boundary_edge_count, 0);
        assert_eq!(asset.topology().non_manifold_edge_count, 0);
        assert_eq!(asset.topology().winding_mismatch_count, 0);
        assert_eq!(asset.topology().component_count, 1);
        assert!(asset.topology().closed);
        assert!(asset.content_digest().starts_with("sha256:"));
        assert!(matches!(asset.source(), MeshAssetSource::Imported));
    }

    #[test]
    fn indexed_mesh_asset_sanitizes_degenerate_evaluated_ir_triangles() {
        let mut mesh = IrMesh::cuboid(2.0, 2.0, 2.0, None);
        let normal = Vector3::new(0.0, 0.0, 1.0);
        mesh.polygons.push(IrPolygon::new(
            vec![
                IrVertex::new(Point3::new(10.0, 0.0, 0.0), normal),
                IrVertex::new(Point3::new(11.0, 0.0, 0.0), normal),
                IrVertex::new(Point3::new(12.0, 0.0, 0.0), normal),
            ],
            None,
        ));

        let asset = IndexedMeshAsset::from_ir_mesh(MeshAssetSource::Imported, &mesh)
            .expect("evaluated IR sidecar must ignore zero-area triangles");

        assert_eq!(asset.vertices().len(), 8);
        assert_eq!(asset.triangles().len(), 12);
        asset
            .validate_for_boolean()
            .expect("remaining cuboid stays Boolean-ready");
    }

    #[test]
    fn indexed_mesh_asset_rejects_out_of_bounds_and_degenerate_triangles() {
        let out_of_bounds = IndexedMeshAsset::new(
            MeshAssetSource::Imported,
            vec![[0.0, 0.0, 0.0]],
            vec![[0, 1, 0]],
        )
        .expect_err("out of bounds index must fail");
        assert!(out_of_bounds.to_string().contains("out-of-bounds"));

        let degenerate = IndexedMeshAsset::new(
            MeshAssetSource::Imported,
            vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            vec![[0, 0, 1]],
        )
        .expect_err("repeated index must fail");
        assert!(degenerate.to_string().contains("degenerate"));
    }

    #[test]
    fn indexed_mesh_asset_accepts_valid_tiny_triangles_independent_of_scale() {
        let asset = IndexedMeshAsset::new(
            MeshAssetSource::Imported,
            vec![[0.0, 0.0, 0.0], [1e-9, 0.0, 0.0], [0.0, 1e-9, 0.0]],
            vec![[0, 1, 2]],
        )
        .expect("non-collinear tiny triangle must remain valid");

        assert_eq!(asset.triangles(), &[[0, 1, 2]]);
    }

    #[test]
    fn indexed_mesh_asset_reports_open_topology_and_blocks_boolean_admission() {
        let asset = IndexedMeshAsset::new(
            MeshAssetSource::Imported,
            vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            vec![[0, 1, 2]],
        )
        .expect("open mesh remains inspectable");

        assert_eq!(asset.topology().boundary_edge_count, 3);
        assert_eq!(asset.topology().component_count, 1);
        assert!(!asset.topology().closed);
        assert!(asset
            .validate_for_boolean()
            .expect_err("open mesh must not reach Boolean kernel")
            .to_string()
            .contains("boundary edges: 3"));
    }

    #[test]
    fn indexed_mesh_asset_allows_multiple_closed_manifold_components() {
        let asset = IndexedMeshAsset::new(
            MeshAssetSource::Imported,
            vec![
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 0.0, 1.0],
                [3.0, 0.0, 0.0],
                [4.0, 0.0, 0.0],
                [3.0, 1.0, 0.0],
                [3.0, 0.0, 1.0],
            ],
            vec![
                [0, 2, 1],
                [0, 1, 3],
                [1, 2, 3],
                [2, 0, 3],
                [4, 6, 5],
                [4, 5, 7],
                [5, 6, 7],
                [6, 4, 7],
            ],
        )
        .expect("two closed tetrahedra");

        assert_eq!(asset.topology().component_count, 2);
        asset
            .validate_for_boolean()
            .expect("disjoint closed components remain Boolean-ready");
    }

    #[test]
    fn indexed_mesh_digest_is_stable_for_identical_geometry() {
        let vertices = vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
        let triangles = vec![[0, 1, 2]];

        let first = IndexedMeshAsset::new(
            MeshAssetSource::Imported,
            vertices.clone(),
            triangles.clone(),
        )
        .expect("first mesh");
        let second = IndexedMeshAsset::new(MeshAssetSource::Imported, vertices, triangles)
            .expect("second mesh");

        assert_eq!(first.content_digest(), second.content_digest());
    }

    #[test]
    fn indexed_mesh_cache_round_trip_revalidates_content_digest() {
        let root =
            std::env::temp_dir().join(format!("ecky-indexed-mesh-cache-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).expect("temp dir");
        let path = root.join("part.indexed-mesh.json");
        let asset = IndexedMeshAsset::from_ir_mesh(
            MeshAssetSource::Imported,
            &IrMesh::cuboid(2.0, 2.0, 2.0, None),
        )
        .expect("indexed asset");
        asset.write_cache(&path).expect("write cache");

        let restored = IndexedMeshAsset::read_cache(&path).expect("read cache");
        assert_eq!(restored.content_digest(), asset.content_digest());
        assert_eq!(restored.vertices(), asset.vertices());
        assert_eq!(restored.triangles(), asset.triangles());

        let raw = std::fs::read_to_string(&path).expect("cache text");
        let tampered = raw.replacen("sha256:", "sha256:tampered-", 1);
        std::fs::write(&path, tampered).expect("tamper cache");
        assert!(IndexedMeshAsset::read_cache(&path)
            .expect_err("tampered digest")
            .to_string()
            .contains("digest mismatch"));

        std::fs::remove_dir_all(root).expect("cleanup");
    }

    // --- Indexed mesh source provenance round-trip (sidecar source DTO) ---

    #[test]
    fn indexed_mesh_cache_round_trips_exact_source_for_every_variant() {
        let root =
            std::env::temp_dir().join(format!("ecky-indexed-mesh-source-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).expect("temp dir");

        let sources = [
            MeshAssetSource::EckyMeshPhase {
                part_id: "body".to_string(),
                node_id: NodeId::new(7),
            },
            MeshAssetSource::Imported,
            MeshAssetSource::Generated {
                provider: "meshy".to_string(),
                model: Some("model-42".to_string()),
            },
            MeshAssetSource::Generated {
                provider: "tri".to_string(),
                model: None,
            },
        ];

        for (index, source) in sources.iter().enumerate() {
            let path = root.join(format!("part-{index}.indexed-mesh.json"));
            let asset = IndexedMeshAsset::from_ir_mesh(
                source.clone(),
                &IrMesh::cuboid(2.0, 2.0, 2.0, None),
            )
            .expect("indexed asset");
            asset.write_cache(&path).expect("write cache");

            let restored = IndexedMeshAsset::read_cache(&path).expect("read cache");
            assert_eq!(restored.source(), source, "source must round-trip exactly");
            assert_eq!(restored.content_digest(), asset.content_digest());
            assert_eq!(restored.vertices(), asset.vertices());
            assert_eq!(restored.triangles(), asset.triangles());
        }

        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn indexed_mesh_sidecar_round_trip_preserves_multipart_bundle_digest() {
        let root = std::env::temp_dir().join(format!(
            "ecky-indexed-mesh-bundle-digest-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&root).expect("temp dir");

        let original = MultipartMeshNativeBundle::new(vec![
            MultipartMeshComponent::new(
                0,
                "body",
                closed_tetrahedron_indexed_asset(
                    MeshAssetSource::EckyMeshPhase {
                        part_id: "body".to_string(),
                        node_id: NodeId::new(7),
                    },
                    [0.0, 0.0, 0.0],
                ),
            ),
            MultipartMeshComponent::new(
                1,
                "imported-island",
                closed_tetrahedron_indexed_asset(
                    MeshAssetSource::Generated {
                        provider: "meshy".to_string(),
                        model: Some("model-42".to_string()),
                    },
                    [10.0, 0.0, 0.0],
                ),
            ),
        ])
        .expect("original bundle");

        // Persist each component through the canonical indexed sidecar and
        // rebuild the bundle from the round-tripped assets. The caller supplies
        // no source: it is read back from the sidecar.
        let rebuilt_components: Vec<MultipartMeshComponent> = original
            .components()
            .iter()
            .enumerate()
            .map(|(index, component)| {
                let sidecar = root.join(format!("part-{index}.indexed-mesh.json"));
                component
                    .asset()
                    .write_cache(&sidecar)
                    .expect("write sidecar");
                let restored = IndexedMeshAsset::read_cache(&sidecar).expect("read sidecar");
                assert_eq!(restored.source(), component.source());
                MultipartMeshComponent::new(index, component.label(), restored)
            })
            .collect();

        let rebuilt = MultipartMeshNativeBundle::new(rebuilt_components).expect("rebuilt bundle");

        assert_eq!(
            rebuilt.bundle_digest(),
            original.bundle_digest(),
            "bundle identity must be byte-identical after a source-preserving sidecar round-trip",
        );
        for (original_component, rebuilt_component) in original
            .components()
            .iter()
            .zip(rebuilt.components().iter())
        {
            assert_eq!(
                rebuilt_component.component_id(),
                original_component.component_id(),
            );
        }

        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn indexed_mesh_cache_without_source_field_is_rejected() {
        let root = std::env::temp_dir().join(format!(
            "ecky-indexed-mesh-no-source-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&root).expect("temp dir");
        let path = root.join("part.indexed-mesh.json");

        // Write a valid cache, then strip the required `source` field to model
        // a cache that predates provenance storage. Deserialization must fail
        // honestly so the caller regenerates instead of guessing a source.
        let asset = IndexedMeshAsset::from_ir_mesh(
            MeshAssetSource::Imported,
            &IrMesh::cuboid(2.0, 2.0, 2.0, None),
        )
        .expect("indexed asset");
        asset.write_cache(&path).expect("write cache");

        let mut value: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).expect("read cache"))
                .expect("cache is json");
        let object = value.as_object_mut().expect("cache is object");
        assert!(
            object.remove("source").is_some(),
            "fixture must carry source"
        );
        std::fs::write(&path, serde_json::to_vec(&value).expect("encode"))
            .expect("write sourceless cache");

        let err =
            IndexedMeshAsset::read_cache(&path).expect_err("cache without source must be rejected");
        assert!(
            err.to_string().contains("invalid"),
            "missing source must surface as an invalid-cache validation error, not a fallback: {err}",
        );

        std::fs::remove_dir_all(root).expect("cleanup");
    }

    // --- Standalone STL/3MF indexed decoders (task 6) -----------------------

    /// Closed, consistently-wound tetrahedron used as the golden indexed mesh.
    /// Coordinates are exactly representable in both f32 and f64 so binary STL
    /// round-trip is bit-exact.
    fn golden_tetrahedron_indexed() -> (Vec<[f64; 3]>, Vec<[u32; 3]>) {
        (
            vec![
                [0.0, 0.0, 0.0],
                [4.0, 0.0, 0.0],
                [1.0, 3.0, 0.0],
                [1.0, 1.0, 3.0],
            ],
            vec![[0, 2, 1], [0, 1, 3], [0, 3, 2], [1, 2, 3]],
        )
    }

    fn golden_tetrahedron_triangles() -> Vec<[[f64; 3]; 3]> {
        let (vertices, triangles) = golden_tetrahedron_indexed();
        triangles
            .iter()
            .map(|triangle| {
                [
                    vertices[triangle[0] as usize],
                    vertices[triangle[1] as usize],
                    vertices[triangle[2] as usize],
                ]
            })
            .collect()
    }

    fn golden_cube_triangles() -> Vec<[[f64; 3]; 3]> {
        let vertices = [
            [-1.0, -1.0, -1.0],
            [1.0, -1.0, -1.0],
            [1.0, 1.0, -1.0],
            [-1.0, 1.0, -1.0],
            [-1.0, -1.0, 1.0],
            [1.0, -1.0, 1.0],
            [1.0, 1.0, 1.0],
            [-1.0, 1.0, 1.0],
        ];
        [
            [0, 2, 1],
            [0, 3, 2],
            [4, 5, 6],
            [4, 6, 7],
            [0, 1, 5],
            [0, 5, 4],
            [3, 7, 6],
            [3, 6, 2],
            [0, 4, 7],
            [0, 7, 3],
            [1, 2, 6],
            [1, 6, 5],
        ]
        .into_iter()
        .map(|triangle| {
            [
                vertices[triangle[0]],
                vertices[triangle[1]],
                vertices[triangle[2]],
            ]
        })
        .collect()
    }

    fn mesh_temp_dir(label: &str) -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!(
            "ecky-mesh-decoder-{label}-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&root).expect("temp dir");
        root
    }

    fn write_binary_stl(path: &std::path::Path, triangles: &[[[f64; 3]; 3]]) {
        let mut buf = Vec::with_capacity(84 + triangles.len() * 50);
        buf.extend_from_slice(&[0u8; 80]);
        buf.extend_from_slice(&(triangles.len() as u32).to_le_bytes());
        for triangle in triangles {
            buf.extend_from_slice(&[0u8; 12]); // unused facet normal
            for vertex in triangle {
                for coordinate in vertex {
                    buf.extend_from_slice(&(*coordinate as f32).to_le_bytes());
                }
            }
            buf.extend_from_slice(&[0u8; 2]); // attribute byte count
        }
        std::fs::write(path, buf).expect("write binary stl");
    }

    fn write_ascii_stl(path: &std::path::Path, triangles: &[[[f64; 3]; 3]]) {
        let mut text = String::from("solid fixture\n");
        for triangle in triangles {
            text.push_str("  facet normal 0 0 0\n    outer loop\n");
            for vertex in triangle {
                text.push_str(&format!(
                    "      vertex {} {} {}\n",
                    vertex[0], vertex[1], vertex[2]
                ));
            }
            text.push_str("    endloop\n  endfacet\n");
        }
        text.push_str("endsolid fixture\n");
        std::fs::write(path, text).expect("write ascii stl");
    }

    fn sorted_vertex_set(vertices: &[[f64; 3]]) -> Vec<[u64; 3]> {
        let mut bits: Vec<[u64; 3]> = vertices
            .iter()
            .map(|vertex| vertex.map(canonical_float_bits))
            .collect();
        bits.sort();
        bits
    }

    fn write_3mf(path: &std::path::Path, vertices: &[[f64; 3]], triangles: &[[u32; 3]]) {
        use std::io::Write as _;
        let file = std::fs::File::create(path).expect("create 3mf");
        let mut zip = zip::ZipWriter::new(file);
        let options =
            zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Stored);
        zip.start_file("[Content_Types].xml", options)
            .expect("content types");
        zip.write_all(br#"<?xml version="1.0" encoding="UTF-8"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="model" ContentType="application/vnd.ms-package.3dmanufacturing-3dmodel+xml"/></Types>"#).expect("content types body");
        zip.add_directory("_rels/", options).expect("rels dir");
        zip.start_file("_rels/.rels", options).expect("rels");
        zip.write_all(br#"<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Target="/3D/3dmodel.model" Id="rel0" Type="http://schemas.microsoft.com/3dmanufacturing/2013/01/3dmodel"/></Relationships>"#).expect("rels body");
        zip.add_directory("3D/", options).expect("3d dir");
        zip.start_file("3D/3dmodel.model", options).expect("model");
        let mut xml = String::new();
        xml.push_str(r#"<?xml version="1.0" encoding="UTF-8"?><model unit="millimeter" xml:lang="en-US" xmlns="http://schemas.microsoft.com/3dmanufacturing/core/2015/02"><resources><object id="1" type="model"><mesh><vertices>"#);
        for vertex in vertices {
            xml.push_str(&format!(
                r#"<vertex x="{}" y="{}" z="{}"/>"#,
                vertex[0], vertex[1], vertex[2]
            ));
        }
        xml.push_str("</vertices><triangles>");
        for triangle in triangles {
            xml.push_str(&format!(
                r#"<triangle v1="{}" v2="{}" v3="{}"/>"#,
                triangle[0], triangle[1], triangle[2]
            ));
        }
        xml.push_str("</triangles></mesh></object></resources><build></build></model>");
        zip.write_all(xml.as_bytes()).expect("model body");
        zip.finish().expect("finish 3mf");
    }

    fn write_3mf_without_model(path: &std::path::Path) {
        use std::io::Write as _;
        let file = std::fs::File::create(path).expect("create 3mf");
        let mut zip = zip::ZipWriter::new(file);
        let options =
            zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Stored);
        zip.start_file("[Content_Types].xml", options)
            .expect("content types");
        zip.write_all(br#"<?xml version="1.0" encoding="UTF-8"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"/>"#).expect("body");
        zip.finish().expect("finish");
    }

    #[test]
    fn indexed_mesh_asset_from_binary_stl_preserves_authored_coordinates() {
        let root = mesh_temp_dir("stl-binary");
        let path = root.join("tetra.stl");
        write_binary_stl(&path, &golden_tetrahedron_triangles());

        let asset = IndexedMeshAsset::from_stl(MeshAssetSource::Imported, &path)
            .expect("binary stl decode");

        let (golden_vertices, _) = golden_tetrahedron_indexed();
        assert_eq!(asset.vertices().len(), 4);
        assert_eq!(asset.triangles().len(), 4);
        assert_eq!(
            sorted_vertex_set(asset.vertices()),
            sorted_vertex_set(&golden_vertices)
        );
        assert_eq!(asset.topology().boundary_edge_count, 0);
        assert_eq!(asset.topology().non_manifold_edge_count, 0);
        assert_eq!(asset.topology().winding_mismatch_count, 0);
        assert_eq!(asset.topology().component_count, 1);
        assert!(asset.topology().closed);
        assert!(asset.content_digest().starts_with("sha256:"));
        asset
            .validate_for_boolean()
            .expect("closed manifold tetrahedron");
    }

    #[test]
    fn indexed_mesh_asset_from_ascii_stl_preserves_authored_coordinates() {
        let root = mesh_temp_dir("stl-ascii");
        let path = root.join("tetra.stl");
        write_ascii_stl(&path, &golden_tetrahedron_triangles());

        let asset =
            IndexedMeshAsset::from_stl(MeshAssetSource::Imported, &path).expect("ascii stl decode");

        let (golden_vertices, _) = golden_tetrahedron_indexed();
        assert_eq!(asset.vertices().len(), 4);
        assert_eq!(asset.triangles().len(), 4);
        assert_eq!(
            sorted_vertex_set(asset.vertices()),
            sorted_vertex_set(&golden_vertices)
        );
        assert_eq!(asset.topology().boundary_edge_count, 0);
        assert!(asset.topology().closed);
        asset
            .validate_for_boolean()
            .expect("closed manifold tetrahedron");
    }

    #[test]
    fn indexed_mesh_asset_from_stl_does_not_weld_explicit_seam() {
        // Explicit imported assets retain supplied coordinates. A 1e-9 seam that
        // the evaluated-CAD weld would collapse must stay open here, so Boolean
        // admission is rejected and both authored coordinates survive.
        let root = mesh_temp_dir("stl-no-weld");
        let path = root.join("seam.stl");
        let triangles = [
            [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            [[0.0, 1.0e-9, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        ];
        write_binary_stl(&path, &triangles);

        let asset = IndexedMeshAsset::from_stl(MeshAssetSource::Imported, &path)
            .expect("open mesh decodes");

        // The two shared edge vertices merge (byte-identical), but the 1e-9 seam
        // vertex stays distinct → 4 vertices, not the 3 a weld would produce.
        assert_eq!(asset.vertices().len(), 4);
        assert!(asset.topology().boundary_edge_count > 0);
        assert!(!asset.topology().closed);
        assert!(asset
            .validate_for_boolean()
            .expect_err("unwelded seam must block Boolean admission")
            .to_string()
            .contains("boundary edges"));
    }

    #[test]
    fn indexed_mesh_asset_from_stl_rejects_malformed_input() {
        let root = mesh_temp_dir("stl-malformed");

        let empty = root.join("empty.stl");
        std::fs::write(&empty, b"").expect("write");
        IndexedMeshAsset::from_stl(MeshAssetSource::Imported, &empty)
            .expect_err("empty stl must fail");

        let bad_ascii = root.join("bad.stl");
        std::fs::write(
            &bad_ascii,
            b"solid bad\n  facet normal 0 0 0\n    outer loop\n      vertex foo 0 0\n    endloop\n  endfacet\nendsolid bad\n",
        )
        .expect("write");
        assert!(
            IndexedMeshAsset::from_stl(MeshAssetSource::Imported, &bad_ascii)
                .expect_err("non-numeric vertex must fail")
                .to_string()
                .to_lowercase()
                .contains("stl")
        );

        let no_triangles = root.join("none.stl");
        std::fs::write(&no_triangles, b"solid none\nendsolid none\n").expect("write");
        assert!(
            IndexedMeshAsset::from_stl(MeshAssetSource::Imported, &no_triangles)
                .expect_err("triangle-free stl must fail")
                .to_string()
                .contains("no triangles")
        );

        // Binary header claims one triangle but supplies no triangle bytes.
        let mut truncated = vec![0u8; 84];
        truncated[80..84].copy_from_slice(&1u32.to_le_bytes());
        let truncated_path = root.join("trunc.stl");
        std::fs::write(&truncated_path, &truncated).expect("write");
        IndexedMeshAsset::from_stl(MeshAssetSource::Imported, &truncated_path)
            .expect_err("truncated binary stl must fail");
    }

    #[test]
    fn indexed_mesh_asset_from_stl_avoids_stl_as_cache_representation() {
        // The decoder produces the canonical indexed sidecar, never an STL
        // cache. Round-tripping the decoded asset through the indexed cache
        // preserves authored coordinates and digest without touching STL.
        let root = mesh_temp_dir("stl-cache");
        let stl_path = root.join("tetra.stl");
        write_binary_stl(&stl_path, &golden_tetrahedron_triangles());
        let asset =
            IndexedMeshAsset::from_stl(MeshAssetSource::Imported, &stl_path).expect("decode");

        let sidecar = root.join("tetra.indexed-mesh.json");
        asset.write_cache(&sidecar).expect("write sidecar");
        assert_eq!(sidecar.extension().and_then(|e| e.to_str()), Some("json"));

        let restored = IndexedMeshAsset::read_cache(&sidecar).expect("read sidecar");
        assert_eq!(restored.content_digest(), asset.content_digest());
        assert_eq!(restored.vertices(), asset.vertices());
        assert_eq!(restored.triangles(), asset.triangles());
        restored
            .validate_for_boolean()
            .expect("still Boolean-ready");
    }

    #[test]
    fn indexed_mesh_import_preparation_reports_target_not_reached_with_provenance() {
        let root = mesh_temp_dir("stl-preparation-target-blocked");
        let path = root.join("cube.stl");
        write_binary_stl(&path, &golden_cube_triangles());
        let raw_before = std::fs::read(&path).expect("raw source before preparation");

        let policy = IndexedMeshPreparationPolicy::new(Some(4), 1.0e-6, true).expect("prep policy");
        let result =
            IndexedMeshAsset::prepare_imported_file(MeshAssetSource::Imported, &path, &policy)
                .expect("prep result");

        assert!(result.asset().triangles().len() > 4);
        assert!(matches!(
            result.warnings(),
            [IndexedMeshPreparationWarning::TargetNotReached {
                requested_triangle_count: 4,
                ..
            }]
        ));
        assert_eq!(result.provenance().raw_triangle_count, 12);
        assert_eq!(
            result.provenance().prepared_triangle_count,
            result.asset().triangles().len()
        );
        assert!(result.provenance().raw_source_digest.starts_with("sha256:"));
        assert!(result
            .provenance()
            .prepared_content_digest
            .starts_with("sha256:"));
        assert_eq!(
            result.provenance().algorithm_version,
            "meshopt-0.6.2:error-absolute:lock-border"
        );
        assert_eq!(
            std::fs::read(&path).expect("raw source after preparation"),
            raw_before
        );

        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn indexed_mesh_import_preparation_is_deterministic_and_error_bounded() {
        let root = mesh_temp_dir("stl-preparation-deterministic");
        let path = root.join("cube.stl");
        write_binary_stl(&path, &golden_cube_triangles());
        let policy = IndexedMeshPreparationPolicy::new(Some(4), 4.0, true).expect("prep policy");

        let first =
            IndexedMeshAsset::prepare_imported_file(MeshAssetSource::Imported, &path, &policy)
                .expect("first preparation");
        let second =
            IndexedMeshAsset::prepare_imported_file(MeshAssetSource::Imported, &path, &policy)
                .expect("second preparation");

        assert_eq!(
            first.asset().content_digest(),
            second.asset().content_digest()
        );
        assert_eq!(first.asset().triangles(), second.asset().triangles());
        assert!(first.provenance().max_error_mm <= policy.max_error_mm);
        assert_eq!(first.asset().topology().component_count, 1);
        assert!(first.asset().topology().closed);
        assert_eq!(first.asset().topology().non_manifold_edge_count, 0);
        assert_eq!(first.asset().topology().winding_mismatch_count, 0);

        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn preparation_canonicalizes_exact_duplicate_stl_faces_without_repairing_legacy_import() {
        let root = mesh_temp_dir("stl-preparation-duplicate-face");
        let path = root.join("cube-with-duplicate.stl");
        let mut triangles = golden_cube_triangles();
        triangles.push(triangles[0]);
        write_binary_stl(&path, &triangles);

        let legacy = IndexedMeshAsset::from_stl(MeshAssetSource::Imported, &path)
            .expect_err("legacy exact import must retain strict duplicate validation");
        assert!(legacy.to_string().contains("duplicates triangle"));

        let policy = IndexedMeshPreparationPolicy::new(Some(12), 0.05, true).unwrap();
        let prepared =
            IndexedMeshAsset::prepare_imported_file(MeshAssetSource::Imported, &path, &policy)
                .expect("prepared import may remove only exact duplicate faces");

        assert_eq!(prepared.asset().triangles().len(), 12);
        assert_eq!(prepared.provenance().raw_triangle_count, 13);
        assert_eq!(prepared.provenance().duplicate_triangle_count_removed, 1);

        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn duplicate_face_canonicalization_keeps_same_winding_and_cancels_opposites() {
        let (same_winding, removed) = deduplicate_triangle_indices(vec![[0, 1, 2], [1, 2, 0]]);
        assert_eq!(same_winding, vec![[0, 1, 2]]);
        assert_eq!(removed, 1);

        let (opposite_winding, removed) = deduplicate_triangle_indices(vec![[0, 1, 2], [0, 2, 1]]);
        assert!(opposite_winding.is_empty());
        assert_eq!(removed, 2);
    }

    #[test]
    fn indexed_mesh_asset_from_3mf_preserves_authored_indexing() {
        let root = mesh_temp_dir("3mf-valid");
        let path = root.join("tetra.3mf");
        let (vertices, triangles) = golden_tetrahedron_indexed();
        write_3mf(&path, &vertices, &triangles);

        let asset =
            IndexedMeshAsset::from_3mf(MeshAssetSource::Imported, &path).expect("3mf decode");

        // 3MF is already explicitly indexed; authored order is retained as-is.
        assert_eq!(asset.vertices(), vertices.as_slice());
        assert_eq!(asset.triangles(), triangles.as_slice());
        assert_eq!(asset.topology().boundary_edge_count, 0);
        assert_eq!(asset.topology().non_manifold_edge_count, 0);
        assert_eq!(asset.topology().winding_mismatch_count, 0);
        assert_eq!(asset.topology().component_count, 1);
        assert!(asset.topology().closed);
        assert!(asset.content_digest().starts_with("sha256:"));
        asset
            .validate_for_boolean()
            .expect("closed manifold tetrahedron");
    }

    #[test]
    fn indexed_mesh_asset_from_3mf_aggregates_multiple_objects() {
        // Two disjoint tetrahedra as separate 3MF objects aggregate into one
        // indexed asset with two closed components.
        let root = mesh_temp_dir("3mf-multi");
        let path = root.join("two.3mf");
        let (first_vertices, first_triangles) = golden_tetrahedron_indexed();
        let second_vertices: Vec<[f64; 3]> = first_vertices
            .iter()
            .map(|v| [v[0] + 10.0, v[1], v[2]])
            .collect();
        // 3MF triangle indices are local to each object's vertex list; the
        // decoder applies the per-object vertex base when aggregating.
        let second_triangles: Vec<[u32; 3]> = first_triangles.clone();

        use std::io::Write as _;
        let file = std::fs::File::create(&path).expect("create");
        let mut zip = zip::ZipWriter::new(file);
        let options =
            zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Stored);
        zip.start_file("[Content_Types].xml", options).expect("ct");
        zip.write_all(br#"<?xml version="1.0" encoding="UTF-8"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="model" ContentType="application/vnd.ms-package.3dmanufacturing-3dmodel+xml"/></Types>"#).expect("ct body");
        zip.start_file("3D/3dmodel.model", options).expect("model");
        let mut xml = String::from(
            r#"<?xml version="1.0" encoding="UTF-8"?><model unit="millimeter" xmlns="http://schemas.microsoft.com/3dmanufacturing/core/2015/02"><resources>"#,
        );
        for (object_id, (verts, tris)) in [
            (1, (&first_vertices, &first_triangles)),
            (2, (&second_vertices, &second_triangles)),
        ] {
            xml.push_str(&format!(
                r#"<object id="{object_id}" type="model"><mesh><vertices>"#
            ));
            for v in verts {
                xml.push_str(&format!(
                    r#"<vertex x="{}" y="{}" z="{}"/>"#,
                    v[0], v[1], v[2]
                ));
            }
            xml.push_str("</vertices><triangles>");
            for t in tris {
                xml.push_str(&format!(
                    r#"<triangle v1="{}" v2="{}" v3="{}"/>"#,
                    t[0], t[1], t[2]
                ));
            }
            xml.push_str("</triangles></mesh></object>");
        }
        xml.push_str("</resources><build></build></model>");
        zip.write_all(xml.as_bytes()).expect("body");
        zip.finish().expect("finish");

        let asset =
            IndexedMeshAsset::from_3mf(MeshAssetSource::Imported, &path).expect("3mf decode");

        assert_eq!(asset.vertices().len(), 8);
        assert_eq!(asset.triangles().len(), 8);
        assert_eq!(asset.topology().component_count, 2);
        assert_eq!(asset.topology().boundary_edge_count, 0);
        assert!(asset.topology().closed);
        asset
            .validate_for_boolean()
            .expect("two disjoint closed components");
    }

    #[test]
    fn indexed_mesh_asset_from_3mf_rejects_malformed_input() {
        let root = mesh_temp_dir("3mf-malformed");

        let not_zip = root.join("notzip.3mf");
        std::fs::write(&not_zip, b"<not a zip>").expect("write");
        assert!(
            IndexedMeshAsset::from_3mf(MeshAssetSource::Imported, &not_zip)
                .expect_err("non-zip must fail")
                .to_string()
                .to_lowercase()
                .contains("3mf")
        );

        let no_model = root.join("nomodel.3mf");
        write_3mf_without_model(&no_model);
        assert!(
            IndexedMeshAsset::from_3mf(MeshAssetSource::Imported, &no_model)
                .expect_err("missing model part must fail")
                .to_string()
                .to_lowercase()
                .contains("3mf")
        );

        let empty_mesh = root.join("empty.3mf");
        write_3mf(&empty_mesh, &[], &[]);
        assert!(
            IndexedMeshAsset::from_3mf(MeshAssetSource::Imported, &empty_mesh)
                .expect_err("triangle-free 3mf must fail")
                .to_string()
                .contains("no triangles")
        );

        let bad_index = root.join("badidx.3mf");
        write_3mf(&bad_index, &[[0.0, 0.0, 0.0]], &[[0, 1, 2]]);
        assert!(
            IndexedMeshAsset::from_3mf(MeshAssetSource::Imported, &bad_index)
                .expect_err("out-of-range index must fail")
                .to_string()
                .contains("out-of-bounds")
        );
    }

    // --- Multipart mesh-native bundle export (task 6) ----------------------

    fn closed_tetrahedron_indexed_asset(
        source: MeshAssetSource,
        offset: [f64; 3],
    ) -> IndexedMeshAsset {
        let (mut vertices, triangles) = golden_tetrahedron_indexed();
        for vertex in vertices.iter_mut() {
            vertex[0] += offset[0];
            vertex[1] += offset[1];
            vertex[2] += offset[2];
        }
        IndexedMeshAsset::new(source, vertices, triangles).expect("closed tetrahedron asset")
    }

    #[test]
    fn multipart_mesh_native_bundle_exports_each_component_with_identity_and_provenance_without_step(
    ) {
        let component_a_asset = closed_tetrahedron_indexed_asset(
            MeshAssetSource::EckyMeshPhase {
                part_id: "body".to_string(),
                node_id: NodeId::new(7),
            },
            [0.0, 0.0, 0.0],
        );
        let component_b_asset =
            closed_tetrahedron_indexed_asset(MeshAssetSource::Imported, [10.0, 0.0, 0.0]);
        assert_ne!(
            component_a_asset.content_digest(),
            component_b_asset.content_digest(),
            "fixture components must be geometrically distinct",
        );

        let bundle = MultipartMeshNativeBundle::new(vec![
            MultipartMeshComponent::new(0, "body", component_a_asset.clone()),
            MultipartMeshComponent::new(1, "imported-island", component_b_asset.clone()),
        ])
        .expect("mesh-native multipart bundle");

        // Every component is exported with deterministic identity and provenance.
        let components = bundle.components();
        assert_eq!(components.len(), 2);
        assert!(
            matches!(components[0].source(), MeshAssetSource::EckyMeshPhase { part_id, node_id }
                if part_id == "body" && node_id.raw() == 7)
        );
        assert!(matches!(components[1].source(), MeshAssetSource::Imported));
        assert_eq!(
            components[0].content_digest(),
            component_a_asset.content_digest()
        );
        assert_eq!(
            components[1].content_digest(),
            component_b_asset.content_digest()
        );
        assert_eq!(components[0].label(), "body");
        assert_eq!(components[1].label(), "imported-island");

        // Identity is deterministic and order-stable: the id encodes the
        // authored index and the content digest, so the same authored bundle
        // always reconstructs identical ids.
        let id_a_first = components[0].component_id().to_string();
        let id_b_first = components[1].component_id().to_string();
        assert!(id_a_first.starts_with("component-0-"));
        assert!(id_b_first.starts_with("component-1-"));
        let hex_a = component_a_asset
            .content_digest()
            .strip_prefix("sha256:")
            .expect("sha256 digest");
        let hex_b = component_b_asset
            .content_digest()
            .strip_prefix("sha256:")
            .expect("sha256 digest");
        assert!(id_a_first.contains(&hex_a[..hex_a.len().min(12)]));
        assert!(id_b_first.contains(&hex_b[..hex_b.len().min(12)]));

        let rebuilt = MultipartMeshNativeBundle::new(vec![
            MultipartMeshComponent::new(0, "body", component_a_asset.clone()),
            MultipartMeshComponent::new(1, "imported-island", component_b_asset.clone()),
        ])
        .expect("rebuilt bundle");
        assert_eq!(rebuilt.components()[0].component_id(), id_a_first);
        assert_eq!(rebuilt.components()[1].component_id(), id_b_first);
        assert_eq!(
            rebuilt.bundle_digest(),
            bundle.bundle_digest(),
            "bundle identity is deterministic for identical ordered components",
        );

        // Authored operand order participates in identity: swapping components
        // changes the bundle digest.
        let swapped = MultipartMeshNativeBundle::new(vec![
            MultipartMeshComponent::new(0, "imported-island", component_b_asset.clone()),
            MultipartMeshComponent::new(1, "body", component_a_asset.clone()),
        ])
        .expect("swapped bundle");
        assert_ne!(
            swapped.bundle_digest(),
            bundle.bundle_digest(),
            "authored component order must participate in bundle identity",
        );

        // Representation is mesh-native and no STEP is fabricated.
        assert_eq!(
            bundle.representation(),
            crate::contracts::GeometryRepresentation::MeshNative
        );
        assert!(
            !bundle.has_step_artifact(),
            "mesh-native multipart export must never fabricate STEP",
        );

        // Exported components remain Boolean-ready indexed manifold meshes.
        for component in bundle.components() {
            component
                .asset()
                .validate_for_boolean()
                .expect("Boolean-ready mesh-native component");
        }

        assert!(bundle.bundle_digest().starts_with("sha256:"));
    }

    #[test]
    fn multipart_mesh_native_bundle_rejects_empty_component_set() {
        let err =
            MultipartMeshNativeBundle::new(Vec::new()).expect_err("empty bundle must be rejected");
        assert!(
            err.to_string().contains("at least one component"),
            "empty rejection must name the missing component requirement"
        );
    }

    #[test]
    fn multipart_bundle_identity_is_label_independent_but_provenance_sensitive() {
        // Geometry and provenance define identity; the human label does not.
        let same_geometry_a = closed_tetrahedron_indexed_asset(
            MeshAssetSource::EckyMeshPhase {
                part_id: "body".to_string(),
                node_id: NodeId::new(7),
            },
            [0.0, 0.0, 0.0],
        );
        let same_geometry_b = closed_tetrahedron_indexed_asset(
            MeshAssetSource::EckyMeshPhase {
                part_id: "body".to_string(),
                node_id: NodeId::new(7),
            },
            [0.0, 0.0, 0.0],
        );
        assert_eq!(
            same_geometry_a.content_digest(),
            same_geometry_b.content_digest()
        );

        let relabeled = MultipartMeshNativeBundle::new(vec![MultipartMeshComponent::new(
            0,
            "renamed-label",
            same_geometry_a.clone(),
        )])
        .expect("relabeled bundle");
        let original = MultipartMeshNativeBundle::new(vec![MultipartMeshComponent::new(
            0,
            "original-label",
            same_geometry_b.clone(),
        )])
        .expect("original bundle");
        assert_eq!(
            relabeled.bundle_digest(),
            original.bundle_digest(),
            "label is presentation metadata and must not affect bundle identity",
        );
        assert_ne!(
            relabeled.components()[0].label(),
            original.components()[0].label()
        );

        // Distinct provenance for identical geometry changes identity.
        let other_node = closed_tetrahedron_indexed_asset(
            MeshAssetSource::EckyMeshPhase {
                part_id: "body".to_string(),
                node_id: NodeId::new(8),
            },
            [0.0, 0.0, 0.0],
        );
        assert_eq!(
            other_node.content_digest(),
            same_geometry_a.content_digest(),
            "same geometry must share a mesh digest",
        );
        let other_bundle = MultipartMeshNativeBundle::new(vec![MultipartMeshComponent::new(
            0, "body", other_node,
        )])
        .expect("other-provenance bundle");
        assert_ne!(
            other_bundle.bundle_digest(),
            original.bundle_digest(),
            "provenance must participate in bundle identity",
        );
    }

    #[test]
    fn multipart_component_id_is_unique_per_authored_index() {
        let asset = closed_tetrahedron_indexed_asset(MeshAssetSource::Imported, [0.0, 0.0, 0.0]);
        // The same placed asset appears at two authored positions; identity is
        // disambiguated by index, so ids never collide.
        let first = MultipartMeshComponent::new(0, "a", asset.clone());
        let second = MultipartMeshComponent::new(1, "b", asset.clone());
        assert_ne!(first.component_id(), second.component_id());
        assert_eq!(first.content_digest(), second.content_digest());
    }
}
