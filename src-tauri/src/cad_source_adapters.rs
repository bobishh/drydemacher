//! Foreign CAD source reduction for the transpile prompt.
//!
//! These adapters only extract source text. They never generate Ecky; the LLM
//! remains the sole Ecky author after `build_transpile_messages`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

/// Process invocation boundary. Tests provide a fake, so extraction has no
/// dependency on an installed FreeCAD binary.
pub trait CadSourceCommandRunner {
    fn run(&self, command: &CadSourceCommand) -> Result<CadSourceCommandOutput, String>;
}

/// One FreeCAD extractor invocation. `script` is deliberately visible to test
/// the extraction contract, while the production runner keeps its temp path
/// private and never includes it in an error returned to callers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CadSourceCommand {
    pub program: String,
    pub args: Vec<String>,
    pub script: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CadSourceCommandOutput {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
}

/// Production `freecadcmd` process runner.
pub struct SystemCadSourceCommandRunner;

impl CadSourceCommandRunner for SystemCadSourceCommandRunner {
    fn run(&self, command: &CadSourceCommand) -> Result<CadSourceCommandOutput, String> {
        let script_path = temporary_extractor_path();
        fs::write(&script_path, &command.script)
            .map_err(|_| "FreeCAD extractor setup failed.".to_string())?;
        let args = command.args.iter().map(|arg| {
            if arg == "{extractor-script}" {
                script_path.as_os_str()
            } else {
                std::ffi::OsStr::new(arg)
            }
        });
        let result = Command::new(&command.program).args(args).output();
        let _ = fs::remove_file(&script_path);
        result
            .map(|output| CadSourceCommandOutput {
                success: output.status.success(),
                stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            })
            .map_err(|_| "FreeCAD extractor launch failed.".to_string())
    }
}

fn temporary_extractor_path() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    std::env::temp_dir().join(format!(
        "ecky-freecad-extract-{}-{nonce}.py",
        std::process::id()
    ))
}

/// Dispatch a foreign CAD source to a text-only adapter. Unknown extensions
/// retain the established raw UTF-8 text behavior.
pub fn adapt_cad_source<R: CadSourceCommandRunner>(
    path: &Path,
    bytes: &[u8],
    runner: &R,
) -> Result<String, String> {
    match extension(path).as_deref() {
        Some("scad") => crate::cad_transpile::adapt_openscad_source(path, bytes),
        Some("fcstd") => adapt_fcstd_source(path, runner),
        Some("step") | Some("stp") | Some("brep") | Some("brp") => {
            adapt_step_or_brep_source(path, bytes)
        }
        _ => std::str::from_utf8(bytes)
            .map(str::to_owned)
            .map_err(|error| format!("read input '{}': {error}", path.display())),
    }
}

fn extension(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.to_ascii_lowercase())
}

/// Extract a fresh feature tree JSON document from an `.FCStd` file. The
/// archive bytes are intentionally not parsed in-process: FreeCAD owns this
/// format and only its command process reads it.
pub fn adapt_fcstd_source<R: CadSourceCommandRunner>(
    path: &Path,
    runner: &R,
) -> Result<String, String> {
    let command = CadSourceCommand {
        program: "freecadcmd".to_string(),
        args: vec![
            "--console".to_string(),
            "--run".to_string(),
            "{extractor-script}".to_string(),
        ],
        script: freecad_feature_tree_script(path),
    };
    let output = runner.run(&command)?;
    if !output.success {
        return Err("FreeCAD feature extraction failed.".to_string());
    }
    let parsed: serde_json::Value = serde_json::from_str(&output.stdout)
        .map_err(|_| "FreeCAD feature extraction returned invalid JSON.".to_string())?;
    if !parsed.is_object() {
        return Err("FreeCAD feature extraction returned invalid JSON.".to_string());
    }
    Ok(output.stdout)
}

fn freecad_feature_tree_script(path: &Path) -> String {
    let source_path =
        serde_json::to_string(&path.to_string_lossy()).unwrap_or_else(|_| "\"\"".to_string());
    format!(
        r#"import FreeCAD as App
import json

source_path = {source_path}
doc = App.openDocument(source_path)

def value_for(obj, prop):
    try:
        value = getattr(obj, prop)
        json.dumps(value)
        return value
    except Exception:
        try:
            return str(getattr(obj, prop))
        except Exception:
            return None

def describe(obj):
    node = {{"name": obj.Name, "label": obj.Label, "typeId": obj.TypeId}}
    props = {{}}
    for prop in obj.PropertiesList:
        if prop == "Shape" or prop == "FileName" or obj.getTypeIdOfProperty(prop) == "App::PropertyFile":
            continue
        props[prop] = value_for(obj, prop)
    node["properties"] = props
    if hasattr(obj, "Group"):
        node["children"] = [describe(child) for child in obj.Group]
    return node

try:
    roots = [obj for obj in doc.Objects if not getattr(obj, "InList", [])]
    print(json.dumps({{"format": "freecad-feature-tree-v1", "document": {{"name": doc.Name, "label": doc.Label, "objects": [describe(obj) for obj in roots]}}}}, separators=(",", ":")))
finally:
    App.closeDocument(doc.Name)
"#
    )
}

/// Produce a deliberately lossy but readable measurement summary. STEP is
/// counted from standard entities; OpenCascade BREP is counted from topology
/// markers. This is prompt source, not a parity measurement or geometry export.
pub fn adapt_step_or_brep_source(path: &Path, bytes: &[u8]) -> Result<String, String> {
    let text = std::str::from_utf8(bytes)
        .map_err(|_| "CAD textual summary requires UTF-8 STEP/BREP input.".to_string())?;
    match extension(path).as_deref() {
        Some("step") | Some("stp") => Ok(step_summary(text)),
        Some("brep") | Some("brp") => Ok(brep_summary(text)),
        _ => Err("CAD textual summary requires STEP or BREP input.".to_string()),
    }
}

fn step_summary(text: &str) -> String {
    let upper = text.to_ascii_uppercase();
    let bodies = count(&upper, "MANIFOLD_SOLID_BREP")
        + count(&upper, "BREP_WITH_VOIDS")
        + count(&upper, "SHELL_BASED_SURFACE_MODEL");
    let points = step_points(text);
    format!(
        "CAD textual summary\nformat: STEP\nbodies: {bodies}\ndimensions: {}\nkey features: advanced_faces={}, edge_curves={}, shells={}, vertices={}\n",
        dimensions(&points),
        count(&upper, "ADVANCED_FACE"),
        count(&upper, "EDGE_CURVE"),
        count(&upper, "CLOSED_SHELL"),
        count(&upper, "VERTEX_POINT"),
    )
}

fn brep_summary(text: &str) -> String {
    let points = text
        .lines()
        .filter_map(triple_from_line)
        .collect::<Vec<_>>();
    let markers = |marker: &str| text.lines().filter(|line| line.trim() == marker).count();
    format!(
        "CAD textual summary\nformat: BREP\nbodies: {}\ndimensions: {}\nkey features: solids={}, shells={}, faces={}, edges={}, vertices={}\n",
        markers("So"), dimensions(&points), markers("So"), markers("Sh"), markers("Fa"), markers("Ed"), markers("Ve"),
    )
}

fn count(haystack: &str, needle: &str) -> usize {
    haystack.match_indices(needle).count()
}

fn step_points(text: &str) -> Vec<[f64; 3]> {
    text.lines()
        .filter(|line| line.to_ascii_uppercase().contains("CARTESIAN_POINT"))
        .filter_map(|line| {
            let values = line
                .rsplit_once('(')?
                .1
                .trim_end_matches(|character| character == ')' || character == ';');
            triple_from_values(values)
        })
        .collect()
}

fn triple_from_line(line: &str) -> Option<[f64; 3]> {
    triple_from_values(line)
}

fn triple_from_values(values: &str) -> Option<[f64; 3]> {
    let numbers = values
        .split(|character: char| {
            !(character.is_ascii_digit()
                || matches!(character, '.' | '-' | '+' | 'E' | 'e' | 'D' | 'd'))
        })
        .filter(|value| !value.is_empty())
        .filter_map(|value| value.replace(['D', 'd'], "E").parse::<f64>().ok())
        .collect::<Vec<_>>();
    (numbers.len() >= 3).then(|| [numbers[0], numbers[1], numbers[2]])
}

fn dimensions(points: &[[f64; 3]]) -> String {
    if points.is_empty() {
        return "unavailable".to_string();
    }
    let mut low = points[0];
    let mut high = points[0];
    for point in points.iter().skip(1) {
        for index in 0..3 {
            low[index] = low[index].min(point[index]);
            high[index] = high[index].max(point[index]);
        }
    }
    format!(
        "x={}, y={}, z={}",
        number(high[0] - low[0]),
        number(high[1] - low[1]),
        number(high[2] - low[2])
    )
}

fn number(value: f64) -> String {
    if value.fract() == 0.0 {
        format!("{value:.0}")
    } else {
        format!("{value:.6}")
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    }
}
