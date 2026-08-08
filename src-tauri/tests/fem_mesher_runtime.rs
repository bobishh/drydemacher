use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use ecky_cad_lib::fem_mesher::{
    probe_ftetwild_runtime, FTetWildRuntimeRequirement, FTETWILD_RUNTIME_SCHEMA_VERSION,
    FTETWILD_WORKER_PROTOCOL,
};
use serde_json::json;
use sha2::{Digest, Sha256};

#[test]
fn runtime_probe_accepts_only_the_manifested_native_worker_and_legal_payload() {
    let root = temp_root("valid");
    let requirement = fixture_runtime(&root);

    let identity = probe_ftetwild_runtime(&root, &requirement).expect("valid runtime");
    assert_eq!(identity.runtime_version, requirement.runtime_version);
    assert_eq!(identity.source_revision, requirement.source_revision);
    assert_eq!(identity.worker_protocol, FTETWILD_WORKER_PROTOCOL);
    assert_eq!(identity.platform, std::env::consts::OS);
    assert_eq!(identity.arch, std::env::consts::ARCH);
    assert!(identity.capabilities.structured_arrays);
    assert!(identity.capabilities.tet4);
    assert!(identity.capabilities.wide_surface_tags);
    assert!(identity.capabilities.isolated_worker);
    assert_eq!(identity.runtime_library_paths.len(), 1);
    assert!(identity.runtime_library_paths[0].ends_with("libgmp.test"));

    fs::write(root.join("bin/ftetwild-worker"), b"tampered").expect("tamper worker");
    let error = probe_ftetwild_runtime(&root, &requirement).expect_err("digest mismatch");
    assert!(error.to_string().contains("executable digest mismatch"));

    let runtime_library = temp_root("tampered-runtime-library");
    let requirement = fixture_runtime(&runtime_library);
    fs::write(runtime_library.join("lib/libgmp.test"), b"tampered").expect("tamper library");
    let error = probe_ftetwild_runtime(&runtime_library, &requirement)
        .expect_err("runtime library digest mismatch");
    assert!(error
        .to_string()
        .contains("runtime library 0 digest mismatch"));

    let _ = fs::remove_dir_all(root);
    let _ = fs::remove_dir_all(runtime_library);
}

#[test]
fn runtime_probe_rejects_missing_source_license_wrong_platform_and_protocol() {
    let missing_executable = temp_root("missing-executable");
    let requirement = fixture_runtime(&missing_executable);
    fs::remove_file(missing_executable.join("bin/ftetwild-worker")).expect("remove executable");
    let error =
        probe_ftetwild_runtime(&missing_executable, &requirement).expect_err("missing executable");
    assert!(error.to_string().contains("executable"));

    let missing_source = temp_root("missing-source");
    let requirement = fixture_runtime(&missing_source);
    fs::remove_file(missing_source.join("source/ftetwild-source.tar.zst")).expect("remove source");
    let error = probe_ftetwild_runtime(&missing_source, &requirement).expect_err("missing source");
    assert!(error.to_string().contains("source archive"));

    let missing_license = temp_root("missing-license");
    let requirement = fixture_runtime(&missing_license);
    fs::remove_file(missing_license.join("legal/LICENSE.MPL-2.0")).expect("remove license");
    let error =
        probe_ftetwild_runtime(&missing_license, &requirement).expect_err("missing license");
    assert!(error.to_string().contains("license"));

    let wrong_platform = temp_root("wrong-platform");
    let requirement = fixture_runtime(&wrong_platform);
    mutate_manifest(&wrong_platform, |manifest| {
        manifest["platform"] = json!("not-this-platform");
    });
    let error =
        probe_ftetwild_runtime(&wrong_platform, &requirement).expect_err("platform mismatch");
    assert!(error.to_string().contains("platform mismatch"));

    let wrong_arch = temp_root("wrong-arch");
    let requirement = fixture_runtime(&wrong_arch);
    mutate_manifest(&wrong_arch, |manifest| {
        manifest["arch"] = json!("not-this-architecture");
    });
    let error =
        probe_ftetwild_runtime(&wrong_arch, &requirement).expect_err("architecture mismatch");
    assert!(error.to_string().contains("architecture mismatch"));

    let bad_digest = temp_root("bad-digest");
    let requirement = fixture_runtime(&bad_digest);
    fs::write(bad_digest.join("bin/ftetwild-worker"), b"tampered worker")
        .expect("tamper executable");
    let error =
        probe_ftetwild_runtime(&bad_digest, &requirement).expect_err("executable digest mismatch");
    assert!(error.to_string().contains("executable digest mismatch"));

    let unsupported_version = temp_root("unsupported-version");
    let mut requirement = fixture_runtime(&unsupported_version);
    requirement.runtime_version = "unsupported-v0".to_string();
    let error = probe_ftetwild_runtime(&unsupported_version, &requirement)
        .expect_err("unsupported runtime version");
    assert!(error.to_string().contains("runtime version mismatch"));

    let wrong_protocol = temp_root("wrong-protocol");
    let requirement = fixture_runtime(&wrong_protocol);
    mutate_manifest(&wrong_protocol, |manifest| {
        manifest["workerProtocol"] = json!("unsupported-v0");
    });
    let error =
        probe_ftetwild_runtime(&wrong_protocol, &requirement).expect_err("protocol mismatch");
    assert!(error.to_string().contains("worker protocol mismatch"));

    for root in [
        missing_source,
        missing_executable,
        missing_license,
        wrong_platform,
        wrong_arch,
        bad_digest,
        unsupported_version,
        wrong_protocol,
    ] {
        let _ = fs::remove_dir_all(root);
    }
}

#[test]
fn packaged_runtime_proves_pinned_binary_source_legal_and_dynamic_library_digests() {
    let Some(root) = std::env::var_os("ECKY_FTETWILD_RUNTIME_ROOT") else {
        eprintln!("ECKY_FTETWILD_RUNTIME_ROOT unset; package proof is platform-gated");
        return;
    };
    let requirement = FTetWildRuntimeRequirement {
        runtime_version: "0.1.0-ecky.1".to_string(),
        source_revision: "d7d99bb4387a07895b9adce058dc7305f6b6e5ab".to_string(),
    };
    let identity =
        probe_ftetwild_runtime(PathBuf::from(root), &requirement).expect("pinned packaged runtime");
    assert_eq!(identity.runtime_library_paths.len(), 1);
    assert!(identity.executable_sha256.starts_with("sha256:"));
    assert!(identity.source_sha256.starts_with("sha256:"));
    assert!(identity.license_sha256.starts_with("sha256:"));
    assert!(identity.notice_sha256.starts_with("sha256:"));
}

fn fixture_runtime(root: &Path) -> FTetWildRuntimeRequirement {
    let files = [
        ("bin/ftetwild-worker", b"native-worker".as_slice()),
        ("lib/libgmp.test", b"dynamic-gmp".as_slice()),
        ("source/ftetwild-source.tar.zst", b"source-archive".as_slice()),
        ("legal/LICENSE.MPL-2.0", b"MPL-2.0".as_slice()),
        ("legal/NOTICE.txt", b"notices".as_slice()),
        (
            "legal/transitive-licenses.json",
            br#"[{"name":"libigl","version":"pinned","license":"MPL-2.0","sourceUrl":"https://github.com/libigl/libigl"}]"#
                .as_slice(),
        ),
    ];
    for (path, bytes) in files {
        let path = root.join(path);
        fs::create_dir_all(path.parent().expect("parent")).expect("create parent");
        fs::write(path, bytes).expect("write fixture");
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(
            root.join("bin/ftetwild-worker"),
            fs::Permissions::from_mode(0o755),
        )
        .expect("mark worker executable");
    }

    let requirement = FTetWildRuntimeRequirement {
        runtime_version: "0.1.0-ecky.1".to_string(),
        source_revision: "0123456789abcdef0123456789abcdef01234567".to_string(),
    };
    let manifest = json!({
        "schemaVersion": FTETWILD_RUNTIME_SCHEMA_VERSION,
        "runtimeName": "fTetWild",
        "runtimeVersion": requirement.runtime_version,
        "sourceRevision": requirement.source_revision,
        "platform": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
        "workerProtocol": FTETWILD_WORKER_PROTOCOL,
        "executable": file_entry("bin/ftetwild-worker", root),
        "sourceArchive": file_entry("source/ftetwild-source.tar.zst", root),
        "license": file_entry("legal/LICENSE.MPL-2.0", root),
        "notice": file_entry("legal/NOTICE.txt", root),
        "transitiveLicenseInventory": file_entry("legal/transitive-licenses.json", root),
        "runtimeLibraries": [file_entry("lib/libgmp.test", root)],
        "capabilities": {
            "structuredArrays": true,
            "tet4": true,
            "wideSurfaceTags": true,
            "isolatedWorker": true
        }
    });
    fs::write(
        root.join("runtime-manifest.json"),
        serde_json::to_vec_pretty(&manifest).expect("serialize manifest"),
    )
    .expect("write manifest");
    requirement
}

fn file_entry(relative_path: &str, root: &Path) -> serde_json::Value {
    let bytes = fs::read(root.join(relative_path)).expect("read fixture");
    json!({
        "path": relative_path,
        "sha256": format!("sha256:{:x}", Sha256::digest(bytes))
    })
}

fn mutate_manifest(root: &Path, mutate: impl FnOnce(&mut serde_json::Value)) {
    let path = root.join("runtime-manifest.json");
    let mut manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(&path).expect("read manifest")).expect("parse manifest");
    mutate(&mut manifest);
    fs::write(
        path,
        serde_json::to_vec_pretty(&manifest).expect("serialize manifest"),
    )
    .expect("write manifest");
}

fn temp_root(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "ecky-fem-runtime-{label}-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir_all(&root).expect("create temp root");
    root
}
