use ecky_cad_lib::services::agent_activity::{
    AgentActivityActorKind, AgentActivityEventInput, AgentActivityJournal, AgentActivityKind,
    AgentActivitySeverity, AgentActivityState,
};

#[test]
fn journal_assigns_global_cursor_and_catch_up_from_after_cursor() {
    let mut journal = AgentActivityJournal::default();

    let first = journal.record(AgentActivityEventInput {
        session_id: "session-a".to_string(),
        thread_id: Some("thread-a".to_string()),
        message_id: None,
        version_id: None,
        actor_kind: ecky_cad_lib::contracts::AgentActivityActorKind::Agent,
        actor_id: "agent-a".to_string(),
        actor_label: "Agent A".to_string(),
        kind: AgentActivityKind::Trace,
        lifecycle_key: Some("session-a:turn-1".to_string()),
        phase: Some("working".to_string()),
        summary: "First event".to_string(),
        detail: None,
        severity: ecky_cad_lib::contracts::AgentActivitySeverity::Info,
        state: AgentActivityState::Active,
        requires_attention: false,
        occurred_at: 100,
        raw: None,
    });

    let second = journal.record(AgentActivityEventInput {
        session_id: "session-b".to_string(),
        thread_id: Some("thread-b".to_string()),
        message_id: None,
        version_id: None,
        actor_kind: ecky_cad_lib::contracts::AgentActivityActorKind::Agent,
        actor_id: "agent-b".to_string(),
        actor_label: "Agent B".to_string(),
        kind: AgentActivityKind::Trace,
        lifecycle_key: Some("session-b:turn-1".to_string()),
        phase: Some("working".to_string()),
        summary: "Second event".to_string(),
        detail: None,
        severity: ecky_cad_lib::contracts::AgentActivitySeverity::Info,
        state: AgentActivityState::Active,
        requires_attention: false,
        occurred_at: 101,
        raw: None,
    });

    assert_eq!(first.cursor, 1);
    assert_eq!(second.cursor, 2);
    assert_ne!(first.event_id, second.event_id);

    let catch_up = journal.catch_up(Some(1));
    assert_eq!(catch_up.latest_cursor, 2);
    assert_eq!(catch_up.events.len(), 1);
    assert_eq!(catch_up.events[0].cursor, 2);
    assert_eq!(catch_up.events[0].summary, "Second event");

    let all = journal.catch_up(None);
    assert_eq!(all.events.len(), 2);
    assert_eq!(all.events[0].cursor, 1);
    assert_eq!(all.events[1].cursor, 2);
}

#[test]
fn activity_event_serializes_camel_case_and_preserves_raw_error_body() {
    let mut journal = AgentActivityJournal::default();
    let event = journal.record(AgentActivityEventInput {
        session_id: "session-error".to_string(),
        thread_id: Some("thread-error".to_string()),
        message_id: Some("message-error".to_string()),
        version_id: None,
        actor_kind: AgentActivityActorKind::System,
        actor_id: "provider".to_string(),
        actor_label: "Provider".to_string(),
        kind: AgentActivityKind::Trace,
        lifecycle_key: Some("session-error:request".to_string()),
        phase: Some("error".to_string()),
        summary: "Provider request failed".to_string(),
        detail: Some("HTTP 429".to_string()),
        severity: AgentActivitySeverity::Error,
        state: AgentActivityState::Failed,
        requires_attention: true,
        occurred_at: 123,
        raw: Some("{\"error\":\"quota exceeded\"}".to_string()),
    });

    let value = serde_json::to_value(event).expect("serialize activity event");
    assert_eq!(value["sessionId"], "session-error");
    assert_eq!(value["threadId"], "thread-error");
    assert_eq!(value["messageId"], "message-error");
    assert_eq!(value["lifecycleKey"], "session-error:request");
    assert_eq!(value["requiresAttention"], true);
    assert_eq!(value["occurredAt"], 123);
    assert_eq!(value["raw"], "{\"error\":\"quota exceeded\"}");
    assert!(value.get("session_id").is_none());
    assert!(value.get("requires_attention").is_none());
}

#[test]
fn long_journal_compacts_and_returns_bounded_cursor_pages() {
    let mut journal = AgentActivityJournal::default();
    for cursor in 1..=2_500 {
        journal.record(AgentActivityEventInput {
            session_id: "session-long".to_string(),
            thread_id: Some("thread-long".to_string()),
            message_id: None,
            version_id: None,
            actor_kind: AgentActivityActorKind::Agent,
            actor_id: "agent".to_string(),
            actor_label: "Agent".to_string(),
            kind: AgentActivityKind::Trace,
            lifecycle_key: Some(format!("turn-{cursor}")),
            phase: Some("working".to_string()),
            summary: "x".repeat(64),
            detail: None,
            severity: AgentActivitySeverity::Info,
            state: AgentActivityState::Active,
            requires_attention: false,
            occurred_at: cursor,
            raw: None,
        });
    }

    let newest = journal.catch_up(None);
    assert!(newest.events.len() <= 256);
    assert_eq!(newest.events.last().map(|event| event.cursor), Some(2_500));
    assert!(newest.oldest_cursor > 1);
    assert!(newest.dropped_count > 0);
    assert!(newest.retained_bytes <= 4 * 1024 * 1024);

    let first_available = journal.catch_up(Some(0));
    assert!(first_available.events.len() <= 256);
    assert!(first_available.has_more);
    assert_eq!(
        first_available.events.first().map(|event| event.cursor),
        Some(first_available.oldest_cursor)
    );
}
