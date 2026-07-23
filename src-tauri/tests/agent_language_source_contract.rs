use ecky_cad_lib::agent_prompt::agent_language_reference;
use ecky_cad_lib::commands::generation::design_system_prompt;
use ecky_cad_lib::ecky_language_surface::supported_surface_reference;
use ecky_cad_lib::models::{GeometryBackend, SourceLanguage};
use std::fs;
use std::path::PathBuf;

const BOOK_SOURCE: &str = include_str!("../../public/docs/ecky-ir.md");
const AGENT_REFERENCE_START: &str = "<!-- ECKY_AGENT_REFERENCE_START -->";
const AGENT_REFERENCE_END: &str = "<!-- ECKY_AGENT_REFERENCE_END -->";

fn canonical_agent_reference() -> &'static str {
    BOOK_SOURCE
        .split_once(AGENT_REFERENCE_START)
        .and_then(|(_, tail)| tail.split_once(AGENT_REFERENCE_END))
        .map(|(body, _)| body.trim())
        .expect("canonical book must contain one agent-reference projection")
}

#[test]
fn api_prompt_projects_the_canonical_book_agent_reference() {
    let canonical = canonical_agent_reference();
    let prompt = agent_language_reference(GeometryBackend::EckyRust);

    assert!(prompt.contains(canonical));
    assert!(canonical.contains("`mesh` and `polyhedron`"));
    assert!(canonical.contains("`heightfield`"));
    assert!(canonical.contains("single perspective image"));
    assert!(canonical.contains("faceted poly-BRep"));
}

#[test]
fn api_design_generation_uses_the_shared_language_reference_verbatim() {
    let shared = agent_language_reference(GeometryBackend::EckyRust);
    let design = design_system_prompt(SourceLanguage::EckyIrV0, GeometryBackend::EckyRust);

    assert!(design.contains(&shared));
}

#[test]
fn committed_prompt_artifacts_are_fresh() {
    let generated_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../docs/generated");
    for (filename, backend) in [
        (
            "ecky-agent-system-prompt-ecky-rust.md",
            GeometryBackend::EckyRust,
        ),
        (
            "ecky-agent-system-prompt-build123d.md",
            GeometryBackend::Build123d,
        ),
        (
            "ecky-agent-system-prompt-freecad.md",
            GeometryBackend::Freecad,
        ),
    ] {
        let generated = fs::read_to_string(generated_dir.join(filename)).expect("generated prompt");
        let generated = generated.trim_end();
        let expected = agent_language_reference(backend);
        let expected = expected.trim_end();
        if generated != expected {
            let offset = generated
                .bytes()
                .zip(expected.bytes())
                .position(|(left, right)| left != right)
                .unwrap_or_else(|| generated.len().min(expected.len()));
            panic!(
                "{filename} stale at byte {offset}; generated_len={}, expected_len={}",
                generated.len(),
                expected.len()
            );
        }
    }
}

#[test]
fn canonical_book_operation_index_covers_the_surface_registry() {
    for backend in [
        GeometryBackend::EckyRust,
        GeometryBackend::Build123d,
        GeometryBackend::Freecad,
    ] {
        for entry in supported_surface_reference(backend).entries {
            let row_key = format!("| `{}` |", entry.name);
            assert!(
                BOOK_SOURCE.contains(&row_key),
                "canonical book operation index missing `{}`",
                entry.name
            );
        }
    }
}
