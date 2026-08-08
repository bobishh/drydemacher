use ecky_fem::{
    ElementAssembler, FemForceVector, FemPoint3, FemSurfaceTriangle, FEM_SCHEMA_VERSION,
};

fn point(x_mm: f64, y_mm: f64, z_mm: f64) -> FemPoint3 {
    FemPoint3::new(x_mm, y_mm, z_mm)
}

fn sum_forces(forces: impl IntoIterator<Item = FemForceVector>) -> FemForceVector {
    forces.into_iter().fold(
        FemForceVector {
            x_n: 0.0,
            y_n: 0.0,
            z_n: 0.0,
        },
        |sum, force| FemForceVector {
            x_n: sum.x_n + force.x_n,
            y_n: sum.y_n + force.y_n,
            z_n: sum.z_n + force.z_n,
        },
    )
}

fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= 1.0e-12,
        "{actual} != {expected}"
    );
}

fn resultant_moment(
    triangles: &[FemSurfaceTriangle],
    nodal_forces: &[[FemForceVector; 3]],
) -> [f64; 3] {
    let mut moment = [0.0; 3];
    for (triangle, forces) in triangles.iter().zip(nodal_forces) {
        for (point, force) in triangle.nodes.iter().zip(forces) {
            moment[0] += point.y_mm * force.z_n - point.z_mm * force.y_n;
            moment[1] += point.z_mm * force.x_n - point.x_mm * force.z_n;
            moment[2] += point.x_mm * force.y_n - point.y_mm * force.x_n;
        }
    }
    moment
}

#[test]
fn triangle_traction_and_inward_pressure_integrate_exact_resultants() {
    let assembler = ElementAssembler;
    let triangle = FemSurfaceTriangle {
        schema_version: FEM_SCHEMA_VERSION,
        nodes: [
            point(0.0, 0.0, 0.0),
            point(1.0, 0.0, 0.0),
            point(0.0, 1.0, 0.0),
        ],
    };

    let traction = assembler
        .integrate_triangle_traction(&triangle, [6.0, 0.0, 0.0])
        .expect("traction load");
    let traction_total = sum_forces(traction);
    assert_close(traction_total.x_n, 3.0);
    assert_close(traction_total.y_n, 0.0);
    assert_close(traction_total.z_n, 0.0);

    let pressure = assembler
        .integrate_triangle_pressure(&triangle, 2.0)
        .expect("inward pressure");
    let pressure_total = sum_forces(pressure);
    assert_close(pressure_total.x_n, 0.0);
    assert_close(pressure_total.y_n, 0.0);
    assert_close(pressure_total.z_n, -1.0);
}

#[test]
fn total_surface_force_is_exact_across_unequal_triangle_areas() {
    let assembler = ElementAssembler;
    let triangles = vec![
        FemSurfaceTriangle {
            schema_version: FEM_SCHEMA_VERSION,
            nodes: [
                point(0.0, 0.0, 0.0),
                point(1.0, 0.0, 0.0),
                point(0.0, 1.0, 0.0),
            ],
        },
        FemSurfaceTriangle {
            schema_version: FEM_SCHEMA_VERSION,
            nodes: [
                point(0.0, 0.0, 0.0),
                point(2.0, 0.0, 0.0),
                point(0.0, 2.0, 0.0),
            ],
        },
    ];
    let authored = FemForceVector {
        x_n: 7.0,
        y_n: -11.0,
        z_n: 13.0,
    };

    let nodal = assembler
        .distribute_total_surface_force(&triangles, authored)
        .expect("total surface force");
    let total = sum_forces(nodal.into_iter().flatten());
    assert_close(total.x_n, authored.x_n);
    assert_close(total.y_n, authored.y_n);
    assert_close(total.z_n, authored.z_n);
}

#[test]
fn total_surface_force_resultant_and_moment_do_not_depend_on_square_triangulation() {
    let assembler = ElementAssembler;
    let triangulations = [
        vec![
            FemSurfaceTriangle {
                schema_version: FEM_SCHEMA_VERSION,
                nodes: [
                    point(0.0, 0.0, 0.0),
                    point(1.0, 0.0, 0.0),
                    point(1.0, 1.0, 0.0),
                ],
            },
            FemSurfaceTriangle {
                schema_version: FEM_SCHEMA_VERSION,
                nodes: [
                    point(0.0, 0.0, 0.0),
                    point(1.0, 1.0, 0.0),
                    point(0.0, 1.0, 0.0),
                ],
            },
        ],
        vec![
            FemSurfaceTriangle {
                schema_version: FEM_SCHEMA_VERSION,
                nodes: [
                    point(0.0, 0.0, 0.0),
                    point(1.0, 0.0, 0.0),
                    point(0.0, 1.0, 0.0),
                ],
            },
            FemSurfaceTriangle {
                schema_version: FEM_SCHEMA_VERSION,
                nodes: [
                    point(1.0, 0.0, 0.0),
                    point(1.0, 1.0, 0.0),
                    point(0.0, 1.0, 0.0),
                ],
            },
        ],
    ];
    let authored = FemForceVector {
        x_n: 0.0,
        y_n: 0.0,
        z_n: -12.0,
    };

    for triangles in triangulations {
        let nodal = assembler
            .distribute_total_surface_force(&triangles, authored)
            .expect("distributed square force");
        let total = sum_forces(nodal.iter().flatten().copied());
        assert_close(total.z_n, -12.0);
        let moment = resultant_moment(&triangles, &nodal);
        assert_close(moment[0], -6.0);
        assert_close(moment[1], 6.0);
        assert_close(moment[2], 0.0);
    }
}
