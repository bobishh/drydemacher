use crate::db;
use crate::mcp::contracts::*;
use crate::mcp::runtime;
use crate::models::{
    AgentSession, AppError, AppResult, AppState, ArtifactBundle, ControlPrimitive, ControlView,
    ControlViewSource, DesignOutput, InteractionMode, MacroDialect, MeasurementAnnotation,
    MeasurementAnnotationSource, ModelManifest, ModelSourceKind, PathResolver, UiSpec,
};
use crate::services::agent_versions::{
    save_or_update_agent_version_for_session, SaveOrUpdateAgentVersionRequest,
};
use crate::services::design::{auto_heal_legacy_params, is_param_schema_mismatch};
use crate::services::{agent_dialogue, history, render};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::Emitter;
use tokio::sync::oneshot;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct AgentContext {
    pub session_id: String,
    pub client_kind: String,
    pub host_label: String,
    pub agent_label: String,
    pub llm_model_id: Option<String>,
    pub llm_model_label: Option<String>,
}

impl AgentContext {
    pub fn as_identity_response(&self) -> AgentIdentityResponse {
        AgentIdentityResponse {
            session_id: self.session_id.clone(),
            client_kind: self.client_kind.clone(),
            host_label: self.host_label.clone(),
            agent_label: self.agent_label.clone(),
            llm_model_id: self.llm_model_id.clone(),
            llm_model_label: self.llm_model_label.clone(),
        }
    }

    pub fn with_override(&self, override_identity: &AgentIdentityOverride) -> Self {
        let agent_label = override_identity
            .agent_label
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| self.agent_label.clone());
        let llm_model_id = override_identity
            .llm_model_id
            .as_ref()
            .and_then(|value| {
                let trimmed = value.trim().to_string();
                (!trimmed.is_empty()).then_some(trimmed)
            })
            .or_else(|| self.llm_model_id.clone());
        let llm_model_label = override_identity
            .llm_model_label
            .as_ref()
            .and_then(|value| {
                let trimmed = value.trim().to_string();
                (!trimmed.is_empty()).then_some(trimmed)
            })
            .or_else(|| self.llm_model_label.clone());

        Self {
            session_id: self.session_id.clone(),
            client_kind: self.client_kind.clone(),
            host_label: self.host_label.clone(),
            agent_label,
            llm_model_id,
            llm_model_label,
        }
    }
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

fn persist_agent_session(
    conn: &rusqlite::Connection,
    ctx: &AgentContext,
    thread_id: Option<String>,
    message_id: Option<String>,
    model_id: Option<String>,
    phase: &str,
    status_text: impl Into<String>,
) -> AppResult<()> {
    db::upsert_agent_session(
        conn,
        &AgentSession {
            session_id: ctx.session_id.clone(),
            client_kind: ctx.client_kind.clone(),
            host_label: ctx.host_label.clone(),
            agent_label: ctx.agent_label.clone(),
            llm_model_id: ctx.llm_model_id.clone(),
            llm_model_label: ctx.llm_model_label.clone(),
            thread_id,
            message_id,
            model_id,
            phase: phase.to_string(),
            status_text: status_text.into(),
            updated_at: now_secs(),
        },
    )
    .map_err(|e| AppError::persistence(e.to_string()))
}

fn try_record_agent_error(
    state: &AppState,
    conn: &rusqlite::Connection,
    ctx: &AgentContext,
    thread_id: Option<String>,
    message_id: Option<String>,
    model_id: Option<String>,
    err: &AppError,
) {
    let _ = persist_agent_session(
        conn,
        ctx,
        thread_id.clone(),
        message_id.clone(),
        model_id.clone(),
        "error",
        err.to_string(),
    );
    push_trace_event_with_conn(
        state,
        conn,
        ctx,
        TraceEvent {
            thread_id: thread_id.clone(),
            message_id: message_id.clone(),
            model_id,
            phase: "error",
            kind: "tool_error",
            summary: err.message.clone(),
            details: err.details.clone(),
        },
    );
    if let Some(thread_id) = thread_id {
        let timestamp = now_secs();
        let _ = db::add_message(
            conn,
            &thread_id,
            &crate::models::Message {
                id: Uuid::new_v4().to_string(),
                role: crate::models::MessageRole::Assistant,
                content: err.message.clone(),
                status: crate::models::MessageStatus::Error,
                output: None,
                usage: None,
                artifact_bundle: None,
                model_manifest: None,
                agent_origin: Some(agent_dialogue::build_agent_origin(
                    &dialogue_identity(ctx),
                    timestamp,
                )),
                image_data: None,
                visual_kind: None,
                attachment_images: Vec::new(),
                timestamp,
            },
        );
    }
}

fn dialogue_identity(ctx: &AgentContext) -> agent_dialogue::AgentDialogueIdentity {
    agent_dialogue::AgentDialogueIdentity {
        session_id: ctx.session_id.clone(),
        client_kind: ctx.client_kind.clone(),
        host_label: ctx.host_label.clone(),
        agent_label: ctx.agent_label.clone(),
        llm_model_id: ctx.llm_model_id.clone(),
        llm_model_label: ctx.llm_model_label.clone(),
    }
}

struct TraceEvent<'a> {
    thread_id: Option<String>,
    message_id: Option<String>,
    model_id: Option<String>,
    phase: &'a str,
    kind: &'a str,
    summary: String,
    details: Option<String>,
}

fn log_trace_event(state: &AppState, ctx: &AgentContext, event: TraceEvent<'_>) {
    let target = match (event.thread_id.as_deref(), event.message_id.as_deref()) {
        (Some(thread_id), Some(message_id)) => {
            format!(" thread={} message={}", thread_id, message_id)
        }
        (Some(thread_id), None) => format!(" thread={}", thread_id),
        _ => String::new(),
    };
    let model = event
        .model_id
        .as_deref()
        .map(|model_id| format!(" model={}", model_id))
        .unwrap_or_default();
    let details = event
        .details
        .as_deref()
        .map(|value| format!("\n{}", value))
        .unwrap_or_default();
    state.push_log(format!(
        "[MCP] session={} agent={} phase={} kind={}{}{}: {}{}",
        ctx.session_id,
        ctx.agent_label,
        event.phase,
        event.kind,
        target,
        model,
        event.summary,
        details,
    ));
}

fn push_trace_event(state: &AppState, ctx: &AgentContext, event: TraceEvent<'_>) {
    log_trace_event(state, ctx, event);
}

fn push_trace_event_with_conn(
    state: &AppState,
    _conn: &rusqlite::Connection,
    ctx: &AgentContext,
    event: TraceEvent<'_>,
) {
    log_trace_event(state, ctx, event);
}

fn has_managed_runtime_session(state: &AppState, session_id: &str) -> bool {
    runtime::runtime_snapshot_by_session_id(state, session_id).is_some()
}

async fn resolve_prompt_thread_context(
    state: &AppState,
    prompt_target: Option<&agent_dialogue::SessionThreadTarget>,
) -> AppResult<(Option<String>, Option<String>)> {
    let Some(target) = prompt_target else {
        return Ok((None, None));
    };

    let thread_id = target.thread_id.clone();
    let conn = state.db.lock().await;
    let thread_title = db::get_thread_title(&conn, &thread_id)
        .map_err(|err| AppError::persistence(err.to_string()))?
        .and_then(|title| {
            let trimmed = title.trim().to_string();
            (!trimmed.is_empty()).then_some(trimmed)
        });
    Ok((Some(thread_id), thread_title))
}

#[derive(Debug, Clone)]
struct ManagedPendingTarget {
    thread_id: String,
    message_id: Option<String>,
    model_id: Option<String>,
}

fn managed_pending_target(state: &AppState, session_id: &str) -> Option<ManagedPendingTarget> {
    runtime::runtime_snapshot_by_session_id(state, session_id).and_then(|snapshot| {
        snapshot
            .pending_thread_id
            .map(|thread_id| ManagedPendingTarget {
                thread_id,
                message_id: snapshot.pending_message_id,
                model_id: snapshot.pending_model_id,
            })
    })
}

async fn mutate_live_session<F>(state: &AppState, ctx: &AgentContext, mutate: F)
where
    F: FnOnce(&mut crate::models::McpSessionState),
{
    let mut sessions = state.mcp_sessions.lock().await;
    if let Some(session) = sessions.get_mut(&ctx.session_id) {
        session.agent_label = ctx.agent_label.clone();
        session.llm_model_id = ctx.llm_model_id.clone();
        session.llm_model_label = ctx.llm_model_label.clone();
        mutate(session);
        session.updated_at = now_secs();
    }
}

fn session_target_ref(
    thread_id: impl Into<String>,
    message_id: impl Into<String>,
    model_id: Option<String>,
) -> crate::models::McpTargetRef {
    crate::models::McpTargetRef {
        thread_id: thread_id.into(),
        message_id: message_id.into(),
        model_id,
    }
}

async fn mark_live_session_waiting(
    state: &AppState,
    ctx: &AgentContext,
    target: Option<crate::models::McpTargetRef>,
    status_text: String,
) {
    mutate_live_session(state, ctx, move |session| {
        if let Some(target) = target.clone() {
            session.bound_thread_id = Some(target.thread_id.clone());
            session.last_target = Some(target);
        }
        session.phase = Some("waiting_for_user".to_string());
        session.status_text = Some(status_text.clone());
        session.busy = false;
        session.activity_label = None;
        session.activity_started_at = None;
        session.attention_kind = None;
        session.waiting_on_prompt = true;
    })
    .await;
}

async fn mark_live_session_busy(
    state: &AppState,
    ctx: &AgentContext,
    target: Option<crate::models::McpTargetRef>,
    phase: impl Into<String>,
    status_text: Option<String>,
    activity_label: Option<String>,
    reset_activity_started_at: bool,
) {
    let phase = phase.into();
    mutate_live_session(state, ctx, move |session| {
        if let Some(target) = target.clone() {
            session.bound_thread_id = Some(target.thread_id.clone());
            session.last_target = Some(target);
        }
        session.phase = Some(phase.clone());
        session.status_text = status_text.clone();
        session.busy = true;
        session.waiting_on_prompt = false;
        if reset_activity_started_at || session.activity_started_at.is_none() {
            session.activity_started_at = Some(now_secs());
        }
        session.activity_label = activity_label.clone();
        session.attention_kind = None;
    })
    .await;
}

async fn mark_live_session_idle(
    state: &AppState,
    ctx: &AgentContext,
    target: Option<crate::models::McpTargetRef>,
    phase: impl Into<String>,
    status_text: Option<String>,
) {
    let phase = phase.into();
    mutate_live_session(state, ctx, move |session| {
        if let Some(target) = target.clone() {
            session.bound_thread_id = Some(target.thread_id.clone());
            session.last_target = Some(target);
        }
        session.phase = Some(phase.clone());
        session.status_text = status_text.clone();
        session.busy = false;
        session.activity_label = None;
        session.activity_started_at = None;
        session.attention_kind = None;
        session.waiting_on_prompt = false;
    })
    .await;
}

async fn current_turn_working_message_id_for_thread(
    state: &AppState,
    session_id: &str,
    thread_id: &str,
) -> Option<String> {
    state
        .mcp_sessions
        .lock()
        .await
        .get(session_id)
        .and_then(|session| {
            (session.current_turn_thread_id.as_deref() == Some(thread_id))
                .then(|| session.current_turn_working_message_id.clone())
                .flatten()
        })
}

async fn remember_turn_working_message(
    state: &AppState,
    session_id: &str,
    thread_id: &str,
    message_id: &str,
) {
    let mut sessions = state.mcp_sessions.lock().await;
    if let Some(session) = sessions.get_mut(session_id) {
        if session.current_turn_id.is_none() {
            session.current_turn_id = Some(Uuid::new_v4().to_string());
        }
        session.current_turn_thread_id = Some(thread_id.to_string());
        session.current_turn_working_message_id = Some(message_id.to_string());
        session.updated_at = now_secs();
    }
}

#[allow(clippy::too_many_arguments)]
async fn ensure_turn_working_version_message(
    state: &AppState,
    app: &dyn PathResolver,
    ctx: &AgentContext,
    thread_id: &str,
    base_message_id: &str,
    model_id: Option<String>,
    design_output: &DesignOutput,
    artifact_bundle: Option<ArtifactBundle>,
    model_manifest: Option<ModelManifest>,
    create_summary: String,
) -> AppResult<String> {
    if let Some(message_id) =
        current_turn_working_message_id_for_thread(state, &ctx.session_id, thread_id).await
    {
        let conn = state.db.lock().await;
        if crate::services::target::resolve_target(
            &conn,
            app,
            Some(thread_id.to_string()),
            Some(message_id.clone()),
        )
        .is_ok()
        {
            return Ok(message_id);
        }
        drop(conn);
    }

    let mut working_design = design_output.clone();
    working_design.version_name.clear();

    let save_result = save_or_update_agent_version_for_session(
        state,
        app,
        SaveOrUpdateAgentVersionRequest {
            session_id: ctx.session_id.clone(),
            thread_id: thread_id.to_string(),
            base_message_id: base_message_id.to_string(),
            model_id,
            design_output: working_design,
            artifact_bundle,
            model_manifest,
            updated_at: now_secs(),
            response_text_created: create_summary,
            response_text_updated: String::new(),
            preserve_existing_title: true,
            preserve_existing_version_name: false,
            force_create_new_message: true,
            announce_created_working_version: true,
        },
    )
    .await?;
    remember_turn_working_message(
        state,
        &ctx.session_id,
        &save_result.thread_id,
        &save_result.message_id,
    )
    .await;
    Ok(save_result.message_id)
}

async fn resolve_turn_working_target(
    state: &AppState,
    app: &dyn PathResolver,
    ctx: &AgentContext,
    target: crate::services::target::ResolvedTarget,
    create_summary: String,
) -> AppResult<crate::services::target::ResolvedTarget> {
    let working_message_id = ensure_turn_working_version_message(
        state,
        app,
        ctx,
        &target.thread_id,
        &target.message_id,
        target
            .artifact_bundle
            .as_ref()
            .map(|bundle| bundle.model_id.clone()),
        target
            .design
            .as_ref()
            .ok_or_else(|| AppError::validation("Target has no design output."))?,
        target.artifact_bundle.clone(),
        target.model_manifest.clone(),
        create_summary,
    )
    .await?;
    let conn = state.db.lock().await;
    crate::services::target::resolve_target(
        &conn,
        app,
        Some(target.thread_id),
        Some(working_message_id),
    )
}

async fn resolve_turn_working_semantic_target(
    state: &AppState,
    app: &dyn PathResolver,
    ctx: &AgentContext,
    target: SemanticTargetState,
    create_summary: String,
) -> AppResult<SemanticTargetState> {
    let working_message_id = ensure_turn_working_version_message(
        state,
        app,
        ctx,
        &target.thread_id,
        &target.message_id,
        Some(target.artifact_bundle.model_id.clone()),
        &target.design_output,
        Some(target.artifact_bundle.clone()),
        Some(target.model_manifest.clone()),
        create_summary,
    )
    .await?;
    let conn = state.db.lock().await;
    resolve_semantic_target(&conn, app, Some(target.thread_id), Some(working_message_id))
}

fn summarize_user_facing_text(content: &str) -> String {
    let normalized = content.split_whitespace().collect::<Vec<_>>().join(" ");
    let trimmed = normalized.trim();
    if trimmed.is_empty() {
        return "Saved an empty agent reply.".to_string();
    }
    if trimmed.len() <= 120 {
        return trimmed.to_string();
    }
    format!("{}…", &trimmed[..119])
}

pub async fn handle_user_confirm_request(
    state: &AppState,
    handle: &tauri::AppHandle,
    req: UserConfirmRequest,
    ctx: &AgentContext,
) -> AppResult<UserConfirmResponse> {
    let request_id = req
        .request_id
        .filter(|id| !id.trim().is_empty())
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    let buttons = req
        .buttons
        .filter(|b| !b.is_empty())
        .unwrap_or_else(|| vec!["Yes".to_string(), "No".to_string()]);
    let timeout_secs = req.timeout_secs.unwrap_or(120).clamp(5, 600);

    let (tx, rx) = oneshot::channel::<String>();

    {
        let mut channels = state.confirm_channels.lock().await;
        channels.insert(request_id.clone(), tx);
    }

    handle
        .emit(
            "agent-confirm-request",
            AgentConfirmEvent {
                request_id: request_id.clone(),
                message: req.message,
                buttons,
                agent_label: ctx.agent_label.clone(),
            },
        )
        .map_err(|e| AppError::internal(format!("Failed to emit confirmation event: {}", e)))?;

    let choice = tokio::time::timeout(std::time::Duration::from_secs(timeout_secs), rx)
        .await
        .map_err(|_| {
            // Clean up stale channel on timeout
            let state_clone = state.confirm_channels.clone();
            let id_clone = request_id.clone();
            tokio::spawn(async move {
                state_clone.lock().await.remove(&id_clone);
            });
            AppError::internal(format!(
                "User confirmation timed out after {} seconds.",
                timeout_secs
            ))
        })?
        .map_err(|_| AppError::internal("Confirmation channel closed unexpectedly.".to_string()))?;

    Ok(UserConfirmResponse { request_id, choice })
}

pub async fn handle_request_user_prompt(
    state: &AppState,
    handle: &tauri::AppHandle,
    req: UserPromptRequest,
    ctx: &AgentContext,
) -> AppResult<UserPromptResponse> {
    let request_id = req
        .request_id
        .filter(|id| !id.trim().is_empty())
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    let timeout_secs = req.timeout_secs.unwrap_or(300).clamp(10, 1800);
    let prompt_message = req.message.clone();
    let prompt_content = agent_dialogue::normalize_prompt_request_message(
        prompt_message.as_deref(),
        &ctx.agent_label,
    );
    let prompt_target =
        agent_dialogue::resolve_session_thread_target(state, &ctx.session_id).await?;
    let (response_thread_id, response_thread_title) =
        resolve_prompt_thread_context(state, prompt_target.as_ref()).await?;
    if has_managed_runtime_session(state, &ctx.session_id) && prompt_target.is_none() {
        push_trace_event(
            state,
            ctx,
            TraceEvent {
                thread_id: None,
                message_id: None,
                model_id: None,
                phase: "error",
                kind: "session_bind_failed",
                summary: "Managed active session tried to prompt the user without a bound target."
                    .to_string(),
                details: Some(
                    "Wake the agent from a selected thread/version before the first request_user_prompt."
                        .to_string(),
                ),
            },
        );
        return Err(AppError::validation(
            "Managed active session has no bound target for request_user_prompt.",
        ));
    }
    push_trace_event(
        state,
        ctx,
        TraceEvent {
            thread_id: prompt_target
                .as_ref()
                .map(|target| target.thread_id.clone()),
            message_id: prompt_target
                .as_ref()
                .and_then(|target| target.message_id.clone()),
            model_id: prompt_target
                .as_ref()
                .and_then(|target| target.model_id.clone()),
            phase: "waiting_for_user",
            kind: "request_user_prompt",
            summary: prompt_content.clone(),
            details: None,
        },
    );

    let (tx, rx) = oneshot::channel::<crate::contracts::ResolveAgentPromptInput>();

    {
        let mut channels = state.prompt_channels.lock().await;
        channels.insert(request_id.clone(), tx);
    }

    if let Some(target) = prompt_target.as_ref() {
        let timestamp = now_secs();
        agent_dialogue::add_dialogue_message(
            state,
            &target.thread_id,
            &crate::models::Message {
                id: Uuid::new_v4().to_string(),
                role: crate::models::MessageRole::Assistant,
                content: prompt_content.clone(),
                status: crate::models::MessageStatus::Success,
                output: None,
                usage: None,
                artifact_bundle: None,
                model_manifest: None,
                agent_origin: Some(agent_dialogue::build_agent_origin(
                    &dialogue_identity(ctx),
                    timestamp,
                )),
                image_data: None,
                visual_kind: None,
                attachment_images: Vec::new(),
                timestamp,
            },
        )
        .await?;
        state.emit_history_updated();
    }

    let prompt_target_ref = prompt_target.as_ref().and_then(|target| {
        target.message_id.clone().map(|message_id| {
            session_target_ref(
                target.thread_id.clone(),
                message_id,
                target.model_id.clone(),
            )
        })
    });
    mark_live_session_waiting(state, ctx, prompt_target_ref, prompt_content.clone()).await;

    handle
        .emit(
            "agent-prompt-request",
            AgentPromptEvent {
                request_id: request_id.clone(),
                message: prompt_message.clone(),
                agent_label: ctx.agent_label.clone(),
                session_id: ctx.session_id.clone(),
            },
        )
        .map_err(|e| AppError::internal(format!("Failed to emit prompt event: {}", e)))?;

    // For active-mode auto-agents: freeze the process group while waiting.
    // The supervisor registered the pgid; we stash it so resolve can SIGCONT.
    #[cfg(unix)]
    let pgid = {
        let pgid = if has_managed_runtime_session(state, &ctx.session_id) {
            runtime::runtime_snapshot_by_session_id(state, &ctx.session_id)
                .and_then(|snapshot| snapshot.pid)
        } else {
            None
        };
        if let Some(pgid) = pgid {
            unsafe {
                libc::kill(-pgid, libc::SIGSTOP);
            }
            eprintln!("[MCP] SIGSTOP pgid {} (agent: {})", pgid, ctx.agent_label);
        }
        pgid
    };
    #[cfg(not(unix))]
    let pgid = None;
    state.prompt_waits.lock().unwrap().insert(
        request_id.clone(),
        crate::models::PromptResumeState {
            pgid,
            agent_label: ctx.agent_label.clone(),
            session_id: ctx.session_id.clone(),
            thread_id: prompt_target.map(|target| target.thread_id),
        },
    );
    if has_managed_runtime_session(state, &ctx.session_id) {
        runtime::mark_managed_session_waiting(
            state,
            &ctx.session_id,
            ctx.llm_model_label.clone(),
            prompt_message
                .clone()
                .or_else(|| Some("Waiting for your next queued message.".to_string())),
        );
    }

    let prompt_input = tokio::time::timeout(std::time::Duration::from_secs(timeout_secs), rx)
        .await
        .map_err(|_| {
            let state_clone = state.prompt_channels.clone();
            let id_clone = request_id.clone();
            tokio::spawn(async move {
                state_clone.lock().await.remove(&id_clone);
            });
            runtime::release_prompt_wait(state, &request_id);
            AppError::internal(format!(
                "User prompt timed out after {} seconds.",
                timeout_secs
            ))
        })?
        .map_err(|_| {
            runtime::release_prompt_wait(state, &request_id);
            AppError::internal("Prompt channel closed unexpectedly.".to_string())
        })?;

    Ok(UserPromptResponse {
        request_id,
        prompt_text: prompt_input.prompt_text,
        thread_id: response_thread_id,
        thread_title: response_thread_title,
        attachments: prompt_input.attachments,
    })
}

pub async fn handle_session_reply_save(
    state: &AppState,
    req: SessionReplySaveRequest,
    ctx: &AgentContext,
) -> AppResult<SessionReplySaveResponse> {
    let ctx = ctx.with_override(&req.identity);
    let body = req.body.trim();
    if body.is_empty() {
        return Err(AppError::validation(
            "session_reply_save requires a non-empty body.",
        ));
    }

    let target = if let Some(thread_id) = req.thread_id.clone() {
        agent_dialogue::SessionThreadTarget {
            thread_id,
            message_id: req.message_id.clone(),
            model_id: None,
        }
    } else {
        agent_dialogue::resolve_session_thread_target(state, &ctx.session_id)
            .await?
            .ok_or_else(|| {
                AppError::validation(
                    "No active session target is available for session_reply_save.",
                )
            })?
    };

    let timestamp = now_secs();
    let message_id = Uuid::new_v4().to_string();
    agent_dialogue::add_dialogue_message(
        state,
        &target.thread_id,
        &crate::models::Message {
            id: message_id.clone(),
            role: crate::models::MessageRole::Assistant,
            content: body.to_string(),
            status: if req.fatal {
                crate::models::MessageStatus::Error
            } else {
                crate::models::MessageStatus::Success
            },
            output: None,
            usage: None,
            artifact_bundle: None,
            model_manifest: None,
            agent_origin: Some(agent_dialogue::build_agent_origin(
                &dialogue_identity(&ctx),
                timestamp,
            )),
            image_data: None,
            visual_kind: None,
            attachment_images: Vec::new(),
            timestamp,
        },
    )
    .await?;
    state.emit_history_updated();

    let conn = state.db.lock().await;
    persist_agent_session(
        &conn,
        &ctx,
        Some(target.thread_id.clone()),
        Some(
            target
                .message_id
                .clone()
                .unwrap_or_else(|| message_id.clone()),
        ),
        target.model_id.clone(),
        if req.fatal { "error" } else { "idle" },
        summarize_user_facing_text(body),
    )?;
    drop(conn);

    push_trace_event(
        state,
        &ctx,
        TraceEvent {
            thread_id: Some(target.thread_id.clone()),
            message_id: target
                .message_id
                .clone()
                .or_else(|| Some(message_id.clone())),
            model_id: target.model_id.clone(),
            phase: if req.fatal { "error" } else { "idle" },
            kind: "final_reply_save",
            summary: summarize_user_facing_text(body),
            details: (!req.fatal)
                .then_some(body.to_string())
                .filter(|text| text.len() > 140),
        },
    );

    if req.fatal {
        mark_live_session_idle(
            state,
            &ctx,
            Some(session_target_ref(
                target.thread_id.clone(),
                target
                    .message_id
                    .clone()
                    .unwrap_or_else(|| message_id.clone()),
                target.model_id.clone(),
            )),
            "error",
            Some(summarize_user_facing_text(body)),
        )
        .await;
        if has_managed_runtime_session(state, &ctx.session_id) {
            runtime::mark_managed_session_error(
                state,
                &ctx.session_id,
                Some(target.thread_id.clone()),
                summarize_user_facing_text(body),
            );
        }
    } else {
        mark_live_session_idle(
            state,
            &ctx,
            Some(session_target_ref(
                target.thread_id.clone(),
                target
                    .message_id
                    .clone()
                    .unwrap_or_else(|| message_id.clone()),
                target.model_id.clone(),
            )),
            "idle",
            Some(summarize_user_facing_text(body)),
        )
        .await;
        if has_managed_runtime_session(state, &ctx.session_id) {
            runtime::mark_managed_session_active(
                state,
                &ctx.session_id,
                Some(target.thread_id.clone()),
                ctx.llm_model_label.clone(),
                Some("Saved final reply.".to_string()),
            );
        }
    }

    Ok(SessionReplySaveResponse {
        thread_id: target.thread_id,
        message_id,
        fatal: req.fatal,
    })
}

pub async fn handle_long_action_notice(
    state: &AppState,
    req: LongActionNoticeRequest,
    ctx: &AgentContext,
) -> AppResult<LongActionNoticeResponse> {
    let response = handle_session_activity_set(
        state,
        SessionActivitySetRequest {
            identity: req.identity,
            phase: req.phase.unwrap_or_else(|| "working".to_string()),
            label: Some(req.message),
            detail: req.details,
            attention_kind: None,
        },
        ctx,
    )
    .await?;

    Ok(LongActionNoticeResponse {
        session_id: response.session_id,
        phase: response.phase,
        busy: response.busy,
        activity_label: response.activity_label.unwrap_or_default(),
        activity_started_at: response.activity_started_at,
    })
}

pub async fn handle_session_activity_set(
    state: &AppState,
    req: SessionActivitySetRequest,
    ctx: &AgentContext,
) -> AppResult<SessionActivitySetResponse> {
    let ctx = ctx.with_override(&req.identity);
    let phase = req.phase.trim().to_string();
    if phase.is_empty() {
        return Err(AppError::validation(
            "session_activity_set requires a non-empty phase.",
        ));
    }
    let label = req
        .label
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    let target = agent_dialogue::resolve_session_thread_target(state, &ctx.session_id).await?;
    let target_ref = target.as_ref().and_then(|target| {
        target.message_id.clone().map(|message_id| {
            session_target_ref(
                target.thread_id.clone(),
                message_id,
                target.model_id.clone(),
            )
        })
    });
    let activity_started_at = now_secs();
    let attention_kind = req
        .attention_kind
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);

    mark_live_session_busy(
        state,
        &ctx,
        target_ref,
        phase.clone(),
        label.clone(),
        label.clone(),
        true,
    )
    .await;
    if attention_kind.is_some() {
        mutate_live_session(state, &ctx, move |session| {
            session.attention_kind = attention_kind.clone();
        })
        .await;
    }

    let conn = state.db.lock().await;
    persist_agent_session(
        &conn,
        &ctx,
        target.as_ref().map(|target| target.thread_id.clone()),
        target.as_ref().and_then(|target| target.message_id.clone()),
        target.as_ref().and_then(|target| target.model_id.clone()),
        &phase,
        label
            .clone()
            .unwrap_or_else(|| format!("Session activity set to {}.", phase)),
    )?;
    drop(conn);

    push_trace_event(
        state,
        &ctx,
        TraceEvent {
            thread_id: target.as_ref().map(|target| target.thread_id.clone()),
            message_id: target.as_ref().and_then(|target| target.message_id.clone()),
            model_id: target.as_ref().and_then(|target| target.model_id.clone()),
            phase: &phase,
            kind: "session_activity_set",
            summary: label
                .clone()
                .unwrap_or_else(|| format!("Session activity set to {}.", phase)),
            details: req
                .detail
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string),
        },
    );

    Ok(SessionActivitySetResponse {
        session_id: ctx.session_id,
        phase,
        busy: true,
        activity_label: label,
        activity_started_at,
    })
}

pub async fn handle_long_action_clear(
    state: &AppState,
    req: LongActionClearRequest,
    ctx: &AgentContext,
) -> AppResult<LongActionClearResponse> {
    let response = handle_session_activity_clear(
        state,
        SessionActivityClearRequest {
            identity: req.identity,
            phase: req.phase,
            status_text: req.status_text,
        },
        ctx,
    )
    .await?;

    Ok(LongActionClearResponse {
        session_id: response.session_id,
        phase: response.phase,
        busy: response.busy,
        status_text: response.status_text,
    })
}

pub async fn handle_session_activity_clear(
    state: &AppState,
    req: SessionActivityClearRequest,
    ctx: &AgentContext,
) -> AppResult<SessionActivityClearResponse> {
    let ctx = ctx.with_override(&req.identity);
    let phase = req
        .phase
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("idle")
        .to_string();
    let status_text = req
        .status_text
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    let target = agent_dialogue::resolve_session_thread_target(state, &ctx.session_id).await?;
    let target_ref = target.as_ref().and_then(|target| {
        target.message_id.clone().map(|message_id| {
            session_target_ref(
                target.thread_id.clone(),
                message_id,
                target.model_id.clone(),
            )
        })
    });

    mark_live_session_idle(state, &ctx, target_ref, phase.clone(), status_text.clone()).await;

    let conn = state.db.lock().await;
    persist_agent_session(
        &conn,
        &ctx,
        target.as_ref().map(|target| target.thread_id.clone()),
        target.as_ref().and_then(|target| target.message_id.clone()),
        target.as_ref().and_then(|target| target.model_id.clone()),
        &phase,
        status_text
            .clone()
            .unwrap_or_else(|| "Cleared long action state.".to_string()),
    )?;
    drop(conn);

    push_trace_event(
        state,
        &ctx,
        TraceEvent {
            thread_id: target.as_ref().map(|target| target.thread_id.clone()),
            message_id: target.as_ref().and_then(|target| target.message_id.clone()),
            model_id: target.as_ref().and_then(|target| target.model_id.clone()),
            phase: &phase,
            kind: "session_activity_clear",
            summary: status_text
                .clone()
                .unwrap_or_else(|| "Cleared long action state.".to_string()),
            details: None,
        },
    );

    Ok(SessionActivityClearResponse {
        session_id: ctx.session_id,
        phase,
        busy: false,
        status_text,
    })
}

pub async fn handle_health_check(
    state: &AppState,
    app: &dyn PathResolver,
) -> AppResult<HealthCheckResponse> {
    let db_ready = state
        .db
        .lock()
        .await
        .query_row("SELECT 1", [], |_row| Ok(()))
        .is_ok();
    let freecad_configured = render::is_freecad_available(state);
    let config_dir = app.app_config_dir();
    let db_path = config_dir
        .join("history.sqlite")
        .to_string_lossy()
        .to_string();

    Ok(HealthCheckResponse {
        server_version: env!("CARGO_PKG_VERSION").to_string(),
        db_path,
        freecad_configured,
        db_ready,
    })
}

pub async fn handle_thread_list(state: &AppState) -> AppResult<ThreadListResponse> {
    let conn = state.db.lock().await;
    let threads = history::get_history(&conn)?;
    let entries = threads
        .into_iter()
        .map(|t| ThreadListEntry {
            thread_id: t.id,
            title: t.title,
            updated_at: t.updated_at,
            version_count: t.version_count,
            pending_count: t.pending_count,
            error_count: t.error_count,
            status: t.status,
            finalized_at: t.finalized_at,
        })
        .collect();

    Ok(ThreadListResponse { threads: entries })
}

pub async fn handle_finalize_thread(
    state: &AppState,
    req: FinalizeThreadRequest,
) -> AppResult<FinalizeThreadResponse> {
    let conn = state.db.lock().await;
    history::finalize_thread(&conn, &req.thread_id)?;
    let finalized_at = now_secs();
    Ok(FinalizeThreadResponse {
        thread_id: req.thread_id,
        finalized_at,
    })
}

pub async fn handle_session_log_in(
    state: &AppState,
    req: SessionLoginRequest,
    ctx: &AgentContext,
) -> AppResult<SessionLoginResponse> {
    let ctx = ctx.with_override(&req.identity);
    let managed_session = has_managed_runtime_session(state, &ctx.session_id);
    let runtime_target = managed_pending_target(state, &ctx.session_id);
    let conn = state.db.lock().await;
    let resolved_thread_id = req.thread_id.clone().or_else(|| {
        runtime_target
            .as_ref()
            .map(|target| target.thread_id.clone())
    });
    let resolved_message_id = req.message_id.clone().or_else(|| {
        runtime_target
            .as_ref()
            .and_then(|target| target.message_id.clone())
    });
    let resolved_model_id = req.model_id.clone().or_else(|| {
        runtime_target
            .as_ref()
            .and_then(|target| target.model_id.clone())
    });

    if managed_session && resolved_thread_id.is_none() {
        push_trace_event(
            state,
            &ctx,
            TraceEvent {
                thread_id: None,
                message_id: None,
                model_id: None,
                phase: "error",
                kind: "session_bind_failed",
                summary: "Managed active session could not bind to a current thread target.".to_string(),
                details: Some(
                    "Wake the active agent from a selected thread/version before the first request_user_prompt."
                        .to_string(),
                ),
            },
        );
        return Err(AppError::validation(
            "Managed active session has no bound thread target. Wake the agent from a selected thread/version first.",
        ));
    }

    persist_agent_session(
        &conn,
        &ctx,
        resolved_thread_id.clone(),
        resolved_message_id.clone(),
        resolved_model_id.clone(),
        "idle",
        "Agent joined the workspace.",
    )?;
    drop(conn);

    if let Some(runtime_target) = runtime_target.as_ref() {
        push_trace_event(
            state,
            &ctx,
            TraceEvent {
                thread_id: Some(runtime_target.thread_id.clone()),
                message_id: runtime_target.message_id.clone(),
                model_id: runtime_target.model_id.clone(),
                phase: "idle",
                kind: "pending_target_captured",
                summary: match runtime_target.message_id.as_deref() {
                    Some(message_id) => format!(
                        "Pending target captured for {} / {}.",
                        runtime_target.thread_id, message_id
                    ),
                    None => format!(
                        "Pending target captured for thread {}.",
                        runtime_target.thread_id
                    ),
                },
                details: None,
            },
        );
    }

    if let Some(message_id) = resolved_message_id.clone() {
        let target = session_target_ref(
            resolved_thread_id
                .clone()
                .expect("message target implies thread target"),
            message_id,
            resolved_model_id.clone(),
        );
        mark_live_session_idle(
            state,
            &ctx,
            Some(target),
            "idle",
            Some("Agent joined the workspace.".to_string()),
        )
        .await;
    } else {
        mutate_live_session(state, &ctx, |session| {
            session.bound_thread_id = resolved_thread_id.clone();
        })
        .await;
        mark_live_session_idle(
            state,
            &ctx,
            None,
            "idle",
            Some("Agent joined the workspace.".to_string()),
        )
        .await;
    }
    if has_managed_runtime_session(state, &ctx.session_id) {
        runtime::mark_managed_session_active(
            state,
            &ctx.session_id,
            resolved_thread_id.clone(),
            ctx.llm_model_label.clone(),
            Some("Connected to Ecky.".to_string()),
        );
    }
    push_trace_event(
        state,
        &ctx,
        TraceEvent {
            thread_id: resolved_thread_id.clone(),
            message_id: resolved_message_id.clone(),
            model_id: resolved_model_id.clone(),
            phase: "idle",
            kind: "session_bound",
            summary: if let Some(thread_id) = resolved_thread_id.as_deref() {
                if let Some(message_id) = resolved_message_id.as_deref() {
                    format!("Bound session to {} / {}.", thread_id, message_id)
                } else {
                    format!("Bound session to thread {}.", thread_id)
                }
            } else {
                "Agent logged in without an active thread target.".to_string()
            },
            details: None,
        },
    );

    Ok(SessionLoginResponse {
        session_id: ctx.session_id.clone(),
        thread_id: resolved_thread_id,
        message_id: resolved_message_id,
        model_id: resolved_model_id,
    })
}

pub async fn handle_session_log_out(
    state: &AppState,
    req: SessionLogoutRequest,
    ctx: &AgentContext,
) -> AppResult<SessionLogoutResponse> {
    let ctx = ctx.with_override(&req.identity);
    let conn = state.db.lock().await;

    // Transition to 'idle' with a status message about leaving.
    // Stale timeout will eventually clear the session UI,
    // but the DB record remains for resume.
    persist_agent_session(
        &conn,
        &ctx,
        None,
        None,
        None,
        "idle",
        "Agent left the workspace (graceful log-out).",
    )?;
    mark_live_session_idle(
        state,
        &ctx,
        None,
        "idle",
        Some("Agent left the workspace.".to_string()),
    )
    .await;
    if has_managed_runtime_session(state, &ctx.session_id) {
        runtime::mark_agent_disconnected_for_session(
            state,
            &ctx.session_id,
            Some("Agent left the workspace.".to_string()),
        );
    }

    Ok(SessionLogoutResponse { success: true })
}

pub async fn handle_session_resume(
    state: &AppState,
    req: SessionResumeRequest,
    ctx: &AgentContext,
) -> AppResult<SessionResumeResponse> {
    let ctx = ctx.with_override(&req.identity);
    let conn = state.db.lock().await;
    let stored_session = db::get_sessions_by_ids(&conn, std::slice::from_ref(&ctx.session_id))
        .map_err(|e| AppError::persistence(e.to_string()))?
        .into_iter()
        .next()
        .ok_or_else(|| {
            AppError::validation(
                "No previous session found for this session id. Passive MCP resume no longer falls back by agent label.",
            )
        })?;

    persist_agent_session(
        &conn,
        &ctx,
        stored_session.thread_id.clone(),
        stored_session.message_id.clone(),
        stored_session.model_id.clone(),
        "idle",
        "Agent resumed previous session.",
    )?;
    drop(conn);

    let target = match (
        stored_session.thread_id.clone(),
        stored_session.message_id.clone(),
    ) {
        (Some(thread_id), Some(message_id)) => Some(session_target_ref(
            thread_id,
            message_id,
            stored_session.model_id.clone(),
        )),
        _ => None,
    };
    if target.is_none() {
        mutate_live_session(state, &ctx, |session| {
            session.bound_thread_id = stored_session.thread_id.clone();
        })
        .await;
    }
    mark_live_session_idle(
        state,
        &ctx,
        target,
        "idle",
        Some("Agent resumed the previous session.".to_string()),
    )
    .await;
    if has_managed_runtime_session(state, &ctx.session_id) {
        runtime::mark_managed_session_active(
            state,
            &ctx.session_id,
            stored_session.thread_id.clone(),
            ctx.llm_model_label.clone(),
            Some("Agent resumed the previous session.".to_string()),
        );
    }

    Ok(SessionResumeResponse {
        thread_id: stored_session.thread_id,
        message_id: stored_session.message_id,
        model_id: stored_session.model_id,
        last_interaction_at: stored_session.updated_at,
    })
}

pub async fn handle_thread_get(
    state: &AppState,
    req: ThreadGetRequest,
) -> AppResult<ThreadGetResponse> {
    let conn = state.db.lock().await;
    let thread = history::get_thread(&conn, &req.thread_id)?;
    Ok(ThreadGetResponse { thread })
}

pub fn handle_agent_identity_set(
    ctx: &AgentContext,
    req: AgentIdentitySetRequest,
) -> AgentIdentityResponse {
    ctx.with_override(&AgentIdentityOverride {
        agent_label: req.agent_label,
        llm_model_id: req.llm_model_id,
        llm_model_label: req.llm_model_label,
    })
    .as_identity_response()
}

pub async fn handle_target_get(
    state: &AppState,
    app: &dyn PathResolver,
    req: TargetGetRequest,
    ctx: &AgentContext,
) -> AppResult<TargetGetResponse> {
    let ctx = ctx.with_override(&req.identity);
    let ctx = &ctx;
    let conn = state.db.lock().await;

    let mut tracked_thread_id = req.thread_id.clone();
    let mut tracked_message_id = req.message_id.clone();
    let mut tracked_model_id = None;

    let result = (|| -> AppResult<TargetGetResponse> {
        persist_agent_session(
            &conn,
            ctx,
            tracked_thread_id.clone(),
            tracked_message_id.clone(),
            None,
            "reading",
            "",
        )?;

        let target = crate::services::target::resolve_target(
            &conn,
            app,
            req.thread_id.clone(),
            req.message_id.clone(),
        )?;

        tracked_thread_id = Some(target.thread_id.clone());
        tracked_message_id = Some(target.message_id.clone());
        tracked_model_id = target
            .artifact_bundle
            .as_ref()
            .map(|bundle| bundle.model_id.clone());

        persist_agent_session(
            &conn,
            ctx,
            tracked_thread_id.clone(),
            tracked_message_id.clone(),
            tracked_model_id.clone(),
            "reading",
            "",
        )?;

        let design = target
            .design
            .ok_or_else(|| AppError::validation("Target has no design output."))?;

        Ok(TargetGetResponse {
            thread_id: target.thread_id,
            message_id: target.message_id,
            title: design.title,
            version_name: design.version_name,
            macro_code: design.macro_code,
            ui_spec: design.ui_spec,
            initial_params: design.initial_params,
            artifact_bundle: target.artifact_bundle,
            model_manifest: target.model_manifest,
            latest_draft: None,
        })
    })();

    if let Err(err) = &result {
        try_record_agent_error(
            state,
            &conn,
            ctx,
            tracked_thread_id,
            tracked_message_id,
            tracked_model_id,
            err,
        );
    }

    result
}

fn map_target_resolved_from(
    source: crate::services::target::EditableTargetSource,
) -> TargetResolvedFrom {
    match source {
        crate::services::target::EditableTargetSource::Base => TargetResolvedFrom::Base,
    }
}

fn build_target_meta_response(
    target: &crate::services::target::EditableTarget,
) -> TargetMetaResponse {
    let (range_count, number_count, select_count, checkbox_count) = target
        .design_output
        .ui_spec
        .fields
        .iter()
        .fold((0, 0, 0, 0), |acc, field| match field {
            crate::models::UiField::Range { .. } => (acc.0 + 1, acc.1, acc.2, acc.3),
            crate::models::UiField::Number { .. } => (acc.0, acc.1 + 1, acc.2, acc.3),
            crate::models::UiField::Select { .. } => (acc.0, acc.1, acc.2 + 1, acc.3),
            crate::models::UiField::Checkbox { .. } => (acc.0, acc.1, acc.2, acc.3 + 1),
            crate::models::UiField::Image { .. } => acc,
        });

    TargetMetaResponse {
        thread_id: target.thread_id.clone(),
        message_id: target.message_id.clone(),
        title: target.design_output.title.clone(),
        version_name: target.design_output.version_name.clone(),
        model_id: target.model_id(),
        has_draft: false,
        resolved_from: map_target_resolved_from(target.resolved_from),
        ui_field_count: target.design_output.ui_spec.fields.len(),
        range_count,
        number_count,
        select_count,
        checkbox_count,
        parameter_count: target.design_output.initial_params.len(),
        has_semantic_manifest: target.model_manifest.is_some(),
        control_primitive_count: target
            .model_manifest
            .as_ref()
            .map(|manifest| manifest.control_primitives.len())
            .unwrap_or(0),
        control_relation_count: target
            .model_manifest
            .as_ref()
            .map(|manifest| manifest.control_relations.len())
            .unwrap_or(0),
        control_view_count: target
            .model_manifest
            .as_ref()
            .map(|manifest| manifest.control_views.len())
            .unwrap_or(0),
        cad_sdk_snippet: Some(include_str!("../../../model-runtime/cad_sdk.py").to_string()),
    }
}

pub async fn handle_target_meta_get(
    state: &AppState,
    app: &dyn PathResolver,
    req: TargetMetaRequest,
    ctx: &AgentContext,
) -> AppResult<TargetMetaResponse> {
    let ctx = ctx.with_override(&req.identity);
    let ctx = &ctx;
    let conn = state.db.lock().await;

    let mut tracked_thread_id = req.thread_id.clone();
    let mut tracked_message_id = req.message_id.clone();
    let mut tracked_model_id = None;

    let result = (|| -> AppResult<TargetMetaResponse> {
        persist_agent_session(
            &conn,
            ctx,
            tracked_thread_id.clone(),
            tracked_message_id.clone(),
            None,
            "reading",
            "Reading target metadata.",
        )?;

        let target = crate::services::target::resolve_editable_target(
            &conn,
            app,
            req.thread_id.clone(),
            req.message_id.clone(),
        )?;

        tracked_thread_id = Some(target.thread_id.clone());
        tracked_message_id = Some(target.message_id.clone());
        tracked_model_id = target.model_id();

        persist_agent_session(
            &conn,
            ctx,
            tracked_thread_id.clone(),
            tracked_message_id.clone(),
            tracked_model_id.clone(),
            "reading",
            "",
        )?;

        Ok(build_target_meta_response(&target))
    })();

    if let Err(err) = &result {
        try_record_agent_error(
            state,
            &conn,
            ctx,
            tracked_thread_id,
            tracked_message_id,
            tracked_model_id,
            err,
        );
    }

    result
}

pub async fn handle_target_macro_get(
    state: &AppState,
    app: &dyn PathResolver,
    req: TargetMacroRequest,
    ctx: &AgentContext,
) -> AppResult<TargetMacroResponse> {
    let ctx = ctx.with_override(&req.identity);
    let ctx = &ctx;
    let conn = state.db.lock().await;

    let mut tracked_thread_id = req.thread_id.clone();
    let mut tracked_message_id = req.message_id.clone();
    let mut tracked_model_id = None;

    let result = (|| -> AppResult<TargetMacroResponse> {
        persist_agent_session(
            &conn,
            ctx,
            tracked_thread_id.clone(),
            tracked_message_id.clone(),
            None,
            "reading",
            "Reading target macro.",
        )?;

        let target = crate::services::target::resolve_editable_target(
            &conn,
            app,
            req.thread_id.clone(),
            req.message_id.clone(),
        )?;

        tracked_thread_id = Some(target.thread_id.clone());
        tracked_message_id = Some(target.message_id.clone());
        tracked_model_id = target.model_id();

        persist_agent_session(
            &conn,
            ctx,
            tracked_thread_id.clone(),
            tracked_message_id.clone(),
            tracked_model_id.clone(),
            "reading",
            "",
        )?;

        Ok(TargetMacroResponse {
            thread_id: target.thread_id,
            message_id: target.message_id,
            title: target.design_output.title,
            version_name: target.design_output.version_name,
            resolved_from: map_target_resolved_from(target.resolved_from),
            macro_code: target.design_output.macro_code,
            macro_dialect: target.design_output.macro_dialect,
            post_processing: target.design_output.post_processing,
        })
    })();

    if let Err(err) = &result {
        try_record_agent_error(
            state,
            &conn,
            ctx,
            tracked_thread_id,
            tracked_message_id,
            tracked_model_id,
            err,
        );
    }

    result
}

pub async fn handle_target_detail_get(
    state: &AppState,
    app: &dyn PathResolver,
    req: TargetDetailRequest,
    ctx: &AgentContext,
) -> AppResult<TargetDetailResponse> {
    let ctx = ctx.with_override(&req.identity);
    let ctx = &ctx;
    let conn = state.db.lock().await;

    let mut tracked_thread_id = req.thread_id.clone();
    let mut tracked_message_id = req.message_id.clone();
    let mut tracked_model_id = None;

    let result = (|| -> AppResult<TargetDetailResponse> {
        persist_agent_session(
            &conn,
            ctx,
            tracked_thread_id.clone(),
            tracked_message_id.clone(),
            None,
            "reading",
            format!("Reading target detail section {:?}.", req.section),
        )?;

        let target = crate::services::target::resolve_editable_target(
            &conn,
            app,
            req.thread_id.clone(),
            req.message_id.clone(),
        )?;

        tracked_thread_id = Some(target.thread_id.clone());
        tracked_message_id = Some(target.message_id.clone());
        tracked_model_id = target.model_id();

        persist_agent_session(
            &conn,
            ctx,
            tracked_thread_id.clone(),
            tracked_message_id.clone(),
            tracked_model_id.clone(),
            "reading",
            "",
        )?;

        let (ui_spec, initial_params, artifact_bundle, latest_draft) = match req.section {
            TargetDetailSection::UiSpec => {
                (Some(target.design_output.ui_spec.clone()), None, None, None)
            }
            TargetDetailSection::InitialParams => (
                None,
                Some(target.design_output.initial_params.clone()),
                None,
                None,
            ),
            TargetDetailSection::ArtifactBundle => {
                (None, None, Some(target.artifact_bundle.clone()), None)
            }
            TargetDetailSection::LatestDraft => (None, None, None, Some(None)),
        };

        Ok(TargetDetailResponse {
            thread_id: target.thread_id,
            message_id: target.message_id,
            title: target.design_output.title,
            version_name: target.design_output.version_name,
            resolved_from: map_target_resolved_from(target.resolved_from),
            section: req.section,
            ui_spec,
            initial_params,
            artifact_bundle,
            latest_draft,
        })
    })();

    if let Err(err) = &result {
        try_record_agent_error(
            state,
            &conn,
            ctx,
            tracked_thread_id,
            tracked_message_id,
            tracked_model_id,
            err,
        );
    }

    result
}

pub async fn handle_params_patch_and_render(
    state: &AppState,
    app: &dyn PathResolver,
    req: ParamsPatchRequest,
    ctx: &AgentContext,
) -> AppResult<ParamsPatchResponse> {
    let ctx = ctx.with_override(&req.identity);
    let ctx = &ctx;
    let mut tracked_thread_id = req.thread_id.clone();
    let mut tracked_message_id = req.message_id.clone();
    let mut tracked_model_id = None;

    let result = async {
        let conn = state.db.lock().await;
        let target = crate::services::target::resolve_target(
            &conn,
            app,
            req.thread_id.clone(),
            req.message_id.clone(),
        )?;

        tracked_thread_id = Some(target.thread_id.clone());
        tracked_message_id = Some(target.message_id.clone());
        tracked_model_id = target
            .artifact_bundle
            .as_ref()
            .map(|bundle| bundle.model_id.clone());

        persist_agent_session(
            &conn,
            ctx,
            tracked_thread_id.clone(),
            tracked_message_id.clone(),
            tracked_model_id.clone(),
            "patching_params",
            "",
        )?;
        mark_live_session_busy(
            state,
            ctx,
            Some(session_target_ref(
                target.thread_id.clone(),
                target.message_id.clone(),
                tracked_model_id.clone(),
            )),
            "patching_params",
            Some("Patching parameters for the active target.".to_string()),
            None,
            false,
        )
        .await;
        push_trace_event_with_conn(
            state,
            &conn,
            ctx,
            TraceEvent {
                thread_id: tracked_thread_id.clone(),
                message_id: tracked_message_id.clone(),
                model_id: tracked_model_id.clone(),
                phase: "patching_params",
                kind: "tool_start",
                summary: "Patching parameters for the active target.".to_string(),
                details: None,
            },
        );

        let base_design = target
            .design
            .ok_or_else(|| AppError::validation("Target has no design output."))?;

        let mut merged_params = base_design.initial_params.clone();
        for (key, value) in req.parameter_patch.clone() {
            merged_params.insert(key, value);
        }

        let mut healed_ui_spec = base_design.ui_spec.clone();
        let mut healed_params = merged_params.clone();
        if let Err(err) = crate::models::validate_design_params(&healed_params, &healed_ui_spec) {
            if base_design.macro_dialect == MacroDialect::Legacy && is_param_schema_mismatch(&err) {
                if let Some((next_ui_spec, next_params, heal_report)) = auto_heal_legacy_params(
                    &base_design.macro_code,
                    &healed_ui_spec,
                    &healed_params,
                    Some(&base_design.initial_params),
                )? {
                    push_trace_event_with_conn(
                        state,
                        &conn,
                        ctx,
                        TraceEvent {
                            thread_id: tracked_thread_id.clone(),
                            message_id: tracked_message_id.clone(),
                            model_id: tracked_model_id.clone(),
                            phase: "patching_params",
                            kind: "auto_heal_applied",
                            summary: "Reconciled legacy uiSpec and initialParams from parsed macro params."
                                .to_string(),
                            details: Some(format!(
                                "added={:?}; dropped={:?}; carried={:?}",
                                heal_report.added_keys, heal_report.dropped_keys, heal_report.carried_keys
                            )),
                        },
                    );
                    healed_ui_spec = next_ui_spec;
                    healed_params = next_params;
                } else {
                    return Err(AppError::with_details(
                        crate::contracts::AppErrorCode::Validation,
                        err.message,
                        format!(
                            "Legacy param auto-heal could not parse dynamic params for session {} on thread {:?}.",
                            ctx.session_id, tracked_thread_id
                        ),
                    ));
                }
            } else {
                return Err(err);
            }
        }

        persist_agent_session(
            &conn,
            ctx,
            tracked_thread_id.clone(),
            tracked_message_id.clone(),
            tracked_model_id.clone(),
            "rendering",
            "",
        )?;
        mark_live_session_busy(
            state,
            ctx,
            Some(session_target_ref(
                target.thread_id.clone(),
                target.message_id.clone(),
                tracked_model_id.clone(),
            )),
            "rendering",
            Some("Rendering the updated version.".to_string()),
            None,
            false,
        )
        .await;

        drop(conn);

        let next_post_processing = req
            .post_processing
            .clone()
            .or_else(|| base_design.post_processing.clone());

        let artifact_bundle = render::render_model(
            &base_design.macro_code,
            &healed_params,
            next_post_processing.as_ref(),
            state,
            app,
        )
        .await?;
        let model_manifest = crate::freecad::get_model_manifest(app, &artifact_bundle.model_id)?;
        tracked_model_id = Some(artifact_bundle.model_id.clone());

        let mut design_output = base_design.clone();
        design_output.ui_spec = healed_ui_spec;
        design_output.initial_params = healed_params.clone();
        design_output.post_processing = next_post_processing;
        design_output.version_name.clear();

        let save_result = save_or_update_agent_version_for_session(
            state,
            app,
            SaveOrUpdateAgentVersionRequest {
                session_id: ctx.session_id.clone(),
                thread_id: target.thread_id.clone(),
                base_message_id: target.message_id.clone(),
                model_id: Some(artifact_bundle.model_id.clone()),
                design_output: design_output.clone(),
                artifact_bundle: Some(artifact_bundle.clone()),
                model_manifest: Some(model_manifest.clone()),
                updated_at: now_secs(),
                response_text_created: String::new(),
                response_text_updated: String::new(),
                preserve_existing_title: true,
                preserve_existing_version_name: true,
                force_create_new_message: false,
                announce_created_working_version: false,
            },
        )
        .await?;
        tracked_message_id = Some(save_result.message_id.clone());

        Ok(ParamsPatchResponse {
            thread_id: target.thread_id,
            message_id: save_result.message_id,
            merged_params: healed_params,
            artifact_bundle,
            model_manifest,
            design_output,
        })
    }
    .await;

    if let Err(err) = &result {
        let conn = state.db.lock().await;
        try_record_agent_error(
            state,
            &conn,
            ctx,
            tracked_thread_id,
            tracked_message_id,
            tracked_model_id,
            err,
        );
    }

    result
}

pub async fn handle_macro_replace_and_render(
    state: &AppState,
    app: &dyn PathResolver,
    req: MacroReplaceRequest,
    ctx: &AgentContext,
) -> AppResult<MacroReplaceResponse> {
    let ctx = ctx.with_override(&req.identity);
    let ctx = &ctx;
    let mut tracked_thread_id = req.thread_id.clone();
    let mut tracked_message_id = req.message_id.clone();
    let mut tracked_model_id = None;

    let result = async {
        let conn = state.db.lock().await;
        let target = crate::services::target::resolve_target(
            &conn,
            app,
            req.thread_id.clone(),
            req.message_id.clone(),
        )?;
        drop(conn);
        let target = resolve_turn_working_target(
            state,
            app,
            ctx,
            target,
            format!("{} created a working version for this turn.", ctx.agent_label),
        )
        .await?;
        let conn = state.db.lock().await;

        tracked_thread_id = Some(target.thread_id.clone());
        tracked_message_id = Some(target.message_id.clone());
        tracked_model_id = target
            .artifact_bundle
            .as_ref()
            .map(|bundle| bundle.model_id.clone());

        persist_agent_session(
            &conn,
            ctx,
            tracked_thread_id.clone(),
            tracked_message_id.clone(),
            tracked_model_id.clone(),
            "patching_macro",
            "",
        )?;
        mark_live_session_busy(
            state,
            ctx,
            Some(session_target_ref(
                target.thread_id.clone(),
                target.message_id.clone(),
                tracked_model_id.clone(),
            )),
            "patching_macro",
            Some("Replacing macro code for the active target.".to_string()),
            None,
            false,
        )
        .await;
        push_trace_event_with_conn(
            state,
            &conn,
            ctx,
            TraceEvent {
                thread_id: tracked_thread_id.clone(),
                message_id: tracked_message_id.clone(),
                model_id: tracked_model_id.clone(),
                phase: "patching_macro",
                kind: "tool_start",
                summary: "Replacing macro code for the active target.".to_string(),
                details: None,
            },
        );

        let base_design = target
            .design
            .ok_or_else(|| AppError::validation("Target has no design output."))?;

        let framework_parsed = crate::commands::design::derive_framework_controls(&req.macro_code)?;
        let parsed_legacy = if framework_parsed.is_none() {
            Some(crate::commands::design::parse_macro_params(req.macro_code.clone()))
        } else {
            None
        };
        let (mut ui_spec, mut initial_params, macro_dialect) = if let Some(parsed) = framework_parsed {
            let current_params = req
                .parameters
                .clone()
                .unwrap_or_else(|| base_design.initial_params.clone());
            (
                UiSpec {
                    fields: parsed.fields.clone(),
                },
                crate::commands::design::reconcile_framework_params(
                    &parsed.fields,
                    &current_params,
                    &parsed.params,
                ),
                MacroDialect::CadFrameworkV1,
            )
        } else {
            let parsed_legacy = parsed_legacy
                .clone()
                .expect("legacy parser should exist when framework parse is absent");
            let mut reconciled_params = parsed_legacy.params.clone();
            if let Some(provided) = req.parameters.clone() {
                for (key, value) in provided {
                    if reconciled_params.contains_key(&key) {
                        reconciled_params.insert(key, value);
                    }
                }
            } else {
                for (key, value) in &base_design.initial_params {
                    if reconciled_params.contains_key(key.as_str()) {
                        reconciled_params.insert(key.clone(), value.clone());
                    }
                }
            }
            (
                req.ui_spec.clone().unwrap_or_else(|| {
                    if parsed_legacy.fields.is_empty() {
                        base_design.ui_spec.clone()
                    } else {
                        UiSpec {
                            fields: parsed_legacy.fields,
                        }
                    }
                }),
                reconciled_params,
                MacroDialect::Legacy,
            )
        };
        if let Err(err) = crate::models::validate_design_params(&initial_params, &ui_spec) {
            if macro_dialect == MacroDialect::Legacy && is_param_schema_mismatch(&err) {
                if let Some((next_ui_spec, next_params, heal_report)) = auto_heal_legacy_params(
                    &req.macro_code,
                    &ui_spec,
                    &initial_params,
                    Some(&base_design.initial_params),
                )? {
                    push_trace_event_with_conn(
                        state,
                        &conn,
                        ctx,
                        TraceEvent {
                            thread_id: tracked_thread_id.clone(),
                            message_id: tracked_message_id.clone(),
                            model_id: tracked_model_id.clone(),
                            phase: "patching_macro",
                            kind: "auto_heal_applied",
                            summary: "Reconciled legacy uiSpec and initialParams from parsed macro params."
                                .to_string(),
                            details: Some(format!(
                                "added={:?}; dropped={:?}; carried={:?}",
                                heal_report.added_keys, heal_report.dropped_keys, heal_report.carried_keys
                            )),
                        },
                    );
                    ui_spec = next_ui_spec;
                    initial_params = next_params;
                } else {
                    let parsed_keys = parsed_legacy
                        .as_ref()
                        .map(|parsed| {
                            parsed
                                .params
                                .keys()
                                .cloned()
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default();
                    return Err(AppError::with_details(
                        crate::contracts::AppErrorCode::Validation,
                        err.message,
                        format!(
                            "Legacy param auto-heal failed for session {} on thread {:?}. parsedKeys={:?}",
                            ctx.session_id, tracked_thread_id, parsed_keys
                        ),
                    ));
                }
            } else {
                return Err(err);
            }
        }

        persist_agent_session(
            &conn,
            ctx,
            tracked_thread_id.clone(),
            tracked_message_id.clone(),
            tracked_model_id.clone(),
            "rendering",
            "",
        )?;
        mark_live_session_busy(
            state,
            ctx,
            Some(session_target_ref(
                target.thread_id.clone(),
                target.message_id.clone(),
                tracked_model_id.clone(),
            )),
            "rendering",
            Some("Rendering the updated version.".to_string()),
            None,
            false,
        )
        .await;

        drop(conn);

        let next_post_processing = req
            .post_processing
            .clone()
            .or_else(|| base_design.post_processing.clone());

        let artifact_bundle = render::render_model(
            &req.macro_code,
            &initial_params,
            next_post_processing.as_ref(),
            state,
            app,
        )
        .await?;
        let model_manifest = crate::freecad::get_model_manifest(app, &artifact_bundle.model_id)?;
        tracked_model_id = Some(artifact_bundle.model_id.clone());

        let design_output = DesignOutput {
            title: base_design.title.clone(),
            version_name: String::new(),
            response: "Draft update via macro replacement.".to_string(),
            interaction_mode: InteractionMode::Design,
            macro_code: req.macro_code.clone(),
            macro_dialect,
            ui_spec: ui_spec.clone(),
            initial_params: initial_params.clone(),
            post_processing: next_post_processing,
        };

        let save_result = save_or_update_agent_version_for_session(
            state,
            app,
            SaveOrUpdateAgentVersionRequest {
                session_id: ctx.session_id.clone(),
                thread_id: target.thread_id.clone(),
                base_message_id: target.message_id.clone(),
                model_id: Some(artifact_bundle.model_id.clone()),
                design_output: design_output.clone(),
                artifact_bundle: Some(artifact_bundle.clone()),
                model_manifest: Some(model_manifest.clone()),
                updated_at: now_secs(),
                response_text_created: String::new(),
                response_text_updated: String::new(),
                preserve_existing_title: true,
                preserve_existing_version_name: true,
                force_create_new_message: false,
                announce_created_working_version: false,
            },
        )
        .await?;
        tracked_message_id = Some(save_result.message_id.clone());

        Ok(MacroReplaceResponse {
            thread_id: target.thread_id,
            message_id: save_result.message_id,
            macro_code: req.macro_code.clone(),
            ui_spec,
            initial_params,
            artifact_bundle,
            model_manifest,
        })
    }
    .await;

    if let Err(err) = &result {
        let conn = state.db.lock().await;
        try_record_agent_error(
            state,
            &conn,
            ctx,
            tracked_thread_id,
            tracked_message_id,
            tracked_model_id,
            err,
        );
    }

    result
}

pub async fn handle_version_save(
    state: &AppState,
    app: &dyn PathResolver,
    req: VersionSaveRequest,
    ctx: &AgentContext,
) -> AppResult<VersionSaveResponse> {
    let ctx = ctx.with_override(&req.identity);
    let ctx = &ctx;
    let mut tracked_thread_id = req.thread_id.clone();
    let mut tracked_message_id = req.message_id.clone();
    let mut tracked_model_id = None;

    let result = async {
        let conn = state.db.lock().await;
        let target = crate::services::target::resolve_target(
            &conn,
            app,
            req.thread_id.clone(),
            req.message_id.clone(),
        )?;
        drop(conn);
        let target = resolve_turn_working_target(
            state,
            app,
            ctx,
            target,
            format!(
                "{} created a working version for this turn.",
                ctx.agent_label
            ),
        )
        .await?;
        let conn = state.db.lock().await;

        tracked_thread_id = Some(target.thread_id.clone());
        tracked_message_id = Some(target.message_id.clone());
        let mut design_output = target
            .design
            .clone()
            .ok_or_else(|| AppError::validation("Target has no design output."))?;
        let model_id = target
            .artifact_bundle
            .as_ref()
            .map(|bundle| bundle.model_id.clone());
        tracked_model_id = model_id.clone();

        persist_agent_session(
            &conn,
            ctx,
            tracked_thread_id.clone(),
            tracked_message_id.clone(),
            tracked_model_id.clone(),
            "saving_version",
            "",
        )?;

        drop(conn);
        if let Some(title) = req.title.clone() {
            design_output.title = title;
        }
        if let Some(version_name) = req.version_name.clone() {
            design_output.version_name = version_name;
        } else {
            design_output.version_name.clear();
        }

        let save_result = save_or_update_agent_version_for_session(
            state,
            app,
            SaveOrUpdateAgentVersionRequest {
                session_id: ctx.session_id.clone(),
                thread_id: target.thread_id.clone(),
                base_message_id: target.message_id.clone(),
                model_id: model_id.clone(),
                design_output,
                artifact_bundle: target.artifact_bundle.clone(),
                model_manifest: target.model_manifest.clone(),
                updated_at: now_secs(),
                response_text_created: String::new(),
                response_text_updated: String::new(),
                preserve_existing_title: req.title.is_none(),
                preserve_existing_version_name: req.version_name.is_none(),
                force_create_new_message: false,
                announce_created_working_version: false,
            },
        )
        .await?;
        tracked_message_id = Some(save_result.message_id.clone());
        tracked_model_id = save_result.model_id.clone();

        Ok(VersionSaveResponse {
            thread_id: target.thread_id,
            message_id: save_result.message_id,
            model_id: save_result.model_id.unwrap_or_default(),
        })
    }
    .await;

    if let Err(err) = &result {
        let conn = state.db.lock().await;
        try_record_agent_error(
            state,
            &conn,
            ctx,
            tracked_thread_id,
            tracked_message_id,
            tracked_model_id,
            err,
        );
    }

    result
}

pub async fn handle_version_restore(
    state: &AppState,
    req: VersionRestoreRequest,
    ctx: &AgentContext,
) -> AppResult<VersionRestoreResponse> {
    let ctx = ctx.with_override(&req.identity);
    let ctx = &ctx;
    let mut tracked_thread_id = None;
    let tracked_message_id = Some(req.message_id.clone());

    let result = async {
        let conn = state.db.lock().await;

        persist_agent_session(
            &conn,
            ctx,
            None,
            tracked_message_id.clone(),
            None,
            "restoring_version",
            "",
        )?;

        history::restore_version(&conn, &req.message_id)?;

        let tid = db::get_message_thread_id(&conn, &req.message_id)
            .map_err(|e| AppError::persistence(e.to_string()))?
            .ok_or_else(|| AppError::not_found("Restored message not found."))?;
        tracked_thread_id = Some(tid.clone());

        persist_agent_session(
            &conn,
            ctx,
            Some(tid.clone()),
            tracked_message_id.clone(),
            None,
            "idle",
            "",
        )?;

        Ok(VersionRestoreResponse {
            thread_id: tid,
            message_id: req.message_id.clone(),
        })
    }
    .await;

    if let Err(err) = &result {
        let conn = state.db.lock().await;
        try_record_agent_error(
            state,
            &conn,
            ctx,
            tracked_thread_id,
            tracked_message_id,
            None,
            err,
        );
    }

    result
}

pub async fn handle_thread_fork_from_target(
    state: &AppState,
    app: &dyn PathResolver,
    req: ThreadForkRequest,
    ctx: &AgentContext,
) -> AppResult<ThreadForkResponse> {
    let ctx = ctx.with_override(&req.identity);
    let ctx = &ctx;
    let mut tracked_thread_id = req.thread_id.clone();
    let mut tracked_message_id = req.message_id.clone();
    let mut tracked_model_id = None;

    let result = async {
        let conn = state.db.lock().await;
        let target = crate::services::target::resolve_target(
            &conn,
            app,
            req.thread_id.clone(),
            req.message_id.clone(),
        )?;

        tracked_thread_id = Some(target.thread_id.clone());
        tracked_message_id = Some(target.message_id.clone());

        let mut design_output = target
            .design
            .clone()
            .ok_or_else(|| AppError::validation("Target has no design output."))?;
        let model_id = target
            .artifact_bundle
            .as_ref()
            .map(|bundle| bundle.model_id.clone());
        tracked_model_id = model_id.clone();

        persist_agent_session(
            &conn,
            ctx,
            tracked_thread_id.clone(),
            tracked_message_id.clone(),
            tracked_model_id.clone(),
            "saving_version",
            "Forking target into a new thread.",
        )?;

        drop(conn);

        let new_thread_id = Uuid::new_v4().to_string();
        if let Some(title) = req.title.clone() {
            design_output.title = title;
        }
        if let Some(version_name) = req.version_name.clone() {
            design_output.version_name = version_name;
        } else {
            design_output.version_name.clear();
        }

        let save_result = save_or_update_agent_version_for_session(
            state,
            app,
            SaveOrUpdateAgentVersionRequest {
                session_id: ctx.session_id.clone(),
                thread_id: new_thread_id.clone(),
                base_message_id: target.message_id.clone(),
                model_id: model_id.clone(),
                design_output,
                artifact_bundle: target.artifact_bundle.clone(),
                model_manifest: target.model_manifest.clone(),
                updated_at: now_secs(),
                response_text_created: format!("{} forked this version via MCP.", ctx.agent_label),
                response_text_updated: format!(
                    "{} updated the forked MCP version.",
                    ctx.agent_label
                ),
                preserve_existing_title: false,
                preserve_existing_version_name: false,
                force_create_new_message: true,
                announce_created_working_version: false,
            },
        )
        .await?;
        tracked_message_id = Some(save_result.message_id.clone());
        tracked_model_id = save_result.model_id.clone();

        Ok(ThreadForkResponse {
            thread_id: new_thread_id,
            message_id: save_result.message_id,
            model_id: save_result.model_id.unwrap_or_default(),
        })
    }
    .await;

    if let Err(err) = &result {
        let conn = state.db.lock().await;
        try_record_agent_error(
            state,
            &conn,
            ctx,
            tracked_thread_id,
            tracked_message_id,
            tracked_model_id,
            err,
        );
    }

    result
}

#[derive(Debug, Clone)]
struct SemanticTargetState {
    thread_id: String,
    message_id: String,
    design_output: DesignOutput,
    artifact_bundle: ArtifactBundle,
    model_manifest: ModelManifest,
}

fn resolve_semantic_target(
    conn: &rusqlite::Connection,
    app: &dyn PathResolver,
    thread_id: Option<String>,
    message_id: Option<String>,
) -> AppResult<SemanticTargetState> {
    let target =
        crate::services::target::resolve_editable_target(conn, app, thread_id, message_id)?;
    let design_output = target.design_output;
    let artifact_bundle = target
        .artifact_bundle
        .ok_or_else(|| AppError::validation("Target has no artifact bundle."))?;
    let model_manifest = target
        .model_manifest
        .ok_or_else(|| AppError::validation("Target has no model manifest."))?;

    if model_manifest.source_kind != ModelSourceKind::Generated {
        return Err(AppError::validation(
            "Semantic knob/view MCP tools currently support generated models only.",
        ));
    }

    crate::models::validate_model_runtime_bundle(&model_manifest, &artifact_bundle)?;

    Ok(SemanticTargetState {
        thread_id: target.thread_id,
        message_id: target.message_id,
        design_output,
        artifact_bundle,
        model_manifest,
    })
}

#[allow(clippy::too_many_arguments)]
async fn save_semantic_manifest_version(
    state: &AppState,
    app: &dyn PathResolver,
    ctx: &AgentContext,
    target: SemanticTargetState,
    next_manifest: ModelManifest,
    title: Option<String>,
    version_name: Option<String>,
    response_text: String,
) -> AppResult<SemanticManifestMutationResponse> {
    crate::models::validate_model_runtime_bundle(&next_manifest, &target.artifact_bundle)?;

    let mut design_output = target.design_output.clone();
    if let Some(next_title) = title.clone() {
        design_output.title = next_title;
    }
    if let Some(next_version_name) = version_name.clone() {
        design_output.version_name = next_version_name;
    } else {
        design_output.version_name.clear();
    }

    let save_result = save_or_update_agent_version_for_session(
        state,
        app,
        SaveOrUpdateAgentVersionRequest {
            session_id: ctx.session_id.clone(),
            thread_id: target.thread_id.clone(),
            base_message_id: target.message_id.clone(),
            model_id: Some(target.artifact_bundle.model_id.clone()),
            design_output: design_output.clone(),
            artifact_bundle: Some(target.artifact_bundle.clone()),
            model_manifest: Some(next_manifest.clone()),
            updated_at: now_secs(),
            response_text_created: response_text.clone(),
            response_text_updated: response_text,
            preserve_existing_title: title.is_none(),
            preserve_existing_version_name: version_name.is_none(),
            force_create_new_message: false,
            announce_created_working_version: false,
        },
    )
    .await?;
    let agent_origin = save_result.agent_origin.clone();
    let artifact_bundle = target.artifact_bundle.clone();

    Ok(SemanticManifestMutationResponse {
        thread_id: target.thread_id,
        message_id: save_result.message_id,
        model_id: artifact_bundle.model_id.clone(),
        title: design_output.title,
        version_name: save_result.version_name,
        artifact_bundle,
        model_manifest: next_manifest,
        agent_origin,
    })
}

fn normalize_llm_primitive(
    primitive: ControlPrimitive,
    existing: Option<&ControlPrimitive>,
    manifest: &ModelManifest,
) -> AppResult<ControlPrimitive> {
    let primitive_id = primitive.primitive_id.trim();
    if primitive_id.is_empty() {
        return Err(AppError::validation("Primitive id cannot be empty."));
    }

    let order = if primitive.order == 0 {
        existing.map(|value| value.order).unwrap_or_else(|| {
            manifest
                .control_primitives
                .iter()
                .map(|entry| entry.order)
                .max()
                .unwrap_or(0)
                + 1
        })
    } else {
        primitive.order
    };

    Ok(ControlPrimitive {
        primitive_id: primitive_id.to_string(),
        label: primitive.label.trim().to_string(),
        kind: primitive.kind,
        source: ControlViewSource::Llm,
        part_ids: primitive.part_ids,
        bindings: primitive.bindings,
        editable: primitive.editable,
        order,
    })
}

fn normalize_llm_view(
    view: ControlView,
    existing: Option<&ControlView>,
    manifest: &ModelManifest,
) -> AppResult<ControlView> {
    let view_id = view.view_id.trim();
    if view_id.is_empty() {
        return Err(AppError::validation("View id cannot be empty."));
    }

    let order = if view.order == 0 {
        existing.map(|value| value.order).unwrap_or_else(|| {
            manifest
                .control_views
                .iter()
                .map(|entry| entry.order)
                .max()
                .unwrap_or(0)
                + 1
        })
    } else {
        view.order
    };

    Ok(ControlView {
        view_id: view_id.to_string(),
        label: view.label.trim().to_string(),
        scope: view.scope,
        part_ids: view.part_ids,
        primitive_ids: view.primitive_ids,
        sections: view.sections,
        is_default: view.is_default,
        source: ControlViewSource::Llm,
        status: view.status,
        order,
    })
}

fn normalize_llm_measurement_annotation(
    annotation: MeasurementAnnotation,
) -> AppResult<MeasurementAnnotation> {
    let annotation_id = annotation.annotation_id.trim();
    if annotation_id.is_empty() {
        return Err(AppError::validation(
            "Measurement annotation id cannot be empty.",
        ));
    }

    let label = annotation.label.trim();
    if label.is_empty() {
        return Err(AppError::validation(
            "Measurement annotation label cannot be empty.",
        ));
    }

    Ok(MeasurementAnnotation {
        annotation_id: annotation_id.to_string(),
        label: label.to_string(),
        basis: annotation.basis,
        axis: annotation.axis,
        parameter_keys: annotation.parameter_keys,
        primitive_ids: annotation.primitive_ids,
        target_ids: annotation.target_ids,
        guide_id: annotation.guide_id.and_then(|value| {
            let trimmed = value.trim().to_string();
            (!trimmed.is_empty()).then_some(trimmed)
        }),
        explanation: annotation.explanation.and_then(|value| {
            let trimmed = value.trim().to_string();
            (!trimmed.is_empty()).then_some(trimmed)
        }),
        formula_hint: annotation.formula_hint.and_then(|value| {
            let trimmed = value.trim().to_string();
            (!trimmed.is_empty()).then_some(trimmed)
        }),
        source: MeasurementAnnotationSource::Llm,
    })
}

pub async fn handle_semantic_manifest_get(
    state: &AppState,
    app: &dyn PathResolver,
    req: SemanticManifestRequest,
    ctx: &AgentContext,
) -> AppResult<SemanticManifestResponse> {
    let ctx = ctx.with_override(&req.identity);
    let ctx = &ctx;
    let conn = state.db.lock().await;
    let mut tracked_thread_id = req.thread_id.clone();
    let mut tracked_message_id = req.message_id.clone();
    let mut tracked_model_id = None;

    let result = (|| -> AppResult<SemanticManifestResponse> {
        persist_agent_session(
            &conn,
            ctx,
            tracked_thread_id.clone(),
            tracked_message_id.clone(),
            None,
            "reading",
            "Reading semantic manifest.",
        )?;

        let target =
            resolve_semantic_target(&conn, app, req.thread_id.clone(), req.message_id.clone())?;

        tracked_thread_id = Some(target.thread_id.clone());
        tracked_message_id = Some(target.message_id.clone());
        tracked_model_id = Some(target.artifact_bundle.model_id.clone());

        persist_agent_session(
            &conn,
            ctx,
            tracked_thread_id.clone(),
            tracked_message_id.clone(),
            tracked_model_id.clone(),
            "idle",
            "",
        )?;

        Ok(SemanticManifestResponse {
            thread_id: target.thread_id,
            message_id: target.message_id,
            title: Some(target.design_output.title),
            version_name: Some(target.design_output.version_name),
            artifact_bundle: target.artifact_bundle,
            model_manifest: target.model_manifest,
            latest_draft: None,
        })
    })();

    if let Err(err) = &result {
        try_record_agent_error(
            state,
            &conn,
            ctx,
            tracked_thread_id,
            tracked_message_id,
            tracked_model_id,
            err,
        );
    }

    result
}

pub async fn handle_control_primitive_save(
    state: &AppState,
    app: &dyn PathResolver,
    req: ControlPrimitiveSaveRequest,
    ctx: &AgentContext,
) -> AppResult<SemanticManifestMutationResponse> {
    let ctx = ctx.with_override(&req.identity);
    let ctx = &ctx;
    let mut tracked_thread_id = req.thread_id.clone();
    let mut tracked_message_id = req.message_id.clone();
    let mut tracked_model_id = None;

    let result = async {
        let conn = state.db.lock().await;
        let target =
            resolve_semantic_target(&conn, app, req.thread_id.clone(), req.message_id.clone())?;
        drop(conn);
        let target = resolve_turn_working_semantic_target(
            state,
            app,
            ctx,
            target,
            format!(
                "{} created a working version for this turn.",
                ctx.agent_label
            ),
        )
        .await?;
        let conn = state.db.lock().await;
        tracked_thread_id = Some(target.thread_id.clone());
        tracked_message_id = Some(target.message_id.clone());
        tracked_model_id = Some(target.artifact_bundle.model_id.clone());

        persist_agent_session(
            &conn,
            ctx,
            tracked_thread_id.clone(),
            tracked_message_id.clone(),
            tracked_model_id.clone(),
            "saving_version",
            "Saving semantic knob.",
        )?;

        let existing = target
            .model_manifest
            .control_primitives
            .iter()
            .find(|entry| entry.primitive_id == req.primitive.primitive_id);
        let next_primitive =
            normalize_llm_primitive(req.primitive, existing, &target.model_manifest)?;
        let next_primitive_id = next_primitive.primitive_id.clone();
        let mut next_manifest = target.model_manifest.clone();
        next_manifest.control_primitives = next_manifest
            .control_primitives
            .into_iter()
            .filter(|entry| entry.primitive_id != next_primitive_id)
            .chain(std::iter::once(next_primitive))
            .collect();
        next_manifest.control_primitives.sort_by(|left, right| {
            left.order
                .cmp(&right.order)
                .then_with(|| left.label.cmp(&right.label))
        });

        drop(conn);

        let response = save_semantic_manifest_version(
            state,
            app,
            ctx,
            target,
            next_manifest,
            req.title,
            req.version_name,
            format!("{} updated a semantic knob via MCP.", ctx.agent_label),
        )
        .await?;
        tracked_message_id = Some(response.message_id.clone());
        tracked_model_id = Some(response.model_id.clone());

        Ok(response)
    }
    .await;

    if let Err(err) = &result {
        let conn = state.db.lock().await;
        try_record_agent_error(
            state,
            &conn,
            ctx,
            tracked_thread_id,
            tracked_message_id,
            tracked_model_id,
            err,
        );
    }

    result
}

pub async fn handle_control_primitive_delete(
    state: &AppState,
    app: &dyn PathResolver,
    req: ControlPrimitiveDeleteRequest,
    ctx: &AgentContext,
) -> AppResult<SemanticManifestMutationResponse> {
    let ctx = ctx.with_override(&req.identity);
    let ctx = &ctx;
    let mut tracked_thread_id = req.thread_id.clone();
    let mut tracked_message_id = req.message_id.clone();
    let mut tracked_model_id = None;

    let result = async {
        let conn = state.db.lock().await;
        let target =
            resolve_semantic_target(&conn, app, req.thread_id.clone(), req.message_id.clone())?;
        drop(conn);
        let target = resolve_turn_working_semantic_target(
            state,
            app,
            ctx,
            target,
            format!(
                "{} created a working version for this turn.",
                ctx.agent_label
            ),
        )
        .await?;
        let conn = state.db.lock().await;
        tracked_thread_id = Some(target.thread_id.clone());
        tracked_message_id = Some(target.message_id.clone());
        tracked_model_id = Some(target.artifact_bundle.model_id.clone());

        persist_agent_session(
            &conn,
            ctx,
            tracked_thread_id.clone(),
            tracked_message_id.clone(),
            tracked_model_id.clone(),
            "saving_version",
            "Deleting semantic knob.",
        )?;

        let mut next_manifest = target.model_manifest.clone();
        let primitive_id = req.primitive_id;
        next_manifest
            .control_primitives
            .retain(|entry| entry.primitive_id != primitive_id);
        next_manifest.control_relations.retain(|relation| {
            relation.source_primitive_id != primitive_id
                && relation.target_primitive_id != primitive_id
        });
        for view in &mut next_manifest.control_views {
            view.primitive_ids.retain(|entry| entry != &primitive_id);
            for section in &mut view.sections {
                section.primitive_ids.retain(|entry| entry != &primitive_id);
            }
        }
        for advisory in &mut next_manifest.advisories {
            advisory
                .primitive_ids
                .retain(|entry| entry != &primitive_id);
        }

        drop(conn);

        let response = save_semantic_manifest_version(
            state,
            app,
            ctx,
            target,
            next_manifest,
            req.title,
            req.version_name,
            format!("{} deleted a semantic knob via MCP.", ctx.agent_label),
        )
        .await?;
        tracked_message_id = Some(response.message_id.clone());
        tracked_model_id = Some(response.model_id.clone());

        Ok(response)
    }
    .await;

    if let Err(err) = &result {
        let conn = state.db.lock().await;
        try_record_agent_error(
            state,
            &conn,
            ctx,
            tracked_thread_id,
            tracked_message_id,
            tracked_model_id,
            err,
        );
    }

    result
}

pub async fn handle_control_view_save(
    state: &AppState,
    app: &dyn PathResolver,
    req: ControlViewSaveRequest,
    ctx: &AgentContext,
) -> AppResult<SemanticManifestMutationResponse> {
    let ctx = ctx.with_override(&req.identity);
    let ctx = &ctx;
    let mut tracked_thread_id = req.thread_id.clone();
    let mut tracked_message_id = req.message_id.clone();
    let mut tracked_model_id = None;

    let result = async {
        let conn = state.db.lock().await;
        let target =
            resolve_semantic_target(&conn, app, req.thread_id.clone(), req.message_id.clone())?;
        drop(conn);
        let target = resolve_turn_working_semantic_target(
            state,
            app,
            ctx,
            target,
            format!(
                "{} created a working version for this turn.",
                ctx.agent_label
            ),
        )
        .await?;
        let conn = state.db.lock().await;
        tracked_thread_id = Some(target.thread_id.clone());
        tracked_message_id = Some(target.message_id.clone());
        tracked_model_id = Some(target.artifact_bundle.model_id.clone());

        persist_agent_session(
            &conn,
            ctx,
            tracked_thread_id.clone(),
            tracked_message_id.clone(),
            tracked_model_id.clone(),
            "saving_version",
            "Saving semantic view.",
        )?;

        let existing = target
            .model_manifest
            .control_views
            .iter()
            .find(|entry| entry.view_id == req.view.view_id);
        let next_view = normalize_llm_view(req.view, existing, &target.model_manifest)?;
        let next_view_id = next_view.view_id.clone();
        let mut next_manifest = target.model_manifest.clone();
        next_manifest.control_views = next_manifest
            .control_views
            .into_iter()
            .filter(|entry| entry.view_id != next_view_id)
            .chain(std::iter::once(next_view))
            .collect();
        next_manifest.control_views.sort_by(|left, right| {
            left.order
                .cmp(&right.order)
                .then_with(|| left.label.cmp(&right.label))
        });

        drop(conn);

        let response = save_semantic_manifest_version(
            state,
            app,
            ctx,
            target,
            next_manifest,
            req.title,
            req.version_name,
            format!("{} updated a semantic view via MCP.", ctx.agent_label),
        )
        .await?;
        tracked_message_id = Some(response.message_id.clone());
        tracked_model_id = Some(response.model_id.clone());

        Ok(response)
    }
    .await;

    if let Err(err) = &result {
        let conn = state.db.lock().await;
        try_record_agent_error(
            state,
            &conn,
            ctx,
            tracked_thread_id,
            tracked_message_id,
            tracked_model_id,
            err,
        );
    }

    result
}

pub async fn handle_control_view_delete(
    state: &AppState,
    app: &dyn PathResolver,
    req: ControlViewDeleteRequest,
    ctx: &AgentContext,
) -> AppResult<SemanticManifestMutationResponse> {
    let ctx = ctx.with_override(&req.identity);
    let ctx = &ctx;
    let mut tracked_thread_id = req.thread_id.clone();
    let mut tracked_message_id = req.message_id.clone();
    let mut tracked_model_id = None;

    let result = async {
        let conn = state.db.lock().await;
        let target =
            resolve_semantic_target(&conn, app, req.thread_id.clone(), req.message_id.clone())?;
        drop(conn);
        let target = resolve_turn_working_semantic_target(
            state,
            app,
            ctx,
            target,
            format!(
                "{} created a working version for this turn.",
                ctx.agent_label
            ),
        )
        .await?;
        let conn = state.db.lock().await;
        tracked_thread_id = Some(target.thread_id.clone());
        tracked_message_id = Some(target.message_id.clone());
        tracked_model_id = Some(target.artifact_bundle.model_id.clone());

        persist_agent_session(
            &conn,
            ctx,
            tracked_thread_id.clone(),
            tracked_message_id.clone(),
            tracked_model_id.clone(),
            "saving_version",
            "Deleting semantic view.",
        )?;

        let mut next_manifest = target.model_manifest.clone();
        let view_id = req.view_id;
        next_manifest
            .control_views
            .retain(|entry| entry.view_id != view_id);
        for advisory in &mut next_manifest.advisories {
            advisory.view_ids.retain(|entry| entry != &view_id);
        }

        drop(conn);

        let response = save_semantic_manifest_version(
            state,
            app,
            ctx,
            target,
            next_manifest,
            req.title,
            req.version_name,
            format!("{} deleted a semantic view via MCP.", ctx.agent_label),
        )
        .await?;
        tracked_message_id = Some(response.message_id.clone());
        tracked_model_id = Some(response.model_id.clone());

        Ok(response)
    }
    .await;

    if let Err(err) = &result {
        let conn = state.db.lock().await;
        try_record_agent_error(
            state,
            &conn,
            ctx,
            tracked_thread_id,
            tracked_message_id,
            tracked_model_id,
            err,
        );
    }

    result
}

pub async fn handle_measurement_annotation_save(
    state: &AppState,
    app: &dyn PathResolver,
    req: MeasurementAnnotationSaveRequest,
    ctx: &AgentContext,
) -> AppResult<SemanticManifestMutationResponse> {
    let ctx = ctx.with_override(&req.identity);
    let ctx = &ctx;
    let mut tracked_thread_id = req.thread_id.clone();
    let mut tracked_message_id = req.message_id.clone();
    let mut tracked_model_id = None;

    let result = async {
        let conn = state.db.lock().await;
        let target =
            resolve_semantic_target(&conn, app, req.thread_id.clone(), req.message_id.clone())?;
        drop(conn);
        let target = resolve_turn_working_semantic_target(
            state,
            app,
            ctx,
            target,
            format!(
                "{} created a working version for this turn.",
                ctx.agent_label
            ),
        )
        .await?;
        let conn = state.db.lock().await;
        tracked_thread_id = Some(target.thread_id.clone());
        tracked_message_id = Some(target.message_id.clone());
        tracked_model_id = Some(target.artifact_bundle.model_id.clone());

        persist_agent_session(
            &conn,
            ctx,
            tracked_thread_id.clone(),
            tracked_message_id.clone(),
            tracked_model_id.clone(),
            "saving_version",
            "Saving measurement annotation.",
        )?;

        let next_annotation = normalize_llm_measurement_annotation(req.annotation)?;
        let next_annotation_id = next_annotation.annotation_id.clone();
        let mut next_manifest = target.model_manifest.clone();
        if let Some(existing) = next_manifest
            .measurement_annotations
            .iter_mut()
            .find(|entry| entry.annotation_id == next_annotation_id)
        {
            *existing = next_annotation;
        } else {
            next_manifest.measurement_annotations.push(next_annotation);
        }

        drop(conn);

        let response = save_semantic_manifest_version(
            state,
            app,
            ctx,
            target,
            next_manifest,
            req.title,
            req.version_name,
            format!(
                "{} updated a measurement annotation via MCP.",
                ctx.agent_label
            ),
        )
        .await?;
        tracked_message_id = Some(response.message_id.clone());
        tracked_model_id = Some(response.model_id.clone());

        Ok(response)
    }
    .await;

    if let Err(err) = &result {
        let conn = state.db.lock().await;
        try_record_agent_error(
            state,
            &conn,
            ctx,
            tracked_thread_id,
            tracked_message_id,
            tracked_model_id,
            err,
        );
    }

    result
}

pub async fn handle_measurement_annotation_delete(
    state: &AppState,
    app: &dyn PathResolver,
    req: MeasurementAnnotationDeleteRequest,
    ctx: &AgentContext,
) -> AppResult<SemanticManifestMutationResponse> {
    let ctx = ctx.with_override(&req.identity);
    let ctx = &ctx;
    let mut tracked_thread_id = req.thread_id.clone();
    let mut tracked_message_id = req.message_id.clone();
    let mut tracked_model_id = None;

    let result = async {
        let conn = state.db.lock().await;
        let target =
            resolve_semantic_target(&conn, app, req.thread_id.clone(), req.message_id.clone())?;
        tracked_thread_id = Some(target.thread_id.clone());
        tracked_message_id = Some(target.message_id.clone());
        tracked_model_id = Some(target.artifact_bundle.model_id.clone());

        persist_agent_session(
            &conn,
            ctx,
            tracked_thread_id.clone(),
            tracked_message_id.clone(),
            tracked_model_id.clone(),
            "saving_version",
            "Deleting measurement annotation.",
        )?;

        let mut next_manifest = target.model_manifest.clone();
        let annotation_id = req.annotation_id;
        next_manifest
            .measurement_annotations
            .retain(|entry| entry.annotation_id != annotation_id);

        drop(conn);

        let response = save_semantic_manifest_version(
            state,
            app,
            ctx,
            target,
            next_manifest,
            req.title,
            req.version_name,
            format!(
                "{} deleted a measurement annotation via MCP.",
                ctx.agent_label
            ),
        )
        .await?;
        tracked_message_id = Some(response.message_id.clone());
        tracked_model_id = Some(response.model_id.clone());

        Ok(response)
    }
    .await;

    if let Err(err) = &result {
        let conn = state.db.lock().await;
        try_record_agent_error(
            state,
            &conn,
            ctx,
            tracked_thread_id,
            tracked_message_id,
            tracked_model_id,
            err,
        );
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::{
        Config, ControlPrimitiveKind, ControlRelationMode, ControlViewScope, DesignParams,
        DocumentMetadata, EnrichmentStatus, McpConfig, MeasurementAnnotation,
        MeasurementAnnotationSource, MeasurementAxis, MeasurementBasis, Message, MessageRole,
        MessageStatus, ParamValue, UiField,
    };
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    struct TestPathResolver {
        root: PathBuf,
    }

    impl PathResolver for TestPathResolver {
        fn app_config_dir(&self) -> PathBuf {
            self.root.clone()
        }

        fn app_data_dir(&self) -> PathBuf {
            self.root.clone()
        }

        fn resource_path(&self, path: &str) -> Option<PathBuf> {
            Some(self.root.join(path))
        }
    }

    fn test_db_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("ecky-mcp-{}-{}", name, Uuid::new_v4()))
    }

    fn test_config() -> Config {
        Config {
            engines: Vec::new(),
            selected_engine_id: String::new(),
            freecad_cmd: String::new(),
            assets: Vec::new(),
            microwave: None,
            mcp: McpConfig::default(),
            has_seen_onboarding: true,
            connection_type: None,
        }
    }

    fn test_ctx() -> AgentContext {
        AgentContext {
            session_id: "session-1".to_string(),
            client_kind: "http".to_string(),
            host_label: "Claude Code".to_string(),
            agent_label: "claude".to_string(),
            llm_model_id: None,
            llm_model_label: Some("Claude Sonnet".to_string()),
        }
    }

    fn sample_ui_spec() -> UiSpec {
        UiSpec {
            fields: vec![
                UiField::Range {
                    key: "diameter".to_string(),
                    label: "Diameter".to_string(),
                    min: Some(10.0),
                    max: Some(200.0),
                    step: Some(1.0),
                    min_from: None,
                    max_from: None,
                    frozen: false,
                },
                UiField::Select {
                    key: "mount".to_string(),
                    label: "Mount".to_string(),
                    options: vec![crate::contracts::SelectOption {
                        label: "Inner".to_string(),
                        value: crate::contracts::SelectValue::String("inner".to_string()),
                    }],
                    frozen: false,
                },
                UiField::Checkbox {
                    key: "lip".to_string(),
                    label: "Lip".to_string(),
                    frozen: false,
                },
            ],
        }
    }

    fn sample_params() -> DesignParams {
        BTreeMap::from([
            ("diameter".to_string(), ParamValue::Number(130.0)),
            ("mount".to_string(), ParamValue::String("inner".to_string())),
            ("lip".to_string(), ParamValue::Boolean(true)),
        ])
    }

    fn sample_design(title: &str, version_name: &str, macro_code: &str) -> DesignOutput {
        DesignOutput {
            title: title.to_string(),
            version_name: version_name.to_string(),
            response: "ok".to_string(),
            interaction_mode: InteractionMode::Design,
            macro_code: macro_code.to_string(),
            macro_dialect: MacroDialect::Legacy,
            ui_spec: sample_ui_spec(),
            initial_params: sample_params(),
            post_processing: Some(crate::contracts::PostProcessingSpec {
                displacement: None,
                lithophane_attachments: vec![],
            }),
        }
    }

    fn sample_bundle(model_id: &str, preview_name: &str) -> ArtifactBundle {
        ArtifactBundle {
            schema_version: crate::contracts::MODEL_RUNTIME_SCHEMA_VERSION,
            model_id: model_id.to_string(),
            source_kind: ModelSourceKind::Generated,
            content_hash: format!("hash-{}", model_id),
            artifact_version: 1,
            fcstd_path: format!("/tmp/{}.FCStd", model_id),
            manifest_path: format!("/tmp/{}.json", model_id),
            macro_path: Some(format!("/tmp/{}.py", model_id)),
            preview_stl_path: format!("/tmp/{}", preview_name),
            viewer_assets: Vec::new(),
            edge_targets: Vec::new(),
            callout_anchors: Vec::new(),
            measurement_guides: Vec::new(),
            export_artifacts: Vec::new(),
        }
    }

    fn sample_manifest(model_id: &str) -> ModelManifest {
        ModelManifest {
            schema_version: crate::contracts::MODEL_RUNTIME_SCHEMA_VERSION,
            model_id: model_id.to_string(),
            source_kind: ModelSourceKind::Generated,
            document: DocumentMetadata {
                document_name: "Doc".to_string(),
                document_label: "Doc".to_string(),
                source_path: None,
                object_count: 1,
                warnings: Vec::new(),
            },
            parts: Vec::new(),
            parameter_groups: Vec::new(),
            control_primitives: vec![
                ControlPrimitive {
                    primitive_id: "diameter".to_string(),
                    label: "Diameter".to_string(),
                    kind: ControlPrimitiveKind::Number,
                    source: ControlViewSource::Llm,
                    part_ids: Vec::new(),
                    bindings: vec![crate::contracts::PrimitiveBinding {
                        parameter_key: "diameter".to_string(),
                        scale: 1.0,
                        offset: 0.0,
                        min: None,
                        max: None,
                    }],
                    editable: true,
                    order: 1,
                },
                ControlPrimitive {
                    primitive_id: "lip".to_string(),
                    label: "Lip".to_string(),
                    kind: ControlPrimitiveKind::Toggle,
                    source: ControlViewSource::Llm,
                    part_ids: Vec::new(),
                    bindings: vec![crate::contracts::PrimitiveBinding {
                        parameter_key: "lip".to_string(),
                        scale: 1.0,
                        offset: 0.0,
                        min: None,
                        max: None,
                    }],
                    editable: true,
                    order: 2,
                },
            ],
            control_relations: vec![crate::contracts::ControlRelation {
                relation_id: "rel-1".to_string(),
                source_primitive_id: "diameter".to_string(),
                target_primitive_id: "lip".to_string(),
                mode: ControlRelationMode::Mirror,
                scale: 1.0,
                offset: 0.0,
                enabled: true,
            }],
            control_views: vec![ControlView {
                view_id: "main".to_string(),
                label: "Main".to_string(),
                scope: ControlViewScope::Global,
                part_ids: Vec::new(),
                primitive_ids: vec!["diameter".to_string(), "lip".to_string()],
                sections: Vec::new(),
                is_default: true,
                source: ControlViewSource::Llm,
                status: EnrichmentStatus::Accepted,
                order: 1,
            }],
            advisories: Vec::new(),
            selection_targets: Vec::new(),
            measurement_annotations: Vec::new(),
            warnings: Vec::new(),
            enrichment_state: crate::contracts::ManifestEnrichmentState {
                status: EnrichmentStatus::None,
                proposals: Vec::new(),
            },
        }
    }

    async fn seed_target() -> (AppState, TestPathResolver) {
        let root = std::env::temp_dir().join(format!("ecky-mcp-root-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let conn = crate::db::init_db(&test_db_path("target-read")).expect("db");
        let state = AppState::new(test_config(), None, conn);
        let resolver = TestPathResolver { root };
        let now = now_secs();

        let base_bundle = sample_bundle("model-base", "base.stl");
        let base_manifest = sample_manifest("model-base");
        let base_design = sample_design("Base Pot", "V-base", "base_macro()");

        {
            let conn = state.db.lock().await;
            db::create_or_update_thread(&conn, "thread-1", "Thread", now, None).unwrap();
            db::add_message(
                &conn,
                "thread-1",
                &Message {
                    id: "msg-1".to_string(),
                    role: MessageRole::Assistant,
                    content: "Base version".to_string(),
                    status: MessageStatus::Success,
                    output: Some(base_design),
                    usage: None,
                    artifact_bundle: Some(base_bundle),
                    model_manifest: Some(base_manifest),
                    agent_origin: None,
                    image_data: None,
                    visual_kind: None,
                    attachment_images: Vec::new(),
                    timestamp: now,
                },
            )
            .unwrap();
        }

        (state, resolver)
    }

    async fn seed_live_session(state: &AppState) {
        state.mcp_sessions.lock().await.insert(
            "session-1".to_string(),
            crate::models::McpSessionState {
                client_kind: "mcp-http".to_string(),
                host_label: "Claude Code".to_string(),
                agent_label: "claude".to_string(),
                llm_model_id: None,
                llm_model_label: Some("Claude Sonnet".to_string()),
                bound_thread_id: None,
                last_target: Some(session_target_ref(
                    "thread-1".to_string(),
                    "msg-1".to_string(),
                    Some("model-base".to_string()),
                )),
                phase: Some("idle".to_string()),
                status_text: Some("Agent joined the workspace.".to_string()),
                busy: false,
                activity_label: None,
                activity_started_at: None,
                attention_kind: None,
                waiting_on_prompt: false,
                current_turn_id: None,
                current_turn_thread_id: None,
                current_turn_working_message_id: None,
                updated_at: now_secs(),
            },
        );
    }

    #[tokio::test]
    async fn resolve_prompt_thread_context_returns_bound_thread_identity() {
        let (state, _resolver) = seed_target().await;

        let (thread_id, thread_title) = resolve_prompt_thread_context(
            &state,
            Some(&agent_dialogue::SessionThreadTarget {
                thread_id: "thread-1".to_string(),
                message_id: Some("msg-1".to_string()),
                model_id: Some("model-base".to_string()),
            }),
        )
        .await
        .expect("prompt thread context");

        assert_eq!(thread_id.as_deref(), Some("thread-1"));
        assert_eq!(thread_title.as_deref(), Some("Thread"));
    }

    #[tokio::test]
    async fn target_meta_get_returns_lightweight_summary_without_heavy_fields() {
        let (state, resolver) = seed_target().await;
        let response = handle_target_meta_get(
            &state,
            &resolver,
            TargetMetaRequest {
                identity: AgentIdentityOverride::default(),
                thread_id: Some("thread-1".to_string()),
                message_id: Some("msg-1".to_string()),
            },
            &test_ctx(),
        )
        .await
        .expect("target meta");

        assert_eq!(response.resolved_from, TargetResolvedFrom::Base);
        assert_eq!(response.model_id.as_deref(), Some("model-base"));
        assert!(!response.has_draft);
        assert_eq!(response.ui_field_count, 3);
        assert_eq!(response.range_count, 1);
        assert_eq!(response.select_count, 1);
        assert_eq!(response.checkbox_count, 1);
        assert_eq!(response.parameter_count, 3);
        assert!(response.has_semantic_manifest);
        assert_eq!(response.control_primitive_count, 2);
        assert_eq!(response.control_relation_count, 1);
        assert_eq!(response.control_view_count, 1);
        assert!(
            response
                .cad_sdk_snippet
                .as_deref()
                .is_some_and(|snippet| snippet.contains("class ControlRegistry")),
            "target meta should surface cad_sdk.py helpers for MCP agents"
        );

        let value = serde_json::to_value(&response).unwrap();
        assert!(value.get("macroCode").is_none());
        assert!(value.get("artifactBundle").is_none());
        assert!(value.get("modelManifest").is_none());
        assert!(value.get("latestDraft").is_none());
    }

    #[tokio::test]
    async fn managed_session_log_in_fails_fast_without_a_bound_target() {
        let conn = crate::db::init_db(&test_db_path("managed-session-login-target")).expect("db");
        let mut config = test_config();
        config.connection_type = Some("mcp".to_string());
        config.mcp.mode = crate::contracts::McpMode::Active;
        config.mcp.primary_agent_id = Some("agent-1".to_string());
        config.mcp.auto_agents = vec![crate::contracts::AutoAgent {
            id: "agent-1".to_string(),
            label: "claude".to_string(),
            cmd: "claude".to_string(),
            model: None,
            args: Vec::new(),
            enabled: true,
            start_on_demand: true,
        }];
        let state = AppState::new(config, None, conn);
        crate::mcp::runtime::initialize_auto_agent_supervisors(state.clone());
        crate::mcp::runtime::bind_managed_http_session(
            &state,
            "agent-1",
            "session-1",
            Some("Connected to Ecky.".to_string()),
        );

        let err = handle_session_log_in(
            &state,
            SessionLoginRequest {
                identity: AgentIdentityOverride::default(),
                thread_id: None,
                message_id: None,
                model_id: None,
            },
            &test_ctx(),
        )
        .await
        .expect_err("managed session should fail without a bound target");

        assert!(
            err.message.contains("no bound thread target"),
            "unexpected error: {}",
            err.message
        );

        let logs = state.app_logs.lock().unwrap();
        let last = logs.back().expect("log entry");
        assert!(last.message.contains("[MCP]"));
        assert!(last.message.contains("kind=session_bind_failed"));
        assert!(last.message.contains("could not bind"));
    }

    #[tokio::test]
    async fn target_macro_get_returns_active_macro_payload() {
        let (state, resolver) = seed_target().await;
        let response = handle_target_macro_get(
            &state,
            &resolver,
            TargetMacroRequest {
                identity: AgentIdentityOverride::default(),
                thread_id: Some("thread-1".to_string()),
                message_id: Some("msg-1".to_string()),
            },
            &test_ctx(),
        )
        .await
        .expect("target macro");

        assert_eq!(response.thread_id, "thread-1");
        assert_eq!(response.message_id, "msg-1");
        assert_eq!(response.title, "Base Pot");
        assert_eq!(response.version_name, "V-base");
        assert_eq!(response.resolved_from, TargetResolvedFrom::Base);
        assert_eq!(response.macro_code, "base_macro()");
        assert_eq!(response.macro_dialect, MacroDialect::Legacy);
        assert!(response.post_processing.is_some());
    }

    #[tokio::test]
    async fn target_detail_get_returns_requested_ui_spec_only() {
        let (state, resolver) = seed_target().await;
        let response = handle_target_detail_get(
            &state,
            &resolver,
            TargetDetailRequest {
                identity: AgentIdentityOverride::default(),
                thread_id: Some("thread-1".to_string()),
                message_id: Some("msg-1".to_string()),
                section: TargetDetailSection::UiSpec,
            },
            &test_ctx(),
        )
        .await
        .expect("target uiSpec detail");

        let value = serde_json::to_value(&response).unwrap();
        assert_eq!(value["section"], "uiSpec");
        assert!(value.get("uiSpec").is_some());
        assert!(value.get("initialParams").is_none());
        assert!(value.get("artifactBundle").is_none());
        assert!(value.get("latestDraft").is_none());
    }

    #[tokio::test]
    async fn target_detail_get_returns_requested_initial_params_only() {
        let (state, resolver) = seed_target().await;
        let response = handle_target_detail_get(
            &state,
            &resolver,
            TargetDetailRequest {
                identity: AgentIdentityOverride::default(),
                thread_id: Some("thread-1".to_string()),
                message_id: Some("msg-1".to_string()),
                section: TargetDetailSection::InitialParams,
            },
            &test_ctx(),
        )
        .await
        .expect("target params detail");

        let value = serde_json::to_value(&response).unwrap();
        assert_eq!(value["section"], "initialParams");
        assert_eq!(value["initialParams"]["diameter"], 130.0);
        assert!(value.get("uiSpec").is_none());
        assert!(value.get("artifactBundle").is_none());
        assert!(value.get("latestDraft").is_none());
    }

    #[tokio::test]
    async fn target_detail_get_returns_active_artifact_bundle_only() {
        let (state, resolver) = seed_target().await;
        let response = handle_target_detail_get(
            &state,
            &resolver,
            TargetDetailRequest {
                identity: AgentIdentityOverride::default(),
                thread_id: Some("thread-1".to_string()),
                message_id: Some("msg-1".to_string()),
                section: TargetDetailSection::ArtifactBundle,
            },
            &test_ctx(),
        )
        .await
        .expect("target artifact detail");

        let value = serde_json::to_value(&response).unwrap();
        assert_eq!(value["section"], "artifactBundle");
        assert_eq!(value["artifactBundle"]["modelId"], "model-base");
        assert_eq!(value["artifactBundle"]["previewStlPath"], "/tmp/base.stl");
        assert!(value.get("uiSpec").is_none());
        assert!(value.get("initialParams").is_none());
        assert!(value.get("latestDraft").is_none());
    }

    #[tokio::test]
    async fn target_detail_get_returns_latest_draft_null_for_compatibility() {
        let (state, resolver) = seed_target().await;
        let response = handle_target_detail_get(
            &state,
            &resolver,
            TargetDetailRequest {
                identity: AgentIdentityOverride::default(),
                thread_id: Some("thread-1".to_string()),
                message_id: Some("msg-1".to_string()),
                section: TargetDetailSection::LatestDraft,
            },
            &test_ctx(),
        )
        .await
        .expect("target draft detail");

        let value = serde_json::to_value(&response).unwrap();
        assert_eq!(value["section"], "latestDraft");
        assert!(value.get("latestDraft").is_some());
        assert!(value["latestDraft"].is_null());
        assert!(value.get("uiSpec").is_none());
        assert!(value.get("initialParams").is_none());
        assert!(value.get("artifactBundle").is_none());
    }

    #[tokio::test]
    async fn target_detail_get_returns_latest_draft_null_when_absent() {
        let (state, resolver) = seed_target().await;
        let response = handle_target_detail_get(
            &state,
            &resolver,
            TargetDetailRequest {
                identity: AgentIdentityOverride::default(),
                thread_id: Some("thread-1".to_string()),
                message_id: Some("msg-1".to_string()),
                section: TargetDetailSection::LatestDraft,
            },
            &test_ctx(),
        )
        .await
        .expect("target null draft detail");

        let value = serde_json::to_value(&response).unwrap();
        assert_eq!(value["section"], "latestDraft");
        assert!(value.get("latestDraft").is_some());
        assert!(value["latestDraft"].is_null());
    }

    #[tokio::test]
    async fn measurement_annotation_save_persists_semantic_annotation_in_new_version() {
        let (state, resolver) = seed_target().await;
        let response = handle_measurement_annotation_save(
            &state,
            &resolver,
            MeasurementAnnotationSaveRequest {
                identity: AgentIdentityOverride::default(),
                thread_id: Some("thread-1".to_string()),
                message_id: Some("msg-1".to_string()),
                annotation: MeasurementAnnotation {
                    annotation_id: "measurement-outer-diameter".to_string(),
                    label: "Outer Diameter".to_string(),
                    basis: MeasurementBasis::Outer,
                    axis: MeasurementAxis::X,
                    parameter_keys: vec!["diameter".to_string()],
                    primitive_ids: vec!["diameter".to_string()],
                    target_ids: Vec::new(),
                    guide_id: None,
                    explanation: Some("Measures the outside width.".to_string()),
                    formula_hint: None,
                    source: MeasurementAnnotationSource::Manual,
                },
                title: None,
                version_name: Some("V-mcp-measurement".to_string()),
            },
            &test_ctx(),
        )
        .await
        .expect("measurement annotation save");

        assert_eq!(response.version_name, "V-mcp-measurement");
        assert_eq!(response.model_manifest.measurement_annotations.len(), 1);
        assert_eq!(
            response.model_manifest.measurement_annotations[0].source,
            MeasurementAnnotationSource::Llm
        );
        assert_eq!(
            response.model_manifest.measurement_annotations[0].annotation_id,
            "measurement-outer-diameter"
        );
    }

    #[tokio::test]
    async fn semantic_manifest_get_includes_measurement_annotations() {
        let (state, resolver) = seed_target().await;

        let created = handle_measurement_annotation_save(
            &state,
            &resolver,
            MeasurementAnnotationSaveRequest {
                identity: AgentIdentityOverride::default(),
                thread_id: Some("thread-1".to_string()),
                message_id: Some("msg-1".to_string()),
                annotation: MeasurementAnnotation {
                    annotation_id: "measurement-inner-width".to_string(),
                    label: "Inner Width".to_string(),
                    basis: MeasurementBasis::Inner,
                    axis: MeasurementAxis::X,
                    parameter_keys: vec!["diameter".to_string()],
                    primitive_ids: vec!["diameter".to_string()],
                    target_ids: Vec::new(),
                    guide_id: None,
                    explanation: Some("Measures the inner cavity span.".to_string()),
                    formula_hint: None,
                    source: MeasurementAnnotationSource::Manual,
                },
                title: None,
                version_name: None,
            },
            &test_ctx(),
        )
        .await
        .expect("seed measurement annotation");

        let response = handle_semantic_manifest_get(
            &state,
            &resolver,
            SemanticManifestRequest {
                identity: AgentIdentityOverride::default(),
                thread_id: Some(created.thread_id.clone()),
                message_id: Some(created.message_id.clone()),
            },
            &test_ctx(),
        )
        .await
        .expect("semantic manifest with measurements");

        assert_eq!(response.model_manifest.measurement_annotations.len(), 1);
        assert_eq!(
            response.model_manifest.measurement_annotations[0].annotation_id,
            "measurement-inner-width"
        );
        assert_eq!(
            response.model_manifest.measurement_annotations[0].basis,
            MeasurementBasis::Inner
        );
    }

    #[tokio::test]
    async fn measurement_annotation_delete_removes_existing_annotation() {
        let (state, resolver) = seed_target().await;

        let created = handle_measurement_annotation_save(
            &state,
            &resolver,
            MeasurementAnnotationSaveRequest {
                identity: AgentIdentityOverride::default(),
                thread_id: Some("thread-1".to_string()),
                message_id: Some("msg-1".to_string()),
                annotation: MeasurementAnnotation {
                    annotation_id: "measurement-wall".to_string(),
                    label: "Wall Thickness".to_string(),
                    basis: MeasurementBasis::Wall,
                    axis: MeasurementAxis::Normal,
                    parameter_keys: vec!["diameter".to_string()],
                    primitive_ids: vec!["diameter".to_string()],
                    target_ids: Vec::new(),
                    guide_id: None,
                    explanation: None,
                    formula_hint: None,
                    source: MeasurementAnnotationSource::Manual,
                },
                title: None,
                version_name: None,
            },
            &test_ctx(),
        )
        .await
        .expect("seed annotation");

        let deleted = handle_measurement_annotation_delete(
            &state,
            &resolver,
            MeasurementAnnotationDeleteRequest {
                identity: AgentIdentityOverride::default(),
                thread_id: Some(created.thread_id.clone()),
                message_id: Some(created.message_id.clone()),
                annotation_id: "measurement-wall".to_string(),
                title: None,
                version_name: None,
            },
            &test_ctx(),
        )
        .await
        .expect("delete annotation");

        assert!(deleted.model_manifest.measurement_annotations.is_empty());
    }

    #[tokio::test]
    async fn session_reply_save_persists_final_reply_to_thread_history_and_logs() {
        let (state, _resolver) = seed_target().await;
        seed_live_session(&state).await;

        let response = handle_session_reply_save(
            &state,
            SessionReplySaveRequest {
                identity: AgentIdentityOverride::default(),
                thread_id: Some("thread-1".to_string()),
                message_id: Some("msg-1".to_string()),
                body: "Saved in the current pot frame thread.".to_string(),
                fatal: false,
            },
            &test_ctx(),
        )
        .await
        .expect("session reply save");

        assert_eq!(response.thread_id, "thread-1");

        let messages = {
            let conn = state.db.lock().await;
            db::get_thread_messages(&conn, "thread-1").expect("messages")
        };
        let saved = messages
            .iter()
            .find(|message| message.id == response.message_id)
            .expect("saved reply");
        assert_eq!(saved.content, "Saved in the current pot frame thread.");
        assert_eq!(saved.role, MessageRole::Assistant);
        assert_eq!(
            saved
                .agent_origin
                .as_ref()
                .map(|origin| origin.session_id.as_str()),
            Some("session-1")
        );

        let logs = state.app_logs.lock().unwrap();
        let last = logs.back().expect("log entry");
        assert!(last.message.contains("kind=final_reply_save"));
        assert!(last
            .message
            .contains("Saved in the current pot frame thread."));
    }

    #[tokio::test]
    async fn long_action_notice_updates_live_session_and_logs() {
        let (state, _resolver) = seed_target().await;
        seed_live_session(&state).await;

        let response = handle_long_action_notice(
            &state,
            LongActionNoticeRequest {
                identity: AgentIdentityOverride::default(),
                message: "Developing the next iteration".to_string(),
                phase: Some("working".to_string()),
                details: Some("connector placement pass".to_string()),
            },
            &test_ctx(),
        )
        .await
        .expect("long action notice");

        assert_eq!(response.phase, "working");
        assert!(response.busy);
        assert_eq!(response.activity_label, "Developing the next iteration");

        let live_session = state
            .mcp_sessions
            .lock()
            .await
            .get("session-1")
            .cloned()
            .expect("live session");
        assert!(live_session.busy);
        assert_eq!(
            live_session.activity_label.as_deref(),
            Some("Developing the next iteration")
        );
        assert_eq!(live_session.phase.as_deref(), Some("working"));

        let logs = state.app_logs.lock().unwrap();
        let last = logs.back().expect("log entry");
        assert!(last.message.contains("kind=session_activity_set"));
        assert!(last.message.contains("connector placement pass"));
    }

    #[tokio::test]
    async fn long_action_clear_resets_live_session_busy_state() {
        let (state, _resolver) = seed_target().await;
        seed_live_session(&state).await;
        handle_long_action_notice(
            &state,
            LongActionNoticeRequest {
                identity: AgentIdentityOverride::default(),
                message: "Developing the next iteration".to_string(),
                phase: Some("working".to_string()),
                details: None,
            },
            &test_ctx(),
        )
        .await
        .expect("seed long action");

        let response = handle_long_action_clear(
            &state,
            LongActionClearRequest {
                identity: AgentIdentityOverride::default(),
                phase: Some("idle".to_string()),
                status_text: Some("Ready for the next queued message.".to_string()),
            },
            &test_ctx(),
        )
        .await
        .expect("long action clear");

        assert_eq!(response.phase, "idle");
        assert!(!response.busy);

        let live_session = state
            .mcp_sessions
            .lock()
            .await
            .get("session-1")
            .cloned()
            .expect("live session");
        assert!(!live_session.busy);
        assert_eq!(live_session.activity_label, None);
        assert_eq!(live_session.phase.as_deref(), Some("idle"));
        assert_eq!(
            live_session.status_text.as_deref(),
            Some("Ready for the next queued message.")
        );
    }
}
