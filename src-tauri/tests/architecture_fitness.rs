use std::fs;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("repo root")
        .to_path_buf()
}

fn collect_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("read dir") {
        let entry = entry.expect("dir entry");
        let path = entry.path();
        if path.is_dir() {
            collect_files(&path, out);
        } else {
            out.push(path);
        }
    }
}

#[test]
fn dev_scripts_do_not_write_application_sqlite_directly() {
    let root = repo_root();
    let scripts = root.join("scripts");
    let mut files = Vec::new();
    collect_files(&scripts, &mut files);

    let allowlist = [scripts.join("guard_no_direct_db_write.sh")];
    let forbidden = [
        "history.sqlite",
        "sqlite3 ",
        "sqlite3\n",
        "sqlite3\t",
        "sqlite3.connect",
        "INSERT INTO",
        "UPDATE ",
        "DELETE FROM",
    ];

    let mut violations = Vec::new();
    for path in files {
        if allowlist.iter().any(|allowed| allowed == &path) {
            continue;
        }
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        for pattern in forbidden {
            if text.contains(pattern) {
                violations.push(format!("{} contains {}", path.display(), pattern));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "direct DB write patterns must stay out of dev scripts:\n{}",
        violations.join("\n")
    );
}

#[test]
fn production_export_modules_do_not_reference_debug_overlay_geometry() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let export_modules = [
        manifest_dir.join("src/commands/render.rs"),
        manifest_dir.join("src/commands/component_package.rs"),
        manifest_dir.join("src/services/render.rs"),
    ];
    let forbidden = ["ShapeGraphDebugOverlay", "debug_overlay", "debug overlay"];

    let mut violations = Vec::new();
    for path in export_modules {
        let text = fs::read_to_string(&path).expect("read export module");
        for pattern in forbidden {
            if text.contains(pattern) {
                violations.push(format!("{} contains {}", path.display(), pattern));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "debug overlays are preview diagnostics and must not enter production export modules:\n{}",
        violations.join("\n")
    );
}
