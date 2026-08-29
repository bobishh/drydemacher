#![cfg(all(target_os = "macos", debug_assertions))]

use ecky_cad_lib::web_content_recovery;

#[test]
fn terminated_web_content_records_raw_reason_and_crash_loop_state() {
    web_content_recovery::acknowledge_stable();

    let first = web_content_recovery::simulate_termination_for_integration_test(
        "WKWebView terminated: memory pressure",
    );
    assert_eq!(first.termination_count, 1);
    assert!(first.automatic_reload_used);
    assert!(!first.blocked);
    assert_eq!(
        first.raw_error.as_deref(),
        Some("WKWebView terminated: memory pressure")
    );

    let second = web_content_recovery::simulate_termination_for_integration_test(
        "WKWebView terminated again before stable acknowledgement",
    );
    assert_eq!(second.termination_count, 2);
    assert!(second.blocked);
    assert_eq!(
        second.raw_error.as_deref(),
        Some("WKWebView terminated again before stable acknowledgement")
    );

    web_content_recovery::acknowledge_stable();
}
