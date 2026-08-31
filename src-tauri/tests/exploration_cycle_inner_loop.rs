#[path = "../src/contracts/exploration_cycle.rs"]
pub mod exploration_contracts;
#[path = "../src/exploration_cycle.rs"]
mod exploration_cycle;
#[path = "../src/exploration_scheduler.rs"]
mod exploration_scheduler;

mod contracts {
    pub use super::exploration_contracts as exploration_cycle;
}

use contracts::exploration_cycle::{
    CyclePhase, CycleStatus, Decision, PlanAction, PlanProposal, Verification, VerificationVerdict,
};
use exploration_cycle::{CycleError, CycleReducer, Transition};
use exploration_scheduler::{LatestWinsScheduler, SubmitResult, WorkKind};

fn build_plan(source_version_id: &str, budget_cost: u32) -> PlanProposal {
    PlanProposal {
        action: PlanAction::Build,
        source_version_id: source_version_id.to_owned(),
        hypothesis: "change radius".to_owned(),
        change_scope: "macro body".to_owned(),
        expected_evidence: "valid solid".to_owned(),
        budget_cost,
        question: None,
        blocked_decision: None,
    }
}

fn record_green(machine: &mut CycleReducer, version_id: &str) {
    machine
        .apply(Transition::BuildStarted {
            source_version_id: "version-a".to_owned(),
        })
        .unwrap();
    machine
        .apply(Transition::BuildAppended {
            result_version_id: version_id.to_owned(),
        })
        .unwrap();
    machine
        .apply(Transition::VerificationRecorded(Verification {
            version_id: version_id.to_owned(),
            input_digest: format!("digest-{version_id}"),
            evidence_ref: format!("evidence-{version_id}"),
            deterministic: VerificationVerdict::Green,
            vision: None,
        }))
        .unwrap();
}

#[test]
fn invalid_plan_rejected_without_mutating_state() {
    let mut machine = CycleReducer::start("cycle-1", "thread-1", "version-a", 2);
    let before = machine.state().clone();
    let error = machine.apply(Transition::PlanAccepted(build_plan("wrong", 1)));
    assert!(matches!(error, Err(CycleError::WrongSourceVersion { .. })));
    assert_eq!(&before, machine.state());
    assert_eq!(machine.state().phase, CyclePhase::Planning);
    assert_eq!(machine.state().budget_used, 0);
}

#[test]
fn plan_reserves_budget_and_rejects_over_budget() {
    let mut machine = CycleReducer::start("cycle-1", "thread-1", "version-a", 1);
    machine
        .apply(Transition::PlanAccepted(build_plan("version-a", 1)))
        .unwrap();
    assert_eq!(machine.state().budget_used, 1);
    let mut machine = CycleReducer::start("cycle-2", "thread-1", "version-a", 1);
    let error = machine.apply(Transition::PlanAccepted(build_plan("version-a", 2)));
    assert!(matches!(error, Err(CycleError::BudgetExceeded { .. })));
    assert_eq!(machine.state().budget_used, 0);
}

#[test]
fn invalid_ask_rejected_without_reserving_budget() {
    let mut machine = CycleReducer::start("cycle-1", "thread-1", "version-a", 2);
    let mut plan = build_plan("version-a", 1);
    plan.action = PlanAction::Ask;
    let error = machine.apply(Transition::PlanAccepted(plan));
    assert!(matches!(error, Err(CycleError::MissingQuestion)));
    assert_eq!(machine.state().budget_used, 0);
    assert_eq!(machine.state().phase, CyclePhase::Planning);
}

#[test]
fn build_verify_decide_complete_keeps_chosen_version_without_extra_version() {
    let mut machine = CycleReducer::start("cycle-1", "thread-1", "version-a", 3);
    machine
        .apply(Transition::PlanAccepted(build_plan("version-a", 1)))
        .unwrap();
    record_green(&mut machine, "version-b");
    machine
        .apply(Transition::Decided(Decision::Complete))
        .unwrap();
    assert_eq!(machine.state().current_version_id, "version-b");
    assert_eq!(
        machine.state().chosen_version_id.as_deref(),
        Some("version-b")
    );
    assert_eq!(machine.state().status, CycleStatus::Completed);
}

#[test]
fn deterministic_red_cannot_be_overridden_by_vision() {
    let mut machine = CycleReducer::start("cycle-1", "thread-1", "version-a", 3);
    machine
        .apply(Transition::PlanAccepted(build_plan("version-a", 1)))
        .unwrap();
    machine
        .apply(Transition::BuildStarted {
            source_version_id: "version-a".to_owned(),
        })
        .unwrap();
    machine
        .apply(Transition::BuildAppended {
            result_version_id: "version-b".to_owned(),
        })
        .unwrap();
    machine
        .apply(Transition::VerificationRecorded(Verification {
            version_id: "version-b".to_owned(),
            input_digest: "digest-b".to_owned(),
            evidence_ref: "evidence-b".to_owned(),
            deterministic: VerificationVerdict::Red,
            vision: Some(VerificationVerdict::Green),
        }))
        .unwrap();
    assert!(matches!(
        machine.apply(Transition::Decided(Decision::Complete)),
        Err(CycleError::DeterministicFailure)
    ));
}

#[test]
fn ask_persists_until_answer_then_replans() {
    let mut machine = CycleReducer::start("cycle-1", "thread-1", "version-a", 3);
    let mut plan = build_plan("version-a", 0);
    plan.action = PlanAction::Ask;
    plan.question = Some("Which clearance?".to_owned());
    plan.blocked_decision = Some("mounting-hole spacing".to_owned());
    machine.apply(Transition::PlanAccepted(plan)).unwrap();
    assert_eq!(machine.state().phase, CyclePhase::AwaitingInput);
    assert_eq!(
        machine.state().pending_question.as_deref(),
        Some("Which clearance?")
    );
    assert_eq!(
        machine.state().pending_blocked_decision.as_deref(),
        Some("mounting-hole spacing")
    );
    machine
        .apply(Transition::Answered("2 mm".to_owned()))
        .unwrap();
    assert_eq!(machine.state().phase, CyclePhase::Planning);
    assert!(machine.state().pending_question.is_none());
    assert_eq!(machine.state().last_answer.as_deref(), Some("2 mm"));
    assert!(machine.state().pending_blocked_decision.is_none());
}

#[test]
fn ask_and_stop_do_not_require_build_fields_but_require_exact_source() {
    let mut machine = CycleReducer::start("cycle-1", "thread-1", "version-a", 3);
    machine
        .apply(Transition::PlanAccepted(PlanProposal {
            action: PlanAction::Ask,
            source_version_id: "version-a".into(),
            hypothesis: String::new(),
            change_scope: String::new(),
            expected_evidence: String::new(),
            budget_cost: 0,
            question: Some("Which face is fixed?".into()),
            blocked_decision: Some("mounting orientation".into()),
        }))
        .unwrap();
    assert_eq!(machine.state().phase, CyclePhase::AwaitingInput);

    machine
        .apply(Transition::Answered("front face".into()))
        .unwrap();
    let wrong_source = machine.apply(Transition::PlanAccepted(PlanProposal {
        action: PlanAction::Stop,
        source_version_id: "wrong-version".into(),
        hypothesis: String::new(),
        change_scope: String::new(),
        expected_evidence: String::new(),
        budget_cost: 0,
        question: None,
        blocked_decision: None,
    }));
    assert!(matches!(
        wrong_source,
        Err(CycleError::WrongSourceVersion { .. })
    ));
    machine
        .apply(Transition::PlanAccepted(PlanProposal {
            action: PlanAction::Stop,
            source_version_id: "version-a".into(),
            hypothesis: String::new(),
            change_scope: String::new(),
            expected_evidence: String::new(),
            budget_cost: 0,
            question: None,
            blocked_decision: None,
        }))
        .unwrap();
    assert_eq!(machine.state().status, CycleStatus::Stopped);
}

#[test]
fn restart_interrupts_in_flight_cycle_without_changing_version_or_evidence() {
    let mut machine = CycleReducer::start("cycle-1", "thread-1", "version-a", 3);
    machine
        .apply(Transition::PlanAccepted(build_plan("version-a", 1)))
        .unwrap();
    machine
        .apply(Transition::BuildStarted {
            source_version_id: "version-a".to_owned(),
        })
        .unwrap();
    machine.apply(Transition::Interrupted).unwrap();
    assert_eq!(machine.state().status, CycleStatus::Interrupted);
    assert_eq!(machine.state().phase, CyclePhase::Idle);
    assert_eq!(machine.state().current_version_id, "version-a");
    assert!(machine.state().last_evidence_ref.is_none());
}

#[test]
fn scheduler_coalesces_interactive_b_c_d_to_d() {
    let mut scheduler = LatestWinsScheduler::default();
    scheduler.submit("a", WorkKind::Controller, "version-a", "A");
    assert_eq!(scheduler.start_next().unwrap().request_id, "a");
    scheduler.submit("b", WorkKind::Interactive, "version-a", "B");
    scheduler.submit("c", WorkKind::Interactive, "version-a", "C");
    assert_eq!(
        scheduler.submit("d", WorkKind::Interactive, "version-a", "D"),
        SubmitResult::ReplacedPendingInteractive
    );
    assert_eq!(scheduler.pending_interactive().unwrap().request_id, "d");
    assert!(!scheduler.publication_allowed("a"));
    scheduler.finish("a");
    assert_eq!(scheduler.start_next().unwrap().request_id, "d");
}

#[test]
fn scheduler_preserves_explicit_controller_builds() {
    let mut scheduler = LatestWinsScheduler::default();
    scheduler.submit("a", WorkKind::Controller, "version-a", "A");
    assert_eq!(scheduler.start_next().unwrap().request_id, "a");
    scheduler.submit("b", WorkKind::Controller, "version-a", "B");
    scheduler.submit("c", WorkKind::Controller, "version-a", "C");
    scheduler.submit("d", WorkKind::Interactive, "version-a", "D");
    assert_eq!(scheduler.pending_controller_count(), 2);
    scheduler.finish("a");
    assert_eq!(scheduler.start_next().unwrap().request_id, "b");
    scheduler.finish("b");
    assert_eq!(scheduler.start_next().unwrap().request_id, "c");
    scheduler.finish("c");
    assert_eq!(scheduler.start_next().unwrap().request_id, "d");
}

#[test]
fn scheduler_rejects_publication_from_stale_or_finished_request() {
    let mut scheduler = LatestWinsScheduler::default();
    scheduler.submit("a", WorkKind::Controller, "version-a", "A");
    scheduler.start_next();
    assert!(scheduler.publication_allowed("a"));
    assert!(!scheduler.publication_allowed("other"));
    scheduler.finish("a");
    assert!(!scheduler.publication_allowed("a"));
}
