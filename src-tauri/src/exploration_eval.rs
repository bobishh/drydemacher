//! Offline route evaluation. No production escalation changes without paired evidence.

use crate::contracts::exploration_cycle::VerificationVerdict;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum EvalScenarioKind {
    ParameterEdit,
    TopologyChange,
    ConstraintRepair,
    ImageGuidedReconstruction,
    RepeatedRedRecovery,
}

pub const REPRESENTATIVE_SCENARIOS: [EvalScenarioKind; 5] = [
    EvalScenarioKind::ParameterEdit,
    EvalScenarioKind::TopologyChange,
    EvalScenarioKind::ConstraintRepair,
    EvalScenarioKind::ImageGuidedReconstruction,
    EvalScenarioKind::RepeatedRedRecovery,
];

/// Stable, deterministic input used for replaying authoring routes. These are
/// intentionally small: route quality belongs in the recorded observations,
/// while the fixture only describes the acceptance contract and known checks.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EvalFixture {
    pub id: String,
    pub scenario: EvalScenarioKind,
    pub objective: String,
    pub acceptance_criteria: Vec<String>,
    pub starting_source: String,
    pub deterministic_checks: Vec<String>,
    pub repair_issue_codes: Vec<String>,
    pub requires_independent_vision: bool,
    pub expected_red_rounds: u32,
}

impl EvalFixture {
    pub fn observation(
        &self,
        route_id: impl Into<String>,
        completed: bool,
        first_build_green: bool,
        repair_succeeded: bool,
        invalid_output: bool,
        unnecessary_versions: u32,
        latency_ms: u64,
        tokens: u64,
        cost_usd: f64,
    ) -> RouteObservation {
        RouteObservation {
            fixture_id: self.id.clone(),
            scenario: self.scenario,
            route_id: route_id.into(),
            completed,
            first_build_green,
            repair_succeeded,
            invalid_output,
            unnecessary_versions,
            latency_ms,
            tokens,
            cost_usd,
        }
    }
}

/// The five replay inputs required by the exploration-build-cycle evaluation
/// gate. Keep this function side-effect free so CI and offline reports agree.
pub fn representative_fixtures() -> Vec<EvalFixture> {
    vec![
        EvalFixture {
            id: "parameter-edit-width".into(),
            scenario: EvalScenarioKind::ParameterEdit,
            objective: "Change enclosure width while preserving the existing feature tree.".into(),
            acceptance_criteria: vec![
                "width equals requested value".into(),
                "solid remains valid".into(),
            ],
            starting_source: "(param width 40)\n(box :width width :height 30 :depth 20)".into(),
            deterministic_checks: vec!["parameter.width".into(), "solid.valid".into()],
            repair_issue_codes: vec![],
            requires_independent_vision: false,
            expected_red_rounds: 0,
        },
        EvalFixture {
            id: "topology-change-add-rib".into(),
            scenario: EvalScenarioKind::TopologyChange,
            objective: "Add one reinforcing rib without breaking the enclosure topology.".into(),
            acceptance_criteria: vec![
                "rib is present".into(),
                "result is one valid solid".into(),
            ],
            starting_source: "(box :width 40 :height 30 :depth 20)\n(repeat rib 1)".into(),
            deterministic_checks: vec!["topology.closed".into(), "solid.count=1".into()],
            repair_issue_codes: vec!["topology.non-manifold".into()],
            requires_independent_vision: false,
            expected_red_rounds: 0,
        },
        EvalFixture {
            id: "constraint-repair-overconstrained".into(),
            scenario: EvalScenarioKind::ConstraintRepair,
            objective: "Repair the sketch constraint set and preserve the requested spacing.".into(),
            acceptance_criteria: vec![
                "constraint system is solvable".into(),
                "mount spacing equals 24".into(),
            ],
            starting_source: "(sketch mount)\n(constraint distance mount 24)\n(constraint coincident mount mount)".into(),
            deterministic_checks: vec!["constraints.solvable".into(), "parameter.mount-spacing".into()],
            repair_issue_codes: vec!["constraint.overconstrained".into(), "constraint.conflict".into()],
            requires_independent_vision: false,
            expected_red_rounds: 1,
        },
        EvalFixture {
            id: "image-guided-bracket".into(),
            scenario: EvalScenarioKind::ImageGuidedReconstruction,
            objective: "Reconstruct the bracket silhouette and mounting-hole layout from the reference image.".into(),
            acceptance_criteria: vec![
                "mounting holes are dimensionally valid".into(),
                "reference silhouette is matched".into(),
            ],
            starting_source: "; reference image: bracket-front.png\n(box :width 60 :height 40 :depth 8)".into(),
            deterministic_checks: vec!["solid.valid".into(), "holes.through".into()],
            repair_issue_codes: vec!["reconstruction.ambiguous".into()],
            requires_independent_vision: true,
            expected_red_rounds: 0,
        },
        EvalFixture {
            id: "repeated-red-recovery".into(),
            scenario: EvalScenarioKind::RepeatedRedRecovery,
            objective: "Recover from repeated red drafts by applying the smallest evidence-bound repair.".into(),
            acceptance_criteria: vec!["final source parses".into(), "all deterministic checks pass".into()],
            starting_source: "(part housing)\n(fillet housing :radius missing)".into(),
            deterministic_checks: vec!["parse.valid".into(), "solid.valid".into()],
            repair_issue_codes: vec!["parse.missing-value".into(), "topology.invalid-fillet".into()],
            requires_independent_vision: false,
            expected_red_rounds: 2,
        },
    ]
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RouteObservation {
    #[serde(default)]
    pub fixture_id: String,
    pub scenario: EvalScenarioKind,
    pub route_id: String,
    pub completed: bool,
    pub first_build_green: bool,
    pub repair_succeeded: bool,
    pub invalid_output: bool,
    pub unnecessary_versions: u32,
    pub latency_ms: u64,
    pub tokens: u64,
    pub cost_usd: f64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RouteVariable {
    Provider,
    Model,
    Effort,
    PromptVersion,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RouteVariant {
    pub id: String,
    pub provider: String,
    pub model: String,
    pub effort: Option<String>,
    pub prompt_version: String,
}

impl RouteVariant {
    pub fn new(
        id: impl Into<String>,
        provider: impl Into<String>,
        model: impl Into<String>,
        effort: Option<impl Into<String>>,
        prompt_version: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            provider: provider.into(),
            model: model.into(),
            effort: effort.map(Into::into),
            prompt_version: prompt_version.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteComparisonError {
    SameRoute,
    MultipleVariables,
    MissingFixtureId,
    MismatchedFixtures,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RouteComparisonReport {
    pub changed_variable: RouteVariable,
    pub baseline: RouteAggregate,
    pub challenger: RouteAggregate,
    pub paired_fixtures: usize,
}

/// Compare paired routes while enforcing the eval protocol's one-variable rule.
pub fn compare_routes(
    baseline: &RouteVariant,
    challenger: &RouteVariant,
) -> Result<RouteVariable, RouteComparisonError> {
    let differences = [
        (
            RouteVariable::Provider,
            baseline.provider != challenger.provider,
        ),
        (RouteVariable::Model, baseline.model != challenger.model),
        (RouteVariable::Effort, baseline.effort != challenger.effort),
        (
            RouteVariable::PromptVersion,
            baseline.prompt_version != challenger.prompt_version,
        ),
    ]
    .into_iter()
    .filter_map(|(variable, changed)| changed.then_some(variable))
    .collect::<Vec<_>>();
    match differences.as_slice() {
        [] => Err(RouteComparisonError::SameRoute),
        [variable] => Ok(*variable),
        _ => Err(RouteComparisonError::MultipleVariables),
    }
}

/// Aggregate a paired replay. Equal fixture IDs are required so route quality
/// cannot be attributed to a route when the underlying task changed.
pub fn compare_route_observations(
    baseline_route: &RouteVariant,
    challenger_route: &RouteVariant,
    baseline: &[RouteObservation],
    challenger: &[RouteObservation],
) -> Result<RouteComparisonReport, RouteComparisonError> {
    let changed_variable = compare_routes(baseline_route, challenger_route)?;
    if baseline
        .iter()
        .any(|observation| observation.fixture_id.is_empty())
        || challenger
            .iter()
            .any(|observation| observation.fixture_id.is_empty())
    {
        return Err(RouteComparisonError::MissingFixtureId);
    }
    let baseline_ids = baseline
        .iter()
        .map(|observation| observation.fixture_id.clone())
        .collect::<BTreeSet<_>>();
    let challenger_ids = challenger
        .iter()
        .map(|observation| observation.fixture_id.clone())
        .collect::<BTreeSet<_>>();
    if baseline_ids.is_empty() || baseline_ids != challenger_ids {
        return Err(RouteComparisonError::MismatchedFixtures);
    }
    Ok(RouteComparisonReport {
        changed_variable,
        baseline: aggregate_route(baseline),
        challenger: aggregate_route(challenger),
        paired_fixtures: baseline_ids.len(),
    })
}

#[derive(Debug, Clone, PartialEq)]
pub struct RouteAggregate {
    pub samples: usize,
    pub completion_rate: f64,
    pub first_build_green_rate: f64,
    pub repair_success_rate: f64,
    pub invalid_output_rate: f64,
    pub mean_unnecessary_versions: f64,
    pub mean_latency_ms: f64,
    pub mean_tokens: f64,
    pub mean_cost_usd: f64,
}

/// Vision is an independent observation only. Deterministic evidence remains
/// the accepted verdict, including when vision reports green over deterministic red.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VisionEvaluation {
    pub fixture_id: String,
    pub route_id: String,
    pub deterministic_verdict: VerificationVerdict,
    pub vision_verdict: VerificationVerdict,
    pub accepted_verdict: VerificationVerdict,
    pub deterministic_red_cannot_be_overridden: bool,
}

pub fn evaluate_vision_route(
    fixture: &EvalFixture,
    route_id: impl Into<String>,
    deterministic_verdict: VerificationVerdict,
    vision_verdict: VerificationVerdict,
) -> VisionEvaluation {
    VisionEvaluation {
        fixture_id: fixture.id.clone(),
        route_id: route_id.into(),
        deterministic_verdict,
        vision_verdict,
        accepted_verdict: deterministic_verdict,
        deterministic_red_cannot_be_overridden: deterministic_verdict == VerificationVerdict::Red,
    }
}

pub fn aggregate_route(observations: &[RouteObservation]) -> RouteAggregate {
    let samples = observations.len();
    let denominator = samples.max(1) as f64;
    let rate = |predicate: fn(&RouteObservation) -> bool| {
        observations.iter().filter(|item| predicate(item)).count() as f64 / denominator
    };
    RouteAggregate {
        samples,
        completion_rate: rate(|item| item.completed),
        first_build_green_rate: rate(|item| item.first_build_green),
        repair_success_rate: rate(|item| item.repair_succeeded),
        invalid_output_rate: rate(|item| item.invalid_output),
        mean_unnecessary_versions: observations
            .iter()
            .map(|item| item.unnecessary_versions as f64)
            .sum::<f64>()
            / denominator,
        mean_latency_ms: observations
            .iter()
            .map(|item| item.latency_ms as f64)
            .sum::<f64>()
            / denominator,
        mean_tokens: observations
            .iter()
            .map(|item| item.tokens as f64)
            .sum::<f64>()
            / denominator,
        mean_cost_usd: observations.iter().map(|item| item.cost_usd).sum::<f64>() / denominator,
    }
}

/// Aggregate only observations for one route. An absent route is distinct from
/// a route with zero-valued metrics, which keeps incomplete eval reports visible.
pub fn aggregate_route_for(
    observations: &[RouteObservation],
    route_id: &str,
) -> Option<RouteAggregate> {
    let selected = observations
        .iter()
        .filter(|observation| observation.route_id == route_id)
        .cloned()
        .collect::<Vec<_>>();
    (!selected.is_empty()).then(|| aggregate_route(&selected))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn representative_fixtures_are_deterministic_and_cover_each_shape() {
        let fixtures = representative_fixtures();
        assert_eq!(fixtures.len(), REPRESENTATIVE_SCENARIOS.len());
        assert!(fixtures.iter().all(|fixture| {
            !fixture.id.is_empty()
                && !fixture.objective.is_empty()
                && !fixture.acceptance_criteria.is_empty()
                && !fixture.starting_source.is_empty()
                && !fixture.deterministic_checks.is_empty()
        }));
        assert!(fixtures.iter().any(|fixture| {
            fixture.scenario == EvalScenarioKind::ConstraintRepair
                && !fixture.repair_issue_codes.is_empty()
        }));
        assert!(fixtures.iter().any(|fixture| {
            fixture.scenario == EvalScenarioKind::ImageGuidedReconstruction
                && fixture.requires_independent_vision
        }));
        assert_eq!(
            fixtures
                .iter()
                .find(|fixture| fixture.scenario == EvalScenarioKind::RepeatedRedRecovery)
                .expect("repeated-red fixture")
                .expected_red_rounds,
            2
        );
        let ids: std::collections::HashSet<_> =
            fixtures.iter().map(|fixture| fixture.id.as_str()).collect();
        assert_eq!(ids.len(), fixtures.len());
    }

    #[test]
    fn fixture_observations_can_be_joined_and_aggregated_by_route() {
        let fixtures = representative_fixtures();
        let observations = fixtures
            .iter()
            .map(|fixture| {
                fixture.observation(
                    "balanced-medium",
                    true,
                    true,
                    true,
                    false,
                    0,
                    100,
                    200,
                    0.01,
                )
            })
            .collect::<Vec<_>>();
        let aggregate = aggregate_route_for(&observations, "balanced-medium").expect("route");
        assert_eq!(aggregate.samples, fixtures.len());
        assert_eq!(aggregate.completion_rate, 1.0);
        assert_eq!(aggregate.mean_tokens, 200.0);
        assert!(aggregate_route_for(&observations, "missing").is_none());
    }

    #[test]
    fn route_comparison_rejects_more_than_one_changed_variable() {
        let baseline = RouteVariant::new(
            "balanced-medium",
            "openai",
            "capable",
            Some("medium"),
            "exploration-cycle-v1",
        );
        let model_only = RouteVariant::new(
            "strong-medium",
            "openai",
            "strong",
            Some("medium"),
            "exploration-cycle-v1",
        );
        let effort_only = RouteVariant::new(
            "balanced-high",
            "openai",
            "capable",
            Some("high"),
            "exploration-cycle-v1",
        );
        let model_and_effort = RouteVariant::new(
            "strong-high",
            "openai",
            "strong",
            Some("high"),
            "exploration-cycle-v1",
        );
        assert_eq!(
            compare_routes(&baseline, &model_only),
            Ok(RouteVariable::Model)
        );
        assert_eq!(
            compare_routes(&baseline, &effort_only),
            Ok(RouteVariable::Effort)
        );
        assert_eq!(
            compare_routes(&baseline, &model_and_effort),
            Err(RouteComparisonError::MultipleVariables)
        );
    }

    #[test]
    fn paired_route_report_requires_the_same_fixture_set() {
        let fixtures = representative_fixtures();
        let baseline_route = RouteVariant::new(
            "balanced-medium",
            "openai",
            "capable",
            Some("medium"),
            "exploration-cycle-v1",
        );
        let challenger_route = RouteVariant::new(
            "strong-medium",
            "openai",
            "strong",
            Some("medium"),
            "exploration-cycle-v1",
        );
        let baseline = fixtures
            .iter()
            .map(|fixture| {
                fixture.observation("balanced-medium", true, true, true, false, 0, 10, 20, 0.01)
            })
            .collect::<Vec<_>>();
        let challenger = fixtures
            .iter()
            .map(|fixture| {
                fixture.observation("strong-medium", true, true, true, false, 0, 10, 20, 0.01)
            })
            .collect::<Vec<_>>();
        let report =
            compare_route_observations(&baseline_route, &challenger_route, &baseline, &challenger)
                .expect("paired report");
        assert_eq!(report.changed_variable, RouteVariable::Model);
        assert_eq!(report.paired_fixtures, fixtures.len());

        let mut incomplete = challenger;
        incomplete.pop();
        assert_eq!(
            compare_route_observations(&baseline_route, &challenger_route, &baseline, &incomplete),
            Err(RouteComparisonError::MismatchedFixtures)
        );
    }

    #[test]
    fn vision_report_keeps_deterministic_red_even_when_vision_is_green() {
        let report = evaluate_vision_route(
            &representative_fixtures()[2],
            "vision-route-a",
            VerificationVerdict::Red,
            VerificationVerdict::Green,
        );
        assert_eq!(report.deterministic_verdict, VerificationVerdict::Red);
        assert_eq!(report.vision_verdict, VerificationVerdict::Green);
        assert_eq!(report.accepted_verdict, VerificationVerdict::Red);
        assert!(report.deterministic_red_cannot_be_overridden);
    }

    #[test]
    fn representative_suite_covers_all_required_shapes() {
        assert_eq!(REPRESENTATIVE_SCENARIOS.len(), 5);
        assert!(REPRESENTATIVE_SCENARIOS.contains(&EvalScenarioKind::ImageGuidedReconstruction));
        assert!(REPRESENTATIVE_SCENARIOS.contains(&EvalScenarioKind::RepeatedRedRecovery));
    }
}
