use std::collections::BTreeMap;

use ecky_fem::{
    run_bounded_sensitivity, FemAcceptanceComparison, FemAcceptanceCriterion,
    FemSensitivityCaseResult, FemSensitivityInputRange,
};

#[test]
fn bounded_sensitivity_reports_dominant_input_and_decision_reversal() {
    let evidence = run_bounded_sensitivity(
        &[FemSensitivityInputRange {
            input_name: "service-load".to_string(),
            evidence_id: "load-evidence".to_string(),
            lower_factor: 0.9,
            upper_factor: 1.1,
        }],
        &[FemAcceptanceCriterion {
            metric_id: "stress".to_string(),
            field: "vonMisesStress".to_string(),
            comparison: FemAcceptanceComparison::LessThanOrEqual,
            limit: 105.0,
            unit: "MPa".to_string(),
            requires_convergence: true,
        }],
        |factors| {
            let factor = factors["service-load"];
            Ok(FemSensitivityCaseResult {
                result_digest: format!("sha256:{factor}"),
                metric_values: BTreeMap::from([("stress".to_string(), 100.0 * factor)]),
            })
        },
    )
    .expect("bounded sensitivity");

    assert!(evidence.completed);
    assert_eq!(evidence.case_result_digests.len(), 3);
    assert!((evidence.metric_ranges[0].minimum - 90.0).abs() <= 1.0e-12);
    assert!((evidence.metric_ranges[0].maximum - 110.0).abs() <= 1.0e-12);
    assert_eq!(
        evidence.metric_ranges[0].dominant_input_name.as_deref(),
        Some("service-load")
    );
    assert!(evidence.metric_ranges[0].decision_changed);
}
