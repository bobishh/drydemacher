pub use crate::contracts::{
    AgentActivityActor, AgentActivityActorKind, AgentActivityCatchUp, AgentActivityEvent,
    AgentActivityEventInput, AgentActivityKind, AgentActivitySeverity, AgentActivityState,
};
use crate::mcp::handlers::AgentContext;
use crate::mcp::runtime::{AutoAgentRuntimePhase, AutoAgentRuntimeSnapshot};
use crate::models::AppState;
use std::collections::VecDeque;

const ACTIVITY_JOURNAL_MAX_EVENTS: usize = 2_048;
const ACTIVITY_CATCH_UP_MAX_EVENTS: usize = 256;
const ACTIVITY_JOURNAL_MAX_BYTES: usize = 4 * 1024 * 1024;
const ACTIVITY_CATCH_UP_MAX_BYTES: usize = 1024 * 1024;

#[derive(Debug)]
pub struct AgentActivityJournal {
    events: VecDeque<AgentActivityEvent>,
    next_cursor: u64,
    dropped_count: u64,
    retained_bytes: usize,
}

impl Default for AgentActivityJournal {
    fn default() -> Self {
        Self {
            events: VecDeque::with_capacity(ACTIVITY_JOURNAL_MAX_EVENTS),
            next_cursor: 0,
            dropped_count: 0,
            retained_bytes: 0,
        }
    }
}

impl AgentActivityJournal {
    pub fn record(&mut self, input: AgentActivityEventInput) -> AgentActivityEvent {
        self.next_cursor = self.next_cursor.saturating_add(1);
        let event = AgentActivityEvent {
            event_id: format!("agent-activity-event-{}", uuid::Uuid::new_v4()),
            cursor: self.next_cursor,
            session_id: input.session_id,
            thread_id: input.thread_id,
            message_id: input.message_id,
            version_id: input.version_id,
            actor: AgentActivityActor {
                kind: input.actor_kind,
                id: input.actor_id,
                label: input.actor_label,
            },
            kind: input.kind,
            lifecycle_key: input.lifecycle_key,
            phase: input.phase,
            summary: crate::transport_budget::bounded_text(&input.summary, 4 * 1024),
            detail: input
                .detail
                .map(|value| crate::transport_budget::bounded_text(&value, 8 * 1024)),
            severity: input.severity,
            state: input.state,
            requires_attention: input.requires_attention,
            occurred_at: input.occurred_at,
            raw: input
                .raw
                .map(|value| crate::transport_budget::bounded_text(&value, 48 * 1024)),
        };
        let event_bytes = serde_json::to_vec(&event)
            .map(|value| value.len())
            .unwrap_or(0);
        self.events.push_back(event.clone());
        self.retained_bytes = self.retained_bytes.saturating_add(event_bytes);
        while self.events.len() > ACTIVITY_JOURNAL_MAX_EVENTS
            || self.retained_bytes > ACTIVITY_JOURNAL_MAX_BYTES
        {
            if let Some(removed) = self.events.pop_front() {
                self.retained_bytes = self.retained_bytes.saturating_sub(
                    serde_json::to_vec(&removed)
                        .map(|value| value.len())
                        .unwrap_or(0),
                );
            }
            self.dropped_count = self.dropped_count.saturating_add(1);
        }
        event
    }

    pub fn catch_up(&self, after_cursor: Option<u64>) -> AgentActivityCatchUp {
        let oldest_cursor = self
            .events
            .front()
            .map(|event| event.cursor)
            .unwrap_or_else(|| self.next_cursor.saturating_add(1));
        let requested_after = after_cursor;
        let after_cursor = requested_after
            .unwrap_or_else(|| {
                self.next_cursor
                    .saturating_sub(ACTIVITY_CATCH_UP_MAX_EVENTS as u64)
            })
            .max(oldest_cursor.saturating_sub(1));
        let mut page_bytes = 0usize;
        let mut events = Vec::new();
        for event in self
            .events
            .iter()
            .filter(|event| event.cursor > after_cursor)
        {
            if events.len() >= ACTIVITY_CATCH_UP_MAX_EVENTS {
                break;
            }
            let bytes = serde_json::to_vec(event)
                .map(|value| value.len())
                .unwrap_or(0);
            if !events.is_empty() && page_bytes.saturating_add(bytes) > ACTIVITY_CATCH_UP_MAX_BYTES
            {
                break;
            }
            page_bytes = page_bytes.saturating_add(bytes);
            events.push(event.clone());
        }
        let last_returned = events
            .last()
            .map(|event| event.cursor)
            .unwrap_or(after_cursor);
        AgentActivityCatchUp {
            events,
            latest_cursor: self.next_cursor,
            oldest_cursor,
            has_more: last_returned < self.next_cursor,
            dropped_count: self.dropped_count,
            retained_bytes: self.retained_bytes,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TraceAgentActivityInput {
    pub thread_id: Option<String>,
    pub message_id: Option<String>,
    pub version_id: Option<String>,
    pub phase: String,
    pub kind: String,
    pub summary: String,
    pub details: Option<String>,
}

fn severity_for_trace(
    kind: &str,
    phase: &str,
) -> (AgentActivitySeverity, AgentActivityState, bool) {
    let lowered_kind = kind.trim().to_ascii_lowercase();
    let lowered_phase = phase.trim().to_ascii_lowercase();
    if lowered_kind.contains("error") || lowered_phase == "error" {
        return (
            AgentActivitySeverity::Error,
            AgentActivityState::Failed,
            true,
        );
    }
    if lowered_kind.contains("warn") {
        return (
            AgentActivitySeverity::Warning,
            AgentActivityState::Resolved,
            false,
        );
    }
    if lowered_kind.contains("prompt") || lowered_phase.contains("waiting") {
        return (
            AgentActivitySeverity::Question,
            AgentActivityState::Active,
            true,
        );
    }
    if lowered_kind.contains("cancel") || lowered_phase.contains("cancel") {
        return (
            AgentActivitySeverity::Warning,
            AgentActivityState::Canceled,
            false,
        );
    }
    if lowered_kind.contains("success") || lowered_phase == "idle" {
        return (
            AgentActivitySeverity::Success,
            AgentActivityState::Resolved,
            false,
        );
    }
    (
        AgentActivitySeverity::Info,
        AgentActivityState::Active,
        false,
    )
}

fn lifecycle_key_for_trace(ctx: &AgentContext, input: &TraceAgentActivityInput) -> String {
    let thread_id = input.thread_id.as_deref().unwrap_or("global");
    let message_id = input.message_id.as_deref().unwrap_or("none");
    format!(
        "trace:{}:{}:{}:{}",
        ctx.session_id, thread_id, message_id, input.kind
    )
}

fn trace_raw_payload(input: &TraceAgentActivityInput) -> serde_json::Value {
    serde_json::json!({
        "phase": input.phase,
        "kind": input.kind,
        "details": input.details,
    })
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[derive(Debug, Clone)]
pub struct LongTaskActivityInput {
    pub session_id: String,
    pub thread_id: Option<String>,
    pub message_id: Option<String>,
    pub actor_kind: AgentActivityActorKind,
    pub actor_id: String,
    pub actor_label: String,
    pub job_id: String,
    pub stage: String,
    pub summary: String,
    pub detail: Option<String>,
    pub progress_current: u64,
    pub progress_total: u64,
    pub expected_duration_ms: u64,
    pub state: AgentActivityState,
    pub cancellable: bool,
}

fn long_task_event_input(input: LongTaskActivityInput) -> AgentActivityEventInput {
    let terminal = input.state != AgentActivityState::Active;
    let requires_attention = input.state == AgentActivityState::Failed;
    let severity = match &input.state {
        AgentActivityState::Failed => AgentActivitySeverity::Error,
        AgentActivityState::Canceled => AgentActivitySeverity::Warning,
        AgentActivityState::Resolved => AgentActivitySeverity::Success,
        AgentActivityState::Active => AgentActivitySeverity::Info,
    };
    let raw = serde_json::json!({
        "kind": if terminal { "long_task_finished" } else { "long_task_progress" },
        "taskId": &input.job_id,
        "expectedDurationMs": input.expected_duration_ms,
        "stage": &input.stage,
        "progressCurrent": input.progress_current,
        "progressTotal": input.progress_total,
        "jobId": &input.job_id,
        "cancellable": input.cancellable && !terminal,
    });
    AgentActivityEventInput {
        session_id: input.session_id,
        thread_id: input.thread_id,
        message_id: input.message_id,
        version_id: None,
        actor_kind: input.actor_kind,
        actor_id: input.actor_id,
        actor_label: input.actor_label,
        kind: AgentActivityKind::Trace,
        lifecycle_key: Some(format!("long-task:{}", input.job_id)),
        phase: Some(input.stage.to_ascii_lowercase()),
        summary: input.summary,
        detail: input.detail,
        severity,
        state: input.state,
        requires_attention,
        occurred_at: now_secs(),
        raw: Some(raw.to_string()),
    }
}

pub fn record_long_task_activity(
    state: &AppState,
    input: LongTaskActivityInput,
) -> AgentActivityEvent {
    let event = state.record_agent_activity_event(long_task_event_input(input));
    state.emit_agent_activity_event(&event);
    event
}

pub fn record_trace_agent_activity(
    state: &AppState,
    ctx: &AgentContext,
    input: TraceAgentActivityInput,
) -> AgentActivityEvent {
    let (severity, state_kind, requires_attention) = severity_for_trace(&input.kind, &input.phase);
    let raw = trace_raw_payload(&input);
    let event = state.record_agent_activity_event(AgentActivityEventInput {
        session_id: ctx.session_id.clone(),
        thread_id: input.thread_id.clone(),
        message_id: input.message_id.clone(),
        version_id: input.version_id.clone(),
        actor_kind: AgentActivityActorKind::Agent,
        actor_id: ctx.session_id.clone(),
        actor_label: ctx.agent_label.clone(),
        kind: AgentActivityKind::Trace,
        lifecycle_key: Some(lifecycle_key_for_trace(ctx, &input)),
        phase: Some(input.phase),
        summary: input.summary,
        detail: input.details,
        severity,
        state: state_kind,
        requires_attention,
        occurred_at: now_secs(),
        raw: Some(raw.to_string()),
    });
    state.emit_agent_activity_event(&event);
    event
}

pub fn record_runtime_agent_activity(
    state: &AppState,
    snapshot: &AutoAgentRuntimeSnapshot,
) -> AgentActivityEvent {
    let (severity, state_kind, requires_attention) = match &snapshot.phase {
        AutoAgentRuntimePhase::Error => (
            AgentActivitySeverity::Error,
            AgentActivityState::Failed,
            true,
        ),
        AutoAgentRuntimePhase::Waiting => (
            AgentActivitySeverity::Question,
            AgentActivityState::Active,
            true,
        ),
        AutoAgentRuntimePhase::Sleeping | AutoAgentRuntimePhase::Disconnected => (
            AgentActivitySeverity::Info,
            AgentActivityState::Resolved,
            false,
        ),
        AutoAgentRuntimePhase::Waking | AutoAgentRuntimePhase::Active => (
            AgentActivitySeverity::Info,
            AgentActivityState::Active,
            false,
        ),
    };
    let session_id = snapshot
        .session_id
        .clone()
        .unwrap_or_else(|| format!("runtime:{}", snapshot.agent_id));
    let phase = snapshot.phase.as_str().to_string();
    let summary = snapshot
        .status_text
        .clone()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| format!("{} is {}.", snapshot.agent_label, phase));
    let raw = serde_json::json!({
        "providerKind": snapshot.provider_kind,
        "llmModelLabel": snapshot.llm_model_label,
        "modelId": snapshot.pending_model_id,
        "busy": snapshot.busy,
        "activityLabel": snapshot.activity_label,
        "activityStartedAt": snapshot.activity_started_at,
        "attentionKind": snapshot.attention_kind,
        "waitingOnPrompt": snapshot.waiting_on_prompt,
    });
    let event = state.record_agent_activity_event(AgentActivityEventInput {
        session_id: session_id.clone(),
        thread_id: snapshot.pending_thread_id.clone(),
        message_id: snapshot.pending_message_id.clone(),
        version_id: None,
        actor_kind: AgentActivityActorKind::Agent,
        actor_id: snapshot.agent_id.clone(),
        actor_label: snapshot.agent_label.clone(),
        kind: AgentActivityKind::Runtime,
        lifecycle_key: Some(format!(
            "runtime:{}:{}",
            snapshot.agent_id,
            snapshot
                .pending_thread_id
                .as_deref()
                .unwrap_or("threadless"),
        )),
        phase: Some(phase),
        summary,
        detail: snapshot.last_error.clone(),
        severity,
        state: state_kind,
        requires_attention,
        occurred_at: snapshot.updated_at,
        raw: Some(raw.to_string()),
    });
    state.emit_agent_activity_event(&event);
    event
}

#[cfg(test)]
mod long_task_tests {
    use super::*;

    #[test]
    fn active_and_terminal_events_share_lifecycle_and_hide_cancel_after_completion() {
        let active = long_task_event_input(LongTaskActivityInput {
            session_id: "ui-fem".into(),
            thread_id: Some("thread-1".into()),
            message_id: Some("message-1".into()),
            actor_kind: AgentActivityActorKind::System,
            actor_id: "fem".into(),
            actor_label: "FEM".into(),
            job_id: "fem-solve-1".into(),
            stage: "SOLVE".into(),
            summary: "bracket-static".into(),
            detail: Some("Factoring.".into()),
            progress_current: 1,
            progress_total: 3,
            expected_duration_ms: 600_000,
            state: AgentActivityState::Active,
            cancellable: true,
        });
        let terminal = long_task_event_input(LongTaskActivityInput {
            state: AgentActivityState::Resolved,
            stage: "DONE".into(),
            summary: "bracket-static complete".into(),
            progress_current: 3,
            ..LongTaskActivityInput {
                session_id: "ui-fem".into(),
                thread_id: Some("thread-1".into()),
                message_id: Some("message-1".into()),
                actor_kind: AgentActivityActorKind::System,
                actor_id: "fem".into(),
                actor_label: "FEM".into(),
                job_id: "fem-solve-1".into(),
                stage: "SOLVE".into(),
                summary: "bracket-static".into(),
                detail: Some("Published.".into()),
                progress_current: 1,
                progress_total: 3,
                expected_duration_ms: 600_000,
                state: AgentActivityState::Active,
                cancellable: true,
            }
        });

        assert_eq!(active.lifecycle_key, terminal.lifecycle_key);
        assert_eq!(active.state, AgentActivityState::Active);
        assert_eq!(terminal.state, AgentActivityState::Resolved);
        let active_raw: serde_json::Value =
            serde_json::from_str(active.raw.as_deref().unwrap()).unwrap();
        let terminal_raw: serde_json::Value =
            serde_json::from_str(terminal.raw.as_deref().unwrap()).unwrap();
        assert_eq!(active_raw["kind"], "long_task_progress");
        assert_eq!(active_raw["cancellable"], true);
        assert_eq!(terminal_raw["kind"], "long_task_finished");
        assert_eq!(terminal_raw["cancellable"], false);
    }
}
