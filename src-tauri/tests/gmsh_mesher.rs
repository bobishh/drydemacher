use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::AtomicBool;

use ecky_cad_lib::gmsh_mesher::{
    probe_gmsh_runtime, run_gmsh_hxt, sha256_file, GmshBrepFaceSignature, GmshBrepMeshRequest,
    GmshMesherControl,
};
use ecky_fem::{FemFaceTarget, FEM_SCHEMA_VERSION};

#[test]
fn exact_brep_hxt_run_preserves_required_surface_and_runtime_identity() {
    let executable = std::env::var_os("ECKY_GMSH_EXECUTABLE")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("gmsh"));
    if Command::new(&executable).arg("--version").output().is_err() {
        eprintln!("Gmsh unavailable; exact-BRep native proof is platform-gated");
        return;
    }
    let runtime = probe_gmsh_runtime(&executable).expect("compatible Gmsh HXT runtime");
    let scratch = temp_root("gmsh-exact-brep");
    std::fs::create_dir_all(&scratch).expect("scratch");
    let step_path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fem/exact-sphere.step");
    let request = GmshBrepMeshRequest {
        schema_version: FEM_SCHEMA_VERSION,
        request_id: "exact-sphere-hxt".to_string(),
        step_sha256: sha256_file(&step_path).expect("STEP digest"),
        step_path,
        source_geometry_digest: "sha256:exact-sphere".to_string(),
        source_boundary_digest: "sha256:exact-sphere-boundary".to_string(),
        face_signatures: vec![GmshBrepFaceSignature {
            face_index: 0,
            area_mm2: 4.0 * std::f64::consts::PI * 100.0,
            center_mm: [0.0, 0.0, 0.0],
        }],
        face_group_targets: vec![FemFaceTarget {
            schema_version: FEM_SCHEMA_VERSION,
            part_id: "sphere".to_string(),
            canonical_target_id: "sphere:face:0".to_string(),
            durable_target_id: "sphere-face".to_string(),
            source_geometry_digest: "sha256:exact-sphere".to_string(),
        }],
        required_face_group_indices: vec![0],
        control: GmshMesherControl {
            global_size_mm: 8.0,
            minimum_scaled_jacobian: 0.001,
            maximum_face_area_relative_error: 0.05,
            maximum_face_centroid_deviation_mm: 4.0,
            thread_count: 4,
            maximum_nodes: 100_000,
            maximum_tet4_cells: 500_000,
            maximum_boundary_triangles: 100_000,
            maximum_result_bytes: 64 * 1024 * 1024,
            maximum_runtime_ms: 30_000,
            local_refinements: vec![],
        },
    };

    let mesh = run_gmsh_hxt(&runtime, &request, &scratch, &AtomicBool::new(false))
        .expect("exact STEP should produce checked Tet4 mesh");

    assert!(!mesh.nodes.is_empty());
    assert!(!mesh.cells.is_empty());
    assert!(!mesh.boundary_triangles.is_empty());
    assert_eq!(mesh.face_group_count, 1);
    assert_eq!(mesh.face_group_targets[0].durable_target_id, "sphere-face");
    assert_eq!(mesh.mesher_identity.library_name, "Gmsh HXT");
    assert_eq!(mesh.meshing_evidence.deterministic_thread_count, 4);
    assert!(mesh.content_digest.starts_with("sha256:"));
    assert_ne!(mesh.content_digest, request.source_boundary_digest);

    let _ = std::fs::remove_dir_all(scratch);
}

#[test]
fn exact_brep_request_rejects_step_changed_after_resolution() {
    let executable = std::env::var_os("ECKY_GMSH_EXECUTABLE")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("gmsh"));
    let Ok(runtime) = probe_gmsh_runtime(&executable) else {
        eprintln!("Gmsh unavailable; exact-BRep native proof is platform-gated");
        return;
    };
    let scratch = temp_root("gmsh-step-identity");
    std::fs::create_dir_all(&scratch).expect("scratch");
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fem/exact-sphere.step");
    let step_path = scratch.join("mutable.step");
    std::fs::copy(&source, &step_path).expect("STEP fixture copy");
    let expected_digest = sha256_file(&step_path).expect("initial STEP digest");
    let request = GmshBrepMeshRequest {
        schema_version: FEM_SCHEMA_VERSION,
        request_id: "changed-step".to_string(),
        step_path: step_path.clone(),
        step_sha256: expected_digest,
        source_geometry_digest: "sha256:exact-sphere".to_string(),
        source_boundary_digest: "sha256:exact-sphere-boundary".to_string(),
        face_signatures: vec![GmshBrepFaceSignature {
            face_index: 0,
            area_mm2: 4.0 * std::f64::consts::PI * 100.0,
            center_mm: [0.0, 0.0, 0.0],
        }],
        face_group_targets: vec![FemFaceTarget {
            schema_version: FEM_SCHEMA_VERSION,
            part_id: "sphere".to_string(),
            canonical_target_id: "sphere:face:0".to_string(),
            durable_target_id: "sphere-face".to_string(),
            source_geometry_digest: "sha256:exact-sphere".to_string(),
        }],
        required_face_group_indices: vec![0],
        control: GmshMesherControl {
            global_size_mm: 2.0,
            minimum_scaled_jacobian: 0.001,
            maximum_face_area_relative_error: 0.05,
            maximum_face_centroid_deviation_mm: 4.0,
            thread_count: 4,
            maximum_nodes: 100_000,
            maximum_tet4_cells: 500_000,
            maximum_boundary_triangles: 100_000,
            maximum_result_bytes: 64 * 1024 * 1024,
            maximum_runtime_ms: 30_000,
            local_refinements: vec![],
        },
    };
    std::fs::write(&step_path, b"changed").expect("mutate STEP after resolution");

    let error = run_gmsh_hxt(&runtime, &request, &scratch, &AtomicBool::new(false))
        .expect_err("changed exact STEP must fail before meshing");
    assert!(error.to_string().contains("Exact STEP changed"));
    let _ = std::fs::remove_dir_all(scratch);
}

fn temp_root(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "ecky-{label}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ))
}
