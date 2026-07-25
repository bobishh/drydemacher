use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use ecky_cad_lib::agent_prompt::agent_language_reference;
use ecky_cad_lib::contracts::GeometryBackend;
use ecky_cad_lib::ecky_language_surface::supported_surface_reference;

const OP_INDEX_START: &str = "<!-- ECKY_GENERATED_OP_INDEX_START -->";
const OP_INDEX_END: &str = "<!-- ECKY_GENERATED_OP_INDEX_END -->";

fn main() {
    let output_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../docs/generated");
    fs::create_dir_all(&output_dir).expect("create generated docs directory");

    for (name, backend) in [
        ("ecky-rust", GeometryBackend::EckyRust),
        ("build123d", GeometryBackend::Build123d),
        ("freecad", GeometryBackend::Freecad),
    ] {
        write_prompt(
            &output_dir.join(format!("ecky-agent-system-prompt-{name}.md")),
            backend,
        );
    }

    write_prompt(
        &output_dir.join("ecky-agent-system-prompt.md"),
        GeometryBackend::EckyRust,
    );
    write_book_operation_index();
}

fn write_prompt(path: &Path, backend: GeometryBackend) {
    let prompt = agent_language_reference(backend);
    fs::write(path, format!("{prompt}\n")).expect("write generated agent prompt");
    eprintln!("Wrote {}", path.display());
}

fn write_book_operation_index() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    let book_path = root.join("docs/books/ecky-ir/ecky-ir-corpus.md");
    let source = fs::read_to_string(&book_path).expect("read canonical Ecky corpus");
    let mut availability: BTreeMap<String, BTreeSet<&'static str>> = BTreeMap::new();

    for (label, backend) in [
        ("ecky-rust", GeometryBackend::EckyRust),
        ("build123d", GeometryBackend::Build123d),
        ("freecad", GeometryBackend::Freecad),
    ] {
        for entry in supported_surface_reference(backend).entries {
            availability.entry(entry.name).or_default().insert(label);
        }
    }

    let mut table = String::from("| Form | Available backends |\n| --- | --- |\n");
    for (name, backends) in availability {
        table.push_str(&format!(
            "| `{name}` | {} |\n",
            backends.into_iter().collect::<Vec<_>>().join(", ")
        ));
    }

    let (prefix, after_start) = source
        .split_once(OP_INDEX_START)
        .expect("canonical corpus operation-index start marker");
    let (_, suffix) = after_start
        .split_once(OP_INDEX_END)
        .expect("canonical corpus operation-index end marker");
    let updated = format!("{prefix}{OP_INDEX_START}\n{table}{OP_INDEX_END}{suffix}");

    fs::write(&book_path, updated).expect("write canonical book operation index");
    eprintln!("Updated {}", book_path.display());
}
