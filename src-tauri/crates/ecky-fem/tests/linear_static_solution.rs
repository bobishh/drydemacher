use ecky_fem::{
    solve_linear_static, solve_linear_static_with_solver_and_observer, FemConstraint,
    FemFaceTarget, FemForceVector, FemLinearSolveResult, FemLoad, FemMaterial, FemMeshingEvidence,
    FemOptionalDisplacement, FemPoint3, FemRuntimeIdentity, FemSolveStage, FemSparseMatrix,
    FemValidationError, FemVolumeMesh, FemVolumeMeshInput, LinearSolver, FEM_SCHEMA_VERSION,
};

#[test]
fn tagged_one_tet_study_passes_solver_equilibrium_energy_and_result_gates() {
    let mesh = one_tet_mesh();
    let solved = solve_linear_static(
        &mesh,
        &material(),
        &[FemLoad::SurfaceForce {
            schema_version: FEM_SCHEMA_VERSION,
            name: "load".to_string(),
            faces: vec![mesh.face_group_targets[0].clone()],
            total_force_n: FemForceVector {
                x_n: 0.0,
                y_n: 0.0,
                z_n: -12.0,
            },
        }],
        &[FemConstraint::Fixed {
            schema_version: FEM_SCHEMA_VERSION,
            name: "mount".to_string(),
            faces: vec![mesh.face_group_targets[3].clone()],
        }],
        1.0e-10,
        12,
    )
    .expect("accepted solve");

    assert_eq!(solved.linear_solve.solver_identity.backend, "faer");
    assert!(solved.strain_energy_n_mm.is_finite() && solved.strain_energy_n_mm >= 0.0);
    assert!(solved.equilibrium.relative_imbalance <= 1.0e-10);
    assert_eq!(solved.support_reactions.len(), 1);
    assert_eq!(solved.postprocess.elements.len(), 1);
    assert!(solved.solution_digest.starts_with("sha256:"));
}

#[test]
fn underconstrained_study_fails_before_factorization_with_rigid_mode_count() {
    let mesh = one_tet_mesh();
    let error = solve_linear_static(
        &mesh,
        &material(),
        &[FemLoad::SurfaceForce {
            schema_version: FEM_SCHEMA_VERSION,
            name: "load".to_string(),
            faces: vec![mesh.face_group_targets[0].clone()],
            total_force_n: FemForceVector {
                x_n: 0.0,
                y_n: 0.0,
                z_n: -1.0,
            },
        }],
        &[FemConstraint::PrescribedDisplacement {
            schema_version: FEM_SCHEMA_VERSION,
            name: "weak-support".to_string(),
            faces: vec![mesh.face_group_targets[3].clone()],
            displacement_mm: FemOptionalDisplacement {
                x_mm: Some(0.0),
                y_mm: None,
                z_mm: None,
            },
        }],
        1.0e-10,
        12,
    )
    .expect_err("rigid modes must fail");
    assert!(error.message.contains("rigid-body modes"));
    assert!(error.message.contains("unconstrained DOF"));
}

#[derive(Debug)]
struct HiddenMechanismSolver;

impl LinearSolver for HiddenMechanismSolver {
    fn solve(
        &self,
        _matrix: &FemSparseMatrix,
        _rhs: &[f64],
        _relative_tolerance: f64,
        _maximum_dimension: usize,
    ) -> Result<FemLinearSolveResult, FemValidationError> {
        Err(FemValidationError {
            field: "matrix.factorization".to_string(),
            message: "zero pivot at reduced DOF 7; likely hidden mechanism or unconstrained DOF"
                .to_string(),
        })
    }
}

#[test]
fn hidden_mechanism_preserves_factorization_diagnostic_without_adding_springs() {
    let mesh = one_tet_mesh();
    let mut observed = Vec::new();
    let error = solve_linear_static_with_solver_and_observer(
        &mesh,
        &material(),
        &[FemLoad::SurfaceForce {
            schema_version: FEM_SCHEMA_VERSION,
            name: "load".to_string(),
            faces: vec![mesh.face_group_targets[0].clone()],
            total_force_n: FemForceVector {
                x_n: 0.0,
                y_n: 0.0,
                z_n: -1.0,
            },
        }],
        &[FemConstraint::Fixed {
            schema_version: FEM_SCHEMA_VERSION,
            name: "mount".to_string(),
            faces: vec![mesh.face_group_targets[3].clone()],
        }],
        1.0e-10,
        12,
        &HiddenMechanismSolver,
        |stage| {
            observed.push(stage);
            Ok(())
        },
    )
    .expect_err("hidden mechanism must fail, never receive numerical springs");

    assert!(observed.contains(&FemSolveStage::Solve));
    assert_eq!(error.field, "matrix.factorization");
    assert_eq!(
        error.message,
        "zero pivot at reduced DOF 7; likely hidden mechanism or unconstrained DOF"
    );
}

fn one_tet_mesh() -> FemVolumeMesh {
    FemVolumeMesh::validate_and_canonicalize(FemVolumeMeshInput {
        schema_version: FEM_SCHEMA_VERSION,
        nodes: vec![
            FemPoint3::new(0.0, 0.0, 0.0),
            FemPoint3::new(1.0, 0.0, 0.0),
            FemPoint3::new(0.0, 1.0, 0.0),
            FemPoint3::new(0.0, 0.0, 1.0),
        ],
        cells: vec![[0, 1, 2, 3]],
        boundary_triangles: vec![[1, 3, 2], [0, 2, 3], [0, 3, 1], [0, 1, 2]],
        boundary_face_group_indices: vec![0, 1, 2, 3],
        face_group_count: 4,
        face_group_targets: (0..4)
            .map(|index| FemFaceTarget {
                schema_version: FEM_SCHEMA_VERSION,
                part_id: "body".to_string(),
                canonical_target_id: format!("body:face:{index}"),
                durable_target_id: format!("body:stable:{index}"),
                source_geometry_digest: "sha256:geometry".to_string(),
            })
            .collect(),
        source_boundary_digest: "sha256:boundary".to_string(),
        mesher_identity: FemRuntimeIdentity {
            schema_version: FEM_SCHEMA_VERSION,
            platform: "test".to_string(),
            architecture: "test".to_string(),
            library_name: "fTetWild".to_string(),
            library_version: "pinned".to_string(),
            library_digest: "sha256:binary".to_string(),
            adapter_protocol_version: 1,
            supported_capabilities: vec!["tet4".to_string()],
            notice_digest: "sha256:notice".to_string(),
        },
        meshing_evidence: FemMeshingEvidence {
            schema_version: FEM_SCHEMA_VERSION,
            source_triangle_count: 4,
            inserted_source_triangle_count: 4,
            tagged_boundary_triangle_count: 4,
            maximum_boundary_deviation_mm: 0.0,
            deterministic_thread_count: 1,
        },
        minimum_scaled_jacobian: 1.0e-6,
    })
    .expect("mesh")
}

fn material() -> FemMaterial {
    FemMaterial {
        schema_version: FEM_SCHEMA_VERSION,
        name: "test".to_string(),
        young_modulus_mpa: 1_000.0,
        poisson_ratio: 0.25,
        density_kg_per_mm3: 1.0e-6,
        yield_strength_mpa: 100.0,
    }
}
