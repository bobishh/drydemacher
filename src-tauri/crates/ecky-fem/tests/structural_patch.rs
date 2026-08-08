use ecky_fem::{
    ElementAssembler, FaerSparseCholeskySolver, FemDirichletConstraint, FemIndexedTet4Mesh,
    FemMaterial, FemPoint3, LinearSolver, FEM_SCHEMA_VERSION,
};

#[test]
fn one_tet_fixed_face_load_solves_with_finite_stress_and_reaction_equilibrium() {
    let assembler = ElementAssembler;
    let mesh = FemIndexedTet4Mesh {
        schema_version: FEM_SCHEMA_VERSION,
        nodes: vec![
            FemPoint3::new(0.0, 0.0, 0.0),
            FemPoint3::new(1.0, 0.0, 0.0),
            FemPoint3::new(0.0, 1.0, 0.0),
            FemPoint3::new(0.0, 0.0, 1.0),
        ],
        cells: vec![[0, 1, 2, 3]],
    };
    let material = FemMaterial {
        schema_version: FEM_SCHEMA_VERSION,
        name: "linear-elastic".to_string(),
        young_modulus_mpa: 1_000.0,
        poisson_ratio: 0.25,
        density_kg_per_mm3: 1.0e-6,
        yield_strength_mpa: 100.0,
    };
    let stiffness = assembler
        .assemble_global_stiffness(&mesh, &material)
        .expect("global stiffness");
    let mut rhs = vec![0.0; 12];
    rhs[11] = -10.0;
    let constraints = (0..9)
        .map(|dof_index| FemDirichletConstraint {
            dof_index,
            value_mm: 0.0,
        })
        .collect::<Vec<_>>();
    let reduced = stiffness
        .eliminate_dirichlet(&rhs, &constraints)
        .expect("fixed-face reduction");
    let solved = FaerSparseCholeskySolver
        .solve(&reduced.matrix, &reduced.rhs, 1.0e-10, 12)
        .expect("reduced solve");
    let full = reduced
        .recover_full_solution(&solved.solution)
        .expect("full displacement");
    let reactions = reduced
        .recover_support_reactions(&solved.solution)
        .expect("support reactions");

    assert!(full.iter().all(|value| value.is_finite()));
    assert!(full[11] < 0.0);
    let reaction_z = reactions
        .iter()
        .filter(|reaction| reaction.dof_index % 3 == 2)
        .map(|reaction| reaction.reaction_n)
        .sum::<f64>();
    assert!((reaction_z + rhs[11]).abs() <= 1.0e-9);
    let strain_energy = 0.5
        * rhs
            .iter()
            .zip(&full)
            .map(|(force, displacement)| force * displacement)
            .sum::<f64>();
    assert!(strain_energy.is_finite() && strain_energy > 0.0);

    let element = ecky_fem::Tet4Element::new(mesh.nodes.clone().try_into().unwrap());
    let displacements = [
        FemPoint3::new(full[0], full[1], full[2]),
        FemPoint3::new(full[3], full[4], full[5]),
        FemPoint3::new(full[6], full[7], full[8]),
        FemPoint3::new(full[9], full[10], full[11]),
    ];
    let stress = assembler
        .stress_from_displacements(&element, &displacements, &material)
        .expect("element stress");
    assert!(stress.iter().all(|value| value.is_finite()));
}

#[test]
fn assembly_observer_can_cancel_before_first_sparse_chunk() {
    let mesh = FemIndexedTet4Mesh {
        schema_version: FEM_SCHEMA_VERSION,
        nodes: vec![
            FemPoint3::new(0.0, 0.0, 0.0),
            FemPoint3::new(1.0, 0.0, 0.0),
            FemPoint3::new(0.0, 1.0, 0.0),
            FemPoint3::new(0.0, 0.0, 1.0),
        ],
        cells: vec![[0, 1, 2, 3]],
    };
    let error = ElementAssembler
        .assemble_global_stiffness_with_observer(&mesh, &material(), |_| {
            Err(ecky_fem::FemValidationError {
                field: "cancelled".into(),
                message: "assembly cancelled".into(),
            })
        })
        .expect_err("assembly cancellation");
    assert_eq!(error.field, "cancelled");
}

fn material() -> FemMaterial {
    FemMaterial {
        schema_version: FEM_SCHEMA_VERSION,
        name: "linear-elastic".to_string(),
        young_modulus_mpa: 1_000.0,
        poisson_ratio: 0.25,
        density_kg_per_mm3: 1.0e-6,
        yield_strength_mpa: 100.0,
    }
}
