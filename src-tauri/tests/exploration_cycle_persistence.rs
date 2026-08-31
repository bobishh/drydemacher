use ecky_cad_lib::contracts::exploration_cycle::{
    CycleDefinition, CycleEvent, CycleEventType, CycleNextAction, CycleNextInput, CyclePacket,
    CyclePhase, CycleRouteMetadata, CycleStatus, PlanAction, PlanProposal, Verification,
    VerificationVerdict,
};
use ecky_cad_lib::contracts::Config;
use ecky_cad_lib::models::AppState;
use ecky_cad_lib::{db, exploration_cycle::CycleReducer, exploration_prompt, exploration_store};
use rusqlite::params;
use std::path::PathBuf;
use uuid::Uuid;

fn fixture_connection() -> rusqlite::Connection {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    conn.execute_batch(
        "PRAGMA foreign_keys = ON;
         CREATE TABLE threads (
           id TEXT PRIMARY KEY, title TEXT NOT NULL, summary TEXT NOT NULL DEFAULT '',
           created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL, deleted_at INTEGER
         );
         CREATE TABLE messages (
           id TEXT PRIMARY KEY, thread_id TEXT NOT NULL, role TEXT NOT NULL,
           status TEXT NOT NULL, output TEXT, version_input_digest TEXT,
           deleted_at INTEGER, FOREIGN KEY(thread_id) REFERENCES threads(id) ON DELETE CASCADE
         );",
    )
    .unwrap();
    exploration_store::ensure_schema(&conn).unwrap();
    conn.execute(
        "INSERT INTO threads(id, title, created_at, updated_at) VALUES ('thread-1', 'Bracket', 1, 1)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO messages(id, thread_id, role, status, output, version_input_digest)
         VALUES ('version-a', 'thread-1', 'assistant', 'success', '{}', 'digest-a')",
        [],
    )
    .unwrap();
    conn
}

fn packet(phase: CyclePhase) -> CyclePacket {
    let mut state = CycleReducer::start("cycle-1", "thread-1", "version-a", 3)
        .state()
        .clone();
    state.phase = phase;
    CyclePacket {
        base_version_id: "version-a".into(),
        state,
        definition: CycleDefinition {
            objective: "Repair bracket".into(),
            acceptance_criteria: vec!["deterministic checks green".into()],
            hard_constraints: vec!["preserve holes".into()],
            soft_preferences: vec![],
        },
        hypothesis: Some("change radius".into()),
        last_verification: None,
        last_route: None,
        event_count: 1,
        prompt_version: exploration_prompt::STATIC_PROMPT_VERSION.into(),
    }
}

fn event(phase: CyclePhase) -> CycleEvent {
    CycleEvent {
        event_id: "event-1".into(),
        cycle_id: "cycle-1".into(),
        sequence: 1,
        event_type: CycleEventType::Started,
        phase,
        source_version_id: Some("version-a".into()),
        result_version_id: None,
        evidence_ref: None,
        raw_error: None,
        render_snapshot_id: None,
        artifact_digest: None,
        route: None,
        plan: None,
        question: None,
        blocked_decision: None,
        answer: None,
        timestamp: 10,
    }
}

fn real_db_fixture() -> (PathBuf, rusqlite::Connection) {
    let path = std::env::temp_dir().join(format!(
        "ecky-exploration-restart-{}.sqlite",
        Uuid::new_v4()
    ));
    let conn = db::init_db(&path).expect("initialize production history schema");
    conn.execute(
        "INSERT INTO threads(id, title, created_at, updated_at)
         VALUES ('thread-restart', 'Restart proof', 1, 1)",
        [],
    )
    .unwrap();
    for (id, status, digest) in [
        ("version-base", "success", "digest-base"),
        ("version-current", "error", "digest-current"),
    ] {
        conn.execute(
            "INSERT INTO messages(
                id, thread_id, role, content, status, output, timestamp, version_input_digest
             ) VALUES (?1, 'thread-restart', 'assistant', ?2, ?3, '{}', 1, ?4)",
            params![id, format!("source for {id}"), status, digest],
        )
        .unwrap();
    }
    (path, conn)
}

fn test_state(conn: rusqlite::Connection) -> AppState {
    let config: Config = serde_json::from_value(serde_json::json!({
        "engines": [{
            "id": "engine-test",
            "name": "Test",
            "provider": "test",
            "apiKey": "",
            "model": "test-model",
            "baseUrl": ""
        }],
        "selectedEngineId": "engine-test"
    }))
    .expect("test config");
    AppState::new(config, None, conn)
}

fn restart_packet(phase: CyclePhase) -> CyclePacket {
    let mut state = CycleReducer::start("cycle-restart", "thread-restart", "version-base", 7)
        .state()
        .clone();
    state.phase = phase;
    state.budget_used = 3;
    state.current_version_id = "version-current".into();
    state.chosen_version_id = Some("version-current".into());
    state.last_evidence_ref = Some("evidence-current".into());
    state.pending_question = Some("Which mounting orientation?".into());
    state.pending_blocked_decision = Some("mounting orientation".into());
    CyclePacket {
        base_version_id: "version-base".into(),
        state,
        definition: CycleDefinition {
            objective: "Restore bracket exploration".into(),
            acceptance_criteria: vec!["all holes remain open".into(), "wall is green".into()],
            hard_constraints: vec!["preserve base footprint".into()],
            soft_preferences: vec!["minimize support material".into()],
        },
        hypothesis: Some("rotate the mounting face".into()),
        last_verification: Some(Verification {
            version_id: "version-current".into(),
            input_digest: "digest-current".into(),
            evidence_ref: "evidence-current".into(),
            deterministic: VerificationVerdict::Red,
            vision: None,
        }),
        last_route: Some(CycleRouteMetadata {
            prompt_version: "exploration-v7".into(),
            provider: "provider-test".into(),
            model: "model-test".into(),
            reasoning_effort: Some("high".into()),
            latency_ms: Some(321),
            input_tokens: Some(111),
            output_tokens: Some(222),
            estimated_cost_usd: Some(0.0042),
        }),
        event_count: 1,
        prompt_version: "exploration-v7".into(),
    }
}

fn restart_event(packet: &CyclePacket) -> CycleEvent {
    CycleEvent {
        event_id: "restart-event-1".into(),
        cycle_id: packet.state.cycle_id.clone(),
        sequence: packet.event_count,
        event_type: CycleEventType::Started,
        phase: packet.state.phase,
        source_version_id: Some(packet.base_version_id.clone()),
        result_version_id: Some(packet.state.current_version_id.clone()),
        evidence_ref: packet.state.last_evidence_ref.clone(),
        raw_error: None,
        render_snapshot_id: Some("snapshot-current".into()),
        artifact_digest: Some("artifact-current".into()),
        route: packet.last_route.clone(),
        plan: None,
        question: packet.state.pending_question.clone(),
        blocked_decision: packet.state.pending_blocked_decision.clone(),
        answer: packet.state.last_answer.clone(),
        timestamp: 10,
    }
}

#[test]
fn cycle_snapshot_and_append_only_event_survive_reload() {
    let conn = fixture_connection();
    let packet = packet(CyclePhase::Planning);
    exploration_store::insert_cycle(&conn, &packet, &event(CyclePhase::Planning)).unwrap();

    let (loaded, build_started) = exploration_store::load_cycle(&conn, "cycle-1")
        .unwrap()
        .unwrap();
    assert_eq!(loaded.definition.objective, "Repair bracket");
    assert_eq!(loaded.state.current_version_id, "version-a");
    assert!(!build_started);
    assert_eq!(
        exploration_store::list_events(&conn, "cycle-1", 0, 50)
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn restart_marks_only_in_flight_cycle_interrupted_without_mutating_versions() {
    let conn = fixture_connection();
    let packet = packet(CyclePhase::Building);
    exploration_store::insert_cycle(&conn, &packet, &event(CyclePhase::Building)).unwrap();
    conn.execute(
        "UPDATE exploration_cycles SET build_started = 1 WHERE cycle_id = 'cycle-1'",
        [],
    )
    .unwrap();
    let before: (String, String) = conn
        .query_row(
            "SELECT status, version_input_digest FROM messages WHERE id = 'version-a'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();

    assert_eq!(
        exploration_store::mark_in_flight_interrupted(&conn, 20).unwrap(),
        1
    );
    let (loaded, build_started) = exploration_store::load_cycle(&conn, "cycle-1")
        .unwrap()
        .unwrap();
    assert_eq!(
        loaded.state.status,
        ecky_cad_lib::contracts::exploration_cycle::CycleStatus::Interrupted
    );
    assert_eq!(loaded.state.current_version_id, "version-a");
    assert!(!build_started);
    let after: (String, String) = conn
        .query_row(
            "SELECT status, version_input_digest FROM messages WHERE id = 'version-a'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(before, after);
}

#[test]
fn provider_failure_event_does_not_create_empty_version() {
    let conn = fixture_connection();
    let packet = packet(CyclePhase::Building);
    exploration_store::insert_cycle(&conn, &packet, &event(CyclePhase::Building)).unwrap();
    let mut failed = event(CyclePhase::Building);
    failed.event_id = "event-2".into();
    failed.sequence = 2;
    failed.event_type = CycleEventType::ProviderFailed;
    failed.raw_error = Some("429 provider quota exceeded".into());
    let mut updated = packet;
    updated.event_count = 2;
    exploration_store::save_transition(&conn, &updated, false, &failed).unwrap();

    let versions: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM messages WHERE thread_id = ?1 AND output IS NOT NULL",
            params!["thread-1"],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(versions, 1);
    let events = exploration_store::list_events(&conn, "cycle-1", 0, 50).unwrap();
    assert_eq!(
        events[1].raw_error.as_deref(),
        Some("429 provider quota exceeded")
    );
}

#[test]
fn accepted_plan_event_persists_the_full_typed_turn_action() {
    let conn = fixture_connection();
    let packet = packet(CyclePhase::Planning);
    exploration_store::insert_cycle(&conn, &packet, &event(CyclePhase::Planning)).unwrap();
    let state = test_state(conn);
    let proposal = PlanProposal {
        action: PlanAction::Build,
        source_version_id: "version-a".into(),
        hypothesis: "thicken the mounting wall".into(),
        change_scope: "wall thickness only".into(),
        expected_evidence: "deterministic wall check becomes green".into(),
        budget_cost: 1,
        question: None,
        blocked_decision: None,
    };
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime
        .block_on(
            ecky_cad_lib::commands::exploration_cycle::next_exploration_cycle_core(
                CycleNextInput {
                    cycle_id: "cycle-1".into(),
                    action: CycleNextAction::Plan {
                        proposal: proposal.clone(),
                    },
                    route: None,
                },
                &state,
            ),
        )
        .expect("accept typed plan");
    let events = runtime
        .block_on(
            ecky_cad_lib::commands::exploration_cycle::get_exploration_cycle_events_core(
                "cycle-1".into(),
                Some(1),
                Some(1),
                &state,
            ),
        )
        .expect("load plan event");

    assert_eq!(events[0].plan.as_ref(), Some(&proposal));
}

#[test]
fn restart_restores_full_cycle_packet_and_ask_answer_semantics_from_production_schema() {
    // GIVEN a durable cycle snapshot containing all restart-owned context
    let (path, conn) = real_db_fixture();
    let packet = restart_packet(CyclePhase::AwaitingInput);
    exploration_store::insert_cycle(&conn, &packet, &restart_event(&packet)).unwrap();
    drop(conn);

    // WHEN the process restarts and opens the same history database
    let conn = db::init_db(&path).expect("reopen production history schema");
    let (restored, build_started) = exploration_store::load_cycle(&conn, "cycle-restart")
        .unwrap()
        .expect("restore cycle snapshot");

    // THEN phase, objective, criteria, budget, exact refs, evidence, and route survive
    assert_eq!(restored.state.phase, CyclePhase::AwaitingInput);
    assert_eq!(restored.state.status, CycleStatus::Active);
    assert_eq!(restored.definition.objective, "Restore bracket exploration");
    assert_eq!(
        restored.definition.acceptance_criteria,
        ["all holes remain open", "wall is green"]
    );
    assert_eq!(restored.state.budget, 7);
    assert_eq!(restored.state.budget_used, 3);
    assert_eq!(restored.base_version_id, "version-base");
    assert_eq!(restored.state.current_version_id, "version-current");
    assert_eq!(
        restored.state.chosen_version_id.as_deref(),
        Some("version-current")
    );
    assert_eq!(
        restored.state.last_evidence_ref.as_deref(),
        Some("evidence-current")
    );
    assert_eq!(
        restored.last_verification.as_ref().unwrap().evidence_ref,
        "evidence-current"
    );
    assert_eq!(
        restored.last_route.as_ref().unwrap().provider,
        "provider-test"
    );
    assert_eq!(restored.last_route.as_ref().unwrap().model, "model-test");
    assert_eq!(restored.last_route.as_ref().unwrap().latency_ms, Some(321));
    let started_event = exploration_store::list_events(&conn, "cycle-restart", 0, 50)
        .unwrap()
        .into_iter()
        .next()
        .expect("started event");
    assert_eq!(
        started_event.render_snapshot_id.as_deref(),
        Some("snapshot-current")
    );
    assert_eq!(
        started_event.artifact_digest.as_deref(),
        Some("artifact-current")
    );
    assert_eq!(started_event.route, restored.last_route);
    assert_eq!(
        restored.state.pending_question.as_deref(),
        Some("Which mounting orientation?")
    );
    assert_eq!(
        restored.state.pending_blocked_decision.as_deref(),
        Some("mounting orientation")
    );
    assert!(!build_started, "ASK restore must not imply running work");

    // AND an answer attaches to that persisted question and resumes planning
    let state = test_state(conn);
    let answered = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(
            ecky_cad_lib::commands::exploration_cycle::answer_exploration_cycle_core(
                "cycle-restart".into(),
                "fixed face outward".into(),
                &state,
            ),
        )
        .expect("answer persisted question");
    assert_eq!(answered.state.phase, CyclePhase::Planning);
    drop(state);

    let conn = db::init_db(&path).expect("reopen after answer");
    let (restored_answer, _) = exploration_store::load_cycle(&conn, "cycle-restart")
        .unwrap()
        .expect("restore answered cycle");
    assert_eq!(restored_answer.state.phase, CyclePhase::Planning);
    assert!(restored_answer.state.pending_question.is_none());
    assert!(restored_answer.state.pending_blocked_decision.is_none());
    assert_eq!(
        restored_answer.state.last_answer.as_deref(),
        Some("fixed face outward")
    );
    let events = exploration_store::list_events(&conn, "cycle-restart", 0, 50).unwrap();
    let answer_event = events
        .iter()
        .find(|event| event.event_type == CycleEventType::Answered)
        .expect("answer event");
    assert_eq!(
        answer_event.question.as_deref(),
        Some("Which mounting orientation?")
    );
    assert_eq!(
        answer_event.blocked_decision.as_deref(),
        Some("mounting orientation")
    );
    assert_eq!(answer_event.answer.as_deref(), Some("fixed face outward"));
    std::fs::remove_file(&path).ok();
}

#[test]
fn restart_interrupts_running_work_without_mutating_versions_or_evidence_or_resuming() {
    // GIVEN a running build over existing version/evidence state
    let (path, conn) = real_db_fixture();
    let mut packet = restart_packet(CyclePhase::Building);
    packet.state.pending_question = None;
    packet.state.pending_blocked_decision = None;
    exploration_store::insert_cycle(&conn, &packet, &restart_event(&packet)).unwrap();
    conn.execute(
        "UPDATE exploration_cycles SET build_started = 1 WHERE cycle_id = 'cycle-restart'",
        [],
    )
    .unwrap();
    let before_versions: Vec<(String, String, String)> = conn
        .prepare(
            "SELECT id, status, version_input_digest FROM messages
             WHERE thread_id = 'thread-restart' ORDER BY id",
        )
        .unwrap()
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    let before_events = exploration_store::list_events(&conn, "cycle-restart", 0, 50).unwrap();
    let before_evidence = (
        packet.state.last_evidence_ref.clone(),
        packet.last_verification.clone(),
    );
    drop(conn);

    // WHEN restart recovery runs against the same production database
    let conn = db::init_db(&path).expect("reopen production history schema");
    assert_eq!(
        exploration_store::mark_in_flight_interrupted(&conn, 30).unwrap(),
        1
    );

    // THEN only cycle state/event changes; versions/evidence are untouched and work is not resumed
    let (restored, build_started) = exploration_store::load_cycle(&conn, "cycle-restart")
        .unwrap()
        .expect("restore interrupted cycle");
    assert_eq!(restored.state.status, CycleStatus::Interrupted);
    assert_eq!(restored.state.phase, CyclePhase::Idle);
    assert_eq!(restored.state.current_version_id, "version-current");
    assert_eq!(restored.state.last_evidence_ref, before_evidence.0);
    assert_eq!(restored.last_verification, before_evidence.1);
    assert!(!build_started);
    let after_versions: Vec<(String, String, String)> = conn
        .prepare(
            "SELECT id, status, version_input_digest FROM messages
             WHERE thread_id = 'thread-restart' ORDER BY id",
        )
        .unwrap()
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(after_versions, before_versions);
    let events = exploration_store::list_events(&conn, "cycle-restart", 0, 50).unwrap();
    assert_eq!(events.len(), before_events.len() + 1);
    assert_eq!(events[0], before_events[0]);
    let interrupted = events.last().unwrap();
    assert_eq!(interrupted.event_type, CycleEventType::Interrupted);
    assert_eq!(
        interrupted.raw_error.as_deref(),
        Some("Cycle interrupted by app restart; expensive work was not resumed.")
    );
    assert_eq!(interrupted.evidence_ref, before_evidence.0);
    assert_eq!(
        exploration_store::mark_in_flight_interrupted(&conn, 31).unwrap(),
        0,
        "restart recovery must not auto-resume or duplicate interruption"
    );
    std::fs::remove_file(&path).ok();
}
