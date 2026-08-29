#[path = "../src/mcp/empty_thread_target.rs"]
mod empty_thread_target;

#[test]
fn recognizes_both_empty_thread_target_error_variants() {
    assert!(empty_thread_target::is_empty_thread_target_message(
        "Thread thread-1 has no versions."
    ));
    assert!(empty_thread_target::is_empty_thread_target_message(
        "Thread thread-1 has no successful versions."
    ));
}

#[test]
fn rejects_unrelated_target_errors() {
    assert!(!empty_thread_target::is_empty_thread_target_message(
        "Message msg-1 not found."
    ));
    assert!(!empty_thread_target::is_empty_thread_target_message(
        "Requested authoring target is stale."
    ));
}
