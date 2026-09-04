//! Per-target build admission queue.
//!
//! A target owns at most one running build. Interactive work is latest-wins
//! while it has not started; explicit controller work remains FIFO. The queue
//! returns replaced requests to its caller so the actor can notify the
//! superseded request instead of silently dropping it.

use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BuildKey(String);

impl BuildKey {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for BuildKey {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for BuildKey {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildKind {
    Interactive,
    Controller,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildRequest {
    pub request_id: String,
    pub key: BuildKey,
    pub kind: BuildKind,
    pub source_version_id: String,
    pub input: String,
    sequence: u64,
}

impl BuildRequest {
    pub fn sequence(&self) -> u64 {
        self.sequence
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubmitOutcome {
    Queued {
        request: BuildRequest,
    },
    ReplacedPendingInteractive {
        request: BuildRequest,
        superseded: BuildRequest,
    },
}

impl SubmitOutcome {
    pub fn request(&self) -> &BuildRequest {
        match self {
            Self::Queued { request } | Self::ReplacedPendingInteractive { request, .. } => request,
        }
    }

    pub fn superseded(&self) -> Option<&BuildRequest> {
        match self {
            Self::Queued { .. } => None,
            Self::ReplacedPendingInteractive { superseded, .. } => Some(superseded),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FinishOutcome {
    pub finished: BuildRequest,
    pub next: Option<BuildRequest>,
}

#[derive(Debug, Default)]
struct KeyQueue {
    running: Option<BuildRequest>,
    pending_interactive: Option<BuildRequest>,
    pending_controller: Vec<BuildRequest>,
}

/// Admission queue shared by build actors. State is intentionally in-memory;
/// persistence and actor wakeups belong to the integration layer.
#[derive(Debug, Default)]
pub struct BuildQueue {
    by_key: HashMap<BuildKey, KeyQueue>,
    next_sequence: u64,
}

impl BuildQueue {
    pub fn submit(
        &mut self,
        key: impl Into<BuildKey>,
        request_id: impl Into<String>,
        kind: BuildKind,
        source_version_id: impl Into<String>,
        input: impl Into<String>,
    ) -> SubmitOutcome {
        self.next_sequence = self.next_sequence.saturating_add(1);
        let key = key.into();
        let request = BuildRequest {
            request_id: request_id.into(),
            key: key.clone(),
            kind,
            source_version_id: source_version_id.into(),
            input: input.into(),
            sequence: self.next_sequence,
        };
        let queue = self.by_key.entry(key).or_default();

        match kind {
            BuildKind::Controller => {
                queue.pending_controller.push(request.clone());
                SubmitOutcome::Queued { request }
            }
            BuildKind::Interactive => {
                let superseded = queue.pending_interactive.replace(request.clone());
                match superseded {
                    Some(superseded) => SubmitOutcome::ReplacedPendingInteractive {
                        request,
                        superseded,
                    },
                    None => SubmitOutcome::Queued { request },
                }
            }
        }
    }

    /// Grant the next request for an idle key. Controller work has FIFO
    /// precedence; interactive work is admitted only when no controller waits.
    pub fn start_next(&mut self, key: impl Into<BuildKey>) -> Option<BuildRequest> {
        let key = key.into();
        let queue = self.by_key.get_mut(&key)?;
        if queue.running.is_some() {
            return None;
        }
        let next = if !queue.pending_controller.is_empty() {
            Some(queue.pending_controller.remove(0))
        } else {
            queue.pending_interactive.take()
        };
        queue.running = next.clone();
        next
    }

    /// Complete the running request and grant the next request atomically.
    pub fn finish(&mut self, key: impl Into<BuildKey>, request_id: &str) -> Option<FinishOutcome> {
        let key = key.into();
        let queue = self.by_key.get_mut(&key)?;
        let running = queue.running.as_ref()?;
        if running.request_id != request_id {
            return None;
        }
        let finished = queue.running.take().expect("running request checked above");
        let next = if !queue.pending_controller.is_empty() {
            Some(queue.pending_controller.remove(0))
        } else {
            queue.pending_interactive.take()
        };
        queue.running = next.clone();
        Some(FinishOutcome { finished, next })
    }

    /// Remove work that has not started. Running work stays admitted until its
    /// actor observes cancellation and calls `finish`, preserving one-build
    /// exclusivity while provider/render work is still in flight.
    pub fn remove_pending(&mut self, key: impl Into<BuildKey>, request_id: &str) -> bool {
        let key = key.into();
        let Some(queue) = self.by_key.get_mut(&key) else {
            return false;
        };
        if queue
            .pending_interactive
            .as_ref()
            .is_some_and(|request| request.request_id == request_id)
        {
            queue.pending_interactive = None;
            return true;
        }
        let before = queue.pending_controller.len();
        queue
            .pending_controller
            .retain(|request| request.request_id != request_id);
        before != queue.pending_controller.len()
    }

    /// Running work cannot publish to the active projection when newer
    /// interactive input is waiting for the same target.
    pub fn publication_allowed(&self, key: impl Into<BuildKey>, request_id: &str) -> bool {
        let Some(queue) = self.by_key.get(&key.into()) else {
            return false;
        };
        let Some(running) = queue.running.as_ref() else {
            return false;
        };
        running.request_id == request_id
            && queue
                .pending_interactive
                .as_ref()
                .is_none_or(|pending| pending.sequence <= running.sequence)
    }

    pub fn running(&self, key: impl Into<BuildKey>) -> Option<&BuildRequest> {
        self.by_key.get(&key.into())?.running.as_ref()
    }

    pub fn pending_interactive(&self, key: impl Into<BuildKey>) -> Option<&BuildRequest> {
        self.by_key.get(&key.into())?.pending_interactive.as_ref()
    }

    pub fn pending_controller_count(&self, key: impl Into<BuildKey>) -> usize {
        self.by_key
            .get(&key.into())
            .map_or(0, |queue| queue.pending_controller.len())
    }
}

#[cfg(test)]
mod tests {
    use super::{BuildKind, BuildQueue, SubmitOutcome};

    fn submit(
        queue: &mut BuildQueue,
        key: &str,
        id: &str,
        kind: BuildKind,
        input: &str,
    ) -> SubmitOutcome {
        queue.submit(key, id, kind, "version-a", input)
    }

    #[test]
    fn enforces_one_running_build_per_key() {
        let mut queue = BuildQueue::default();
        submit(&mut queue, "thread-a", "a1", BuildKind::Controller, "A");
        submit(&mut queue, "thread-a", "a2", BuildKind::Controller, "B");
        submit(&mut queue, "thread-b", "b1", BuildKind::Controller, "B");

        assert_eq!(queue.start_next("thread-a").unwrap().request_id, "a1");
        assert!(queue.start_next("thread-a").is_none());
        assert_eq!(queue.start_next("thread-b").unwrap().request_id, "b1");
    }

    #[test]
    fn replaces_only_unstarted_interactive_and_returns_superseded_request() {
        let mut queue = BuildQueue::default();
        submit(&mut queue, "thread-a", "a", BuildKind::Controller, "A");
        queue.start_next("thread-a");

        let first = submit(&mut queue, "thread-a", "b", BuildKind::Interactive, "B");
        assert!(first.superseded().is_none());
        let second = submit(&mut queue, "thread-a", "c", BuildKind::Interactive, "C");
        assert_eq!(second.superseded().unwrap().request_id, "b");
        assert_eq!(
            queue.pending_interactive("thread-a").unwrap().request_id,
            "c"
        );
        assert_eq!(queue.pending_interactive("thread-a").unwrap().input, "C");
    }

    #[test]
    fn finish_grants_controller_fifo_then_latest_interactive() {
        let mut queue = BuildQueue::default();
        submit(&mut queue, "thread-a", "a", BuildKind::Controller, "A");
        queue.start_next("thread-a");
        submit(&mut queue, "thread-a", "b", BuildKind::Controller, "B");
        submit(&mut queue, "thread-a", "c", BuildKind::Controller, "C");
        submit(&mut queue, "thread-a", "d", BuildKind::Interactive, "D");
        submit(&mut queue, "thread-a", "e", BuildKind::Interactive, "E");

        let first = queue.finish("thread-a", "a").unwrap();
        assert_eq!(first.finished.request_id, "a");
        assert_eq!(first.next.as_ref().unwrap().request_id, "b");
        let second = queue.finish("thread-a", "b").unwrap();
        assert_eq!(second.next.as_ref().unwrap().request_id, "c");
        let third = queue.finish("thread-a", "c").unwrap();
        assert_eq!(third.next.as_ref().unwrap().request_id, "e");
        assert!(queue.finish("thread-a", "e").unwrap().next.is_none());
    }

    #[test]
    fn stale_interactive_completion_cannot_publish() {
        let mut queue = BuildQueue::default();
        submit(&mut queue, "thread-a", "a", BuildKind::Interactive, "A");
        let running = queue.start_next("thread-a").unwrap();
        submit(&mut queue, "thread-a", "b", BuildKind::Interactive, "B");
        assert!(!queue.publication_allowed("thread-a", &running.request_id));
        assert!(queue.finish("thread-a", "wrong").is_none());
        assert!(!queue.publication_allowed("thread-a", &running.request_id));
    }

    #[test]
    fn cancellation_removes_only_pending_work() {
        let mut queue = BuildQueue::default();
        submit(
            &mut queue,
            "thread-a",
            "running",
            BuildKind::Controller,
            "A",
        );
        queue.start_next("thread-a");
        submit(
            &mut queue,
            "thread-a",
            "pending",
            BuildKind::Controller,
            "B",
        );

        assert!(!queue.remove_pending("thread-a", "running"));
        assert!(queue.remove_pending("thread-a", "pending"));
        assert_eq!(queue.running("thread-a").unwrap().request_id, "running");
        assert_eq!(queue.pending_controller_count("thread-a"), 0);
    }
}
