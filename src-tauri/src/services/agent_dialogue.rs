use crate::contracts::{Attachment, AttachmentKind};
use crate::db;
use crate::models::{AgentOrigin, AppError, AppResult, AppState, Message};

#[derive(Debug, Clone)]
pub struct AgentDialogueIdentity {
    pub session_id: String,
    pub client_kind: String,
    pub host_label: String,
    pub agent_label: String,
    pub llm_model_id: Option<String>,
    pub llm_model_label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionThreadTarget {
    pub thread_id: String,
    pub message_id: Option<String>,
    pub model_id: Option<String>,
}

pub fn build_agent_origin(identity: &AgentDialogueIdentity, created_at: u64) -> AgentOrigin {
    AgentOrigin {
        host_label: identity.host_label.clone(),
        client_kind: identity.client_kind.clone(),
        agent_label: identity.agent_label.clone(),
        llm_model_id: identity.llm_model_id.clone(),
        llm_model_label: identity.llm_model_label.clone(),
        session_id: identity.session_id.clone(),
        created_at,
    }
}

pub fn default_prompt_request_message(agent_label: &str) -> String {
    format!("{} is waiting for your input.", agent_label)
}

pub fn normalize_prompt_request_message(message: Option<&str>, agent_label: &str) -> String {
    message
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| default_prompt_request_message(agent_label))
}

pub fn collect_attachment_image_paths(attachments: &[Attachment]) -> Vec<String> {
    attachments
        .iter()
        .filter(|attachment| attachment.kind == AttachmentKind::Image)
        .map(|attachment| attachment.path.clone())
        .collect()
}

pub fn build_user_reply_message_content(prompt_text: &str, attachments: &[Attachment]) -> String {
    let trimmed = prompt_text.trim();
    if !trimmed.is_empty() {
        return trimmed.to_string();
    }

    let count = attachments.len();
    if count == 0 {
        return "Shared a follow-up reply.".to_string();
    }

    let attachment_names = attachments
        .iter()
        .take(3)
        .map(|attachment| attachment.name.trim())
        .filter(|name| !name.is_empty())
        .collect::<Vec<_>>();
    if attachment_names.is_empty() {
        return format!(
            "Shared {} attachment{}.",
            count,
            if count == 1 { "" } else { "s" }
        );
    }

    let suffix = if count > attachment_names.len() {
        format!(" (+{} more)", count - attachment_names.len())
    } else {
        String::new()
    };
    format!(
        "Shared attachment{}: {}{}",
        if count == 1 { "" } else { "s" },
        attachment_names.join(", "),
        suffix
    )
}

pub async fn resolve_session_thread_target(
    state: &AppState,
    session_id: &str,
) -> AppResult<Option<SessionThreadTarget>> {
    let live_target = {
        let sessions = state.mcp_sessions.lock().await;
        sessions
            .get(session_id)
            .and_then(|session| session.last_target.clone())
            .map(|target| SessionThreadTarget {
                thread_id: target.thread_id,
                message_id: Some(target.message_id),
                model_id: target.model_id,
            })
    };
    if live_target.is_some() {
        return Ok(live_target);
    }

    let conn = state.db.lock().await;
    if let Some(runtime_target) =
        crate::mcp::runtime::runtime_snapshot_by_session_id(state, session_id)
            .and_then(|snapshot| snapshot.pending_thread_id)
    {
        let message_id = db::get_latest_successful_message_id_in_thread(&conn, &runtime_target)
            .map_err(|err| AppError::persistence(err.to_string()))?;
        return Ok(Some(SessionThreadTarget {
            thread_id: runtime_target,
            message_id,
            model_id: None,
        }));
    }

    let stored_session = db::get_sessions_by_ids(&conn, &[session_id.to_string()])
        .map_err(|err| AppError::persistence(err.to_string()))?
        .into_iter()
        .next();

    Ok(stored_session.and_then(|session| {
        session.thread_id.map(|thread_id| SessionThreadTarget {
            thread_id,
            message_id: session.message_id,
            model_id: session.model_id,
        })
    }))
}

pub async fn add_dialogue_message(
    state: &AppState,
    thread_id: &str,
    message: &Message,
) -> AppResult<()> {
    let conn = state.db.lock().await;
    db::add_message(&conn, thread_id, message).map_err(|err| AppError::persistence(err.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::{Config, McpConfig};
    use crate::models::{AppState, McpSessionState, McpTargetRef};
    use std::path::PathBuf;

    fn test_db_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "ecky-agent-dialogue-{}-{}",
            name,
            uuid::Uuid::new_v4()
        ))
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

    #[test]
    fn user_reply_content_falls_back_to_attachment_summary() {
        let content = build_user_reply_message_content(
            "",
            &[Attachment {
                path: "/tmp/ref.png".to_string(),
                name: "ref.png".to_string(),
                explanation: String::new(),
                kind: AttachmentKind::Image,
            }],
        );

        assert_eq!(content, "Shared attachment: ref.png");
    }

    #[test]
    fn collect_attachment_image_paths_ignores_non_images() {
        let attachments = vec![
            Attachment {
                path: "/tmp/ref.png".to_string(),
                name: "ref.png".to_string(),
                explanation: String::new(),
                kind: AttachmentKind::Image,
            },
            Attachment {
                path: "/tmp/model.fcstd".to_string(),
                name: "model.fcstd".to_string(),
                explanation: String::new(),
                kind: AttachmentKind::Cad,
            },
        ];

        assert_eq!(
            collect_attachment_image_paths(&attachments),
            vec!["/tmp/ref.png".to_string()]
        );
    }

    #[tokio::test]
    async fn resolve_session_thread_target_prefers_live_target() {
        let conn = crate::db::init_db(&test_db_path("live-target")).expect("db");
        let state = AppState::new(test_config(), None, conn);
        state.mcp_sessions.lock().await.insert(
            "session-1".to_string(),
            McpSessionState {
                client_kind: "mcp-http".to_string(),
                host_label: "Claude".to_string(),
                agent_label: "Claude".to_string(),
                llm_model_id: None,
                llm_model_label: None,
                last_target: Some(McpTargetRef {
                    thread_id: "thread-live".to_string(),
                    message_id: "msg-live".to_string(),
                    model_id: Some("model-live".to_string()),
                }),
            },
        );

        let target = resolve_session_thread_target(&state, "session-1")
            .await
            .expect("target")
            .expect("live target");
        assert_eq!(target.thread_id, "thread-live");
        assert_eq!(target.message_id.as_deref(), Some("msg-live"));
        assert_eq!(target.model_id.as_deref(), Some("model-live"));
    }

    #[tokio::test]
    async fn resolve_session_thread_target_falls_back_to_runtime_pending_thread() {
        let conn = crate::db::init_db(&test_db_path("runtime-target")).expect("db");
        let mut config = test_config();
        config.connection_type = Some("mcp".to_string());
        config.mcp.mode = crate::contracts::McpMode::Active;
        config.mcp.primary_agent_id = Some("agent-claude".to_string());
        config.mcp.auto_agents = vec![crate::contracts::AutoAgent {
            id: "agent-claude".to_string(),
            label: "Claude".to_string(),
            cmd: "claude".to_string(),
            model: None,
            args: Vec::new(),
            enabled: true,
            start_on_demand: true,
        }];
        let state = AppState::new(config, None, conn);
        crate::mcp::runtime::initialize_auto_agent_supervisors(state.clone());

        {
            let conn = state.db.lock().await;
            crate::db::create_or_update_thread(&conn, "thread-runtime", "Runtime", 1, None)
                .expect("thread");
            crate::db::add_message(
                &conn,
                "thread-runtime",
                &crate::models::Message {
                    id: "msg-runtime".to_string(),
                    role: crate::models::MessageRole::Assistant,
                    content: "Saved".to_string(),
                    status: crate::models::MessageStatus::Success,
                    output: Some(crate::models::DesignOutput {
                        title: "Runtime".to_string(),
                        version_name: "V1".to_string(),
                        response: String::new(),
                        interaction_mode: crate::models::InteractionMode::Design,
                        macro_code: "print('hi')".to_string(),
                        macro_dialect: crate::models::MacroDialect::Legacy,
                        ui_spec: crate::models::UiSpec { fields: Vec::new() },
                        initial_params: std::collections::BTreeMap::new(),
                        post_processing: None,
                    }),
                    usage: None,
                    artifact_bundle: None,
                    model_manifest: None,
                    agent_origin: None,
                    image_data: None,
                    visual_kind: None,
                    attachment_images: Vec::new(),
                    timestamp: 1,
                },
            )
            .expect("message");
        }

        state.mcp_sessions.lock().await.insert(
            "session-1".to_string(),
            McpSessionState::new("mcp-http".to_string(), "Claude".to_string()),
        );
        crate::mcp::runtime::mark_agent_active(
            &state,
            "Claude",
            Some("session-1".to_string()),
            Some("thread-runtime".to_string()),
            None,
            Some("Working".to_string()),
        );

        let target = resolve_session_thread_target(&state, "session-1")
            .await
            .expect("target")
            .expect("runtime target");
        assert_eq!(target.thread_id, "thread-runtime");
        assert_eq!(target.message_id.as_deref(), Some("msg-runtime"));
    }

    #[tokio::test]
    async fn resolve_session_thread_target_falls_back_to_persisted_session_row() {
        let conn = crate::db::init_db(&test_db_path("stored-target")).expect("db");
        let state = AppState::new(test_config(), None, conn);
        {
            let conn = state.db.lock().await;
            crate::db::upsert_agent_session(
                &conn,
                &crate::contracts::AgentSession {
                    session_id: "session-1".to_string(),
                    client_kind: "mcp-http".to_string(),
                    host_label: "Claude".to_string(),
                    agent_label: "Claude".to_string(),
                    llm_model_id: None,
                    llm_model_label: None,
                    thread_id: Some("thread-db".to_string()),
                    message_id: Some("msg-db".to_string()),
                    model_id: Some("model-db".to_string()),
                    phase: "reading".to_string(),
                    status_text: String::new(),
                    updated_at: 1,
                },
            )
            .expect("session row");
        }

        let target = resolve_session_thread_target(&state, "session-1")
            .await
            .expect("target")
            .expect("stored target");
        assert_eq!(target.thread_id, "thread-db");
        assert_eq!(target.message_id.as_deref(), Some("msg-db"));
        assert_eq!(target.model_id.as_deref(), Some("model-db"));
    }
}
