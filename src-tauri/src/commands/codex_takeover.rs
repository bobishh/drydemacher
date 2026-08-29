use std::time::{SystemTime, UNIX_EPOCH};

use tauri::{Emitter, State};

use crate::contracts::{
    AppError, AppResult, CodexDialogueMessage, CodexMessagePage, CodexMessagePageInput,
    CodexPromptInput, CodexSteerInput, CodexStopInput, CodexTakeoverBinding, CodexTakeoverSnapshot,
};
use crate::models::AppState;
use crate::services::codex_takeover;

static CODEX_BINDING_CREATE_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
static CODEX_HISTORY_BACKFILL_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
const CODEX_QUEUE_POLL_INTERVAL: std::time::Duration = std::time::Duration::from_secs(1);
const CODEX_QUEUE_RETRY_DELAY_SECONDS: i64 = 3;

fn now_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

fn record_queue_delivery_error(
    conn: &rusqlite::Connection,
    queue_id: &str,
    error: &AppError,
) -> AppResult<()> {
    let text = codex_takeover::error_text(error);
    if codex_takeover::is_retryable_delivery_error(&text) {
        codex_takeover::defer_queue_item(
            conn,
            queue_id,
            &text,
            now_seconds() + CODEX_QUEUE_RETRY_DELAY_SECONDS,
        )
    } else {
        codex_takeover::fail_queue_item(conn, queue_id, &text, now_seconds())
    }
}

fn require_mcp_endpoint(state: &AppState) -> AppResult<String> {
    let status = state.mcp_status();
    if status.running && !status.endpoint_url.trim().is_empty() {
        return Ok(status.endpoint_url);
    }
    Err(AppError::provider(
        status.last_startup_error.unwrap_or_else(|| {
            format!(
                "Ecky MCP endpoint {} is not running; Codex provider conversation cannot resume without CAD tools.",
                status.endpoint_url
            )
        }),
    ))
}

fn require_codex_provider_mode(state: &AppState) -> AppResult<()> {
    let configured = state.config.lock().unwrap().connection_type.clone();
    if configured.as_deref() == Some("provider:codex") {
        return Ok(());
    }
    Err(AppError::validation(format!(
        "Codex provider send requires Settings connectionType provider:codex; current value is {}.",
        configured.as_deref().unwrap_or("unset")
    )))
}

fn configured_codex_model(state: &AppState) -> Option<String> {
    let model = state
        .config
        .lock()
        .unwrap()
        .provider_models
        .codex
        .trim()
        .to_string();
    (!model.is_empty()).then_some(model)
}

async fn project_title(state: &AppState, ecky_thread_id: &str) -> AppResult<String> {
    let conn = state.db.lock().await;
    crate::db::get_thread_title(&conn, ecky_thread_id)
        .map_err(|error| AppError::persistence(error.to_string()))?
        .ok_or_else(|| AppError::not_found(format!("Ecky thread {ecky_thread_id} was not found.")))
}

async fn canonical_handoff(state: &AppState, ecky_thread_id: &str) -> AppResult<String> {
    let conn = state.db.lock().await;
    let context =
        crate::context::assemble_context(&conn, Some(ecky_thread_id.to_string()), None, None);
    Ok(format!(
        "THREAD SUMMARY\n{}\n\nRECENT DIALOGUE\n{}\n\nDESIGN DIGEST\n{}\n\nARTIFACT DIGEST\n{}",
        if context.summary.trim().is_empty() {
            "[none]"
        } else {
            &context.summary
        },
        if context.recent_dialogue.trim().is_empty() {
            "[none]"
        } else {
            &context.recent_dialogue
        },
        if context.design_digest.trim().is_empty() {
            "[none]"
        } else {
            &context.design_digest
        },
        if context.artifact_digest.trim().is_empty() {
            "[none]"
        } else {
            &context.artifact_digest
        },
    ))
}

async fn provider_project_cwd(
    app: &tauri::AppHandle,
    state: &AppState,
    ecky_thread_id: &str,
    title: &str,
) -> AppResult<String> {
    let has_version = {
        let conn = state.db.lock().await;
        crate::db::get_thread_latest_version(&conn, ecky_thread_id)
            .map_err(|error| AppError::persistence(error.to_string()))?
            .is_some()
    };
    if has_version {
        let exported = crate::mcp::handlers::handle_project_folder_export(
            state,
            app,
            crate::mcp::handlers::ProjectFolderExportRequest {
                identity: crate::mcp::contracts::AgentIdentityOverride::default(),
                thread_id: Some(ecky_thread_id.to_string()),
                message_id: None,
                slug: None,
            },
            &crate::mcp::handlers::AgentContext {
                session_id: format!("codex-provider:{ecky_thread_id}"),
                client_kind: "provider".to_string(),
                host_label: "Ecky".to_string(),
                agent_label: "Codex".to_string(),
                llm_model_id: None,
                llm_model_label: None,
            },
        )
        .await?;
        return Ok(exported.folder);
    }

    // A blank Ecky thread has no source to mirror yet. Give Codex a stable,
    // isolated workspace; the first committed version will replace this with
    // the canonical thread-source binding on the next resume.
    let configured_root = state.config.lock().unwrap().projects_root.clone();
    let slug = crate::project_mirror::project_slug(title, ecky_thread_id);
    let path = crate::project_mirror::project_dir(app, configured_root.as_deref(), &slug)?;
    std::fs::create_dir_all(&path).map_err(|error| {
        AppError::persistence(format!(
            "Failed to create Ecky provider workspace '{}': {error}",
            path.display()
        ))
    })?;
    Ok(path.to_string_lossy().into_owned())
}

async fn binding_for(state: &AppState, ecky_thread_id: &str) -> AppResult<CodexTakeoverBinding> {
    let conn = state.db.lock().await;
    codex_takeover::get_binding(&conn, ecky_thread_id)?.ok_or_else(|| {
        AppError::not_found(format!(
            "Ecky thread {ecky_thread_id} has no owned Codex conversation."
        ))
    })
}

async fn snapshot_for(
    state: &AppState,
    binding: CodexTakeoverBinding,
    cursor: Option<String>,
) -> AppResult<CodexTakeoverSnapshot> {
    let runtime = state
        .codex_app_server
        .runtime(&binding.codex_thread_id)
        .await;
    let live_messages = state
        .codex_app_server
        .live_messages(&binding.codex_thread_id)
        .await;
    let turn_traces = state
        .codex_app_server
        .turn_traces(&binding.codex_thread_id)
        .await;
    let queue = {
        let conn = state.db.lock().await;
        let page = codex_takeover::provider_message_page(
            &conn,
            &binding.ecky_thread_id,
            codex_takeover::CODEX_PROVIDER_ID,
            cursor.as_deref(),
        )?;
        let title = crate::db::get_thread_title(&conn, &binding.ecky_thread_id)
            .map_err(|error| AppError::persistence(error.to_string()))?
            .unwrap_or_else(|| binding.label.clone());
        let ecky_messages =
            crate::db::get_thread_messages_for_context(&conn, &binding.ecky_thread_id)
                .unwrap_or_default();
        let canonical = crate::context::build_thread_summary(&title, &ecky_messages);
        let handoff = codex_takeover::build_provider_handoff_summary(&canonical, &page.messages);
        crate::db::update_thread_summary(&conn, &binding.ecky_thread_id, &handoff)
            .map_err(|error| AppError::persistence(error.to_string()))?;
        (
            page,
            codex_takeover::list_queue(&conn, &binding.ecky_thread_id)?,
        )
    };
    let (page, queue) = queue;
    Ok(CodexTakeoverSnapshot {
        binding,
        messages: page.messages,
        live_messages,
        turn_traces,
        next_cursor: page.next_cursor,
        backwards_cursor: page.backwards_cursor,
        runtime,
        queue,
    })
}

async fn queued_snapshot_for(
    state: &AppState,
    binding: CodexTakeoverBinding,
) -> AppResult<CodexTakeoverSnapshot> {
    snapshot_for(state, binding, None).await
}

async fn persist_codex_messages(
    state: &AppState,
    binding: &CodexTakeoverBinding,
    messages: &[CodexDialogueMessage],
) -> AppResult<usize> {
    let conn = state.db.lock().await;
    let persisted = codex_takeover::persist_finished_provider_messages(
        &conn,
        &binding.ecky_thread_id,
        codex_takeover::CODEX_PROVIDER_ID,
        &binding.codex_thread_id,
        messages,
    )?;
    if persisted > 0 {
        let title = crate::db::get_thread_title(&conn, &binding.ecky_thread_id)
            .map_err(|error| AppError::persistence(error.to_string()))?
            .unwrap_or_else(|| binding.label.clone());
        let ecky_messages =
            crate::db::get_thread_messages_for_context(&conn, &binding.ecky_thread_id)
                .unwrap_or_default();
        let canonical = crate::context::build_thread_summary(&title, &ecky_messages);
        let local = codex_takeover::list_provider_messages(
            &conn,
            &binding.ecky_thread_id,
            codex_takeover::CODEX_PROVIDER_ID,
            30,
        )?;
        let handoff = codex_takeover::build_provider_handoff_summary(&canonical, &local);
        crate::db::update_thread_summary(&conn, &binding.ecky_thread_id, &handoff)
            .map_err(|error| AppError::persistence(error.to_string()))?;
    }
    Ok(persisted)
}

async fn persist_latest_codex_history(
    state: &AppState,
    binding: &CodexTakeoverBinding,
) -> AppResult<usize> {
    let provider_page = state
        .codex_app_server
        .message_page(&binding.codex_thread_id, None, None)
        .await?;
    persist_codex_messages(state, binding, &provider_page.messages).await
}

async fn resume_binding(
    state: &AppState,
    binding: &CodexTakeoverBinding,
    force_writer_activation: bool,
) -> AppResult<()> {
    let endpoint = require_mcp_endpoint(state)?;
    let title = project_title(state, &binding.ecky_thread_id).await?;
    let handoff = canonical_handoff(state, &binding.ecky_thread_id).await?;
    let refresh_developer_instructions =
        binding.bootstrap_version < codex_takeover::CODEX_BOOTSTRAP_VERSION;
    state
        .codex_app_server
        .resume_thread(
            binding,
            &title,
            &endpoint,
            &handoff,
            refresh_developer_instructions,
            force_writer_activation,
            configured_codex_model(state).as_deref(),
        )
        .await?;
    state
        .codex_app_server
        .name_thread(&binding.codex_thread_id, &title)
        .await?;
    if refresh_developer_instructions {
        let conn = state.db.lock().await;
        codex_takeover::record_bootstrap_version(
            &conn,
            &binding.ecky_thread_id,
            codex_takeover::CODEX_PROVIDER_ID,
            &binding.codex_thread_id,
            codex_takeover::CODEX_BOOTSTRAP_VERSION,
            now_seconds(),
        )?;
    }
    Ok(())
}

pub(crate) async fn activate_bound_writer(state: &AppState, ecky_thread_id: &str) -> AppResult<()> {
    let binding = {
        let conn = state.db.lock().await;
        codex_takeover::get_binding(&conn, ecky_thread_id)?
    };
    if let Some(binding) = binding {
        resume_binding(state, &binding, true).await?;
    }
    Ok(())
}

async fn refresh_binding_workspace(
    app: &tauri::AppHandle,
    state: &AppState,
    binding: CodexTakeoverBinding,
) -> AppResult<CodexTakeoverBinding> {
    let title = project_title(state, &binding.ecky_thread_id).await?;
    let cwd = provider_project_cwd(app, state, &binding.ecky_thread_id, &title).await?;
    if binding.cwd == cwd && binding.label == title {
        return Ok(binding);
    }
    let conn = state.db.lock().await;
    codex_takeover::refresh_binding_metadata(&conn, &binding, &title, &cwd, now_seconds())
}

async fn ensure_binding(
    app: &tauri::AppHandle,
    state: &AppState,
    ecky_thread_id: &str,
) -> AppResult<CodexTakeoverBinding> {
    if let Some(binding) = {
        let conn = state.db.lock().await;
        codex_takeover::get_binding(&conn, ecky_thread_id)?
    } {
        let binding = refresh_binding_workspace(app, state, binding).await?;
        resume_binding(state, &binding, false).await?;
        return binding_for(state, ecky_thread_id).await;
    }

    let _creation = CODEX_BINDING_CREATE_LOCK.lock().await;
    if let Some(binding) = {
        let conn = state.db.lock().await;
        codex_takeover::get_binding(&conn, ecky_thread_id)?
    } {
        let binding = refresh_binding_workspace(app, state, binding).await?;
        resume_binding(state, &binding, false).await?;
        return binding_for(state, ecky_thread_id).await;
    }

    let endpoint = require_mcp_endpoint(state)?;
    let title = project_title(state, ecky_thread_id).await?;
    let cwd = provider_project_cwd(app, state, ecky_thread_id, &title).await?;
    let handoff = canonical_handoff(state, ecky_thread_id).await?;
    let thread = state
        .codex_app_server
        .start_thread(
            ecky_thread_id,
            &title,
            &cwd,
            &endpoint,
            &handoff,
            configured_codex_model(state).as_deref(),
        )
        .await?;
    let binding = {
        let conn = state.db.lock().await;
        codex_takeover::bind_owned_thread(
            &conn,
            ecky_thread_id,
            &thread.id,
            &title,
            &cwd,
            now_seconds(),
        )
    };
    let binding = match binding {
        Ok(binding) => binding,
        Err(error) => {
            let cleanup = state.codex_app_server.delete_thread(&thread.id).await;
            return match cleanup {
                Ok(()) => Err(error),
                Err(cleanup_error) => Err(AppError::with_details(
                    error.code,
                    error.message,
                    format!(
                        "{}\nCreated Codex thread cleanup also failed: {}",
                        error.details.unwrap_or_default(),
                        codex_takeover::error_text(&cleanup_error),
                    ),
                )),
            };
        }
    };
    state
        .codex_app_server
        .name_thread(&binding.codex_thread_id, &title)
        .await?;
    Ok(binding)
}

async fn rotate_binding_after_writer_conflict(
    state: &AppState,
    current: &CodexTakeoverBinding,
) -> AppResult<CodexTakeoverBinding> {
    let _rotation = CODEX_BINDING_CREATE_LOCK.lock().await;
    let saved = binding_for(state, &current.ecky_thread_id).await?;
    if saved.codex_thread_id != current.codex_thread_id {
        return Ok(saved);
    }

    let endpoint = require_mcp_endpoint(state)?;
    let title = project_title(state, &current.ecky_thread_id).await?;
    let handoff = format!(
        "{}\n\nPROVIDER THREAD LINEAGE\nPrevious Codex thread id: {}\nReason: another Codex client still owns that writer. Continue from Ecky durable history above; do not require the previous writer.",
        canonical_handoff(state, &current.ecky_thread_id).await?,
        current.codex_thread_id,
    );
    let thread = state
        .codex_app_server
        .start_thread(
            &current.ecky_thread_id,
            &title,
            &current.cwd,
            &endpoint,
            &handoff,
            configured_codex_model(state).as_deref(),
        )
        .await?;
    let rotated = {
        let conn = state.db.lock().await;
        codex_takeover::rotate_owned_thread(
            &conn,
            current,
            &thread.id,
            "active_writer",
            now_seconds(),
        )
    };
    let rotated = match rotated {
        Ok(binding) => binding,
        Err(error) => {
            let cleanup = state.codex_app_server.delete_thread(&thread.id).await;
            return match cleanup {
                Ok(()) => Err(error),
                Err(cleanup_error) => Err(AppError::with_details(
                    error.code,
                    error.message,
                    format!(
                        "{}\nReplacement Codex thread cleanup also failed: {}",
                        error.details.unwrap_or_default(),
                        codex_takeover::error_text(&cleanup_error),
                    ),
                )),
            };
        }
    };
    state
        .codex_app_server
        .name_thread(&rotated.codex_thread_id, &title)
        .await?;
    Ok(rotated)
}

async fn dispatch_queue_for(state: &AppState, binding: &CodexTakeoverBinding) -> AppResult<()> {
    let mut binding = binding.clone();
    loop {
        binding = binding_for(state, &binding.ecky_thread_id).await?;
        let mut runtime = state
            .codex_app_server
            .runtime(&binding.codex_thread_id)
            .await;
        if runtime.active_turn_id.is_some() {
            runtime = state
                .codex_app_server
                .reconcile_runtime(&binding.codex_thread_id)
                .await?;
        }
        if runtime.phase == "stopping" {
            return Ok(());
        }
        let queue = {
            let conn = state.db.lock().await;
            codex_takeover::list_queue(&conn, &binding.ecky_thread_id)?
        };
        let Some(head) = queue.first().cloned() else {
            return Ok(());
        };
        if head.status == "failed" || head.status == "sending" {
            return Ok(());
        }

        if runtime.active_turn_id.is_some() {
            return Ok(());
        }

        if let Err(error) = persist_latest_codex_history(state, &binding).await {
            state.push_log(format!(
                "[CODEX] read-only history backfill failed before delivery for {}: {}",
                binding.ecky_thread_id,
                codex_takeover::error_text(&error)
            ));
        }
        if let Err(error) = resume_binding(state, &binding, false).await {
            let text = codex_takeover::error_text(&error);
            if codex_takeover::is_active_writer_error(&text) {
                binding = rotate_binding_after_writer_conflict(state, &binding).await?;
                continue;
            }
            let claimed = {
                let conn = state.db.lock().await;
                codex_takeover::claim_queue_item(&conn, &head.id, now_seconds())?
            };
            if claimed {
                let conn = state.db.lock().await;
                record_queue_delivery_error(&conn, &head.id, &error)?;
            }
            return Err(error);
        }
        let reconciled = state
            .codex_app_server
            .runtime(&binding.codex_thread_id)
            .await;
        if reconciled.active_turn_id.is_some() {
            return Ok(());
        }

        let claimed = {
            let conn = state.db.lock().await;
            codex_takeover::claim_queue_item(&conn, &head.id, now_seconds())?
        };
        if !claimed {
            return Ok(());
        }

        match state
            .codex_app_server
            .start_turn(
                &binding.codex_thread_id,
                &head.prompt_text,
                configured_codex_model(state).as_deref(),
            )
            .await
        {
            Ok(turn_id) => {
                let conn = state.db.lock().await;
                codex_takeover::persist_finished_provider_messages(
                    &conn,
                    &binding.ecky_thread_id,
                    codex_takeover::CODEX_PROVIDER_ID,
                    &binding.codex_thread_id,
                    &[CodexDialogueMessage {
                        id: format!("codex:{}:{}:user:0", binding.codex_thread_id, turn_id),
                        role: "user".to_string(),
                        content: head.prompt_text.clone(),
                        status: "success".to_string(),
                        timestamp: head.created_at,
                        provider_event_kind: None,
                    }],
                )?;
                codex_takeover::complete_queue_item(&conn, &head.id)?;
            }
            Err(error) => {
                let text = codex_takeover::error_text(&error);
                let conn = state.db.lock().await;
                if codex_takeover::is_active_writer_error(&text) {
                    codex_takeover::defer_queue_item(&conn, &head.id, &text, now_seconds())?;
                    drop(conn);
                    binding = rotate_binding_after_writer_conflict(state, &binding).await?;
                    continue;
                }
                record_queue_delivery_error(&conn, &head.id, &error)?;
                return Err(error);
            }
        }
        let started = state
            .codex_app_server
            .runtime(&binding.codex_thread_id)
            .await;
        if started.active_turn_id.is_some() {
            return Ok(());
        }
    }
}

pub fn initialize_codex_queue_supervisor(state: AppState, app: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::select! {
                _ = tokio::time::sleep(CODEX_QUEUE_POLL_INTERVAL) => {}
                _ = codex_takeover::wait_for_queue_supervisor() => {}
            }
            if state.config.lock().unwrap().connection_type.as_deref() != Some("provider:codex") {
                continue;
            }
            let bindings = {
                let conn = state.db.lock().await;
                let now = now_seconds();
                if let Err(error) = codex_takeover::recover_retryable_failures(&conn, now) {
                    state.push_log(format!(
                        "[CODEX] prompt queue recovery failed: {}",
                        codex_takeover::error_text(&error)
                    ));
                    continue;
                }
                match codex_takeover::pending_queue_bindings(&conn, now) {
                    Ok(bindings) => bindings,
                    Err(error) => {
                        state.push_log(format!(
                            "[CODEX] prompt queue scan failed: {}",
                            codex_takeover::error_text(&error)
                        ));
                        continue;
                    }
                }
            };
            for binding in bindings {
                let dispatch_state = state.clone();
                let dispatch_app = app.clone();
                tauri::async_runtime::spawn(async move {
                    let result = dispatch_queue_for(&dispatch_state, &binding).await;
                    if let Err(error) = &result {
                        let error_text = codex_takeover::error_text(error);
                        if !codex_takeover::is_retryable_delivery_error(&error_text) {
                            dispatch_state.push_log(format!(
                                "[CODEX] prompt queue dispatch failed for {}: {}",
                                binding.ecky_thread_id, error_text
                            ));
                        }
                    }
                    let _ = dispatch_app.emit(
                        "codex-provider-updated",
                        serde_json::json!({
                            "threadId": binding.codex_thread_id,
                            "method": if result.is_ok() { "queue/dispatched" } else { "queue/failed" },
                        }),
                    );
                });
            }
        }
    });
}

#[tauri::command]
#[specta::specta]
pub async fn get_codex_takeover(
    ecky_thread_id: String,
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> AppResult<Option<CodexTakeoverSnapshot>> {
    let binding = {
        let conn = state.db.lock().await;
        codex_takeover::get_binding(&conn, &ecky_thread_id)?
    };
    match binding {
        Some(binding) => {
            let snapshot = snapshot_for(&state, binding.clone(), None).await?;
            let backfill_state = state.inner().clone();
            tauri::async_runtime::spawn(async move {
                let Ok(_backfill) = CODEX_HISTORY_BACKFILL_LOCK.try_lock() else {
                    return;
                };
                let mut cursor = None;
                loop {
                    let page = match backfill_state
                        .codex_app_server
                        .message_page(&binding.codex_thread_id, cursor.clone(), None)
                        .await
                    {
                        Ok(page) => page,
                        Err(error) => {
                            backfill_state.push_log(format!(
                                "[CODEX] background history backfill failed for {}: {}",
                                binding.ecky_thread_id,
                                codex_takeover::error_text(&error)
                            ));
                            break;
                        }
                    };
                    let next_cursor = page.next_cursor.clone();
                    match persist_codex_messages(&backfill_state, &binding, &page.messages).await {
                        Ok(changed) if changed > 0 => {
                            let _ = app.emit(
                                "codex-provider-updated",
                                serde_json::json!({
                                    "threadId": binding.codex_thread_id,
                                    "method": "history/persisted",
                                }),
                            );
                        }
                        Ok(_) => {}
                        Err(error) => {
                            backfill_state.push_log(format!(
                                "[CODEX] background history persistence failed for {}: {}",
                                binding.ecky_thread_id,
                                codex_takeover::error_text(&error)
                            ));
                            break;
                        }
                    }
                    if next_cursor.is_none() || next_cursor == cursor {
                        break;
                    }
                    cursor = next_cursor;
                }
            });
            Ok(Some(snapshot))
        }
        None => Ok(None),
    }
}

#[tauri::command]
#[specta::specta]
pub async fn get_codex_takeover_messages(
    input: CodexMessagePageInput,
    state: State<'_, AppState>,
) -> AppResult<CodexMessagePage> {
    let conn = state.db.lock().await;
    codex_takeover::provider_message_page(
        &conn,
        &input.ecky_thread_id,
        codex_takeover::CODEX_PROVIDER_ID,
        input.cursor.as_deref(),
    )
}

#[tauri::command]
#[specta::specta]
pub async fn send_codex_takeover_prompt(
    input: CodexPromptInput,
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> AppResult<CodexTakeoverSnapshot> {
    require_codex_provider_mode(&state)?;
    let existing_binding = {
        let conn = state.db.lock().await;
        codex_takeover::get_binding(&conn, &input.ecky_thread_id)?
    };
    let binding = match existing_binding {
        Some(binding) => binding,
        None => ensure_binding(&app, &state, &input.ecky_thread_id).await?,
    };
    {
        let conn = state.db.lock().await;
        codex_takeover::enqueue_prompt(
            &conn,
            &input.ecky_thread_id,
            &input.prompt_text,
            now_seconds(),
        )?;
    }
    codex_takeover::notify_queue_supervisor();
    let snapshot = queued_snapshot_for(&state, binding.clone()).await?;
    let delivery_state = state.inner().clone();
    let delivery_app = app.clone();
    let codex_thread_id = binding.codex_thread_id.clone();
    tauri::async_runtime::spawn(async move {
        let _ = dispatch_queue_for(&delivery_state, &binding).await;
        let _ = delivery_app.emit(
            "codex-provider-updated",
            serde_json::json!({
                "threadId": codex_thread_id,
                "method": "queue/dispatched",
            }),
        );
    });
    Ok(snapshot)
}

#[tauri::command]
#[specta::specta]
pub async fn dispatch_codex_prompt_queue(
    ecky_thread_id: String,
    state: State<'_, AppState>,
) -> AppResult<CodexTakeoverSnapshot> {
    let binding = binding_for(&state, &ecky_thread_id).await?;
    dispatch_queue_for(&state, &binding).await?;
    snapshot_for(&state, binding_for(&state, &ecky_thread_id).await?, None).await
}

#[tauri::command]
#[specta::specta]
pub async fn steer_codex_takeover(
    input: CodexSteerInput,
    state: State<'_, AppState>,
) -> AppResult<CodexTakeoverSnapshot> {
    let binding = binding_for(&state, &input.ecky_thread_id).await?;
    resume_binding(&state, &binding, false).await?;
    let runtime = state
        .codex_app_server
        .runtime(&binding.codex_thread_id)
        .await;
    if runtime.active_turn_id.as_deref() != Some(&input.expected_turn_id) {
        return Err(AppError::conflict(format!(
            "Codex active turn changed; expected {}, current {}.",
            input.expected_turn_id,
            runtime.active_turn_id.as_deref().unwrap_or("none")
        )));
    }
    state
        .codex_app_server
        .steer_turn(
            &binding.codex_thread_id,
            &input.expected_turn_id,
            &input.prompt_text,
        )
        .await?;
    {
        let conn = state.db.lock().await;
        codex_takeover::persist_provider_turn_user_input(
            &conn,
            &binding.ecky_thread_id,
            codex_takeover::CODEX_PROVIDER_ID,
            &binding.codex_thread_id,
            &input.expected_turn_id,
            &input.prompt_text,
            now_seconds(),
        )?;
    }
    snapshot_for(&state, binding, None).await
}

#[tauri::command]
#[specta::specta]
pub async fn stop_codex_takeover(
    input: CodexStopInput,
    state: State<'_, AppState>,
) -> AppResult<CodexTakeoverSnapshot> {
    let binding = binding_for(&state, &input.ecky_thread_id).await?;
    let runtime = state
        .codex_app_server
        .runtime(&binding.codex_thread_id)
        .await;
    if runtime.active_turn_id.as_deref() != Some(&input.turn_id) {
        return Err(AppError::conflict(format!(
            "Codex active turn changed; expected {}, current {}.",
            input.turn_id,
            runtime.active_turn_id.as_deref().unwrap_or("none")
        )));
    }
    state
        .codex_app_server
        .interrupt_turn(&binding.codex_thread_id, &input.turn_id)
        .await?;
    snapshot_for(&state, binding, None).await
}

#[tauri::command]
#[specta::specta]
pub async fn retry_codex_queued_prompt(
    ecky_thread_id: String,
    queue_id: String,
    state: State<'_, AppState>,
) -> AppResult<CodexTakeoverSnapshot> {
    let binding = binding_for(&state, &ecky_thread_id).await?;
    {
        let conn = state.db.lock().await;
        codex_takeover::retry_queue_item(&conn, &ecky_thread_id, &queue_id, now_seconds())?;
    }
    dispatch_queue_for(&state, &binding).await?;
    snapshot_for(&state, binding_for(&state, &ecky_thread_id).await?, None).await
}

#[tauri::command]
#[specta::specta]
pub async fn remove_codex_queued_prompt(
    ecky_thread_id: String,
    queue_id: String,
    state: State<'_, AppState>,
) -> AppResult<CodexTakeoverSnapshot> {
    let binding = binding_for(&state, &ecky_thread_id).await?;
    {
        let conn = state.db.lock().await;
        codex_takeover::remove_queue_item(&conn, &ecky_thread_id, &queue_id)?;
    }
    snapshot_for(&state, binding, None).await
}
