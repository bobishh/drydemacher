use crate::contracts::ThreadStatus;

/// Canonical lifecycle aggregate. Legacy SQLite columns are adapted into this
/// shape at read time; callers cannot represent finalized state without time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThreadLifecycle {
    status: ThreadStatus,
    finalized_at: Option<u64>,
    pending_confirm: Option<String>,
}

impl ThreadLifecycle {
    pub fn active() -> Self {
        Self {
            status: ThreadStatus::Active,
            finalized_at: None,
            pending_confirm: None,
        }
    }

    pub fn from_legacy(
        status: ThreadStatus,
        finalized_at: Option<u64>,
        pending_confirm: Option<String>,
    ) -> Self {
        let finalized = matches!(status, ThreadStatus::Finalized) || finalized_at.is_some();
        if finalized && finalized_at.is_some() {
            Self {
                status: ThreadStatus::Finalized,
                finalized_at,
                pending_confirm: None,
            }
        } else {
            Self {
                status: ThreadStatus::Active,
                finalized_at: None,
                pending_confirm,
            }
        }
    }

    pub fn finalize(&self, at: u64) -> Self {
        Self {
            status: ThreadStatus::Finalized,
            finalized_at: Some(at),
            pending_confirm: None,
        }
    }

    pub fn reopen(&self) -> Self {
        Self::active()
    }

    pub fn with_pending_confirm(&self, pending_confirm: Option<String>) -> Self {
        if self.status == ThreadStatus::Finalized {
            self.clone()
        } else {
            Self {
                pending_confirm,
                ..self.clone()
            }
        }
    }

    pub fn status(&self) -> ThreadStatus {
        self.status.clone()
    }

    pub fn finalized_at(&self) -> Option<u64> {
        self.finalized_at
    }

    pub fn pending_confirm(&self) -> Option<&str> {
        self.pending_confirm.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finalize_clears_pending_and_always_has_timestamp() {
        let state = ThreadLifecycle::active()
            .with_pending_confirm(Some("review".into()))
            .finalize(42);
        assert_eq!(state.status, ThreadStatus::Finalized);
        assert_eq!(state.finalized_at, Some(42));
        assert_eq!(state.pending_confirm, None);
    }

    #[test]
    fn reopen_clears_finalization_and_pending_state() {
        let state = ThreadLifecycle::active().finalize(42).reopen();
        assert_eq!(state, ThreadLifecycle::active());
    }

    #[test]
    fn pending_confirm_cannot_be_added_to_finalized_thread() {
        let state = ThreadLifecycle::active()
            .finalize(42)
            .with_pending_confirm(Some("bad".into()));
        assert_eq!(state.pending_confirm, None);
    }

    #[test]
    fn legacy_rows_normalize_invalid_combinations() {
        let missing_time =
            ThreadLifecycle::from_legacy(ThreadStatus::Finalized, None, Some("review".into()));
        assert_eq!(
            missing_time,
            ThreadLifecycle::active().with_pending_confirm(Some("review".into()))
        );

        let stale_time =
            ThreadLifecycle::from_legacy(ThreadStatus::Active, Some(9), Some("review".into()));
        assert_eq!(stale_time.status, ThreadStatus::Finalized);
        assert_eq!(stale_time.finalized_at, Some(9));
        assert_eq!(stale_time.pending_confirm, None);
    }
}
