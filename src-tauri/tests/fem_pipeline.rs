use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::time::{SystemTime, UNIX_EPOCH};

use ecky_cad_lib::contracts::{TaggedAnchorBinding, TaggedAnchorKind};
use ecky_cad_lib::ecky_cad_host::analysis_boundary::{
    AnalysisBoundaryEvidence, AnalysisBoundaryFaceGroup, AnalysisBoundarySurface,
};
use ecky_cad_lib::fem_engineering::authored_study_from_core;
use ecky_cad_lib::fem_mesher::{
    probe_ftetwild_runtime, FTetWildRuntimeCapabilities, FTetWildRuntimeIdentity,
    FTetWildRuntimeRequirement, FTetWildWorkerRequest,
};
use ecky_cad_lib::models::PathResolver;
use ecky_cad_lib::services::fem::{execute_fem_pipeline, FemPipelineControl, FemPipelineStage};
use ecky_cad_lib::services::fem_artifacts::{load_fem_result_asset, publish_fem_result_asset};
use ecky_fem::{
    solve_linear_static, CanonicalDigest, FemBudgetLimits, FemConstraint, FemFaceTarget,
    FemForceVector, FemIdealizationKind, FemLoad, FemMaterial, FemMeshingEvidence,
    FemOptionalDisplacement, FemPoint3, FemRuntimeIdentity, FemVolumeMesh, FemVolumeMeshInput,
    FEM_SCHEMA_VERSION,
};
use ecky_render::scheme::compile_to_core_program;

#[test]
fn native_pipeline_runs_authored_tagged_study_with_ordered_progress_and_engineering_gates() {
    let Some(runtime_root) = std::env::var_os("ECKY_FTETWILD_RUNTIME_ROOT") else {
        eprintln!("ECKY_FTETWILD_RUNTIME_ROOT unset; native FEM pipeline proof is platform-gated");
        return;
    };
    let runtime = probe_ftetwild_runtime(
        PathBuf::from(runtime_root),
        &FTetWildRuntimeRequirement {
            runtime_version: "0.1.0-ecky.1".to_string(),
            source_revision: "d7d99bb4387a07895b9adce058dc7305f6b6e5ab".to_string(),
        },
    )
    .expect("pinned runtime");
    let program = compile_to_core_program(AUTHORED_STUDY).expect("compile study");
    let root = temp_root();
    let mut progress = Vec::new();
    let result = execute_fem_pipeline(
        &program,
        "bracket-static",
        &tagged_anchors(),
        &tetra_boundary(),
        budgets(),
        &runtime,
        &root,
        &FemPipelineControl {
            envelope_mm: 0.1,
            minimum_scaled_jacobian: 1.0e-6,
            maximum_runtime_ms: 120_000,
            relative_solver_tolerance: 1.0e-8,
        },
        &AtomicBool::new(false),
        |event| progress.push(event),
    )
    .expect("native authored FEM pipeline");

    assert!(!result.mesh.cells.is_empty());
    assert!(result
        .solution
        .postprocess
        .summary
        .maximum_displacement
        .value
        .is_finite());
    assert!(result
        .solution
        .postprocess
        .summary
        .maximum_von_mises
        .value
        .is_finite());
    assert!(result.solution.equilibrium.relative_imbalance <= 1.0e-8);
    assert!(result.solution.postprocess.summary.mass_kg > 0.0);
    assert_eq!(result.mesh.source_boundary_digest, "sha256:boundary");
    assert!(result
        .analysis_identity
        .canonical_digest()
        .starts_with("sha256:"));
    assert!(
        result.decision_readiness_error.is_none(),
        "{:?}",
        result.decision_readiness_error
    );
    assert_eq!(
        progress.iter().map(|event| event.stage).collect::<Vec<_>>(),
        vec![
            FemPipelineStage::Resolve,
            FemPipelineStage::BoundaryMesh,
            FemPipelineStage::VolumeMesh,
            FemPipelineStage::ValidateMesh,
            FemPipelineStage::Assemble,
            FemPipelineStage::ApplyConstraints,
            FemPipelineStage::Solve,
            FemPipelineStage::Postprocess,
            FemPipelineStage::Verify,
        ]
    );
    assert!(progress[6].cancellation_boundary);
    assert!(fs::read_dir(&root).expect("scratch").next().is_none());

    let resolver = TestResolver { root: root.clone() };
    let asset =
        publish_fem_result_asset(&resolver, &result, "sha256:test-source", 64 * 1024 * 1024)
            .expect("atomic immutable result asset");
    assert_eq!(asset.arrays.len(), 7);
    assert!(asset.manifest_path.is_file());
    assert!(asset.decision_ready);
    assert_eq!(
        asset.idealization_artifact.kind,
        FemIdealizationKind::ExactSolid
    );
    assert_eq!(
        asset.idealization_artifact.manufacturing_geometry_digest,
        result
            .engineering_evidence
            .idealization
            .source_geometry_digest
    );
    assert_eq!(
        asset.idealization_artifact_digest,
        asset.idealization_artifact.canonical_digest()
    );
    let cached =
        publish_fem_result_asset(&resolver, &result, "sha256:test-source", 64 * 1024 * 1024)
            .expect("exact immutable cache hit");
    assert_eq!(cached.manifest_path, asset.manifest_path);
    assert_eq!(
        cached.idealization_artifact_digest,
        asset.idealization_artifact_digest
    );

    let nodes_path = asset
        .manifest_path
        .parent()
        .expect("asset directory")
        .join(&asset.arrays[0].path);
    fs::write(&nodes_path, b"corrupt").expect("corrupt fixture array");
    let error = load_fem_result_asset(
        &resolver,
        &asset.analysis_identity_digest,
        &asset.solution_digest,
        64 * 1024 * 1024,
    )
    .expect_err("corrupt sidecar must fail");
    assert!(
        error.to_string().contains("byte length") || error.to_string().contains("digest mismatch")
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn outer_failures_stop_at_resolve_boundary_or_rigid_mode_gate_before_publication() {
    let program = compile_to_core_program(AUTHORED_STUDY).expect("compile study");
    let scratch = temp_root();
    let mut progress = Vec::new();
    let missing_tag_error = execute_fem_pipeline(
        &program,
        "bracket-static",
        &BTreeMap::new(),
        &tetra_boundary(),
        budgets(),
        &dummy_runtime(),
        &scratch,
        &pipeline_control(),
        &AtomicBool::new(false),
        |event| progress.push(event),
    )
    .expect_err("unresolved tags stop before meshing");
    assert!(missing_tag_error.to_string().contains("tag 'mounting'"));
    assert_eq!(progress, vec![progress[0].clone()]);
    assert_eq!(progress[0].stage, FemPipelineStage::Resolve);

    progress.clear();
    let cancelled_error = execute_fem_pipeline(
        &program,
        "bracket-static",
        &tagged_anchors(),
        &tetra_boundary(),
        budgets(),
        &dummy_runtime(),
        &scratch,
        &pipeline_control(),
        &AtomicBool::new(true),
        |event| progress.push(event),
    )
    .expect_err("cancelled job stops before meshing");
    assert!(cancelled_error.to_string().contains("cancelled"));
    assert_eq!(progress.len(), 1);
    assert_eq!(progress[0].stage, FemPipelineStage::Resolve);

    let study = authored_study_from_core(
        &program,
        "bracket-static",
        &BTreeMap::from([
            ("mounting".to_string(), vec![volume_face(3)]),
            ("load-pad".to_string(), vec![volume_face(0)]),
        ]),
        budgets(),
    )
    .expect("authored study");
    let mut open_boundary = tetra_boundary();
    open_boundary.evidence.closed = false;
    open_boundary.evidence.boundary_edge_count = 3;
    let open_error = FTetWildWorkerRequest::from_analysis_boundary(
        "open-domain",
        &open_boundary,
        &study.mesh_control,
        0.1,
        1.0e-6,
        1000,
    )
    .expect_err("open domain stops at boundary gate");
    assert!(open_error.to_string().contains("closed manifold"));

    let mesh = one_tet_volume_mesh();
    let underconstrained = solve_linear_static(
        &mesh,
        &FemMaterial {
            schema_version: FEM_SCHEMA_VERSION,
            name: "test".into(),
            young_modulus_mpa: 1000.0,
            poisson_ratio: 0.25,
            density_kg_per_mm3: 1.0e-6,
            yield_strength_mpa: 100.0,
        },
        &[FemLoad::SurfaceForce {
            schema_version: FEM_SCHEMA_VERSION,
            name: "load".into(),
            faces: vec![mesh.face_group_targets[0].clone()],
            total_force_n: FemForceVector {
                x_n: 0.0,
                y_n: 0.0,
                z_n: -1.0,
            },
        }],
        &[FemConstraint::PrescribedDisplacement {
            schema_version: FEM_SCHEMA_VERSION,
            name: "weak".into(),
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
    .expect_err("underconstrained body stops before accepted solve");
    assert!(underconstrained
        .message
        .contains("unconstrained DOF rigid-body modes"));
    assert!(fs::read_dir(&scratch).expect("scratch").next().is_none());
    let _ = fs::remove_dir_all(scratch);
}

fn pipeline_control() -> FemPipelineControl {
    FemPipelineControl {
        envelope_mm: 0.1,
        minimum_scaled_jacobian: 1.0e-6,
        maximum_runtime_ms: 1000,
        relative_solver_tolerance: 1.0e-8,
    }
}

fn dummy_runtime() -> FTetWildRuntimeIdentity {
    FTetWildRuntimeIdentity {
        runtime_name: "must-not-run".into(),
        runtime_version: "test".into(),
        source_revision: "test".into(),
        platform: "test".into(),
        arch: "test".into(),
        worker_protocol: "test".into(),
        executable_path: PathBuf::from("/must/not/run"),
        runtime_library_paths: vec![],
        executable_sha256: "sha256:test".into(),
        source_sha256: "sha256:test".into(),
        license_sha256: "sha256:test".into(),
        notice_sha256: "sha256:test".into(),
        transitive_license_inventory_sha256: "sha256:test".into(),
        capabilities: FTetWildRuntimeCapabilities {
            structured_arrays: true,
            tet4: true,
            wide_surface_tags: true,
            isolated_worker: true,
        },
    }
}

fn volume_face(index: usize) -> FemFaceTarget {
    FemFaceTarget {
        schema_version: FEM_SCHEMA_VERSION,
        part_id: "bracket".into(),
        canonical_target_id: format!("face:{index}"),
        durable_target_id: format!("durable:{index}"),
        source_geometry_digest: "sha256:geometry".into(),
    }
}

fn one_tet_volume_mesh() -> FemVolumeMesh {
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
        face_group_targets: (0..4).map(volume_face).collect(),
        source_boundary_digest: "sha256:boundary".into(),
        mesher_identity: FemRuntimeIdentity {
            schema_version: FEM_SCHEMA_VERSION,
            platform: "test".into(),
            architecture: "test".into(),
            library_name: "test".into(),
            library_version: "test".into(),
            library_digest: "sha256:test".into(),
            adapter_protocol_version: 1,
            supported_capabilities: vec!["tet4".into()],
            notice_digest: "sha256:test".into(),
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
    .expect("one Tet4")
}

const AUTHORED_STUDY: &str = r#"
  (model
    (tag-face mounting :faces "bottom" bracket)
    (tag-face load-pad :faces "top" bracket)
    (part bracket (box 10 10 10))
    (analysis bracket-static
      (linear-static :part bracket)
      (question bracket-strength
        :statement "Does the part remain elastic?"
        :decision "accept or revise"
        :acceptance-metrics [stress-limit])
      (acceptance-criterion stress-limit
        :field von-mises-stress :comparison less-than-or-equal
        :limit "200" :unit MPa :requires-convergence false)
      (idealization exact-solid :justification "Exact connected solid." :accepted true)
      (evidence material-source :subject material :source "qualified material record"
        :authority recorded-source :uncertainty-percent 0 :decision-critical true)
      (evidence load-source :subject load :source "accepted load case"
        :authority user-accepted :uncertainty-percent 0 :decision-critical true)
      (evidence support-source :subject support :source "accepted fixture"
        :authority user-accepted :uncertainty-percent 0 :decision-critical true)
      (input-evidence aluminum :evidence material-source)
      (input-evidence applied-load :evidence load-source)
      (input-evidence mounting :evidence support-source)
      (assumption small-strain :category physics
        :statement "Small displacement linear elasticity." :status accepted
        :evidence [material-source load-source support-source])
      (validation-evidence fixture :kind physical-test
        :source "versioned fixture" :result-digest "sha256:fixture")
      (material aluminum :young-modulus 68900MPa :poisson-ratio 0.33
        :density 2700kg-per-m3 :yield-strength 276MPa)
      (volume-mesh :element tet4 :size 5mm)
      (fixed :faces (tag mounting))
      (surface-force :faces (tag load-pad) :total [0N 0N -10N])
      (solve :method sparse-direct)))
"#;

fn tagged_anchors() -> BTreeMap<String, TaggedAnchorBinding> {
    BTreeMap::from([
        ("mounting".to_string(), anchor("mounting", 3)),
        ("load-pad".to_string(), anchor("load-pad", 0)),
    ])
}

fn anchor(name: &str, group: usize) -> TaggedAnchorBinding {
    TaggedAnchorBinding {
        kind: TaggedAnchorKind::Face,
        authored_selector: name.to_string(),
        target: "bracket".to_string(),
        target_ids: vec![format!("target:{group}")],
        durable_target_ids: vec![format!("durable:{group}")],
        canonical_target_ids: vec![format!("face:{group}")],
        alias_ids: vec![],
    }
}

fn tetra_boundary() -> AnalysisBoundarySurface {
    AnalysisBoundarySurface {
        part_id: "bracket".to_string(),
        label: "Bracket".to_string(),
        source_geometry_digest: "sha256:geometry".to_string(),
        vertices: vec![
            [0.0, 0.0, 0.0],
            [10.0, 0.0, 0.0],
            [0.0, 10.0, 0.0],
            [0.0, 0.0, 10.0],
        ],
        triangles: vec![[1, 2, 3], [0, 3, 2], [0, 1, 3], [0, 2, 1]],
        triangle_face_group_indices: vec![0, 1, 2, 3],
        face_groups: (0..4)
            .map(|group| AnalysisBoundaryFaceGroup {
                part_id: "bracket".to_string(),
                target_id: format!("target:{group}"),
                canonical_target_id: format!("face:{group}"),
                durable_target_id: Some(format!("durable:{group}")),
                label: format!("Face {group}"),
                area: if group == 0 {
                    50.0 * 3.0_f64.sqrt()
                } else {
                    50.0
                },
                triangle_count: 1,
            })
            .collect(),
        evidence: AnalysisBoundaryEvidence {
            closed: true,
            manifold: true,
            component_count: 1,
            positive_volume: true,
            boundary_edge_count: 0,
            non_manifold_edge_count: 0,
            winding_mismatch_count: 0,
            signed_volume: 1000.0 / 6.0,
        },
        content_digest: "sha256:boundary".to_string(),
    }
}

fn budgets() -> FemBudgetLimits {
    FemBudgetLimits {
        schema_version: FEM_SCHEMA_VERSION,
        boundary_triangles: 10_000,
        tet4_cells: 500_000,
        nodes: 100_000,
        dofs: 300_000,
        sparse_nonzeros: 20_000_000,
        result_bytes: 64 * 1024 * 1024,
        convergence_levels: 3,
    }
}

fn temp_root() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let root =
        std::env::temp_dir().join(format!("ecky-fem-pipeline-{}-{nonce}", std::process::id()));
    fs::create_dir_all(&root).expect("temp root");
    root
}

struct TestResolver {
    root: PathBuf,
}

impl PathResolver for TestResolver {
    fn app_config_dir(&self) -> PathBuf {
        self.root.join("config")
    }

    fn app_data_dir(&self) -> PathBuf {
        self.root.join("data")
    }

    fn resource_path(&self, _path: &str) -> Option<PathBuf> {
        None
    }
}
