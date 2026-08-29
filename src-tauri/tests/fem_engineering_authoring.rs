use std::collections::BTreeMap;

use ecky_cad_lib::contracts::{TaggedAnchorBinding, TaggedAnchorKind};
use ecky_cad_lib::ecky_cad_host::analysis_boundary::{
    AnalysisBoundaryEvidence, AnalysisBoundaryFaceGroup, AnalysisBoundarySurface,
};
use ecky_cad_lib::fem_engineering::{
    authored_study_from_core, engineering_ledger_from_core, resolve_fem_face_tags,
};
use ecky_fem::{FemBudgetLimits, FemConstraint, FemFaceTarget, FemLoad, FEM_SCHEMA_VERSION};
use ecky_render::scheme::compile_to_core_program;

#[test]
fn authored_fem_evidence_lowers_to_digest_bound_domain_ledger_and_stays_pending_runtime_audits() {
    let source = r#"
      (model
        (part bracket (box 10 10 10))
        (analysis bracket-static
          (linear-static :part bracket)
          (question bracket-strength
            :statement "Does stress remain below the limit?"
            :decision "accept or revise thickness"
            :acceptance-metrics [bracket-stress])
          (acceptance-criterion bracket-stress
            :field von-mises-stress :comparison less-than-or-equal
            :limit "138" :unit MPa :requires-convergence true)
          (idealization exact-solid :justification "Use exact connected solid." :accepted true)
          (evidence material-6061 :subject material :source "qualified datasheet"
            :authority recorded-source :uncertainty-percent 3 :decision-critical true)
          (evidence load-top :subject load :source "user service load"
            :authority user-accepted :uncertainty-percent 10 :decision-critical true)
          (evidence support-mount :subject support :source "user fixture definition"
            :authority user-accepted :uncertainty-percent 0 :decision-critical true)
          (input-evidence aluminum-6061 :evidence material-6061)
          (input-evidence top-load :evidence load-top)
          (input-evidence mounting :evidence support-mount)
          (assumption small-strain :category physics
            :statement "Displacement remains small relative to span."
            :status accepted :evidence [load-top])
          (validation-evidence bracket-bench :kind physical-test
            :source "versioned bench fixture" :result-digest "sha256:bench")
          (material aluminum-6061
            :young-modulus 68900MPa :poisson-ratio 0.33
            :density 2700kg-per-m3 :yield-strength 276MPa)
          (volume-mesh :element tet4 :size 2mm)
          (fixed :faces (tag mounting))
          (surface-force :faces (tag load-pad) :total [0N 0N -1000N])
          (solve :method sparse-direct)))
    "#;
    let program = compile_to_core_program(source).expect("compile authored FEM evidence");
    let ledger = engineering_ledger_from_core(
        &program,
        "bracket-static",
        "sha256:geometry",
        "sha256:geometry",
    )
    .expect("domain evidence ledger");

    assert_eq!(ledger.question.question_id, "bracket-strength");
    assert_eq!(ledger.acceptance_criteria[0].limit, 138.0);
    assert_eq!(ledger.evidence.len(), 3);
    assert_eq!(ledger.input_bindings.len(), 3);
    assert!(ledger.validate().is_ok());
    assert!(ledger
        .validate_decision_readiness()
        .unwrap_err()
        .message
        .contains("applicability check"));
}

#[test]
fn authored_fem_mechanics_lower_to_validated_material_mesh_loads_and_supports() {
    let source = r#"
      (model
        (params
          (number load-n 1000 :min 10 :max 10000 :step 10 :unit "N")
          (number mesh-size 2 :min 0.25 :max 10 :step 0.25 :unit length))
        (part bracket (box 10 10 10))
        (analysis bracket-static
          (linear-static :part bracket)
          (material aluminum-6061
            :young-modulus 68900MPa :poisson-ratio 0.33
            :density 2700kg-per-m3 :yield-strength 276MPa)
          (volume-mesh :element tet4 :size mesh-size
            (refine :faces (tag load-pad) :size 1mm))
          (fixed :faces (tag mounting))
          (surface-force :faces (tag load-pad) :total [0N 0N (- load-n)])
          (solve :method sparse-direct)))
    "#;
    let program = compile_to_core_program(source).expect("compile authored FEM mechanics");
    let resolved_faces = BTreeMap::from([
        ("mounting".to_string(), vec![face("mounting")]),
        ("load-pad".to_string(), vec![face("load-pad")]),
    ]);

    let study = authored_study_from_core(&program, "bracket-static", &resolved_faces, budgets())
        .expect("lower authored FEM mechanics");

    assert_eq!(study.part_id, "bracket");
    assert_eq!(study.material.young_modulus_mpa, 68900.0);
    assert_eq!(study.material.density_kg_per_mm3, 2.7e-6);
    assert_eq!(study.mesh_control.global_size_mm, 2.0);
    assert_eq!(study.mesh_control.local_refinements[0].size_mm, 1.0);
    assert!(matches!(
        &study.constraints[0],
        FemConstraint::Fixed { faces, .. }
            if faces[0].durable_target_id == "durable:mounting"
    ));
    assert!(matches!(
        &study.loads[0],
        FemLoad::SurfaceForce { total_force_n, .. }
            if total_force_n.z_n == -1000.0
    ));
    assert_eq!(study.solver_method, "sparse-direct");
}

#[test]
fn authored_topology_controls_lower_from_selected_ecky_study() {
    let source = r#"
      (model
        (params
          (number target-volume 0.35 :min 0.1 :max 0.8 :step 0.01)
          (number filter-radius 3mm :min 0.5mm :max 10mm :step 0.5mm :unit length))
        (part bracket (box 10 10 10))
        (analysis bracket-topology
          (linear-static :part bracket)
          (material aluminum-6061
            :young-modulus 68900MPa :poisson-ratio 0.33
            :density 2700kg-per-m3 :yield-strength 276MPa)
          (volume-mesh :element tet4 :size 2mm)
          (topology-controls
            :volume-fraction target-volume
            :penalty 3
            :minimum-density 0.001
            :filter-radius filter-radius
            :move-limit 0.2
            :convergence-tolerance 0.01)
          (fixed :faces (tag mounting))
          (surface-force :faces (tag load-pad) :total [0N 0N -1000N])
          (solve :method sparse-direct)))
    "#;
    let program = compile_to_core_program(source).expect("compile authored topology controls");
    let resolved_faces = BTreeMap::from([
        ("mounting".to_string(), vec![face("mounting")]),
        ("load-pad".to_string(), vec![face("load-pad")]),
    ]);

    let study = authored_study_from_core(&program, "bracket-topology", &resolved_faces, budgets())
        .expect("lower authored topology controls");
    let controls = study
        .topology_controls
        .expect("selected study owns topology controls");

    assert_eq!(controls.volume_fraction, 0.35);
    assert_eq!(controls.penalty, 3.0);
    assert_eq!(controls.minimum_density, 0.001);
    assert_eq!(controls.filter_radius_mm, 3.0);
    assert_eq!(controls.move_limit, 0.2);
    assert_eq!(controls.convergence_tolerance, 0.01);
}

fn face(name: &str) -> FemFaceTarget {
    FemFaceTarget {
        schema_version: FEM_SCHEMA_VERSION,
        part_id: "bracket".to_string(),
        canonical_target_id: format!("face:{name}"),
        durable_target_id: format!("durable:{name}"),
        source_geometry_digest: "sha256:geometry".to_string(),
    }
}

fn budgets() -> FemBudgetLimits {
    FemBudgetLimits {
        schema_version: FEM_SCHEMA_VERSION,
        boundary_triangles: 100_000,
        tet4_cells: 500_000,
        nodes: 200_000,
        dofs: 600_000,
        sparse_nonzeros: 30_000_000,
        result_bytes: 256_000_000,
        convergence_levels: 4,
    }
}

#[test]
fn fem_face_tags_resolve_through_exact_canonical_and_durable_brep_identity() {
    let anchors = BTreeMap::from([(
        "mounting".to_string(),
        TaggedAnchorBinding {
            kind: TaggedAnchorKind::Face,
            authored_selector: "bottom".to_string(),
            target: "bracket".to_string(),
            target_ids: vec!["target:mounting".to_string()],
            durable_target_ids: vec!["durable:mounting".to_string()],
            canonical_target_ids: vec!["face:mounting".to_string()],
            alias_ids: vec![],
        },
    )]);
    let boundary = AnalysisBoundarySurface {
        tessellation_policy: Default::default(),
        part_id: "bracket".to_string(),
        label: "Bracket".to_string(),
        source_geometry_digest: "sha256:geometry".to_string(),
        vertices: vec![],
        triangles: vec![],
        triangle_face_group_indices: vec![],
        face_groups: vec![AnalysisBoundaryFaceGroup {
            part_id: "bracket".to_string(),
            target_id: "target:mounting".to_string(),
            canonical_target_id: "face:mounting".to_string(),
            durable_target_id: Some("durable:mounting".to_string()),
            label: "mounting".to_string(),
            area: 100.0,
            triangle_count: 2,
        }],
        evidence: AnalysisBoundaryEvidence {
            closed: true,
            manifold: true,
            component_count: 1,
            positive_volume: true,
            boundary_edge_count: 0,
            non_manifold_edge_count: 0,
            winding_mismatch_count: 0,
            signed_volume: 1.0,
        },
        content_digest: "sha256:boundary".to_string(),
    };

    let resolved = resolve_fem_face_tags(&anchors, &boundary).expect("exact FEM face tags");
    assert_eq!(
        resolved["mounting"][0].durable_target_id,
        "durable:mounting"
    );

    let mut stale = boundary.clone();
    stale.face_groups[0].durable_target_id = Some("durable:changed".to_string());
    assert!(resolve_fem_face_tags(&anchors, &stale)
        .unwrap_err()
        .message
        .contains("resolved to 0"));
}

#[test]
fn fem_semantics_reject_wrong_pressure_dimension_and_duplicate_study_identity() {
    let invalid_pressure = r#"
      (model
        (part bracket (box 10 10 10))
        (analysis duplicate
          (linear-static :part bracket)
          (material steel :young-modulus 210000MPa :poisson-ratio 0.3
            :density 7850kg-per-m3 :yield-strength 250MPa)
          (volume-mesh :element tet4 :size 2mm)
          (fixed :faces (tag mounting))
          (pressure :faces (tag load-pad) :value 5N)
          (solve :method sparse-direct)))
    "#;
    let program = compile_to_core_program(invalid_pressure).expect("compile dimensional payload");
    let faces = BTreeMap::from([
        ("mounting".to_string(), vec![face("mounting")]),
        ("load-pad".to_string(), vec![face("load-pad")]),
    ]);
    let error = authored_study_from_core(&program, "duplicate", &faces, budgets()).unwrap_err();
    assert!(error.message.contains("expected MPa"), "{error:?}");

    let duplicate_studies = invalid_pressure.replace(
        "    </model-never>",
        "",
    ).replace(
        "          (solve :method sparse-direct)))",
        "          (solve :method sparse-direct))\n        (analysis duplicate (linear-static :part bracket)))",
    );
    let duplicate_program =
        compile_to_core_program(&duplicate_studies).expect("compile duplicate study names");
    let error =
        authored_study_from_core(&duplicate_program, "duplicate", &faces, budgets()).unwrap_err();
    assert!(error.message.contains("duplicate"), "{error:?}");
}

#[test]
fn fem_length_and_force_reject_dimensionless_literals_even_in_permissive_cad_mode() {
    let study = |mesh_size: &str, force: &str| {
        format!(
            r#"
      (model
        (part bracket (box 10 10 10))
        (analysis strict-units
          (linear-static :part bracket)
          (material steel :young-modulus 210000MPa :poisson-ratio 0.3
            :density 7850kg-per-m3 :yield-strength 250MPa)
          (volume-mesh :element tet4 :size {mesh_size})
          (fixed :faces (tag mounting))
          (surface-force :faces (tag load-pad) :total [0N 0N {force}])
          (solve :method sparse-direct)))
    "#
        )
    };
    let faces = BTreeMap::from([
        ("mounting".to_string(), vec![face("mounting")]),
        ("load-pad".to_string(), vec![face("load-pad")]),
    ]);

    let dimensionless_length =
        compile_to_core_program(&study("2", "-5N")).expect("parse dimensionless mesh size");
    let error = authored_study_from_core(&dimensionless_length, "strict-units", &faces, budgets())
        .expect_err("FEM mesh size requires explicit length unit");
    assert!(error.message.contains("expected mm"), "{error:?}");

    let dimensionless_force =
        compile_to_core_program(&study("2mm", "-5")).expect("parse dimensionless force");
    let error = authored_study_from_core(&dimensionless_force, "strict-units", &faces, budgets())
        .expect_err("FEM force requires explicit force unit");
    assert!(error.message.contains("expected N"), "{error:?}");
}

fn semantic_study_source(extra_or_replacement: &str) -> String {
    format!(
        r#"
      (model
        (part bracket (box 10 10 10))
        (analysis static
          (linear-static :part bracket)
          (material steel :young-modulus 210000MPa :poisson-ratio 0.3
            :density 7850kg-per-m3 :yield-strength 250MPa)
          (volume-mesh :element tet4 :size 2mm)
          (fixed :faces (tag mounting))
          (surface-force :faces (tag load-pad) :total [0N 0N -10N])
          (solve :method sparse-direct)
          {extra_or_replacement}))
    "#
    )
}

fn resolved_semantic_faces() -> BTreeMap<String, Vec<FemFaceTarget>> {
    BTreeMap::from([
        ("mounting".to_string(), vec![face("mounting")]),
        ("load-pad".to_string(), vec![face("load-pad")]),
    ])
}

#[test]
fn fem_unit_matrix_rejects_cross_dimension_and_non_finite_mechanics_values() {
    let faces = resolved_semantic_faces();
    for (label, from, to, expected) in [
        ("modulus-as-force", "210000MPa", "210000N", "expected MPa"),
        (
            "density-as-stress",
            "7850kg-per-m3",
            "7850MPa",
            "expected kg/mm^3",
        ),
        ("mesh-as-force", "2mm", "2N", "expected mm"),
        ("force-as-pressure", "-10N", "-10MPa", "expected N"),
        (
            "pressure-as-force",
            "(surface-force :faces (tag load-pad) :total [0N 0N -10N])",
            "(pressure :faces (tag load-pad) :value 10N)",
            "expected MPa",
        ),
    ] {
        let source = semantic_study_source("").replace(from, to);
        let program = compile_to_core_program(&source).expect("dimension payload parses");
        let error =
            authored_study_from_core(&program, "static", &faces, budgets()).expect_err(label);
        assert!(error.message.contains(expected), "{label}: {error:?}");
    }

    let source = semantic_study_source("");
    let mut program = compile_to_core_program(&source).expect("finite source");
    let material = program.analyses[0]
        .clauses
        .iter_mut()
        .find_map(|clause| match &mut clause.kind {
            ecky_render::core_ir::CoreAnalysisClauseKind::Material { young_modulus, .. } => {
                Some(young_modulus)
            }
            _ => None,
        })
        .expect("material scalar");
    *material = ecky_render::core_ir::CoreAnalysisScalarExpr::Literal {
        value: f64::INFINITY,
        unit: "MPa".into(),
    };
    let error = authored_study_from_core(&program, "static", &faces, budgets())
        .expect_err("non-finite material input");
    assert!(error.message.contains("finite"), "{error:?}");
}

#[test]
fn fem_semantic_matrix_rejects_missing_duplicate_unsupported_and_geometry_cycle_cases() {
    let faces = resolved_semantic_faces();
    let base = semantic_study_source("");
    let cases = [
        ("duplicate-material", base.replace(
            "          (volume-mesh",
            "          (material duplicate :young-modulus 1MPa :poisson-ratio 0.2 :density 1kg-per-m3 :yield-strength 1MPa)\n          (volume-mesh",
        ), "duplicate material"),
        ("unsupported-element", base.replace(":element tet4", ":element hex8"), "unsupported element"),
        ("unsupported-solver", base.replace("sparse-direct", "iterative"), "unsupported solver"),
        ("missing-part", base.replace(":part bracket", ":part ghost"), "missing part"),
        ("missing-material", base.replace(
            "          (material steel :young-modulus 210000MPa :poisson-ratio 0.3\n            :density 7850kg-per-m3 :yield-strength 250MPa)\n",
            "",
        ), "missing material"),
        ("missing-mesh", base.replace("          (volume-mesh :element tet4 :size 2mm)\n", ""), "missing volume mesh"),
        ("missing-constraint", base.replace("          (fixed :faces (tag mounting))\n", ""), "needs at least one displacement constraint"),
        ("missing-load", base.replace("          (surface-force :faces (tag load-pad) :total [0N 0N -10N])\n", ""), "needs at least one load"),
    ];
    for (label, source, expected) in cases {
        let program = compile_to_core_program(&source).expect("semantic case parses");
        let error =
            authored_study_from_core(&program, "static", &faces, budgets()).expect_err(label);
        assert!(error.message.contains(expected), "{label}: {error:?}");
    }

    let invalid_materials = [
        (
            "poisson",
            ":poisson-ratio 0.3",
            ":poisson-ratio 0.5",
            "poissonRatio",
        ),
        ("modulus", "210000MPa", "0MPa", "youngModulusMpa"),
        ("density", "7850kg-per-m3", "0kg-per-m3", "densityKgPerMm3"),
        ("yield", "250MPa", "-1MPa", "yieldStrengthMpa"),
    ];
    for (label, from, to, expected) in invalid_materials {
        let program =
            compile_to_core_program(&base.replace(from, to)).expect("invalid material parses");
        let error =
            authored_study_from_core(&program, "static", &faces, budgets()).expect_err(label);
        assert!(error.message.contains(expected), "{label}: {error:?}");
    }

    let unsupported_study = "(model (part body (box 1 1 1)) (analysis modes (modal :part body)))";
    let error = compile_to_core_program(unsupported_study).expect_err("unsupported study kind");
    assert!(
        error.message.contains("analysis") || error.message.contains("modal"),
        "{error:?}"
    );

    let cycle = "(model (part body (box (fem-max body-static von-mises) 10 10)))";
    let error = compile_to_core_program(cycle)
        .expect_err("analysis result cannot drive same-version geometry");
    assert!(
        error.message.contains("analysis-to-geometry cycle"),
        "{error:?}"
    );
}
