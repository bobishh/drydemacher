use std::fs;
use std::path::{Path, PathBuf};

fn rust_sources(root: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    for entry in fs::read_dir(root).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            paths.extend(rust_sources(&path));
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            paths.push(path);
        }
    }
    paths
}

fn production_source(source: &str) -> &str {
    source
        .split("\n#[cfg(test)]\nmod tests {")
        .next()
        .unwrap_or(source)
}

#[test]
fn config_json_runtime_access_is_limited_to_legacy_boundary() {
    let source_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let offenders: Vec<_> = rust_sources(&source_root)
        .into_iter()
        .filter_map(|path| {
            let source = fs::read_to_string(&path).unwrap();
            production_source(&source)
                .contains("config.json")
                .then_some(path)
        })
        .collect();

    assert_eq!(
        offenders,
        vec![source_root.join("config_store.rs")],
        "only config_store may detect, import, or clean up legacy config.json"
    );

    let store = fs::read_to_string(source_root.join("config_store.rs")).unwrap();
    assert!(store.contains("let json = config_dir.join(CONFIG_JSON_FILE)"));
    assert!(store.contains("serde_json::from_str(&raw)"));
    assert!(store.contains("delete_legacy_json"));
}
