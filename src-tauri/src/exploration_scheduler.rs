//! Pure per-cycle latest-wins scheduler.
//!
//! Interactive requests collapse to the newest unstarted request. Explicit
//! controller builds remain individually queued. Publication is allowed only
//! for the running request that still targets the latest interactive input.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkKind {
    Interactive,
    Controller,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkRequest {
    pub request_id: String,
    pub kind: WorkKind,
    pub source_version_id: String,
    pub input: String,
    sequence: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubmitResult {
    Queued,
    ReplacedPendingInteractive,
}

#[derive(Debug, Default)]
pub struct LatestWinsScheduler {
    running: Option<WorkRequest>,
    pending_interactive: Option<WorkRequest>,
    pending_controller: Vec<WorkRequest>,
    next_sequence: u64,
}

impl LatestWinsScheduler {
    pub fn submit(
        &mut self,
        request_id: impl Into<String>,
        kind: WorkKind,
        source_version_id: impl Into<String>,
        input: impl Into<String>,
    ) -> SubmitResult {
        self.next_sequence += 1;
        let request = WorkRequest {
            request_id: request_id.into(),
            kind,
            source_version_id: source_version_id.into(),
            input: input.into(),
            sequence: self.next_sequence,
        };
        match kind {
            WorkKind::Controller => {
                self.pending_controller.push(request);
                SubmitResult::Queued
            }
            WorkKind::Interactive => {
                let replaced = self.pending_interactive.is_some();
                self.pending_interactive = Some(request);
                if replaced {
                    SubmitResult::ReplacedPendingInteractive
                } else {
                    SubmitResult::Queued
                }
            }
        }
    }

    pub fn start_next(&mut self) -> Option<WorkRequest> {
        if self.running.is_some() {
            return None;
        }
        let next = self
            .pending_controller
            .is_empty()
            .then(|| self.pending_interactive.take())
            .flatten()
            .or_else(|| {
                if self.pending_controller.is_empty() {
                    None
                } else {
                    Some(self.pending_controller.remove(0))
                }
            });
        self.running = next.clone();
        next
    }

    pub fn finish(&mut self, request_id: &str) -> Option<WorkRequest> {
        if self
            .running
            .as_ref()
            .is_some_and(|request| request.request_id == request_id)
        {
            self.running.take()
        } else {
            None
        }
    }

    pub fn publication_allowed(&self, request_id: &str) -> bool {
        let Some(running) = self.running.as_ref() else {
            return false;
        };
        if running.request_id != request_id {
            return false;
        }
        self.pending_interactive
            .as_ref()
            .is_none_or(|pending| pending.sequence <= running.sequence)
    }

    pub fn running(&self) -> Option<&WorkRequest> {
        self.running.as_ref()
    }

    pub fn pending_interactive(&self) -> Option<&WorkRequest> {
        self.pending_interactive.as_ref()
    }

    pub fn pending_controller_count(&self) -> usize {
        self.pending_controller.len()
    }
}
