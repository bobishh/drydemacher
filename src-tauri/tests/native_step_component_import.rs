//! Dedicated contract tests for the host-side STEP asset seam.

use std::fs;
use std::path::PathBuf;

use ecky_cad_lib::component_step_runtime::{
    lower_step_assets_to_compiler_source, merge_step_geometry_provenance, validate_step_asset,
    StepAsset,
};
use ecky_cad_lib::contracts::{ComponentCoordinate, GeometryProvenance, GeometryRepresentation};
use sha2::{Digest, Sha256};

fn temporary_step(bytes: &[u8]) -> (PathBuf, String) {
    let path = std::env::temp_dir().join(format!(
        "ecky-native-step-component-{}.step",
        uuid::Uuid::new_v4()
    ));
    fs::write(&path, bytes).expect("write STEP fixture");
    let digest = format!("sha256:{:x}", Sha256::digest(bytes));
    (path, digest)
}

fn asset(
    path: PathBuf,
    payload_digest: String,
    representation: GeometryRepresentation,
) -> StepAsset {
    StepAsset {
        coordinate: ComponentCoordinate {
            package_id: "fixture.bracket".to_string(),
            version: "1.0.0".to_string(),
            component_id: "bracket".to_string(),
        },
        alias: "mount".to_string(),
        path,
        payload_digest,
        geometry_provenance: GeometryProvenance {
            representation,
            source_mesh_digests: Vec::new(),
            closed: None,
            boundary_or_non_manifold_edge_count: None,
        },
    }
}

#[test]
fn verified_step_asset_lowers_to_ephemeral_zero_argument_import_step_leaf() {
    let (path, digest) = temporary_step(b"locked STEP bytes");
    let asset = asset(path.clone(), digest, GeometryRepresentation::AnalyticBrep);
    let authored = r#"
        (import-component "fixture.bracket" :version "1.0.0" :component "bracket" :as mount)
        (model (part body (difference (mount) (box 2 2 2))))
    "#;

    let compiler_source = lower_step_assets_to_compiler_source(authored, &[asset])
        .expect("lower verified static asset");

    assert!(compiler_source.contains("(define-component mount () (import-step"));
    assert!(!compiler_source.contains("(import-component"));
    assert!(compiler_source.contains("(difference (mount) (box 2 2 2))"));
    assert!(compiler_source.contains(path.to_string_lossy().as_ref()));
    assert!(authored.contains("fixture.bracket"));
    assert!(!authored.contains(path.to_string_lossy().as_ref()));

    let program = ecky_cad_lib::ecky_scheme::compile_to_core_program(&compiler_source)
        .expect("static import-step leaf compiles through the normal compiler");
    let plan = ecky_cad_lib::ecky_cad_host::direct_occt::plan_core_program(&program)
        .expect("static import-step leaf plans for Direct OCCT");
    assert!(plan.parts[0].commands.iter().any(|command| matches!(
        command.op,
        ecky_cad_lib::ecky_cad_host::direct_occt::OcctOp::ImportStep
    )));

    fs::remove_file(path).ok();
}

#[test]
fn mutated_or_unprovenanced_step_fails_before_lowering_or_native_execution() {
    let (path, digest) = temporary_step(b"locked STEP bytes");
    let mut asset = asset(path.clone(), digest, GeometryRepresentation::AnalyticBrep);
    fs::write(&path, b"mutated STEP bytes").expect("mutate fixture");
    let error = validate_step_asset(&asset).expect_err("mutation blocks admission");
    assert!(
        error.message.contains("digest mismatch"),
        "{}",
        error.message
    );

    asset.payload_digest = format!("sha256:{:x}", Sha256::digest(b"mutated STEP bytes"));
    asset.geometry_provenance.representation = GeometryRepresentation::MeshNative;
    let error =
        validate_step_asset(&asset).expect_err("mesh provenance is not STEP admission evidence");
    assert!(error.message.contains("repack"), "{}", error.message);
    fs::remove_file(path).ok();
}

#[test]
fn static_step_alias_rejects_positional_and_keyword_geometry_arguments() {
    let (path, digest) = temporary_step(b"locked STEP bytes");
    let asset = asset(path.clone(), digest, GeometryRepresentation::AnalyticBrep);
    let authored = r#"
        (import-component "fixture.bracket" :version "1.0.0" :component "bracket" :as mount)
        (model (part body (mount :width 20)))
    "#;
    let error = lower_step_assets_to_compiler_source(authored, &[asset])
        .expect_err("static components cannot accept overrides");
    assert!(error.message.contains("zero-argument"), "{}", error.message);
    fs::remove_file(path).ok();
}

#[test]
fn package_representation_evidence_merges_conservatively() {
    let (analytic_path, analytic_digest) = temporary_step(b"analytic");
    let (faceted_path, faceted_digest) = temporary_step(b"faceted");
    let analytic = asset(
        analytic_path.clone(),
        analytic_digest,
        GeometryRepresentation::AnalyticBrep,
    );
    let faceted = asset(
        faceted_path.clone(),
        faceted_digest,
        GeometryRepresentation::FacetedPolyBrep,
    );

    assert_eq!(
        merge_step_geometry_provenance(
            GeometryRepresentation::AnalyticBrep,
            std::slice::from_ref(&analytic)
        )
        .representation,
        GeometryRepresentation::AnalyticBrep
    );
    assert_eq!(
        merge_step_geometry_provenance(
            GeometryRepresentation::FacetedPolyBrep,
            std::slice::from_ref(&faceted)
        )
        .representation,
        GeometryRepresentation::FacetedPolyBrep
    );
    assert_eq!(
        merge_step_geometry_provenance(GeometryRepresentation::AnalyticBrep, &[faceted])
            .representation,
        GeometryRepresentation::Hybrid
    );

    fs::remove_file(analytic_path).ok();
    fs::remove_file(faceted_path).ok();
}
