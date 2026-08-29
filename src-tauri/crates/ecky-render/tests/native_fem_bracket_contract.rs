use ecky_render::scheme::compile_to_core_program;

#[test]
fn parameterized_bracket_study_compiles_as_non_geometry_metadata() {
    let source = r#"
        (model
          (params
            (number load-n 1000 :min 10 :max 10000 :step 10 :unit "N")
            (number mesh-size 2.0 :min 0.25 :max 10 :step 0.25 :unit length))
          (part bracket
            (box 80 30 12))
          (analysis bracket-static
            (linear-static :part bracket)
            (question bracket-strength
              :statement "Does the bracket remain below the stress limit?"
              :decision "accept or revise bracket thickness"
              :acceptance-metrics [bracket-stress])
            (acceptance-criterion bracket-stress
              :field von-mises-stress
              :comparison less-than-or-equal
              :limit "138"
              :unit MPa
              :requires-convergence true)
            (idealization exact-solid
              :justification "Use exact connected manufacturing solid."
              :accepted true)
            (evidence load-top
              :subject load
              :source "user service load case"
              :authority user-accepted
              :uncertainty-percent 10
              :decision-critical true)
            (input-evidence top-load :evidence load-top)
            (assumption small-strain
              :category physics
              :statement "Displacement remains small relative to span."
              :status accepted
              :evidence [load-top])
            (validation-evidence bracket-bench
              :kind physical-test
              :source "versioned bracket bench fixture"
              :result-digest "sha256:bench")
            (material aluminum-6061
              :young-modulus 68900MPa
              :poisson-ratio 0.33
              :density 2700kg-per-m3
              :yield-strength 276MPa)
            (volume-mesh :element tet4 :size mesh-size)
            (fixed :faces (tag mounting))
            (surface-force :faces (tag load-pad) :total [0N 0N (- load-n)])
            (solve :method sparse-direct)))
    "#;

    let program = compile_to_core_program(source).expect("FEM bracket should compile");
    assert_eq!(program.parts.len(), 1);
    assert_eq!(program.analyses.len(), 1);
    assert_eq!(program.analyses[0].id.raw(), 1);
    assert_eq!(program.analyses[0].name, "bracket-static");
    assert_eq!(program.analyses[0].part, "bracket");
    assert_eq!(program.analyses[0].element, "tet4");
    assert!(program.analyses[0].span.is_some());
    assert!(program.analyses[0].clauses.iter().any(|clause| matches!(
        &clause.kind,
        ecky_render::core_ir::CoreAnalysisClauseKind::Question {
            question_id,
            statement,
            decision,
            acceptance_metric_ids,
        } if question_id == "bracket-strength"
            && statement == "Does the bracket remain below the stress limit?"
            && decision == "accept or revise bracket thickness"
            && acceptance_metric_ids == &["bracket-stress"]
    )));
    assert!(program.analyses[0].clauses.iter().any(|clause| matches!(
        &clause.kind,
        ecky_render::core_ir::CoreAnalysisClauseKind::AcceptanceCriterion {
            metric_id,
            field,
            comparison,
            limit,
            unit,
            requires_convergence: true,
        } if metric_id == "bracket-stress"
            && field == "von-mises-stress"
            && comparison == "less-than-or-equal"
            && limit == "138"
            && unit == "MPa"
    )));
    assert!(program.analyses[0].clauses.iter().any(|clause| matches!(
        &clause.kind,
        ecky_render::core_ir::CoreAnalysisClauseKind::Evidence {
            evidence_id,
            subject,
            authority,
            uncertainty_percent,
            decision_critical: true,
            ..
        } if evidence_id == "load-top"
            && subject == "load"
            && authority == "user-accepted"
            && (*uncertainty_percent - 10.0).abs() <= f64::EPSILON
    )));
    assert!(program.analyses[0].clauses.iter().any(|clause| matches!(
        &clause.kind,
        ecky_render::core_ir::CoreAnalysisClauseKind::Material {
            name,
            young_modulus,
            poisson_ratio,
            density,
            yield_strength,
        } if name == "aluminum-6061"
            && young_modulus.literal_value("MPa") == Some(68900.0)
            && poisson_ratio.literal_value("") == Some(0.33)
            && density.literal_value("kg-per-m3") == Some(2700.0)
            && yield_strength.literal_value("MPa") == Some(276.0)
    )));
    assert!(program.analyses[0].clauses.iter().any(|clause| matches!(
        &clause.kind,
        ecky_render::core_ir::CoreAnalysisClauseKind::VolumeMesh {
            element,
            size,
            local_refinements,
        } if element == "tet4"
            && size.parameter_key() == Some("mesh-size")
            && local_refinements.is_empty()
    )));
    assert!(program.analyses[0].clauses.iter().any(|clause| matches!(
        &clause.kind,
        ecky_render::core_ir::CoreAnalysisClauseKind::Fixed { face_tag }
            if face_tag == "mounting"
    )));
    assert!(program.analyses[0].clauses.iter().any(|clause| matches!(
        &clause.kind,
        ecky_render::core_ir::CoreAnalysisClauseKind::SurfaceForce {
            face_tag,
            total,
        } if face_tag == "load-pad"
            && total[0].literal_value("N") == Some(0.0)
            && total[1].literal_value("N") == Some(0.0)
            && total[2].parameter_scale("load-n") == Some(-1.0)
    )));
}

#[test]
fn topology_controls_retain_typed_model_expressions() {
    let source = r#"
      (model
        (params
          (number target-volume 0.35 :min 0.1 :max 0.8 :step 0.01)
          (number filter-radius 3mm :min 0.5mm :max 10mm :step 0.5mm :unit length))
        (part body (box 10 10 10))
        (analysis topology
          (linear-static :part body)
          (topology-controls
            :volume-fraction target-volume
            :penalty 3
            :minimum-density 0.001
            :filter-radius filter-radius
            :move-limit 0.2
            :convergence-tolerance 0.01)))
    "#;

    let program = compile_to_core_program(source).expect("compile topology controls");
    assert!(program.analyses[0].clauses.iter().any(|clause| matches!(
        &clause.kind,
        ecky_render::core_ir::CoreAnalysisClauseKind::TopologyControls {
            volume_fraction,
            penalty,
            minimum_density,
            filter_radius,
            move_limit,
            convergence_tolerance,
        } if volume_fraction.parameter_key() == Some("target-volume")
            && penalty.literal_value("") == Some(3.0)
            && minimum_density.literal_value("") == Some(0.001)
            && filter_radius.parameter_key() == Some("filter-radius")
            && move_limit.literal_value("") == Some(0.2)
            && convergence_tolerance.literal_value("") == Some(0.01)
    )));
}

#[test]
fn analysis_stays_out_of_geometry_expressions() {
    let source = r#"
        (model
          (part bracket
            (begin
              (analysis bracket-static
                (linear-static :part bracket)
                (volume-mesh :element tet4 :size 10mm))
              (box 10mm 10 10))))
    "#;

    let err = compile_to_core_program(source).expect_err("analysis should stay top-level");
    assert!(
        err.message.contains("analysis"),
        "unexpected error: {err:?}"
    );
}

#[test]
fn every_supported_boundary_condition_retains_typed_payload_and_identity() {
    let source = r#"
      (model
        (part body (box 10 10 10))
        (analysis mechanics
          (linear-static :part body)
          (material steel :young-modulus 210000MPa :poisson-ratio 0.3
            :density 7850kg-per-m3 :yield-strength 250MPa)
          (volume-mesh :element tet4 :size 2mm
            (refine :faces (tag loaded) :size 1mm))
          (fixed :faces (tag support))
          (prescribed-displacement :faces (tag guided)
            :displacement [0mm free 0.2mm])
          (surface-force :faces (tag loaded) :total [1N 2N 3N])
          (traction :faces (tag shear) :vector [1MPa 2MPa 3MPa])
          (pressure :faces (tag pressure-face) :value 4MPa)
          (solve :method sparse-direct)))
    "#;

    let program = compile_to_core_program(source).expect("compile all FEM clauses");
    let clauses = &program.analyses[0].clauses;
    assert!(clauses.iter().all(|clause| clause.span.is_some()));
    let ids = clauses
        .iter()
        .map(|clause| clause.id.raw())
        .collect::<Vec<_>>();
    assert_eq!(ids, (1..=ids.len() as u64).collect::<Vec<_>>());
    assert!(clauses.iter().any(|clause| matches!(
        &clause.kind,
        ecky_render::core_ir::CoreAnalysisClauseKind::PrescribedDisplacement {
            face_tag,
            displacement,
        } if face_tag == "guided"
            && displacement[0].as_ref().and_then(|v| v.literal_value("mm")) == Some(0.0)
            && displacement[1].is_none()
            && displacement[2].as_ref().and_then(|v| v.literal_value("mm")) == Some(0.2)
    )));
    assert!(clauses.iter().any(|clause| matches!(
        &clause.kind,
        ecky_render::core_ir::CoreAnalysisClauseKind::Traction { face_tag, vector }
            if face_tag == "shear" && vector[2].literal_value("MPa") == Some(3.0)
    )));
    assert!(clauses.iter().any(|clause| matches!(
        &clause.kind,
        ecky_render::core_ir::CoreAnalysisClauseKind::Pressure { face_tag, pressure }
            if face_tag == "pressure-face" && pressure.literal_value("MPa") == Some(4.0)
    )));
}
