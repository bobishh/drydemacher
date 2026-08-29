use super::{claim_owner_for_thread, AgentContext, THREAD_MESSAGE_CONTENT_MAX_CHARS};
use crate::contracts::AppResult;
use crate::mcp::contracts::{
    AgentIdentityOverride, AgentIdentityResponse, AgentIdentitySetRequest, ThreadGetRequest,
    ThreadGetResponse, ThreadMessageEntry, ThreadMessagesRequest, ThreadMessagesResponse,
};
use crate::models::AppState;
use crate::services::history;

fn compact_message_content(content: &str) -> String {
    crate::context::compact_text(content, THREAD_MESSAGE_CONTENT_MAX_CHARS)
}

pub async fn handle_thread_get(
    state: &AppState,
    req: ThreadGetRequest,
) -> AppResult<ThreadGetResponse> {
    let conn = state.db.lock().await;
    let thread = history::get_thread_summary(&conn, &req.thread_id)?;
    drop(conn);
    Ok(ThreadGetResponse {
        thread,
        claim_owner: claim_owner_for_thread(state, &req.thread_id).await,
    })
}

pub async fn handle_thread_messages_get(
    state: &AppState,
    req: ThreadMessagesRequest,
) -> AppResult<ThreadMessagesResponse> {
    let roles = req.roles.as_ref().map(|raw_roles| {
        raw_roles
            .iter()
            .filter_map(|role| match role.trim().to_ascii_lowercase().as_str() {
                "user" => Some(crate::contracts::MessageRole::User),
                "assistant" => Some(crate::contracts::MessageRole::Assistant),
                _ => None,
            })
            .collect::<Vec<_>>()
    });
    let conn = state.db.lock().await;
    let page = history::get_thread_messages_page_filtered(
        &conn,
        &req.thread_id,
        req.before.clone(),
        req.limit,
        roles.as_deref(),
    )?;
    drop(conn);

    let compact_messages = page
        .messages
        .into_iter()
        .map(|m| ThreadMessageEntry {
            id: m.id,
            role: serde_json::to_value(&m.role)
                .ok()
                .and_then(|v| v.as_str().map(String::from))
                .unwrap_or_default(),
            status: serde_json::to_value(&m.status)
                .ok()
                .and_then(|v| v.as_str().map(String::from))
                .unwrap_or_default(),
            timestamp: m.timestamp,
            content: compact_message_content(&m.content),
            has_output: m
                .version_summary
                .as_ref()
                .is_some_and(|version| version.has_output),
            has_artifacts: m
                .version_summary
                .as_ref()
                .is_some_and(|version| version.has_runtime),
            has_manifest: m
                .version_summary
                .as_ref()
                .is_some_and(|version| version.has_manifest),
        })
        .collect();

    Ok(ThreadMessagesResponse {
        thread_id: req.thread_id,
        messages: compact_messages,
        next_cursor: page.next_before,
        has_more: page.has_more,
        observed_bytes: page.observed_bytes,
    })
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
