use std::fs;
use std::io::Write;
use std::path::PathBuf;

use portable_pty::PtySize;
use tauri::{AppHandle, Manager, State};

use crate::db;
use crate::mcp::runtime;
use crate::models::{
    AgentSession, AppError, AppResult, AppState, LastDesignSnapshot, McpServerStatus,
    ThreadAgentState, ViewportScreenshotCapture,
};
use crate::services::agent_dialogue;

fn encode_control_key(key: &str) -> Option<u8> {
    if key.eq_ignore_ascii_case("space") {
        return Some(0);
    }

    let mut chars = key.chars();
    let ch = chars.next()?;
    if chars.next().is_some() {
        return None;
    }

    match ch {
        'a'..='z' | 'A'..='Z' => Some((ch.to_ascii_uppercase() as u8) & 0x1f),
        '@' | '`' | ' ' => Some(0),
        '[' => Some(27),
        '\\' => Some(28),
        ']' => Some(29),
        '^' | '6' => Some(30),
        '_' | '-' => Some(31),
        _ => None,
    }
}

fn encode_named_terminal_key(key: &str) -> Option<&'static [u8]> {
    match key {
        "Enter" => Some(b"\r"),
        "Tab" => Some(b"\t"),
        "Escape" => Some(b"\x1b"),
        "Backspace" => Some(b"\x7f"),
        "Delete" => Some(b"\x1b[3~"),
        "ArrowUp" => Some(b"\x1b[A"),
        "ArrowDown" => Some(b"\x1b[B"),
        "ArrowRight" => Some(b"\x1b[C"),
        "ArrowLeft" => Some(b"\x1b[D"),
        "Home" => Some(b"\x1b[H"),
        "End" => Some(b"\x1b[F"),
        "PageUp" => Some(b"\x1b[5~"),
        "PageDown" => Some(b"\x1b[6~"),
        "Insert" => Some(b"\x1b[2~"),
        _ => None,
    }
}

fn encode_terminal_key_input(key: &str, ctrl: bool, alt: bool) -> AppResult<Vec<u8>> {
    let mut payload = Vec::new();
    if alt {
        payload.push(0x1b);
    }

    let bytes = if ctrl {
        vec![encode_control_key(key).ok_or_else(|| {
            AppError::validation(format!("Unsupported terminal control key: {}", key))
        })?]
    } else if let Some(named) = encode_named_terminal_key(key) {
        named.to_vec()
    } else {
        let mut chars = key.chars();
        let ch = chars
            .next()
            .ok_or_else(|| AppError::validation("Terminal key input must not be empty."))?;
        if chars.next().is_some() {
            return Err(AppError::validation(format!(
                "Unsupported terminal key: {}",
                key
            )));
        }
        let mut buffer = [0_u8; 4];
        ch.encode_utf8(&mut buffer).as_bytes().to_vec()
    };

    payload.extend(bytes);
    Ok(payload)
}

fn encode_agent_terminal_input(input: &crate::contracts::AgentTerminalInput) -> AppResult<Vec<u8>> {
    let mut payload = Vec::new();

    if !input.text.is_empty() {
        payload.extend_from_slice(input.text.as_bytes());
    }

    if let Some(key) = input.key.as_deref() {
        payload.extend(encode_terminal_key_input(key, input.ctrl, input.alt)?);
    }

    if input.submit || (payload.is_empty() && input.key.is_none()) {
        payload.extend_from_slice(b"\r");
    }

    if payload.is_empty() {
        return Err(AppError::validation(
            "Agent terminal input is empty and produced no PTY bytes.",
        ));
    }

    Ok(payload)
}

#[tauri::command]
#[specta::specta]
pub async fn get_active_agent_sessions(state: State<'_, AppState>) -> AppResult<Vec<AgentSession>> {
    let conn = state.db.lock().await;
    db::get_active_agent_sessions(&conn, 600)
        .map_err(|e| crate::models::AppError::persistence(e.to_string()))
}

#[tauri::command]
#[specta::specta]
pub async fn get_mcp_server_status(state: State<'_, AppState>) -> AppResult<McpServerStatus> {
    Ok(state.mcp_status())
}

#[tauri::command]
#[specta::specta]
pub async fn get_agent_terminal_snapshots(
    state: State<'_, AppState>,
) -> AppResult<Vec<crate::contracts::AgentTerminalSnapshot>> {
    Ok(state
        .agent_terminals
        .lock()
        .unwrap()
        .values()
        .map(|runtime| runtime.snapshot.clone())
        .collect())
}

#[tauri::command]
#[specta::specta]
pub async fn send_agent_terminal_input(
    input: crate::contracts::AgentTerminalInput,
    state: State<'_, AppState>,
) -> AppResult<()> {
    let writer = {
        let mut terminals = state.agent_terminals.lock().unwrap();
        let Some(runtime) = terminals.get_mut(&input.agent_id) else {
            return Err(AppError::not_found(format!(
                "No active terminal for agent {}.",
                input.agent_id
            )));
        };
        if !runtime.snapshot.active {
            return Err(AppError::validation(format!(
                "{} terminal is not accepting input right now.",
                runtime.snapshot.agent_label
            )));
        }
        runtime.snapshot.attention_required = false;
        runtime.snapshot.summary = None;
        runtime.snapshot.updated_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let snapshot = runtime.snapshot.clone();
        let writer = runtime.writer.clone();
        drop(terminals);
        state.emit_agent_terminal_update(&snapshot);
        writer
    };

    let payload = encode_agent_terminal_input(&input)?;

    let mut writer = writer.lock().unwrap();
    writer
        .write_all(&payload)
        .map_err(|err| AppError::internal(format!("Failed to write to agent terminal: {}", err)))?;
    writer
        .flush()
        .map_err(|err| AppError::internal(format!("Failed to flush agent terminal: {}", err)))?;
    Ok(())
}

fn resize_agent_terminal_impl(
    agent_id: &str,
    cols: u16,
    rows: u16,
    state: &AppState,
) -> AppResult<()> {
    if cols < 2 || rows < 1 {
        return Err(AppError::validation(format!(
            "Invalid PTY size {}x{}.",
            cols, rows
        )));
    }

    let pty = {
        let terminals = state.agent_terminals.lock().unwrap();
        let Some(runtime) = terminals.get(agent_id) else {
            return Err(AppError::not_found(format!(
                "No active terminal for agent {}.",
                agent_id
            )));
        };
        if !runtime.snapshot.active {
            return Err(AppError::validation(format!(
                "{} terminal is not accepting resize right now.",
                runtime.snapshot.agent_label
            )));
        }
        runtime.pty.clone()
    };

    let pty = pty.lock().unwrap();
    pty.resize(PtySize {
        rows,
        cols,
        pixel_width: 0,
        pixel_height: 0,
    })
    .map_err(|err| AppError::internal(format!("Failed to resize agent terminal: {}", err)))?;
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn resize_agent_terminal(
    agent_id: String,
    cols: u16,
    rows: u16,
    state: State<'_, AppState>,
) -> AppResult<()> {
    resize_agent_terminal_impl(&agent_id, cols, rows, &state)
}

fn last_snapshot_path(app: &AppHandle) -> PathBuf {
    app.path()
        .app_config_dir()
        .unwrap()
        .join("last_design.json")
}

pub(crate) fn write_last_snapshot(app: &AppHandle, snapshot: Option<&LastDesignSnapshot>) {
    let path = last_snapshot_path(app);
    match snapshot {
        Some(snapshot) => {
            if let Ok(serialized) = serde_json::to_string_pretty(snapshot) {
                let _ = fs::write(path, serialized);
            }
        }
        None => {
            let _ = fs::remove_file(path);
        }
    }
}

pub(crate) fn build_runtime_snapshot(
    design: Option<crate::models::DesignOutput>,
    thread_id: Option<String>,
    message_id: Option<String>,
    artifact_bundle: Option<crate::models::ArtifactBundle>,
    model_manifest: Option<crate::models::ModelManifest>,
    selected_part_id: Option<String>,
) -> LastDesignSnapshot {
    LastDesignSnapshot {
        design,
        thread_id,
        message_id,
        artifact_bundle,
        model_manifest,
        selected_part_id,
    }
}

#[tauri::command]
#[specta::specta]
pub async fn get_last_design(state: State<'_, AppState>) -> AppResult<Option<LastDesignSnapshot>> {
    Ok(state.last_snapshot.lock().unwrap().clone())
}

#[tauri::command]
#[specta::specta]
pub async fn save_last_design(
    snapshot: Option<LastDesignSnapshot>,
    state: State<'_, AppState>,
    app: AppHandle,
) -> AppResult<()> {
    {
        let mut last = state.last_snapshot.lock().unwrap();
        *last = snapshot.clone();
    }
    write_last_snapshot(&app, snapshot.as_ref());
    Ok(())
}

/// Returns the current agent state for the given thread — for status bar display.
#[tauri::command]
#[specta::specta]
pub async fn get_thread_agent_state(
    thread_id: String,
    state: State<'_, AppState>,
) -> AppResult<ThreadAgentState> {
    let conn = state.db.lock().await;
    let last_session = db::get_thread_last_agent_session(&conn, &thread_id)
        .map_err(|e| AppError::persistence(e.to_string()))?;
    drop(conn);

    let config = state.config.lock().unwrap().clone();
    let runtime_snapshot = runtime::primary_runtime_snapshot(&state);
    let live_session = if let Some(session_id) = runtime_snapshot
        .as_ref()
        .and_then(|runtime| runtime.session_id.clone())
    {
        state.mcp_sessions.lock().await.get(&session_id).cloned()
    } else {
        None
    };
    let session_id_for_trace = runtime_snapshot
        .as_ref()
        .and_then(|runtime| runtime.session_id.clone())
        .or_else(|| {
            last_session
                .as_ref()
                .map(|session| session.session_id.clone())
        });
    let latest_trace_summary = session_id_for_trace.as_deref().and_then(|session_id| {
        crate::services::agent_trace::latest_summary_for_session(&state, session_id)
    });
    let has_trace = session_id_for_trace.as_deref().is_some_and(|session_id| {
        crate::services::agent_trace::has_trace_for_session(&state, session_id)
    });

    Ok(runtime::derive_thread_agent_state(
        &config,
        &thread_id,
        runtime::ThreadAgentStateInputs {
            runtime: runtime_snapshot,
            live_session,
            last_session,
            latest_trace_summary,
            has_trace,
            now: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        },
    ))
}

#[tauri::command]
#[specta::specta]
pub async fn get_agent_session_trace(
    session_id: String,
    state: State<'_, AppState>,
) -> AppResult<Vec<crate::contracts::AgentSessionTraceEntry>> {
    Ok(state.agent_session_trace(&session_id))
}

async fn resolve_agent_prompt_impl(
    input: crate::contracts::ResolveAgentPromptInput,
    state: &AppState,
) -> AppResult<()> {
    let request_id = input.request_id.clone();

    // Wake a frozen active-mode agent before unblocking its HTTP request.
    let prompt_control = state.prompt_waits.lock().unwrap().remove(&request_id);
    #[cfg(unix)]
    if let Some(pgid) = prompt_control.as_ref().and_then(|control| control.pgid) {
        unsafe {
            libc::kill(-pgid, libc::SIGCONT);
        }
        eprintln!("[MCP] SIGCONT pgid {} (prompt: {})", pgid, request_id);
    }
    if let Some(control) = prompt_control.as_ref() {
        runtime::mark_agent_turn_busy(
            state,
            &control.agent_label,
            None,
            None,
            None,
            Some("Working through the queued message.".to_string()),
        );
    }

    let mut channels = state.prompt_channels.lock().await;
    if let Some(tx) = channels.remove(&request_id) {
        let delivered = input.clone();
        let _ = tx.send(delivered);
    } else {
        return Err(AppError::not_found(format!(
            "No pending prompt request with id: {}",
            request_id
        )));
    }

    if let Some(thread_id) = prompt_control
        .as_ref()
        .and_then(|control| control.thread_id.clone())
    {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let attachment_images = agent_dialogue::collect_attachment_image_paths(&input.attachments);
        let reply_content = agent_dialogue::build_user_reply_message_content(
            &input.prompt_text,
            &input.attachments,
        );
        agent_dialogue::add_dialogue_message(
            state,
            &thread_id,
            &crate::models::Message {
                id: uuid::Uuid::new_v4().to_string(),
                role: crate::models::MessageRole::User,
                content: reply_content,
                status: crate::models::MessageStatus::Success,
                output: None,
                usage: None,
                artifact_bundle: None,
                model_manifest: None,
                agent_origin: None,
                image_data: None,
                visual_kind: None,
                attachment_images,
                timestamp,
            },
        )
        .await?;
        state.emit_history_updated();
    }
    Ok(())
}

/// Called by the frontend when the user submits a prompt in MCP mode.
/// Resolves the pending oneshot channel so the MCP handler can return the text and attachments.
#[tauri::command]
#[specta::specta]
pub async fn resolve_agent_prompt(
    input: crate::contracts::ResolveAgentPromptInput,
    state: State<'_, AppState>,
) -> AppResult<()> {
    resolve_agent_prompt_impl(input, &state).await
}

/// Called by the frontend when the user clicks a confirmation button.
/// Resolves the pending oneshot channel so the MCP handler can return.
#[tauri::command]
#[specta::specta]
pub async fn resolve_agent_confirm(
    request_id: String,
    choice: String,
    state: State<'_, AppState>,
) -> AppResult<()> {
    let mut channels = state.confirm_channels.lock().await;
    if let Some(tx) = channels.remove(&request_id) {
        let _ = tx.send(choice);
    } else {
        return Err(AppError::not_found(format!(
            "No pending confirmation with id: {}",
            request_id
        )));
    }
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn resolve_agent_viewport_screenshot(
    input: crate::contracts::ResolveViewportScreenshotInput,
    state: State<'_, AppState>,
) -> AppResult<()> {
    let mut channels = state.viewport_screenshot_channels.lock().await;
    if let Some(tx) = channels.remove(&input.request_id) {
        let _ = tx.send(Ok(ViewportScreenshotCapture {
            data_url: input.data_url,
            width: input.width,
            height: input.height,
            camera: input.camera,
            source: input.source,
            thread_id: input.thread_id,
            message_id: input.message_id,
            model_id: input.model_id,
            include_overlays: input.include_overlays,
        }));
    } else {
        return Err(AppError::not_found(format!(
            "No pending viewport screenshot with id: {}",
            input.request_id
        )));
    }
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn reject_agent_viewport_screenshot(
    input: crate::contracts::RejectViewportScreenshotInput,
    state: State<'_, AppState>,
) -> AppResult<()> {
    let mut channels = state.viewport_screenshot_channels.lock().await;
    if let Some(tx) = channels.remove(&input.request_id) {
        let _ = tx.send(Err(input.error));
    } else {
        return Err(AppError::not_found(format!(
            "No pending viewport screenshot with id: {}",
            input.request_id
        )));
    }
    Ok(())
}

/// Called by the frontend when the user queues a message in MCP mode and no agent is running.
/// Fires the wake notifier so the supervisor loop can respawn the named agent.
/// Safe to call redundantly — noop if the agent is already running.
#[tauri::command]
#[specta::specta]
pub async fn wake_auto_agent(label: String, state: State<'_, AppState>) -> AppResult<()> {
    runtime::wake_auto_agent_by_label(&state, &label, None).await
}

#[tauri::command]
#[specta::specta]
pub async fn wake_primary_auto_agent(
    thread_id: Option<String>,
    state: State<'_, AppState>,
) -> AppResult<()> {
    runtime::wake_primary_auto_agent(&state, thread_id).await
}

#[tauri::command]
#[specta::specta]
pub async fn stop_primary_auto_agent(
    thread_id: Option<String>,
    state: State<'_, AppState>,
) -> AppResult<()> {
    runtime::stop_primary_auto_agent(&state, thread_id).await
}

#[tauri::command]
#[specta::specta]
pub async fn restart_primary_auto_agent(
    thread_id: Option<String>,
    state: State<'_, AppState>,
) -> AppResult<()> {
    runtime::restart_primary_auto_agent(&state, thread_id).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::{Config, McpConfig};
    use portable_pty::native_pty_system;
    use std::path::PathBuf;

    fn test_db_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("ecky-{}-{}", name, uuid::Uuid::new_v4()))
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

    #[tokio::test]
    async fn resolve_agent_prompt_impl_preserves_attachments() {
        let conn =
            crate::db::init_db(&test_db_path("resolve-agent-prompt-attachments")).expect("db");
        let state = AppState::new(test_config(), None, conn);
        let (tx, rx) = tokio::sync::oneshot::channel();

        state
            .prompt_channels
            .lock()
            .await
            .insert("req-1".to_string(), tx);

        resolve_agent_prompt_impl(
            crate::contracts::ResolveAgentPromptInput {
                request_id: "req-1".to_string(),
                prompt_text: "show outer frame".to_string(),
                attachments: vec![crate::contracts::Attachment {
                    path: "/tmp/frame.png".to_string(),
                    name: "frame.png".to_string(),
                    explanation: "Reference photo".to_string(),
                    kind: crate::contracts::AttachmentKind::Image,
                }],
            },
            &state,
        )
        .await
        .expect("resolve agent prompt");

        let delivered = rx.await.expect("prompt delivery");
        assert_eq!(delivered.prompt_text, "show outer frame");
        assert_eq!(delivered.attachments.len(), 1);
        assert_eq!(delivered.attachments[0].path, "/tmp/frame.png");
        assert_eq!(
            delivered.attachments[0].kind,
            crate::contracts::AttachmentKind::Image
        );
    }

    #[tokio::test]
    async fn resolve_agent_prompt_impl_persists_user_reply_to_thread_history() {
        let conn = crate::db::init_db(&test_db_path("resolve-agent-prompt-history")).expect("db");
        let state = AppState::new(test_config(), None, conn);
        let (tx, rx) = tokio::sync::oneshot::channel();
        let timestamp = 42_u64;

        {
            let conn = state.db.lock().await;
            crate::db::create_or_update_thread(&conn, "thread-1", "Thread", timestamp, None)
                .expect("thread");
        }

        state
            .prompt_channels
            .lock()
            .await
            .insert("req-2".to_string(), tx);
        state.prompt_waits.lock().unwrap().insert(
            "req-2".to_string(),
            crate::models::PromptResumeState {
                pgid: None,
                agent_label: "Claude".to_string(),
                thread_id: Some("thread-1".to_string()),
            },
        );

        resolve_agent_prompt_impl(
            crate::contracts::ResolveAgentPromptInput {
                request_id: "req-2".to_string(),
                prompt_text: "Use the smoother lip.".to_string(),
                attachments: vec![crate::contracts::Attachment {
                    path: "/tmp/reference.png".to_string(),
                    name: "reference.png".to_string(),
                    explanation: "Reference".to_string(),
                    kind: crate::contracts::AttachmentKind::Image,
                }],
            },
            &state,
        )
        .await
        .expect("resolve prompt");

        let delivered = rx.await.expect("prompt delivery");
        assert_eq!(delivered.prompt_text, "Use the smoother lip.");

        let stored_messages = {
            let conn = state.db.lock().await;
            crate::db::get_thread_messages(&conn, "thread-1").expect("messages")
        };
        assert_eq!(stored_messages.len(), 1);
        assert_eq!(stored_messages[0].role, crate::models::MessageRole::User);
        assert_eq!(stored_messages[0].content, "Use the smoother lip.");
        assert_eq!(
            stored_messages[0].attachment_images,
            vec!["/tmp/reference.png".to_string()]
        );
    }

    #[test]
    fn encode_agent_terminal_input_appends_enter_for_submit_lines() {
        let payload = encode_agent_terminal_input(&crate::contracts::AgentTerminalInput {
            agent_id: "claude".to_string(),
            text: "2".to_string(),
            key: None,
            ctrl: false,
            alt: false,
            shift: false,
            meta: false,
            submit: true,
        })
        .expect("payload");

        assert_eq!(payload, b"2\r");
    }

    #[test]
    fn encode_agent_terminal_input_supports_arrow_keys() {
        let payload = encode_agent_terminal_input(&crate::contracts::AgentTerminalInput {
            agent_id: "claude".to_string(),
            text: String::new(),
            key: Some("ArrowDown".to_string()),
            ctrl: false,
            alt: false,
            shift: false,
            meta: false,
            submit: false,
        })
        .expect("payload");

        assert_eq!(payload, b"\x1b[B");
    }

    #[test]
    fn encode_agent_terminal_input_supports_ctrl_shortcuts() {
        let payload = encode_agent_terminal_input(&crate::contracts::AgentTerminalInput {
            agent_id: "claude".to_string(),
            text: String::new(),
            key: Some("c".to_string()),
            ctrl: true,
            alt: false,
            shift: false,
            meta: false,
            submit: false,
        })
        .expect("payload");

        assert_eq!(payload, vec![0x03]);
    }

    #[tokio::test]
    async fn resize_agent_terminal_updates_pty_size() {
        let conn = crate::db::init_db(&test_db_path("resize-agent-terminal")).expect("db");
        let state = AppState::new(test_config(), None, conn);
        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                rows: 24,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
            })
            .expect("pty");
        let writer = pair.master.take_writer().expect("writer");
        let pty: crate::models::AgentTerminalPty =
            std::sync::Arc::new(std::sync::Mutex::new(pair.master));

        state.agent_terminals.lock().unwrap().insert(
            "gemini".to_string(),
            crate::models::AgentTerminalRuntime {
                snapshot: crate::contracts::AgentTerminalSnapshot {
                    agent_id: "gemini".to_string(),
                    agent_label: "gemini".to_string(),
                    provider_kind: Some("gemini".to_string()),
                    session_nonce: 1,
                    screen_text: String::new(),
                    vt_stream: String::new(),
                    vt_delta: None,
                    attention_required: false,
                    busy: false,
                    activity_label: None,
                    activity_started_at: None,
                    attention_kind: None,
                    summary: None,
                    active: true,
                    updated_at: 1,
                },
                writer: std::sync::Arc::new(std::sync::Mutex::new(writer)),
                pty: pty.clone(),
                pending_utf8: Vec::new(),
                pending_escape: String::new(),
                last_emitted_at: None,
            },
        );

        resize_agent_terminal_impl("gemini", 132, 41, &state).expect("resize");

        let size = pty.lock().unwrap().get_size().expect("size");
        assert_eq!(size.cols, 132);
        assert_eq!(size.rows, 41);
    }
}
