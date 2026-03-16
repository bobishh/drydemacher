use crate::contracts::AgentSessionTraceEntry;
use crate::models::AppState;

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[allow(clippy::too_many_arguments)]
pub fn push_session_trace(
    state: &AppState,
    session_id: impl Into<String>,
    agent_label: impl Into<String>,
    thread_id: Option<String>,
    message_id: Option<String>,
    model_id: Option<String>,
    phase: impl Into<String>,
    kind: impl Into<String>,
    summary: impl Into<String>,
    details: Option<String>,
) {
    state.push_agent_session_trace(AgentSessionTraceEntry {
        session_id: session_id.into(),
        agent_label: agent_label.into(),
        thread_id,
        message_id,
        model_id,
        phase: phase.into(),
        kind: kind.into(),
        summary: summary.into(),
        details,
        timestamp: now_secs(),
    });
}

pub fn latest_summary_for_session(state: &AppState, session_id: &str) -> Option<String> {
    state.latest_agent_session_trace_summary(session_id)
}

pub fn has_trace_for_session(state: &AppState, session_id: &str) -> bool {
    !state.agent_session_trace(session_id).is_empty()
}
