use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use ecky_render::{
    KernelArg as RunnerArg, KernelCommand as RunnerCommand, KernelKeyword as RunnerKeyword,
    KernelPart as RunnerPart, KernelPartialBooleanGroupPlan as RunnerPartialBooleanGroupPlan,
    KernelPlan as RunnerPlan, KernelRepresentation,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};

use super::direct_occt::{OcctArg, OcctCommand, OcctKeyword, OcctOp, OcctPlan, OcctSlot};
use crate::contracts::{AppError, AppResult};
use crate::models::PathResolver;

const PLAN_FILE_NAME: &str = "plan.json";
const RUNNER_RESOURCE_PATH: &str = "runtime/occt/bin/direct-occt-runner";
const LEGACY_RUNNER_RESOURCE_PATH: &str = "bin/direct-occt-runner";
const RUNNER_DISABLED_ENV: &str = "ECKY_DIRECT_OCCT_RUNNER_DISABLED";

/// Tests that need a machine-independent "no runner anywhere" environment
/// disable the CWD-relative fallback paths through a thread-local guard
/// instead of chdir, which would poison every concurrently running test.
#[cfg(test)]
pub(crate) mod test_discovery {
    use std::cell::Cell;

    thread_local! {
        static DISABLE_CWD_FALLBACKS: Cell<bool> = const { Cell::new(false) };
    }

    pub(crate) fn cwd_fallbacks_disabled() -> bool {
        DISABLE_CWD_FALLBACKS.with(Cell::get)
    }

    pub(crate) struct CwdFallbackGuard;

    impl CwdFallbackGuard {
        pub(crate) fn disable() -> Self {
            DISABLE_CWD_FALLBACKS.with(|flag| flag.set(true));
            Self
        }
    }

    impl Drop for CwdFallbackGuard {
        fn drop(&mut self) {
            DISABLE_CWD_FALLBACKS.with(|flag| flag.set(false));
        }
    }
}
const MODEL_STEP_FILE_NAME: &str = "model.step";
const PREVIEW_STL_FILE_NAME: &str = "preview.stl";
const STAGE_REPORT_FILE_NAME: &str = "stage-report.json";
const RUNNER_STAGE_NAMES: [&str; 8] = [
    "import", "validate", "solidify", "boolean", "cleanup", "mesh", "verify", "export",
];

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct RunnerStageReport {
    schema_version: u32,
    total_elapsed_ms: u64,
    #[serde(default)]
    worker_budget: Option<u32>,
    #[serde(default)]
    parallel_policy: Option<String>,
    #[serde(default)]
    serial_boolean_count: Option<u32>,
    #[serde(default)]
    parallel_boolean_count: Option<u32>,
    #[serde(default)]
    mesh_boolean_count: Option<u32>,
    #[serde(default)]
    tessellated_step_part_count: Option<u32>,
    #[serde(default)]
    max_nested_kernel_lease: Option<u32>,
    #[serde(default)]
    peak_total_allocated_cpu_units: Option<u32>,
    #[serde(default)]
    peak_dag_concurrency: Option<u32>,
    #[serde(default)]
    mesh_outer_worker_budget: Option<u32>,
    #[serde(default)]
    mesh_pool_budget: Option<u32>,
    #[serde(default)]
    mesh_launcher_budget: Option<u32>,
    #[serde(default)]
    mesh_build_count: Option<u32>,
    #[serde(default)]
    mesh_cache_hit_count: Option<u32>,
    #[serde(default)]
    preview_facet_count: Option<u64>,
    #[serde(default)]
    partial_boolean_cache_hit_count: Option<u64>,
    #[serde(default)]
    partial_boolean_cache_miss_count: Option<u64>,
    #[serde(default)]
    partial_boolean_cache_write_count: Option<u64>,
    #[serde(default)]
    four_way_intersection_count: Option<u64>,
    #[serde(default)]
    parts: Vec<RunnerPartExecutionEvidence>,
    #[serde(default)]
    commands: Vec<RunnerCommandExecutionEvidence>,
    #[serde(default)]
    partial_boolean_groups: Vec<RunnerPartialBooleanGroupEvidence>,
    stages: Vec<RunnerStageReportEntry>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct RunnerPartExecutionEvidence {
    part_id: String,
    cache_hit: bool,
    executed_command_count: u32,
    #[serde(default)]
    representation: Option<KernelRepresentation>,
    #[serde(default)]
    mesh_identity: Option<String>,
    #[serde(default)]
    mesh_facet_count: Option<u64>,
    #[serde(default)]
    executed_command_ids: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct RunnerCommandExecutionEvidence {
    command_id: String,
    cache_admitted: bool,
    cache_hit: bool,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct RunnerPartialBooleanGroupEvidence {
    part_id: String,
    parent_output: u64,
    key: String,
    cache_hit: bool,
    recompute_count: u32,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct RunnerStageReportEntry {
    name: String,
    status: String,
    execution_count: u32,
    elapsed_ms: u64,
}

fn validate_runner_stage_report(report: &RunnerStageReport) -> AppResult<()> {
    if report.schema_version != 1 {
        return Err(AppError::validation(format!(
            "Direct OCCT runner stage report has unsupported schemaVersion {}.",
            report.schema_version
        )));
    }
    if report.stages.len() != RUNNER_STAGE_NAMES.len() {
        return Err(AppError::validation(format!(
            "Direct OCCT runner stage report must contain {} ordered stages, got {}.",
            RUNNER_STAGE_NAMES.len(),
            report.stages.len()
        )));
    }
    if let Some(policy) = report.parallel_policy.as_deref() {
        if !matches!(policy, "outer-only" | "adaptive") {
            return Err(AppError::validation(format!(
                "Direct OCCT runner reported unsupported parallelPolicy `{policy}`."
            )));
        }
    }
    if let (Some(peak), Some(budget)) =
        (report.peak_total_allocated_cpu_units, report.worker_budget)
    {
        if peak > budget {
            return Err(AppError::validation(format!(
                "Direct OCCT runner allocated {peak} CPU units with worker budget {budget}."
            )));
        }
    }
    for (entry, expected_name) in report.stages.iter().zip(RUNNER_STAGE_NAMES) {
        if entry.name != expected_name {
            return Err(AppError::validation(format!(
                "Direct OCCT runner stage report expected stage `{expected_name}`, got `{}`.",
                entry.name
            )));
        }
        let expected_status = if entry.execution_count == 0 {
            "skipped"
        } else {
            "executed"
        };
        if entry.status != expected_status {
            return Err(AppError::validation(format!(
                "Direct OCCT runner stage `{expected_name}` status `{}` conflicts with executionCount {}.",
                entry.status, entry.execution_count
            )));
        }
        if entry.execution_count == 0 && entry.elapsed_ms != 0 {
            return Err(AppError::validation(format!(
                "Direct OCCT runner skipped stage `{expected_name}` reported elapsedMs {}.",
                entry.elapsed_ms
            )));
        }
    }
    Ok(())
}

fn read_runner_stage_report(output_dir: &Path) -> AppResult<RunnerStageReport> {
    let path = output_dir.join(STAGE_REPORT_FILE_NAME);
    let report_text = fs::read_to_string(&path).map_err(|err| {
        AppError::validation(format!(
            "Direct OCCT runner did not write stage report '{}': {}",
            path.display(),
            err
        ))
    })?;
    let report: RunnerStageReport = serde_json::from_str(&report_text).map_err(|err| {
        AppError::validation(format!(
            "Direct OCCT runner wrote invalid stage report '{}': {}",
            path.display(),
            err
        ))
    })?;
    validate_runner_stage_report(&report)?;
    Ok(report)
}

pub(crate) fn run_plan_step_stl_if_available(
    plan: &OcctPlan,
    output_dir: impl AsRef<Path>,
    app: &dyn PathResolver,
) -> AppResult<Option<super::direct_occt_sdk::NativeExportOutcome>> {
    run_plan_step_stl_with_mode(plan, output_dir, app, runner_enabled())
}

pub(crate) fn run_plan_step_stl_with_mode(
    plan: &OcctPlan,
    output_dir: impl AsRef<Path>,
    app: &dyn PathResolver,
    enabled: bool,
) -> AppResult<Option<super::direct_occt_sdk::NativeExportOutcome>> {
    if !enabled {
        return Ok(None);
    }

    let Some(runner_path) = discover_direct_occt_runner_with_mode(app, enabled) else {
        return Ok(None);
    };

    let runner_safe_plan = runner_supports_plan(plan);
    if !runner_safe_plan {
        return Err(AppError::validation(
            "Direct OCCT runner does not support plan; generated-C++ fallback was removed.",
        ));
    }

    let Some(serialized_plan) = runner_plan(plan)? else {
        return Err(AppError::validation(
            "Direct OCCT runner support gate accepted plan, but runner serialization rejected it."
                .to_string(),
        ));
    };
    let source_mesh_digests = serialized_plan
        .parts
        .iter()
        .flat_map(|part| &part.commands)
        .filter(|command| command.op == "import-indexed-mesh")
        .filter_map(|command| command.args.get(2))
        .filter_map(|arg| arg.value.as_str())
        .map(str::to_string)
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let plan_json = serde_json::to_string_pretty(&serialized_plan).map_err(|err| {
        AppError::validation(format!(
            "Direct OCCT runner plan serialization failed: {err}"
        ))
    })?;

    let output_dir = output_dir.as_ref();
    fs::create_dir_all(output_dir).map_err(|err| {
        AppError::validation(format!(
            "Direct OCCT runner could not create output dir '{}': {}",
            output_dir.display(),
            err
        ))
    })?;

    let plan_path = output_dir.join(PLAN_FILE_NAME);
    fs::write(&plan_path, plan_json).map_err(|err| {
        AppError::validation(format!(
            "Direct OCCT runner could not write '{}': {}",
            plan_path.display(),
            err
        ))
    })?;

    let output = Command::new(&runner_path)
        .arg("--plan")
        .arg(&plan_path)
        .arg("--out")
        .arg(output_dir)
        .output()
        .map_err(|err| {
            AppError::validation(format!(
                "Direct OCCT runner could not start '{}': {}",
                runner_path.display(),
                err
            ))
        })?;

    if !output.status.success() && runner_reported_unsupported(&output) {
        return Err(AppError::with_details(
            crate::contracts::AppErrorCode::Validation,
            "Direct OCCT runner rejected the native plan; generated-C++ fallback was removed.",
            format!(
                "runner: {}\nexit: {}\nstdout: {}\nstderr: {}",
                runner_path.display(),
                output
                    .status
                    .code()
                    .map(|code| code.to_string())
                    .unwrap_or_else(|| "terminated by signal".to_string()),
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            ),
        ));
    }

    if !output.status.success() {
        return Err(AppError::with_details(
            crate::contracts::AppErrorCode::Validation,
            "Direct OCCT runner failed.",
            format!(
                "runner: {}\nexit: {}\nstdout: {}\nstderr: {}",
                runner_path.display(),
                output
                    .status
                    .code()
                    .map(|code| code.to_string())
                    .unwrap_or_else(|| "terminated by signal".to_string()),
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            ),
        ));
    }

    let stage_report = read_runner_stage_report(output_dir)?;

    // Scan for per-part binary STL files written by the runner into parts/.
    let mut part_stl_paths = Vec::new();
    let parts_dir = output_dir.join("parts");
    if parts_dir.is_dir() {
        for part in plan.parts.iter() {
            let key = if part.key.trim().is_empty() {
                "body".to_string()
            } else {
                part.key.clone()
            };
            let candidate = parts_dir.join(format!("{}.stl", key));
            if candidate.is_file() {
                part_stl_paths.push((key, candidate));
            }
        }
    }

    let stl_path = output_dir.join(PREVIEW_STL_FILE_NAME);
    let step_path = output_dir.join(MODEL_STEP_FILE_NAME);
    Ok(Some(if step_path.is_file() {
        super::direct_occt_sdk::NativeExportOutcome::Exported {
            step_path: output_dir.join(MODEL_STEP_FILE_NAME),
            stl_path,
            part_stl_paths,
            tessellated_step: stage_report.tessellated_step_part_count.unwrap_or(0) > 0,
            source_mesh_digests,
        }
    } else {
        super::direct_occt_sdk::NativeExportOutcome::MeshExported {
            stl_path,
            part_stl_paths,
            source_mesh_digests,
        }
    }))
}

pub(crate) fn discover_direct_occt_runner_with_mode(
    app: &dyn PathResolver,
    enabled: bool,
) -> Option<PathBuf> {
    if !enabled {
        return None;
    }

    let mut candidates = Vec::new();
    if let Some(path) = app.resource_path(RUNNER_RESOURCE_PATH) {
        candidates.push(path);
    }
    if let Some(path) = app.resource_path(LEGACY_RUNNER_RESOURCE_PATH) {
        candidates.push(path);
    }

    #[cfg(test)]
    let skip_cwd_fallbacks = test_discovery::cwd_fallbacks_disabled();
    #[cfg(not(test))]
    let skip_cwd_fallbacks = false;
    if !skip_cwd_fallbacks {
        for fallback in runner_fallback_paths() {
            candidates.push(PathBuf::from(fallback));
        }
    }

    candidates.into_iter().find(|candidate| candidate.exists())
}

fn runner_enabled() -> bool {
    match std::env::var(RUNNER_DISABLED_ENV) {
        Ok(value) => {
            let value = value.trim().to_ascii_lowercase();
            !(value == "1" || value == "true" || value == "yes" || value == "on")
        }
        Err(_) => true,
    }
}

fn runner_fallback_paths() -> &'static [&'static str] {
    if cfg!(windows) {
        &[
            "../.dist/runtime/occt/bin/direct-occt-runner.exe",
            ".dist/runtime/occt/bin/direct-occt-runner.exe",
            "../bin/direct-occt-runner.exe",
            "bin/direct-occt-runner.exe",
            "../.dist/runtime/occt/bin/direct-occt-runner",
            ".dist/runtime/occt/bin/direct-occt-runner",
            "../bin/direct-occt-runner",
            "bin/direct-occt-runner",
        ]
    } else {
        &[
            "../.dist/runtime/occt/bin/direct-occt-runner",
            ".dist/runtime/occt/bin/direct-occt-runner",
            "../bin/direct-occt-runner",
            "bin/direct-occt-runner",
            "../.dist/runtime/occt/bin/direct-occt-runner.exe",
            ".dist/runtime/occt/bin/direct-occt-runner.exe",
            "../bin/direct-occt-runner.exe",
            "bin/direct-occt-runner.exe",
        ]
    }
}

#[cfg(test)]
fn serialize_runner_plan(plan: &OcctPlan) -> AppResult<Option<String>> {
    let Some(plan) = runner_plan(plan)? else {
        return Ok(None);
    };
    serde_json::to_string_pretty(&plan)
        .map(Some)
        .map_err(|err| {
            AppError::validation(format!(
                "Direct OCCT runner plan serialization failed: {}",
                err
            ))
        })
}

fn runner_plan(plan: &OcctPlan) -> AppResult<Option<RunnerPlan>> {
    let mut parts = Vec::with_capacity(plan.parts.len());
    let mut partial_boolean_groups = Vec::new();
    for part in &plan.parts {
        let indexed_imports = indexed_mesh_imports_for_root_boolean(part)?;
        let hybrid_lid = part.commands.iter().any(|command| {
            command.output == part.root
                && command.op == OcctOp::Union
                && command.args.len() == 4
                && decorated_dome_pair(command, &part.commands)
                && command.args[2..4].iter().any(|arg| match arg {
                    OcctArg::Ref(root) => {
                        slot_depends_on_any(*root, &part.commands, indexed_imports.keys().copied())
                    }
                    _ => false,
                })
        });
        let threaded_mesh_closure = part
            .commands
            .iter()
            .any(|command| command.op == OcctOp::HelixPath)
            && part
                .commands
                .iter()
                .filter(|command| {
                    matches!(
                        command.op,
                        OcctOp::Union | OcctOp::Difference | OcctOp::Intersection
                    )
                })
                .count()
                >= 4
            && mesh_domain_boolean_closure_supported(part);
        let mut commands = Vec::with_capacity(part.commands.len());
        for command in &part.commands {
            if command.op == OcctOp::Union
                && command.output == part.root
                && command.args.len() == 4
                && hybrid_lid
                && command
                    .args
                    .iter()
                    .all(|arg| matches!(arg, OcctArg::Ref(_)))
                && command.keywords.is_empty()
                && decorated_dome_pair(command, &part.commands)
            {
                for (ordinal, input_indices) in [[0_u32, 1], [2, 3]].into_iter().enumerate() {
                    partial_boolean_groups.push(RunnerPartialBooleanGroupPlan {
                        part_key: part.key.clone(),
                        parent_output: command.output.0,
                        key: if ordinal == 1 {
                            "decorated-dome".to_string()
                        } else {
                            "operand-pair-0".to_string()
                        },
                        representation: if ordinal == 1 {
                            KernelRepresentation::MeshDomain
                        } else {
                            KernelRepresentation::AnalyticBrep
                        },
                        operation: "union".to_string(),
                        input_indices: input_indices.to_vec(),
                        ordinal: ordinal as u32,
                        version: 2,
                    });
                }
            }
            let runner_command = if let Some(asset) = indexed_imports.get(&command.output.0) {
                runner_indexed_mesh_command(command, asset)?
            } else {
                let Some(runner_command) = runner_command(command)? else {
                    return Ok(None);
                };
                runner_command
            };
            commands.push(runner_command);
        }
        parts.push(RunnerPart {
            key: part.key.clone(),
            label: part.label.clone(),
            root: part.root.0,
            representation: if !indexed_imports.is_empty() || threaded_mesh_closure {
                KernelRepresentation::MeshDomain
            } else {
                KernelRepresentation::AnalyticBrep
            },
            commands,
        });
    }

    let body = RunnerPlan {
        schema_version: 1,
        plan_id: runner_plan_id(&parts, &partial_boolean_groups)?,
        parts,
        partial_boolean_groups,
    };
    Ok(Some(body))
}

fn mesh_domain_boolean_closure_supported(part: &super::direct_occt::OcctPartPlan) -> bool {
    let mut mesh_outputs = HashSet::new();
    for command in &part.commands {
        let consumes_mesh = command
            .args
            .iter()
            .chain(command.keywords.iter().map(OcctKeyword::source_arg))
            .any(|arg| matches!(arg, OcctArg::Ref(slot) if mesh_outputs.contains(slot)));
        if matches!(
            command.op,
            OcctOp::Union | OcctOp::Difference | OcctOp::Intersection
        ) {
            mesh_outputs.insert(command.output);
        } else if consumes_mesh {
            if matches!(
                command.op,
                OcctOp::Translate | OcctOp::Rotate | OcctOp::Scale
            ) {
                mesh_outputs.insert(command.output);
            } else {
                return false;
            }
        }
    }
    mesh_outputs.contains(&part.root)
}

fn decorated_dome_pair(command: &OcctCommand, commands: &[OcctCommand]) -> bool {
    let [OcctArg::Ref(left), OcctArg::Ref(right)] = &command.args[2..4] else {
        return false;
    };
    let by_output = commands
        .iter()
        .map(|candidate| (candidate.output, candidate))
        .collect::<HashMap<_, _>>();
    let contains = |root: OcctSlot, accepted: fn(OcctOp) -> bool| {
        let mut pending = vec![root];
        let mut visited = HashSet::new();
        while let Some(slot) = pending.pop() {
            if !visited.insert(slot) {
                continue;
            }
            let Some(candidate) = by_output.get(&slot) else {
                continue;
            };
            if accepted(candidate.op) {
                return true;
            }
            for arg in &candidate.args {
                if let OcctArg::Ref(dependency) = arg {
                    pending.push(*dependency);
                }
            }
            for keyword in &candidate.keywords {
                if let OcctArg::Ref(dependency) = keyword.source_arg() {
                    pending.push(*dependency);
                }
            }
        }
        false
    };
    let is_import = |op| matches!(op, OcctOp::ImportStl | OcctOp::ImportStep);
    let is_analytic_clip = |op| op == OcctOp::Intersection;
    (contains(*left, is_import) && contains(*right, is_analytic_clip))
        || (contains(*right, is_import) && contains(*left, is_analytic_clip))
}

fn slot_depends_on_any(
    root: OcctSlot,
    commands: &[OcctCommand],
    targets: impl IntoIterator<Item = u64>,
) -> bool {
    let targets = targets.into_iter().collect::<HashSet<_>>();
    let by_output = commands
        .iter()
        .map(|command| (command.output, command))
        .collect::<HashMap<_, _>>();
    let mut pending = vec![root];
    let mut visited = HashSet::new();
    while let Some(slot) = pending.pop() {
        if !visited.insert(slot) {
            continue;
        }
        if targets.contains(&slot.0) {
            return true;
        }
        let Some(command) = by_output.get(&slot) else {
            continue;
        };
        for arg in &command.args {
            if let OcctArg::Ref(dependency) = arg {
                pending.push(*dependency);
            }
        }
        for keyword in &command.keywords {
            if let OcctArg::Ref(dependency) = keyword.source_arg() {
                pending.push(*dependency);
            }
        }
    }
    false
}

fn indexed_mesh_imports_for_root_boolean(
    part: &super::direct_occt::OcctPartPlan,
) -> AppResult<HashMap<u64, crate::ecky_ir::mesh_asset::IndexedMeshAsset>> {
    let mut consumers = HashMap::<u64, Vec<&OcctCommand>>::new();
    for command in &part.commands {
        let mut refs = Vec::new();
        for arg in &command.args {
            collect_runner_arg_refs(arg, &mut refs);
        }
        for keyword in &command.keywords {
            collect_runner_arg_refs(keyword.source_arg(), &mut refs);
        }
        for slot in refs {
            consumers.entry(slot).or_default().push(command);
        }
    }

    let mut admitted = HashMap::new();
    for command in &part.commands {
        if command.op != OcctOp::ImportStl || command.args.len() != 1 {
            continue;
        }
        let path = match &command.args[0] {
            OcctArg::Text(path) | OcctArg::Symbol(path) => PathBuf::from(path),
            _ => continue,
        };
        let Some([solidify]) = consumers.get(&command.output.0).map(Vec::as_slice) else {
            continue;
        };
        if solidify.op != OcctOp::Solidify
            || solidify.args.as_slice() != [OcctArg::Ref(command.output)]
        {
            continue;
        }
        let mut current = solidify.output;
        let boolean = loop {
            let Some([consumer]) = consumers.get(&current.0).map(Vec::as_slice) else {
                break None;
            };
            if runner_manifold_transform(consumer, current) {
                current = consumer.output;
                continue;
            }
            break Some(*consumer);
        };
        let Some(boolean) = boolean else { continue };
        let binary_root_chain = matches!(
            boolean.op,
            OcctOp::Union | OcctOp::Difference | OcctOp::Intersection
        ) && boolean.args.len() == 2
            && boolean.args.first() == Some(&OcctArg::Ref(current))
            && binary_boolean_chain_reaches_root(boolean.output, part.root, &consumers);
        let decorated_root_union = boolean.output == part.root
            && boolean.op == OcctOp::Union
            && boolean.args.len() == 4
            && boolean.args[2..4].contains(&OcctArg::Ref(current))
            && decorated_dome_pair(boolean, &part.commands)
            && !consumers.contains_key(&part.root.0);
        if !binary_root_chain && !decorated_root_union {
            continue;
        }

        let sidecar = path.with_extension("indexed-mesh.json");
        let asset = if sidecar.is_file() {
            crate::ecky_ir::mesh_asset::IndexedMeshAsset::read_cache(&sidecar)?
        } else {
            // Standalone imports preserve authored coordinates: decode through
            // the canonical authored dispatch (STL by IEEE-754 bit-equality,
            // 3MF by explicit indexing). The evaluated-CAD 1e-6 mm seam weld
            // is intentionally NOT applied here, so the runner consumes the
            // same authored coordinates/digest a sidecar would carry.
            crate::ecky_ir::mesh_asset::IndexedMeshAsset::from_imported_file(
                crate::ecky_ir::mesh_asset::MeshAssetSource::Imported,
                &path,
            )?
        };
        if asset.validate_for_boolean().is_err() {
            continue;
        }
        admitted.insert(command.output.0, asset);
    }
    Ok(admitted)
}

/// Imported indexed meshes remain on the Manifold path when their first root
/// Boolean is the first link of a binary Boolean chain. The optimizer emits
/// `Difference(import, tool-a) -> fresh` then consumes that fresh output as the
/// BASE of each later link, ending at `part.root`. Do not admit a mesh used as a
/// tool, branched result, or with a non-Boolean post-consumer.
fn binary_boolean_chain_reaches_root(
    mut current: super::direct_occt::OcctSlot,
    root: super::direct_occt::OcctSlot,
    consumers: &HashMap<u64, Vec<&OcctCommand>>,
) -> bool {
    let mut visited = HashSet::new();
    loop {
        if current == root {
            return !consumers.contains_key(&root.0);
        }
        if !visited.insert(current.0) {
            return false;
        }
        let Some([consumer]) = consumers.get(&current.0).map(Vec::as_slice) else {
            return false;
        };
        if !matches!(
            consumer.op,
            OcctOp::Union | OcctOp::Difference | OcctOp::Intersection
        ) || consumer.args.len() != 2
            || consumer.args.first() != Some(&OcctArg::Ref(current))
        {
            return false;
        }
        current = consumer.output;
    }
}

fn runner_manifold_transform(command: &OcctCommand, input: super::direct_occt::OcctSlot) -> bool {
    if !matches!(
        command.op,
        OcctOp::Translate | OcctOp::Rotate | OcctOp::Scale
    ) {
        return false;
    }
    let mut refs = Vec::new();
    for arg in &command.args {
        collect_runner_arg_refs(arg, &mut refs);
    }
    for keyword in &command.keywords {
        collect_runner_arg_refs(keyword.source_arg(), &mut refs);
    }
    refs.as_slice() == [input.0]
}

fn collect_runner_arg_refs(arg: &OcctArg, refs: &mut Vec<u64>) {
    match arg {
        OcctArg::Ref(slot) => refs.push(slot.0),
        OcctArg::List(items) => {
            for item in items {
                collect_runner_arg_refs(item, refs);
            }
        }
        OcctArg::Number(_)
        | OcctArg::Boolean(_)
        | OcctArg::Text(_)
        | OcctArg::Symbol(_)
        | OcctArg::Point2(_)
        | OcctArg::Point3(_)
        | OcctArg::Param(_) => {}
    }
}

fn runner_indexed_mesh_command(
    command: &OcctCommand,
    asset: &crate::ecky_ir::mesh_asset::IndexedMeshAsset,
) -> AppResult<RunnerCommand> {
    let vertices = asset
        .vertices()
        .iter()
        .map(|point| RunnerArg {
            kind: "point3".to_string(),
            value: serde_json::json!(point),
        })
        .collect::<Vec<_>>();
    let triangles = asset
        .triangles()
        .iter()
        .map(|triangle| RunnerArg {
            kind: "list".to_string(),
            value: serde_json::json!(triangle
                .iter()
                .map(|index| RunnerArg {
                    kind: "number".to_string(),
                    value: serde_json::json!(index),
                })
                .collect::<Vec<_>>()),
        })
        .collect::<Vec<_>>();
    Ok(RunnerCommand {
        output: command.output.0,
        op: "import-indexed-mesh".to_string(),
        args: vec![
            RunnerArg {
                kind: "list".to_string(),
                value: serde_json::to_value(vertices).map_err(|err| {
                    AppError::validation(format!(
                        "Direct OCCT indexed vertex serialization failed: {err}"
                    ))
                })?,
            },
            RunnerArg {
                kind: "list".to_string(),
                value: serde_json::to_value(triangles).map_err(|err| {
                    AppError::validation(format!(
                        "Direct OCCT indexed triangle serialization failed: {err}"
                    ))
                })?,
            },
            RunnerArg {
                kind: "text".to_string(),
                value: serde_json::json!(asset.content_digest()),
            },
        ],
        keywords: Vec::new(),
    })
}

fn runner_plan_id(
    parts: &[RunnerPart],
    partial_boolean_groups: &[RunnerPartialBooleanGroupPlan],
) -> AppResult<String> {
    let body = serde_json::json!({
        "schemaVersion": 1,
        "parts": parts,
        "partialBooleanGroups": partial_boolean_groups,
    });
    let body = serde_json::to_vec(&body).map_err(|err| {
        AppError::validation(format!("Direct OCCT runner plan hashing failed: {}", err))
    })?;
    let mut hasher = Sha256::new();
    hasher.update(&body);
    Ok(format!("sha256:{:x}", hasher.finalize()))
}

fn runner_supports_plan(plan: &OcctPlan) -> bool {
    plan.parts
        .iter()
        .all(|part| part.commands.iter().all(runner_command_supported))
}

fn runner_command_supported(command: &OcctCommand) -> bool {
    if !runner_args_supported(&command.args)
        || !runner_keywords_sources_supported(&command.keywords)
    {
        return false;
    }
    if !runner_op_supported(command.op) {
        return false;
    }
    if command.keywords.is_empty() {
        return true;
    }
    match command.op {
        OcctOp::Box => runner_box_keywords_supported(command),
        OcctOp::Sphere => runner_primitive_align_keywords_supported(command, 1),
        OcctOp::Cylinder => runner_primitive_align_keywords_supported(command, 2),
        OcctOp::Cone => runner_primitive_align_keywords_supported(command, 3),
        OcctOp::Torus => runner_primitive_align_keywords_supported(command, 2),
        OcctOp::Wedge => runner_primitive_align_keywords_supported(command, 7),
        OcctOp::Profile => runner_profile_keywords_supported(command),
        OcctOp::Plane => runner_plane_keywords_supported(command),
        OcctOp::Location => runner_location_keywords_supported(command),
        OcctOp::PathFrame => runner_path_frame_keywords_supported(command),
        OcctOp::ClipBox => runner_clip_box_keywords_supported(command),
        OcctOp::Fillet | OcctOp::Chamfer => runner_exact_edge_selector_supported(command),
        OcctOp::Shell => runner_shell_supported(command),
        OcctOp::Bspline => runner_bspline_keywords_supported(command),
        OcctOp::Sweep => runner_sweep_keywords_supported(command),
        OcctOp::Draft => runner_draft_keywords_supported(command),
        _ => false,
    }
}

/// The runner's `draft_shape` honours a single `:neutral-z` (or `:neutral_z`)
/// numeric keyword; any other keyword stays unsupported so the plan falls
/// back to the executor.
fn runner_draft_keywords_supported(command: &OcctCommand) -> bool {
    command.keywords.iter().all(|keyword| {
        matches!(keyword.name.as_str(), "neutral-z" | "neutral_z")
            && keyword.selector_payload().is_none()
            && matches!(keyword.source_arg(), OcctArg::Number(_))
    })
}

/// The runner's `sweep_shape` honours a single `:frenet` boolean (the trihedron
/// mode used for helical thread spines). Any other keyword stays unsupported so
/// the plan falls back to the executor.
fn runner_sweep_keywords_supported(command: &OcctCommand) -> bool {
    command.keywords.iter().all(|keyword| {
        keyword.name == "frenet"
            && keyword.selector_payload().is_none()
            && matches!(keyword.source_arg(), OcctArg::Boolean(_))
    })
}

fn runner_box_keywords_supported(command: &OcctCommand) -> bool {
    if command.args.len() != 3
        || !command
            .args
            .iter()
            .all(|arg| matches!(arg, OcctArg::Number(_)))
    {
        return false;
    }
    if command.keywords.len() != 1 {
        return false;
    }
    let keyword = &command.keywords[0];
    keyword.name == "align"
        && keyword.selector_payload().is_none()
        && runner_align_tuple_supported(keyword.source_arg())
}

fn runner_primitive_align_keywords_supported(command: &OcctCommand, arg_count: usize) -> bool {
    if command.args.len() < arg_count
        || !command
            .args
            .iter()
            .all(|arg| matches!(arg, OcctArg::Number(_)))
    {
        return false;
    }
    if command.keywords.len() != 1 {
        return false;
    }
    let keyword = &command.keywords[0];
    keyword.name == "align"
        && keyword.selector_payload().is_none()
        && runner_align_tuple_supported(keyword.source_arg())
}

fn runner_profile_keywords_supported(command: &OcctCommand) -> bool {
    if !command.args.is_empty() {
        return false;
    }
    let mut saw_outer = false;
    for keyword in &command.keywords {
        match keyword.name.as_str() {
            "outer" => {
                saw_outer = true;
                if !runner_ref_collection_supported(keyword.source_arg()) {
                    return false;
                }
            }
            "holes" => {
                if !runner_ref_collection_supported(keyword.source_arg()) {
                    return false;
                }
            }
            "fill-rule" => {
                if !matches!(keyword.source_arg(), OcctArg::Text(_)) {
                    return false;
                }
            }
            _ => return false,
        }
        if keyword.selector_payload().is_some() {
            return false;
        }
    }
    saw_outer
}

fn runner_clip_box_keywords_supported(command: &OcctCommand) -> bool {
    if command.args.len() != 1 || !matches!(command.args[0], OcctArg::Ref(_)) {
        return false;
    }
    let mut saw_x = false;
    let mut saw_y = false;
    let mut saw_z = false;
    for keyword in &command.keywords {
        match keyword.name.as_str() {
            "x" => {
                saw_x = true;
                if !runner_range_arg_supported(keyword.source_arg()) {
                    return false;
                }
            }
            "y" => {
                saw_y = true;
                if !runner_range_arg_supported(keyword.source_arg()) {
                    return false;
                }
            }
            "z" => {
                saw_z = true;
                if !runner_range_arg_supported(keyword.source_arg()) {
                    return false;
                }
            }
            _ => return false,
        }
        if keyword.selector_payload().is_some() {
            return false;
        }
    }
    saw_x && saw_y && saw_z
}

fn runner_plane_keywords_supported(command: &OcctCommand) -> bool {
    if !command.args.is_empty() {
        return false;
    }
    for keyword in &command.keywords {
        if keyword.selector_payload().is_some() {
            return false;
        }
        match keyword.name.as_str() {
            "origin" | "x" | "normal" => {
                if !runner_point3_like_arg_supported(keyword.source_arg()) {
                    return false;
                }
            }
            _ => return false,
        }
    }
    true
}

fn runner_location_keywords_supported(command: &OcctCommand) -> bool {
    if command.args.len() > 1
        || command
            .args
            .first()
            .is_some_and(|arg| !matches!(arg, OcctArg::Ref(_)))
    {
        return false;
    }
    command.keywords.iter().all(|keyword| {
        keyword.selector_payload().is_none()
            && matches!(keyword.name.as_str(), "offset" | "rotate")
            && runner_point3_like_arg_supported(keyword.source_arg())
    })
}

fn runner_point3_like_arg_supported(arg: &OcctArg) -> bool {
    matches!(arg, OcctArg::Point3(_))
        || matches!(arg, OcctArg::List(items) if items.len() == 3 && items.iter().all(|item| matches!(item, OcctArg::Number(_))))
}

fn runner_path_frame_keywords_supported(command: &OcctCommand) -> bool {
    if command.args.len() != 1 || !matches!(command.args[0], OcctArg::Ref(_)) {
        return false;
    }
    for keyword in &command.keywords {
        if keyword.selector_payload().is_some() {
            return false;
        }
        match keyword.name.as_str() {
            "at" => {
                if !matches!(
                    keyword.source_arg(),
                    OcctArg::Number(_) | OcctArg::Symbol(_) | OcctArg::Text(_)
                ) {
                    return false;
                }
            }
            "up" => {
                if !matches!(keyword.source_arg(), OcctArg::Point3(_)) {
                    return false;
                }
            }
            _ => return false,
        }
    }
    true
}

fn runner_bspline_keywords_supported(command: &OcctCommand) -> bool {
    if command.args.is_empty() || command.args.len() > 2 {
        return false;
    }
    if !matches!(command.args[0], OcctArg::List(_)) {
        return false;
    }
    if command.args.len() == 2 && !matches!(command.args[1], OcctArg::Boolean(_)) {
        return false;
    }
    for keyword in &command.keywords {
        if keyword.selector_payload().is_some() {
            return false;
        }
        match keyword.name.as_str() {
            "closed" => {
                if !matches!(keyword.source_arg(), OcctArg::Boolean(_)) {
                    return false;
                }
            }
            "tangents" => {
                if !matches!(keyword.source_arg(), OcctArg::List(_)) {
                    return false;
                }
            }
            "tangent_scalars" | "tangent-scalars" => {
                if !matches!(keyword.source_arg(), OcctArg::List(_)) {
                    return false;
                }
            }
            _ => return false,
        }
    }
    true
}

fn runner_ref_collection_supported(arg: &OcctArg) -> bool {
    match arg {
        OcctArg::Ref(_) => true,
        OcctArg::List(items) => items.iter().all(|item| matches!(item, OcctArg::Ref(_))),
        _ => false,
    }
}

fn runner_range_arg_supported(arg: &OcctArg) -> bool {
    match arg {
        OcctArg::Point2(_) => true,
        OcctArg::List(items) if items.len() == 2 => {
            items.iter().all(|item| matches!(item, OcctArg::Number(_)))
        }
        _ => false,
    }
}

fn runner_align_tuple_supported(arg: &OcctArg) -> bool {
    let OcctArg::List(items) = arg else {
        return false;
    };
    if items.len() != 3 {
        return false;
    }
    items.iter().all(|item| {
        matches!(
            item,
            OcctArg::Symbol(value) | OcctArg::Text(value)
                if value == "min" || value == "center" || value == "max"
        )
    })
}

fn runner_exact_edge_selector_supported(command: &OcctCommand) -> bool {
    if command.args.len() != 2 {
        return false;
    }
    if !matches!(command.args[0], OcctArg::Number(_)) || !matches!(command.args[1], OcctArg::Ref(_))
    {
        return false;
    }
    if command.keywords.is_empty() {
        return true;
    }
    command
        .keywords
        .iter()
        .all(|keyword| match keyword.name.as_str() {
            "edges" => matches!(
                keyword.selector_payload(),
                Some(crate::ecky_core_ir::CoreSelectorPayload::EdgeAll)
                    | Some(crate::ecky_core_ir::CoreSelectorPayload::EdgeTargetIds(_))
                    | Some(crate::ecky_core_ir::CoreSelectorPayload::EdgeClauses(_))
            ),
            // Tapered fillet: `:to-radius` rides alongside the edge selector and is
            // consumed by the cpp runner's `fillet_shape`.
            "to-radius" | "to_radius" => {
                matches!(keyword.source_arg(), OcctArg::Number(_))
            }
            _ => false,
        })
}

fn runner_exact_face_selector_supported(command: &OcctCommand) -> bool {
    if command.args.len() != 2 {
        return false;
    }
    if !matches!(command.args[0], OcctArg::Number(_)) || !matches!(command.args[1], OcctArg::Ref(_))
    {
        return false;
    }
    if command.keywords.len() != 1 {
        return false;
    }
    let keyword = &command.keywords[0];
    if keyword.name != "faces" {
        return false;
    }
    matches!(
        keyword.selector_payload(),
        Some(crate::ecky_core_ir::CoreSelectorPayload::FaceTargetIds(_))
            | Some(crate::ecky_core_ir::CoreSelectorPayload::FaceClauses(_))
    )
}

fn runner_shell_supported(command: &OcctCommand) -> bool {
    if command.args.len() != 2 {
        return false;
    }
    if !matches!(command.args[0], OcctArg::Number(_)) || !matches!(command.args[1], OcctArg::Ref(_))
    {
        return false;
    }
    if command.keywords.is_empty() {
        return true;
    }
    runner_exact_face_selector_supported(command)
}

fn runner_op_supported(op: OcctOp) -> bool {
    matches!(
        op,
        OcctOp::Box
            | OcctOp::Sphere
            | OcctOp::Cylinder
            | OcctOp::Cone
            | OcctOp::Torus
            | OcctOp::Wedge
            | OcctOp::Circle
            | OcctOp::Ellipse
            | OcctOp::Slot
            | OcctOp::SlotArc
            | OcctOp::Rectangle
            | OcctOp::RoundedRectangle
            | OcctOp::RoundedPolygon
            | OcctOp::Polygon
            | OcctOp::Profile
            | OcctOp::MakeFace
            | OcctOp::ImportStl
            | OcctOp::ImportStep
            | OcctOp::Solidify
            | OcctOp::Extrude
            | OcctOp::Revolve
            | OcctOp::Loft
            | OcctOp::Sweep
            | OcctOp::Twist
            | OcctOp::Taper
            | OcctOp::Draft
            | OcctOp::Offset
            | OcctOp::Path
            | OcctOp::HelixPath
            | OcctOp::BezierPath
            | OcctOp::Bspline
            | OcctOp::Plane
            | OcctOp::Location
            | OcctOp::PathFrame
            | OcctOp::Place
            | OcctOp::ClipBox
            | OcctOp::LinearArray
            | OcctOp::RadialArray
            | OcctOp::GridArray
            | OcctOp::ArcArray
            | OcctOp::Union
            | OcctOp::Difference
            | OcctOp::Intersection
            | OcctOp::Fillet
            | OcctOp::Chamfer
            | OcctOp::Shell
            | OcctOp::Translate
            | OcctOp::Rotate
            | OcctOp::Scale
            | OcctOp::Mirror
            | OcctOp::Compound
            | OcctOp::Hull
    )
}

fn runner_reported_unsupported(output: &std::process::Output) -> bool {
    let stderr = String::from_utf8_lossy(&output.stderr);
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&stderr) {
        return json["class"] == "validation_error"
            && (json["code"] == "unsupported_op" || json["code"] == "unsupported_selector_form");
    }
    output.status.code() == Some(11) && stderr.contains("unsupported direct OCCT op")
}

fn runner_command(command: &OcctCommand) -> AppResult<Option<RunnerCommand>> {
    if !runner_args_supported(&command.args)
        || !runner_keywords_sources_supported(&command.keywords)
    {
        return Err(AppError::validation(
            "Direct OCCT runner requires resolved args before serialization.".to_string(),
        ));
    }
    if !runner_command_supported(command) {
        return Ok(None);
    }
    let mut keywords = Vec::with_capacity(command.keywords.len());
    for keyword in &command.keywords {
        if matches!(
            keyword.selector_payload(),
            Some(crate::ecky_core_ir::CoreSelectorPayload::EdgeAll)
        ) {
            continue;
        }
        let Some(serialized_keyword) = runner_keyword(keyword)? else {
            return Ok(None);
        };
        keywords.push(serialized_keyword);
    }

    Ok(Some(RunnerCommand {
        output: command.output.0,
        op: runner_op_token(command.op).to_string(),
        args: command
            .args
            .iter()
            .map(runner_arg)
            .collect::<AppResult<Vec<_>>>()?,
        keywords,
    }))
}

fn runner_keyword(keyword: &OcctKeyword) -> AppResult<Option<RunnerKeyword>> {
    match &keyword.value {
        super::direct_occt::OcctKeywordValue::Arg(value) => Ok(Some(RunnerKeyword {
            name: keyword.name.clone(),
            kind: "arg".to_string(),
            value: Some(runner_arg(value)?),
            payload: None,
        })),
        super::direct_occt::OcctKeywordValue::Selector { source, payload } => {
            let Some(payload) = runner_selector_payload(payload)? else {
                return Ok(None);
            };
            Ok(Some(RunnerKeyword {
                name: keyword.name.clone(),
                kind: "selector".to_string(),
                value: Some(runner_arg(source)?),
                payload: Some(payload),
            }))
        }
    }
}

fn runner_selector_payload(
    payload: &crate::ecky_core_ir::CoreSelectorPayload,
) -> AppResult<Option<serde_json::Value>> {
    let value = match payload {
        crate::ecky_core_ir::CoreSelectorPayload::EdgeAll => return Ok(None),
        crate::ecky_core_ir::CoreSelectorPayload::EdgeTargetIds(target_ids) => {
            serde_json::json!({
                "type": "targetIds",
                "kind": "edge",
                "targetIds": target_ids,
            })
        }
        crate::ecky_core_ir::CoreSelectorPayload::EdgeTag(tag_name) => serde_json::json!({
            "type": "targetIds",
            "kind": "edge",
            "targetIds": [format!("tag:{tag_name}")],
        }),
        crate::ecky_core_ir::CoreSelectorPayload::EdgeClauses(clauses) => serde_json::json!({
            "type": "clauses",
            "kind": "edge",
            "clauses": clauses.iter().map(runner_edge_clause).collect::<Vec<_>>(),
        }),
        crate::ecky_core_ir::CoreSelectorPayload::FaceTargetIds(target_ids) => {
            serde_json::json!({
                "type": "targetIds",
                "kind": "face",
                "targetIds": target_ids,
            })
        }
        crate::ecky_core_ir::CoreSelectorPayload::FaceTag(tag_name) => serde_json::json!({
            "type": "targetIds",
            "kind": "face",
            "targetIds": [format!("tag:{tag_name}")],
        }),
        crate::ecky_core_ir::CoreSelectorPayload::FaceClauses(clauses) => serde_json::json!({
            "type": "clauses",
            "kind": "face",
            "clauses": clauses.iter().map(runner_face_clause).collect::<Vec<_>>(),
        }),
    };

    Ok(Some(value))
}

fn runner_edge_clause(clause: &crate::ecky_core_ir::CoreEdgeSelectorClause) -> serde_json::Value {
    match clause {
        crate::ecky_core_ir::CoreEdgeSelectorClause::Axis(axis) => serde_json::json!({
            "type": "axis",
            "axis": runner_edge_axis(axis),
        }),
        crate::ecky_core_ir::CoreEdgeSelectorClause::Boundary { axis, bound } => {
            serde_json::json!({
                "type": "boundary",
                "axis": runner_edge_axis(axis),
                "bound": runner_edge_bound(bound),
            })
        }
    }
}

fn runner_face_clause(clause: &crate::ecky_core_ir::CoreFaceSelectorClause) -> serde_json::Value {
    match clause {
        crate::ecky_core_ir::CoreFaceSelectorClause::Boundary { axis, bound } => {
            serde_json::json!({
                "type": "boundary",
                "axis": runner_edge_axis(axis),
                "bound": runner_edge_bound(bound),
            })
        }
        crate::ecky_core_ir::CoreFaceSelectorClause::Planar => serde_json::json!({
            "type": "planar",
        }),
        crate::ecky_core_ir::CoreFaceSelectorClause::Normal(axis) => serde_json::json!({
            "type": "normal",
            "axis": runner_edge_axis(axis),
        }),
        crate::ecky_core_ir::CoreFaceSelectorClause::Area(rank) => serde_json::json!({
            "type": "area",
            "rank": runner_face_area_rank(rank),
        }),
    }
}

fn runner_edge_axis(axis: &crate::ecky_core_ir::CoreEdgeAxis) -> &'static str {
    match axis {
        crate::ecky_core_ir::CoreEdgeAxis::X => "x",
        crate::ecky_core_ir::CoreEdgeAxis::Y => "y",
        crate::ecky_core_ir::CoreEdgeAxis::Z => "z",
    }
}

fn runner_edge_bound(bound: &crate::ecky_core_ir::CoreEdgeBound) -> &'static str {
    match bound {
        crate::ecky_core_ir::CoreEdgeBound::Min => "min",
        crate::ecky_core_ir::CoreEdgeBound::Max => "max",
    }
}

fn runner_face_area_rank(rank: &crate::ecky_core_ir::CoreFaceAreaRank) -> &'static str {
    match rank {
        crate::ecky_core_ir::CoreFaceAreaRank::Min => "min",
        crate::ecky_core_ir::CoreFaceAreaRank::Max => "max",
    }
}

fn runner_arg(arg: &OcctArg) -> AppResult<RunnerArg> {
    Ok(match arg {
        OcctArg::Number(value) => RunnerArg {
            kind: "number".to_string(),
            value: serde_json::json!(value),
        },
        OcctArg::Boolean(value) => RunnerArg {
            kind: "boolean".to_string(),
            value: serde_json::json!(value),
        },
        OcctArg::Text(value) => RunnerArg {
            kind: "text".to_string(),
            value: serde_json::json!(value),
        },
        OcctArg::Symbol(value) => RunnerArg {
            kind: "symbol".to_string(),
            value: serde_json::json!(value),
        },
        OcctArg::Point2(value) => RunnerArg {
            kind: "point2".to_string(),
            value: serde_json::json!(value),
        },
        OcctArg::Point3(value) => RunnerArg {
            kind: "point3".to_string(),
            value: serde_json::json!(value),
        },
        OcctArg::List(values) => RunnerArg {
            kind: "list".to_string(),
            value: serde_json::Value::Array(
                values
                    .iter()
                    .map(runner_arg)
                    .collect::<AppResult<Vec<_>>>()?
                    .into_iter()
                    .map(|item| serde_json::json!(item))
                    .collect(),
            ),
        },
        OcctArg::Param(value) => {
            return Err(AppError::validation(format!(
                "Direct OCCT runner requires resolved args; unresolved param `{value}` reached runner serialization."
            )));
        }
        OcctArg::Ref(value) => RunnerArg {
            kind: "ref".to_string(),
            value: serde_json::json!(value.0),
        },
    })
}

fn runner_args_supported(args: &[OcctArg]) -> bool {
    args.iter().all(runner_arg_supported)
}

fn runner_arg_supported(arg: &OcctArg) -> bool {
    match arg {
        OcctArg::Param(_) => false,
        OcctArg::List(items) => items.iter().all(runner_arg_supported),
        _ => true,
    }
}

fn runner_keywords_sources_supported(keywords: &[OcctKeyword]) -> bool {
    keywords
        .iter()
        .all(|keyword| runner_arg_supported(keyword.source_arg()))
}

fn runner_op_token(op: OcctOp) -> &'static str {
    match op {
        OcctOp::Box => "box",
        OcctOp::Sphere => "sphere",
        OcctOp::Cylinder => "cylinder",
        OcctOp::Cone => "cone",
        OcctOp::Torus => "torus",
        OcctOp::Wedge => "wedge",
        OcctOp::Circle => "circle",
        OcctOp::Ellipse => "ellipse",
        OcctOp::Slot => "slot-overall",
        OcctOp::SlotArc => "slot-arc",
        OcctOp::Rectangle => "rectangle",
        OcctOp::RoundedRectangle => "rounded-rect",
        OcctOp::RoundedPolygon => "rounded-polygon",
        OcctOp::Polygon => "polygon",
        OcctOp::Profile => "profile",
        OcctOp::MakeFace => "make-face",
        OcctOp::ImportStl => "import-stl",
        OcctOp::ImportStep => "import-step",
        OcctOp::Extrude => "extrude",
        OcctOp::Revolve => "revolve",
        OcctOp::Loft => "loft",
        OcctOp::Sweep => "sweep",
        OcctOp::Twist => "twist",
        OcctOp::Taper => "taper",
        OcctOp::Draft => "draft",
        OcctOp::Offset => "offset",
        OcctOp::Path => "path",
        OcctOp::HelixPath => "helix-path",
        OcctOp::BezierPath => "bezier-path",
        OcctOp::Bspline => "bspline",
        OcctOp::Plane => "plane",
        OcctOp::Location => "location",
        OcctOp::PathFrame => "path-frame",
        OcctOp::Place => "place",
        OcctOp::ClipBox => "clip-box",
        OcctOp::LinearArray => "linear-array",
        OcctOp::RadialArray => "radial-array",
        OcctOp::GridArray => "grid-array",
        OcctOp::ArcArray => "arc-array",
        OcctOp::Union => "union",
        OcctOp::Difference => "difference",
        OcctOp::Intersection => "intersection",
        OcctOp::Fillet => "fillet",
        OcctOp::Chamfer => "chamfer",
        OcctOp::Shell => "shell",
        OcctOp::Translate => "translate",
        OcctOp::Rotate => "rotate",
        OcctOp::Scale => "scale",
        OcctOp::Mirror => "mirror",
        OcctOp::Compound => "compound",
        OcctOp::Hull => "hull",
        OcctOp::Solidify => "solidify",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecky_cad_host::direct_occt::{
        OcctArg, OcctCommand, OcctKeyword, OcctKeywordValue, OcctOp, OcctParameter,
        OcctParameterKind, OcctPartPlan, OcctPlan, OcctSlot,
    };
    use crate::ecky_core_ir::{
        CoreEdgeAxis, CoreEdgeBound, CoreFaceAreaRank, CoreFaceSelectorClause, CoreSelectorPayload,
    };
    use crate::models::PathResolver;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::OnceLock;

    struct TestResolver {
        root: PathBuf,
    }

    impl PathResolver for TestResolver {
        fn app_config_dir(&self) -> PathBuf {
            self.root.join("config")
        }

        fn app_data_dir(&self) -> PathBuf {
            self.root.join("data")
        }

        fn resource_path(&self, path: &str) -> Option<PathBuf> {
            let candidate = self.root.join("resources").join(path);
            candidate.exists().then_some(candidate)
        }
    }

    fn temp_root(label: &str) -> PathBuf {
        let unique = format!(
            "{}-{}-{}",
            label,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        );
        std::env::temp_dir().join(format!("ecky-direct-occt-runner-{}", unique))
    }

    #[test]
    fn native_runner_reports_part_command_context_for_occt_failures() {
        let source = include_str!("../../native/direct_occt_runner.cpp");

        assert!(
            source.contains("Direct OCCT part `")
                && source.contains("command #")
                && source.contains("output slot")
                && source.contains("op `"),
            "native runner must preserve the failing part, command index, output slot, and op"
        );
        assert!(
            source.contains("Direct OCCT export stage `")
                && source.contains("write-step")
                && source.contains("write-preview-stl")
                && source.contains("write-part-stl:"),
            "native runner must preserve the failing STEP/STL export stage"
        );
        assert!(
            source.contains("ExecutionContext")
                && source.contains("context.set_stage(\"write-topology\")")
                && source.contains("context.set_stage(\"assemble-export-shape\")"),
            "native runner must retain render-local context outside command and file-writer wrappers"
        );
    }

    #[test]
    fn runner_stage_report_contract_requires_fixed_order_and_explicit_skips() {
        let report = RunnerStageReport {
            schema_version: 1,
            total_elapsed_ms: 3,
            worker_budget: None,
            parallel_policy: None,
            serial_boolean_count: None,
            parallel_boolean_count: None,
            mesh_boolean_count: None,
            tessellated_step_part_count: None,
            max_nested_kernel_lease: None,
            peak_total_allocated_cpu_units: None,
            peak_dag_concurrency: None,
            mesh_outer_worker_budget: None,
            mesh_pool_budget: None,
            mesh_launcher_budget: None,
            mesh_build_count: None,
            mesh_cache_hit_count: None,
            preview_facet_count: None,
            partial_boolean_cache_hit_count: None,
            partial_boolean_cache_miss_count: None,
            partial_boolean_cache_write_count: None,
            four_way_intersection_count: None,
            parts: Vec::new(),
            commands: Vec::new(),
            partial_boolean_groups: Vec::new(),
            stages: RUNNER_STAGE_NAMES
                .into_iter()
                .map(|name| {
                    let execution_count = u32::from(matches!(name, "boolean" | "mesh" | "export"));
                    RunnerStageReportEntry {
                        name: name.to_string(),
                        status: if execution_count == 0 {
                            "skipped".to_string()
                        } else {
                            "executed".to_string()
                        },
                        execution_count,
                        elapsed_ms: u64::from(execution_count),
                    }
                })
                .collect(),
        };

        validate_runner_stage_report(&report).expect("valid stage report");
    }

    #[test]
    fn native_runner_writes_stage_report_beside_topology() {
        let source = include_str!("../../native/direct_occt_runner.cpp");

        assert!(source.contains("stage-report.json"));
        assert!(source.contains("write_stage_report(stage_report_path, context)"));
        for stage in RUNNER_STAGE_NAMES {
            assert!(
                source.contains(&format!("\"{stage}\"")),
                "missing `{stage}` stage"
            );
        }
        assert!(source.contains("StageExecutionTimer stage_timer(context, \"import\")"));
        assert!(source.contains("StageExecutionTimer stage_timer(context, \"solidify\")"));
        assert!(source.contains("StageExecutionTimer stage_timer(context, \"boolean\")"));
        assert!(source.contains("StageExecutionTimer stage_timer(context, \"mesh\")"));
        assert!(source.contains("StageExecutionTimer stage_timer(context, \"export\")"));
    }

    #[test]
    fn runner_plan_does_not_decompose_unrelated_four_ref_union() {
        let program = crate::ecky_scheme::compile_to_core_program(
            r#"(model (part lid (union
                (box 1 1 1)
                (translate 2 0 0 (box 1 1 1))
                (translate 4 0 0 (box 1 1 1))
                (translate 6 0 0 (box 1 1 1)))))"#,
        )
        .expect("compile");
        let occt = crate::ecky_cad_host::direct_occt::plan_core_program(&program).expect("plan");
        let runner = runner_plan(&occt).expect("runner plan").expect("supported");
        assert!(runner.partial_boolean_groups.is_empty());
    }

    #[test]
    fn runner_plan_does_not_cache_decorated_group_without_indexed_relief_admission() {
        let command = |output, op, args| OcctCommand {
            output: OcctSlot(output),
            op,
            args,
            keywords: Vec::new(),
        };
        let plan = OcctPlan {
            parameters: Vec::new(),
            parts: vec![OcctPartPlan {
                key: "lid".to_string(),
                label: "lid".to_string(),
                root: OcctSlot(9),
                commands: vec![
                    command(1, OcctOp::Box, vec![OcctArg::Number(1.0); 3]),
                    command(2, OcctOp::Sphere, vec![OcctArg::Number(2.0)]),
                    command(
                        3,
                        OcctOp::Cylinder,
                        vec![
                            OcctArg::Number(2.0),
                            OcctArg::Number(1.0),
                            OcctArg::Number(32.0),
                        ],
                    ),
                    command(
                        4,
                        OcctOp::Intersection,
                        vec![OcctArg::Ref(OcctSlot(2)), OcctArg::Ref(OcctSlot(3))],
                    ),
                    command(
                        5,
                        OcctOp::ImportStep,
                        vec![OcctArg::Text("ladybug.step".to_string())],
                    ),
                    command(
                        6,
                        OcctOp::Translate,
                        vec![
                            OcctArg::Number(0.0),
                            OcctArg::Number(0.0),
                            OcctArg::Number(1.0),
                            OcctArg::Ref(OcctSlot(5)),
                        ],
                    ),
                    command(7, OcctOp::Box, vec![OcctArg::Number(1.0); 3]),
                    command(8, OcctOp::Box, vec![OcctArg::Number(1.0); 3]),
                    command(
                        9,
                        OcctOp::Union,
                        vec![
                            OcctArg::Ref(OcctSlot(7)),
                            OcctArg::Ref(OcctSlot(8)),
                            OcctArg::Ref(OcctSlot(4)),
                            OcctArg::Ref(OcctSlot(6)),
                        ],
                    ),
                ],
            }],
        };
        let runner = runner_plan(&plan).expect("runner plan").expect("supported");
        assert!(runner.partial_boolean_groups.is_empty());
        assert_eq!(
            runner.parts[0].representation,
            KernelRepresentation::AnalyticBrep
        );
    }

    #[test]
    fn runner_plan_declares_bracelet_lid_hybrid_and_keeps_relief_indexed() {
        let root = temp_root("decorated-lid-hybrid-plan");
        fs::create_dir_all(&root).expect("fixture dir");
        let stl_path = root.join("relief.stl");
        write_indexed_cube_sidecar(&stl_path);
        let command = |output, op, args| OcctCommand {
            output: OcctSlot(output),
            op,
            args,
            keywords: Vec::new(),
        };
        let plan = OcctPlan {
            parameters: Vec::new(),
            parts: vec![OcctPartPlan {
                key: "lid".to_string(),
                label: "Lid".to_string(),
                root: OcctSlot(9),
                commands: vec![
                    command(1, OcctOp::Box, vec![OcctArg::Number(10.0); 3]),
                    command(2, OcctOp::Box, vec![OcctArg::Number(10.0); 3]),
                    command(3, OcctOp::Sphere, vec![OcctArg::Number(6.0)]),
                    command(
                        4,
                        OcctOp::Cylinder,
                        vec![
                            OcctArg::Number(6.0),
                            OcctArg::Number(6.0),
                            OcctArg::Number(64.0),
                        ],
                    ),
                    command(
                        5,
                        OcctOp::Intersection,
                        vec![OcctArg::Ref(OcctSlot(3)), OcctArg::Ref(OcctSlot(4))],
                    ),
                    command(
                        6,
                        OcctOp::ImportStl,
                        vec![OcctArg::Text(stl_path.to_string_lossy().to_string())],
                    ),
                    command(7, OcctOp::Solidify, vec![OcctArg::Ref(OcctSlot(6))]),
                    command(
                        8,
                        OcctOp::Translate,
                        vec![
                            OcctArg::Number(0.0),
                            OcctArg::Number(0.0),
                            OcctArg::Number(1.0),
                            OcctArg::Ref(OcctSlot(7)),
                        ],
                    ),
                    command(
                        9,
                        OcctOp::Union,
                        vec![
                            OcctArg::Ref(OcctSlot(1)),
                            OcctArg::Ref(OcctSlot(2)),
                            OcctArg::Ref(OcctSlot(5)),
                            OcctArg::Ref(OcctSlot(8)),
                        ],
                    ),
                ],
            }],
        };

        let runner = runner_plan(&plan).expect("runner plan").expect("supported");
        assert_eq!(
            runner.parts[0].representation,
            KernelRepresentation::MeshDomain
        );
        assert_eq!(runner.parts[0].commands[5].op, "import-indexed-mesh");
        assert_eq!(
            runner.partial_boolean_groups[0].representation,
            KernelRepresentation::AnalyticBrep
        );
        assert_eq!(
            runner.partial_boolean_groups[1].representation,
            KernelRepresentation::MeshDomain
        );
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn runner_plan_does_not_label_invalid_relief_as_mesh_domain() {
        let root = temp_root("invalid-decorated-lid-plan");
        fs::create_dir_all(&root).expect("fixture dir");
        let stl_path = root.join("open-relief.stl");
        fs::write(
            &stl_path,
            b"solid open\nfacet normal 0 0 1\nouter loop\nvertex 0 0 0\nvertex 1 0 0\nvertex 0 1 0\nendloop\nendfacet\nendsolid open\n",
        )
        .expect("open STL");
        let mut source = indexed_mesh_boolean_plan(&stl_path, false);
        let make = |output, op, args| OcctCommand {
            output: OcctSlot(output),
            op,
            args,
            keywords: Vec::new(),
        };
        source.parts[0].commands = vec![
            make(
                1,
                OcctOp::ImportStl,
                vec![OcctArg::Text(stl_path.to_string_lossy().to_string())],
            ),
            make(2, OcctOp::Solidify, vec![OcctArg::Ref(OcctSlot(1))]),
            make(3, OcctOp::Box, vec![OcctArg::Number(10.0); 3]),
            make(4, OcctOp::Box, vec![OcctArg::Number(10.0); 3]),
            make(5, OcctOp::Sphere, vec![OcctArg::Number(6.0)]),
            make(
                6,
                OcctOp::Cylinder,
                vec![OcctArg::Number(6.0), OcctArg::Number(6.0)],
            ),
            make(
                7,
                OcctOp::Intersection,
                vec![OcctArg::Ref(OcctSlot(5)), OcctArg::Ref(OcctSlot(6))],
            ),
            make(
                9,
                OcctOp::Union,
                vec![
                    OcctArg::Ref(OcctSlot(3)),
                    OcctArg::Ref(OcctSlot(4)),
                    OcctArg::Ref(OcctSlot(7)),
                    OcctArg::Ref(OcctSlot(2)),
                ],
            ),
        ];
        source.parts[0].root = OcctSlot(9);

        let runner = runner_plan(&source)
            .expect("runner plan")
            .expect("supported");
        assert_eq!(
            runner.parts[0].representation,
            KernelRepresentation::AnalyticBrep
        );
        assert_eq!(runner.parts[0].commands[0].op, "import-stl");
        assert!(runner.partial_boolean_groups.is_empty());
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn runner_plan_moves_pathological_threaded_boolean_closure_to_mesh_domain() {
        let plan = compiled_plan(
            r#"
            (model
              (part threaded-body
                (difference
                  (difference
                    (difference
                      (union (box 40 40 12) (cylinder 22 12 96))
                      (cylinder 15 14 96))
                    (translate 8 0 -1 (cylinder 2 14 48)))
                  (thread :radius 16 :pitch 1.5 :length 6 :depth 0.6))))
            "#,
        );

        let runner = runner_plan(&plan).expect("runner plan").expect("supported");
        assert_eq!(
            runner.parts[0].representation,
            KernelRepresentation::MeshDomain
        );
    }

    #[test]
    fn runner_plan_keeps_threaded_boolean_closure_analytic_when_post_op_needs_brep() {
        let plan = compiled_plan(
            r#"
            (model
              (part threaded-body
                (fillet 0.2
                  (difference
                    (difference
                      (difference
                        (union (box 40 40 12) (cylinder 22 12 96))
                        (cylinder 15 14 96))
                      (translate 8 0 -1 (cylinder 2 14 48)))
                    (thread :radius 16 :pitch 1.5 :length 6 :depth 0.6)))))
            "#,
        );

        let runner = runner_plan(&plan).expect("runner plan").expect("supported");
        assert_eq!(
            runner.parts[0].representation,
            KernelRepresentation::AnalyticBrep
        );
    }

    #[test]
    fn live_runner_stage_report_marks_unused_stages_skipped_when_available() {
        let Some((root, _topology)) =
            run_real_runner_plan_json("live-runner-stage-report", &supported_sample_plan())
        else {
            return;
        };

        let report = read_runner_stage_report(&root.join("bundle")).expect("stage report");
        assert_eq!(
            report
                .stages
                .iter()
                .map(|entry| entry.name.as_str())
                .collect::<Vec<_>>(),
            RUNNER_STAGE_NAMES
        );
        for skipped in [
            "import", "validate", "solidify", "boolean", "cleanup", "verify",
        ] {
            let entry = report
                .stages
                .iter()
                .find(|entry| entry.name == skipped)
                .expect("stage");
            assert_eq!(entry.status, "skipped");
            assert_eq!(entry.execution_count, 0);
            assert_eq!(entry.elapsed_ms, 0);
        }
        assert!(report.total_elapsed_ms > 0);
        assert_eq!(report.stages[5].status, "executed");
        assert_eq!(report.stages[7].status, "executed");

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn given_independent_ready_commands_when_worker_budget_is_four_then_native_runner_reports_overlap(
    ) {
        let root = temp_root("native-dag-ready-overlap");
        let resolver = TestResolver { root: root.clone() };
        let Some(runner) = discover_direct_occt_runner_with_mode(&resolver, true) else {
            return;
        };
        let output_dir = root.join("bundle");
        fs::create_dir_all(&output_dir).expect("output directory");
        fs::write(
            output_dir.join(PLAN_FILE_NAME),
            serialize_runner_plan(&independent_ready_dag_plan())
                .expect("plan serialization")
                .expect("runner plan"),
        )
        .expect("write plan");
        let output = std::process::Command::new(runner)
            .env("ECKY_DIRECT_OCCT_WORKERS", "4")
            .arg("--plan")
            .arg(output_dir.join(PLAN_FILE_NAME))
            .arg("--out")
            .arg(&output_dir)
            .output()
            .expect("start runner");
        assert!(
            output.status.success(),
            "runner failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let report = read_runner_stage_report(&output_dir).expect("stage report");
        assert_eq!(report.worker_budget, Some(4));
        assert!(
            report.peak_dag_concurrency.unwrap_or_default() >= 2,
            "independent ready commands must overlap: {report:#?}"
        );
        assert_eq!(report.parts.len(), 2);
        assert!(report
            .parts
            .iter()
            .all(|part| part.executed_command_count == 5));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn given_complete_part_cache_when_same_plan_rerenders_then_clean_parts_execute_zero_commands() {
        let root = temp_root("native-dag-part-cache");
        let resolver = TestResolver { root: root.clone() };
        let Some(runner) = discover_direct_occt_runner_with_mode(&resolver, true) else {
            return;
        };
        let cache_dir = root.join("selective-cache");
        let plan = serialize_runner_plan(&independent_ready_dag_plan())
            .expect("plan serialization")
            .expect("runner plan");
        for run in ["cold", "warm"] {
            let output_dir = root.join(run);
            fs::create_dir_all(&output_dir).expect("output directory");
            fs::write(output_dir.join(PLAN_FILE_NAME), &plan).expect("write plan");
            let output = std::process::Command::new(&runner)
                .env("ECKY_DIRECT_OCCT_WORKERS", "4")
                .env("ECKY_DIRECT_OCCT_CACHE_DIR", &cache_dir)
                .arg("--plan")
                .arg(output_dir.join(PLAN_FILE_NAME))
                .arg("--out")
                .arg(&output_dir)
                .output()
                .expect("start runner");
            assert!(
                output.status.success(),
                "runner failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        let report = read_runner_stage_report(&root.join("warm")).expect("warm stage report");
        assert_eq!(report.parts.len(), 2);
        assert!(report.parts.iter().all(|part| part.cache_hit));
        assert!(report
            .parts
            .iter()
            .all(|part| part.executed_command_count == 0));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn given_cached_expensive_boolean_when_late_transform_changes_then_only_transform_executes() {
        let root = temp_root("native-dag-command-cache");
        let resolver = TestResolver { root: root.clone() };
        let Some(runner) = discover_direct_occt_runner_with_mode(&resolver, true) else {
            return;
        };
        let cache_dir = root.join("selective-cache");
        for (label, translate_x) in [("cold", 0.0), ("late-change", 7.0)] {
            let output_dir = root.join(label);
            fs::create_dir_all(&output_dir).expect("output directory");
            fs::write(
                output_dir.join(PLAN_FILE_NAME),
                serialize_runner_plan(&boolean_with_late_translate_plan(translate_x))
                    .expect("plan serialization")
                    .expect("runner plan"),
            )
            .expect("write plan");
            let output = std::process::Command::new(&runner)
                .env("ECKY_DIRECT_OCCT_CACHE_DIR", &cache_dir)
                .arg("--plan")
                .arg(output_dir.join(PLAN_FILE_NAME))
                .arg("--out")
                .arg(&output_dir)
                .output()
                .expect("start runner");
            assert!(
                output.status.success(),
                "runner failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        let report = read_runner_stage_report(&root.join("late-change")).expect("stage report");
        let part = report
            .parts
            .iter()
            .find(|part| part.part_id == "cached-boolean")
            .expect("part evidence");
        assert!(!part.cache_hit);
        assert_eq!(part.executed_command_count, 1);
        assert_eq!(part.executed_command_ids, vec!["cached-boolean:4"]);
        assert!(report
            .commands
            .iter()
            .any(|command| command.command_id == "cached-boolean:3"
                && command.cache_admitted
                && command.cache_hit));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn given_late_sibling_failure_when_selective_entries_were_staged_then_no_entry_is_published() {
        let root = temp_root("native-dag-cache-transaction");
        let resolver = TestResolver { root: root.clone() };
        let Some(runner) = discover_direct_occt_runner_with_mode(&resolver, true) else {
            return;
        };
        let cache_dir = root.join("selective-cache");
        let output_dir = root.join("failed-render");
        fs::create_dir_all(&output_dir).expect("output directory");
        let plan = serialize_runner_plan(&boolean_with_late_translate_plan(0.0))
            .expect("plan serialization")
            .expect("runner plan")
            .replacen(
                "\"op\": \"translate\"",
                "\"op\": \"unsupported-late-sibling\"",
                1,
            );
        assert!(
            plan.contains("unsupported-late-sibling"),
            "test plan must contain the late failure"
        );
        fs::write(output_dir.join(PLAN_FILE_NAME), plan).expect("write failing plan");

        let output = std::process::Command::new(runner)
            .env("ECKY_DIRECT_OCCT_CACHE_DIR", &cache_dir)
            .arg("--plan")
            .arg(output_dir.join(PLAN_FILE_NAME))
            .arg("--out")
            .arg(&output_dir)
            .output()
            .expect("start runner");
        assert!(!output.status.success(), "late sibling must fail");
        for kind in ["commands", "parts", "part-meshes"] {
            let directory = cache_dir.join(kind);
            assert!(
                !directory.exists()
                    || fs::read_dir(&directory)
                        .expect("cache directory")
                        .next()
                        .is_none(),
                "failed render published selective cache entries in {}",
                directory.display()
            );
        }
        assert!(
            !cache_dir.read_dir().expect("cache root").any(|entry| {
                entry
                    .expect("cache entry")
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".render-cache-staging-")
            }),
            "failed render left private cache staging behind"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn given_corrupt_selective_brep_when_plan_rerenders_then_entry_is_rejected_and_rebuilt() {
        let root = temp_root("native-dag-corrupt-selective-cache");
        let resolver = TestResolver { root: root.clone() };
        let Some(runner) = discover_direct_occt_runner_with_mode(&resolver, true) else {
            return;
        };
        let cache_dir = root.join("selective-cache");
        let plan = serialize_runner_plan(&independent_ready_dag_plan())
            .expect("plan serialization")
            .expect("runner plan");
        for label in ["cold", "corrupt-rebuild"] {
            let output_dir = root.join(label);
            fs::create_dir_all(&output_dir).expect("output directory");
            fs::write(output_dir.join(PLAN_FILE_NAME), &plan).expect("write plan");
            if label == "corrupt-rebuild" {
                let artifact = fs::read_dir(cache_dir.join("parts"))
                    .expect("part cache directory")
                    .map(|entry| entry.expect("cache entry").path())
                    .find(|path| {
                        path.extension()
                            .is_some_and(|extension| extension == "brepbin")
                    })
                    .expect("part artifact");
                fs::write(artifact, b"corrupt-brep").expect("corrupt artifact");
            }
            let output = std::process::Command::new(&runner)
                .env("ECKY_DIRECT_OCCT_CACHE_DIR", &cache_dir)
                .arg("--plan")
                .arg(output_dir.join(PLAN_FILE_NAME))
                .arg("--out")
                .arg(&output_dir)
                .output()
                .expect("start runner");
            assert!(
                output.status.success(),
                "runner failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        let report = read_runner_stage_report(&root.join("corrupt-rebuild")).expect("stage report");
        assert!(report.parts.iter().any(|part| !part.cache_hit));
        assert!(report
            .parts
            .iter()
            .any(|part| part.executed_command_count == 5));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn given_zero_byte_selective_cache_budget_when_plan_rerenders_then_lru_keeps_no_entries() {
        let root = temp_root("native-dag-selective-cache-lru");
        let resolver = TestResolver { root: root.clone() };
        let Some(runner) = discover_direct_occt_runner_with_mode(&resolver, true) else {
            return;
        };
        let cache_dir = root.join("selective-cache");
        let plan = serialize_runner_plan(&independent_ready_dag_plan())
            .expect("plan serialization")
            .expect("runner plan");
        for label in ["cold", "warm"] {
            let output_dir = root.join(label);
            fs::create_dir_all(&output_dir).expect("output directory");
            fs::write(output_dir.join(PLAN_FILE_NAME), &plan).expect("write plan");
            let output = std::process::Command::new(&runner)
                .env("ECKY_DIRECT_OCCT_CACHE_DIR", &cache_dir)
                .env("ECKY_DIRECT_OCCT_CACHE_BYTES", "0")
                .arg("--plan")
                .arg(output_dir.join(PLAN_FILE_NAME))
                .arg("--out")
                .arg(&output_dir)
                .output()
                .expect("start runner");
            assert!(
                output.status.success(),
                "runner failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        let report = read_runner_stage_report(&root.join("warm")).expect("stage report");
        assert!(report.parts.iter().all(|part| !part.cache_hit));
        assert!(report
            .parts
            .iter()
            .all(|part| part.executed_command_count == 5));
        assert!(
            !cache_dir.join("parts").exists()
                || fs::read_dir(cache_dir.join("parts"))
                    .expect("part cache")
                    .next()
                    .is_none(),
            "zero-byte budget must evict part entries"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn given_two_brep_parts_when_native_runner_exports_then_each_part_mesh_is_built_once_and_reused(
    ) {
        let root = temp_root("native-part-mesh-reuse");
        let resolver = TestResolver { root: root.clone() };
        let Some(runner) = discover_direct_occt_runner_with_mode(&resolver, true) else {
            return;
        };
        let cache_dir = root.join("selective-cache");
        let output_dir = root.join("bundle");
        fs::create_dir_all(&output_dir).expect("output directory");
        fs::write(
            output_dir.join(PLAN_FILE_NAME),
            serialize_runner_plan(&shared_root_part_mesh_plan())
                .expect("plan serialization")
                .expect("runner plan"),
        )
        .expect("write plan");
        let output = std::process::Command::new(runner)
            .env("ECKY_DIRECT_OCCT_WORKERS", "2")
            .env("ECKY_DIRECT_OCCT_CACHE_DIR", &cache_dir)
            .arg("--plan")
            .arg(output_dir.join(PLAN_FILE_NAME))
            .arg("--out")
            .arg(&output_dir)
            .output()
            .expect("start runner");
        assert!(
            output.status.success(),
            "runner failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let report = read_runner_stage_report(&output_dir).expect("stage report");
        assert_eq!(report.parts.len(), 2);
        assert_eq!(report.mesh_outer_worker_budget, Some(1));
        assert_eq!(report.mesh_pool_budget, Some(2));
        assert_eq!(report.mesh_launcher_budget, Some(2));
        assert_eq!(
            report.mesh_build_count,
            Some(report.parts.len() as u32),
            "one final BRep part must produce one tessellation; preview and per-part STL reuse it"
        );
        assert_eq!(report.mesh_cache_hit_count, Some(0));
        assert_eq!(
            report.preview_facet_count,
            Some(
                report
                    .parts
                    .iter()
                    .map(|part| part.mesh_facet_count.expect("per-part facet evidence"))
                    .sum()
            ),
            "preview triangle stream must be deterministic concatenation of part streams"
        );
        assert!(
            report.parts.iter().all(|part| part.mesh_identity.is_some()),
            "per-part STL must identify the exact immutable mesh reused by preview"
        );
        assert_eq!(
            report.parts[0].mesh_identity,
            report.parts[1].mesh_identity,
            "two instances of one root must retain geometry parity while each mesh job owns private topology"
        );

        let warm_dir = root.join("warm");
        fs::create_dir_all(&warm_dir).expect("warm output directory");
        fs::write(
            warm_dir.join(PLAN_FILE_NAME),
            serialize_runner_plan(&shared_root_part_mesh_plan())
                .expect("plan serialization")
                .expect("runner plan"),
        )
        .expect("write warm plan");
        let warm_output = std::process::Command::new(
            discover_direct_occt_runner_with_mode(&resolver, true)
                .expect("runner remains available"),
        )
        .env("ECKY_DIRECT_OCCT_WORKERS", "2")
        .env("ECKY_DIRECT_OCCT_CACHE_DIR", &cache_dir)
        .arg("--plan")
        .arg(warm_dir.join(PLAN_FILE_NAME))
        .arg("--out")
        .arg(&warm_dir)
        .output()
        .expect("start warm runner");
        assert!(
            warm_output.status.success(),
            "warm runner failed: {}",
            String::from_utf8_lossy(&warm_output.stderr)
        );
        let warm_report = read_runner_stage_report(&warm_dir).expect("warm stage report");
        assert_eq!(warm_report.mesh_build_count, Some(0));
        assert_eq!(
            warm_report.mesh_cache_hit_count,
            Some(warm_report.parts.len() as u32)
        );
        assert_eq!(
            warm_report.preview_facet_count, report.preview_facet_count,
            "warm preview must assemble the cached immutable streams in plan order"
        );
        assert_eq!(
            warm_report
                .parts
                .iter()
                .map(|part| part.mesh_identity.as_deref())
                .collect::<Vec<_>>(),
            report
                .parts
                .iter()
                .map(|part| part.mesh_identity.as_deref())
                .collect::<Vec<_>>(),
            "warm cache must reuse each BRep/policy/runtime mesh identity"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn live_runner_stage_report_observes_hybrid_boundary_when_available() {
        let Some((root, _topology)) = run_real_runner_plan_json(
            "live-runner-hybrid-stage-report",
            &solidify_boolean_import_stl_parity_plan(),
        ) else {
            return;
        };

        let report = read_runner_stage_report(&root.join("bundle")).expect("stage report");
        for executed in ["validate", "solidify", "boolean", "mesh", "export"] {
            let entry = report
                .stages
                .iter()
                .find(|entry| entry.name == executed)
                .expect("stage");
            assert_eq!(entry.status, "executed", "{executed}");
            assert!(entry.execution_count > 0, "{executed}");
        }
        for skipped in ["import", "cleanup", "verify"] {
            let entry = report
                .stages
                .iter()
                .find(|entry| entry.name == skipped)
                .expect("stage");
            assert_eq!(entry.status, "skipped", "{skipped}");
            assert_eq!(entry.elapsed_ms, 0, "{skipped}");
        }
        assert!(report.total_elapsed_ms > 0);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn native_runner_fillet_builds_checks_and_retries_once() {
        let source = include_str!("../../native/direct_occt_runner.cpp");

        assert!(source.contains("fillet.Build()"));
        assert!(source.contains("fillet.IsDone()"));
        assert!(source.contains("radius * 0.5"));
    }

    #[test]
    fn native_runner_uses_current_occt_triangle_accessor() {
        let source = include_str!("../../native/direct_occt_runner.cpp");

        assert!(source.contains("triangulation->Triangle(triangle_index)"));
        assert!(!source.contains("triangulation->Triangles()"));
    }

    #[test]
    fn native_runner_keyword_allowlist_includes_path_frame() {
        let source = include_str!("../../native/direct_occt_runner.cpp");

        assert!(source.contains("op != \"draft\" && op != \"path-frame\""));
    }

    #[test]
    fn native_runner_build_treats_occt_headers_as_system_headers() {
        let script = include_str!("../../../scripts/build_direct_occt_runner.sh");

        assert!(script.contains("-isystem\n  \"$OUT_DIR/include/opencascade\""));
        assert!(!script.contains("-I\"$OUT_DIR/include/opencascade\""));
    }

    fn write_executable(path: &Path, contents: impl AsRef<str>) {
        fs::write(path, contents.as_ref()).expect("write script");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut permissions = fs::metadata(path).expect("metadata").permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(path, permissions).expect("chmod");
        }
    }

    fn fake_runner_stage_report_command() -> &'static str {
        "printf '%s' '{\"schemaVersion\":1,\"totalElapsedMs\":0,\"stages\":[{\"name\":\"import\",\"status\":\"skipped\",\"executionCount\":0,\"elapsedMs\":0},{\"name\":\"validate\",\"status\":\"skipped\",\"executionCount\":0,\"elapsedMs\":0},{\"name\":\"solidify\",\"status\":\"skipped\",\"executionCount\":0,\"elapsedMs\":0},{\"name\":\"boolean\",\"status\":\"skipped\",\"executionCount\":0,\"elapsedMs\":0},{\"name\":\"cleanup\",\"status\":\"skipped\",\"executionCount\":0,\"elapsedMs\":0},{\"name\":\"mesh\",\"status\":\"skipped\",\"executionCount\":0,\"elapsedMs\":0},{\"name\":\"verify\",\"status\":\"skipped\",\"executionCount\":0,\"elapsedMs\":0},{\"name\":\"export\",\"status\":\"skipped\",\"executionCount\":0,\"elapsedMs\":0}]}' > \"$out/stage-report.json\"\nexit 0\n"
    }

    fn run_real_runner_plan_json(
        label: &str,
        plan: &OcctPlan,
    ) -> Option<(PathBuf, serde_json::Value)> {
        let root = temp_root(label);
        let resolver = TestResolver { root: root.clone() };
        let runner = discover_direct_occt_runner_with_mode(&resolver, true)?;
        if !runner.is_file() {
            return None;
        }

        let output_dir = root.join("bundle");
        fs::create_dir_all(&output_dir).expect("output dir");
        let plan_json = serialize_runner_plan(plan)
            .expect("plan serialization")
            .expect("runner plan");
        let plan_path = output_dir.join(PLAN_FILE_NAME);
        fs::write(&plan_path, plan_json).expect("write plan");

        let output = std::process::Command::new(&runner)
            .arg("--plan")
            .arg(&plan_path)
            .arg("--out")
            .arg(&output_dir)
            .output()
            .expect("start runner");
        assert!(
            output.status.success(),
            "runner failed: stdout={} stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );

        let topology = serde_json::from_str(
            &fs::read_to_string(output_dir.join("topology.json")).expect("read topology"),
        )
        .expect("parse topology");
        Some((root, topology))
    }

    fn run_real_runner_plan_text(
        label: &str,
        plan_text: &str,
    ) -> Option<(PathBuf, std::process::Output)> {
        let root = temp_root(label);
        let resolver = TestResolver { root: root.clone() };
        let runner = discover_direct_occt_runner_with_mode(&resolver, true)?;
        if !runner.is_file() {
            return None;
        }

        let output_dir = root.join("bundle");
        fs::create_dir_all(&output_dir).expect("output dir");
        let plan_path = output_dir.join(PLAN_FILE_NAME);
        fs::write(&plan_path, plan_text).expect("write plan");

        let output = std::process::Command::new(&runner)
            .arg("--plan")
            .arg(&plan_path)
            .arg("--out")
            .arg(&output_dir)
            .output()
            .expect("start runner");
        Some((root, output))
    }

    fn import_step_plan(step_path: &Path, with_boolean: bool) -> OcctPlan {
        let mut commands = vec![OcctCommand {
            output: OcctSlot(1),
            op: OcctOp::ImportStep,
            args: vec![OcctArg::Text(step_path.to_string_lossy().to_string())],
            keywords: Vec::new(),
        }];
        let root = if with_boolean {
            commands.push(OcctCommand {
                output: OcctSlot(2),
                op: OcctOp::Box,
                args: vec![
                    OcctArg::Number(3.0),
                    OcctArg::Number(3.0),
                    OcctArg::Number(3.0),
                ],
                keywords: Vec::new(),
            });
            commands.push(OcctCommand {
                output: OcctSlot(3),
                op: OcctOp::Difference,
                args: vec![OcctArg::Ref(OcctSlot(1)), OcctArg::Ref(OcctSlot(2))],
                keywords: Vec::new(),
            });
            OcctSlot(3)
        } else {
            OcctSlot(1)
        };
        OcctPlan {
            parameters: Vec::new(),
            parts: vec![OcctPartPlan {
                key: "body".to_string(),
                label: "Body".to_string(),
                root,
                commands,
            }],
        }
    }

    fn multi_solid_step_fixture_plan() -> OcctPlan {
        OcctPlan {
            parameters: Vec::new(),
            parts: vec![
                OcctPartPlan {
                    key: "left".to_string(),
                    label: "Left".to_string(),
                    root: OcctSlot(1),
                    commands: vec![OcctCommand {
                        output: OcctSlot(1),
                        op: OcctOp::Box,
                        args: vec![
                            OcctArg::Number(10.0),
                            OcctArg::Number(10.0),
                            OcctArg::Number(10.0),
                        ],
                        keywords: Vec::new(),
                    }],
                },
                OcctPartPlan {
                    key: "right".to_string(),
                    label: "Right".to_string(),
                    root: OcctSlot(3),
                    commands: vec![
                        OcctCommand {
                            output: OcctSlot(2),
                            op: OcctOp::Box,
                            args: vec![
                                OcctArg::Number(10.0),
                                OcctArg::Number(10.0),
                                OcctArg::Number(10.0),
                            ],
                            keywords: Vec::new(),
                        },
                        OcctCommand {
                            output: OcctSlot(3),
                            op: OcctOp::Translate,
                            args: vec![
                                OcctArg::Number(30.0),
                                OcctArg::Number(0.0),
                                OcctArg::Number(0.0),
                                OcctArg::Ref(OcctSlot(2)),
                            ],
                            keywords: Vec::new(),
                        },
                    ],
                },
            ],
        }
    }

    #[test]
    fn live_runner_imports_valid_and_multi_solid_step_without_repair_when_available() {
        // First export two disconnected solids using the real runner. Reimport
        // the resulting STEP through the same native binary, then exercise a
        // normal boolean. This proves STEP reaches Direct OCCT as BRep and can
        // flow through downstream topology/export without any FreeCAD/STL path.
        let Some((fixture_root, _)) = run_real_runner_plan_json(
            "live-runner-step-multi-fixture",
            &multi_solid_step_fixture_plan(),
        ) else {
            return;
        };
        let fixture_step = fixture_root.join("bundle").join("model.step");
        assert!(fixture_step.is_file(), "fixture STEP was not exported");

        let Some((root, topology)) = run_real_runner_plan_json(
            "live-runner-import-step-multi",
            &import_step_plan(&fixture_step, true),
        ) else {
            let _ = fs::remove_dir_all(fixture_root);
            return;
        };
        let output_dir = root.join("bundle");
        let faces = topology["parts"][0]["faces"].as_array().expect("faces");
        assert!(
            faces.len() >= 6,
            "expected native STEP topology after boolean"
        );
        assert!(output_dir.join("model.step").is_file());
        assert!(output_dir.join("preview.stl").is_file());
        let report = read_runner_stage_report(&output_dir).expect("stage report");
        assert_eq!(report.stages[0].name, "import");
        assert_eq!(report.stages[0].execution_count, 1);
        assert_eq!(report.stages[1].name, "validate");
        assert_eq!(report.stages[1].execution_count, 1);
        assert_eq!(report.stages[2].name, "solidify");
        assert_eq!(report.stages[2].execution_count, 0);

        let _ = fs::remove_dir_all(fixture_root);
        let _ = fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn installed_live_step_component_resolves_cuts_and_renders_natively_when_available() {
        let Some((fixture_root, _)) = run_real_runner_plan_json(
            "live-package-step-fixture",
            &multi_solid_step_fixture_plan(),
        ) else {
            return;
        };
        let fixture_step = fixture_root.join("bundle").join("model.step");
        let package_root = temp_root("live-package-step");
        let project_dir = package_root.join("package");
        fs::create_dir_all(project_dir.join("assets")).expect("package assets");
        fs::copy(&fixture_step, project_dir.join("assets/bracket.step"))
            .expect("copy fixture STEP");
        fs::write(
            project_dir.join(crate::component_package_runtime::COMPONENT_PACKAGE_FILE_NAME),
            serde_json::to_vec_pretty(&serde_json::json!({
                "schemaVersion": 1,
                "packageId": "fixture.native-step",
                "version": "1.0.0",
                "displayName": "Native STEP fixture",
                "visibility": "source",
                "components": [{
                    "componentId": "bracket",
                    "version": "1.0.0",
                    "displayName": "Bracket",
                    "sourceRef": "assets/bracket.step",
                    "geometryProvenance": {
                        "representation": "analyticBrep"
                    }
                }]
            }))
            .expect("package json"),
        )
        .expect("write package manifest");
        let archive = package_root.join("fixture.eckypkg");
        crate::component_package_runtime::write_component_package_archive(&project_dir, &archive)
            .expect("write package archive");
        let resolver = TestResolver {
            root: package_root.clone(),
        };
        crate::component_package_runtime::install_component_package_to_store(&resolver, &archive)
            .expect("install package into CAS");

        let authored = r#"
            (import-component "fixture.native-step" :version "1.0.0" :component "bracket" :as mount)
            (model
              (part body
                (difference
                  (mount)
                  (translate 1 1 1 (box 2 2 2)))))
        "#;
        let compilation = crate::component_import_runtime::compile_authoring_source(
            crate::component_import_runtime::ResolveAuthoringSourceRequest {
                authored_source: authored,
                expected_lock: None,
            },
            &crate::component_import_runtime::InstalledLibraryComponentResolver { app: &resolver },
        )
        .expect("resolve locked STEP component");
        let locked = &compilation.dependency_lock.dependencies[0].components[0];
        assert_eq!(
            locked.payload_kind,
            Some(crate::contracts::ComponentPayloadKind::Step)
        );
        assert_eq!(
            locked.geometry_representation,
            Some(crate::contracts::GeometryRepresentation::AnalyticBrep)
        );
        assert!(compilation.compiler_source.contains("(import-step"));
        assert!(!compilation.compiler_source.contains("(import-component"));
        assert!(!authored.contains(compilation.step_assets[0].path.to_string_lossy().as_ref()));

        let plan = crate::ecky_cad_host::direct_occt::plan_core_program(&compilation.program)
            .expect("plan package STEP component");
        assert!(plan.parts[0]
            .commands
            .iter()
            .any(|command| command.op == OcctOp::ImportStep));
        assert!(!plan.parts[0]
            .commands
            .iter()
            .any(|command| command.op == OcctOp::Solidify));
        let Some((render_root, topology)) =
            run_real_runner_plan_json("live-package-step-render", &plan)
        else {
            let _ = fs::remove_dir_all(fixture_root);
            let _ = fs::remove_dir_all(package_root);
            return;
        };
        assert!(!topology["parts"][0]["faces"]
            .as_array()
            .expect("native faces")
            .is_empty());
        let output_dir = render_root.join("bundle");
        assert!(output_dir.join("model.step").is_file());
        let report = read_runner_stage_report(&output_dir).expect("stage report");
        assert_eq!(report.stages[0].execution_count, 1);
        assert_eq!(report.stages[1].execution_count, 1);
        assert_eq!(report.stages[2].execution_count, 0);

        let forbidden_freecad_marker = package_root.join("freecad-invoked");
        let forbidden_freecad = package_root.join("forbidden-freecad.sh");
        write_executable(
            &forbidden_freecad,
            format!(
                "#!/bin/sh\nprintf invoked > '{}'\nexit 88\n",
                forbidden_freecad_marker.display()
            ),
        );
        let state = crate::models::AppState::new(
            crate::contracts::Config {
                engines: Vec::new(),
                selected_engine_id: String::new(),
                freecad_cmd: forbidden_freecad.to_string_lossy().to_string(),
                cad_text_font_path: String::new(),
                freecad_library_roots: Vec::new(),
                assets: Vec::new(),
                microwave: None,
                voice: crate::contracts::VoiceConfig::default(),
                mcp: crate::contracts::McpConfig::default(),
                has_seen_onboarding: true,
                connection_type: None,
                default_engine_kind: crate::contracts::EngineKind::EckyIrV0,
                default_source_language: crate::contracts::SourceLanguage::EckyIrV0,
                default_geometry_backend: crate::contracts::GeometryBackend::EckyRust,
                max_generation_attempts: 1,
                max_verify_attempts: 0,
                projects_root: None,
            },
            None,
            crate::db::init_db(&package_root.join("test.db")).expect("test db"),
        );
        let service_bundle = crate::services::render::render_model_with_previous_manifest(
            authored,
            &crate::contracts::DesignParams::new(),
            Some(crate::contracts::MacroDialect::EckyIrV0),
            Some(crate::contracts::GeometryBackend::EckyRust),
            None,
            None,
            &state,
            &resolver,
        )
        .await
        .expect("normal render service consumes installed STEP package");
        assert_eq!(
            service_bundle.geometry_backend,
            crate::contracts::GeometryBackend::EckyRust
        );
        assert_eq!(
            service_bundle
                .geometry_provenance
                .as_ref()
                .map(|evidence| evidence.representation.clone()),
            Some(crate::contracts::GeometryRepresentation::AnalyticBrep)
        );
        assert_eq!(service_bundle.component_import_origins.len(), 1);
        assert!(service_bundle
            .export_artifacts
            .iter()
            .any(|artifact| artifact.format.eq_ignore_ascii_case("step")
                && Path::new(&artifact.path).is_file()
                && artifact
                    .geometry_provenance
                    .as_ref()
                    .is_some_and(|evidence| evidence.representation
                        == crate::contracts::GeometryRepresentation::AnalyticBrep)));
        let service_manifest =
            crate::model_runtime::read_model_manifest(&resolver, &service_bundle.model_id)
                .expect("service manifest");
        assert_eq!(
            service_manifest.geometry_provenance,
            service_bundle.geometry_provenance
        );
        assert_eq!(
            service_manifest.component_import_origins,
            service_bundle.component_import_origins
        );
        assert!(
            !forbidden_freecad_marker.exists(),
            "native package composition must never invoke FreeCAD"
        );
        let service_report = read_runner_stage_report(
            Path::new(&service_bundle.manifest_path)
                .parent()
                .expect("runtime bundle dir"),
        )
        .expect("service stage report");
        assert_eq!(service_report.stages[0].execution_count, 1);
        assert_eq!(service_report.stages[1].execution_count, 1);
        assert_eq!(service_report.stages[2].execution_count, 0);

        fs::write(
            &compilation.step_assets[0].path,
            b"mutated after committed lock",
        )
        .expect("mutate installed STEP");
        let error = crate::component_import_runtime::compile_authoring_source(
            crate::component_import_runtime::ResolveAuthoringSourceRequest {
                authored_source: authored,
                expected_lock: Some(&compilation.dependency_lock),
            },
            &crate::component_import_runtime::InstalledLibraryComponentResolver { app: &resolver },
        )
        .expect_err("mutated STEP must fail before native execution");
        assert!(
            error.message.contains("does not match package inventory"),
            "{}",
            error.message
        );

        let legacy_dir = package_root.join("legacy-package");
        fs::create_dir_all(legacy_dir.join("assets")).expect("legacy package assets");
        fs::copy(&fixture_step, legacy_dir.join("assets/bracket.step"))
            .expect("copy legacy fixture STEP");
        fs::write(
            legacy_dir.join(crate::component_package_runtime::COMPONENT_PACKAGE_FILE_NAME),
            serde_json::to_vec_pretty(&serde_json::json!({
                "schemaVersion": 1,
                "packageId": "fixture.legacy-step",
                "version": "1.0.0",
                "displayName": "Legacy STEP fixture",
                "visibility": "source",
                "components": [{
                    "componentId": "bracket",
                    "version": "1.0.0",
                    "displayName": "Bracket",
                    "sourceRef": "assets/bracket.step"
                }]
            }))
            .expect("legacy package json"),
        )
        .expect("write legacy package manifest");
        let legacy_archive = package_root.join("legacy.eckypkg");
        crate::component_package_runtime::write_component_package_archive(
            &legacy_dir,
            &legacy_archive,
        )
        .expect("write legacy package archive");
        crate::component_package_runtime::install_component_package_to_store(
            &resolver,
            &legacy_archive,
        )
        .expect("install legacy STEP package");
        let legacy_authored = r#"
            (import-component "fixture.legacy-step" :version "1.0.0" :component "bracket" :as mount)
            (model (part body (mount)))
        "#;
        let error = crate::component_import_runtime::compile_authoring_source(
            crate::component_import_runtime::ResolveAuthoringSourceRequest {
                authored_source: legacy_authored,
                expected_lock: None,
            },
            &crate::component_import_runtime::InstalledLibraryComponentResolver { app: &resolver },
        )
        .expect_err("legacy STEP without provenance must require repackaging");
        assert!(error.message.contains("repackage"), "{}", error.message);

        let _ = fs::remove_dir_all(fixture_root);
        let _ = fs::remove_dir_all(render_root);
        let _ = fs::remove_dir_all(package_root);
    }

    #[test]
    fn live_runner_step_read_failure_publishes_no_exports_when_available() {
        let missing = std::env::temp_dir().join(format!(
            "ecky-missing-native-step-{}.step",
            uuid::Uuid::new_v4()
        ));
        let plan = import_step_plan(&missing, false);
        let plan_text = serialize_runner_plan(&plan)
            .expect("serialize plan")
            .expect("runner-safe STEP plan");
        let Some((root, output)) =
            run_real_runner_plan_text("live-runner-import-step-missing", &plan_text)
        else {
            return;
        };
        assert!(
            !output.status.success(),
            "missing STEP unexpectedly succeeded"
        );
        let error: serde_json::Value =
            serde_json::from_slice(&output.stderr).expect("structured runner error");
        assert!(
            error["message"]
                .as_str()
                .unwrap_or_default()
                .contains("ReadFile"),
            "{error}"
        );
        let output_dir = root.join("bundle");
        assert!(!output_dir.join("model.step").exists());
        assert!(!output_dir.join("preview.stl").exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn live_runner_rejects_zero_transferred_step_roots_when_available() {
        // `ReadFile` accepts a syntactically valid but rootless ISO-10303-21
        // document. `TransferRoots` must then gate publication before
        // `OneShape` or any downstream operation can run.
        let empty_step = std::env::temp_dir().join(format!(
            "ecky-zero-step-roots-{}.step",
            uuid::Uuid::new_v4()
        ));
        fs::write(
            &empty_step,
            "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION(('empty'),'2;1');\nFILE_NAME('empty.step','2026-07-29T00:00:00',('ecky'),('ecky'),'ecky','ecky','');\nFILE_SCHEMA(('AUTOMOTIVE_DESIGN'));\nENDSEC;\nDATA;\nENDSEC;\nEND-ISO-10303-21;\n",
        )
        .expect("write rootless STEP");
        let plan = import_step_plan(&empty_step, false);
        let plan_text = serialize_runner_plan(&plan)
            .expect("serialize plan")
            .expect("runner-safe STEP plan");
        let Some((root, output)) =
            run_real_runner_plan_text("live-runner-import-step-zero-roots", &plan_text)
        else {
            fs::remove_file(empty_step).ok();
            return;
        };
        assert!(
            !output.status.success(),
            "rootless STEP unexpectedly succeeded"
        );
        let error: serde_json::Value =
            serde_json::from_slice(&output.stderr).expect("structured runner error");
        assert!(
            error["message"]
                .as_str()
                .unwrap_or_default()
                .contains("zero roots"),
            "{error}"
        );
        let output_dir = root.join("bundle");
        assert!(!output_dir.join("model.step").exists());
        assert!(!output_dir.join("preview.stl").exists());
        fs::remove_file(empty_step).ok();
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn live_runner_rejects_shell_only_step_without_hidden_repair_when_available() {
        let shell_fixture = OcctPlan {
            parameters: Vec::new(),
            parts: vec![OcctPartPlan {
                key: "face".to_string(),
                label: "Face".to_string(),
                root: OcctSlot(1),
                commands: vec![OcctCommand {
                    output: OcctSlot(1),
                    op: OcctOp::Rectangle,
                    args: vec![OcctArg::Number(10.0), OcctArg::Number(10.0)],
                    keywords: Vec::new(),
                }],
            }],
        };
        let Some((fixture_root, _)) =
            run_real_runner_plan_json("live-runner-step-shell-fixture", &shell_fixture)
        else {
            return;
        };
        let fixture_step = fixture_root.join("bundle").join("model.step");
        let import = import_step_plan(&fixture_step, false);
        let plan_text = serialize_runner_plan(&import)
            .expect("serialize plan")
            .expect("runner-safe STEP plan");
        let Some((root, output)) =
            run_real_runner_plan_text("live-runner-import-step-shell", &plan_text)
        else {
            let _ = fs::remove_dir_all(fixture_root);
            return;
        };
        assert!(
            !output.status.success(),
            "shell-only STEP unexpectedly succeeded"
        );
        let error: serde_json::Value =
            serde_json::from_slice(&output.stderr).expect("structured runner error");
        assert!(
            error["message"]
                .as_str()
                .unwrap_or_default()
                .contains("shell-only"),
            "{error}"
        );
        let output_dir = root.join("bundle");
        assert!(!output_dir.join("model.step").exists());
        assert!(!output_dir.join("preview.stl").exists());
        let _ = fs::remove_dir_all(fixture_root);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn native_runner_boolean_builder_adapts_pool_to_outer_dag_budget_obb_and_nary_union() {
        let source = include_str!("../../native/direct_occt_runner.cpp");
        assert!(source.contains("class BooleanParallelLease"), "{source}");
        assert!(source.contains("fair_share"), "{source}");
        assert!(
            source.contains("filler.SetRunParallel(parallel_lease.runs_parallel());"),
            "{source}"
        );
        assert!(
            source.contains("builder.SetNonDestructive(false);"),
            "{source}"
        );
        assert!(source.contains("builder.SetUseOBB(true);"), "{source}");
        assert!(
            source.contains("fuse_shapes(shapes_to_fuse, context)"),
            "{source}"
        );
    }

    #[test]
    fn serializes_plane_list_origin_and_location_rotate_point3_or_list() {
        let plan = OcctPlan {
            parameters: Vec::new(),
            parts: vec![OcctPartPlan {
                key: "body".to_string(),
                label: "Body".to_string(),
                root: OcctSlot(4),
                commands: vec![
                    OcctCommand {
                        output: OcctSlot(1),
                        op: OcctOp::Plane,
                        args: Vec::new(),
                        keywords: vec![
                            OcctKeyword::arg(
                                "origin".to_string(),
                                OcctArg::List(vec![
                                    OcctArg::Number(2.0),
                                    OcctArg::Number(3.0),
                                    OcctArg::Number(4.0),
                                ]),
                            ),
                            OcctKeyword::arg(
                                "normal".to_string(),
                                OcctArg::Point3([0.0, 0.0, 1.0]),
                            ),
                        ],
                    },
                    OcctCommand {
                        output: OcctSlot(2),
                        op: OcctOp::Location,
                        args: vec![OcctArg::Ref(OcctSlot(1))],
                        keywords: vec![OcctKeyword::arg(
                            "rotate".to_string(),
                            OcctArg::Point3([0.0, 0.0, 90.0]),
                        )],
                    },
                    OcctCommand {
                        output: OcctSlot(3),
                        op: OcctOp::Location,
                        args: vec![OcctArg::Ref(OcctSlot(2))],
                        keywords: vec![OcctKeyword::arg(
                            "rotate".to_string(),
                            OcctArg::List(vec![
                                OcctArg::Number(0.0),
                                OcctArg::Number(90.0),
                                OcctArg::Number(0.0),
                            ]),
                        )],
                    },
                    OcctCommand {
                        output: OcctSlot(4),
                        op: OcctOp::Place,
                        args: vec![OcctArg::Ref(OcctSlot(3)), OcctArg::Ref(OcctSlot(5))],
                        keywords: Vec::new(),
                    },
                    OcctCommand {
                        output: OcctSlot(5),
                        op: OcctOp::Box,
                        args: vec![
                            OcctArg::Number(2.0),
                            OcctArg::Number(3.0),
                            OcctArg::Number(4.0),
                        ],
                        keywords: Vec::new(),
                    },
                ],
            }],
        };

        let json = serialize_runner_plan(&plan)
            .expect("plan serialization")
            .expect("runner accepts plane/location point3-like forms");
        let value: serde_json::Value = serde_json::from_str(&json).expect("runner JSON");
        assert_eq!(
            value["parts"][0]["commands"][0]["keywords"][0]["value"]["kind"],
            "list"
        );
        assert_eq!(
            value["parts"][0]["commands"][1]["keywords"][0]["value"]["kind"],
            "point3"
        );
        assert_eq!(
            value["parts"][0]["commands"][2]["keywords"][0]["value"]["kind"],
            "list"
        );

        let mut native_plan = plan.clone();
        let box_command = native_plan.parts[0].commands.remove(4);
        native_plan.parts[0].commands.insert(0, box_command);
        let Some((root, _topology)) =
            run_real_runner_plan_json("plane-location-point3-like", &native_plan)
        else {
            return;
        };
        assert!(root.join("bundle/model.step").is_file());
        assert!(root.join("bundle/preview.stl").is_file());
        fs::remove_dir_all(root).expect("cleanup runner bundle");
    }

    #[test]
    fn native_runner_treats_singleton_union_as_identity() {
        let plan = OcctPlan {
            parameters: Vec::new(),
            parts: vec![OcctPartPlan {
                key: "body".to_string(),
                label: "Body".to_string(),
                root: OcctSlot(2),
                commands: vec![
                    OcctCommand {
                        output: OcctSlot(1),
                        op: OcctOp::Box,
                        args: vec![
                            OcctArg::Number(2.0),
                            OcctArg::Number(3.0),
                            OcctArg::Number(4.0),
                        ],
                        keywords: Vec::new(),
                    },
                    OcctCommand {
                        output: OcctSlot(2),
                        op: OcctOp::Union,
                        args: vec![OcctArg::Ref(OcctSlot(1))],
                        keywords: Vec::new(),
                    },
                ],
            }],
        };
        let Some((root, topology)) = run_real_runner_plan_json("singleton-union", &plan) else {
            return;
        };
        assert_eq!(topology["parts"][0]["partId"], "body");
        assert!(root.join("bundle/model.step").is_file());
        fs::remove_dir_all(root).expect("cleanup runner bundle");
    }

    fn sample_plan() -> OcctPlan {
        OcctPlan {
            parameters: vec![OcctParameter {
                key: "width".to_string(),
                kind: OcctParameterKind::Number,
            }],
            parts: vec![OcctPartPlan {
                key: "body".to_string(),
                label: "Body".to_string(),
                root: OcctSlot(1),
                commands: vec![OcctCommand {
                    output: OcctSlot(1),
                    op: OcctOp::RoundedRectangle,
                    args: vec![OcctArg::Number(12.0), OcctArg::Param("width".to_string())],
                    keywords: vec![OcctKeyword {
                        name: "faces".to_string(),
                        value: OcctKeywordValue::Selector {
                            source: OcctArg::Ref(OcctSlot(1)),
                            payload: CoreSelectorPayload::FaceClauses(vec![
                                CoreFaceSelectorClause::Planar,
                                CoreFaceSelectorClause::Normal(CoreEdgeAxis::Z),
                                CoreFaceSelectorClause::Area(CoreFaceAreaRank::Max),
                                CoreFaceSelectorClause::Boundary {
                                    axis: CoreEdgeAxis::X,
                                    bound: CoreEdgeBound::Min,
                                },
                            ]),
                        },
                    }],
                }],
            }],
        }
    }

    fn user_honjo_stay_clamp_program() -> crate::ecky_core_ir::CoreProgram {
        crate::ecky_scheme::compile_to_core_program(
            r#"
            (model
              (params
                (number tube_od 14.5 :label "Feather Tube OD" :min 11 :max 24 :step 0.1)
                (number clamp_clearance 0.20 :label "Slicing Fit Clearance" :min 0.05 :max 0.5 :step 0.05)
                (number clamp_wall 3.5 :label "Clamp Wall Thickness" :min 2.5 :max 5.5 :step 0.1)
                (number part_width 16.0 :label "Print Width along Stay" :min 12 :max 24 :step 0.5)
                (number honjo_wire_d 5.2 :label "Honjo Wire Diameter" :min 4.8 :max 5.5 :step 0.05)
                (number hardware_m4_d 4.2 :label "M4 Clearance Bolt Diameter" :min 3.8 :max 4.5 :step 0.05)
                (number nut_flat 7.2 :label "M4 Hex Nut Flat Width" :min 6.8 :max 7.6 :step 0.05)
                (number nut_thick 3.2 :label "M4 Hex Nut Depth" :min 2.5 :max 4.0 :step 0.1)
                (number boss_offset 12.0 :label "Stay Extension Offset" :min 8 :max 20 :step 0.5))

              (part honjo_stay_clamp_v2
                (let* ((inner_r (/ (+ tube_od clamp_clearance) 2))
                       (outer_r (+ inner_r clamp_wall))
                       (ear_w (* clamp_wall 2))
                       (ear_l 12.0)
                       (boss_r (+ (/ honjo_wire_d 2) clamp_wall))
                       (z_center (/ part_width 2)))
                  (build
                    (shape collar_shell
                      (cylinder outer_r part_width 64 :align '(center center min)))
                    (shape clamping_ear_left
                      (translate (- 0 outer_r (/ ear_l 2)) 0 0
                        (box ear_l ear_w part_width :align '(center center min))))
                    (shape clamping_ear_right
                      (translate (+ outer_r (/ ear_l 2)) 0 0
                        (box ear_l ear_w part_width :align '(center center min))))
                    (shape neck_bridge
                      (translate 0 (+ outer_r (/ boss_offset 2)) 0
                        (box (* boss_r 1.6) boss_offset part_width :align '(center center min))))
                    (shape wire_boss
                      (translate 0 (+ outer_r boss_offset) 0
                        (cylinder boss_r part_width 48 :align '(center center min))))
                    (shape full_blank
                      (union collar_shell clamping_ear_left clamping_ear_right neck_bridge wire_boss))
                    (shape tube_void
                      (translate 0 0 -1
                        (cylinder inner_r (+ part_width 2) 64 :align '(center center min))))
                    (shape bolt_void_L
                      (translate (- 0 outer_r (/ ear_l 2)) 0 z_center
                        (rotate 90 0 0
                          (cylinder (/ hardware_m4_d 2) (+ ear_w 4) 32 :align '(center center center)))))
                    (shape bolt_void_R
                      (translate (+ outer_r (/ ear_l 2)) 0 z_center
                        (rotate 90 0 0
                          (cylinder (/ hardware_m4_d 2) (+ ear_w 4) 32 :align '(center center center)))))
                    (shape honjo_stay_void
                      (translate 0 (+ outer_r boss_offset) -1
                        (cylinder (/ honjo_wire_d 2) (+ part_width 2) 48 :align '(center center min))))
                    (shape nut_trap_L
                      (translate (- 0 outer_r 4.5) (- 0 (/ ear_w 2) 0.1) z_center
                        (box nut_thick nut_flat nut_flat :align '(center min center))))
                    (shape nut_trap_R
                      (translate (+ outer_r 4.5) (- 0 (/ ear_w 2) 0.1) z_center
                        (box nut_thick nut_flat nut_flat :align '(center min center))))
                    (shape split_line_cutter
                      (translate 0 (+ (/ boss_offset 2) 5) -1
                        (box (+ (* outer_r 2) (* ear_l 2) 20) (+ (* outer_r 2) boss_offset 30) (+ part_width 2) :align '(center center min))))
                    (result
                      (difference full_blank
                                  tube_void
                                  bolt_void_L
                                  bolt_void_R
                                  honjo_stay_void
                                  nut_trap_L
                                  nut_trap_R
                                  split_line_cutter))))))
            "#,
        )
        .expect("program")
    }

    fn user_honjo_stay_clamp_params() -> crate::contracts::DesignParams {
        use crate::contracts::ParamValue;

        [
            ("tube_od".to_string(), ParamValue::Number(14.5)),
            ("clamp_clearance".to_string(), ParamValue::Number(0.2)),
            ("clamp_wall".to_string(), ParamValue::Number(3.5)),
            ("part_width".to_string(), ParamValue::Number(16.0)),
            ("honjo_wire_d".to_string(), ParamValue::Number(5.2)),
            ("hardware_m4_d".to_string(), ParamValue::Number(4.2)),
            ("nut_flat".to_string(), ParamValue::Number(7.2)),
            ("nut_thick".to_string(), ParamValue::Number(3.2)),
            ("boss_offset".to_string(), ParamValue::Number(12.0)),
        ]
        .into_iter()
        .collect()
    }

    fn unsupported_resolved_selector_plan() -> OcctPlan {
        OcctPlan {
            parameters: Vec::new(),
            parts: vec![OcctPartPlan {
                key: "body".to_string(),
                label: "Body".to_string(),
                root: OcctSlot(1),
                commands: vec![OcctCommand {
                    output: OcctSlot(1),
                    op: OcctOp::RoundedRectangle,
                    args: vec![OcctArg::Number(12.0), OcctArg::Number(24.0)],
                    keywords: vec![OcctKeyword {
                        name: "faces".to_string(),
                        value: OcctKeywordValue::Selector {
                            source: OcctArg::Ref(OcctSlot(1)),
                            payload: CoreSelectorPayload::FaceClauses(vec![
                                CoreFaceSelectorClause::Planar,
                                CoreFaceSelectorClause::Normal(CoreEdgeAxis::Z),
                                CoreFaceSelectorClause::Area(CoreFaceAreaRank::Max),
                                CoreFaceSelectorClause::Boundary {
                                    axis: CoreEdgeAxis::X,
                                    bound: CoreEdgeBound::Min,
                                },
                            ]),
                        },
                    }],
                }],
            }],
        }
    }

    fn supported_sample_plan() -> OcctPlan {
        sample_plan_for_command(OcctCommand {
            output: OcctSlot(1),
            op: OcctOp::Box,
            args: vec![
                OcctArg::Number(12.0),
                OcctArg::Number(8.0),
                OcctArg::Number(4.0),
            ],
            keywords: Vec::new(),
        })
    }

    fn independent_ready_dag_plan() -> OcctPlan {
        let part = |key: &str, offset: f64| OcctPartPlan {
            key: key.to_string(),
            label: key.to_string(),
            root: OcctSlot(5),
            commands: vec![
                OcctCommand {
                    output: OcctSlot(1),
                    op: OcctOp::Sphere,
                    args: vec![OcctArg::Number(18.0 + offset)],
                    keywords: Vec::new(),
                },
                OcctCommand {
                    output: OcctSlot(2),
                    op: OcctOp::Sphere,
                    args: vec![OcctArg::Number(19.0 + offset)],
                    keywords: Vec::new(),
                },
                OcctCommand {
                    output: OcctSlot(3),
                    op: OcctOp::Sphere,
                    args: vec![OcctArg::Number(20.0 + offset)],
                    keywords: Vec::new(),
                },
                OcctCommand {
                    output: OcctSlot(4),
                    op: OcctOp::Sphere,
                    args: vec![OcctArg::Number(21.0 + offset)],
                    keywords: Vec::new(),
                },
                OcctCommand {
                    output: OcctSlot(5),
                    op: OcctOp::Compound,
                    args: vec![
                        OcctArg::Ref(OcctSlot(1)),
                        OcctArg::Ref(OcctSlot(2)),
                        OcctArg::Ref(OcctSlot(3)),
                        OcctArg::Ref(OcctSlot(4)),
                    ],
                    keywords: Vec::new(),
                },
            ],
        };
        OcctPlan {
            parameters: Vec::new(),
            parts: vec![part("left", 0.0), part("right", 30.0)],
        }
    }

    fn shared_root_part_mesh_plan() -> OcctPlan {
        let mut plan = independent_ready_dag_plan();
        let mut instance = plan.parts[0].clone();
        instance.key = "right".to_string();
        instance.label = "right".to_string();
        plan.parts = vec![plan.parts[0].clone(), instance];
        plan
    }

    fn boolean_with_late_translate_plan(translate_x: f64) -> OcctPlan {
        OcctPlan {
            parameters: Vec::new(),
            parts: vec![OcctPartPlan {
                key: "cached-boolean".to_string(),
                label: "cached-boolean".to_string(),
                root: OcctSlot(4),
                commands: vec![
                    OcctCommand {
                        output: OcctSlot(1),
                        op: OcctOp::Box,
                        args: vec![
                            OcctArg::Number(20.0),
                            OcctArg::Number(20.0),
                            OcctArg::Number(20.0),
                        ],
                        keywords: Vec::new(),
                    },
                    OcctCommand {
                        output: OcctSlot(2),
                        op: OcctOp::Cylinder,
                        args: vec![OcctArg::Number(4.0), OcctArg::Number(30.0)],
                        keywords: Vec::new(),
                    },
                    OcctCommand {
                        output: OcctSlot(3),
                        op: OcctOp::Difference,
                        args: vec![OcctArg::Ref(OcctSlot(1)), OcctArg::Ref(OcctSlot(2))],
                        keywords: Vec::new(),
                    },
                    OcctCommand {
                        output: OcctSlot(4),
                        op: OcctOp::Translate,
                        args: vec![
                            OcctArg::Number(translate_x),
                            OcctArg::Number(0.0),
                            OcctArg::Number(0.0),
                            OcctArg::Ref(OcctSlot(3)),
                        ],
                        keywords: Vec::new(),
                    },
                ],
            }],
        }
    }

    fn indexed_mesh_boolean_plan(stl_path: &Path, post_boolean: bool) -> OcctPlan {
        let mut commands = vec![
            OcctCommand {
                output: OcctSlot(1),
                op: OcctOp::ImportStl,
                args: vec![OcctArg::Text(stl_path.to_string_lossy().to_string())],
                keywords: Vec::new(),
            },
            OcctCommand {
                output: OcctSlot(2),
                op: OcctOp::Solidify,
                args: vec![OcctArg::Ref(OcctSlot(1))],
                keywords: Vec::new(),
            },
            OcctCommand {
                output: OcctSlot(3),
                op: OcctOp::Cylinder,
                args: vec![OcctArg::Number(0.5), OcctArg::Number(4.0)],
                keywords: Vec::new(),
            },
            OcctCommand {
                output: OcctSlot(4),
                op: OcctOp::Difference,
                args: vec![OcctArg::Ref(OcctSlot(2)), OcctArg::Ref(OcctSlot(3))],
                keywords: Vec::new(),
            },
        ];
        let root = if post_boolean {
            commands.push(OcctCommand {
                output: OcctSlot(5),
                op: OcctOp::Chamfer,
                args: vec![OcctArg::Number(0.1), OcctArg::Ref(OcctSlot(4))],
                keywords: Vec::new(),
            });
            OcctSlot(5)
        } else {
            OcctSlot(4)
        };
        sample_plan_for_commands(root, commands)
    }

    fn write_indexed_cube_sidecar(stl_path: &Path) {
        fs::write(stl_path, b"solid indexed\nendsolid indexed\n").expect("stl fixture");
        let asset = crate::ecky_ir::mesh_asset::IndexedMeshAsset::new(
            crate::ecky_ir::mesh_asset::MeshAssetSource::Imported,
            vec![
                [-1.0, -1.0, -1.0],
                [1.0, -1.0, -1.0],
                [1.0, 1.0, -1.0],
                [-1.0, 1.0, -1.0],
                [-1.0, -1.0, 1.0],
                [1.0, -1.0, 1.0],
                [1.0, 1.0, 1.0],
                [-1.0, 1.0, 1.0],
            ],
            vec![
                [0, 2, 1],
                [0, 3, 2],
                [4, 5, 6],
                [4, 6, 7],
                [0, 1, 5],
                [0, 5, 4],
                [1, 2, 6],
                [1, 6, 5],
                [2, 3, 7],
                [2, 7, 6],
                [3, 0, 4],
                [3, 4, 7],
            ],
        )
        .expect("indexed cube");
        asset
            .write_cache(&stl_path.with_extension("indexed-mesh.json"))
            .expect("indexed sidecar");
    }

    fn write_closed_tetra_stl(stl_path: &Path) {
        let triangles = [
            [[0.0_f32, 0.0, 0.0], [0.0, 1.0, 0.0], [1.0, 0.0, 0.0]],
            [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]],
            [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            [[0.0, 1.0, 0.0], [0.0, 0.0, 0.0], [0.0, 0.0, 1.0]],
        ];
        let mut bytes = vec![0_u8; 80];
        bytes.extend_from_slice(&(triangles.len() as u32).to_le_bytes());
        for triangle in triangles {
            bytes.extend_from_slice(&[0_u8; 12]);
            for vertex in triangle {
                for coordinate in vertex {
                    bytes.extend_from_slice(&coordinate.to_le_bytes());
                }
            }
            bytes.extend_from_slice(&0_u16.to_le_bytes());
        }
        fs::write(stl_path, bytes).expect("tetra STL");
    }

    #[test]
    fn runner_plan_inlines_valid_indexed_mesh_for_root_boolean_only() {
        let root = temp_root("indexed-mesh-runner-plan");
        fs::create_dir_all(&root).expect("fixture dir");
        let stl_path = root.join("island.stl");
        write_indexed_cube_sidecar(&stl_path);

        let plan = runner_plan(&indexed_mesh_boolean_plan(&stl_path, false))
            .expect("runner plan")
            .expect("supported plan");
        let import = &plan.parts[0].commands[0];
        assert_eq!(import.op, "import-indexed-mesh");
        assert_eq!(import.args.len(), 3);
        assert_eq!(import.args[0].kind, "list");
        assert_eq!(import.args[1].kind, "list");
        assert_eq!(import.args[2].kind, "text");

        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn runner_plan_admits_indexed_root_booleans_per_part() {
        let root = temp_root("indexed-mesh-multipart-runner-plan");
        fs::create_dir_all(&root).expect("fixture dir");
        let stl_path = root.join("island.stl");
        write_indexed_cube_sidecar(&stl_path);

        let mut source = indexed_mesh_boolean_plan(&stl_path, false);
        let mut second = source.parts[0].clone();
        second.key = "second".to_string();
        second.label = "Second".to_string();
        source.parts.push(second);

        let plan = runner_plan(&source)
            .expect("runner plan")
            .expect("supported plan");
        assert_eq!(plan.parts.len(), 2);
        assert!(
            plan.parts
                .iter()
                .all(|part| part.commands[0].op == "import-indexed-mesh"),
            "each root Boolean part must keep the imported STL on the Manifold path"
        );

        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn runner_plan_builds_indexed_import_when_sidecar_is_missing() {
        let root = temp_root("indexed-mesh-sidecar-missing");
        fs::create_dir_all(&root).expect("fixture dir");
        let stl_path = root.join("island.stl");
        write_closed_tetra_stl(&stl_path);

        let plan = runner_plan(&indexed_mesh_boolean_plan(&stl_path, false))
            .expect("runner plan")
            .expect("supported plan");
        assert_eq!(plan.parts[0].commands[0].op, "import-indexed-mesh");
        assert!(
            !stl_path.with_extension("indexed-mesh.json").exists(),
            "admission fallback must remain in memory"
        );

        fs::remove_dir_all(root).expect("cleanup");
    }

    // --- Standalone STL/3MF authored-coordinate admission (task 6) ---------

    /// Two disjoint tetrahedra where tetra B is tetra A shifted by 1 nm in X.
    /// The authored (`from_stl`) decode keeps all 8 vertices bit-distinct; the
    /// evaluated-CAD weld (`from_ir_mesh`, 1e-6 mm) collapses each pair, so the
    /// two paths produce different vertex counts and digests. Coordinates are
    /// authored as ASCII (f64) so the 1e-9 shift survives verbatim and the two
    /// paths are distinguishable at the runner.
    fn write_two_tetra_authored_seam_ascii_stl(path: &Path) {
        let a = [
            [0.0_f64, 0.0, 0.0],
            [2.0, 0.0, 0.0],
            [1.0, 2.0, 0.0],
            [1.0, 1.0, 2.0],
        ];
        let shift = [1.0e-9_f64, 0.0, 0.0];
        let b = a.map(|vertex| {
            [
                vertex[0] + shift[0],
                vertex[1] + shift[1],
                vertex[2] + shift[2],
            ]
        });
        let tris_a = [[0_usize, 2, 1], [0, 1, 3], [0, 3, 2], [1, 2, 3]];
        let tris_b = tris_a.map(|triangle| triangle.map(|index| index + 4));
        let mut text = String::from("solid seam\n");
        for triangle in tris_a.iter().chain(tris_b.iter()) {
            text.push_str("  facet normal 0 0 0\n    outer loop\n");
            let source = if triangle[0] < 4 { &a } else { &b };
            for vertex in triangle {
                let coordinate = source[*vertex % 4];
                text.push_str(&format!(
                    "      vertex {} {} {}\n",
                    coordinate[0], coordinate[1], coordinate[2]
                ));
            }
            text.push_str("    endloop\n  endfacet\n");
        }
        text.push_str("endsolid seam\n");
        fs::write(path, text).expect("write authored seam stl");
    }

    /// Minimal 3MF core package carrying one closed tetrahedron with authored
    /// vertex coordinates and authored triangle indices.
    fn write_tetra_3mf(path: &Path) {
        use std::io::Write as _;
        let vertices = [
            [0.0_f64, 0.0, 0.0],
            [2.0, 0.0, 0.0],
            [1.0, 2.0, 0.0],
            [1.0, 1.0, 2.0],
        ];
        let triangles = [[0_u32, 2, 1], [0, 1, 3], [0, 3, 2], [1, 2, 3]];
        let file = fs::File::create(path).expect("create 3mf");
        let mut zip = zip::ZipWriter::new(file);
        let options =
            zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Stored);
        zip.start_file("3D/3dmodel.model", options)
            .expect("3mf model part");
        let mut xml = String::from(
            r#"<?xml version="1.0" encoding="UTF-8"?><model unit="millimeter" xmlns="http://schemas.microsoft.com/3dmanufacturing/core/2015/02"><resources><object id="1" type="model"><mesh><vertices>"#,
        );
        for vertex in &vertices {
            xml.push_str(&format!(
                r#"<vertex x="{}" y="{}" z="{}"/>"#,
                vertex[0], vertex[1], vertex[2]
            ));
        }
        xml.push_str("</vertices><triangles>");
        for triangle in &triangles {
            xml.push_str(&format!(
                r#"<triangle v1="{}" v2="{}" v3="{}"/>"#,
                triangle[0], triangle[1], triangle[2]
            ));
        }
        xml.push_str("</triangles></mesh></object></resources><build></build></model>");
        zip.write_all(xml.as_bytes()).expect("3mf model body");
        zip.finish().expect("finish 3mf");
    }

    fn admitted_indexed_vertex_count(plan: &RunnerPlan) -> usize {
        let import = &plan.parts[0].commands[0];
        assert_eq!(import.op, "import-indexed-mesh", "import must be admitted");
        import.args[0]
            .value
            .as_array()
            .expect("indexed vertex list")
            .len()
    }

    fn admitted_indexed_digest(plan: &RunnerPlan) -> String {
        let import = &plan.parts[0].commands[0];
        assert_eq!(import.op, "import-indexed-mesh", "import must be admitted");
        import.args[2]
            .value
            .as_str()
            .expect("digest text")
            .to_string()
    }

    #[test]
    fn runner_plan_inlines_authored_stl_coordinates_for_root_boolean_without_weld() {
        // A standalone imported STL must reach the runner with authored
        // coordinates preserved. Two tetrahedra shifted by 1 nm decode to 8
        // authored vertices; the evaluated-CAD weld would collapse them to 4,
        // so only the authored decoder keeps the seam open at the runner.
        let root = temp_root("indexed-mesh-authored-stl");
        fs::create_dir_all(&root).expect("fixture dir");
        let stl_path = root.join("seam.stl");
        write_two_tetra_authored_seam_ascii_stl(&stl_path);

        let expected = crate::ecky_ir::mesh_asset::IndexedMeshAsset::from_stl(
            crate::ecky_ir::mesh_asset::MeshAssetSource::Imported,
            &stl_path,
        )
        .expect("authored decode");
        assert_eq!(expected.vertices().len(), 8);

        let plan = runner_plan(&indexed_mesh_boolean_plan(&stl_path, false))
            .expect("runner plan")
            .expect("supported plan");
        assert_eq!(admitted_indexed_vertex_count(&plan), 8);
        assert_eq!(admitted_indexed_digest(&plan), expected.content_digest());

        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn runner_plan_reads_authored_stl_sidecar_consistently_with_decode() {
        // Sidecar consumption and authored decode must agree. A sidecar written
        // from the authored decoder must inline the same digest as a fresh
        // authored decode: no welded-vs-authored split between sidecar and
        // runner.
        let root = temp_root("indexed-mesh-authored-stl-sidecar");
        fs::create_dir_all(&root).expect("fixture dir");
        let stl_path = root.join("seam.stl");
        write_two_tetra_authored_seam_ascii_stl(&stl_path);

        let authored = crate::ecky_ir::mesh_asset::IndexedMeshAsset::from_stl(
            crate::ecky_ir::mesh_asset::MeshAssetSource::Imported,
            &stl_path,
        )
        .expect("authored decode");
        authored
            .write_cache(&stl_path.with_extension("indexed-mesh.json"))
            .expect("write sidecar");

        let plan = runner_plan(&indexed_mesh_boolean_plan(&stl_path, false))
            .expect("runner plan")
            .expect("supported plan");
        assert_eq!(admitted_indexed_vertex_count(&plan), 8);
        assert_eq!(admitted_indexed_digest(&plan), authored.content_digest());

        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn runner_plan_inlines_authored_3mf_import_for_root_boolean() {
        // A standalone imported 3MF reaches the runner with authored indexing.
        // The legacy STL-only fallback cannot parse a 3MF package, so this must
        // route through the authored 3MF decoder rather than erroring or
        // welding.
        let root = temp_root("indexed-mesh-authored-3mf");
        fs::create_dir_all(&root).expect("fixture dir");
        let path = root.join("island.3mf");
        write_tetra_3mf(&path);

        let expected = crate::ecky_ir::mesh_asset::IndexedMeshAsset::from_3mf(
            crate::ecky_ir::mesh_asset::MeshAssetSource::Imported,
            &path,
        )
        .expect("authored 3mf decode");

        let plan = runner_plan(&indexed_mesh_boolean_plan(&path, false))
            .expect("runner plan")
            .expect("supported plan");
        assert_eq!(admitted_indexed_vertex_count(&plan), 4);
        assert_eq!(admitted_indexed_digest(&plan), expected.content_digest());

        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn indexed_mesh_import_rejects_authored_3mf_used_as_first_boolean_tool() {
        let root = temp_root("indexed-mesh-authored-3mf-tool");
        fs::create_dir_all(&root).expect("fixture dir");
        let path = root.join("tool.3mf");
        write_tetra_3mf(&path);
        let mut source = indexed_mesh_boolean_plan(&path, false);
        let difference = source.parts[0]
            .commands
            .iter_mut()
            .find(|command| command.op == OcctOp::Difference)
            .expect("difference");
        difference.args.swap(0, 1);

        let admitted = indexed_mesh_imports_for_root_boolean(&source.parts[0])
            .expect("indexed mesh admission");
        assert!(
            admitted.is_empty(),
            "an imported 3MF used as Boolean tool must not enter indexed-mesh path"
        );

        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn indexed_mesh_import_rejects_malformed_cyclic_binary_chain() {
        let root = temp_root("indexed-mesh-cyclic-binary-chain");
        fs::create_dir_all(&root).expect("fixture dir");
        let stl_path = root.join("island.stl");
        write_indexed_cube_sidecar(&stl_path);
        let mut source = indexed_mesh_boolean_plan(&stl_path, false);
        source.parts[0].commands.push(OcctCommand {
            output: OcctSlot(2),
            op: OcctOp::Difference,
            args: vec![OcctArg::Ref(OcctSlot(4)), OcctArg::Ref(OcctSlot(3))],
            keywords: Vec::new(),
        });
        source.parts[0].root = OcctSlot(9);

        let plan = runner_plan(&source).expect("cyclic plan must reject without hanging");
        assert_eq!(
            plan.expect("supported plan").parts[0].commands[0].op,
            "import-stl",
            "cyclic Boolean chain must not be admitted as indexed mesh"
        );

        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn indexed_mesh_import_rejects_binary_chain_root_with_consumer() {
        let root = temp_root("indexed-mesh-root-consumer");
        fs::create_dir_all(&root).expect("fixture dir");
        let stl_path = root.join("island.stl");
        write_indexed_cube_sidecar(&stl_path);
        let mut source = indexed_mesh_boolean_plan(&stl_path, false);
        source.parts[0].commands.push(OcctCommand {
            output: OcctSlot(5),
            op: OcctOp::Difference,
            args: vec![OcctArg::Ref(OcctSlot(4)), OcctArg::Ref(OcctSlot(3))],
            keywords: Vec::new(),
        });

        let plan = runner_plan(&source)
            .expect("runner plan")
            .expect("supported plan");
        assert_eq!(
            plan.parts[0].commands[0].op, "import-stl",
            "a claimed root with a Boolean consumer must not admit indexed mesh"
        );

        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn runner_plan_admits_authored_3mf_through_binary_difference_chain() {
        // Binary Difference lowering gives the first cut a fresh intermediate
        // output. Admission must follow that base-side Boolean chain to the
        // root so a 3MF never falls back to the STL-only import path.
        let root = temp_root("indexed-mesh-authored-3mf-binary-chain");
        fs::create_dir_all(&root).expect("fixture dir");
        let path = root.join("island.3mf");
        write_tetra_3mf(&path);

        let mut source = indexed_mesh_boolean_plan(&path, false);
        source.parts[0].commands.push(OcctCommand {
            output: OcctSlot(5),
            op: OcctOp::Cylinder,
            args: vec![OcctArg::Number(0.25), OcctArg::Number(4.0)],
            keywords: Vec::new(),
        });
        source.parts[0].commands.push(OcctCommand {
            output: OcctSlot(6),
            op: OcctOp::Difference,
            args: vec![OcctArg::Ref(OcctSlot(4)), OcctArg::Ref(OcctSlot(5))],
            keywords: Vec::new(),
        });
        source.parts[0].root = OcctSlot(6);

        let plan = runner_plan(&source)
            .expect("runner plan")
            .expect("supported plan");
        assert_eq!(admitted_indexed_vertex_count(&plan), 4);

        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn runner_plan_admits_indexed_import_through_transform_chain() {
        let root = temp_root("indexed-mesh-transform-chain");
        fs::create_dir_all(&root).expect("fixture dir");
        let stl_path = root.join("island.stl");
        write_closed_tetra_stl(&stl_path);
        let mut source = indexed_mesh_boolean_plan(&stl_path, false);
        let commands = &mut source.parts[0].commands;
        commands.insert(
            2,
            OcctCommand {
                output: OcctSlot(5),
                op: OcctOp::Rotate,
                args: vec![
                    OcctArg::Number(180.0),
                    OcctArg::Number(0.0),
                    OcctArg::Number(0.0),
                    OcctArg::Ref(OcctSlot(2)),
                ],
                keywords: Vec::new(),
            },
        );
        commands.insert(
            3,
            OcctCommand {
                output: OcctSlot(6),
                op: OcctOp::Scale,
                args: vec![
                    OcctArg::Number(14.0),
                    OcctArg::Number(14.0),
                    OcctArg::Number(14.0),
                    OcctArg::Ref(OcctSlot(5)),
                ],
                keywords: Vec::new(),
            },
        );
        commands.insert(
            4,
            OcctCommand {
                output: OcctSlot(7),
                op: OcctOp::Translate,
                args: vec![
                    OcctArg::Number(45.0),
                    OcctArg::Number(0.0),
                    OcctArg::Number(7.0),
                    OcctArg::Ref(OcctSlot(6)),
                ],
                keywords: Vec::new(),
            },
        );
        commands
            .iter_mut()
            .find(|command| command.op == OcctOp::Difference)
            .expect("difference")
            .args[0] = OcctArg::Ref(OcctSlot(7));

        let plan = runner_plan(&source)
            .expect("runner plan")
            .expect("supported plan");
        assert_eq!(plan.parts[0].commands[0].op, "import-indexed-mesh");

        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn native_runner_exports_multipart_manifold_roots() {
        let fixture = temp_root("multipart-manifold-fixture");
        fs::create_dir_all(&fixture).expect("fixture dir");
        let stl_path = fixture.join("island.stl");
        write_closed_tetra_stl(&stl_path);
        let mut source = indexed_mesh_boolean_plan(&stl_path, false);
        let mut second = source.parts[0].clone();
        second.key = "second".to_string();
        second.label = "Second".to_string();
        source.parts.push(second);

        let Some((root, _topology)) =
            run_real_runner_plan_json("multipart-manifold-export", &source)
        else {
            fs::remove_dir_all(fixture).expect("cleanup fixture");
            return;
        };
        let bundle = root.join("bundle");
        assert!(bundle.join("preview.stl").is_file());
        assert!(bundle.join("parts/body.stl").is_file());
        assert!(bundle.join("parts/second.stl").is_file());
        assert!(
            !bundle.join("model.step").exists(),
            "mesh-native multipart export must not fabricate STEP"
        );

        fs::remove_dir_all(root).expect("cleanup runner");
        fs::remove_dir_all(fixture).expect("cleanup fixture");
    }

    #[test]
    fn native_runner_exports_mixed_analytic_and_mesh_parts_with_tessellated_step_member() {
        let fixture = temp_root("mixed-representation-fixture");
        fs::create_dir_all(&fixture).expect("fixture dir");
        let stl_path = fixture.join("island.stl");
        write_closed_tetra_stl(&stl_path);
        let mut source = indexed_mesh_boolean_plan(&stl_path, false);
        source.parts[0].key = "mesh-lid".to_string();
        source.parts[0].label = "Mesh Lid".to_string();
        source.parts.push(OcctPartPlan {
            key: "analytic-body".to_string(),
            label: "Analytic Body".to_string(),
            root: OcctSlot(10),
            commands: vec![OcctCommand {
                output: OcctSlot(10),
                op: OcctOp::Box,
                args: vec![
                    OcctArg::Number(2.0),
                    OcctArg::Number(2.0),
                    OcctArg::Number(2.0),
                ],
                keywords: Vec::new(),
            }],
        });

        let Some((root, topology)) =
            run_real_runner_plan_json("mixed-representation-export", &source)
        else {
            fs::remove_dir_all(fixture).expect("cleanup fixture");
            return;
        };
        let bundle = root.join("bundle");
        assert!(bundle.join("model.step").is_file());
        assert!(bundle.join("preview.stl").is_file());
        assert!(bundle.join("parts/mesh-lid.stl").is_file());
        assert!(bundle.join("parts/analytic-body.stl").is_file());
        assert_eq!(topology["parts"].as_array().expect("parts").len(), 2);
        assert_eq!(topology["parts"][0]["representation"], "meshDomain");
        assert_eq!(topology["parts"][1]["representation"], "analyticBrep");
        let report = read_runner_stage_report(&bundle).expect("stage report");
        assert_eq!(report.tessellated_step_part_count, Some(1));

        fs::remove_dir_all(root).expect("cleanup runner");
        fs::remove_dir_all(fixture).expect("cleanup fixture");
    }

    #[test]
    fn runner_plan_keeps_occt_import_when_boolean_has_post_brep_consumer() {
        let root = temp_root("indexed-mesh-post-brep");
        fs::create_dir_all(&root).expect("fixture dir");
        let stl_path = root.join("island.stl");
        write_indexed_cube_sidecar(&stl_path);

        let plan = runner_plan(&indexed_mesh_boolean_plan(&stl_path, true))
            .expect("runner plan")
            .expect("supported plan");
        assert_eq!(plan.parts[0].commands[0].op, "import-stl");

        fs::remove_dir_all(root).expect("cleanup");
    }

    fn expanded_transform_plan() -> OcctPlan {
        sample_plan_for_commands(
            OcctSlot(4),
            vec![
                OcctCommand {
                    output: OcctSlot(1),
                    op: OcctOp::Box,
                    args: vec![
                        OcctArg::Number(4.0),
                        OcctArg::Number(5.0),
                        OcctArg::Number(6.0),
                    ],
                    keywords: Vec::new(),
                },
                OcctCommand {
                    output: OcctSlot(2),
                    op: OcctOp::Rotate,
                    args: vec![
                        OcctArg::Number(10.0),
                        OcctArg::Number(20.0),
                        OcctArg::Number(30.0),
                        OcctArg::Ref(OcctSlot(1)),
                    ],
                    keywords: Vec::new(),
                },
                OcctCommand {
                    output: OcctSlot(3),
                    op: OcctOp::Scale,
                    args: vec![
                        OcctArg::Number(1.1),
                        OcctArg::Number(1.2),
                        OcctArg::Number(1.0),
                        OcctArg::Ref(OcctSlot(2)),
                    ],
                    keywords: Vec::new(),
                },
                OcctCommand {
                    output: OcctSlot(4),
                    op: OcctOp::Mirror,
                    args: vec![
                        OcctArg::Text("x".to_string()),
                        OcctArg::Number(0.0),
                        OcctArg::Ref(OcctSlot(3)),
                    ],
                    keywords: Vec::new(),
                },
            ],
        )
    }

    fn expanded_array_plan() -> OcctPlan {
        sample_plan_for_commands(
            OcctSlot(6),
            vec![
                OcctCommand {
                    output: OcctSlot(1),
                    op: OcctOp::Box,
                    args: vec![
                        OcctArg::Number(1.0),
                        OcctArg::Number(1.0),
                        OcctArg::Number(1.0),
                    ],
                    keywords: Vec::new(),
                },
                OcctCommand {
                    output: OcctSlot(2),
                    op: OcctOp::LinearArray,
                    args: vec![
                        OcctArg::Number(3.0),
                        OcctArg::Number(2.0),
                        OcctArg::Number(0.0),
                        OcctArg::Number(0.0),
                        OcctArg::Ref(OcctSlot(1)),
                    ],
                    keywords: Vec::new(),
                },
                OcctCommand {
                    output: OcctSlot(3),
                    op: OcctOp::GridArray,
                    args: vec![
                        OcctArg::Number(2.0),
                        OcctArg::Number(2.0),
                        OcctArg::Number(3.0),
                        OcctArg::Number(3.0),
                        OcctArg::Ref(OcctSlot(1)),
                    ],
                    keywords: Vec::new(),
                },
                OcctCommand {
                    output: OcctSlot(4),
                    op: OcctOp::RadialArray,
                    args: vec![
                        OcctArg::Number(3.0),
                        OcctArg::Number(45.0),
                        OcctArg::Number(6.0),
                        OcctArg::Ref(OcctSlot(1)),
                    ],
                    keywords: Vec::new(),
                },
                OcctCommand {
                    output: OcctSlot(5),
                    op: OcctOp::ArcArray,
                    args: vec![
                        OcctArg::Number(3.0),
                        OcctArg::Number(8.0),
                        OcctArg::Number(0.0),
                        OcctArg::Number(90.0),
                        OcctArg::Ref(OcctSlot(1)),
                    ],
                    keywords: Vec::new(),
                },
                OcctCommand {
                    output: OcctSlot(6),
                    op: OcctOp::Compound,
                    args: vec![
                        OcctArg::Ref(OcctSlot(2)),
                        OcctArg::Ref(OcctSlot(3)),
                        OcctArg::Ref(OcctSlot(4)),
                        OcctArg::Ref(OcctSlot(5)),
                    ],
                    keywords: Vec::new(),
                },
            ],
        )
    }

    fn expanded_profile_surface_plan() -> OcctPlan {
        sample_plan_for_commands(
            OcctSlot(13),
            vec![
                OcctCommand {
                    output: OcctSlot(1),
                    op: OcctOp::RoundedRectangle,
                    args: vec![
                        OcctArg::Number(5.0),
                        OcctArg::Number(4.0),
                        OcctArg::Number(0.5),
                    ],
                    keywords: Vec::new(),
                },
                OcctCommand {
                    output: OcctSlot(2),
                    op: OcctOp::Extrude,
                    args: vec![OcctArg::Ref(OcctSlot(1)), OcctArg::Number(2.0)],
                    keywords: Vec::new(),
                },
                OcctCommand {
                    output: OcctSlot(3),
                    op: OcctOp::RoundedPolygon,
                    args: vec![
                        OcctArg::List(vec![
                            OcctArg::Point2([-2.0, -1.0]),
                            OcctArg::Point2([2.0, -1.0]),
                            OcctArg::Point2([2.0, 1.0]),
                            OcctArg::Point2([-2.0, 1.0]),
                        ]),
                        OcctArg::Number(0.2),
                    ],
                    keywords: Vec::new(),
                },
                OcctCommand {
                    output: OcctSlot(4),
                    op: OcctOp::Taper,
                    args: vec![
                        OcctArg::Number(3.0),
                        OcctArg::Number(0.7),
                        OcctArg::Ref(OcctSlot(3)),
                    ],
                    keywords: Vec::new(),
                },
                OcctCommand {
                    output: OcctSlot(5),
                    op: OcctOp::Circle,
                    args: vec![OcctArg::Number(0.5)],
                    keywords: Vec::new(),
                },
                OcctCommand {
                    output: OcctSlot(6),
                    op: OcctOp::Rectangle,
                    args: vec![OcctArg::Number(1.2), OcctArg::Number(1.2)],
                    keywords: Vec::new(),
                },
                OcctCommand {
                    output: OcctSlot(7),
                    op: OcctOp::Loft,
                    args: vec![
                        OcctArg::Number(2.0),
                        OcctArg::Ref(OcctSlot(5)),
                        OcctArg::Ref(OcctSlot(6)),
                    ],
                    keywords: Vec::new(),
                },
                OcctCommand {
                    output: OcctSlot(8),
                    op: OcctOp::BezierPath,
                    args: vec![
                        OcctArg::Point3([0.0, 0.0, 0.0]),
                        OcctArg::Point3([1.0, 0.0, 1.0]),
                        OcctArg::Point3([2.0, 0.0, 1.0]),
                        OcctArg::Point3([3.0, 0.0, 0.0]),
                    ],
                    keywords: Vec::new(),
                },
                OcctCommand {
                    output: OcctSlot(9),
                    op: OcctOp::Circle,
                    args: vec![OcctArg::Number(0.2)],
                    keywords: Vec::new(),
                },
                OcctCommand {
                    output: OcctSlot(10),
                    op: OcctOp::Sweep,
                    args: vec![OcctArg::Ref(OcctSlot(9)), OcctArg::Ref(OcctSlot(8))],
                    keywords: Vec::new(),
                },
                OcctCommand {
                    output: OcctSlot(11),
                    op: OcctOp::Bspline,
                    args: vec![OcctArg::List(vec![
                        OcctArg::Point2([-1.0, -0.5]),
                        OcctArg::Point2([0.0, -1.0]),
                        OcctArg::Point2([1.0, -0.5]),
                        OcctArg::Point2([1.0, 0.5]),
                        OcctArg::Point2([0.0, 1.0]),
                        OcctArg::Point2([-1.0, 0.5]),
                    ])],
                    keywords: Vec::new(),
                },
                OcctCommand {
                    output: OcctSlot(12),
                    op: OcctOp::Extrude,
                    args: vec![OcctArg::Ref(OcctSlot(11)), OcctArg::Number(1.0)],
                    keywords: Vec::new(),
                },
                OcctCommand {
                    output: OcctSlot(13),
                    op: OcctOp::Compound,
                    args: vec![
                        OcctArg::Ref(OcctSlot(2)),
                        OcctArg::Ref(OcctSlot(4)),
                        OcctArg::Ref(OcctSlot(7)),
                        OcctArg::Ref(OcctSlot(10)),
                        OcctArg::Ref(OcctSlot(12)),
                    ],
                    keywords: Vec::new(),
                },
            ],
        )
    }

    fn expanded_revolve_plan() -> OcctPlan {
        sample_plan_for_commands(
            OcctSlot(3),
            vec![
                OcctCommand {
                    output: OcctSlot(1),
                    op: OcctOp::Rectangle,
                    args: vec![OcctArg::Number(0.6), OcctArg::Number(1.0)],
                    keywords: Vec::new(),
                },
                OcctCommand {
                    output: OcctSlot(2),
                    op: OcctOp::Translate,
                    args: vec![
                        OcctArg::Number(2.0),
                        OcctArg::Number(0.0),
                        OcctArg::Number(0.0),
                        OcctArg::Ref(OcctSlot(1)),
                    ],
                    keywords: Vec::new(),
                },
                OcctCommand {
                    output: OcctSlot(3),
                    op: OcctOp::Revolve,
                    args: vec![OcctArg::Ref(OcctSlot(2)), OcctArg::Number(120.0)],
                    keywords: Vec::new(),
                },
            ],
        )
    }

    fn expanded_profile_offset_twist_plan() -> OcctPlan {
        sample_plan_for_commands(
            OcctSlot(8),
            vec![
                OcctCommand {
                    output: OcctSlot(1),
                    op: OcctOp::Circle,
                    args: vec![OcctArg::Number(1.0)],
                    keywords: Vec::new(),
                },
                OcctCommand {
                    output: OcctSlot(2),
                    op: OcctOp::Profile,
                    args: vec![OcctArg::Ref(OcctSlot(1))],
                    keywords: Vec::new(),
                },
                OcctCommand {
                    output: OcctSlot(3),
                    op: OcctOp::Extrude,
                    args: vec![OcctArg::Ref(OcctSlot(2)), OcctArg::Number(1.0)],
                    keywords: Vec::new(),
                },
                OcctCommand {
                    output: OcctSlot(4),
                    op: OcctOp::Rectangle,
                    args: vec![OcctArg::Number(1.0), OcctArg::Number(1.0)],
                    keywords: Vec::new(),
                },
                OcctCommand {
                    output: OcctSlot(5),
                    op: OcctOp::Offset,
                    args: vec![OcctArg::Number(0.25), OcctArg::Ref(OcctSlot(4))],
                    keywords: Vec::new(),
                },
                OcctCommand {
                    output: OcctSlot(6),
                    op: OcctOp::Extrude,
                    args: vec![OcctArg::Ref(OcctSlot(5)), OcctArg::Number(1.0)],
                    keywords: Vec::new(),
                },
                OcctCommand {
                    output: OcctSlot(7),
                    op: OcctOp::Twist,
                    args: vec![
                        OcctArg::Number(2.0),
                        OcctArg::Number(120.0),
                        OcctArg::Ref(OcctSlot(4)),
                    ],
                    keywords: Vec::new(),
                },
                OcctCommand {
                    output: OcctSlot(8),
                    op: OcctOp::Compound,
                    args: vec![
                        OcctArg::Ref(OcctSlot(3)),
                        OcctArg::Ref(OcctSlot(6)),
                        OcctArg::Ref(OcctSlot(7)),
                    ],
                    keywords: Vec::new(),
                },
            ],
        )
    }

    fn sample_plan_for_command(command: OcctCommand) -> OcctPlan {
        sample_plan_for_commands(command.output, vec![command])
    }

    fn sample_plan_for_commands(root: OcctSlot, commands: Vec<OcctCommand>) -> OcctPlan {
        OcctPlan {
            parameters: Vec::new(),
            parts: vec![OcctPartPlan {
                key: "body".to_string(),
                label: "Body".to_string(),
                root,
                commands,
            }],
        }
    }

    fn compiled_plan(source: &str) -> OcctPlan {
        let program = crate::ecky_scheme::compile_to_core_program(source).expect("compile fixture");
        crate::ecky_cad_host::direct_occt::plan_core_program(&program).expect("plan fixture")
    }

    fn combine_fixture_plans(label: &str, plans: Vec<OcctPlan>) -> OcctPlan {
        let mut parts = Vec::new();
        for (plan_index, plan) in plans.into_iter().enumerate() {
            for (part_index, mut part) in plan.parts.into_iter().enumerate() {
                part.key = format!("{label}-{plan_index}-{part_index}");
                part.label = part.key.clone();
                parts.push(part);
            }
        }
        OcctPlan {
            parameters: Vec::new(),
            parts,
        }
    }

    fn primitive_profile_parity_plan() -> OcctPlan {
        compiled_plan(
            r#"
            (model
              (part torus-body (torus 6 1.5))
              (part wedge-body (wedge 12 8 10 2 1 10 8))
              (part ellipse-body (extrude (ellipse 4 2) 3))
              (part slot-body (extrude (slot-overall 12 4) 3))
              (part slot-arc-body (extrude (slot-arc 8 0 100 3) 3))
              (part polygon-face-body
                (extrude (make-face (polygon ((0 0) (7 0) (6 5) (1 6)))) 3))
              (part basic-primitives
                (compound
                  (box 3 4 5)
                  (translate 8 0 0 (sphere 2.5))
                  (translate 16 0 0 (cylinder 2 5))
                  (translate 24 0 0 (cone 2.5 1 5))))
              (part profile-primitives
                (compound
                  (extrude (circle 2) 2)
                  (translate 8 0 0 (extrude (rectangle 5 3) 2))
                  (translate 16 0 0 (extrude (rounded-rect 5 3 0.5) 2))
                  (translate 24 0 0
                    (extrude
                      (rounded-polygon ((-2 -1) (2 -1) (2 1) (-2 1)) 0.25)
                      2))))
            )
            "#,
        )
    }

    fn frame_path_parity_plan() -> OcctPlan {
        compiled_plan(
            r#"
            (model
              (part plane-location-body
                (build
                  (shape base (plane :origin (0 0 4) :normal (0 0 1)))
                  (shape loc (location base))
                  (shape peg (box 2 4 6))
                  (shape placed (place loc peg))
                  (result (clip-box placed :x (0 10) :y (-5 5) :z (0 12)))))
              (part path-frame-body
                (build
                  (shape rail (path ((0 0 0) (6 0 8) (0 0 18))))
                  (shape peg (cylinder 2 6))
                  (shape end-frame (path-frame rail :at end :up (0 1 0)))
                  (result (place end-frame peg))))
            )
            "#,
        )
    }

    fn boolean_hull_parity_plan() -> OcctPlan {
        compiled_plan(
            r#"
            (model
              (part boolean-hull-body
                (build
                  (shape base (box 20 14 10))
                  (shape lobe (translate 7 0 0 (sphere 7)))
                  (shape fused (union base lobe))
                  (shape bore (cylinder 3 16))
                  (shape cut-body (difference fused bore))
                  (shape overlap (intersection base lobe))
                  (shape blended
                    (hull (sphere 3) (translate 10 0 0 (sphere 3))))
                  (result
                    (compound
                      cut-body
                      (translate 35 0 0 overlap)
                      (translate 55 0 0 blended)))))
            )
            "#,
        )
    }

    fn helical_path_parity_plan() -> OcctPlan {
        compiled_plan(
            r#"
            (model
              (part body
                (helical-ridge
                  :radius 8
                  :pitch 4
                  :height 10
                  :base-width 1.5
                  :crest-width 0.8
                  :depth 1.0)))
            "#,
        )
    }

    fn import_stl_parity_plan() -> OcctPlan {
        let fixture =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/vertex-genie-ecky.stl");
        compiled_plan(&format!(
            "(model (part imported (import-stl {:?})))",
            fixture.to_string_lossy()
        ))
    }

    fn solidify_import_stl_parity_plan() -> OcctPlan {
        let fixture =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/vertex-genie-ecky.stl");
        compiled_plan(&format!(
            "(model (part imported (solidify (import-stl {:?}))))",
            fixture.to_string_lossy()
        ))
    }

    fn solidify_boolean_import_stl_parity_plan() -> OcctPlan {
        let fixture =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/vertex-genie-ecky.stl");
        compiled_plan(&format!(
            "(model (part imported (difference (solidify (import-stl {:?})) (cylinder 0.5 4))))",
            fixture.to_string_lossy()
        ))
    }

    fn keyword_free_plane_plan() -> OcctPlan {
        sample_plan_for_command(OcctCommand {
            output: OcctSlot(1),
            op: OcctOp::Plane,
            args: Vec::new(),
            keywords: Vec::new(),
        })
    }

    fn keyworded_plane_plan() -> OcctPlan {
        sample_plan_for_command(OcctCommand {
            output: OcctSlot(1),
            op: OcctOp::Plane,
            args: Vec::new(),
            keywords: vec![
                OcctKeyword {
                    name: "origin".to_string(),
                    value: OcctKeywordValue::Arg(OcctArg::Point3([0.0, 0.0, 0.0])),
                },
                OcctKeyword {
                    name: "normal".to_string(),
                    value: OcctKeywordValue::Arg(OcctArg::Point3([0.0, 0.0, 1.0])),
                },
            ],
        })
    }

    fn supported_box_with_keyword_plan() -> OcctPlan {
        sample_plan_for_command(OcctCommand {
            output: OcctSlot(1),
            op: OcctOp::Box,
            args: vec![
                OcctArg::Number(12.0),
                OcctArg::Number(8.0),
                OcctArg::Number(4.0),
            ],
            keywords: vec![OcctKeyword {
                name: "align".to_string(),
                value: OcctKeywordValue::Arg(OcctArg::List(vec![
                    OcctArg::Symbol("min".to_string()),
                    OcctArg::Symbol("center".to_string()),
                    OcctArg::Symbol("max".to_string()),
                ])),
            }],
        })
    }

    fn supported_round_primitives_with_align_plan() -> OcctPlan {
        let align = || {
            vec![OcctKeyword {
                name: "align".to_string(),
                value: OcctKeywordValue::Arg(OcctArg::List(vec![
                    OcctArg::Symbol("min".to_string()),
                    OcctArg::Symbol("center".to_string()),
                    OcctArg::Symbol("max".to_string()),
                ])),
            }]
        };
        sample_plan_for_commands(
            OcctSlot(3),
            vec![
                OcctCommand {
                    output: OcctSlot(1),
                    op: OcctOp::Sphere,
                    args: vec![OcctArg::Number(4.0)],
                    keywords: align(),
                },
                OcctCommand {
                    output: OcctSlot(2),
                    op: OcctOp::Cylinder,
                    args: vec![OcctArg::Number(2.0), OcctArg::Number(8.0)],
                    keywords: align(),
                },
                OcctCommand {
                    output: OcctSlot(3),
                    op: OcctOp::Cone,
                    args: vec![
                        OcctArg::Number(3.0),
                        OcctArg::Number(1.0),
                        OcctArg::Number(7.0),
                    ],
                    keywords: align(),
                },
            ],
        )
    }

    fn supported_round_primitives_with_align_and_extra_numeric_args_plan() -> OcctPlan {
        let align = || {
            vec![OcctKeyword {
                name: "align".to_string(),
                value: OcctKeywordValue::Arg(OcctArg::List(vec![
                    OcctArg::Symbol("min".to_string()),
                    OcctArg::Symbol("center".to_string()),
                    OcctArg::Symbol("max".to_string()),
                ])),
            }]
        };
        sample_plan_for_commands(
            OcctSlot(3),
            vec![
                OcctCommand {
                    output: OcctSlot(1),
                    op: OcctOp::Sphere,
                    args: vec![OcctArg::Number(4.0), OcctArg::Number(48.0)],
                    keywords: align(),
                },
                OcctCommand {
                    output: OcctSlot(2),
                    op: OcctOp::Cylinder,
                    args: vec![
                        OcctArg::Number(2.0),
                        OcctArg::Number(8.0),
                        OcctArg::Number(64.0),
                    ],
                    keywords: align(),
                },
                OcctCommand {
                    output: OcctSlot(3),
                    op: OcctOp::Cone,
                    args: vec![
                        OcctArg::Number(3.0),
                        OcctArg::Number(1.0),
                        OcctArg::Number(7.0),
                        OcctArg::Number(48.0),
                    ],
                    keywords: align(),
                },
            ],
        )
    }

    fn keyword_profile_holes_plan() -> OcctPlan {
        sample_plan_for_commands(
            OcctSlot(4),
            vec![
                OcctCommand {
                    output: OcctSlot(1),
                    op: OcctOp::Circle,
                    args: vec![OcctArg::Number(10.0)],
                    keywords: Vec::new(),
                },
                OcctCommand {
                    output: OcctSlot(2),
                    op: OcctOp::Circle,
                    args: vec![OcctArg::Number(3.0)],
                    keywords: Vec::new(),
                },
                OcctCommand {
                    output: OcctSlot(3),
                    op: OcctOp::Profile,
                    args: Vec::new(),
                    keywords: vec![
                        OcctKeyword::arg("outer".to_string(), OcctArg::Ref(OcctSlot(1))),
                        OcctKeyword::arg(
                            "holes".to_string(),
                            OcctArg::List(vec![OcctArg::Ref(OcctSlot(2))]),
                        ),
                    ],
                },
                OcctCommand {
                    output: OcctSlot(4),
                    op: OcctOp::Extrude,
                    args: vec![OcctArg::Ref(OcctSlot(3)), OcctArg::Number(4.0)],
                    keywords: Vec::new(),
                },
            ],
        )
    }

    fn keyword_clip_box_plan() -> OcctPlan {
        sample_plan_for_commands(
            OcctSlot(2),
            vec![
                OcctCommand {
                    output: OcctSlot(1),
                    op: OcctOp::Box,
                    args: vec![
                        OcctArg::Number(20.0),
                        OcctArg::Number(20.0),
                        OcctArg::Number(20.0),
                    ],
                    keywords: Vec::new(),
                },
                OcctCommand {
                    output: OcctSlot(2),
                    op: OcctOp::ClipBox,
                    args: vec![OcctArg::Ref(OcctSlot(1))],
                    keywords: vec![
                        OcctKeyword::arg(
                            "x".to_string(),
                            OcctArg::List(vec![OcctArg::Number(0.0), OcctArg::Number(10.0)]),
                        ),
                        OcctKeyword::arg(
                            "y".to_string(),
                            OcctArg::List(vec![OcctArg::Number(-5.0), OcctArg::Number(5.0)]),
                        ),
                        OcctKeyword::arg(
                            "z".to_string(),
                            OcctArg::List(vec![OcctArg::Number(0.0), OcctArg::Number(12.0)]),
                        ),
                    ],
                },
            ],
        )
    }

    fn exact_fillet_plan() -> OcctPlan {
        sample_plan_for_commands(
            OcctSlot(2),
            vec![
                OcctCommand {
                    output: OcctSlot(1),
                    op: OcctOp::Box,
                    args: vec![
                        OcctArg::Number(20.0),
                        OcctArg::Number(20.0),
                        OcctArg::Number(10.0),
                    ],
                    keywords: Vec::new(),
                },
                OcctCommand {
                    output: OcctSlot(2),
                    op: OcctOp::Fillet,
                    args: vec![OcctArg::Number(1.5), OcctArg::Ref(OcctSlot(1))],
                    keywords: vec![OcctKeyword::selector(
                        "edges".to_string(),
                        OcctArg::Text("target-id:body:edge:-10--10-0_10--10-0".to_string()),
                        CoreSelectorPayload::EdgeTargetIds(vec![
                            "body:edge:-10--10-0_10--10-0".to_string()
                        ]),
                    )],
                },
            ],
        )
    }

    fn draft_plan() -> OcctPlan {
        sample_plan_for_commands(
            OcctSlot(2),
            vec![
                OcctCommand {
                    output: OcctSlot(1),
                    op: OcctOp::Box,
                    args: vec![
                        OcctArg::Number(20.0),
                        OcctArg::Number(20.0),
                        OcctArg::Number(20.0),
                    ],
                    keywords: Vec::new(),
                },
                OcctCommand {
                    output: OcctSlot(2),
                    op: OcctOp::Draft,
                    args: vec![OcctArg::Number(10.0), OcctArg::Ref(OcctSlot(1))],
                    keywords: Vec::new(),
                },
            ],
        )
    }

    fn draft_neutral_z_plan() -> OcctPlan {
        sample_plan_for_commands(
            OcctSlot(2),
            vec![
                OcctCommand {
                    output: OcctSlot(1),
                    op: OcctOp::Box,
                    args: vec![
                        OcctArg::Number(20.0),
                        OcctArg::Number(20.0),
                        OcctArg::Number(20.0),
                    ],
                    keywords: Vec::new(),
                },
                OcctCommand {
                    output: OcctSlot(2),
                    op: OcctOp::Draft,
                    args: vec![OcctArg::Number(10.0), OcctArg::Ref(OcctSlot(1))],
                    keywords: vec![OcctKeyword::arg(
                        "neutral-z".to_string(),
                        OcctArg::Number(5.0),
                    )],
                },
            ],
        )
    }

    fn exact_chamfer_plan() -> OcctPlan {
        sample_plan_for_commands(
            OcctSlot(2),
            vec![
                OcctCommand {
                    output: OcctSlot(1),
                    op: OcctOp::Box,
                    args: vec![
                        OcctArg::Number(20.0),
                        OcctArg::Number(20.0),
                        OcctArg::Number(10.0),
                    ],
                    keywords: Vec::new(),
                },
                OcctCommand {
                    output: OcctSlot(2),
                    op: OcctOp::Chamfer,
                    args: vec![OcctArg::Number(1.25), OcctArg::Ref(OcctSlot(1))],
                    keywords: vec![OcctKeyword::selector(
                        "edges".to_string(),
                        OcctArg::Text("target-id:body:edge:-10--10-0_10--10-0".to_string()),
                        CoreSelectorPayload::EdgeTargetIds(vec![
                            "body:edge:-10--10-0_10--10-0".to_string()
                        ]),
                    )],
                },
            ],
        )
    }

    fn clause_fillet_plan() -> OcctPlan {
        sample_plan_for_commands(
            OcctSlot(2),
            vec![
                OcctCommand {
                    output: OcctSlot(1),
                    op: OcctOp::Box,
                    args: vec![
                        OcctArg::Number(20.0),
                        OcctArg::Number(20.0),
                        OcctArg::Number(10.0),
                    ],
                    keywords: Vec::new(),
                },
                OcctCommand {
                    output: OcctSlot(2),
                    op: OcctOp::Fillet,
                    args: vec![OcctArg::Number(1.5), OcctArg::Ref(OcctSlot(1))],
                    keywords: vec![OcctKeyword::selector(
                        "edges".to_string(),
                        OcctArg::Text("left+vertical".to_string()),
                        CoreSelectorPayload::EdgeClauses(vec![
                            crate::ecky_core_ir::CoreEdgeSelectorClause::Boundary {
                                axis: crate::ecky_core_ir::CoreEdgeAxis::X,
                                bound: crate::ecky_core_ir::CoreEdgeBound::Min,
                            },
                            crate::ecky_core_ir::CoreEdgeSelectorClause::Axis(
                                crate::ecky_core_ir::CoreEdgeAxis::Z,
                            ),
                        ]),
                    )],
                },
            ],
        )
    }

    fn clause_chamfer_plan() -> OcctPlan {
        sample_plan_for_commands(
            OcctSlot(2),
            vec![
                OcctCommand {
                    output: OcctSlot(1),
                    op: OcctOp::Box,
                    args: vec![
                        OcctArg::Number(20.0),
                        OcctArg::Number(20.0),
                        OcctArg::Number(10.0),
                    ],
                    keywords: Vec::new(),
                },
                OcctCommand {
                    output: OcctSlot(2),
                    op: OcctOp::Chamfer,
                    args: vec![OcctArg::Number(1.25), OcctArg::Ref(OcctSlot(1))],
                    keywords: vec![OcctKeyword::selector(
                        "edges".to_string(),
                        OcctArg::Text("left+vertical".to_string()),
                        CoreSelectorPayload::EdgeClauses(vec![
                            crate::ecky_core_ir::CoreEdgeSelectorClause::Boundary {
                                axis: crate::ecky_core_ir::CoreEdgeAxis::X,
                                bound: crate::ecky_core_ir::CoreEdgeBound::Min,
                            },
                            crate::ecky_core_ir::CoreEdgeSelectorClause::Axis(
                                crate::ecky_core_ir::CoreEdgeAxis::Z,
                            ),
                        ]),
                    )],
                },
            ],
        )
    }

    fn edge_all_fillet_plan() -> OcctPlan {
        sample_plan_for_commands(
            OcctSlot(2),
            vec![
                OcctCommand {
                    output: OcctSlot(1),
                    op: OcctOp::Box,
                    args: vec![
                        OcctArg::Number(20.0),
                        OcctArg::Number(20.0),
                        OcctArg::Number(10.0),
                    ],
                    keywords: Vec::new(),
                },
                OcctCommand {
                    output: OcctSlot(2),
                    op: OcctOp::Fillet,
                    args: vec![OcctArg::Number(1.5), OcctArg::Ref(OcctSlot(1))],
                    keywords: vec![OcctKeyword::selector(
                        "edges".to_string(),
                        OcctArg::Text("all".to_string()),
                        CoreSelectorPayload::EdgeAll,
                    )],
                },
            ],
        )
    }

    fn edge_all_chamfer_plan() -> OcctPlan {
        sample_plan_for_commands(
            OcctSlot(2),
            vec![
                OcctCommand {
                    output: OcctSlot(1),
                    op: OcctOp::Box,
                    args: vec![
                        OcctArg::Number(20.0),
                        OcctArg::Number(20.0),
                        OcctArg::Number(10.0),
                    ],
                    keywords: Vec::new(),
                },
                OcctCommand {
                    output: OcctSlot(2),
                    op: OcctOp::Chamfer,
                    args: vec![OcctArg::Number(1.25), OcctArg::Ref(OcctSlot(1))],
                    keywords: vec![OcctKeyword::selector(
                        "edges".to_string(),
                        OcctArg::Text("all".to_string()),
                        CoreSelectorPayload::EdgeAll,
                    )],
                },
            ],
        )
    }

    fn exact_shell_plan() -> OcctPlan {
        sample_plan_for_commands(
            OcctSlot(2),
            vec![
                OcctCommand {
                    output: OcctSlot(1),
                    op: OcctOp::Box,
                    args: vec![
                        OcctArg::Number(20.0),
                        OcctArg::Number(20.0),
                        OcctArg::Number(10.0),
                    ],
                    keywords: Vec::new(),
                },
                OcctCommand {
                    output: OcctSlot(2),
                    op: OcctOp::Shell,
                    args: vec![OcctArg::Number(1.0), OcctArg::Ref(OcctSlot(1))],
                    keywords: vec![OcctKeyword::selector(
                        "faces".to_string(),
                        OcctArg::Text("target-id:body:face:0-0-10:400".to_string()),
                        CoreSelectorPayload::FaceTargetIds(
                            vec!["body:face:0-0-10:400".to_string()],
                        ),
                    )],
                },
            ],
        )
    }

    fn shell_clause_plan() -> OcctPlan {
        sample_plan_for_commands(
            OcctSlot(2),
            vec![
                OcctCommand {
                    output: OcctSlot(1),
                    op: OcctOp::Box,
                    args: vec![
                        OcctArg::Number(20.0),
                        OcctArg::Number(20.0),
                        OcctArg::Number(10.0),
                    ],
                    keywords: Vec::new(),
                },
                OcctCommand {
                    output: OcctSlot(2),
                    op: OcctOp::Shell,
                    args: vec![OcctArg::Number(1.0), OcctArg::Ref(OcctSlot(1))],
                    keywords: vec![OcctKeyword::selector(
                        "faces".to_string(),
                        OcctArg::Text("faces:[planar normal:z area:max]".to_string()),
                        CoreSelectorPayload::FaceClauses(vec![
                            CoreFaceSelectorClause::Planar,
                            CoreFaceSelectorClause::Normal(CoreEdgeAxis::Z),
                            CoreFaceSelectorClause::Area(CoreFaceAreaRank::Max),
                        ]),
                    )],
                },
            ],
        )
    }

    fn shell_plan() -> OcctPlan {
        sample_plan_for_commands(
            OcctSlot(2),
            vec![
                OcctCommand {
                    output: OcctSlot(1),
                    op: OcctOp::Box,
                    args: vec![
                        OcctArg::Number(20.0),
                        OcctArg::Number(20.0),
                        OcctArg::Number(10.0),
                    ],
                    keywords: Vec::new(),
                },
                OcctCommand {
                    output: OcctSlot(2),
                    op: OcctOp::Shell,
                    args: vec![OcctArg::Number(1.0), OcctArg::Ref(OcctSlot(1))],
                    keywords: Vec::new(),
                },
            ],
        )
    }

    fn runner_supported_ops() -> Vec<OcctOp> {
        vec![
            OcctOp::Box,
            OcctOp::Sphere,
            OcctOp::Cylinder,
            OcctOp::Cone,
            OcctOp::Torus,
            OcctOp::Wedge,
            OcctOp::Circle,
            OcctOp::Ellipse,
            OcctOp::Slot,
            OcctOp::SlotArc,
            OcctOp::Rectangle,
            OcctOp::RoundedRectangle,
            OcctOp::RoundedPolygon,
            OcctOp::Polygon,
            OcctOp::Profile,
            OcctOp::MakeFace,
            OcctOp::ImportStl,
            OcctOp::ImportStep,
            OcctOp::Solidify,
            OcctOp::Extrude,
            OcctOp::Revolve,
            OcctOp::Loft,
            OcctOp::Sweep,
            OcctOp::Twist,
            OcctOp::Taper,
            OcctOp::Draft,
            OcctOp::Offset,
            OcctOp::Path,
            OcctOp::HelixPath,
            OcctOp::BezierPath,
            OcctOp::Bspline,
            OcctOp::Plane,
            OcctOp::Location,
            OcctOp::PathFrame,
            OcctOp::Place,
            OcctOp::ClipBox,
            OcctOp::LinearArray,
            OcctOp::RadialArray,
            OcctOp::GridArray,
            OcctOp::ArcArray,
            OcctOp::Union,
            OcctOp::Difference,
            OcctOp::Intersection,
            OcctOp::Fillet,
            OcctOp::Chamfer,
            OcctOp::Shell,
            OcctOp::Translate,
            OcctOp::Rotate,
            OcctOp::Scale,
            OcctOp::Mirror,
            OcctOp::Compound,
            OcctOp::Hull,
        ]
    }

    fn runner_parity_fixture_plans() -> Vec<(&'static str, OcctPlan)> {
        vec![
            ("primitives-profiles", primitive_profile_parity_plan()),
            (
                "surfaces",
                combine_fixture_plans(
                    "surface",
                    vec![
                        expanded_profile_surface_plan(),
                        expanded_revolve_plan(),
                        expanded_profile_offset_twist_plan(),
                        keyword_profile_holes_plan(),
                    ],
                ),
            ),
            (
                "transforms-arrays",
                combine_fixture_plans(
                    "transform-array",
                    vec![expanded_transform_plan(), expanded_array_plan()],
                ),
            ),
            ("frames-paths", frame_path_parity_plan()),
            ("booleans-hull", boolean_hull_parity_plan()),
            ("selector-fillet-all", edge_all_fillet_plan()),
            ("selector-fillet-clause", clause_fillet_plan()),
            ("selector-fillet-exact", exact_fillet_plan()),
            ("selector-chamfer-all", edge_all_chamfer_plan()),
            ("selector-chamfer-clause", clause_chamfer_plan()),
            ("selector-chamfer-exact", exact_chamfer_plan()),
            ("selector-shell-default", shell_plan()),
            ("selector-shell-clause", shell_clause_plan()),
            ("selector-shell-exact", exact_shell_plan()),
            ("draft", draft_plan()),
            ("draft-neutral-z", draft_neutral_z_plan()),
            ("helix", helical_path_parity_plan()),
            ("import-stl", import_stl_parity_plan()),
            ("solidify-import-stl", solidify_import_stl_parity_plan()),
        ]
    }

    #[test]
    fn generated_source_runner_parity_matrix_covers_every_runner_supported_op() {
        let covered = runner_parity_fixture_plans()
            .into_iter()
            .flat_map(|(_, plan)| {
                plan.parts
                    .into_iter()
                    .flat_map(|part| part.commands.into_iter())
                    .map(|command| runner_op_token(command.op))
            })
            .collect::<std::collections::BTreeSet<_>>();
        // External imports need immutable on-disk payloads, so generated-source A/B
        // fixtures cannot own them. Real-runner STEP tests cover this op separately.
        let external_asset_ops = std::collections::BTreeSet::from(["import-step"]);
        let missing = runner_supported_ops()
            .into_iter()
            .map(runner_op_token)
            .filter(|op| !covered.contains(op) && !external_asset_ops.contains(op))
            .collect::<Vec<_>>();

        assert!(
            missing.is_empty(),
            "runner-supported ops missing generated-source A/B fixtures: {}",
            missing.join(", ")
        );
    }

    #[test]
    fn runner_support_gate_matches_proven_subset() {
        for op in runner_supported_ops() {
            assert_eq!(
                runner_op_supported(op),
                true,
                "runner support gate for {}",
                runner_op_token(op)
            );
        }
    }

    #[test]
    fn runner_supports_plan_rejects_keywords_even_on_supported_ops() {
        assert!(runner_supports_plan(&supported_sample_plan()));
        assert!(!runner_supports_plan(&sample_plan()));
    }

    #[test]
    fn runner_supports_plan_accepts_supported_keyword_profile_and_clip_box_forms() {
        assert!(runner_supports_plan(&supported_box_with_keyword_plan()));
        assert!(runner_supports_plan(
            &supported_round_primitives_with_align_plan()
        ));
        assert!(runner_supports_plan(
            &supported_round_primitives_with_align_and_extra_numeric_args_plan()
        ));
        assert!(runner_supports_plan(&keyworded_plane_plan()));
        assert!(runner_supports_plan(&keyword_profile_holes_plan()));
        assert!(runner_supports_plan(&keyword_clip_box_plan()));
    }

    #[test]
    fn runner_supports_plan_accepts_exact_selector_forms() {
        assert!(runner_supports_plan(&shell_plan()));
        assert!(runner_supports_plan(&edge_all_fillet_plan()));
        assert!(runner_supports_plan(&edge_all_chamfer_plan()));
        assert!(runner_supports_plan(&exact_fillet_plan()));
        assert!(runner_supports_plan(&exact_chamfer_plan()));
        assert!(runner_supports_plan(&clause_fillet_plan()));
        assert!(runner_supports_plan(&clause_chamfer_plan()));
        assert!(runner_supports_plan(&exact_shell_plan()));
        assert!(runner_supports_plan(&shell_clause_plan()));
        assert!(runner_supports_plan(&draft_plan()));
        assert!(runner_supports_plan(&draft_neutral_z_plan()));
    }

    #[test]
    fn runner_supports_helical_ridge_plan() {
        let program = crate::ecky_scheme::compile_to_core_program(
            r#"
            (model
              (part body
                (helical-ridge
                  :radius 20
                  :pitch 6
                  :height 30
                  :base-width 2
                  :crest-width 1
                  :depth 1.5)))
            "#,
        )
        .expect("program");
        let plan = crate::ecky_cad_host::direct_occt::plan_core_program(&program).expect("plan");
        assert!(
            runner_supports_plan(&plan),
            "helical-ridge plan must be runner-safe"
        );
    }

    #[test]
    fn user_honjo_stay_clamp_routes_through_runner_when_available() {
        let root = temp_root("user-honjo-runner");
        let runner = root
            .join("resources")
            .join("bin")
            .join("direct-occt-runner");
        let output_dir = root.join("bundle");
        fs::create_dir_all(runner.parent().expect("runner parent")).expect("mkdir");
        write_executable(
            &runner,
            r#"#!/bin/sh
out=""
while [ "$#" -gt 0 ]; do
  case "$1" in
    --out)
      out="$2"
      shift 2
      ;;
    --plan)
      shift 2
      ;;
    *)
      shift
      ;;
  esac
done
mkdir -p "$out"
: > "$out/model.step"
: > "$out/preview.stl"
printf '{"parts":[{"partId":"honjo_stay_clamp_v2","label":"Honjo Stay Clamp V2","edges":[],"faces":[]}]}' > "$out/topology.json"
"#
            .to_owned()
                + fake_runner_stage_report_command(),
        );
        let resolver = TestResolver { root: root.clone() };
        let layout = crate::ecky_cad_host::direct_occt_sdk::DirectOcctSdkLayout {
            runtime_root: root.join("runtime").join("occt"),
            dylib_dir: None,
            include_dir: None,
            missing_headers: Vec::new(),
            missing_libs: Vec::new(),
            install_name_prefix: "@rpath",
        };

        let outcome =
            crate::ecky_cad_host::direct_occt_executor::export_core_program_step_stl_with_params_runner_first(
                &user_honjo_stay_clamp_program(),
                &user_honjo_stay_clamp_params(),
                &layout,
                &output_dir,
                &resolver,
            )
            .expect("runner export");

        let crate::ecky_cad_host::direct_occt_sdk::NativeExportOutcome::Exported {
            step_path,
            stl_path,
            ..
        } = outcome
        else {
            panic!("expected runner export for user honjo stay clamp");
        };

        assert!(output_dir.join(PLAN_FILE_NAME).is_file());
        assert!(output_dir.join("topology.json").is_file());
        assert!(step_path.is_file());
        assert!(stl_path.is_file());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn runner_supports_plan_accepts_keyword_free_frame_ops() {
        let cases = [
            OcctOp::Plane,
            OcctOp::Location,
            OcctOp::PathFrame,
            OcctOp::Place,
        ];

        for op in cases {
            let plan = sample_plan_for_command(OcctCommand {
                output: OcctSlot(1),
                op,
                args: Vec::new(),
                keywords: Vec::new(),
            });
            assert!(
                runner_supports_plan(&plan),
                "runner support gate for {}",
                runner_op_token(op)
            );
        }
    }

    #[test]
    fn supported_runner_plan_rejection_is_hard_error_not_fallback() {
        let root = temp_root("runner-supported-plan-rejection");
        fs::create_dir_all(
            root.join("resources")
                .join("runtime")
                .join("occt")
                .join("bin"),
        )
        .expect("runner dir");
        let runner = root
            .join("resources")
            .join("runtime")
            .join("occt")
            .join("bin")
            .join("direct-occt-runner");
        write_executable(
            &runner,
            r#"#!/bin/sh
if [ "${1:-}" = "--version" ]; then
  echo "direct-occt-runner 0.1.0"
  exit 0
fi
echo '{"class":"validation_error","code":"unsupported_op","message":"forced unsupported","details":"boom"}' >&2
exit 11
"#,
        );
        let resolver = TestResolver { root: root.clone() };
        let output_dir = root.join("bundle");

        let err =
            run_plan_step_stl_with_mode(&supported_sample_plan(), &output_dir, &resolver, true)
                .expect_err("supported runner-safe plan must hard fail");
        let diagnostic = format!("{} {}", err, err.details.as_deref().unwrap_or(""));
        assert!(
            diagnostic.contains("runner support gate accepted")
                || diagnostic.contains("forced unsupported"),
            "unexpected error: {err:?}"
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn serializes_plan_json_for_runner_abi() {
        let plan_json = serialize_runner_plan(&shell_clause_plan())
            .expect("plan serialization")
            .expect("runner plan");
        let json: serde_json::Value = serde_json::from_str(&plan_json).expect("json");

        assert_eq!(json["schemaVersion"], 1);
        assert!(json["planId"].as_str().unwrap().starts_with("sha256:"));
        assert!(json.get("parameters").is_none());
        assert_eq!(json["parts"][0]["key"], "body");
        assert_eq!(json["parts"][0]["label"], "Body");
        assert_eq!(json["parts"][0]["root"], 2);
        assert_eq!(json["parts"][0]["commands"][1]["op"], "shell");
        assert_eq!(json["parts"][0]["commands"][0]["args"][0]["kind"], "number");
        assert_eq!(json["parts"][0]["commands"][1]["args"][1]["kind"], "ref");
        assert_eq!(
            json["parts"][0]["commands"][1]["keywords"][0]["kind"],
            "selector"
        );
        assert_eq!(
            json["parts"][0]["commands"][1]["keywords"][0]["value"]["kind"],
            "text"
        );
        assert_eq!(
            json["parts"][0]["commands"][1]["keywords"][0]["payload"]["kind"],
            "face"
        );
        assert_eq!(
            json["parts"][0]["commands"][1]["keywords"][0]["payload"]["type"],
            "clauses"
        );
    }

    #[test]
    fn serializes_flattened_cutters_as_one_nary_difference() {
        let plan = compiled_plan(
            r#"
            (model
              (part body
                (difference
                  (box 40 30 20)
                  (union
                    (translate -8 0 0 (cylinder 2 24))
                    (union
                      (cylinder 2 24)
                      (translate 8 0 0 (cylinder 2 24)))))))
            "#,
        );
        let plan_json = serialize_runner_plan(&plan)
            .expect("plan serialization")
            .expect("runner plan");
        let json: serde_json::Value = serde_json::from_str(&plan_json).expect("json");
        let commands = json["parts"][0]["commands"].as_array().expect("commands");
        // parametric-thread-feature 3.1 binary-cut: base plus three flattened
        // cutter tools now serialize as a chain of three binary Differences
        // (each args.len() == 2), not one n-ary difference.
        let differences: Vec<_> = commands
            .iter()
            .filter(|command| command["op"] == "difference")
            .collect();
        assert_eq!(
            differences.len(),
            3,
            "three binary cuts for base plus three tools"
        );
        for difference in &differences {
            assert_eq!(
                difference["args"]
                    .as_array()
                    .expect("difference args")
                    .len(),
                2,
                "each serialized cut must be binary"
            );
        }
        assert!(
            commands.iter().all(|command| command["op"] != "union"),
            "dead cutter unions must not reach runner ABI"
        );
    }

    #[test]
    fn difference_tools_remain_before_fillet_and_chamfer_barriers() {
        let plan = compiled_plan(
            r#"
            (model
              (part filleted
                (fillet 0.5 :edges "vertical"
                  (difference
                    (box 40 30 20)
                    (union
                      (translate -7 0 -2 (cylinder 2 24))
                      (translate 7 0 -2 (cylinder 2 24))))))
              (part chamfered
                (chamfer 0.4 :edges "top"
                  (difference
                    (box 40 30 20)
                    (union
                      (translate -7 0 -2 (cylinder 2 24))
                      (translate 7 0 -2 (cylinder 2 24)))))))
            "#,
        );

        for part in &plan.parts {
            let barrier_index = part
                .commands
                .iter()
                .position(|command| matches!(command.op, OcctOp::Fillet | OcctOp::Chamfer))
                .expect("topology barrier");
            let barrier_input_slot = match part.commands[barrier_index].args.last() {
                Some(OcctArg::Ref(slot)) => *slot,
                other => panic!("barrier must consume a shape ref, got {other:?}"),
            };
            // parametric-thread-feature 3.1 binary-cut: the cut is now a chain
            // of binary Differences. The barrier (fillet/chamfer) consumes the
            // FINAL link's output, which keeps the original difference's slot.
            let final_difference_index = part
                .commands
                .iter()
                .position(|command| {
                    command.op == OcctOp::Difference && command.output == barrier_input_slot
                })
                .expect("barrier consumes a difference output");
            assert!(
                final_difference_index < barrier_index,
                "final binary cut must precede the topology barrier"
            );
            let differences: Vec<_> = part
                .commands
                .iter()
                .filter(|command| command.op == OcctOp::Difference)
                .collect();
            assert_eq!(
                differences.len(),
                2,
                "base plus two tools -> two binary cuts"
            );
            for command in &differences {
                assert_eq!(
                    command.args.len(),
                    2,
                    "each cut must be binary (base plus one tool)"
                );
            }
            assert_eq!(
                part.commands[barrier_index].args.last(),
                Some(&OcctArg::Ref(part.commands[final_difference_index].output))
            );
        }
    }

    #[test]
    fn serializes_edge_all_selector_without_runner_keyword() {
        let plan_json = serialize_runner_plan(&edge_all_fillet_plan())
            .expect("plan serialization")
            .expect("runner plan");
        let json: serde_json::Value = serde_json::from_str(&plan_json).expect("json");
        assert_eq!(
            json["parts"][0]["commands"][1]["keywords"]
                .as_array()
                .expect("keywords")
                .len(),
            0
        );
    }

    #[test]
    fn rejects_unresolved_params_during_runner_serialization() {
        let err = serialize_runner_plan(&sample_plan()).expect_err("unresolved param should fail");
        assert!(
            err.message.contains("requires resolved args"),
            "unexpected error: {:?}",
            err
        );
    }

    #[test]
    fn skips_unsupported_resolved_selector_plan_during_runner_serialization() {
        let plan = serialize_runner_plan(&unsupported_resolved_selector_plan())
            .expect("serialization should not error");
        assert!(
            plan.is_none(),
            "unsupported plan should skip runner serialization"
        );
    }

    #[test]
    fn discovers_runner_from_resources_and_skips_when_disabled() {
        let root = temp_root("discover");
        let runner = root
            .join("resources")
            .join("runtime")
            .join("occt")
            .join("bin")
            .join("direct-occt-runner");
        fs::create_dir_all(runner.parent().expect("runner parent")).expect("mkdir");
        fs::write(&runner, "#!/bin/sh\nexit 0\n").expect("write runner");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut permissions = fs::metadata(&runner).expect("metadata").permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(&runner, permissions).expect("chmod");
        }
        let resolver = TestResolver { root };

        assert_eq!(
            discover_direct_occt_runner_with_mode(&resolver, true),
            Some(runner.clone())
        );
        assert_eq!(
            discover_direct_occt_runner_with_mode(&resolver, false),
            None
        );
    }

    #[cfg(unix)]
    #[test]
    fn runner_failure_preserves_stdout_stderr_and_exit_status() {
        static LOCK: OnceLock<std::sync::Mutex<()>> = OnceLock::new();
        let _guard = LOCK
            .get_or_init(|| std::sync::Mutex::new(()))
            .lock()
            .expect("lock");

        let root = temp_root("runner-failure");
        let runner = root
            .join("resources")
            .join("bin")
            .join("direct-occt-runner");
        fs::create_dir_all(runner.parent().expect("runner parent")).expect("mkdir");
        write_executable(
            &runner,
            r#"#!/bin/sh
echo "runner stdout" 
echo "runner stderr" >&2
exit 7
"#,
        );
        let resolver = TestResolver { root: root.clone() };
        let output_dir = root.join("bundle");
        let err =
            run_plan_step_stl_with_mode(&supported_sample_plan(), &output_dir, &resolver, true)
                .expect_err("runner failure");

        let details = err.details.expect("details");
        assert!(details.contains("runner stdout"));
        assert!(details.contains("runner stderr"));
        assert!(details.contains("exit: 7"));
        assert!(output_dir.join(PLAN_FILE_NAME).is_file());
    }

    #[cfg(unix)]
    #[test]
    fn keyword_runner_plan_reports_structured_unsupported_as_hard_error() {
        let root = temp_root("unsupported-json-skip");
        let runner = root
            .join("resources")
            .join("bin")
            .join("direct-occt-runner");
        let output_dir = root.join("bundle");
        fs::create_dir_all(runner.parent().expect("runner parent")).expect("mkdir");
        write_executable(
            &runner,
            r#"#!/bin/sh
echo '{"class":"validation_error","code":"unsupported_op","message":"unsupported direct OCCT op `box`","details":"unsupported direct OCCT op `box`"}' >&2
exit 3
"#,
        );
        let resolver = TestResolver { root: root.clone() };

        let err =
            run_plan_step_stl_with_mode(&supported_sample_plan(), &output_dir, &resolver, true)
                .expect_err("structured unsupported must not silently fall back");
        let diagnostic = format!("{} {}", err, err.details.as_deref().unwrap_or_default());
        assert!(diagnostic.contains("unsupported direct OCCT op `box`"));
        assert!(output_dir.join(PLAN_FILE_NAME).is_file());
        let _ = fs::remove_dir_all(root);
    }

    #[cfg(unix)]
    #[test]
    fn runner_first_path_uses_discovered_runner_and_writes_plan_and_artifacts() {
        static LOCK: OnceLock<std::sync::Mutex<()>> = OnceLock::new();
        let _guard = LOCK
            .get_or_init(|| std::sync::Mutex::new(()))
            .lock()
            .expect("lock");

        let root = temp_root("runner-first-route");
        let source_dir = root.join("source");
        let runner = root
            .join("resources")
            .join("bin")
            .join("direct-occt-runner");
        let output_dir = root.join("bundle");
        fs::create_dir_all(&source_dir).expect("source dir");
        fs::create_dir_all(runner.parent().expect("runner parent")).expect("mkdir");
        fs::write(source_dir.join(MODEL_STEP_FILE_NAME), b"baseline-step").expect("step");
        fs::write(source_dir.join(PREVIEW_STL_FILE_NAME), b"baseline-stl").expect("stl");
        fs::write(
            source_dir.join("topology.json"),
            r#"{"parts":[{"partId":"body","label":"Body","edges":[],"faces":[]}]}"#,
        )
        .expect("topology");
        let runner_script = format!(
            r#"#!/bin/sh
set -eu
source_dir='{}'
plan=""
out=""
while [ "$#" -gt 0 ]; do
  case "$1" in
    --plan)
      plan=$2
      shift 2
      ;;
    --out)
      out=$2
      shift 2
      ;;
    *)
      echo "unexpected arg: $1" >&2
      exit 1
      ;;
  esac
done
mkdir -p "$out"
cp "$source_dir/model.step" "$out/model.step"
cp "$source_dir/preview.stl" "$out/preview.stl"
cp "$source_dir/topology.json" "$out/topology.json"
echo "fake runner plan: $plan"
{}"#,
            source_dir.display(),
            fake_runner_stage_report_command()
        );
        write_executable(&runner, &runner_script);
        let resolver = TestResolver { root: root.clone() };

        let outcome =
            run_plan_step_stl_with_mode(&supported_sample_plan(), &output_dir, &resolver, true)
                .expect("runner export");
        let Some(crate::ecky_cad_host::direct_occt_sdk::NativeExportOutcome::Exported {
            step_path,
            stl_path,
            ..
        }) = outcome
        else {
            panic!("expected runner export");
        };

        let plan_json = fs::read_to_string(output_dir.join(PLAN_FILE_NAME)).expect("plan json");
        let plan: serde_json::Value = serde_json::from_str(&plan_json).expect("plan");
        assert_eq!(plan["schemaVersion"], 1);
        assert!(plan["planId"].as_str().unwrap().starts_with("sha256:"));
        assert_eq!(plan["parts"][0]["commands"][0]["op"], "box");

        assert_eq!(fs::read(&step_path).expect("step"), b"baseline-step");
        assert_eq!(fs::read(&stl_path).expect("stl"), b"baseline-stl");
        assert_eq!(
            fs::read_to_string(output_dir.join("topology.json")).expect("topology"),
            r#"{"parts":[{"partId":"body","label":"Body","edges":[],"faces":[]}]}"#
        );

        let _ = fs::remove_dir_all(root);
    }

    #[cfg(unix)]
    #[test]
    fn keyword_free_frame_plan_uses_runner_when_available() {
        let root = temp_root("frame-op-runner");
        let runner = root
            .join("resources")
            .join("bin")
            .join("direct-occt-runner");
        let output_dir = root.join("bundle");
        fs::create_dir_all(runner.parent().expect("runner parent")).expect("mkdir");
        write_executable(
            &runner,
            r#"#!/bin/sh
out=""
while [ "$#" -gt 0 ]; do
  case "$1" in
    --out)
      out="$2"
      shift 2
      ;;
    --plan)
      shift 2
      ;;
    *)
      shift
      ;;
  esac
done
mkdir -p "$out"
: > "$out/model.step"
: > "$out/preview.stl"
: > "$out/topology.json"
"#
            .to_owned()
                + fake_runner_stage_report_command(),
        );
        let resolver = TestResolver { root: root.clone() };

        let outcome =
            run_plan_step_stl_with_mode(&keyword_free_plane_plan(), &output_dir, &resolver, true)
                .expect("frame runner export");

        let Some(crate::ecky_cad_host::direct_occt_sdk::NativeExportOutcome::Exported {
            step_path,
            stl_path,
            ..
        }) = outcome
        else {
            panic!("expected frame runner export");
        };

        assert!(output_dir.join(PLAN_FILE_NAME).is_file());
        assert!(output_dir.join("topology.json").is_file());
        assert!(step_path.is_file());
        assert!(stl_path.is_file());
        let _ = fs::remove_dir_all(root);
    }

    #[cfg(unix)]
    #[test]
    fn supported_keyword_plans_use_runner_when_available() {
        let root = temp_root("keyword-op-runner");
        let runner = root
            .join("resources")
            .join("bin")
            .join("direct-occt-runner");
        let output_dir = root.join("bundle");
        fs::create_dir_all(runner.parent().expect("runner parent")).expect("mkdir");
        write_executable(
            &runner,
            r#"#!/bin/sh
out=""
while [ "$#" -gt 0 ]; do
  case "$1" in
    --out)
      out="$2"
      shift 2
      ;;
    --plan)
      shift 2
      ;;
    *)
      shift
      ;;
  esac
done
mkdir -p "$out"
: > "$out/model.step"
: > "$out/preview.stl"
printf '{"parts":[{"partId":"body","label":"Body","edges":[],"faces":[]}]}' > "$out/topology.json"
"#
            .to_owned()
                + fake_runner_stage_report_command(),
        );
        let resolver = TestResolver { root: root.clone() };

        for plan in [keyword_profile_holes_plan(), keyword_clip_box_plan()] {
            let outcome = run_plan_step_stl_with_mode(&plan, &output_dir, &resolver, true)
                .expect("keyword runner export");
            let Some(crate::ecky_cad_host::direct_occt_sdk::NativeExportOutcome::Exported {
                step_path,
                stl_path,
                ..
            }) = outcome
            else {
                panic!("expected keyword runner export");
            };

            assert!(output_dir.join(PLAN_FILE_NAME).is_file());
            assert!(output_dir.join("topology.json").is_file());
            assert!(step_path.is_file());
            assert!(stl_path.is_file());
        }

        let _ = fs::remove_dir_all(root);
    }

    #[cfg(unix)]
    #[test]
    fn supported_exact_selector_plans_use_runner_when_available() {
        let root = temp_root("exact-selector-runner");
        let runner = root
            .join("resources")
            .join("bin")
            .join("direct-occt-runner");
        let output_dir = root.join("bundle");
        fs::create_dir_all(runner.parent().expect("runner parent")).expect("mkdir");
        write_executable(
            &runner,
            r#"#!/bin/sh
out=""
while [ "$#" -gt 0 ]; do
  case "$1" in
    --out)
      out="$2"
      shift 2
      ;;
    --plan)
      shift 2
      ;;
    *)
      shift
      ;;
  esac
done
mkdir -p "$out"
: > "$out/model.step"
: > "$out/preview.stl"
printf '{"parts":[{"partId":"body","label":"Body","edges":[],"faces":[]}]}' > "$out/topology.json"
"#
            .to_owned()
                + fake_runner_stage_report_command(),
        );
        let resolver = TestResolver { root: root.clone() };

        for plan in [
            shell_plan(),
            edge_all_fillet_plan(),
            edge_all_chamfer_plan(),
            exact_fillet_plan(),
            exact_chamfer_plan(),
            clause_fillet_plan(),
            clause_chamfer_plan(),
            exact_shell_plan(),
            shell_clause_plan(),
        ] {
            let outcome = run_plan_step_stl_with_mode(&plan, &output_dir, &resolver, true)
                .expect("exact selector runner export");
            let Some(crate::ecky_cad_host::direct_occt_sdk::NativeExportOutcome::Exported {
                step_path,
                stl_path,
                ..
            }) = outcome
            else {
                panic!("expected exact selector runner export");
            };

            assert!(output_dir.join(PLAN_FILE_NAME).is_file());
            assert!(output_dir.join("topology.json").is_file());
            assert!(step_path.is_file());
            assert!(stl_path.is_file());
        }

        let _ = fs::remove_dir_all(root);
    }

    #[cfg(unix)]
    #[test]
    fn runner_only_path_rejects_unsupported_plan_instead_of_requesting_generated_source_fallback() {
        let root = temp_root("unsupported-skip");
        let runner = root
            .join("resources")
            .join("bin")
            .join("direct-occt-runner");
        let output_dir = root.join("bundle");
        fs::create_dir_all(runner.parent().expect("runner parent")).expect("mkdir");
        write_executable(
            &runner,
            r#"#!/bin/sh
echo "runner should not run" >&2
exit 7
"#,
        );
        let resolver = TestResolver { root: root.clone() };

        let error = run_plan_step_stl_with_mode(
            &unsupported_resolved_selector_plan(),
            &output_dir,
            &resolver,
            true,
        )
        .expect_err("runner-only path must reject unsupported plans");

        assert!(error.to_string().contains("runner does not support plan"));
        assert!(!output_dir.join(PLAN_FILE_NAME).exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn live_precompiled_runner_exports_supported_plan_when_available() {
        let root = temp_root("live-precompiled");
        let resolver = TestResolver { root: root.clone() };
        let Some(runner) = discover_direct_occt_runner_with_mode(&resolver, true) else {
            return;
        };
        if !runner.is_file() {
            return;
        }

        let output_dir = root.join("bundle");
        let outcome =
            run_plan_step_stl_with_mode(&supported_sample_plan(), &output_dir, &resolver, true)
                .expect("live runner export");
        let Some(crate::ecky_cad_host::direct_occt_sdk::NativeExportOutcome::Exported {
            step_path,
            stl_path,
            ..
        }) = outcome
        else {
            panic!("expected live runner export");
        };

        assert!(output_dir.join(PLAN_FILE_NAME).is_file());
        assert!(output_dir.join("topology.json").is_file());
        assert!(
            fs::metadata(&step_path).expect("step metadata").len() > 1024,
            "STEP export too small"
        );
        assert!(
            fs::metadata(&stl_path).expect("stl metadata").len() > 512,
            "STL export too small"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn live_precompiled_runner_exports_expanded_keyword_free_subset_when_available() {
        let root = temp_root("live-precompiled-expanded");
        let resolver = TestResolver { root: root.clone() };
        let Some(runner) = discover_direct_occt_runner_with_mode(&resolver, true) else {
            return;
        };
        if !runner.is_file() {
            return;
        }

        let cases = [
            ("transform", expanded_transform_plan()),
            ("array", expanded_array_plan()),
            ("profile-surface", expanded_profile_surface_plan()),
            ("revolve", expanded_revolve_plan()),
            ("profile-offset-twist", expanded_profile_offset_twist_plan()),
        ];
        for (label, plan) in cases {
            assert!(
                runner_supports_plan(&plan),
                "runner support gate for {label}"
            );
            let output_dir = root.join(label);
            let outcome = run_plan_step_stl_with_mode(&plan, &output_dir, &resolver, true)
                .unwrap_or_else(|err| panic!("live runner export failed for {label}: {err}"));
            let Some(crate::ecky_cad_host::direct_occt_sdk::NativeExportOutcome::Exported {
                step_path,
                stl_path,
                ..
            }) = outcome
            else {
                panic!("expected live runner export for {label}");
            };

            assert!(
                output_dir.join(PLAN_FILE_NAME).is_file(),
                "missing plan for {label}"
            );
            assert!(
                output_dir.join("topology.json").is_file(),
                "missing topology for {label}"
            );
            assert!(
                fs::metadata(&step_path).expect("step metadata").len() > 1024,
                "STEP export too small for {label}"
            );
            assert!(
                fs::metadata(&stl_path).expect("stl metadata").len() > 512,
                "STL export too small for {label}"
            );
        }
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn live_precompiled_runner_accepts_profile_holes_and_emits_target_ids_when_available() {
        let Some((root, topology)) =
            run_real_runner_plan_json("live-runner-profile-holes", &keyword_profile_holes_plan())
        else {
            return;
        };

        let edges = topology["parts"][0]["edges"].as_array().expect("edges");
        let faces = topology["parts"][0]["faces"].as_array().expect("faces");
        assert!(!edges.is_empty(), "expected edges");
        assert!(!faces.is_empty(), "expected faces");
        assert!(
            edges[0]["targetId"]
                .as_str()
                .expect("edge target id")
                .starts_with("body:edge:"),
            "unexpected edge target id: {}",
            edges[0]["targetId"]
        );
        assert!(
            faces[0]["targetId"]
                .as_str()
                .expect("face target id")
                .starts_with("body:face:"),
            "unexpected face target id: {}",
            faces[0]["targetId"]
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn live_precompiled_runner_resolves_svg_wire_soup_artwork_when_available() {
        // Two disjoint filled squares = a compound the clean profile path rejects.
        // The tolerant wire-soup fallback hands both wires to the runner with a
        // fill-rule; OCCT must resolve them into two extruded regions.
        let program = crate::ecky_scheme::compile_to_core_program(
            r##"(model (part body (extrude (svg "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 20 10\"><path fill-rule=\"evenodd\" d=\"M0 0h4v4h-4z M10 0h4v4h-4z\"/></svg>" 20 10 "contain") 4)))"##,
        )
        .expect("compile");
        let plan = crate::ecky_cad_host::direct_occt::plan_core_program(&program).expect("plan");

        let Some((root, topology)) = run_real_runner_plan_json("live-runner-svg-wire-soup", &plan)
        else {
            return;
        };

        let faces = topology["parts"][0]["faces"].as_array().expect("faces");
        // Two extruded square prisms → at least the two top caps survive as faces.
        assert!(
            faces.len() >= 2,
            "expected wire-soup compound to yield multiple faces, got {}",
            faces.len()
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn live_precompiled_runner_accepts_clip_box_keywords_when_available() {
        let Some((root, topology)) =
            run_real_runner_plan_json("live-runner-clip-box", &keyword_clip_box_plan())
        else {
            return;
        };

        let faces = topology["parts"][0]["faces"].as_array().expect("faces");
        assert!(!faces.is_empty(), "expected faces");
        assert!(
            faces[0]["targetId"]
                .as_str()
                .expect("face target id")
                .starts_with("body:face:"),
            "unexpected face target id: {}",
            faces[0]["targetId"]
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn live_precompiled_runner_accepts_exact_selector_plans_when_available() {
        for (label, plan) in [
            ("live-runner-shell-default", shell_plan()),
            ("live-runner-fillet-edge-all", edge_all_fillet_plan()),
            ("live-runner-chamfer-edge-all", edge_all_chamfer_plan()),
            ("live-runner-fillet-exact", exact_fillet_plan()),
            ("live-runner-chamfer-exact", exact_chamfer_plan()),
            ("live-runner-fillet-clause", clause_fillet_plan()),
            ("live-runner-chamfer-clause", clause_chamfer_plan()),
            ("live-runner-shell-exact", exact_shell_plan()),
            ("live-runner-shell-clause", shell_clause_plan()),
            ("live-runner-draft", draft_plan()),
            ("live-runner-draft-neutral-z", draft_neutral_z_plan()),
        ] {
            let Some((root, topology)) = run_real_runner_plan_json(label, &plan) else {
                return;
            };

            let faces = topology["parts"][0]["faces"].as_array().expect("faces");
            assert!(!faces.is_empty(), "expected faces for {label}");
            let _ = fs::remove_dir_all(root);
        }
    }

    #[test]
    fn live_precompiled_runner_reports_structured_parse_and_schema_errors_when_available() {
        let Some((parse_root, parse_output)) =
            run_real_runner_plan_text("live-runner-parse-error", "{")
        else {
            return;
        };
        assert_eq!(parse_output.status.code(), Some(1));
        let parse_error: serde_json::Value =
            serde_json::from_slice(&parse_output.stderr).expect("parse error json");
        assert_eq!(parse_error["class"], "parse_error");
        assert_eq!(parse_error["code"], "parse_failed");
        let _ = fs::remove_dir_all(parse_root);

        let Some((schema_root, schema_output)) = run_real_runner_plan_text(
            "live-runner-schema-error",
            r#"{"schemaVersion":99,"planId":"bad","parts":[]}"#,
        ) else {
            return;
        };
        assert_eq!(schema_output.status.code(), Some(2));
        let schema_error: serde_json::Value =
            serde_json::from_slice(&schema_output.stderr).expect("schema error json");
        assert_eq!(schema_error["class"], "schema_error");
        assert_eq!(schema_error["code"], "schema_mismatch");
        let _ = fs::remove_dir_all(schema_root);

        let Some((param_root, param_output)) = run_real_runner_plan_text(
            "live-runner-param-schema-error",
            r#"{"schemaVersion":1,"planId":"bad","parts":[{"key":"body","label":"Body","root":1,"commands":[{"output":1,"op":"box","args":[{"kind":"param","value":"width"},{"kind":"number","value":8},{"kind":"number","value":4}],"keywords":[]}]}]}"#,
        ) else {
            return;
        };
        assert_eq!(param_output.status.code(), Some(2));
        let param_error: serde_json::Value =
            serde_json::from_slice(&param_output.stderr).expect("param schema error json");
        assert_eq!(param_error["class"], "schema_error");
        assert_eq!(param_error["code"], "schema_mismatch");
        let _ = fs::remove_dir_all(param_root);
    }
}
