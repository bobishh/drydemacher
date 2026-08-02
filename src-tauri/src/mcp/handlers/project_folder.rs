use super::version_write::handle_commit_preview_version_from_external_source;
use super::{handle_macro_preview_render, handle_verify_generated_model, now_secs, AgentContext};
use crate::contracts::{AppError, AppResult};
use crate::db;
use crate::mcp::contracts::{AgentIdentityOverride, MacroReplaceRequest, VersionSaveRequest};
use crate::models::{AppState, PathResolver};
use std::collections::HashMap;

// --- Filesystem project mirror tools (filesystem-project-mirror T2) ---

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectFolderExportRequest {
    #[serde(flatten)]
    pub identity: AgentIdentityOverride,
    pub thread_id: Option<String>,
    pub message_id: Option<String>,
    /// Folder slug; defaults to a deterministic slug from title + thread id.
    pub slug: Option<String>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectFolderExportResponse {
    pub slug: String,
    pub folder: String,
    pub manifest: crate::project_mirror::ProjectManifest,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectFolderStatusRequest {
    pub slug: String,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectFolderApplyRequest {
    #[serde(flatten)]
    pub identity: AgentIdentityOverride,
    pub slug: String,
    /// Apply the file on top of the current thread head even when both the
    /// file and the thread moved since export (`conflict`).
    #[serde(default)]
    pub force: bool,
    pub title: Option<String>,
    pub version_name: Option<String>,
}

#[derive(Debug, serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ProjectFolderApplyResponse {
    pub state_before: crate::project_mirror::ProjectSyncState,
    /// True when the folder was already clean and nothing was committed.
    pub no_op: bool,
    pub thread_id: String,
    pub message_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,
    pub manifest: crate::project_mirror::ProjectManifest,
}

pub async fn handle_project_folder_export(
    state: &AppState,
    app: &dyn PathResolver,
    req: ProjectFolderExportRequest,
    ctx: &AgentContext,
) -> AppResult<ProjectFolderExportResponse> {
    let ctx = ctx.with_override(&req.identity);
    let ctx = &ctx;
    let _ = ctx;
    let target = {
        let conn = state.db.lock().await;
        crate::services::target::resolve_editable_target(
            &conn,
            app,
            req.thread_id.clone(),
            req.message_id.clone(),
        )?
    };
    let slug = match req.slug {
        Some(slug) => slug,
        None => crate::project_mirror::project_slug(&target.design_output.title, &target.thread_id),
    };
    let model_id = target
        .artifact_bundle
        .as_ref()
        .map(|bundle| bundle.model_id.clone());
    let projects_root = configured_projects_root(state);
    let stored_binding = {
        let conn = state.db.lock().await;
        crate::thread_source_binding::get_binding(&conn, &target.thread_id)
            .map_err(|err| AppError::persistence(err.to_string()))?
    };
    if stored_binding.is_some() {
        let binding = {
            let conn = state.db.lock().await;
            crate::thread_source_binding::prepare_editor_source(
                app,
                &conn,
                projects_root.as_deref(),
                &target.thread_id,
                &target.design_output.title,
                &target.design_output.macro_code,
                &target.message_id,
                model_id.as_deref(),
            )?
        };
        let dir = std::path::PathBuf::from(&binding.folder_path);
        let (_, slug) = crate::thread_source_binding::stored_folder_export_args(&dir)?;
        let manifest = crate::project_mirror::read_manifest(&dir)?.ok_or_else(|| {
            AppError::persistence(format!(
                "Bound project folder '{}' lost its manifest.",
                dir.display()
            ))
        })?;
        match target
            .artifact_bundle
            .as_ref()
            .and_then(|bundle| bundle.component_dependency_lock.as_ref())
        {
            Some(lock) => crate::project_mirror::write_project_lock(&dir, lock)?,
            None => {
                let lock_path = dir.join(crate::project_mirror::PROJECT_LOCK_FILE_NAME);
                if lock_path.exists() {
                    std::fs::remove_file(&lock_path)
                        .map_err(|error| AppError::persistence(error.to_string()))?;
                }
            }
        }
        return Ok(ProjectFolderExportResponse {
            slug,
            folder: binding.folder_path,
            manifest,
        });
    }
    let (dir, manifest) = crate::project_mirror::export_project_with_lock(
        app,
        &crate::project_mirror::ExportProjectRequest {
            slug: &slug,
            thread_id: &target.thread_id,
            message_id: &target.message_id,
            model_id: model_id.as_deref(),
            source: &target.design_output.macro_code,
            projects_root: projects_root.as_deref(),
        },
        target
            .artifact_bundle
            .as_ref()
            .and_then(|bundle| bundle.component_dependency_lock.as_ref()),
    )?;

    // Index the export as this thread's bound source folder (digest-safe:
    // binding row digest == on-disk file digest == manifest digest).
    {
        let conn = state.db.lock().await;
        crate::thread_source_binding::index_export(
            &conn,
            &target.thread_id,
            &dir,
            &manifest.source_digest,
        )?;
    }

    Ok(ProjectFolderExportResponse {
        slug,
        folder: dir.to_string_lossy().to_string(),
        manifest,
    })
}

/// Configured `projectsRoot` override; `None` falls back to `<app_data>/projects`.
fn configured_projects_root(state: &AppState) -> Option<String> {
    state.config.lock().unwrap().projects_root.clone()
}

async fn project_folder_path(
    state: &AppState,
    slug: &str,
    app: &dyn PathResolver,
) -> AppResult<std::path::PathBuf> {
    let binding = {
        let conn = state.db.lock().await;
        crate::thread_source_binding::get_binding_by_folder_name(&conn, slug)
            .map_err(|err| AppError::persistence(err.to_string()))?
    };
    if let Some(binding) = binding {
        return Ok(std::path::PathBuf::from(binding.folder_path));
    }
    let root = configured_projects_root(state);
    crate::project_mirror::project_dir(app, root.as_deref(), slug)
}

async fn project_thread_head(
    state: &AppState,
    slug: &str,
    app: &dyn PathResolver,
) -> AppResult<Option<String>> {
    let dir = project_folder_path(state, slug, app).await?;
    let Some(manifest) = crate::project_mirror::read_manifest(&dir)? else {
        return Ok(None);
    };
    let conn = state.db.lock().await;
    db::get_latest_successful_message_id_in_thread(&conn, &manifest.thread_id)
        .map_err(|err| AppError::persistence(err.to_string()))
}

pub async fn handle_project_folder_status(
    state: &AppState,
    app: &dyn PathResolver,
    req: ProjectFolderStatusRequest,
) -> AppResult<crate::project_mirror::ProjectFolderStatus> {
    let head = project_thread_head(state, &req.slug, app).await?;
    let dir = project_folder_path(state, &req.slug, app).await?;
    let (root, slug) = crate::thread_source_binding::stored_folder_export_args(&dir)?;
    crate::project_mirror::folder_status(app, root.as_deref(), &slug, head.as_deref())
}

pub async fn handle_project_folder_apply(
    state: &AppState,
    app: &dyn PathResolver,
    req: ProjectFolderApplyRequest,
    ctx: &AgentContext,
) -> AppResult<ProjectFolderApplyResponse> {
    use crate::project_mirror::ProjectSyncState;

    let ctx = ctx.with_override(&req.identity);
    let ctx = &ctx;
    let status = handle_project_folder_status(
        state,
        app,
        ProjectFolderStatusRequest {
            slug: req.slug.clone(),
        },
    )
    .await?;

    let manifest = match (&status.state, status.manifest.clone()) {
        (ProjectSyncState::Missing, _) | (_, None) => {
            return Err(AppError::validation(format!(
                "Project folder `{}` has no exported model; run project_folder_export first.",
                req.slug
            )))
        }
        (_, Some(manifest)) => manifest,
    };

    match status.state {
        ProjectSyncState::Clean => {
            return Ok(ProjectFolderApplyResponse {
                state_before: ProjectSyncState::Clean,
                no_op: true,
                thread_id: manifest.thread_id.clone(),
                message_id: manifest.message_id.clone(),
                model_id: manifest.model_id.clone(),
                manifest,
            })
        }
        ProjectSyncState::ThreadAdvanced => {
            return Err(AppError::validation(format!(
                "Project folder `{}` is stale: thread `{}` advanced past the exported version. Run project_folder_export to refresh the folder.",
                req.slug, manifest.thread_id
            )))
        }
        ProjectSyncState::Conflict if !req.force => {
            return Err(AppError::validation(format!(
                "Project folder `{}` conflicts: both the file and thread `{}` changed since export. Pass force=true to apply the file on top of the current head, or re-export to discard the file edit.",
                req.slug, manifest.thread_id
            )))
        }
        _ => {}
    }

    let dir = std::path::PathBuf::from(&status.folder);
    let (source, expected_lock) = crate::project_mirror::read_project_apply_input(&dir)?
        .ok_or_else(|| {
            AppError::validation(format!(
                "Project folder `{}` lost its model.ecky during apply.",
                req.slug
            ))
        })?;
    let state_before = status.state;

    let preview = Box::pin(handle_macro_preview_render(
        state,
        app,
        MacroReplaceRequest {
            identity: AgentIdentityOverride::default(),
            thread_id: Some(manifest.thread_id.clone()),
            message_id: status.thread_head_message_id.clone(),
            macro_code: source.clone(),
            macro_dialect: None,
            ui_spec: None,
            parameters: None,
            post_processing: None,
            geometry_backend: None,
            source_window: None,
        },
        ctx,
    ))
    .await?;
    if preview.artifact_bundle.component_dependency_lock != expected_lock {
        return Err(AppError::validation(format!(
            "Project folder `{}` preview dependency lock differs from canonical ecky.lock.json; apply refused.",
            req.slug
        )));
    }
    let verification = Box::pin(handle_verify_generated_model(
        state,
        app,
        &preview.thread_id,
        &preview.message_id,
        &preview.artifact_bundle.model_id,
        "",
    ))
    .await?;
    if !verification.result.passed {
        return Err(AppError::validation(format!(
            "Project folder `{}` preview verification failed: {}",
            req.slug, verification.result.summary
        )));
    }
    // File->version direction: commit without the Ecky->file overwrite guard
    // or refresh. The pending external bytes stay untouched; only after the
    // version succeeds do we rebase manifest + binding digest below.
    let commit = Box::pin(handle_commit_preview_version_from_external_source(
        state,
        app,
        VersionSaveRequest {
            identity: AgentIdentityOverride::default(),
            thread_id: Some(preview.thread_id.clone()),
            message_id: Some(preview.message_id.clone()),
            title: req.title,
            version_name: req.version_name,
        },
        ctx,
    ))
    .await?;

    // Rebase the manifest onto the committed version; the file bytes we
    // applied become the new clean baseline.
    let rebased = crate::project_mirror::ProjectManifest {
        message_id: commit.message_id.clone(),
        model_id: Some(commit.model_id.clone()),
        source_digest: crate::project_mirror::source_digest(&source),
        exported_at: now_secs(),
        ..manifest
    };
    crate::project_mirror::write_manifest(&dir, &rebased)?;
    {
        let conn = state.db.lock().await;
        crate::thread_source_binding::index_export(
            &conn,
            &rebased.thread_id,
            &dir,
            &rebased.source_digest,
        )?;
    }

    Ok(ProjectFolderApplyResponse {
        state_before,
        no_op: false,
        thread_id: commit.thread_id,
        message_id: commit.message_id,
        model_id: Some(commit.model_id),
        manifest: rebased,
    })
}

// --- Project folder watcher (filesystem-project-mirror T5) ---

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum ProjectFolderWatchEvent {
    /// A settled external edit was applied and committed.
    Applied {
        slug: String,
        thread_id: String,
        message_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        model_id: Option<String>,
    },
    /// A settled external edit failed to compile/render/commit. Reported
    /// once per file digest; editing the file again retries.
    ApplyFailed { slug: String, error: String },
}

/// Polling watcher state. One instance lives in the app's watcher loop;
/// tests drive `tick` directly.
///
/// Two-tick settle: a changed digest must be seen unchanged on two
/// consecutive ticks before applying, so partial editor writes never reach
/// the compiler. A digest that failed to apply is memoized and skipped until
/// the file changes again, so a broken save does not re-render every tick.
#[derive(Default)]
pub struct ProjectFolderWatcher {
    pending: HashMap<String, String>,
    failed: HashMap<String, String>,
}

impl ProjectFolderWatcher {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn tick(
        &mut self,
        state: &AppState,
        app: &dyn PathResolver,
        ctx: &AgentContext,
    ) -> Vec<ProjectFolderWatchEvent> {
        let mut events = Vec::new();
        let root = configured_projects_root(state);
        let mut folders = std::collections::BTreeMap::new();
        if let Ok(slugs) = crate::project_mirror::list_project_slugs(app, root.as_deref()) {
            for slug in slugs {
                if let Ok(dir) = crate::project_mirror::project_dir(app, root.as_deref(), &slug) {
                    folders.insert(slug, dir);
                }
            }
        }
        let bindings = {
            let conn = state.db.lock().await;
            crate::thread_source_binding::list_bindings(&conn).unwrap_or_default()
        };
        for binding in bindings {
            let dir = std::path::PathBuf::from(binding.folder_path);
            if let Some(slug) = dir
                .file_name()
                .and_then(|value| value.to_str())
                .map(str::to_string)
            {
                // Stored binding wins over a same-name folder under the
                // currently configured root.
                folders.insert(slug, dir);
            }
        }
        for (slug, dir) in folders {
            let Ok(Some(manifest)) = crate::project_mirror::read_manifest(&dir) else {
                continue;
            };
            let Ok(Some(source)) = crate::project_mirror::read_project_source(&dir) else {
                continue;
            };
            let digest = crate::project_mirror::source_digest(&source);
            if digest == manifest.source_digest {
                self.pending.remove(&slug);
                self.failed.remove(&slug);
                continue;
            }
            if self.failed.get(&slug) == Some(&digest) {
                continue;
            }
            // Two-tick settle before touching the compiler.
            if self.pending.insert(slug.clone(), digest.clone()) != Some(digest.clone()) {
                continue;
            }

            let applied = handle_project_folder_apply(
                state,
                app,
                ProjectFolderApplyRequest {
                    identity: AgentIdentityOverride::default(),
                    slug: slug.clone(),
                    force: false,
                    title: None,
                    version_name: Some("folder-sync".to_string()),
                },
                ctx,
            )
            .await;
            self.pending.remove(&slug);
            match applied {
                Ok(response) => {
                    self.failed.remove(&slug);
                    events.push(ProjectFolderWatchEvent::Applied {
                        slug,
                        thread_id: response.thread_id,
                        message_id: response.message_id,
                        model_id: response.model_id,
                    });
                }
                Err(err) => {
                    // Stale/conflict folders are not auto-resolved; like a
                    // compile failure they are reported once per digest.
                    self.failed.insert(slug.clone(), digest);
                    events.push(ProjectFolderWatchEvent::ApplyFailed {
                        slug,
                        error: err.to_string(),
                    });
                }
            }
        }
        events
    }
}

pub fn project_folder_watcher_context() -> AgentContext {
    AgentContext {
        session_id: "project-folder-watcher".to_string(),
        client_kind: "watcher".to_string(),
        host_label: "Ecky".to_string(),
        agent_label: "folder-sync".to_string(),
        llm_model_id: None,
        llm_model_label: None,
    }
}
