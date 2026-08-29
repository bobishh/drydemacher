//! Repository-owned campaign definition loader. EDN is parsed as strict data;
//! no Steel engine or evaluator is involved.

use crate::contracts::{
    validate_artifact_bundle, AppError, AppResult, ArtifactBundle, DesignParams, GeometryBackend,
};
use crate::steel_data::{parse_steel_data, SteelDataValue};
use serde::Serialize;
use sha2::{Digest, Sha256};
use specta::Type;
use std::fs;
use std::path::{Path, PathBuf};
use tauri::Manager;

#[derive(Clone, Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CampaignSummary {
    pub definition_id: String,
    pub section_slug: String,
    pub title: String,
    pub step_count: usize,
    pub first_step_id: String,
}

#[derive(Clone, Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CampaignStepPayload {
    pub definition_id: String,
    pub definition_version: String,
    pub current_step: Option<CampaignCurrentStep>,
}

#[derive(Clone, Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CampaignCurrentStep {
    pub mission_index: usize,
    pub step_index: usize,
    pub step_count: usize,
    pub id: String,
    pub kind: String,
    pub title: String,
    pub prose: String,
    pub source: Option<String>,
    pub canonical_source_digest: Option<String>,
    pub canonical_preview: Option<CampaignCanonicalPreview>,
    pub acceptance: Option<CampaignAcceptance>,
    pub next_step_id: Option<String>,
    pub previous_step: Option<CampaignPreviousStep>,
    pub mission_count: usize,
}

#[derive(Clone, Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CampaignCanonicalPreview {
    pub canonical_source_digest: String,
    pub runtime_digest: String,
    pub artifact_bundle: ArtifactBundle,
}
#[derive(Clone, Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CampaignAcceptance {
    pub mode: String,
    pub reference_step_id: String,
}
#[derive(Clone, Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CampaignPreviousStep {
    pub id: String,
}

#[derive(Clone, Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CampaignPreviewPackReport {
    pub reused_count: usize,
    pub rendered_count: usize,
    pub missing_count: usize,
    pub missing_step_ids: Vec<String>,
}

#[derive(Clone, Debug)]
struct ManifestStep {
    id: String,
    kind: String,
    title: String,
    source: Option<String>,
    acceptance_reference_step_id: Option<String>,
}

#[derive(Clone, Debug)]
struct ManifestMission {
    id: String,
    content: String,
    steps: Vec<ManifestStep>,
}

/// Repository-owned, data-only preview package index. Each entry binds an
/// immutable runtime bundle to the exact canonical source used by a campaign
/// step. This loader never calls a renderer.
#[derive(Clone, Debug)]
struct PreviewIndexEntry {
    step_id: String,
    canonical_source_digest: String,
    runtime_digest: String,
    artifact_bundle_path: String,
    model_stl_path: String,
}

pub fn packaged_root(app: &tauri::AppHandle) -> AppResult<PathBuf> {
    let resource = app
        .path()
        .resource_dir()
        .map_err(|error| AppError::not_found(format!("Campaign resources unavailable: {error}")))?
        .join("campaign-content/books/ecky-ir/missions");
    if resource.join("manifest.edn").is_file() {
        return Ok(resource);
    }
    Ok(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../docs/books/ecky-ir/missions"))
}

pub fn summaries(root: &Path) -> AppResult<Vec<CampaignSummary>> {
    let missions = load_manifest(root)?;
    let first = missions
        .first()
        .and_then(|mission| mission.steps.first())
        .ok_or_else(|| AppError::validation("Campaign manifest has no first step."))?;
    Ok(vec![CampaignSummary {
        definition_id: "ecky-ir-build-missions".to_owned(),
        section_slug: "ecky-ir".to_owned(),
        title: "Ecky: six build missions".to_owned(),
        step_count: missions.iter().map(|mission| mission.steps.len()).sum(),
        first_step_id: format!("{}/{}", missions[0].id, first.id),
    }])
}

pub fn step(root: &Path, definition_id: &str, step_id: &str) -> AppResult<CampaignStepPayload> {
    let manifest_raw = fs::read(root.join("manifest.edn"))
        .map_err(|error| AppError::not_found(format!("Campaign manifest unavailable: {error}")))?;
    let definition_version = format!("sha256:{:x}", Sha256::digest(&manifest_raw));
    if definition_id != "ecky-ir-build-missions" {
        return Err(AppError::not_found(format!(
            "Campaign definition not found: {definition_id}"
        )));
    }
    let missions = load_manifest(root)?;
    let (mission_id, local_step_id) = step_id
        .split_once('/')
        .ok_or_else(|| AppError::validation("Campaign step id must be mission-id/step-id."))?;
    let mission_index = missions
        .iter()
        .position(|mission| mission.id == mission_id)
        .ok_or_else(|| AppError::not_found(format!("Campaign mission not found: {mission_id}")))?;
    let mission = &missions[mission_index];
    let local_index = mission
        .steps
        .iter()
        .position(|step| step.id == local_step_id)
        .ok_or_else(|| {
            AppError::not_found(format!(
                "Campaign step not found: {definition_id}/{step_id}"
            ))
        })?;
    let step = &mission.steps[local_index];
    let flattened = missions
        .iter()
        .flat_map(|mission| mission.steps.iter().map(move |step| (mission, step)))
        .collect::<Vec<_>>();
    let step_index = flattened
        .iter()
        .position(|(candidate_mission, candidate_step)| {
            candidate_mission.id == mission.id && candidate_step.id == step.id
        })
        .unwrap_or(0);
    let markdown = read_repo_file(root, &mission.content)?;
    let prose_markdown = markdown_section(&markdown, &step.id).ok_or_else(|| {
        AppError::validation(format!(
            "Campaign section missing or empty: {definition_id}/{}",
            step.id
        ))
    })?;
    let source = step
        .source
        .as_deref()
        .map(|path| read_repo_file(root, path))
        .transpose()?;
    let canonical_source_digest = source
        .as_ref()
        .map(|source| format!("sha256:{:x}", Sha256::digest(source.as_bytes())));
    let composite_step_id = format!("{}/{}", mission.id, step.id);
    let canonical_preview = match canonical_source_digest.as_deref() {
        Some(source_digest) => resolve_canonical_preview(root, &composite_step_id, source_digest)?,
        None => None,
    };
    Ok(CampaignStepPayload {
        definition_id: definition_id.to_owned(),
        definition_version,
        current_step: Some(CampaignCurrentStep {
            mission_index: mission_index + 1,
            mission_count: missions.len(),
            step_index: step_index + 1,
            step_count: flattened.len(),
            id: composite_step_id,
            kind: step.kind.clone(),
            title: step.title.clone(),
            prose: prose_markdown,
            source,
            canonical_source_digest,
            canonical_preview,
            acceptance: step
                .acceptance_reference_step_id
                .clone()
                .map(|reference_step_id| CampaignAcceptance {
                    mode: "equivalentCoreIr".to_owned(),
                    reference_step_id: format!("{}/{}", mission.id, reference_step_id),
                }),
            next_step_id: flattened
                .get(step_index + 1)
                .map(|(mission, step)| format!("{}/{}", mission.id, step.id)),
            previous_step: step_index
                .checked_sub(1)
                .and_then(|index| flattened.get(index))
                .map(|(mission, step)| CampaignPreviousStep {
                    id: format!("{}/{}", mission.id, step.id),
                }),
        }),
    })
}

pub fn check_step(
    root: &Path,
    definition_id: &str,
    step_id: &str,
    candidate_source: String,
) -> AppResult<crate::commands::mission_evaluation::MissionCoreIrEvaluation> {
    if definition_id != "ecky-ir-build-missions" {
        return Err(AppError::not_found(format!(
            "Campaign definition not found: {definition_id}"
        )));
    }
    let (mission_id, local_step_id) = step_id
        .split_once('/')
        .ok_or_else(|| AppError::validation("Campaign step id must be mission-id/step-id."))?;
    let missions = load_manifest(root)?;
    let mission = missions
        .iter()
        .find(|mission| mission.id == mission_id)
        .ok_or_else(|| AppError::not_found(format!("Campaign mission not found: {mission_id}")))?;
    let candidate_step = mission
        .steps
        .iter()
        .find(|step| step.id == local_step_id)
        .ok_or_else(|| AppError::not_found(format!("Campaign step not found: {step_id}")))?;
    let reference_id = candidate_step
        .acceptance_reference_step_id
        .as_deref()
        .ok_or_else(|| {
            AppError::validation(format!(
                "Campaign step has no acceptance reference: {step_id}"
            ))
        })?;
    let reference_step = mission
        .steps
        .iter()
        .find(|step| step.id == reference_id)
        .ok_or_else(|| {
            AppError::validation(format!(
                "Campaign acceptance reference missing: {mission_id}/{reference_id}"
            ))
        })?;
    let reference_path = reference_step.source.as_deref().ok_or_else(|| {
        AppError::validation(format!(
            "Campaign acceptance reference has no source: {mission_id}/{reference_id}"
        ))
    })?;
    let reference_source = read_repo_file(root, reference_path)?;
    crate::commands::mission_evaluation::evaluate_mission_core_ir(
        candidate_source,
        reference_source,
    )
}

fn resolve_canonical_preview(
    root: &Path,
    step_id: &str,
    source_digest: &str,
) -> AppResult<Option<CampaignCanonicalPreview>> {
    let index_path = root.join("previews/index.edn");
    if !index_path.is_file() {
        return Ok(None);
    }
    let entries = load_preview_index(&index_path)?;
    let Some(entry) = entries.into_iter().find(|entry| entry.step_id == step_id) else {
        return Ok(None);
    };
    if entry.canonical_source_digest != source_digest {
        return Ok(None);
    }
    let bundle_path =
        preview_package_path(root, &entry.artifact_bundle_path, "artifactBundlePath")?;
    let model_stl_path = preview_package_path(root, &entry.model_stl_path, "modelStlPath")?;
    let raw_bundle = fs::read(&bundle_path).map_err(|error| {
        AppError::not_found(format!("Campaign preview bundle unavailable: {error}"))
    })?;
    let actual_runtime_digest = format!("sha256:{:x}", Sha256::digest(&raw_bundle));
    if actual_runtime_digest != entry.runtime_digest {
        return Err(AppError::validation(format!(
            "Campaign preview runtime digest mismatch for {step_id}."
        )));
    }
    if !model_stl_path.is_file()
        || fs::metadata(&model_stl_path)
            .map_err(|error| {
                AppError::not_found(format!("Campaign model STL unavailable: {error}"))
            })?
            .len()
            == 0
    {
        return Err(AppError::validation(format!(
            "Campaign model STL missing or empty for {step_id}."
        )));
    }
    let mut artifact_bundle: ArtifactBundle =
        serde_json::from_slice(&raw_bundle).map_err(|error| {
            AppError::parse(format!("Campaign preview bundle invalid JSON: {error}"))
        })?;
    artifact_bundle.model_stl_path = model_stl_path.to_string_lossy().into_owned();
    validate_artifact_bundle(&artifact_bundle)?;
    Ok(Some(CampaignCanonicalPreview {
        canonical_source_digest: entry.canonical_source_digest,
        runtime_digest: entry.runtime_digest,
        artifact_bundle,
    }))
}

fn load_preview_index(index_path: &Path) -> AppResult<Vec<PreviewIndexEntry>> {
    let raw = fs::read_to_string(index_path).map_err(|error| {
        AppError::not_found(format!("Campaign preview index unavailable: {error}"))
    })?;
    let value = parse_steel_data(&raw).map_err(|error| {
        AppError::parse(format!("Campaign preview index invalid data: {error}"))
    })?;
    let index = map(&value, "preview index")?;
    let schema_version = integer(
        required(index, "schema-version")?,
        "preview index.schema-version",
    )?;
    if schema_version != 1 {
        return Err(AppError::validation(format!(
            "Campaign preview index schema-version {schema_version} is unsupported."
        )));
    }
    let entries = vector(required(index, "entries")?, "preview index.entries")?;
    let mut result = Vec::with_capacity(entries.len());
    let mut seen = std::collections::HashSet::new();
    for value in entries {
        let entry = map(value, "preview index entry")?;
        let parsed = PreviewIndexEntry {
            step_id: string(required(entry, "step-id")?, "preview step-id")?.to_owned(),
            canonical_source_digest: string(
                required(entry, "canonical-source-digest")?,
                "preview canonical-source-digest",
            )?
            .to_owned(),
            runtime_digest: string(required(entry, "runtime-digest")?, "preview runtime-digest")?
                .to_owned(),
            artifact_bundle_path: string(
                required(entry, "artifact-bundle-path")?,
                "preview artifact-bundle-path",
            )?
            .to_owned(),
            model_stl_path: optional_string(entry, "model-stl-path")?
                .or(optional_string(entry, "preview-stl-path")?)
                .ok_or_else(|| {
                    AppError::validation("Campaign preview index missing model-stl-path.")
                })?
                .to_owned(),
        };
        if !seen.insert(parsed.step_id.clone()) {
            return Err(AppError::validation(format!(
                "Campaign preview duplicate step-id: {}",
                parsed.step_id
            )));
        }
        if !parsed.canonical_source_digest.starts_with("sha256:")
            || !parsed.runtime_digest.starts_with("sha256:")
        {
            return Err(AppError::validation(format!(
                "Campaign preview digests must use sha256: for {}.",
                parsed.step_id
            )));
        }
        result.push(parsed);
    }
    Ok(result)
}

fn preview_package_path(root: &Path, relative: &str, label: &str) -> AppResult<PathBuf> {
    let relative = Path::new(relative);
    if relative.is_absolute()
        || relative
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        return Err(AppError::validation(format!(
            "Campaign preview {label} escapes its package."
        )));
    }
    let package_root = root.join("previews");
    Ok(package_root.join(relative))
}

/// Explicit maintainer operation. It processes campaign sources in manifest
/// order, reuses valid model-runtime artifacts by exact source digest, and
/// renders no more than one absent source per call.
pub fn pack_previews(
    root: &Path,
    render_one_missing: bool,
    app: &dyn crate::models::PathResolver,
) -> AppResult<CampaignPreviewPackReport> {
    let missions = load_manifest(root)?;
    let mut reused_count = 0;
    let mut rendered_count = 0;
    let mut missing_step_ids = Vec::new();
    let mut rendered_source = false;
    for mission in missions {
        for step in mission.steps {
            let Some(source_path) = step.source.as_deref() else {
                continue;
            };
            let source = read_repo_file(root, source_path)?;
            let source_digest = format!("sha256:{:x}", Sha256::digest(source.as_bytes()));
            let step_id = format!("{}/{}", mission.id, step.id);
            if resolve_canonical_preview(root, &step_id, &source_digest)?.is_some() {
                continue;
            }
            if let Some(bundle) = reusable_runtime_bundle(app, &source_digest)? {
                store_preview_package(root, &step_id, &source_digest, &bundle)?;
                reused_count += 1;
                continue;
            }
            if render_one_missing && !rendered_source {
                let bundle = crate::services::render::render_cli_ecky(
                    &source,
                    &DesignParams::new(),
                    GeometryBackend::EckyRust,
                    None,
                    app,
                )?;
                store_preview_package(root, &step_id, &source_digest, &bundle)?;
                rendered_source = true;
                rendered_count += 1;
            } else {
                missing_step_ids.push(step_id);
            }
        }
    }
    Ok(CampaignPreviewPackReport {
        reused_count,
        rendered_count,
        missing_count: missing_step_ids.len(),
        missing_step_ids,
    })
}

fn reusable_runtime_bundle(
    app: &dyn crate::models::PathResolver,
    source_digest: &str,
) -> AppResult<Option<ArtifactBundle>> {
    let runtime_root = crate::model_runtime::runtime_root(app)?;
    let mut pending = vec![runtime_root];
    while let Some(directory) = pending.pop() {
        let entries = match fs::read_dir(&directory) {
            Ok(entries) => entries,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
                continue;
            }
            if path.file_name().and_then(|name| name.to_str()) != Some("bundle.json") {
                continue;
            }
            let Ok(raw) = fs::read(&path) else {
                continue;
            };
            let Ok(unchecked_bundle) = serde_json::from_slice::<ArtifactBundle>(&raw) else {
                continue;
            };
            let Ok((bundle, manifest)) =
                crate::model_runtime::read_runtime_bundle(app, &unchecked_bundle.model_id)
            else {
                continue;
            };
            if manifest.source_digest.as_deref() != Some(source_digest) {
                continue;
            }
            let Ok(source) = bundle
                .macro_path
                .as_deref()
                .map(fs::read_to_string)
                .transpose()
            else {
                continue;
            };
            let Some(source) = source else {
                continue;
            };
            if format!("sha256:{:x}", Sha256::digest(source.as_bytes())) != source_digest {
                continue;
            }
            if !Path::new(&bundle.model_stl_path).is_file() {
                continue;
            }
            if validate_artifact_bundle(&bundle).is_ok() {
                return Ok(Some(bundle));
            }
        }
    }
    Ok(None)
}

fn store_preview_package(
    root: &Path,
    step_id: &str,
    source_digest: &str,
    bundle: &ArtifactBundle,
) -> AppResult<()> {
    let preview_source = Path::new(&bundle.model_stl_path);
    if !preview_source.is_file() {
        return Err(AppError::validation(format!(
            "Cannot package preview for {step_id}: runtime model STL is missing."
        )));
    }
    let package_key = source_digest.trim_start_matches("sha256:");
    let package_relative = format!("{package_key}/bundle.json");
    let stl_relative = format!("{package_key}/model.stl");
    let package_dir = root.join("previews").join(package_key);
    fs::create_dir_all(&package_dir).map_err(|error| {
        AppError::persistence(format!("Cannot create campaign preview package: {error}"))
    })?;
    fs::copy(preview_source, package_dir.join("model.stl")).map_err(|error| {
        AppError::persistence(format!("Cannot copy campaign model STL: {error}"))
    })?;
    let mut packaged_bundle = bundle.clone();
    packaged_bundle.model_stl_path = "model.stl".to_owned();
    let bundle_bytes = serde_json::to_vec_pretty(&packaged_bundle).map_err(|error| {
        AppError::persistence(format!("Cannot serialize campaign preview bundle: {error}"))
    })?;
    fs::write(package_dir.join("bundle.json"), &bundle_bytes).map_err(|error| {
        AppError::persistence(format!("Cannot write campaign preview bundle: {error}"))
    })?;
    upsert_preview_index(
        root,
        PreviewIndexEntry {
            step_id: step_id.to_owned(),
            canonical_source_digest: source_digest.to_owned(),
            runtime_digest: format!("sha256:{:x}", Sha256::digest(&bundle_bytes)),
            artifact_bundle_path: package_relative,
            model_stl_path: stl_relative,
        },
    )
}

fn upsert_preview_index(root: &Path, entry: PreviewIndexEntry) -> AppResult<()> {
    use crate::steel_data::{write_steel_data, SteelDataValue};
    let index_path = root.join("previews/index.edn");
    let mut entries = if index_path.is_file() {
        load_preview_index(&index_path)?
    } else {
        Vec::new()
    };
    entries.retain(|existing| existing.step_id != entry.step_id);
    entries.push(entry);
    entries.sort_by(|left, right| left.step_id.cmp(&right.step_id));
    let entries = entries
        .into_iter()
        .map(|entry| {
            SteelDataValue::Map(vec![
                (":step-id".to_owned(), SteelDataValue::String(entry.step_id)),
                (
                    ":canonical-source-digest".to_owned(),
                    SteelDataValue::String(entry.canonical_source_digest),
                ),
                (
                    ":runtime-digest".to_owned(),
                    SteelDataValue::String(entry.runtime_digest),
                ),
                (
                    ":artifact-bundle-path".to_owned(),
                    SteelDataValue::String(entry.artifact_bundle_path),
                ),
                (
                    ":model-stl-path".to_owned(),
                    SteelDataValue::String(entry.model_stl_path),
                ),
            ])
        })
        .collect();
    let data = write_steel_data(&SteelDataValue::Map(vec![
        (":schema-version".to_owned(), SteelDataValue::Integer(1)),
        (":entries".to_owned(), SteelDataValue::Vector(entries)),
    ]))
    .map_err(|error| {
        AppError::persistence(format!("Cannot serialize campaign preview index: {error}"))
    })?;
    fs::write(index_path, data).map_err(|error| {
        AppError::persistence(format!("Cannot write campaign preview index: {error}"))
    })
}

fn load_manifest(root: &Path) -> AppResult<Vec<ManifestMission>> {
    let raw = fs::read_to_string(root.join("manifest.edn"))
        .map_err(|error| AppError::not_found(format!("Campaign manifest unavailable: {error}")))?;
    let value = parse_steel_data(&raw)
        .map_err(|error| AppError::parse(format!("Campaign manifest.edn invalid data: {error}")))?;
    let root_map = map(&value, "manifest")?;
    let missions = vector(required(root_map, "missions")?, "manifest.missions")?;
    if missions.is_empty() {
        return Err(AppError::validation(
            "Campaign manifest missions must not be empty.",
        ));
    }
    let mut result = Vec::with_capacity(missions.len());
    for value in missions {
        let mission = map(value, "mission")?;
        let _ = string(required(mission, "section-slug")?, "mission.section-slug")?;
        let _ = string(required(mission, "title")?, "mission.title")?;
        let steps = vector(required(mission, "steps")?, "mission.steps")?
            .iter()
            .map(|value| {
                let step = map(value, "step")?;
                let acceptance = optional_map(step, "acceptance")?;
                let _ = optional_string(step, "reveal")?;
                Ok(ManifestStep {
                    id: string(required(step, "id")?, "step.id")?.to_owned(),
                    kind: string(required(step, "kind")?, "step.kind")?.to_owned(),
                    title: string(required(step, "title")?, "step.title")?.to_owned(),
                    source: optional_string(step, "source")?.map(str::to_owned),
                    acceptance_reference_step_id: acceptance
                        .and_then(|value| {
                            optional_string(value, "reference-step-id").ok().flatten()
                        })
                        .map(str::to_owned),
                })
            })
            .collect::<AppResult<Vec<_>>>()?;
        result.push(ManifestMission {
            id: string(required(mission, "id")?, "mission.id")?.to_owned(),
            content: string(required(mission, "content")?, "mission.content")?.to_owned(),
            steps,
        });
    }
    for mission in &result {
        validate_sections(root, mission)?;
    }
    Ok(result)
}

fn validate_sections(root: &Path, mission: &ManifestMission) -> AppResult<()> {
    let markdown = read_repo_file(root, &mission.content)?;
    let mut section_ids = std::collections::HashSet::new();
    for line in markdown.lines().filter(|line| line.starts_with("## ")) {
        let Some(id) = line
            .rsplit_once("{#")
            .and_then(|(_, value)| value.strip_suffix('}'))
        else {
            continue;
        };
        if !section_ids.insert(id.to_owned()) {
            return Err(AppError::validation(format!(
                "Campaign duplicate section id: {}/{}",
                mission.id, id
            )));
        }
        if markdown_section(&markdown, id).is_none() {
            return Err(AppError::validation(format!(
                "Campaign section empty: {}/{}",
                mission.id, id
            )));
        }
    }
    let step_ids = mission
        .steps
        .iter()
        .map(|step| step.id.as_str())
        .collect::<std::collections::HashSet<_>>();
    for step in &mission.steps {
        if !section_ids.contains(&step.id) {
            return Err(AppError::validation(format!(
                "Campaign section missing: {}/{}",
                mission.id, step.id
            )));
        }
    }
    for id in section_ids {
        if !step_ids.contains(id.as_str()) {
            return Err(AppError::validation(format!(
                "Campaign orphaned section: {}/{}",
                mission.id, id
            )));
        }
    }
    Ok(())
}

fn read_repo_file(root: &Path, repo_path: &str) -> AppResult<String> {
    let packaged = root
        .components()
        .any(|component| component.as_os_str() == "campaign-content");
    let repository = if packaged {
        root.ancestors()
            .nth(3)
            .ok_or_else(|| AppError::internal("Campaign resource root has no content parent."))?
    } else {
        root.ancestors()
            .nth(4)
            .ok_or_else(|| AppError::internal("Campaign root has no repository parent."))?
    };
    let relative = if packaged {
        repo_path.strip_prefix("docs/").unwrap_or(repo_path)
    } else {
        repo_path
    };
    let path = repository.join(relative);
    if !path.starts_with(repository) {
        return Err(AppError::validation("Campaign path escapes repository."));
    }
    fs::read_to_string(&path).map_err(|error| {
        AppError::not_found(format!("Campaign file unavailable {repo_path}: {error}"))
    })
}

fn markdown_section(markdown: &str, id: &str) -> Option<String> {
    let marker = format!(" {{#{id}}}");
    let start = markdown
        .lines()
        .scan(0usize, |offset, line| {
            let at = *offset;
            *offset += line.len() + 1;
            Some((at, line))
        })
        .find_map(|(offset, line)| {
            (line.starts_with("## ") && line.trim_end().ends_with(&marker))
                .then_some(offset + line.len() + 1)
        })?;
    let rest = &markdown[start..];
    let end = rest.find("\n## ").unwrap_or(rest.len());
    let prose = rest[..end].trim();
    (!prose.is_empty()).then(|| prose.to_owned())
}

fn map<'a>(value: &'a SteelDataValue, label: &str) -> AppResult<&'a [(String, SteelDataValue)]> {
    if let SteelDataValue::Map(value) = value {
        Ok(value)
    } else {
        Err(AppError::validation(format!(
            "Campaign {label} must be a map."
        )))
    }
}
fn vector<'a>(value: &'a SteelDataValue, label: &str) -> AppResult<&'a [SteelDataValue]> {
    if let SteelDataValue::Vector(value) = value {
        Ok(value)
    } else {
        Err(AppError::validation(format!(
            "Campaign {label} must be a vector."
        )))
    }
}
fn required<'a>(map: &'a [(String, SteelDataValue)], key: &str) -> AppResult<&'a SteelDataValue> {
    map.iter()
        .find(|(name, _)| name.trim_start_matches(':') == key)
        .map(|(_, value)| value)
        .ok_or_else(|| AppError::validation(format!("Campaign manifest missing {key}.")))
}
fn optional_string<'a>(
    map: &'a [(String, SteelDataValue)],
    key: &str,
) -> AppResult<Option<&'a str>> {
    match map
        .iter()
        .find(|(name, _)| name.trim_start_matches(':') == key)
    {
        None => Ok(None),
        Some((_, value)) => Ok(Some(string(value, key)?)),
    }
}
fn optional_map<'a>(
    entries: &'a [(String, SteelDataValue)],
    key: &str,
) -> AppResult<Option<&'a [(String, SteelDataValue)]>> {
    match entries
        .iter()
        .find(|(name, _)| name.trim_start_matches(':') == key)
    {
        None => Ok(None),
        Some((_, value)) => Ok(Some(map(value, key)?)),
    }
}
fn string<'a>(value: &'a SteelDataValue, label: &str) -> AppResult<&'a str> {
    if let SteelDataValue::String(value) = value {
        (!value.trim().is_empty())
            .then_some(value.as_str())
            .ok_or_else(|| AppError::validation(format!("Campaign {label} must not be empty.")))
    } else {
        Err(AppError::validation(format!(
            "Campaign {label} must be a string."
        )))
    }
}
fn integer(value: &SteelDataValue, label: &str) -> AppResult<i64> {
    if let SteelDataValue::Integer(value) = value {
        Ok(*value)
    } else {
        Err(AppError::validation(format!(
            "Campaign {label} must be an integer."
        )))
    }
}
