//! Boundary guard for full `macro_preview_render` replacements.
//!
//! Implements the `render-snapshot-authority` requirement: a truncated
//! `target_macro_get` / `macro_buffer_get` read window cannot be submitted as a
//! full macro replacement without explicit truncation acknowledgement.
//!
//! The guard is metadata-driven: it never inspects submitted content to guess
//! whether it is a window. When [`MacroSourceWindow`] is absent, the link
//! between a read window and the replacement cannot be proven from request
//! metadata, so the guard does not act (no heuristic detection). When the
//! caller declares the window, the raw observed/window/full-size details must
//! be self-consistent, and a truncated window requires explicit
//! acknowledgement.

use crate::contracts::{AppError, AppResult};
use crate::mcp::contracts::MacroSourceWindow;

/// Validate a full `macro_preview_render` replacement against an optional
/// declared read window.
///
/// Returns `Ok(())` when no window is declared (the link cannot be proven, so
/// no heuristic detection runs) or when a declared window is self-consistent
/// and either non-truncated or explicitly acknowledged.
pub fn validate_macro_source_window_replacement(
    submitted_macro_code: &str,
    source_window: Option<&MacroSourceWindow>,
) -> AppResult<()> {
    let Some(window) = source_window else {
        return Ok(());
    };
    validate_declared_window(submitted_macro_code, window)
}

fn validate_declared_window(
    submitted_macro_code: &str,
    window: &MacroSourceWindow,
) -> AppResult<()> {
    if window.window_start_line == 0 {
        return Err(AppError::validation(
            "macro_preview_render sourceWindow.windowStartLine is 1-based and must be >= 1.",
        ));
    }
    if window.window_end_line < window.window_start_line {
        return Err(AppError::validation(format!(
            "macro_preview_render sourceWindow.windowEndLine ({}) is before windowStartLine ({}).",
            window.window_end_line, window.window_start_line
        )));
    }
    if window.full_size_line_count < window.window_end_line {
        return Err(AppError::validation(format!(
            "macro_preview_render sourceWindow.fullSizeLineCount ({}) is smaller than windowEndLine ({}).",
            window.full_size_line_count, window.window_end_line
        )));
    }

    let observed_line_count = submitted_macro_code.lines().count();
    if window.observed_line_count != observed_line_count {
        return Err(AppError::validation(format!(
            "macro_preview_render sourceWindow.observedLineCount ({}) does not match submitted macroCode line count ({}). Re-read the target with target_macro_get and resubmit matching sourceWindow details.",
            window.observed_line_count, observed_line_count
        )));
    }

    let truncated =
        window.window_start_line > 1 || window.window_end_line < window.full_size_line_count;
    if !truncated || window.acknowledges_truncation {
        return Ok(());
    }

    let deleted_line_count = window
        .full_size_line_count
        .saturating_sub(window.observed_line_count);
    Err(AppError::validation(format!(
        "macro_preview_render received a truncated target_macro_get window (lines {}..{} of fullSizeLineCount {}) as a full macro replacement. This replaces the whole target macro and would delete {} unread line(s). Set sourceWindow.acknowledgesTruncation=true to confirm, or re-read the full target with target_macro_get (omit startLine/endLine) and resubmit.",
        window.window_start_line,
        window.window_end_line,
        window.full_size_line_count,
        deleted_line_count
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::AppErrorCode;

    fn window(
        full_size_line_count: usize,
        window_start_line: usize,
        window_end_line: usize,
        observed_line_count: usize,
        acknowledges_truncation: bool,
    ) -> MacroSourceWindow {
        MacroSourceWindow {
            full_size_line_count,
            window_start_line,
            window_end_line,
            observed_line_count,
            acknowledges_truncation,
        }
    }

    // BDD outer RED -> GREEN: a truncated read window submitted as a full
    // replacement without acknowledgement is rejected.
    #[test]
    fn macro_source_window_guard_rejects_truncated_window_without_acknowledgement() {
        // Construct a truncated TargetMacroResponse-shaped window: the full
        // target has 6 lines, the read window observed only lines 3..4.
        let window_lines = ["line three", "line four"];
        let submitted_macro_code = window_lines.join("\n");
        let declared = window(6, 3, 4, window_lines.len(), false);

        let err = validate_macro_source_window_replacement(&submitted_macro_code, Some(&declared))
            .expect_err("truncated window without acknowledgement must be rejected");

        assert_eq!(err.code, AppErrorCode::Validation);
        assert!(
            err.message.contains("truncated target_macro_get window"),
            "unexpected message: {}",
            err.message
        );
        assert!(err.message.contains("lines 3..4"));
        assert!(err.message.contains("fullSizeLineCount 6"));
        assert!(err.message.contains("acknowledgesTruncation"));
        assert!(err.message.contains("delete 4 unread line"));
    }

    #[test]
    fn macro_source_window_guard_accepts_truncated_window_with_explicit_acknowledgement() {
        let window_lines = ["line three", "line four"];
        let submitted_macro_code = window_lines.join("\n");
        let declared = window(6, 3, 4, window_lines.len(), true);

        validate_macro_source_window_replacement(&submitted_macro_code, Some(&declared))
            .expect("explicitly acknowledged truncated window is allowed");
    }

    #[test]
    fn macro_source_window_guard_accepts_non_truncated_full_window_without_acknowledgement() {
        // Non-truncated compatibility case: the read window covered the whole
        // target, so no acknowledgement is required.
        let window_lines = ["one", "two", "three"];
        let submitted_macro_code = window_lines.join("\n");
        let declared = window(3, 1, 3, window_lines.len(), false);

        validate_macro_source_window_replacement(&submitted_macro_code, Some(&declared))
            .expect("non-truncated full window is allowed without acknowledgement");
    }

    #[test]
    fn macro_source_window_guard_rejects_observed_line_count_mismatch() {
        // Raw observed detail must match the submitted macroCode line count.
        let submitted_macro_code = "only one line".to_string();
        let declared = window(6, 3, 4, 2, true);

        let err = validate_macro_source_window_replacement(&submitted_macro_code, Some(&declared))
            .expect_err("observed line count mismatch must be rejected");

        assert_eq!(err.code, AppErrorCode::Validation);
        assert!(err.message.contains("observedLineCount (2)"));
        assert!(err.message.contains("submitted macroCode line count (1)"));
    }

    #[test]
    fn macro_source_window_guard_does_not_heuristically_reject_undeclared_window() {
        // When no sourceWindow is declared, the link between a read window and
        // this replacement cannot be proven from request metadata, so the guard
        // must not heuristically detect truncation from content.
        let submitted_macro_code = "looks like a fragment".to_string();

        validate_macro_source_window_replacement(&submitted_macro_code, None)
            .expect("undeclared replacement must not be heuristically rejected");
    }

    #[test]
    fn macro_source_window_guard_rejects_invalid_window_bounds() {
        let submitted_macro_code = "x".to_string();

        let zero_start = window(3, 0, 1, 1, false);
        let err =
            validate_macro_source_window_replacement(&submitted_macro_code, Some(&zero_start))
                .expect_err("zero windowStartLine must be rejected");
        assert!(err.message.contains("windowStartLine is 1-based"));

        let end_before_start = window(3, 4, 2, 1, false);
        let err = validate_macro_source_window_replacement(
            &submitted_macro_code,
            Some(&end_before_start),
        )
        .expect_err("end before start must be rejected");
        assert!(err.message.contains("before windowStartLine"));

        let full_smaller_than_end = window(2, 1, 3, 1, false);
        let err = validate_macro_source_window_replacement(
            &submitted_macro_code,
            Some(&full_smaller_than_end),
        )
        .expect_err("fullSizeLineCount < windowEndLine must be rejected");
        assert!(err.message.contains("smaller than windowEndLine"));
    }
}
