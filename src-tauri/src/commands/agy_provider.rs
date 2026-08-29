use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use tauri::{Emitter, State};

use crate::contracts::{
    AgyMessagePage, AgyMessagePageInput, AgyPromptInput, AgyProviderBinding, AgyProviderSnapshot,
    AgyStopInput, AppError, AppResult, ProviderCapabilities,
};
use crate::models::AppState;
use crate::services::{agy_provider, codex_takeover};

static AGY_BINDING_CREATE_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
static AGY_QUEUE_WAKE: tokio::sync::Notify = tokio::sync::Notify::const_new();
const AGY_QUEUE_POLL_INTERVAL: std::time::Duration = std::time::Duration::from_secs(1);

fn now_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

fn require_agy_provider_mode(state: &AppState) -> AppResult<()> {
    let configured = state.config.lock().unwrap().connection_type.clone();
    if configured.as_deref() == Some("provider:agy") {
        return Ok(());
    }
    Err(AppError::validation(format!(
        "Agy provider send requires Settings connectionType provider:agy; current value is {}.",
        configured.as_deref().unwrap_or("unset")
    )))
}

fn configured_agy_model(state: &AppState) -> Option<String> {
    let model = state
        .config
        .lock()
        .unwrap()
        .provider_models
        .agy
        .trim()
        .to_string();
    (!model.is_empty()).then_some(model)
}

fn require_mcp_endpoint(state: &AppState) -> AppResult<String> {
    let status = state.mcp_status();
    if status.running && !status.endpoint_url.trim().is_empty() {
        return Ok(status.endpoint_url);
    }
    Err(AppError::provider(status.last_startup_error.unwrap_or_else(|| {
        format!(
            "Ecky MCP endpoint {} is not running; Agy provider conversation cannot start without CAD tools.",
            status.endpoint_url
        )
    })))
}

async fn project_title(state: &AppState, ecky_thread_id: &str) -> AppResult<String> {
    let conn = state.db.lock().await;
    crate::db::get_thread_title(&conn, ecky_thread_id)
        .map_err(|error| AppError::persistence(error.to_string()))?
        .ok_or_else(|| AppError::not_found(format!("Ecky thread {ecky_thread_id} was not found.")))
}

async fn canonical_handoff(state: &AppState, ecky_thread_id: &str) -> String {
    let conn = state.db.lock().await;
    let context =
        crate::context::assemble_context(&conn, Some(ecky_thread_id.to_string()), None, None);
    format!(
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
    )
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
                session_id: format!("agy-provider:{ecky_thread_id}"),
                client_kind: "provider".to_string(),
                host_label: "Ecky".to_string(),
                agent_label: "Agy".to_string(),
                llm_model_id: None,
                llm_model_label: None,
            },
        )
        .await?;
        return Ok(exported.folder);
    }
    let configured_root = state.config.lock().unwrap().projects_root.clone();
    let slug = crate::project_mirror::project_slug(title, ecky_thread_id);
    let path = crate::project_mirror::project_dir(app, configured_root.as_deref(), &slug)?;
    std::fs::create_dir_all(&path).map_err(|error| {
        AppError::persistence(format!(
            "Failed to create Ecky Agy workspace '{}': {error}",
            path.display()
        ))
    })?;
    Ok(path.to_string_lossy().into_owned())
}

const AGY_PROVIDER_TOOL_GUIDE: &str = r#"# Ecky provider tool guide

- Provider target is already pre-bound by Ecky. Do not call `thread_borrow` for the assigned thread. Use it only when the user explicitly asks to switch to another existing Ecky thread.
- First call `workspace_overview`. Confirm `defaultTarget.threadId` matches the assigned thread.
- Before editing, read `agentBrief.primaryGuideUri` and every URI in `agentBrief.mustRead` through MCP resources. Use `capability_search` and `capability_enable` before guessing specialist tool names.
- When `defaultTarget.sourcePath` exists, inspect and edit that exact file. The watcher appends, validates, and previews settled changes. Do not export first or call a manual commit/finalize operation.
- Follow inspect -> validate -> preview -> verify. Prefer MCP/normal file tools. Browser work is only for explicit web or UI requests.
- Stop after a bounded repair attempt. Surface exact diagnostics instead of repeating the same tool/action loop.
"#;

#[derive(Debug)]
struct AgyWorkspaceMaterialization {
    config_path: String,
    guide_path: String,
    bound_endpoint: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AgyPromptPhase {
    Bootstrap,
    Resume,
    Continuation,
}

fn materialize_agy_mcp_config(
    cwd: &str,
    endpoint: &str,
    ecky_thread_id: &str,
) -> AppResult<AgyWorkspaceMaterialization> {
    let agents_dir = Path::new(cwd).join(".agents");
    std::fs::create_dir_all(&agents_dir).map_err(|error| {
        AppError::persistence(format!(
            "Failed to create Agy workspace config directory '{}': {error}",
            agents_dir.display()
        ))
    })?;
    let plugin_dir = agents_dir.join("plugins").join("ecky-provider");
    let rules_dir = plugin_dir.join("rules");
    std::fs::create_dir_all(&rules_dir).map_err(|error| {
        AppError::persistence(format!(
            "Failed to create Agy provider plugin directory '{}': {error}",
            rules_dir.display()
        ))
    })?;
    let path = plugin_dir.join("mcp_config.json");
    let manifest_path = plugin_dir.join("plugin.json");
    let guide_path = rules_dir.join("AGENTS.md");
    let bound_endpoint = crate::mcp::server::provider_bound_endpoint(endpoint, ecky_thread_id);
    let mut root = if path.exists() {
        let raw = std::fs::read_to_string(&path).map_err(|error| {
            AppError::persistence(format!("Failed to read '{}': {error}", path.display()))
        })?;
        serde_json::from_str::<serde_json::Value>(&raw).map_err(|error| {
            AppError::validation(format!(
                "Existing Agy MCP config '{}' is invalid JSON: {error}",
                path.display()
            ))
        })?
    } else {
        serde_json::json!({})
    };
    let root_object = root.as_object_mut().ok_or_else(|| {
        AppError::validation(format!(
            "Existing Agy MCP config '{}' must contain a JSON object.",
            path.display()
        ))
    })?;
    let servers = root_object
        .entry("mcpServers")
        .or_insert_with(|| serde_json::json!({}))
        .as_object_mut()
        .ok_or_else(|| AppError::validation("Agy mcpServers must be a JSON object."))?;
    servers.insert(
        "ecky_mcp".to_string(),
        serde_json::json!({ "serverUrl": bound_endpoint }),
    );
    let encoded = serde_json::to_string_pretty(&root).map_err(|error| {
        AppError::persistence(format!("Failed to encode Agy MCP config: {error}"))
    })?;
    std::fs::write(&path, format!("{encoded}\n")).map_err(|error| {
        AppError::persistence(format!("Failed to write '{}': {error}", path.display()))
    })?;
    std::fs::write(
        &manifest_path,
        "{\n  \"name\": \"ecky-provider\",\n  \"disabled\": false\n}\n",
    )
    .map_err(|error| {
        AppError::persistence(format!(
            "Failed to write '{}': {error}",
            manifest_path.display()
        ))
    })?;
    std::fs::write(&guide_path, AGY_PROVIDER_TOOL_GUIDE).map_err(|error| {
        AppError::persistence(format!(
            "Failed to write '{}': {error}",
            guide_path.display()
        ))
    })?;
    Ok(AgyWorkspaceMaterialization {
        config_path: path.to_string_lossy().into_owned(),
        guide_path: guide_path.to_string_lossy().into_owned(),
        bound_endpoint,
    })
}

fn provider_prompt(
    phase: AgyPromptPhase,
    ecky_thread_id: &str,
    title: &str,
    cwd: &str,
    endpoint: &str,
    mcp_config_path: &str,
    tool_guide_path: &str,
    handoff: &str,
    prompt: &str,
) -> String {
    if phase == AgyPromptPhase::Continuation {
        return format!(
            "[ECKY USER TURN v{}]\nContinue the already pre-bound Ecky provider conversation. Do not call `thread_borrow`. Answer this user message directly.\n\n[USER MESSAGE]\n{prompt}",
            agy_provider::AGY_BOOTSTRAP_VERSION,
        );
    }
    let phase_label = match phase {
        AgyPromptPhase::Bootstrap => "THREAD BOOTSTRAP",
        AgyPromptPhase::Resume => "CONTEXT REFRESH",
        AgyPromptPhase::Continuation => unreachable!(),
    };
    format!(
        "[ECKY {phase_label} v{}]\nYou are Agy inside Ecky CAD. This provider conversation belongs only to Ecky thread {ecky_thread_id} ({title}).\nCanonical workspace: {cwd}\nWorkspace MCP config: {mcp_config_path}\nRequired MCP endpoint: {endpoint} under ecky_mcp. The workspace plugin overrides any same-named global server for this project.\nThis MCP connection is already pre-bound to thread {ecky_thread_id}. Do not call `thread_borrow`; it is only for an intentional switch to another existing target.\nRead the provider tool guide first: {tool_guide_path}. Then call `workspace_overview`, verify its target, and read `agentBrief.primaryGuideUri` plus every URI in `agentBrief.mustRead` before editing.\nUse MCP inspect -> validate -> preview -> verify for CAD changes; the bound file watcher creates the version, so do not call a manual commit/finalize operation. Never invent thread ids or import foreign conversations. Treat the context below as canonical across API/MCP/Codex/Agy switching.\nWhen useful, cite the bound source in the user-facing answer as `[model.ecky]({cwd}/model.ecky:LINE)` so Ecky can open the exact line. Do not include internal `messageId` or `modelId` fields in the user-facing answer; keep those identifiers only in internal tool evidence.\n\n{handoff}\n\n[USER MESSAGE]\n{prompt}",
        agy_provider::AGY_BOOTSTRAP_VERSION,
    )
}

async fn binding_for(state: &AppState, ecky_thread_id: &str) -> AppResult<AgyProviderBinding> {
    let conn = state.db.lock().await;
    agy_provider::get_binding(&conn, ecky_thread_id)?.ok_or_else(|| {
        AppError::not_found(format!(
            "Ecky thread {ecky_thread_id} has no owned Agy conversation."
        ))
    })
}

pub(crate) async fn activate_bound_writer(
    _state: &AppState,
    _ecky_thread_id: &str,
) -> AppResult<()> {
    // Agy `--conversation` is an active resume, not a passive subscription.
    // Writer acquisition therefore happens only while delivering a claimed prompt.
    Ok(())
}

async fn snapshot_for(
    state: &AppState,
    binding: AgyProviderBinding,
    cursor: Option<&str>,
) -> AppResult<AgyProviderSnapshot> {
    let (page, queue) = {
        let conn = state.db.lock().await;
        (
            agy_provider::message_page(&conn, &binding.ecky_thread_id, cursor)?,
            agy_provider::list_queue(&conn, &binding.ecky_thread_id)?,
        )
    };
    let mut runtime = state
        .agy_provider
        .runtime(&binding.agy_conversation_id)
        .await;
    if runtime.error.is_none() {
        runtime.error = queue
            .first()
            .filter(|item| item.status == "failed")
            .and_then(|item| item.error.clone());
    }
    Ok(AgyProviderSnapshot {
        messages: page.messages,
        next_cursor: page.next_cursor,
        backwards_cursor: page.backwards_cursor,
        runtime,
        live_messages: state
            .agy_provider
            .live_messages(&binding.agy_conversation_id)
            .await,
        turn_traces: state
            .agy_provider
            .turn_traces(&binding.agy_conversation_id)
            .await,
        queue,
        binding,
        capabilities: ProviderCapabilities {
            steer: false,
            stop: true,
        },
    })
}

async fn emit_provider_update(app: &tauri::AppHandle, binding: &AgyProviderBinding, method: &str) {
    let _ = app.emit(
        "agy-provider-updated",
        serde_json::json!({
            "conversationId": binding.agy_conversation_id,
            "method": method,
        }),
    );
}

fn spawn_turn_finalizer(
    state: AppState,
    app: tauri::AppHandle,
    binding: AgyProviderBinding,
    queue_id: String,
    result: tokio::sync::oneshot::Receiver<AppResult<agy_provider::AgyTurnResult>>,
) {
    tauri::async_runtime::spawn(async move {
        let outcome = match result.await {
            Ok(result) => result,
            Err(_) => Err(AppError::provider(
                "Agy turn result channel closed unexpectedly.",
            )),
        };
        {
            let conn = state.db.lock().await;
            match outcome {
                Ok(result) if result.status == "SUCCESS" => {
                    if !result.response.trim().is_empty() {
                        let _ = agy_provider::insert_message(
                            &conn,
                            &binding.ecky_thread_id,
                            &binding.agy_conversation_id,
                            "assistant",
                            &result.response,
                            "success",
                            now_seconds(),
                        );
                    }
                    let _ = codex_takeover::complete_queue_item(&conn, &queue_id);
                    let title = crate::db::get_thread_title(&conn, &binding.ecky_thread_id)
                        .ok()
                        .flatten()
                        .unwrap_or_else(|| binding.label.clone());
                    let ecky_messages =
                        crate::db::get_thread_messages_for_context(&conn, &binding.ecky_thread_id)
                            .unwrap_or_default();
                    let canonical = crate::context::build_thread_summary(&title, &ecky_messages);
                    if let Ok(page) =
                        agy_provider::message_page(&conn, &binding.ecky_thread_id, None)
                    {
                        let handoff = codex_takeover::build_provider_handoff_summary_for(
                            "AGY",
                            &canonical,
                            &page.messages,
                        );
                        let _ = crate::db::update_thread_summary(
                            &conn,
                            &binding.ecky_thread_id,
                            &handoff,
                        );
                    }
                }
                Ok(result) if matches!(result.status.as_str(), "CANCELED" | "INTERRUPTED") => {
                    let _ = codex_takeover::complete_queue_item(&conn, &queue_id);
                }
                Ok(result) => {
                    let raw = result.error.unwrap_or(result.status);
                    let _ = codex_takeover::fail_queue_item(&conn, &queue_id, &raw, now_seconds());
                }
                Err(error) => {
                    let raw = codex_takeover::error_text(&error);
                    let _ = codex_takeover::fail_queue_item(&conn, &queue_id, &raw, now_seconds());
                }
            }
        }
        emit_provider_update(&app, &binding, "turn/terminal").await;
        let _ = dispatch_queue_for(&app, &state, &binding).await;
        emit_provider_update(&app, &binding, "queue/dispatched").await;
        AGY_QUEUE_WAKE.notify_one();
    });
}

async fn dispatch_queue_for(
    app: &tauri::AppHandle,
    state: &AppState,
    binding: &AgyProviderBinding,
) -> AppResult<()> {
    let runtime = state
        .agy_provider
        .runtime(&binding.agy_conversation_id)
        .await;
    if runtime.active_turn_id.is_some() || runtime.phase == "stopping" {
        return Ok(());
    }
    let head = {
        let conn = state.db.lock().await;
        agy_provider::queue_head(&conn, &binding.ecky_thread_id)?
    };
    let Some(head) = head else { return Ok(()) };
    if head.status != "queued" {
        return Ok(());
    }
    let endpoint = require_mcp_endpoint(state)?;
    let requested_bound_endpoint =
        crate::mcp::server::provider_bound_endpoint(&endpoint, &binding.ecky_thread_id);
    let warm_session = state
        .agy_provider
        .has_compatible_session(
            &binding.agy_conversation_id,
            configured_agy_model(state).as_deref(),
            Some(&requested_bound_endpoint),
        )
        .await;
    let prompt_phase = if warm_session {
        AgyPromptPhase::Continuation
    } else {
        AgyPromptPhase::Resume
    };
    let handoff = if warm_session {
        String::new()
    } else {
        canonical_handoff(state, &binding.ecky_thread_id).await
    };
    let workspace = materialize_agy_mcp_config(&binding.cwd, &endpoint, &binding.ecky_thread_id)?;
    let prompt = provider_prompt(
        prompt_phase,
        &binding.ecky_thread_id,
        &binding.label,
        &binding.cwd,
        &workspace.bound_endpoint,
        &workspace.config_path,
        &workspace.guide_path,
        &handoff,
        &head.prompt_text,
    );
    let claimed = {
        let conn = state.db.lock().await;
        codex_takeover::claim_queue_item(&conn, &head.id, now_seconds())?
    };
    if !claimed {
        return Ok(());
    }
    let started = match state
        .agy_provider
        .start_turn(
            &binding.agy_conversation_id,
            &binding.cwd,
            &prompt,
            configured_agy_model(state).as_deref(),
            Some(&workspace.bound_endpoint),
        )
        .await
    {
        Ok(started) => started,
        Err(error) => {
            let conn = state.db.lock().await;
            codex_takeover::fail_queue_item(
                &conn,
                &head.id,
                &codex_takeover::error_text(&error),
                now_seconds(),
            )?;
            return Err(error);
        }
    };
    let persistence_result = {
        let conn = state.db.lock().await;
        (|| -> AppResult<()> {
            agy_provider::record_process_lease(
                &conn,
                &head.id,
                &started.conversation_id,
                &started.process,
                now_seconds(),
            )?;
            agy_provider::insert_message_with_id(
                &conn,
                &format!("agy:user:{}", head.id),
                &binding.ecky_thread_id,
                &binding.agy_conversation_id,
                "user",
                &head.prompt_text,
                "success",
                head.created_at,
            )?;
            Ok(())
        })()
    };
    if let Err(error) = persistence_result {
        let _ = state
            .agy_provider
            .stop_turn(&started.conversation_id, &started.turn_id)
            .await;
        let conn = state.db.lock().await;
        codex_takeover::fail_queue_item(
            &conn,
            &head.id,
            &codex_takeover::error_text(&error),
            now_seconds(),
        )?;
        return Err(error);
    }
    spawn_turn_finalizer(
        state.clone(),
        app.clone(),
        binding.clone(),
        head.id,
        started.result,
    );
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn get_agy_provider(
    ecky_thread_id: String,
    state: State<'_, AppState>,
) -> AppResult<Option<AgyProviderSnapshot>> {
    let binding = {
        let conn = state.db.lock().await;
        agy_provider::get_binding(&conn, &ecky_thread_id)?
    };
    match binding {
        Some(binding) => snapshot_for(&state, binding, None).await.map(Some),
        None => Ok(None),
    }
}

#[tauri::command]
#[specta::specta]
pub async fn get_agy_provider_messages(
    input: AgyMessagePageInput,
    state: State<'_, AppState>,
) -> AppResult<AgyMessagePage> {
    let conn = state.db.lock().await;
    agy_provider::message_page(&conn, &input.ecky_thread_id, input.cursor.as_deref())
}

#[tauri::command]
#[specta::specta]
pub async fn send_agy_provider_prompt(
    input: AgyPromptInput,
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> AppResult<AgyProviderSnapshot> {
    require_agy_provider_mode(&state)?;
    if let Some(binding) = {
        let conn = state.db.lock().await;
        agy_provider::get_binding(&conn, &input.ecky_thread_id)?
    } {
        {
            let conn = state.db.lock().await;
            agy_provider::enqueue_prompt(
                &conn,
                &input.ecky_thread_id,
                &input.prompt_text,
                now_seconds(),
            )?;
        }
        AGY_QUEUE_WAKE.notify_one();
        let dispatch_state = state.inner().clone();
        let dispatch_app = app.clone();
        let dispatch_binding = binding.clone();
        tauri::async_runtime::spawn(async move {
            let _ = dispatch_queue_for(&dispatch_app, &dispatch_state, &dispatch_binding).await;
            emit_provider_update(&dispatch_app, &dispatch_binding, "queue/dispatched").await;
        });
        return snapshot_for(&state, binding, None).await;
    }

    let _creation = AGY_BINDING_CREATE_LOCK.lock().await;
    if let Some(binding) = {
        let conn = state.db.lock().await;
        agy_provider::get_binding(&conn, &input.ecky_thread_id)?
    } {
        drop(_creation);
        return send_existing(input, app, state, binding).await;
    }
    let endpoint = require_mcp_endpoint(&state)?;
    let title = project_title(&state, &input.ecky_thread_id).await?;
    let cwd = provider_project_cwd(&app, &state, &input.ecky_thread_id, &title).await?;
    let handoff = canonical_handoff(&state, &input.ecky_thread_id).await;
    let workspace = materialize_agy_mcp_config(&cwd, &endpoint, &input.ecky_thread_id)?;
    let prompt = provider_prompt(
        AgyPromptPhase::Bootstrap,
        &input.ecky_thread_id,
        &title,
        &cwd,
        &workspace.bound_endpoint,
        &workspace.config_path,
        &workspace.guide_path,
        &handoff,
        &input.prompt_text,
    );
    let started = state
        .agy_provider
        .start_new_turn(
            &cwd,
            &prompt,
            configured_agy_model(&state).as_deref(),
            Some(&workspace.bound_endpoint),
        )
        .await?;
    let binding = {
        let conn = state.db.lock().await;
        match agy_provider::bind_owned_conversation(
            &conn,
            &input.ecky_thread_id,
            &started.conversation_id,
            &title,
            &cwd,
            now_seconds(),
        ) {
            Ok(binding) => binding,
            Err(error) => {
                drop(conn);
                let _ = state
                    .agy_provider
                    .stop_turn(&started.conversation_id, &started.turn_id)
                    .await;
                return Err(error);
            }
        }
    };
    let queue = {
        let conn = state.db.lock().await;
        let item = agy_provider::enqueue_prompt(
            &conn,
            &input.ecky_thread_id,
            &input.prompt_text,
            now_seconds(),
        )?;
        codex_takeover::claim_queue_item(&conn, &item.id, now_seconds())?;
        item
    };
    let persistence_result = {
        let conn = state.db.lock().await;
        (|| -> AppResult<()> {
            agy_provider::record_process_lease(
                &conn,
                &queue.id,
                &started.conversation_id,
                &started.process,
                now_seconds(),
            )?;
            agy_provider::insert_message_with_id(
                &conn,
                &format!("agy:user:{}", queue.id),
                &binding.ecky_thread_id,
                &binding.agy_conversation_id,
                "user",
                &input.prompt_text,
                "success",
                queue.created_at,
            )?;
            Ok(())
        })()
    };
    if let Err(error) = persistence_result {
        let _ = state
            .agy_provider
            .stop_turn(&started.conversation_id, &started.turn_id)
            .await;
        let conn = state.db.lock().await;
        codex_takeover::fail_queue_item(
            &conn,
            &queue.id,
            &codex_takeover::error_text(&error),
            now_seconds(),
        )?;
        return Err(error);
    }
    spawn_turn_finalizer(
        state.inner().clone(),
        app,
        binding.clone(),
        queue.id,
        started.result,
    );
    snapshot_for(&state, binding, None).await
}

async fn send_existing(
    input: AgyPromptInput,
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    binding: AgyProviderBinding,
) -> AppResult<AgyProviderSnapshot> {
    {
        let conn = state.db.lock().await;
        agy_provider::enqueue_prompt(
            &conn,
            &input.ecky_thread_id,
            &input.prompt_text,
            now_seconds(),
        )?;
    }
    let dispatch_state = state.inner().clone();
    let dispatch_app = app.clone();
    let dispatch_binding = binding.clone();
    tauri::async_runtime::spawn(async move {
        let _ = dispatch_queue_for(&dispatch_app, &dispatch_state, &dispatch_binding).await;
    });
    snapshot_for(&state, binding, None).await
}

#[tauri::command]
#[specta::specta]
pub async fn dispatch_agy_prompt_queue(
    ecky_thread_id: String,
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> AppResult<AgyProviderSnapshot> {
    let binding = binding_for(&state, &ecky_thread_id).await?;
    dispatch_queue_for(&app, &state, &binding).await?;
    snapshot_for(&state, binding, None).await
}

#[tauri::command]
#[specta::specta]
pub async fn stop_agy_provider(
    input: AgyStopInput,
    state: State<'_, AppState>,
) -> AppResult<AgyProviderSnapshot> {
    let binding = binding_for(&state, &input.ecky_thread_id).await?;
    state
        .agy_provider
        .stop_turn(&binding.agy_conversation_id, &input.turn_id)
        .await?;
    snapshot_for(&state, binding, None).await
}

#[tauri::command]
#[specta::specta]
pub async fn retry_agy_queued_prompt(
    ecky_thread_id: String,
    queue_id: String,
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> AppResult<AgyProviderSnapshot> {
    let binding = binding_for(&state, &ecky_thread_id).await?;
    {
        let conn = state.db.lock().await;
        agy_provider::retry_queue_item(&conn, &ecky_thread_id, &queue_id, now_seconds())?;
    }
    dispatch_queue_for(&app, &state, &binding).await?;
    snapshot_for(&state, binding, None).await
}

#[tauri::command]
#[specta::specta]
pub async fn remove_agy_queued_prompt(
    ecky_thread_id: String,
    queue_id: String,
    state: State<'_, AppState>,
) -> AppResult<AgyProviderSnapshot> {
    let binding = binding_for(&state, &ecky_thread_id).await?;
    {
        let conn = state.db.lock().await;
        agy_provider::remove_queue_item(&conn, &ecky_thread_id, &queue_id)?;
    }
    snapshot_for(&state, binding, None).await
}

pub fn initialize_agy_queue_supervisor(state: AppState, app: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::select! {
                _ = tokio::time::sleep(AGY_QUEUE_POLL_INTERVAL) => {}
                _ = AGY_QUEUE_WAKE.notified() => {}
            }
            if state.config.lock().unwrap().connection_type.as_deref() != Some("provider:agy") {
                continue;
            }
            let bindings = {
                let conn = state.db.lock().await;
                match agy_provider::pending_queue_bindings(&conn) {
                    Ok(bindings) => bindings,
                    Err(error) => {
                        state.push_log(format!(
                            "[AGY] prompt queue scan failed: {}",
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
                    let _ = dispatch_queue_for(&dispatch_app, &dispatch_state, &binding).await;
                    emit_provider_update(&dispatch_app, &binding, "queue/dispatched").await;
                });
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::{materialize_agy_mcp_config, provider_prompt, AgyPromptPhase};

    #[test]
    fn provider_prompt_requests_clickable_bound_source_evidence_without_internal_ids() {
        let prompt = provider_prompt(
            AgyPromptPhase::Bootstrap,
            "ecky-thread-1",
            "Dryer",
            "/workspace/dryer",
            "http://127.0.0.1:39249/mcp",
            "/workspace/dryer/.agy/mcp_config.json",
            "/workspace/dryer/.agents/ecky-provider-tools.md",
            "Current target: dryer",
            "Increase capacity.",
        );

        assert!(prompt.contains("[model.ecky](/workspace/dryer/model.ecky:LINE)"));
        assert!(prompt.contains("Do not include internal `messageId` or `modelId`"));
        assert!(prompt.contains("Read the provider tool guide first"));
        assert!(prompt.contains("already pre-bound"));
        assert!(prompt.contains("Do not call `thread_borrow`"));
    }

    #[test]
    fn warm_continuation_sends_user_turn_without_repeating_canonical_handoff() {
        let prompt = provider_prompt(
            AgyPromptPhase::Continuation,
            "ecky-thread-1",
            "Dryer",
            "/workspace/dryer",
            "http://127.0.0.1:39249/mcp?providerThreadId=ecky-thread-1",
            "/workspace/dryer/.agents/plugins/ecky-provider/mcp_config.json",
            "/workspace/dryer/.agents/plugins/ecky-provider/rules/AGENTS.md",
            "THREAD SUMMARY\nlarge canonical handoff",
            "Increase capacity.",
        );

        assert!(prompt.contains("[ECKY USER TURN v2]"));
        assert!(prompt.contains("Increase capacity."));
        assert!(!prompt.contains("large canonical handoff"));
        assert!(!prompt.contains("THREAD BOOTSTRAP"));
    }

    #[test]
    fn workspace_mcp_config_prebinds_exact_thread_and_writes_tool_guide() {
        let directory = std::env::temp_dir().join(format!(
            "ecky-agy-workspace-config-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&directory).unwrap();

        let materialized = materialize_agy_mcp_config(
            directory.to_str().unwrap(),
            "http://127.0.0.1:39249/mcp",
            "ecky-thread-1",
        )
        .unwrap();
        let config = std::fs::read_to_string(&materialized.config_path).unwrap();
        let guide = std::fs::read_to_string(&materialized.guide_path).unwrap();
        let manifest =
            std::fs::read_to_string(directory.join(".agents/plugins/ecky-provider/plugin.json"))
                .unwrap();

        assert!(config.contains("http://127.0.0.1:39249/mcp?providerThreadId=ecky-thread-1"));
        assert!(config.contains("\"ecky_mcp\""));
        assert!(!config.contains("\"ecky_provider_mcp\""));
        assert!(guide.contains("Provider target is already pre-bound"));
        assert!(guide.contains("Do not call `thread_borrow`"));
        assert!(guide.contains("workspace_overview"));
        assert!(guide.contains("agentBrief.primaryGuideUri"));
        assert!(materialized
            .config_path
            .ends_with(".agents/plugins/ecky-provider/mcp_config.json"));
        assert!(materialized
            .guide_path
            .ends_with(".agents/plugins/ecky-provider/rules/AGENTS.md"));
        assert!(manifest.contains("\"disabled\": false"));

        let _ = std::fs::remove_dir_all(directory);
    }
}
