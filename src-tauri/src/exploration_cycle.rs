//! Pure four-stage exploration controller reducer.
//!
//! No persistence, provider, render, or version mutation lives here. The
//! service layer translates accepted transitions into durable events and
//! immutable version appends.

use crate::contracts::exploration_cycle::{
    CyclePhase, CycleState, CycleStatus, Decision, PlanAction, PlanProposal, Verification,
    VerificationVerdict,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CycleError {
    InvalidPhase {
        expected: CyclePhase,
        actual: CyclePhase,
    },
    Inactive,
    WrongSourceVersion {
        expected: String,
        actual: String,
    },
    BuildNotStarted,
    EmptyField(&'static str),
    BudgetExceeded {
        requested: u32,
        remaining: u32,
    },
    MissingQuestion,
    MissingVerification,
    VerificationVersionMismatch,
    VerificationDigestMismatch,
    DeterministicFailure,
    CannotCompleteWithoutGreen,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Transition {
    PlanAccepted(PlanProposal),
    BuildStarted {
        source_version_id: String,
    },
    BuildAppended {
        result_version_id: String,
    },
    VerificationRecorded(Verification),
    Decided(Decision),
    Answered(String),
    /// A build/provider/render attempt failed, but the bounded run may retry.
    /// Return to planning while keeping the cycle active and its current
    /// version/evidence durable.
    ProviderFailed,
    Stopped,
    Interrupted,
}

#[derive(Debug, Clone)]
pub struct CycleReducer {
    state: CycleState,
    verification: Option<Verification>,
    build_started: bool,
}

impl CycleReducer {
    pub fn start(
        cycle_id: impl Into<String>,
        thread_id: impl Into<String>,
        base_version_id: impl Into<String>,
        budget: u32,
    ) -> Self {
        Self {
            state: CycleState {
                cycle_id: cycle_id.into(),
                thread_id: thread_id.into(),
                phase: CyclePhase::Planning,
                status: CycleStatus::Active,
                current_version_id: base_version_id.into(),
                chosen_version_id: None,
                pending_question: None,
                pending_blocked_decision: None,
                last_answer: None,
                last_evidence_ref: None,
                budget,
                budget_used: 0,
            },
            verification: None,
            build_started: false,
        }
    }

    pub fn state(&self) -> &CycleState {
        &self.state
    }

    pub fn restore(
        state: CycleState,
        verification: Option<Verification>,
        build_started: bool,
    ) -> Self {
        Self {
            state,
            verification,
            build_started,
        }
    }

    pub fn build_started(&self) -> bool {
        self.build_started
    }

    pub fn verification(&self) -> Option<&Verification> {
        self.verification.as_ref()
    }

    pub fn apply(&mut self, transition: Transition) -> Result<(), CycleError> {
        if self.state.status != CycleStatus::Active {
            return Err(CycleError::Inactive);
        }

        match transition {
            Transition::PlanAccepted(plan) => self.accept_plan(plan),
            Transition::BuildStarted { source_version_id } => {
                self.require_phase(CyclePhase::Building)?;
                self.require_source(&source_version_id)?;
                self.build_started = true;
                Ok(())
            }
            Transition::BuildAppended { result_version_id } => {
                self.require_phase(CyclePhase::Building)?;
                if !self.build_started {
                    return Err(CycleError::BuildNotStarted);
                }
                if result_version_id.trim().is_empty() {
                    return Err(CycleError::EmptyField("resultVersionId"));
                }
                self.state.current_version_id = result_version_id;
                self.state.phase = CyclePhase::Verifying;
                self.build_started = false;
                Ok(())
            }
            Transition::VerificationRecorded(verification) => {
                self.record_verification(verification)
            }
            Transition::Decided(decision) => self.decide(decision),
            Transition::Answered(answer) => self.answer(answer),
            Transition::ProviderFailed => {
                self.state.phase = CyclePhase::Planning;
                self.build_started = false;
                Ok(())
            }
            Transition::Stopped => {
                self.state.phase = CyclePhase::Idle;
                self.state.status = CycleStatus::Stopped;
                self.build_started = false;
                Ok(())
            }
            Transition::Interrupted => {
                if !matches!(
                    self.state.phase,
                    CyclePhase::Building | CyclePhase::Verifying | CyclePhase::Deciding
                ) {
                    return Err(CycleError::InvalidPhase {
                        expected: CyclePhase::Building,
                        actual: self.state.phase,
                    });
                }
                self.state.phase = CyclePhase::Idle;
                self.state.status = CycleStatus::Interrupted;
                Ok(())
            }
        }
    }

    fn accept_plan(&mut self, plan: PlanProposal) -> Result<(), CycleError> {
        self.require_phase(CyclePhase::Planning)?;
        self.require_source(&plan.source_version_id)?;
        // Validate action-specific fields before reserving budget or changing
        // phase. Rejected plans must be observationally pure.
        let question = if plan.action == PlanAction::Ask {
            let question = plan.question.clone().ok_or(CycleError::MissingQuestion)?;
            if question.trim().is_empty() {
                return Err(CycleError::MissingQuestion);
            }
            Some(question)
        } else {
            None
        };

        match plan.action {
            PlanAction::Build => {
                self.require_build_fields(&plan)?;
                let remaining = self.state.budget.saturating_sub(self.state.budget_used);
                if plan.budget_cost > remaining {
                    return Err(CycleError::BudgetExceeded {
                        requested: plan.budget_cost,
                        remaining,
                    });
                }
                self.state.budget_used += plan.budget_cost;
                self.state.phase = CyclePhase::Building;
                self.verification = None;
                self.build_started = false;
            }
            PlanAction::Ask => {
                if plan.budget_cost != 0 {
                    return Err(CycleError::EmptyField("askBudgetCostMustBeZero"));
                }
                let blocked_decision = plan
                    .blocked_decision
                    .clone()
                    .filter(|value| !value.trim().is_empty())
                    .ok_or(CycleError::EmptyField("blockedDecision"))?;
                self.state.pending_question = question;
                self.state.pending_blocked_decision = Some(blocked_decision);
                self.state.phase = CyclePhase::AwaitingInput;
            }
            PlanAction::Stop => {
                if plan.budget_cost != 0 {
                    return Err(CycleError::EmptyField("stopBudgetCostMustBeZero"));
                }
                self.state.phase = CyclePhase::Idle;
                self.state.status = CycleStatus::Stopped;
            }
        }
        Ok(())
    }

    fn require_build_fields(&self, plan: &PlanProposal) -> Result<(), CycleError> {
        if plan.hypothesis.trim().is_empty() {
            return Err(CycleError::EmptyField("hypothesis"));
        }
        if plan.change_scope.trim().is_empty() {
            return Err(CycleError::EmptyField("changeScope"));
        }
        if plan.expected_evidence.trim().is_empty() {
            return Err(CycleError::EmptyField("expectedEvidence"));
        }
        Ok(())
    }

    fn record_verification(&mut self, verification: Verification) -> Result<(), CycleError> {
        self.require_phase(CyclePhase::Verifying)?;
        if verification.version_id != self.state.current_version_id {
            return Err(CycleError::VerificationVersionMismatch);
        }
        if verification.input_digest.trim().is_empty() {
            return Err(CycleError::VerificationDigestMismatch);
        }
        if verification.evidence_ref.trim().is_empty() {
            return Err(CycleError::EmptyField("evidenceRef"));
        }
        self.state.last_evidence_ref = Some(verification.evidence_ref.clone());
        self.verification = Some(verification);
        self.state.phase = CyclePhase::Deciding;
        Ok(())
    }

    fn decide(&mut self, decision: Decision) -> Result<(), CycleError> {
        self.require_phase(CyclePhase::Deciding)?;
        let verification = self
            .verification
            .as_ref()
            .ok_or(CycleError::MissingVerification)?;
        if verification.deterministic == VerificationVerdict::Red && decision == Decision::Complete
        {
            return Err(CycleError::DeterministicFailure);
        }
        match decision {
            Decision::Complete => {
                if verification.deterministic != VerificationVerdict::Green {
                    return Err(CycleError::CannotCompleteWithoutGreen);
                }
                self.state.chosen_version_id = Some(self.state.current_version_id.clone());
                self.state.phase = CyclePhase::Idle;
                self.state.status = CycleStatus::Completed;
            }
            Decision::Replan | Decision::Compare => {
                self.state.phase = CyclePhase::Planning;
                self.verification = None;
            }
            Decision::Ask {
                question,
                blocked_decision,
            } => {
                if question.trim().is_empty() {
                    return Err(CycleError::EmptyField("question"));
                }
                if blocked_decision.trim().is_empty() {
                    return Err(CycleError::EmptyField("blockedDecision"));
                }
                self.state.pending_question = Some(question);
                self.state.pending_blocked_decision = Some(blocked_decision);
                self.state.phase = CyclePhase::AwaitingInput;
            }
            Decision::Stop => {
                self.state.phase = CyclePhase::Idle;
                self.state.status = CycleStatus::Stopped;
            }
        }
        Ok(())
    }

    fn answer(&mut self, answer: String) -> Result<(), CycleError> {
        if self.state.phase != CyclePhase::AwaitingInput {
            return Err(CycleError::InvalidPhase {
                expected: CyclePhase::AwaitingInput,
                actual: self.state.phase,
            });
        }
        if answer.trim().is_empty() {
            return Err(CycleError::EmptyField("answer"));
        }
        self.state.pending_question = None;
        self.state.last_answer = Some(answer);
        self.state.pending_blocked_decision = None;
        self.state.phase = CyclePhase::Planning;
        Ok(())
    }

    fn require_phase(&self, expected: CyclePhase) -> Result<(), CycleError> {
        if self.state.phase == expected {
            Ok(())
        } else {
            Err(CycleError::InvalidPhase {
                expected,
                actual: self.state.phase,
            })
        }
    }

    fn require_source(&self, source_version_id: &str) -> Result<(), CycleError> {
        if source_version_id == self.state.current_version_id {
            Ok(())
        } else {
            Err(CycleError::WrongSourceVersion {
                expected: self.state.current_version_id.clone(),
                actual: source_version_id.to_owned(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{CycleReducer, Transition};
    use crate::contracts::exploration_cycle::{CyclePhase, PlanAction, PlanProposal};

    fn build_plan() -> PlanProposal {
        PlanProposal {
            action: PlanAction::Build,
            source_version_id: "base".into(),
            hypothesis: "repair source".into(),
            change_scope: "one bounded edit".into(),
            expected_evidence: "structural checks".into(),
            budget_cost: 1,
            question: None,
            blocked_decision: None,
        }
    }

    #[test]
    fn provider_failure_returns_active_cycle_to_planning_for_retry() {
        let mut reducer = CycleReducer::start("cycle", "thread", "base", 2);
        reducer
            .apply(Transition::PlanAccepted(build_plan()))
            .unwrap();
        reducer
            .apply(Transition::BuildStarted {
                source_version_id: "base".into(),
            })
            .unwrap();
        reducer.apply(Transition::ProviderFailed).unwrap();
        assert_eq!(reducer.state().phase, CyclePhase::Planning);
        assert!(!reducer.build_started());
        reducer
            .apply(Transition::PlanAccepted(build_plan()))
            .unwrap();
        assert_eq!(reducer.state().phase, CyclePhase::Building);
    }

    #[test]
    fn render_failure_after_append_returns_verifying_cycle_to_planning() {
        let mut reducer = CycleReducer::start("cycle", "thread", "base", 2);
        reducer
            .apply(Transition::PlanAccepted(build_plan()))
            .unwrap();
        reducer
            .apply(Transition::BuildStarted {
                source_version_id: "base".into(),
            })
            .unwrap();
        reducer
            .apply(Transition::BuildAppended {
                result_version_id: "red-version".into(),
            })
            .unwrap();
        assert_eq!(reducer.state().phase, CyclePhase::Verifying);

        reducer.apply(Transition::ProviderFailed).unwrap();

        assert_eq!(reducer.state().phase, CyclePhase::Planning);
        assert_eq!(reducer.state().current_version_id, "red-version");
    }
}
