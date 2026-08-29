use crate::contracts::{AppError, AppResult};
use serde::Serialize;
use std::time::Duration;

pub const THREAD_LIST_MAX_BYTES: usize = 256 * 1024;
pub const TIMELINE_PAGE_MAX_BYTES: usize = 1024 * 1024;
pub const ACTIVITY_EVENT_MAX_BYTES: usize = 64 * 1024;
pub const VERSION_CORE_MAX_BYTES: usize = 2 * 1024 * 1024;
pub const TOPOLOGY_PAGE_MAX_BYTES: usize = 1024 * 1024;
pub const ORDINARY_JSON_MAX_BYTES: usize = 8 * 1024 * 1024;

pub fn serialized_size<T: Serialize>(value: &T) -> AppResult<usize> {
    serde_json::to_vec(value)
        .map(|bytes| bytes.len())
        .map_err(|error| AppError::internal(format!("Failed to measure IPC payload: {error}")))
}

pub fn require_serialized_budget<T: Serialize>(
    projection: &str,
    value: &T,
    allowed_bytes: usize,
    sectioned_read: &str,
) -> AppResult<usize> {
    let observed_bytes = serialized_size(value)?;
    if observed_bytes > allowed_bytes {
        return Err(AppError::validation(format!(
            "IPC projection '{projection}' is {observed_bytes} bytes; allowed {allowed_bytes} bytes. Use {sectioned_read}."
        )));
    }
    if observed_bytes > ORDINARY_JSON_MAX_BYTES {
        return Err(AppError::validation(format!(
            "IPC response is {observed_bytes} bytes; ordinary JSON ceiling is {ORDINARY_JSON_MAX_BYTES} bytes. Use {sectioned_read}."
        )));
    }
    Ok(observed_bytes)
}

pub fn bounded_text(value: &str, allowed_bytes: usize) -> String {
    if value.len() <= allowed_bytes {
        return value.to_string();
    }
    let mut boundary = allowed_bytes;
    while !value.is_char_boundary(boundary) {
        boundary = boundary.saturating_sub(1);
    }
    format!(
        "{}\n[truncated: observed {} bytes; allowed {} bytes]",
        &value[..boundary],
        value.len(),
        allowed_bytes
    )
}

pub fn observe_projection<T: Serialize>(
    command: &str,
    projection: &str,
    value: &T,
    rows: usize,
    truncated_fields: usize,
    json_columns_selected: usize,
    mutex_wait: Duration,
    mutex_hold: Duration,
) {
    if !cfg!(debug_assertions)
        || std::env::var("ECKY_PROJECTION_PROFILER").ok().as_deref() != Some("1")
    {
        return;
    }
    let bytes = serde_json::to_vec(value)
        .map(|payload| payload.len())
        .unwrap_or(0);
    eprintln!(
        "[PROJECTION] command={command} projection={projection} bytes={bytes} rows={rows} truncated={truncated_fields} json_columns={json_columns_selected} mutex_wait_us={} mutex_hold_us={}",
        mutex_wait.as_micros(),
        mutex_hold.as_micros(),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oversized_projection_reports_observed_allowed_and_sectioned_read() {
        let value = "x".repeat(128);
        let error = require_serialized_budget("test", &value, 32, "read_test_window")
            .expect_err("over budget");
        assert!(error.message.contains("observed") || error.message.contains("128"));
        assert!(error.message.contains("allowed 32"));
        assert!(error.message.contains("read_test_window"));
    }
}
