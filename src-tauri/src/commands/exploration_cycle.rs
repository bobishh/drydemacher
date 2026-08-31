use crate::contracts::exploration_cycle::{
    CycleDefinition, CycleEvent, CycleEventType, CycleNextAction, CycleNextInput, CyclePacket,
    CyclePhase, CycleStatus, StartCycleInput,
};
use crate::contracts::{AppError, AppResult};
use crate::exploration_cycle::{CycleReducer, Transition};
use crate::exploration_prompt::STATIC_PROMPT_VERSION;
use crate::exploration_store;
use crate::models::AppState;
use tauri::State;
use uuid::Uuid;

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn validation(message: impl Into<String>) -> AppError {
    AppError::validation(message.into())
}

#[tauri::command]
#[specta::specta]
pub async fn start_exploration_cycle(
    input: StartCycleInput,
    state: State<'_, AppState>,
) -> AppResult<CyclePacket> {
    start_exploration_cycle_core(input, state.inner()).await
}

pub async fn start_exploration_cycle_core(
    input: StartCycleInput,
    state: &AppState,
) -> AppResult<CyclePacket> {
    if input.objective.trim().is_empty() {
        return Err(validation("Exploration objective cannot be empty."));
    }
    if input.budget == 0 {
        return Err(validation(
            "Exploration budget must allow at least one build.",
        ));
    }
    let conn = state.db.lock().await;
    if !exploration_store::validate_version_ref(&conn, &input.thread_id, &input.base_version_id)
        .map_err(|error| AppError::persistence(error.to_string()))?
    {
        return Err(validation(format!(
            "Base version '{}' does not belong to thread '{}'.",
            input.base_version_id, input.thread_id
        )));
    }

    let cycle_id = Uuid::new_v4().to_string();
    let reducer = CycleReducer::start(
        cycle_id.clone(),
        input.thread_id,
        input.base_version_id,
        input.budget,
    );
    let mut packet = CyclePacket {
        base_version_id: reducer.state().current_version_id.clone(),
        state: reducer.state().clone(),
        definition: CycleDefinition {
            objective: input.objective,
            acceptance_criteria: input.acceptance_criteria,
            hard_constraints: input.hard_constraints,
            soft_preferences: input.soft_preferences,
        },
        hypothesis: None,
        last_verification: None,
        last_route: None,
        event_count: 1,
        prompt_version: STATIC_PROMPT_VERSION.to_string(),
    };
    normalize_definition(&mut packet.definition)?;
    let event = event_for(
        &packet,
        CycleEventType::Started,
        None,
        None,
        None,
        None,
        None,
    );
    exploration_store::insert_cycle(&conn, &packet, &event)
        .map_err(|error| AppError::persistence(error.to_string()))?;
    Ok(packet)
}

#[tauri::command]
#[specta::specta]
pub async fn get_exploration_cycle(
    cycle_id: String,
    state: State<'_, AppState>,
) -> AppResult<CyclePacket> {
    get_exploration_cycle_core(cycle_id, state.inner()).await
}

pub async fn get_exploration_cycle_core(
    cycle_id: String,
    state: &AppState,
) -> AppResult<CyclePacket> {
    let conn = state.db.lock().await;
    exploration_store::load_cycle(&conn, &cycle_id)
        .map_err(|error| AppError::persistence(error.to_string()))?
        .map(|(packet, _)| packet)
        .ok_or_else(|| validation(format!("Exploration cycle '{cycle_id}' was not found.")))
}

#[tauri::command]
#[specta::specta]
pub async fn get_active_exploration_cycle(
    thread_id: String,
    state: State<'_, AppState>,
) -> AppResult<Option<CyclePacket>> {
    get_active_exploration_cycle_core(thread_id, state.inner()).await
}

pub async fn get_active_exploration_cycle_core(
    thread_id: String,
    state: &AppState,
) -> AppResult<Option<CyclePacket>> {
    let conn = state.db.lock().await;
    exploration_store::load_latest_active_cycle_for_thread(&conn, &thread_id)
        .map(|value| value.map(|(packet, _)| packet))
        .map_err(|error| AppError::persistence(error.to_string()))
}

#[tauri::command]
#[specta::specta]
pub async fn get_exploration_cycle_events(
    cycle_id: String,
    after_sequence: Option<u64>,
    limit: Option<usize>,
    state: State<'_, AppState>,
) -> AppResult<Vec<CycleEvent>> {
    get_exploration_cycle_events_core(cycle_id, after_sequence, limit, state.inner()).await
}

pub async fn get_exploration_cycle_events_core(
    cycle_id: String,
    after_sequence: Option<u64>,
    limit: Option<usize>,
    state: &AppState,
) -> AppResult<Vec<CycleEvent>> {
    let conn = state.db.lock().await;
    exploration_store::list_events(
        &conn,
        &cycle_id,
        after_sequence.unwrap_or(0),
        limit.unwrap_or(50),
    )
    .map_err(|error| AppError::persistence(error.to_string()))
}

#[tauri::command]
#[specta::specta]
pub async fn next_exploration_cycle(
    input: CycleNextInput,
    state: State<'_, AppState>,
) -> AppResult<CyclePacket> {
    next_exploration_cycle_core(input, state.inner()).await
}

pub async fn next_exploration_cycle_core(
    input: CycleNextInput,
    state: &AppState,
) -> AppResult<CyclePacket> {
    let conn = state.db.lock().await;
    let (mut packet, build_started) = exploration_store::load_cycle(&conn, &input.cycle_id)
        .map_err(|error| AppError::persistence(error.to_string()))?
        .ok_or_else(|| {
            validation(format!(
                "Exploration cycle '{}' was not found.",
                input.cycle_id
            ))
        })?;
    let mut reducer = CycleReducer::restore(
        packet.state.clone(),
        packet.last_verification.clone(),
        build_started,
    );

    let mut source_version_id = None;
    let mut result_version_id = None;
    let mut evidence_ref = None;
    let mut raw_error = None;
    let mut render_snapshot_id = None;
    let mut artifact_digest = None;
    let mut accepted_plan = None;
    let event_type = match input.action {
        CycleNextAction::Plan { proposal } => {
            packet.hypothesis = Some(proposal.hypothesis.clone());
            source_version_id = Some(proposal.source_version_id.clone());
            accepted_plan = Some(proposal.clone());
            reducer
                .apply(Transition::PlanAccepted(proposal))
                .map_err(|error| validation(format!("Invalid PLAN transition: {error:?}")))?;
            CycleEventType::PlanAccepted
        }
        CycleNextAction::BuildStarted {
            source_version_id: source,
        } => {
            source_version_id = Some(source.clone());
            reducer
                .apply(Transition::BuildStarted {
                    source_version_id: source,
                })
                .map_err(|error| validation(format!("Invalid BUILD transition: {error:?}")))?;
            CycleEventType::BuildStarted
        }
        CycleNextAction::VersionAppended {
            result_version_id: result,
        } => {
            let unchanged = result == packet.state.current_version_id;
            if !exploration_store::validate_version_ref(&conn, &packet.state.thread_id, &result)
                .map_err(|error| AppError::persistence(error.to_string()))?
            {
                return Err(validation(format!(
                    "Result version '{}' does not belong to thread '{}'.",
                    result, packet.state.thread_id
                )));
            }
            source_version_id = Some(packet.state.current_version_id.clone());
            result_version_id = Some(result.clone());
            reducer
                .apply(Transition::BuildAppended {
                    result_version_id: result,
                })
                .map_err(|error| validation(format!("Invalid BUILD append: {error:?}")))?;
            if unchanged {
                CycleEventType::BuildUnchanged
            } else {
                CycleEventType::VersionAppended
            }
        }
        CycleNextAction::Verify {
            verification,
            raw_error: error,
            render_snapshot_id: snapshot,
            artifact_digest: digest,
        } => {
            if snapshot.as_deref().is_none_or(str::is_empty)
                || digest.as_deref().is_none_or(str::is_empty)
            {
                return Err(validation(
                    "VERIFY requires exact renderSnapshotId and artifactDigest refs.",
                ));
            }
            verify_persisted_digest(&conn, &packet.state.thread_id, &verification)?;
            result_version_id = Some(verification.version_id.clone());
            evidence_ref = Some(verification.evidence_ref.clone());
            raw_error = error;
            render_snapshot_id = snapshot;
            artifact_digest = digest;
            reducer
                .apply(Transition::VerificationRecorded(verification.clone()))
                .map_err(|error| validation(format!("Invalid VERIFY transition: {error:?}")))?;
            packet.last_verification = Some(verification);
            CycleEventType::VerificationRecorded
        }
        CycleNextAction::Decide { decision } => {
            reducer
                .apply(Transition::Decided(decision))
                .map_err(|error| validation(format!("Invalid DECIDE transition: {error:?}")))?;
            CycleEventType::DecisionRecorded
        }
        CycleNextAction::ProviderFailed { raw_error: error } => {
            if error.trim().is_empty() {
                return Err(validation("Provider failure requires rawError."));
            }
            source_version_id = Some(packet.state.current_version_id.clone());
            raw_error = Some(error);
            reducer.apply(Transition::ProviderFailed).map_err(|error| {
                validation(format!("Invalid provider failure transition: {error:?}"))
            })?;
            CycleEventType::ProviderFailed
        }
    };

    packet.state = reducer.state().clone();
    packet.last_route = input.route.clone().or(packet.last_route);
    packet.event_count += 1;
    let event = event_for(
        &packet,
        event_type,
        source_version_id,
        result_version_id,
        evidence_ref,
        raw_error,
        input.route,
    );
    let event = CycleEvent {
        render_snapshot_id,
        artifact_digest,
        plan: accepted_plan,
        question: None,
        blocked_decision: None,
        answer: None,
        ..event
    };
    exploration_store::save_transition(&conn, &packet, reducer.build_started(), &event)
        .map_err(|error| AppError::persistence(error.to_string()))?;
    Ok(packet)
}

#[tauri::command]
#[specta::specta]
pub async fn answer_exploration_cycle(
    cycle_id: String,
    answer: String,
    state: State<'_, AppState>,
) -> AppResult<CyclePacket> {
    answer_exploration_cycle_core(cycle_id, answer, state.inner()).await
}

pub async fn answer_exploration_cycle_core(
    cycle_id: String,
    answer: String,
    state: &AppState,
) -> AppResult<CyclePacket> {
    transition_without_route(
        cycle_id,
        Transition::Answered(answer),
        CycleEventType::Answered,
        state,
    )
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn stop_exploration_cycle(
    cycle_id: String,
    state: State<'_, AppState>,
) -> AppResult<CyclePacket> {
    stop_exploration_cycle_core(cycle_id, state.inner()).await
}

pub async fn stop_exploration_cycle_core(
    cycle_id: String,
    state: &AppState,
) -> AppResult<CyclePacket> {
    transition_without_route(
        cycle_id,
        Transition::Stopped,
        CycleEventType::Stopped,
        state,
    )
    .await
}

async fn transition_without_route(
    cycle_id: String,
    transition: Transition,
    event_type: CycleEventType,
    state: &AppState,
) -> AppResult<CyclePacket> {
    let conn = state.db.lock().await;
    let (mut packet, build_started) = exploration_store::load_cycle(&conn, &cycle_id)
        .map_err(|error| AppError::persistence(error.to_string()))?
        .ok_or_else(|| validation(format!("Exploration cycle '{cycle_id}' was not found.")))?;
    let mut reducer = CycleReducer::restore(
        packet.state.clone(),
        packet.last_verification.clone(),
        build_started,
    );
    let prior_question = packet.state.pending_question.clone();
    let prior_blocked_decision = packet.state.pending_blocked_decision.clone();
    reducer
        .apply(transition)
        .map_err(|error| validation(format!("Invalid cycle transition: {error:?}")))?;
    packet.state = reducer.state().clone();
    packet.event_count += 1;
    let mut event = event_for(&packet, event_type, None, None, None, None, None);
    if event_type == CycleEventType::Answered {
        event.question = prior_question;
        event.blocked_decision = prior_blocked_decision;
        event.answer = packet.state.last_answer.clone();
    }
    exploration_store::save_transition(&conn, &packet, reducer.build_started(), &event)
        .map_err(|error| AppError::persistence(error.to_string()))?;
    Ok(packet)
}

fn event_for(
    packet: &CyclePacket,
    event_type: CycleEventType,
    source_version_id: Option<String>,
    result_version_id: Option<String>,
    evidence_ref: Option<String>,
    raw_error: Option<String>,
    route: Option<crate::contracts::exploration_cycle::CycleRouteMetadata>,
) -> CycleEvent {
    CycleEvent {
        event_id: Uuid::new_v4().to_string(),
        cycle_id: packet.state.cycle_id.clone(),
        sequence: packet.event_count,
        event_type,
        phase: packet.state.phase,
        source_version_id,
        result_version_id,
        evidence_ref,
        raw_error,
        render_snapshot_id: None,
        artifact_digest: None,
        route,
        plan: None,
        question: packet.state.pending_question.clone(),
        blocked_decision: packet.state.pending_blocked_decision.clone(),
        answer: packet.state.last_answer.clone(),
        timestamp: now(),
    }
}

fn normalize_definition(definition: &mut CycleDefinition) -> AppResult<()> {
    definition.objective = definition.objective.trim().to_string();
    for values in [
        &mut definition.acceptance_criteria,
        &mut definition.hard_constraints,
        &mut definition.soft_preferences,
    ] {
        values.retain(|value| !value.trim().is_empty());
        for value in values {
            *value = value.trim().to_string();
        }
    }
    if definition.acceptance_criteria.is_empty() {
        return Err(validation(
            "Exploration requires at least one acceptance criterion.",
        ));
    }
    Ok(())
}

fn verify_persisted_digest(
    conn: &rusqlite::Connection,
    thread_id: &str,
    verification: &crate::contracts::exploration_cycle::Verification,
) -> AppResult<()> {
    let stored: Option<String> = conn
        .query_row(
            "SELECT version_input_digest FROM messages
             WHERE id = ?1 AND thread_id = ?2 AND output IS NOT NULL AND deleted_at IS NULL",
            rusqlite::params![verification.version_id, thread_id],
            |row| row.get(0),
        )
        .map_err(|error| AppError::persistence(error.to_string()))?;
    if stored.as_deref() != Some(verification.input_digest.as_str()) {
        return Err(validation(format!(
            "Verification digest '{}' does not match persisted version '{}'.",
            verification.input_digest, verification.version_id
        )));
    }
    Ok(())
}

#[allow(dead_code)]
fn _phase_anchor(_: CyclePhase, _: CycleStatus) {}
