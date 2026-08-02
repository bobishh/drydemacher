use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

struct TestDir {
    path: PathBuf,
}

impl TestDir {
    fn new(name: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock before unix epoch")
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("ecky-cli-{name}-{}-{nonce}", std::process::id()));
        fs::create_dir_all(&path).expect("create temp test dir");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn write_file(path: &Path, contents: &str) {
    fs::write(path, contents).unwrap_or_else(|err| {
        panic!("write {} failed: {err}", path.display());
    });
}

fn ecky_command() -> Command {
    Command::new(env!("CARGO_BIN_EXE_ecky"))
}

fn output_text(stream: &[u8]) -> String {
    String::from_utf8_lossy(stream).into_owned()
}

#[test]
fn check_accepts_simple_model_source() {
    let dir = TestDir::new("check-valid");
    let input_path = dir.path().join("input.ecky");
    write_file(&input_path, "(model (part body (box 1 2 3)))");

    let output = ecky_command()
        .arg("check")
        .arg(&input_path)
        .output()
        .expect("run ecky check");
    let stdout = output_text(&output.stdout);
    let stderr = output_text(&output.stderr);

    assert!(
        output.status.success(),
        "check should succeed\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert!(
        stderr.trim().is_empty(),
        "check success should keep stderr empty: {stderr}"
    );
    assert_eq!(stdout, "check: ok\nparts: 1\nparams: 0\n");
}

#[test]
fn check_reports_compile_error_on_stderr_for_invalid_source() {
    let dir = TestDir::new("check-invalid");
    let input_path = dir.path().join("invalid.ecky");
    write_file(&input_path, "(model\n  (part body (box 1 2 3))\n$)");

    let output = ecky_command()
        .arg("check")
        .arg(&input_path)
        .output()
        .expect("run ecky check");
    let stderr = output_text(&output.stderr);

    assert!(
        !output.status.success(),
        "check should fail for invalid source\nstderr:\n{stderr}"
    );
    assert!(
        stderr.contains("parse:") && stderr.contains('$'),
        "stderr should surface compile error\nstderr:\n{stderr}"
    );
    assert_eq!(output.status.code(), Some(3));
}

#[test]
fn lower_build123d_writes_requested_output_file() {
    let dir = TestDir::new("lower-build123d");
    let input_path = dir.path().join("input.ecky");
    let output_path = dir.path().join("nested").join("lowered.py");
    write_file(&input_path, "(model (part body (box 1 2 3)))");

    let output = ecky_command()
        .arg("lower")
        .arg("--backend")
        .arg("build123d")
        .arg(&input_path)
        .arg("--out")
        .arg(&output_path)
        .output()
        .expect("run ecky lower");
    let stdout = output_text(&output.stdout);
    let stderr = output_text(&output.stderr);

    assert!(
        output.status.success(),
        "lower should succeed\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert!(
        output_path.is_file(),
        "missing output file: {}",
        output_path.display()
    );

    let lowered = fs::read_to_string(&output_path).unwrap_or_else(|err| {
        panic!("read {} failed: {err}", output_path.display());
    });
    assert!(
        lowered.contains("from build123d import *"),
        "lowered build123d file missing expected import\n{lowered}"
    );
}

#[test]
fn lower_freecad_writes_lowered_source_to_stdout() {
    let dir = TestDir::new("lower-freecad");
    let input_path = dir.path().join("input.ecky");
    write_file(&input_path, "(model (part body (box 1 2 3)))");

    let output = ecky_command()
        .args(["lower", "--backend", "freecad"])
        .arg(&input_path)
        .output()
        .expect("run ecky lower");
    assert!(output.status.success());
    assert!(output_text(&output.stdout).contains("import FreeCAD"));
}

#[test]
fn lower_reports_raw_lowering_error_with_exit_four() {
    let dir = TestDir::new("lower-error");
    let input_path = dir.path().join("input.ecky");
    write_file(
        &input_path,
        "(model (part body (hull (box 1 2 3) (box 4 5 6))))",
    );

    let output = ecky_command()
        .args(["lower", "--backend", "freecad"])
        .arg(&input_path)
        .output()
        .expect("run ecky lower");
    assert_eq!(output.status.code(), Some(4));
    assert!(output_text(&output.stderr).contains("hull"));
}

#[test]
fn lower_reports_write_error_with_exit_six() {
    let dir = TestDir::new("write-error");
    let input_path = dir.path().join("input.ecky");
    let output_path = dir.path().join("output-dir");
    write_file(&input_path, "(model (part body (box 1 2 3)))");
    fs::create_dir_all(&output_path).expect("create output directory");

    let output = ecky_command()
        .args(["lower", "--backend", "build123d"])
        .arg(&input_path)
        .args(["--out"])
        .arg(&output_path)
        .output()
        .expect("run ecky lower");
    assert_eq!(output.status.code(), Some(6));
    assert!(!output_text(&output.stderr).trim().is_empty());
}

#[test]
fn invalid_cli_usage_exits_two() {
    let output = ecky_command()
        .args(["lower", "--backend", "unknown", "input.ecky"])
        .output()
        .expect("run ecky lower");
    assert_eq!(output.status.code(), Some(2));
    assert!(output_text(&output.stderr).contains("unsupported backend: unknown"));
}

#[test]
fn no_arguments_reports_usage_with_exit_two() {
    let output = ecky_command().output().expect("run ecky");
    assert_eq!(output.status.code(), Some(2));
    assert!(output_text(&output.stdout).is_empty());
    assert!(output_text(&output.stderr).contains("Usage:"));
}

#[test]
fn render_rejects_malformed_param_before_backend_execution() {
    let output = ecky_command()
        .args([
            "render",
            "--backend",
            "build123d",
            "model.ecky",
            "--param",
            "width",
            "--stl",
            "out.stl",
        ])
        .output()
        .expect("run ecky render");

    assert_eq!(output.status.code(), Some(2));
    assert!(
        output_text(&output.stderr).contains("--param requires key=value"),
        "stderr:\n{}",
        output_text(&output.stderr)
    );
}

#[test]
fn render_requires_an_explicit_stl_or_bundle_dir() {
    let output = ecky_command()
        .args(["render", "--backend", "direct-occt", "model.ecky"])
        .output()
        .expect("run ecky render");

    assert_eq!(output.status.code(), Some(2));
    assert!(
        output_text(&output.stderr).contains("render requires --stl or --bundle-dir"),
        "stderr:\n{}",
        output_text(&output.stderr)
    );
}

#[test]
fn render_build123d_writes_stl_with_json_params_overridden_by_cli_param() {
    let dir = TestDir::new("render-build123d-params");
    let input_path = dir.path().join("input.ecky");
    let params_path = dir.path().join("params.json");
    let output_path = dir.path().join("nested").join("model.stl");
    write_file(
        &input_path,
        "(model (params (number width 10)) (part body (box width 2 3)))",
    );
    write_file(&params_path, r#"{"width":20}"#);

    let output = ecky_command()
        .args(["render", "--backend", "build123d"])
        .arg(&input_path)
        .args(["--params"])
        .arg(&params_path)
        .args(["--param", "width=42", "--stl"])
        .arg(&output_path)
        .arg("--json")
        .output()
        .expect("run ecky render");
    let stdout = output_text(&output.stdout);
    let stderr = output_text(&output.stderr);

    assert!(
        output.status.success(),
        "render should succeed\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert!(
        output_path.is_file(),
        "missing STL: {}",
        output_path.display()
    );
    assert!(
        fs::metadata(&output_path).expect("inspect STL").len() > 0,
        "STL should not be empty"
    );
    let summary: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("render JSON summary");
    assert_eq!(summary["backend"], "build123d");
}
