use ecky_fem::{
    ElementAssembler, FemMaterial, FemPoint3, Tet4Element, Tet4Orientation, FEM_SCHEMA_VERSION,
};

fn point(x: f64, y: f64, z: f64) -> FemPoint3 {
    FemPoint3::new(x, y, z)
}

fn unit_tet() -> Tet4Element {
    Tet4Element::new([
        point(0.0, 0.0, 0.0),
        point(1.0, 0.0, 0.0),
        point(0.0, 1.0, 0.0),
        point(0.0, 0.0, 1.0),
    ])
}

fn shared_face_tet() -> Tet4Element {
    Tet4Element::new([
        point(1.0, 0.0, 0.0),
        point(0.0, 1.0, 0.0),
        point(0.0, 0.0, 1.0),
        point(1.0, 1.0, 1.0),
    ])
}

fn material() -> FemMaterial {
    FemMaterial {
        schema_version: FEM_SCHEMA_VERSION,
        name: "steel".to_string(),
        young_modulus_mpa: 210_000.0,
        poisson_ratio: 0.3,
        density_kg_per_mm3: 0.000_007_85,
        yield_strength_mpa: 355.0,
    }
}

fn affine_field(p: FemPoint3) -> FemPoint3 {
    FemPoint3::new(
        2.0 + 0.2 * p.x_mm - 0.1 * p.y_mm + 0.4 * p.z_mm,
        -1.5 + 0.5 * p.x_mm + 0.7 * p.y_mm + 0.2 * p.z_mm,
        0.25 - 0.6 * p.x_mm + 0.15 * p.y_mm + 0.25 * p.z_mm,
    )
}

fn rigid_translation_field(_: FemPoint3) -> FemPoint3 {
    FemPoint3::new(3.0, -2.0, 0.5)
}

fn rigid_rotation_field(p: FemPoint3) -> FemPoint3 {
    let omega_x = 0.4;
    let omega_y = -0.3;
    let omega_z = 0.2;
    FemPoint3::new(
        omega_y * p.z_mm - omega_z * p.y_mm,
        omega_z * p.x_mm - omega_x * p.z_mm,
        omega_x * p.y_mm - omega_y * p.x_mm,
    )
}

fn node_displacements(element: &Tet4Element, field: fn(FemPoint3) -> FemPoint3) -> [FemPoint3; 4] {
    element.nodes.map(field)
}

fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= 1e-9,
        "expected {expected}, got {actual}"
    );
}

fn assert_vec_close(actual: [f64; 6], expected: [f64; 6]) {
    for (index, (actual, expected)) in actual.into_iter().zip(expected).enumerate() {
        assert!(
            (actual - expected).abs() <= 1e-9,
            "component {index}: expected {expected}, got {actual}"
        );
    }
}

fn assert_matrix_symmetric(matrix: &[[f64; 12]; 12]) {
    for (row, values) in matrix.iter().enumerate() {
        for (col, value) in values.iter().enumerate() {
            assert!(
                (*value - matrix[col][row]).abs() <= 1e-9,
                "matrix asymmetry at ({row}, {col})"
            );
        }
    }
}

fn flatten_dofs(displacements: [FemPoint3; 4]) -> [f64; 12] {
    [
        displacements[0].x_mm,
        displacements[0].y_mm,
        displacements[0].z_mm,
        displacements[1].x_mm,
        displacements[1].y_mm,
        displacements[1].z_mm,
        displacements[2].x_mm,
        displacements[2].y_mm,
        displacements[2].z_mm,
        displacements[3].x_mm,
        displacements[3].y_mm,
        displacements[3].z_mm,
    ]
}

fn mat_vec_product(matrix: &[[f64; 12]; 12], vector: &[f64; 12]) -> [f64; 12] {
    let mut result = [0.0; 12];
    for row in 0..12 {
        for col in 0..12 {
            result[row] += matrix[row][col] * vector[col];
        }
    }
    result
}

#[test]
fn tet4_signed_volume_orientation_gradients_and_affine_reproduction_are_exact() {
    let assembler = ElementAssembler;
    let tet = unit_tet();

    assert_close(
        assembler.signed_volume_mm3(&tet).expect("signed volume"),
        1.0 / 6.0,
    );
    assert_eq!(
        assembler.orientation(&tet).expect("orientation"),
        Tet4Orientation::Positive
    );

    let reversed = Tet4Element::new([tet.nodes[0], tet.nodes[2], tet.nodes[1], tet.nodes[3]]);
    assert!(
        assembler
            .signed_volume_mm3(&reversed)
            .expect("signed volume")
            < 0.0
    );
    assert_eq!(
        assembler.orientation(&reversed).expect("orientation"),
        Tet4Orientation::Negative
    );

    assert_eq!(
        assembler.reference_shape_gradients(),
        [
            [-1.0, -1.0, -1.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0]
        ]
    );
    assert_eq!(
        assembler
            .world_shape_gradients(&tet)
            .expect("world gradients"),
        [
            [-1.0, -1.0, -1.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0]
        ]
    );

    let local = [0.2, 0.3, 0.1];
    let shape = assembler
        .reference_shape_functions(local)
        .expect("reference shape functions");
    assert_close(shape.into_iter().sum::<f64>(), 1.0);

    let nodal_values = tet
        .nodes
        .map(|node| 2.0 - 3.0 * node.x_mm + 5.0 * node.y_mm + 7.0 * node.z_mm);
    let interpolated = shape
        .into_iter()
        .zip(nodal_values)
        .map(|(shape, value)| shape * value)
        .sum::<f64>();
    let exact = 2.0 - 3.0 * local[0] + 5.0 * local[1] + 7.0 * local[2];
    assert_close(interpolated, exact);
}

#[test]
fn tet4_isotropic_constitutive_tensor_is_symmetric_positive_and_has_uniaxial_response() {
    let assembler = ElementAssembler;
    let material = material();
    let d = assembler
        .constitutive_matrix(&material)
        .expect("constitutive matrix");

    for (row, values) in d.iter().enumerate() {
        for (col, value) in values.iter().enumerate() {
            assert!(
                (*value - d[col][row]).abs() <= 1e-12,
                "constitutive matrix asymmetry at ({row}, {col})"
            );
        }
    }

    let strain = [0.002, -0.001, 0.003, 0.004, -0.002, 0.005];
    let mut stress = [0.0; 6];
    for (row, stress_value) in stress.iter_mut().enumerate() {
        for (col, strain_value) in strain.iter().enumerate() {
            *stress_value += d[row][col] * strain_value;
        }
    }
    let energy = 0.5
        * strain
            .iter()
            .zip(stress)
            .map(|(epsilon, sigma)| epsilon * sigma)
            .sum::<f64>();
    assert!(energy > 0.0, "strain energy should be positive");

    let e = 0.001;
    let poisson = material.poisson_ratio;
    let young = material.young_modulus_mpa;
    let lambda = young * poisson / ((1.0 + poisson) * (1.0 - 2.0 * poisson));
    let mu = young / (2.0 * (1.0 + poisson));
    let expected = [
        (lambda + 2.0 * mu) * e,
        lambda * e,
        lambda * e,
        0.0,
        0.0,
        0.0,
    ];
    let actual = assembler
        .stress_from_strain(&material, [e, 0.0, 0.0, 0.0, 0.0, 0.0])
        .expect("uniaxial response");
    assert_vec_close(actual, expected);
}

#[test]
fn tet4_constant_strain_patch_and_rigid_body_modes_are_zero_strain() {
    let assembler = ElementAssembler;
    let material = material();
    let elements = [unit_tet(), shared_face_tet()];

    let expected_strain = [0.2, 0.7, 0.25, 0.35, -0.2, 0.4];
    let expected_stress = assembler
        .stress_from_strain(&material, expected_strain)
        .expect("expected stress");

    for element in &elements {
        let nodal_displacements = node_displacements(element, affine_field);
        let strain = assembler
            .strain_from_displacements(element, &nodal_displacements)
            .expect("affine strain");
        assert_vec_close(strain, expected_strain);

        let stress = assembler
            .stress_from_displacements(element, &nodal_displacements, &material)
            .expect("affine stress");
        assert_vec_close(stress, expected_stress);

        let translation = node_displacements(element, rigid_translation_field);
        assert_vec_close(
            assembler
                .strain_from_displacements(element, &translation)
                .expect("translation strain"),
            [0.0; 6],
        );

        let rotation = node_displacements(element, rigid_rotation_field);
        assert_vec_close(
            assembler
                .strain_from_displacements(element, &rotation)
                .expect("rotation strain"),
            [0.0; 6],
        );
    }
}

#[test]
fn tet4_element_stiffness_is_symmetric_and_annihilates_rigid_body_modes() {
    let assembler = ElementAssembler;
    let material = material();
    let tet = unit_tet();
    let stiffness = assembler
        .stiffness_matrix(&tet, &material)
        .expect("stiffness matrix");

    assert_matrix_symmetric(&stiffness);

    let translation = flatten_dofs(node_displacements(&tet, rigid_translation_field));
    let translation_result = mat_vec_product(&stiffness, &translation);
    assert!(translation_result.iter().all(|value| value.abs() <= 1e-9));

    let rotation = flatten_dofs(node_displacements(&tet, rigid_rotation_field));
    let rotation_result = mat_vec_product(&stiffness, &rotation);
    assert!(rotation_result.iter().all(|value| value.abs() <= 1e-9));
}
