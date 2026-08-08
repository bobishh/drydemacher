use ecky_fem::{
    solve_linear_static, FemConstraint, FemFaceTarget, FemForceVector, FemLoad, FemMaterial,
    FemMeshingEvidence, FemPoint3, FemRuntimeIdentity, FemVolumeMesh, FemVolumeMeshInput,
    FEM_SCHEMA_VERSION,
};

#[test]
fn cantilever_tet4_refinement_moves_toward_beam_deflection_without_claiming_exactness() {
    let length_mm: f64 = 100.0;
    let width_mm: f64 = 10.0;
    let force_n: f64 = 100.0;
    let young_modulus_mpa: f64 = 200_000.0;
    let material = FemMaterial {
        schema_version: FEM_SCHEMA_VERSION,
        name: "steel-reference".to_string(),
        young_modulus_mpa,
        poisson_ratio: 0.3,
        density_kg_per_mm3: 7.85e-6,
        yield_strength_mpa: 250.0,
    };
    let analytical_mm = force_n * length_mm.powi(3)
        / (3.0 * young_modulus_mpa * (width_mm * width_mm.powi(3) / 12.0));
    let mut deflections = Vec::new();
    for segments in [1_usize, 2, 4, 8] {
        let mesh = cantilever_mesh(length_mm, width_mm, segments);
        let solved = solve_linear_static(
            &mesh,
            &material,
            &[FemLoad::SurfaceForce {
                schema_version: FEM_SCHEMA_VERSION,
                name: "tip-force".to_string(),
                faces: vec![mesh.face_group_targets[1].clone()],
                total_force_n: FemForceVector {
                    x_n: 0.0,
                    y_n: 0.0,
                    z_n: -force_n,
                },
            }],
            &[FemConstraint::Fixed {
                schema_version: FEM_SCHEMA_VERSION,
                name: "root".to_string(),
                faces: vec![mesh.face_group_targets[0].clone()],
            }],
            1.0e-9,
            mesh.nodes.len() * 3,
        )
        .expect("cantilever reference solve");
        let tip_deflection_mm = mesh
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, point)| (point.x_mm - length_mm).abs() <= f64::EPSILON)
            .map(|(node, _)| -solved.displacement_dofs_mm[node * 3 + 2])
            .sum::<f64>()
            / 4.0;
        assert!(tip_deflection_mm.is_finite() && tip_deflection_mm > 0.0);
        assert!(solved.equilibrium.relative_imbalance <= 1.0e-8);
        deflections.push(tip_deflection_mm);
    }

    assert!(
        deflections.windows(2).all(|pair| pair[1] > pair[0]),
        "{deflections:?}"
    );
    let recorded_mm = [
        0.002_569_897_865_870_969_3,
        0.007_209_195_891_076_719,
        0.019_790_097_880_035_586,
        0.038_822_192_071_805_824,
    ];
    for (observed, recorded) in deflections.iter().zip(recorded_mm) {
        assert!((observed - recorded).abs() <= recorded * 1.0e-8);
    }
    assert!(
        (analytical_mm - deflections[3]).abs() < (analytical_mm - deflections[0]).abs(),
        "Tet4 refinement must approach the beam reference: analytical={analytical_mm}, observed={deflections:?}"
    );
    assert!(
        deflections[3] < analytical_mm,
        "one-cell-thick linear Tet4 remains bending-stiff"
    );
}

fn cantilever_mesh(length_mm: f64, width_mm: f64, segments: usize) -> FemVolumeMesh {
    let mut nodes = Vec::with_capacity((segments + 1) * 4);
    for plane in 0..=segments {
        let x = length_mm * plane as f64 / segments as f64;
        nodes.extend([
            FemPoint3::new(x, 0.0, 0.0),
            FemPoint3::new(x, width_mm, 0.0),
            FemPoint3::new(x, 0.0, width_mm),
            FemPoint3::new(x, width_mm, width_mm),
        ]);
    }
    let mut cells = Vec::with_capacity(segments * 6);
    let mut boundary_triangles = vec![[0, 2, 3], [0, 3, 1]];
    let mut boundary_face_group_indices = vec![0, 0];
    for segment in 0..segments {
        let a = (segment * 4) as u32;
        let b = a + 4;
        cells.extend([
            [a, b, b + 1, b + 3],
            [a, b + 1, a + 1, b + 3],
            [a, a + 1, a + 3, b + 3],
            [a, a + 3, a + 2, b + 3],
            [a, a + 2, b + 2, b + 3],
            [a, b + 2, b, b + 3],
        ]);
        let sides = [
            ([a, b + 1, b], [a, a + 1, b + 1], 2),
            ([a + 2, b + 2, b + 3], [a + 2, b + 3, a + 3], 3),
            ([a, b, b + 2], [a, b + 2, a + 2], 4),
            ([a + 1, a + 3, b + 3], [a + 1, b + 3, b + 1], 5),
        ];
        for (first, second, group) in sides {
            boundary_triangles.extend([first, second]);
            boundary_face_group_indices.extend([group, group]);
        }
    }
    let end = (segments * 4) as u32;
    boundary_triangles.extend([[end, end + 1, end + 3], [end, end + 3, end + 2]]);
    boundary_face_group_indices.extend([1, 1]);
    FemVolumeMesh::validate_and_canonicalize(FemVolumeMeshInput {
        schema_version: FEM_SCHEMA_VERSION,
        nodes,
        cells,
        boundary_triangles,
        boundary_face_group_indices,
        face_group_count: 6,
        face_group_targets: (0..6).map(face_target).collect(),
        source_boundary_digest: "sha256:cantilever-boundary".to_string(),
        mesher_identity: FemRuntimeIdentity {
            schema_version: FEM_SCHEMA_VERSION,
            platform: "reference".to_string(),
            architecture: "reference".to_string(),
            library_name: "analytic-fixture".to_string(),
            library_version: "1".to_string(),
            library_digest: "sha256:analytic-mesh".to_string(),
            adapter_protocol_version: 1,
            supported_capabilities: vec!["tet4".to_string()],
            notice_digest: "sha256:notice".to_string(),
        },
        meshing_evidence: FemMeshingEvidence {
            schema_version: FEM_SCHEMA_VERSION,
            source_triangle_count: (segments * 8 + 4) as u64,
            inserted_source_triangle_count: (segments * 8 + 4) as u64,
            tagged_boundary_triangle_count: (segments * 8 + 4) as u64,
            maximum_boundary_deviation_mm: 0.0,
            deterministic_thread_count: 1,
        },
        minimum_scaled_jacobian: 1.0e-8,
    })
    .expect("valid cantilever mesh")
}

fn face_target(index: usize) -> FemFaceTarget {
    FemFaceTarget {
        schema_version: FEM_SCHEMA_VERSION,
        part_id: "cantilever".to_string(),
        canonical_target_id: format!("cantilever:face:{index}"),
        durable_target_id: format!("cantilever:stable:{index}"),
        source_geometry_digest: "sha256:cantilever-geometry".to_string(),
    }
}
