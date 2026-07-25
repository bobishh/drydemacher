use std::collections::{HashMap, HashSet, VecDeque};

use crate::contracts::{AppError, AppErrorCode, AppResult};
use csgrs::float_types::parry3d::na::{Point3, Vector3};
use csgrs::mesh::polygon::Polygon as IrPolygon;
use csgrs::mesh::vertex::Vertex as IrVertex;

use super::shared::IrMesh;

pub(crate) use ecky_render::scheme::compiler::{
    MAX_MESH_LITERAL_TRIANGLES, MAX_MESH_LITERAL_VERTICES,
};

pub(super) fn build_mesh_literal(
    operation: &str,
    vertices: Vec<[f64; 3]>,
    triangles: Vec<[usize; 3]>,
    require_closed: bool,
) -> AppResult<IrMesh> {
    validate_budget(
        operation,
        "vertices",
        vertices.len(),
        MAX_MESH_LITERAL_VERTICES,
    )?;
    validate_budget(
        operation,
        "triangles",
        triangles.len(),
        MAX_MESH_LITERAL_TRIANGLES,
    )?;
    if vertices.is_empty() {
        return Err(mesh_error(operation, "vertex list is empty"));
    }
    if triangles.is_empty() {
        return Err(mesh_error(operation, "triangle list is empty"));
    }
    for (index, point) in vertices.iter().enumerate() {
        if point.iter().any(|component| !component.is_finite()) {
            return Err(mesh_error(
                operation,
                format!("vertex {index} contains a non-finite coordinate"),
            ));
        }
    }

    let mut canonical_triangles = HashMap::<[usize; 3], usize>::new();
    let mut edge_incidence = HashMap::<(usize, usize), Vec<(usize, bool)>>::new();
    for (triangle_index, triangle) in triangles.iter().copied().enumerate() {
        for vertex_index in triangle {
            if vertex_index >= vertices.len() {
                return Err(mesh_error(
                    operation,
                    format!(
                        "triangle {triangle_index} references index {vertex_index}, but mesh has {} vertices",
                        vertices.len()
                    ),
                ));
            }
        }
        if triangle[0] == triangle[1] || triangle[1] == triangle[2] || triangle[2] == triangle[0] {
            return Err(mesh_error(
                operation,
                format!("triangle {triangle_index} has a repeated vertex index"),
            ));
        }

        let a = point_vector(vertices[triangle[0]]);
        let b = point_vector(vertices[triangle[1]]);
        let c = point_vector(vertices[triangle[2]]);
        if (b - a).cross(&(c - a)).norm_squared() <= f64::EPSILON {
            return Err(mesh_error(
                operation,
                format!("triangle {triangle_index} has zero area"),
            ));
        }

        let mut canonical = triangle;
        canonical.sort_unstable();
        if let Some(original_index) = canonical_triangles.insert(canonical, triangle_index) {
            return Err(mesh_error(
                operation,
                format!("triangle {triangle_index} duplicates triangle {original_index}"),
            ));
        }

        for (start, end) in [
            (triangle[0], triangle[1]),
            (triangle[1], triangle[2]),
            (triangle[2], triangle[0]),
        ] {
            let edge = if start < end {
                (start, end)
            } else {
                (end, start)
            };
            edge_incidence
                .entry(edge)
                .or_default()
                .push((triangle_index, start < end));
        }
    }

    let boundary_edges = edge_incidence
        .values()
        .filter(|incidence| incidence.len() == 1)
        .count();
    let non_manifold_edges = edge_incidence
        .values()
        .filter(|incidence| incidence.len() > 2)
        .count();
    let inconsistent_edges = edge_incidence
        .values()
        .filter(|incidence| incidence.len() == 2 && incidence[0].1 == incidence[1].1)
        .count();
    let connected_components = triangle_component_count(triangles.len(), &edge_incidence);
    let signed_volume = triangles.iter().fold(0.0, |volume, triangle| {
        let a = point_vector(vertices[triangle[0]]);
        let b = point_vector(vertices[triangle[1]]);
        let c = point_vector(vertices[triangle[2]]);
        volume + a.dot(&b.cross(&c)) / 6.0
    });

    if non_manifold_edges > 0 {
        return Err(mesh_error(
            operation,
            format!(
                "non-manifold edges: {non_manifold_edges}; boundary edges: {boundary_edges}; connected components: {connected_components}"
            ),
        ));
    }
    if inconsistent_edges > 0 {
        return Err(mesh_error(
            operation,
            format!(
                "inconsistent winding across {inconsistent_edges} edges; boundary edges: {boundary_edges}; connected components: {connected_components}"
            ),
        ));
    }
    if require_closed && boundary_edges > 0 {
        return Err(mesh_error(
            operation,
            format!(
                "boundary edges: {boundary_edges}; non-manifold edges: 0; connected components: {connected_components}"
            ),
        ));
    }
    if require_closed && connected_components != 1 {
        return Err(mesh_error(
            operation,
            format!("connected components: {connected_components}; expected 1"),
        ));
    }
    if require_closed && signed_volume.abs() <= f64::EPSILON {
        return Err(mesh_error(operation, "signed volume is zero"));
    }

    let polygons = triangles
        .iter()
        .map(|triangle| {
            let a = point_vector(vertices[triangle[0]]);
            let b = point_vector(vertices[triangle[1]]);
            let c = point_vector(vertices[triangle[2]]);
            let normal = (b - a).cross(&(c - a)).normalize();
            IrPolygon::new(
                triangle
                    .iter()
                    .map(|index| {
                        let point = vertices[*index];
                        IrVertex::new(Point3::new(point[0], point[1], point[2]), normal)
                    })
                    .collect(),
                None,
            )
        })
        .collect::<Vec<_>>();
    Ok(IrMesh::from_polygons(&polygons, None))
}

fn validate_budget(operation: &str, kind: &str, observed: usize, allowed: usize) -> AppResult<()> {
    if observed <= allowed {
        return Ok(());
    }
    Err(mesh_error(
        operation,
        format!("{kind} count {observed} exceeds allowed count {allowed}"),
    ))
}

fn mesh_error(operation: &str, details: impl Into<String>) -> AppError {
    AppError::with_details(
        AppErrorCode::Validation,
        format!("Invalid `{operation}` mesh literal."),
        details,
    )
    .with_operation(operation)
}

fn point_vector(point: [f64; 3]) -> Vector3<f64> {
    Vector3::new(point[0], point[1], point[2])
}

fn triangle_component_count(
    triangle_count: usize,
    edge_incidence: &HashMap<(usize, usize), Vec<(usize, bool)>>,
) -> usize {
    let mut neighbors = vec![Vec::new(); triangle_count];
    for incidence in edge_incidence.values() {
        for left in 0..incidence.len() {
            for right in (left + 1)..incidence.len() {
                neighbors[incidence[left].0].push(incidence[right].0);
                neighbors[incidence[right].0].push(incidence[left].0);
            }
        }
    }

    let mut seen = HashSet::new();
    let mut components = 0;
    for start in 0..triangle_count {
        if !seen.insert(start) {
            continue;
        }
        components += 1;
        let mut queue = VecDeque::from([start]);
        while let Some(current) = queue.pop_front() {
            for &neighbor in &neighbors[current] {
                if seen.insert(neighbor) {
                    queue.push_back(neighbor);
                }
            }
        }
    }
    components
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tetra_vertices() -> Vec<[f64; 3]> {
        vec![
            [0.0, 0.0, 0.0],
            [10.0, 0.0, 0.0],
            [0.0, 10.0, 0.0],
            [0.0, 0.0, 10.0],
        ]
    }

    fn tetra_triangles() -> Vec<[usize; 3]> {
        vec![[0, 2, 1], [0, 1, 3], [1, 2, 3], [2, 0, 3]]
    }

    fn error_for(vertices: Vec<[f64; 3]>, triangles: Vec<[usize; 3]>) -> String {
        build_mesh_literal("polyhedron", vertices, triangles, true)
            .expect_err("fixture must reject")
            .details
            .expect("validation error should expose details")
    }

    #[test]
    fn closed_tetrahedron_builds_four_triangles() {
        let mesh = build_mesh_literal("polyhedron", tetra_vertices(), tetra_triangles(), true)
            .expect("closed tetrahedron should validate");
        assert_eq!(mesh.polygons.len(), 4);
    }

    #[test]
    fn rejects_out_of_range_index() {
        let error = error_for(tetra_vertices(), vec![[0, 1, 4]]);
        assert!(error.contains("triangle 0"));
        assert!(error.contains("index 4"));
        assert!(error.contains("4 vertices"));
    }

    #[test]
    fn rejects_repeated_triangle_index() {
        let error = error_for(tetra_vertices(), vec![[0, 0, 1]]);
        assert!(error.contains("triangle 0"));
        assert!(error.contains("repeated vertex index"));
    }

    #[test]
    fn rejects_zero_area_triangle() {
        let error = error_for(
            vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [2.0, 0.0, 0.0]],
            vec![[0, 1, 2]],
        );
        assert!(error.contains("triangle 0"));
        assert!(error.contains("zero area"));
    }

    #[test]
    fn rejects_duplicate_triangle_regardless_of_winding() {
        let error = error_for(tetra_vertices(), vec![[0, 1, 2], [2, 1, 0]]);
        assert!(error.contains("triangle 1"));
        assert!(error.contains("duplicates triangle 0"));
    }

    #[test]
    fn polyhedron_reports_boundary_edges() {
        let mut triangles = tetra_triangles();
        triangles.pop();
        let error = error_for(tetra_vertices(), triangles);
        assert!(error.contains("boundary edges: 3"));
    }

    #[test]
    fn rejects_non_manifold_edge() {
        let vertices = vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, -1.0, 0.0],
            [0.0, 0.0, 1.0],
        ];
        let error = error_for(vertices, vec![[0, 1, 2], [1, 0, 3], [0, 1, 4]]);
        assert!(error.contains("non-manifold edges: 1"));
    }

    #[test]
    fn rejects_inconsistent_winding() {
        let mut triangles = tetra_triangles();
        triangles[3].swap(1, 2);
        let error = error_for(tetra_vertices(), triangles);
        assert!(error.contains("inconsistent winding"));
    }

    #[test]
    fn rejects_multiple_closed_components() {
        let mut vertices = tetra_vertices();
        vertices.extend(tetra_vertices().into_iter().map(|mut point| {
            point[0] += 30.0;
            point
        }));
        let mut triangles = tetra_triangles();
        triangles.extend(
            tetra_triangles()
                .into_iter()
                .map(|[a, b, c]| [a + 4, b + 4, c + 4]),
        );
        let error = error_for(vertices, triangles);
        assert!(error.contains("connected components: 2"));
    }

    #[test]
    fn rejects_zero_signed_volume_closed_surface() {
        let vertices = vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
        ];
        let triangles = tetra_triangles();
        let error = error_for(vertices, triangles);
        assert!(error.contains("signed volume is zero"));
    }

    #[test]
    fn open_mesh_accepts_boundary_edges() {
        let mesh = build_mesh_literal(
            "mesh",
            tetra_vertices(),
            tetra_triangles()[..3].to_vec(),
            false,
        )
        .expect("open mesh should remain renderable");
        assert_eq!(mesh.polygons.len(), 3);
    }
}
