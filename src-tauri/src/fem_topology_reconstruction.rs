use ecky_fem::FemDensitySurfaceMesh;

use crate::contracts::{AppError, AppResult};

pub const MAXIMUM_TOPOLOGY_SOLID_VERTICES: usize = 100_000;
pub const MAXIMUM_TOPOLOGY_SOLID_TRIANGLES: usize = 200_000;

pub fn density_surface_solid_expression(surface: &FemDensitySurfaceMesh) -> AppResult<String> {
    if surface.vertices.is_empty() || surface.triangles.is_empty() {
        return Err(AppError::validation(
            "FEM topology solid requires non-empty vertices and triangles.",
        ));
    }
    if surface.vertices.len() > MAXIMUM_TOPOLOGY_SOLID_VERTICES
        || surface.triangles.len() > MAXIMUM_TOPOLOGY_SOLID_TRIANGLES
    {
        return Err(AppError::validation(format!(
            "FEM topology solid exceeds source budget: {} vertices and {} triangles.",
            surface.vertices.len(),
            surface.triangles.len()
        )));
    }
    if surface.boundary_edge_count != 0
        || surface.non_manifold_edge_count != 0
        || surface.connected_component_count != 1
        || !surface.signed_volume_mm3.is_finite()
        || surface.signed_volume_mm3 <= 0.0
    {
        return Err(AppError::validation(
            "FEM topology solid requires one closed positive-volume manifold surface.",
        ));
    }

    let mut source = String::from("(solidify (polyhedron :vertices (");
    for vertex in &surface.vertices {
        if vertex.iter().any(|component| !component.is_finite()) {
            return Err(AppError::validation(
                "FEM topology solid contains a non-finite vertex.",
            ));
        }
        source.push_str(&format!(
            "({} {} {})",
            canonical_number(vertex[0]),
            canonical_number(vertex[1]),
            canonical_number(vertex[2])
        ));
    }
    source.push_str(") :triangles (");
    for triangle in &surface.triangles {
        if triangle
            .iter()
            .any(|index| *index as usize >= surface.vertices.len())
        {
            return Err(AppError::validation(
                "FEM topology solid contains an out-of-range triangle index.",
            ));
        }
        source.push_str(&format!(
            "({} {} {})",
            triangle[0], triangle[1], triangle[2]
        ));
    }
    source.push_str(")))");
    Ok(source)
}

fn canonical_number(value: f64) -> String {
    if value == 0.0 {
        "0".into()
    } else {
        format!("{value:.12}")
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ecky_fem::FemDensitySurfaceMesh;

    fn tetra_surface() -> FemDensitySurfaceMesh {
        FemDensitySurfaceMesh {
            vertices: vec![
                [0.0, 0.0, 0.0],
                [10.0, 0.0, 0.0],
                [0.0, 10.0, 0.0],
                [0.0, 0.0, 10.0],
            ],
            triangles: vec![[0, 2, 1], [0, 1, 3], [1, 2, 3], [2, 0, 3]],
            connected_anchor_ids: vec!["mount".into()],
            discarded_cell_indices: vec![],
            discarded_active_volume_fraction: 0.0,
            boundary_edge_count: 0,
            non_manifold_edge_count: 0,
            connected_component_count: 1,
            signed_volume_mm3: 1000.0 / 6.0,
        }
    }

    #[test]
    fn closed_density_surface_compiles_as_hybrid_solidification() {
        let expression = density_surface_solid_expression(&tetra_surface()).unwrap();
        let source = format!("(model (part optimized {expression}))");
        let program = crate::ecky_scheme::compile_to_core_program(&source).expect("compile");
        assert_eq!(
            crate::ecky_ir::poly_partition::analyze_program(&program)[0].strategy,
            crate::ecky_ir::poly_partition::PartRenderStrategy::Hybrid,
        );
    }

    #[test]
    fn open_surface_cannot_enter_solidification() {
        let mut surface = tetra_surface();
        surface.boundary_edge_count = 3;
        let error = density_surface_solid_expression(&surface).unwrap_err();
        assert!(error.message.contains("closed positive-volume manifold"));
    }
}
