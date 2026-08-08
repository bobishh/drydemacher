use ecky_fem::{
    audit_post_solve_applicability, FemApplicabilityStatus, FemPostSolveApplicabilityInput,
    FEM_SCHEMA_VERSION,
};

#[test]
fn post_solve_audit_blocks_large_displacement_plastic_range_and_unstable_hotspot() {
    let checks = audit_post_solve_applicability(&FemPostSolveApplicabilityInput {
        schema_version: FEM_SCHEMA_VERSION,
        characteristic_size_mm: 100.0,
        maximum_displacement_mm: 8.0,
        maximum_von_mises_mpa: 320.0,
        yield_strength_mpa: 276.0,
        hotspot_movement_mm: 5.0,
        boundary_condition_singularity: true,
    })
    .expect("post-solve audit");

    assert_eq!(checks.len(), 4);
    assert!(checks
        .iter()
        .all(|check| check.status == FemApplicabilityStatus::Blocked));
}
