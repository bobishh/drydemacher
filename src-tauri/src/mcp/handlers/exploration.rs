use crate::contracts::exploration_cycle::{CycleEvent, CyclePacket};
use crate::contracts::exploration_run::{
    ExplorationRunOutput, StartExplorationRunInput, StopExplorationRunInput,
};
use crate::contracts::AppResult;
use crate::models::{AppState, PathResolver};

pub async fn handle_exploration_run_start(
    state: &AppState,
    app: &dyn PathResolver,
    input: StartExplorationRunInput,
) -> AppResult<ExplorationRunOutput> {
    crate::commands::exploration_run::start_exploration_run_core(input, state, app).await
}

pub async fn handle_exploration_cycle_get(
    state: &AppState,
    cycle_id: String,
) -> AppResult<CyclePacket> {
    crate::commands::exploration_cycle::get_exploration_cycle_core(cycle_id, state).await
}

pub async fn handle_active_exploration_cycle_get(
    state: &AppState,
    thread_id: String,
) -> AppResult<Option<CyclePacket>> {
    crate::commands::exploration_cycle::get_active_exploration_cycle_core(thread_id, state).await
}

pub async fn handle_exploration_cycle_events(
    state: &AppState,
    cycle_id: String,
    after_sequence: Option<u64>,
    limit: Option<usize>,
) -> AppResult<Vec<CycleEvent>> {
    crate::commands::exploration_cycle::get_exploration_cycle_events_core(
        cycle_id,
        after_sequence,
        limit,
        state,
    )
    .await
}

pub async fn handle_exploration_cycle_answer(
    state: &AppState,
    cycle_id: String,
    answer: String,
) -> AppResult<CyclePacket> {
    crate::commands::exploration_cycle::answer_exploration_cycle_core(cycle_id, answer, state).await
}

pub async fn handle_exploration_run_stop(
    state: &AppState,
    input: StopExplorationRunInput,
) -> AppResult<Option<CyclePacket>> {
    crate::commands::exploration_run::stop_exploration_run_core(input, state).await
}
