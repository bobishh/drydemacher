pub(crate) fn is_empty_thread_target_message(message: &str) -> bool {
    message.contains("has no versions") || message.contains("has no successful versions")
}
