use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use ecky_cad_lib::gmsh_mesher::{
    run_exact_brep_mesher, sha256_file, ExactBrepMesherRuntime, GmshBrepFaceSignature,
    GmshBrepMeshRequest, GmshMesherControl, GmshRuntimeIdentity,
};
use ecky_cad_lib::netgen_mesher::{probe_default_netgen_runtime, run_netgen_exact_brep};
use ecky_fem::{FemFaceTarget, FEM_SCHEMA_VERSION};

#[test]
fn exact_brep_netgen_run_preserves_required_surface_and_runtime_identity() {
    let runtime = match probe_default_netgen_runtime() {
        Ok(runtime) => runtime,
        Err(error) => {
            eprintln!("Netgen unavailable; exact-BRep fallback proof is platform-gated: {error}");
            return;
        }
    };
    let scratch = temp_root("netgen-exact-brep");
    std::fs::create_dir_all(&scratch).expect("scratch");
    let request = exact_sphere_request("exact-sphere-netgen");

    let mesh = run_netgen_exact_brep(&runtime, &request, &scratch, &AtomicBool::new(false))
        .expect("exact STEP should produce checked fallback Tet4 mesh");

    assert!(!mesh.nodes.is_empty());
    assert!(!mesh.cells.is_empty());
    assert!(!mesh.boundary_triangles.is_empty());
    assert_eq!(mesh.face_group_count, 1);
    assert_eq!(mesh.face_group_targets[0].durable_target_id, "sphere-face");
    assert_eq!(mesh.mesher_identity.library_name, "Netgen OCC");
    assert!(mesh.mesher_identity.library_digest.starts_with("sha256:"));
    assert_eq!(mesh.meshing_evidence.deterministic_thread_count, 4);
    assert!(mesh.content_digest.starts_with("sha256:"));

    std::fs::remove_dir_all(scratch).expect("scratch cleanup");
}

#[test]
fn exact_brep_pipeline_falls_back_to_netgen_after_hxt_meshing_failure() {
    let netgen = match probe_default_netgen_runtime() {
        Ok(runtime) => runtime,
        Err(error) => {
            eprintln!("Netgen unavailable; fallback routing proof is platform-gated: {error}");
            return;
        }
    };
    let false_executable = PathBuf::from("/usr/bin/false");
    let runtime = ExactBrepMesherRuntime {
        gmsh: GmshRuntimeIdentity {
            executable_sha256: sha256_file(&false_executable).expect("false executable digest"),
            executable_path: false_executable,
            version: "forced-hxt-failure".into(),
            platform: std::env::consts::OS.into(),
            architecture: std::env::consts::ARCH.into(),
        },
        netgen: Some(netgen),
    };
    let scratch = temp_root("exact-brep-fallback");
    std::fs::create_dir_all(&scratch).expect("scratch");
    let request = exact_sphere_request("exact-sphere-fallback");

    let mesh = run_exact_brep_mesher(&runtime, &request, &scratch, &AtomicBool::new(false))
        .expect("failed HXT attempt should route to exact-BRep Netgen");

    assert_eq!(mesh.mesher_identity.library_name, "Netgen OCC");
    assert_eq!(mesh.face_group_count, 1);
    assert_eq!(mesh.face_group_targets[0].durable_target_id, "sphere-face");
    std::fs::remove_dir_all(scratch).expect("scratch cleanup");
}

fn exact_sphere_request(request_id: &str) -> GmshBrepMeshRequest {
    let step_path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fem/exact-sphere.step");
    GmshBrepMeshRequest {
        schema_version: FEM_SCHEMA_VERSION,
        request_id: request_id.into(),
        step_sha256: sha256_file(&step_path).expect("STEP digest"),
        step_path,
        source_geometry_digest: "sha256:exact-sphere".into(),
        source_boundary_digest: "sha256:exact-sphere-boundary".into(),
        face_signatures: vec![GmshBrepFaceSignature {
            face_index: 0,
            area_mm2: 4.0 * std::f64::consts::PI * 100.0,
            center_mm: [0.0, 0.0, 0.0],
        }],
        face_group_targets: vec![FemFaceTarget {
            schema_version: FEM_SCHEMA_VERSION,
            part_id: "sphere".into(),
            canonical_target_id: "sphere:face:0".into(),
            durable_target_id: "sphere-face".into(),
            source_geometry_digest: "sha256:exact-sphere".into(),
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
    }
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
