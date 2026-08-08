use std::collections::BTreeMap;

use ecky_fem::{
    audit_post_solve_applicability, audit_pre_solve_applicability, run_bounded_sensitivity,
    CanonicalDigest, FemAcceptanceComparison, FemAcceptanceCriterion, FemAdmissionEstimate,
    FemAnalysisIdentity, FemApplicabilityCheck, FemApplicabilityCheckKind, FemApplicabilityStatus,
    FemBudgetLimits, FemConstraint, FemElementKind, FemEngineeringEvidenceLedger,
    FemEngineeringQuestion, FemEvidenceAuthority, FemEvidenceRecord, FemEvidenceSubject,
    FemFaceTarget, FemForceVector, FemIdealizationArtifact, FemIdealizationKind,
    FemIdealizationRecord, FemInputEvidenceBinding, FemLoad, FemMaterial, FemMeshControl,
    FemOptionalDisplacement, FemPostSolveApplicabilityInput, FemPreSolveApplicabilityInput,
    FemRuntimeIdentity, FemSensitivityCaseResult, FemSensitivityEvidence, FemSensitivityInputRange,
    FemSensitivityMetricRange, FemStudyAssumption, FemStudyAssumptionCategory,
    FemStudyAssumptionStatus, FemValidationEvidence, FemValidationEvidenceKind, FEM_SCHEMA_VERSION,
};

#[test]
fn defeatured_idealization_is_a_digest_bound_artifact_that_cannot_replace_manufacturing_geometry() {
    let record = FemIdealizationRecord {
        source_geometry_digest: "sha256:manufacturing-brep".into(),
        analysis_geometry_digest: "sha256:defeatured-analysis-brep".into(),
        affected_topology_ids: vec!["bracket:face:small-hole".into()],
        justification: "Remove sub-mesh hole below declared influence threshold.".into(),
        expected_influence_percent: 1.5,
        accepted_by_user: true,
    };

    let artifact = FemIdealizationArtifact::from_record(&record).expect("defeatured artifact");

    assert_eq!(artifact.kind, FemIdealizationKind::DefeaturedSolid);
    assert_eq!(
        artifact.manufacturing_geometry_digest,
        "sha256:manufacturing-brep"
    );
    assert_ne!(
        artifact.manufacturing_geometry_digest,
        artifact.analysis_geometry_digest
    );
    assert_eq!(artifact.affected_topology_ids, ["bracket:face:small-hole"]);
    assert_eq!(artifact.expected_influence_percent, 1.5);
    assert!(artifact.accepted_by_user);
    assert!(artifact.justification.contains("influence threshold"));
    assert!(artifact.canonical_digest().starts_with("sha256:"));

    let mut silently_replaced = artifact.clone();
    silently_replaced.manufacturing_geometry_digest =
        silently_replaced.analysis_geometry_digest.clone();
    assert!(silently_replaced.validate().is_err());

    let mut missing_topology = artifact.clone();
    missing_topology.affected_topology_ids.clear();
    assert!(missing_topology.validate().is_err());
    let mut missing_threshold = artifact.clone();
    missing_threshold.expected_influence_percent = 0.0;
    assert!(missing_threshold.validate().is_err());
    let mut unapproved = artifact;
    unapproved.accepted_by_user = false;
    assert!(unapproved.validate().is_err());
}

fn face_target(id: &str) -> FemFaceTarget {
    FemFaceTarget {
        schema_version: FEM_SCHEMA_VERSION,
        part_id: "bracket".to_string(),
        canonical_target_id: format!("bracket:face:{id}"),
        durable_target_id: format!("bracket:stable-node-key:{id}"),
        source_geometry_digest: "sha256:geometry".to_string(),
    }
}

fn material() -> FemMaterial {
    FemMaterial {
        schema_version: FEM_SCHEMA_VERSION,
        name: "aluminum-6061".to_string(),
        young_modulus_mpa: 68_900.0,
        poisson_ratio: 0.33,
        density_kg_per_mm3: 0.000_002_7,
        yield_strength_mpa: 276.0,
    }
}

fn load_faces() -> Vec<FemFaceTarget> {
    vec![face_target("a"), face_target("b")]
}

fn constraint_faces() -> Vec<FemFaceTarget> {
    vec![face_target("c"), face_target("d")]
}

fn mesh_control() -> FemMeshControl {
    FemMeshControl {
        schema_version: FEM_SCHEMA_VERSION,
        element_kind: FemElementKind::Tet4,
        global_size_mm: 2.0,
        local_refinements: vec![
            ecky_fem::FemLocalRefinement {
                schema_version: FEM_SCHEMA_VERSION,
                faces: vec![face_target("b"), face_target("a")],
                size_mm: 1.0,
            },
            ecky_fem::FemLocalRefinement {
                schema_version: FEM_SCHEMA_VERSION,
                faces: vec![face_target("d"), face_target("c")],
                size_mm: 0.5,
            },
        ],
        budgets: FemBudgetLimits {
            schema_version: FEM_SCHEMA_VERSION,
            boundary_triangles: 200,
            tet4_cells: 100,
            nodes: 150,
            dofs: 450,
            sparse_nonzeros: 3_000,
            result_bytes: 100_000,
            convergence_levels: 3,
        },
    }
}

fn runtime_identity() -> FemRuntimeIdentity {
    FemRuntimeIdentity {
        schema_version: FEM_SCHEMA_VERSION,
        platform: "macos".to_string(),
        architecture: "aarch64".to_string(),
        library_name: "gmsh".to_string(),
        library_version: "4.12.2".to_string(),
        library_digest: "sha256:gmsh".to_string(),
        adapter_protocol_version: 1,
        supported_capabilities: vec!["solve".to_string(), "mesh".to_string()],
        notice_digest: "sha256:notices".to_string(),
    }
}

fn engineering_evidence(load_authority: FemEvidenceAuthority) -> FemEngineeringEvidenceLedger {
    FemEngineeringEvidenceLedger {
        schema_version: FEM_SCHEMA_VERSION,
        question: FemEngineeringQuestion {
            question_id: "bracket-strength".to_string(),
            statement: "Does the bracket remain below 138 MPa under service load?".to_string(),
            decision: "accept or revise bracket thickness".to_string(),
            acceptance_metric_ids: vec!["bracket-stress".to_string()],
        },
        acceptance_criteria: vec![FemAcceptanceCriterion {
            metric_id: "bracket-stress".to_string(),
            field: "vonMisesStress".to_string(),
            comparison: FemAcceptanceComparison::LessThanOrEqual,
            limit: 138.0,
            unit: "MPa".to_string(),
            requires_convergence: true,
        }],
        idealization: FemIdealizationRecord {
            source_geometry_digest: "sha256:geometry".to_string(),
            analysis_geometry_digest: "sha256:geometry".to_string(),
            affected_topology_ids: vec![],
            justification: "Exact connected solid; no defeaturing.".to_string(),
            expected_influence_percent: 0.0,
            accepted_by_user: true,
        },
        evidence: vec![
            FemEvidenceRecord {
                evidence_id: "material-6061".to_string(),
                subject: FemEvidenceSubject::Material,
                label: "6061-T6 datasheet".to_string(),
                source: "qualified material datasheet".to_string(),
                authority: FemEvidenceAuthority::RecordedSource,
                uncertainty_percent: Some(3.0),
                decision_critical: true,
            },
            FemEvidenceRecord {
                evidence_id: "load-top".to_string(),
                subject: FemEvidenceSubject::Load,
                label: "service load".to_string(),
                source: "user load case".to_string(),
                authority: load_authority,
                uncertainty_percent: Some(10.0),
                decision_critical: true,
            },
            FemEvidenceRecord {
                evidence_id: "support-mount".to_string(),
                subject: FemEvidenceSubject::Support,
                label: "mounting face restraint".to_string(),
                source: "user fixture definition".to_string(),
                authority: FemEvidenceAuthority::UserAccepted,
                uncertainty_percent: None,
                decision_critical: true,
            },
        ],
        input_bindings: vec![
            FemInputEvidenceBinding {
                input_name: "aluminum-6061".to_string(),
                evidence_id: "material-6061".to_string(),
            },
            FemInputEvidenceBinding {
                input_name: "top-load".to_string(),
                evidence_id: "load-top".to_string(),
            },
            FemInputEvidenceBinding {
                input_name: "mount".to_string(),
                evidence_id: "support-mount".to_string(),
            },
        ],
        assumptions: vec![FemStudyAssumption {
            assumption_id: "small-strain".to_string(),
            category: FemStudyAssumptionCategory::Physics,
            statement: "Displacement remains small relative to bracket span.".to_string(),
            status: FemStudyAssumptionStatus::Accepted,
            evidence_ids: vec!["load-top".to_string()],
        }],
        applicability_checks: vec![FemApplicabilityCheck {
            check_id: "single-solid".to_string(),
            kind: FemApplicabilityCheckKind::OneSolidScope,
            status: FemApplicabilityStatus::Pass,
            observed: Some(1.0),
            limit: Some(1.0),
            unit: Some("solid".to_string()),
            evidence_ids: vec![],
            detail: "One connected solid.".to_string(),
        }],
        sensitivity: Some(FemSensitivityEvidence {
            completed: true,
            input_ranges: vec![FemSensitivityInputRange {
                input_name: "top-load".to_string(),
                evidence_id: "load-top".to_string(),
                lower_factor: 0.9,
                upper_factor: 1.1,
            }],
            case_result_digests: vec![
                "sha256:load-low".to_string(),
                "sha256:load-high".to_string(),
            ],
            metric_ranges: vec![FemSensitivityMetricRange {
                metric_id: "bracket-stress".to_string(),
                nominal: 100.0,
                minimum: 90.0,
                maximum: 110.0,
                unit: "MPa".to_string(),
                dominant_input_name: Some("top-load".to_string()),
                decision_changed: false,
            }],
        }),
        validation_evidence: vec![FemValidationEvidence {
            validation_id: "bracket-bench".to_string(),
            kind: FemValidationEvidenceKind::PhysicalTest,
            source: "versioned bracket bench fixture".to_string(),
            result_digest: "sha256:bench".to_string(),
        }],
    }
}

#[test]
fn engineering_readiness_blocks_failed_applicability_and_decision_reversing_uncertainty() {
    let mut ledger = engineering_evidence(FemEvidenceAuthority::UserAccepted);
    ledger.applicability_checks[0].status = FemApplicabilityStatus::Blocked;
    assert_eq!(
        ledger.validate_decision_readiness().unwrap_err().message,
        "applicability check 'single-solid' blocks the engineering decision"
    );

    ledger.applicability_checks[0].status = FemApplicabilityStatus::Pass;
    ledger.sensitivity.as_mut().unwrap().metric_ranges[0].decision_changed = true;
    assert_eq!(
        ledger.validate_decision_readiness().unwrap_err().message,
        "sensitivity range for metric 'bracket-stress' changes the engineering decision"
    );
}

#[test]
fn engineering_readiness_never_invents_missing_material_load_or_support_evidence() {
    let mut ledger = engineering_evidence(FemEvidenceAuthority::UserAccepted);
    ledger
        .evidence
        .retain(|record| record.subject != FemEvidenceSubject::Material);
    ledger
        .input_bindings
        .retain(|binding| binding.evidence_id != "material-6061");
    ledger
        .validate()
        .expect("incomplete evidence remains representable");
    assert_eq!(
        ledger.validate_decision_readiness().unwrap_err().message,
        "required material evidence is missing"
    );
}

#[test]
fn numerically_green_never_hides_missing_evidence_nonlinearity_contact_or_decision_reversal() {
    let numerical_residual_passed = true;
    let convergence_passed = true;
    assert!(numerical_residual_passed && convergence_passed);

    let mut missing_evidence = engineering_evidence(FemEvidenceAuthority::UserAccepted);
    missing_evidence
        .evidence
        .retain(|record| record.subject != FemEvidenceSubject::Load);
    missing_evidence
        .input_bindings
        .retain(|binding| binding.evidence_id != "load-top");
    missing_evidence.assumptions[0].evidence_ids.clear();
    missing_evidence.sensitivity = None;
    assert!(missing_evidence
        .validate_decision_readiness()
        .unwrap_err()
        .message
        .contains("required load evidence is missing"));

    let mut outside_linear = engineering_evidence(FemEvidenceAuthority::UserAccepted);
    outside_linear.applicability_checks =
        audit_post_solve_applicability(&FemPostSolveApplicabilityInput {
            schema_version: FEM_SCHEMA_VERSION,
            characteristic_size_mm: 100.0,
            maximum_displacement_mm: 8.0,
            maximum_von_mises_mpa: 320.0,
            yield_strength_mpa: 276.0,
            hotspot_movement_mm: 5.0,
            boundary_condition_singularity: true,
        })
        .expect("post-solve applicability");
    assert!(outside_linear
        .validate_decision_readiness()
        .unwrap_err()
        .message
        .contains("blocks the engineering decision"));

    let mut unsupported_contact = engineering_evidence(FemEvidenceAuthority::UserAccepted);
    unsupported_contact.applicability_checks =
        audit_pre_solve_applicability(&FemPreSolveApplicabilityInput {
            schema_version: FEM_SCHEMA_VERSION,
            solid_count: 1,
            unsupported_interface_count: 1,
            characteristic_size_mm: 100.0,
            minimum_thickness_mm: 10.0,
            poisson_ratio: 0.33,
            constrained_translation_components: 3,
            selected_load_area_mm2: 100.0,
            selected_support_area_mm2: 100.0,
            has_point_load_or_support: false,
        })
        .expect("pre-solve applicability");
    let error = unsupported_contact
        .validate_decision_readiness()
        .expect_err("unsupported contact/interface path must block");
    assert!(error.field.contains("interfaces"));

    let mut uncertainty = engineering_evidence(FemEvidenceAuthority::UserAccepted);
    uncertainty.sensitivity = Some(
        run_bounded_sensitivity(
            &[FemSensitivityInputRange {
                input_name: "top-load".into(),
                evidence_id: "load-top".into(),
                lower_factor: 0.9,
                upper_factor: 1.1,
            }],
            &uncertainty.acceptance_criteria,
            |factors| {
                let factor = factors["top-load"];
                Ok(FemSensitivityCaseResult {
                    result_digest: format!("sha256:case-{factor}"),
                    metric_values: BTreeMap::from([("bracket-stress".into(), 130.0 * factor)]),
                })
            },
        )
        .expect("bounded sensitivity"),
    );
    let sensitivity = uncertainty.sensitivity.as_ref().expect("sensitivity");
    assert!(sensitivity.metric_ranges[0].decision_changed);
    assert_eq!(
        sensitivity.metric_ranges[0].dominant_input_name.as_deref(),
        Some("top-load")
    );
    assert!(uncertainty
        .validate_decision_readiness()
        .unwrap_err()
        .message
        .contains("changes the engineering decision"));
}

#[test]
fn engineering_evidence_blocks_proposed_inputs_and_enters_analysis_identity() {
    let proposed = engineering_evidence(FemEvidenceAuthority::Proposed);
    proposed
        .validate()
        .expect("structurally valid proposed evidence");
    assert_eq!(
        proposed.validate_decision_readiness().unwrap_err().message,
        "decision-critical evidence 'load-top' is not authoritative"
    );

    let accepted = engineering_evidence(FemEvidenceAuthority::UserAccepted);
    accepted
        .validate_decision_readiness()
        .expect("decision-ready evidence");
    assert_ne!(proposed.canonical_digest(), accepted.canonical_digest());
}

#[test]
fn camel_case_serialization_and_canonical_digests_stay_stable() {
    let material = material();
    let material_json = serde_json::to_value(&material).expect("material json");
    assert_eq!(material_json["schemaVersion"], FEM_SCHEMA_VERSION);
    assert_eq!(material_json["youngModulusMpa"], 68_900.0);
    assert_eq!(material_json["densityKgPerMm3"], 0.000_002_7);

    let load_a = FemLoad::SurfaceForce {
        schema_version: FEM_SCHEMA_VERSION,
        name: "top-load".to_string(),
        faces: load_faces(),
        total_force_n: FemForceVector {
            x_n: 0.0,
            y_n: 0.0,
            z_n: -1_000.0,
        },
    };
    let load_b = FemLoad::SurfaceForce {
        schema_version: FEM_SCHEMA_VERSION,
        name: "top-load".to_string(),
        faces: vec![face_target("b"), face_target("a")],
        total_force_n: FemForceVector {
            x_n: 0.0,
            y_n: 0.0,
            z_n: -1_000.0,
        },
    };
    assert_eq!(load_a.canonical_digest(), load_b.canonical_digest());

    let constraint_a = FemConstraint::PrescribedDisplacement {
        schema_version: FEM_SCHEMA_VERSION,
        name: "mount".to_string(),
        faces: constraint_faces(),
        displacement_mm: FemOptionalDisplacement {
            x_mm: Some(0.0),
            y_mm: Some(0.0),
            z_mm: Some(0.0),
        },
    };
    let constraint_b = FemConstraint::PrescribedDisplacement {
        schema_version: FEM_SCHEMA_VERSION,
        name: "mount".to_string(),
        faces: vec![face_target("d"), face_target("c")],
        displacement_mm: FemOptionalDisplacement {
            x_mm: Some(0.0),
            y_mm: Some(0.0),
            z_mm: Some(0.0),
        },
    };
    assert_eq!(
        constraint_a.canonical_digest(),
        constraint_b.canonical_digest()
    );

    let mesh_control_a = mesh_control();
    let mut mesh_control_b = mesh_control();
    mesh_control_b.local_refinements.reverse();
    assert_eq!(
        mesh_control_a.canonical_digest(),
        mesh_control_b.canonical_digest()
    );

    let runtime_a = runtime_identity();
    let mut runtime_b = runtime_identity();
    runtime_b.supported_capabilities.reverse();
    assert_eq!(runtime_a.canonical_digest(), runtime_b.canonical_digest());
    let evidence = engineering_evidence(FemEvidenceAuthority::UserAccepted);

    let analysis_a = FemAnalysisIdentity {
        schema_version: FEM_SCHEMA_VERSION,
        study_name: "bracket-static".to_string(),
        part_id: "bracket".to_string(),
        geometry_digest: "sha256:geometry".to_string(),
        engineering_evidence_digest: evidence.canonical_digest(),
        material_digest: material.canonical_digest(),
        load_digests: vec![load_a.canonical_digest(), "sha256:load-b".to_string()],
        constraint_digests: vec![
            constraint_a.canonical_digest(),
            "sha256:constraint-b".to_string(),
        ],
        mesh_control_digest: mesh_control_a.canonical_digest(),
        runtime_identity_digest: runtime_a.canonical_digest(),
    };
    let analysis_b = FemAnalysisIdentity {
        schema_version: FEM_SCHEMA_VERSION,
        study_name: "bracket-static".to_string(),
        part_id: "bracket".to_string(),
        geometry_digest: "sha256:geometry".to_string(),
        engineering_evidence_digest: evidence.canonical_digest(),
        material_digest: material.canonical_digest(),
        load_digests: vec!["sha256:load-b".to_string(), load_b.canonical_digest()],
        constraint_digests: vec![
            "sha256:constraint-b".to_string(),
            constraint_b.canonical_digest(),
        ],
        mesh_control_digest: mesh_control_b.canonical_digest(),
        runtime_identity_digest: runtime_b.canonical_digest(),
    };
    assert_eq!(analysis_a.canonical_digest(), analysis_b.canonical_digest());
    let mut changed_evidence = analysis_b;
    changed_evidence.engineering_evidence_digest = "sha256:different-evidence".to_string();
    assert_ne!(
        analysis_a.canonical_digest(),
        changed_evidence.canonical_digest()
    );
}

#[test]
fn validated_contracts_reject_invalid_values() {
    let invalid_material = FemMaterial {
        schema_version: FEM_SCHEMA_VERSION,
        name: "invalid".to_string(),
        young_modulus_mpa: f64::NAN,
        poisson_ratio: 0.5,
        density_kg_per_mm3: -1.0,
        yield_strength_mpa: f64::INFINITY,
    };
    assert!(
        invalid_material.validate().is_err(),
        "material should reject invalid fields"
    );

    let invalid_load = FemLoad::Pressure {
        schema_version: FEM_SCHEMA_VERSION,
        name: "bad-pressure".to_string(),
        faces: load_faces(),
        pressure_mpa: f64::NAN,
    };
    assert!(
        invalid_load.validate().is_err(),
        "load should reject invalid pressure"
    );

    let invalid_constraint = FemConstraint::PrescribedDisplacement {
        schema_version: FEM_SCHEMA_VERSION,
        name: "bad-displacement".to_string(),
        faces: constraint_faces(),
        displacement_mm: FemOptionalDisplacement {
            x_mm: Some(f64::INFINITY),
            y_mm: Some(0.0),
            z_mm: Some(0.0),
        },
    };
    assert!(
        invalid_constraint.validate().is_err(),
        "constraint should reject invalid displacement"
    );

    let invalid_mesh = FemMeshControl {
        schema_version: FEM_SCHEMA_VERSION,
        element_kind: FemElementKind::Tet4,
        global_size_mm: -1.0,
        local_refinements: vec![],
        budgets: FemBudgetLimits {
            schema_version: FEM_SCHEMA_VERSION,
            boundary_triangles: 0,
            tet4_cells: 0,
            nodes: 0,
            dofs: 0,
            sparse_nonzeros: 0,
            result_bytes: 0,
            convergence_levels: 0,
        },
    };
    assert!(
        invalid_mesh.validate().is_err(),
        "mesh control should reject invalid size and budgets"
    );
}

#[test]
fn admission_reports_observed_and_allowed_for_each_resource() {
    let estimate = FemAdmissionEstimate {
        schema_version: FEM_SCHEMA_VERSION,
        boundary_triangles: 250,
        tet4_cells: 120,
        nodes: 160,
        dofs: 480,
        sparse_nonzeros: 3_200,
        result_bytes: 120_000,
        convergence_levels: 4,
    };
    let limits = FemBudgetLimits {
        schema_version: FEM_SCHEMA_VERSION,
        boundary_triangles: 200,
        tet4_cells: 100,
        nodes: 150,
        dofs: 450,
        sparse_nonzeros: 3_000,
        result_bytes: 100_000,
        convergence_levels: 3,
    };

    let diagnostics = estimate.diagnostics(&limits);
    assert_eq!(diagnostics.len(), 7);
    assert_eq!(diagnostics[0].observed, 250);
    assert_eq!(diagnostics[0].allowed, 200);
    assert_eq!(diagnostics[1].observed, 120);
    assert_eq!(diagnostics[1].allowed, 100);
    assert_eq!(diagnostics[2].observed, 160);
    assert_eq!(diagnostics[2].allowed, 150);
    assert_eq!(diagnostics[3].observed, 480);
    assert_eq!(diagnostics[3].allowed, 450);
    assert_eq!(diagnostics[4].observed, 3_200);
    assert_eq!(diagnostics[4].allowed, 3_000);
    assert_eq!(diagnostics[5].observed, 120_000);
    assert_eq!(diagnostics[5].allowed, 100_000);
    assert_eq!(diagnostics[6].observed, 4);
    assert_eq!(diagnostics[6].allowed, 3);

    assert!(
        estimate.admit(&limits).is_err(),
        "over-budget admission should fail"
    );
}
