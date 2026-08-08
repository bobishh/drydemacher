use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use ecky_cad_lib::ecky_cad_host::analysis_boundary::{
    AnalysisBoundaryEvidence, AnalysisBoundaryFaceGroup, AnalysisBoundarySurface,
};
use ecky_cad_lib::fem_mesher::{
    run_ftetwild_worker, FTetWildRuntimeCapabilities, FTetWildRuntimeIdentity,
    FTetWildWorkerControl, FTetWildWorkerRequest, FTETWILD_WORKER_PROTOCOL,
};
use ecky_fem::{
    FemBudgetLimits, FemElementKind, FemFaceTarget, FemLocalRefinement, FemMeshControl,
    FEM_SCHEMA_VERSION,
};

#[test]
fn fem_meshing_path_contains_no_compatibility_or_remote_fallback() {
    let rust_adapter = include_str!("../src/fem_mesher.rs").to_ascii_lowercase();
    let pipeline = include_str!("../src/services/fem.rs").to_ascii_lowercase();
    let native_worker = include_str!("../native/ftetwild_worker.cpp").to_ascii_lowercase();
    for forbidden in [
        "tetgen",
        "gmsh",
        "freecad",
        "python",
        "calculix",
        "import-stl",
        "http://",
    ] {
        assert!(
            !rust_adapter.contains(forbidden),
            "Rust mesher contains fallback token {forbidden}"
        );
        assert!(
            !pipeline.contains(forbidden),
            "FEM pipeline contains fallback token {forbidden}"
        );
        assert!(
            !native_worker.contains(forbidden),
            "native worker contains fallback token {forbidden}"
        );
    }
}

#[test]
fn protocol_rejects_malformed_arrays_indices_budget_and_element_order() {
    let mut malformed = valid_request();
    malformed.vertices_mm.pop();
    assert!(malformed
        .validate()
        .unwrap_err()
        .to_string()
        .contains("verticesMm"));

    let mut out_of_range = valid_request();
    out_of_range.triangles[0] = 99;
    assert!(out_of_range
        .validate()
        .unwrap_err()
        .to_string()
        .contains("out-of-range"));

    let mut over_budget = valid_request();
    over_budget.control.maximum_boundary_triangles = 3;
    assert!(over_budget
        .validate()
        .unwrap_err()
        .to_string()
        .contains("budget"));

    let mut high_order = valid_request();
    high_order.control.element_order = 2;
    assert!(high_order
        .validate()
        .unwrap_err()
        .to_string()
        .contains("elementOrder"));
}

#[test]
fn exact_boundary_and_durable_refinement_lower_to_wide_worker_tags() {
    let boundary = boundary_surface();
    let control = FemMeshControl {
        schema_version: FEM_SCHEMA_VERSION,
        element_kind: FemElementKind::Tet4,
        global_size_mm: 0.5,
        local_refinements: vec![FemLocalRefinement {
            schema_version: FEM_SCHEMA_VERSION,
            faces: vec![FemFaceTarget {
                schema_version: FEM_SCHEMA_VERSION,
                part_id: "body".to_string(),
                canonical_target_id: "body:face:3".to_string(),
                durable_target_id: "body:stable:3".to_string(),
                source_geometry_digest: "sha256:geometry".to_string(),
            }],
            size_mm: 0.2,
        }],
        budgets: FemBudgetLimits {
            schema_version: FEM_SCHEMA_VERSION,
            boundary_triangles: 100,
            tet4_cells: 200,
            nodes: 150,
            dofs: 450,
            sparse_nonzeros: 10_000,
            result_bytes: 64 * 1024,
            convergence_levels: 3,
        },
    };

    let request = FTetWildWorkerRequest::from_analysis_boundary(
        "request-1",
        &boundary,
        &control,
        0.001,
        1.0e-6,
        5_000,
    )
    .expect("lower exact tagged boundary");
    assert_eq!(request.source_boundary_digest, boundary.content_digest);
    assert_eq!(request.face_group_count, 4);
    assert_eq!(request.triangle_face_group_indices, vec![0, 1, 2, 3]);
    assert_eq!(
        request.control.local_refinements[0].face_group_indices,
        vec![3]
    );
    assert_eq!(
        request.control.local_refinements[0].target_edge_length_mm,
        0.2
    );

    let mut open = boundary.clone();
    open.evidence.closed = false;
    let error = FTetWildWorkerRequest::from_analysis_boundary(
        "open", &open, &control, 0.001, 1.0e-6, 5_000,
    )
    .expect_err("open surface");
    assert!(error.to_string().contains("closed=false"));

    let mut non_manifold = boundary;
    non_manifold.evidence.manifold = false;
    let error = FTetWildWorkerRequest::from_analysis_boundary(
        "non-manifold",
        &non_manifold,
        &control,
        0.001,
        1.0e-6,
        5_000,
    )
    .expect_err("non-manifold surface");
    assert!(error.to_string().contains("manifold=false"));
}

#[cfg(unix)]
#[test]
fn isolated_worker_returns_validated_mesh_and_propagates_raw_stderr_on_crash() {
    let root = temp_root("worker");
    let scratch = root.join("scratch");
    fs::create_dir_all(&scratch).expect("scratch");
    let worker = root.join("worker.sh");
    write_executable(
        &worker,
        r#"#!/bin/sh
response=""
while [ "$#" -gt 0 ]; do
  if [ "$1" = "--response" ]; then
    shift
    response="$1"
  fi
  shift
done
printf '%s' '{"schemaVersion":1,"workerProtocol":"ecky-ftetwild-worker-v1","requestId":"request-1","nodesMm":[0,0,0,1,0,0,0,1,0,0,0,1],"tet4Cells":[0,1,2,3],"boundaryTriangles":[1,3,2,0,2,3,0,3,1,0,1,2],"boundaryFaceGroupIndices":[0,1,2,3],"faceGroupCount":4,"insertionCount":0,"maximumBoundaryDeviationMm":0,"threadCount":1}' > "$response"
"#,
    );
    let identity = runtime_identity(worker.clone());
    let mesh = run_ftetwild_worker(
        &identity,
        &valid_request(),
        &scratch,
        &AtomicBool::new(false),
    )
    .expect("valid worker mesh");
    assert_eq!(mesh.cells.len(), 1);
    assert_eq!(mesh.boundary_face_group_indices.len(), 4);
    assert_eq!(
        mesh.face_group_targets[3].durable_target_id,
        "body:stable:3"
    );
    assert_eq!(mesh.source_boundary_digest, "sha256:boundary");

    write_executable(
        &worker,
        "#!/bin/sh\necho 'native mesher exploded' >&2\nexit 23\n",
    );
    let error = run_ftetwild_worker(
        &identity,
        &valid_request(),
        &scratch,
        &AtomicBool::new(false),
    )
    .expect_err("worker crash");
    let text = error.to_string();
    assert!(text.contains("exit code 23"));
    assert!(text.contains("native mesher exploded"));
    assert!(fs::read_dir(&scratch)
        .expect("scratch listing")
        .next()
        .is_none());

    write_executable(
        &worker,
        r#"#!/bin/sh
response=""
while [ "$#" -gt 0 ]; do
  if [ "$1" = "--response" ]; then shift; response="$1"; fi
  shift
done
printf '%s' '{"schemaVersion":1,"workerProtocol":"ecky-ftetwild-worker-v1","requestId":"request-1","nodesMm":[0,0],"tet4Cells":[0,1,2,3],"boundaryTriangles":[1,3,2,0,2,3,0,3,1,0,1,2],"boundaryFaceGroupIndices":[0,1,2,3],"faceGroupCount":4,"insertionCount":0,"maximumBoundaryDeviationMm":0,"threadCount":1}' > "$response"
"#,
    );
    let error = run_ftetwild_worker(
        &identity,
        &valid_request(),
        &scratch,
        &AtomicBool::new(false),
    )
    .expect_err("malformed worker array");
    assert!(
        error
            .to_string()
            .contains("malformed typed-array cardinality"),
        "{error:?}"
    );

    write_executable(
        &worker,
        r#"#!/bin/sh
response=""
while [ "$#" -gt 0 ]; do
  if [ "$1" = "--response" ]; then shift; response="$1"; fi
  shift
done
printf '%s' '{"schemaVersion":1,"workerProtocol":"ecky-ftetwild-worker-v1","requestId":"request-1","nodesMm":[0,0,0,1,0,0,0,1,0,0,0,1],"tet4Cells":[0,1,2,99],"boundaryTriangles":[1,3,2,0,2,3,0,3,1,0,1,2],"boundaryFaceGroupIndices":[0,1,2,3],"faceGroupCount":4,"insertionCount":0,"maximumBoundaryDeviationMm":0,"threadCount":1}' > "$response"
"#,
    );
    let error = run_ftetwild_worker(
        &identity,
        &valid_request(),
        &scratch,
        &AtomicBool::new(false),
    )
    .expect_err("out-of-range worker cell");
    assert!(error.to_string().contains("out-of-range"));

    write_executable(&worker, "#!/bin/sh\nexec sleep 30\n");
    let cancelled = Arc::new(AtomicBool::new(false));
    let trigger = cancelled.clone();
    let canceller = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(50));
        trigger.store(true, Ordering::Release);
    });
    let started = Instant::now();
    let error = run_ftetwild_worker(&identity, &valid_request(), &scratch, cancelled.as_ref())
        .expect_err("worker cancellation");
    canceller.join().expect("cancellation trigger");
    assert!(error.to_string().contains("cancelled"));
    assert!(started.elapsed() < Duration::from_secs(2));
    assert!(fs::read_dir(&scratch)
        .expect("scratch after cancellation")
        .next()
        .is_none());

    let _ = fs::remove_dir_all(root);
}

#[test]
fn native_worker_tetrahedralizes_tagged_surface_and_localizes_refinement() {
    let Some(worker) = std::env::var_os("ECKY_FTETWILD_WORKER") else {
        eprintln!("ECKY_FTETWILD_WORKER unset; native fTetWild proof is platform-gated");
        return;
    };
    let worker = PathBuf::from(worker);
    assert!(worker.is_file(), "native worker path must exist");
    let root = temp_root("native-worker");
    let scratch = root.join("scratch");
    fs::create_dir_all(&scratch).expect("scratch");
    let identity = runtime_identity(worker);

    let mut global_request = valid_request();
    global_request.request_id = "native-global".to_string();
    global_request
        .vertices_mm
        .iter_mut()
        .for_each(|value| *value *= 10.0);
    global_request.control.target_edge_length_mm = 5.0;
    global_request.control.envelope_mm = 0.1;
    global_request.control.maximum_boundary_triangles = 10_000;
    global_request.control.maximum_nodes = 100_000;
    global_request.control.maximum_tet4_cells = 500_000;
    global_request.control.maximum_result_bytes = 64 * 1024 * 1024;
    global_request.control.maximum_runtime_ms = 120_000;
    let global = run_ftetwild_worker(
        &identity,
        &global_request,
        &scratch,
        &AtomicBool::new(false),
    )
    .expect("native global Tet4 mesh");

    let mut local_request = global_request.clone();
    local_request.request_id = "native-local".to_string();
    local_request.control.local_refinements =
        vec![ecky_cad_lib::fem_mesher::FTetWildWorkerLocalRefinement {
            face_group_indices: vec![3],
            target_edge_length_mm: 1.0,
        }];
    let local = run_ftetwild_worker(&identity, &local_request, &scratch, &AtomicBool::new(false))
        .expect("native locally refined Tet4 mesh");

    assert!(!global.cells.is_empty());
    assert!(!local.cells.is_empty());
    assert_eq!(
        global
            .boundary_face_group_indices
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>(),
        [0, 1, 2, 3].into_iter().collect()
    );
    assert_eq!(global.meshing_evidence.inserted_source_triangle_count, 4);
    assert!(global.meshing_evidence.maximum_boundary_deviation_mm <= 0.1);
    let global_target_facets = global
        .boundary_face_group_indices
        .iter()
        .filter(|group| **group == 3)
        .count();
    let local_target_facets = local
        .boundary_face_group_indices
        .iter()
        .filter(|group| **group == 3)
        .count();
    assert!(
        local_target_facets > global_target_facets,
        "target group refinement must increase local boundary resolution: global={global_target_facets}, local={local_target_facets}"
    );
    assert!(local.nodes.len() > global.nodes.len());

    let _ = fs::remove_dir_all(root);
}

fn valid_request() -> FTetWildWorkerRequest {
    FTetWildWorkerRequest {
        schema_version: FEM_SCHEMA_VERSION,
        worker_protocol: FTETWILD_WORKER_PROTOCOL.to_string(),
        request_id: "request-1".to_string(),
        source_boundary_digest: "sha256:boundary".to_string(),
        vertices_mm: vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0],
        triangles: vec![1, 2, 3, 0, 3, 2, 0, 1, 3, 0, 2, 1],
        triangle_face_group_indices: vec![0, 1, 2, 3],
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
        control: FTetWildWorkerControl {
            element_order: 1,
            target_edge_length_mm: 0.5,
            envelope_mm: 0.001,
            minimum_scaled_jacobian: 1.0e-6,
            deterministic_thread_count: 1,
            allow_hole_filling: false,
            maximum_boundary_triangles: 4,
            maximum_nodes: 100,
            maximum_tet4_cells: 100,
            maximum_result_bytes: 64 * 1024,
            maximum_runtime_ms: 5_000,
            local_refinements: vec![],
        },
    }
}

fn boundary_surface() -> AnalysisBoundarySurface {
    AnalysisBoundarySurface {
        part_id: "body".to_string(),
        label: "Body".to_string(),
        source_geometry_digest: "sha256:geometry".to_string(),
        vertices: vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
        ],
        triangles: vec![[1, 2, 3], [0, 3, 2], [0, 1, 3], [0, 2, 1]],
        triangle_face_group_indices: vec![0, 1, 2, 3],
        face_groups: (0..4)
            .map(|index| AnalysisBoundaryFaceGroup {
                part_id: "body".to_string(),
                target_id: format!("body:face:{index}"),
                canonical_target_id: format!("body:face:{index}"),
                durable_target_id: Some(format!("body:stable:{index}")),
                label: format!("Face {index}"),
                area: 0.5,
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
            signed_volume: 1.0 / 6.0,
        },
        content_digest: "sha256:boundary".to_string(),
    }
}

fn runtime_identity(executable_path: PathBuf) -> FTetWildRuntimeIdentity {
    FTetWildRuntimeIdentity {
        runtime_name: "fTetWild".to_string(),
        runtime_version: "test".to_string(),
        source_revision: "0123456789abcdef0123456789abcdef01234567".to_string(),
        platform: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        worker_protocol: FTETWILD_WORKER_PROTOCOL.to_string(),
        executable_path,
        runtime_library_paths: vec![],
        executable_sha256: "sha256:binary".to_string(),
        source_sha256: "sha256:source".to_string(),
        license_sha256: "sha256:license".to_string(),
        notice_sha256: "sha256:notice".to_string(),
        transitive_license_inventory_sha256: "sha256:inventory".to_string(),
        capabilities: FTetWildRuntimeCapabilities {
            structured_arrays: true,
            tet4: true,
            wide_surface_tags: true,
            isolated_worker: true,
        },
    }
}

#[cfg(unix)]
fn write_executable(path: &Path, source: &str) {
    use std::os::unix::fs::PermissionsExt;
    fs::write(path, source).expect("write worker");
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).expect("worker permissions");
}

fn temp_root(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let root =
        std::env::temp_dir().join(format!("ecky-fem-{label}-{}-{nonce}", std::process::id()));
    fs::create_dir_all(&root).expect("create temp root");
    root
}
