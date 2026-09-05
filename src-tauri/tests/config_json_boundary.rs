use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use ecky_cad_lib::config_store::save_config;
use ecky_cad_lib::contracts::{decode_config, encode_config, Config};
use serde_json::json;

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
                .contains("CONFIG_JSON_FILE")
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

#[test]
fn provider_connection_round_trips_through_canonical_edn_contract() {
    let mut config: Config = serde_json::from_value(json!({
        "engines": [],
        "selectedEngineId": ""
    }))
    .unwrap();
    for provider in ["provider:codex", "provider:agy"] {
        config.connection_type = Some(provider.to_string());
        let encoded = encode_config(&config).unwrap();
        let decoded = decode_config(&encoded).unwrap();
        assert_eq!(decoded.connection_type.as_deref(), Some(provider));
    }
}

#[test]
fn config_store_preserves_exact_encode_failure_detail() {
    let mut config: Config = serde_json::from_value(json!({
        "engines": [],
        "selectedEngineId": ""
    }))
    .unwrap();
    config.connection_type = Some("provider:".to_string());
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "ecky-config-encode-detail-{}-{nonce}",
        std::process::id()
    ));

    let error = save_config(&root, config).unwrap_err();

    assert_eq!(error.message, "config.edn: encode");
    assert!(error
        .details
        .as_deref()
        .is_some_and(|details| details.contains("invalid config field connection-type")));
    assert!(!root.join("config.edn").exists());
    if root.exists() {
        fs::remove_dir_all(root).unwrap();
    }
}
