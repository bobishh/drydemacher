use ecky_fem::{
    audit_pre_solve_applicability, FemApplicabilityStatus, FemPreSolveApplicabilityInput,
    FEM_SCHEMA_VERSION,
};

#[test]
fn pre_solve_applicability_accepts_supported_scope_and_blocks_invalid_physics() {
    let supported = audit_pre_solve_applicability(&FemPreSolveApplicabilityInput {
        schema_version: FEM_SCHEMA_VERSION,
        solid_count: 1,
        unsupported_interface_count: 0,
        characteristic_size_mm: 100.0,
        minimum_thickness_mm: 10.0,
        poisson_ratio: 0.33,
        constrained_translation_components: 3,
        selected_load_area_mm2: 100.0,
        selected_support_area_mm2: 100.0,
        has_point_load_or_support: false,
    })
    .expect("supported applicability");
    assert!(supported
        .iter()
        .all(|check| check.status == FemApplicabilityStatus::Pass));

    let blocked = audit_pre_solve_applicability(&FemPreSolveApplicabilityInput {
        schema_version: FEM_SCHEMA_VERSION,
        solid_count: 2,
        unsupported_interface_count: 1,
        characteristic_size_mm: 100.0,
        minimum_thickness_mm: 1.0,
        poisson_ratio: 0.49,
        constrained_translation_components: 2,
        selected_load_area_mm2: 0.0,
        selected_support_area_mm2: 0.0,
        has_point_load_or_support: true,
    })
    .expect("deterministic blocked audit");
    assert!(blocked
        .iter()
        .all(|check| check.status == FemApplicabilityStatus::Blocked));
    assert!(blocked
        .iter()
        .any(|check| check.check_id == "tet4-slenderness"));
    assert!(blocked.iter().any(|check| check.check_id == "locking"));
    assert!(blocked.iter().any(|check| check.check_id == "singularity"));
}
