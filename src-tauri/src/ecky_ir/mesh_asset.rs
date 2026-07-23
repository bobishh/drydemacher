use std::collections::{BTreeMap, VecDeque};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::contracts::{AppError, AppResult};
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

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IndexedMeshCacheArtifact {
    schema_version: u32,
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

    pub(crate) fn read_cache(source: MeshAssetSource, path: &Path) -> AppResult<Self> {
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

        let restored =
            IndexedMeshAsset::read_cache(MeshAssetSource::Imported, &path).expect("read cache");
        assert_eq!(restored.content_digest(), asset.content_digest());
        assert_eq!(restored.vertices(), asset.vertices());
        assert_eq!(restored.triangles(), asset.triangles());

        let raw = std::fs::read_to_string(&path).expect("cache text");
        let tampered = raw.replacen("sha256:", "sha256:tampered-", 1);
        std::fs::write(&path, tampered).expect("tamper cache");
        assert!(
            IndexedMeshAsset::read_cache(MeshAssetSource::Imported, &path)
                .expect_err("tampered digest")
                .to_string()
                .contains("digest mismatch")
        );

        std::fs::remove_dir_all(root).expect("cleanup");
    }
}
