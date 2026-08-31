//! Backend-owned admission and cancellation registry for generation runs.

use crate::build_queue::{BuildKey, BuildKind, BuildQueue, SubmitOutcome};
use std::collections::HashSet;
use tokio::sync::{Mutex, Notify};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdmissionError {
    Superseded,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueueCounts {
    pub running: u32,
    pub pending: u32,
}

#[derive(Debug, Default)]
pub struct ExplorationRunRegistry {
    queue: Mutex<BuildQueue>,
    superseded: Mutex<HashSet<String>>,
    cancelled: Mutex<HashSet<String>>,
    changed: Notify,
}

impl ExplorationRunRegistry {
    pub async fn admit(
        &self,
        key: impl Into<BuildKey>,
        request_id: impl Into<String>,
        kind: BuildKind,
        source_version_id: impl Into<String>,
        input: impl Into<String>,
    ) -> Result<(), AdmissionError> {
        let key = key.into();
        let request_id = request_id.into();
        {
            let mut queue = self.queue.lock().await;
            let outcome = queue.submit(
                key.clone(),
                request_id.clone(),
                kind,
                source_version_id,
                input,
            );
            if let SubmitOutcome::ReplacedPendingInteractive { superseded, .. } = outcome {
                self.superseded.lock().await.insert(superseded.request_id);
            }
            if queue.running(key.clone()).is_none() {
                queue.start_next(key.clone());
            }
        }
        self.changed.notify_waiters();

        loop {
            let changed = self.changed.notified();
            if self.superseded.lock().await.remove(&request_id) {
                return Err(AdmissionError::Superseded);
            }
            let cancelled = self.cancelled.lock().await.contains(&request_id);
            let is_running = self
                .queue
                .lock()
                .await
                .running(key.clone())
                .is_some_and(|request| request.request_id == request_id);
            if cancelled {
                if is_running {
                    self.finish(key.clone(), &request_id).await;
                }
                self.cancelled.lock().await.remove(&request_id);
                return Err(AdmissionError::Cancelled);
            }
            if is_running {
                return Ok(());
            }
            changed.await;
        }
    }

    pub async fn finish(&self, key: impl Into<BuildKey>, request_id: &str) {
        self.queue.lock().await.finish(key.into(), request_id);
        self.changed.notify_waiters();
    }

    pub async fn cancel(&self, key: impl Into<BuildKey>, request_id: impl Into<String>) -> bool {
        let key = key.into();
        let request_id = request_id.into();
        let (running, removed_pending) = {
            let mut queue = self.queue.lock().await;
            let running = queue
                .running(key.clone())
                .is_some_and(|request| request.request_id == request_id);
            let removed_pending = !running && queue.remove_pending(key, &request_id);
            (running, removed_pending)
        };
        let matched = running || removed_pending;
        if matched {
            self.cancelled.lock().await.insert(request_id);
        }
        self.changed.notify_waiters();
        matched
    }

    pub async fn is_cancelled(&self, request_id: &str) -> bool {
        self.cancelled.lock().await.contains(request_id)
    }

    pub async fn is_running(&self, key: impl Into<BuildKey>, request_id: &str) -> bool {
        self.queue
            .lock()
            .await
            .running(key.into())
            .is_some_and(|request| request.request_id == request_id)
    }

    pub async fn publication_allowed(&self, key: impl Into<BuildKey>, request_id: &str) -> bool {
        self.queue
            .lock()
            .await
            .publication_allowed(key.into(), request_id)
    }

    pub async fn counts(&self, key: impl Into<BuildKey>) -> QueueCounts {
        let key = key.into();
        let queue = self.queue.lock().await;
        QueueCounts {
            running: u32::from(queue.running(key.clone()).is_some()),
            pending: queue.pending_controller_count(key.clone()) as u32
                + u32::from(queue.pending_interactive(key).is_some()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn latest_interactive_waiter_is_the_only_one_admitted_after_running_work() {
        let registry = std::sync::Arc::new(ExplorationRunRegistry::default());
        registry
            .admit("thread", "a", BuildKind::Controller, "v1", "A")
            .await
            .unwrap();

        let first = {
            let registry = registry.clone();
            tokio::spawn(async move {
                registry
                    .admit("thread", "b", BuildKind::Interactive, "v1", "B")
                    .await
            })
        };
        tokio::task::yield_now().await;
        let latest = {
            let registry = registry.clone();
            tokio::spawn(async move {
                registry
                    .admit("thread", "c", BuildKind::Interactive, "v1", "C")
                    .await
            })
        };
        tokio::task::yield_now().await;

        registry.finish("thread", "a").await;
        assert_eq!(first.await.unwrap(), Err(AdmissionError::Superseded));
        assert_eq!(latest.await.unwrap(), Ok(()));
    }

    #[tokio::test]
    async fn cancellation_releases_running_slot() {
        let registry = ExplorationRunRegistry::default();
        registry
            .admit("thread", "a", BuildKind::Controller, "v1", "A")
            .await
            .unwrap();
        assert!(registry.cancel("thread", "a").await);
        assert!(registry.is_cancelled("a").await);
        registry.finish("thread", "a").await;
    }

    #[tokio::test]
    async fn cancellation_removes_pending_request_without_releasing_running_actor() {
        let registry = ExplorationRunRegistry::default();
        registry
            .admit("thread", "running", BuildKind::Controller, "v1", "A")
            .await
            .unwrap();
        {
            let mut queue = registry.queue.lock().await;
            queue.submit("thread", "pending", BuildKind::Controller, "v1", "B");
        }

        assert!(registry.cancel("thread", "pending").await);

        assert!(registry.is_running("thread", "running").await);
        assert_eq!(registry.counts("thread").await.pending, 0);
    }

    #[tokio::test]
    async fn cancellation_of_unknown_request_does_not_leak_tombstone() {
        let registry = ExplorationRunRegistry::default();
        assert!(!registry.cancel("thread", "missing").await);
        assert!(!registry.is_cancelled("missing").await);
    }

    #[tokio::test]
    async fn running_identity_is_scoped_to_thread_and_request() {
        let registry = ExplorationRunRegistry::default();
        registry
            .admit("thread-a", "request-a", BuildKind::Controller, "v1", "A")
            .await
            .unwrap();

        assert!(registry.is_running("thread-a", "request-a").await);
        assert!(!registry.is_running("thread-a", "request-b").await);
        assert!(!registry.is_running("thread-b", "request-a").await);
    }
}
