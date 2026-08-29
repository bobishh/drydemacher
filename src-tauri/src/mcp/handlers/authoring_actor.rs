use crate::contracts::AppResult;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex as StdMutex};
use tokio::sync::{Mutex, OwnedMutexGuard};

/// Process-wide monotonic generation counter. Shared across **all** registries
/// so a freshly created actor is always distinguishable from any older actor,
/// even after a registry is dropped and recreated or an actor is invalidated.
/// Keeping this global is what lets an invalidated/superseded revision token
/// reliably fail to publish regardless of which `AppState` owns it.
static NEXT_ACTOR_GENERATION: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct AuthoringActorKey {
    session_id: String,
    thread_id: String,
}

#[derive(Debug)]
struct AuthoringActorState {
    generation: u64,
    current_revision: u64,
    published_revision: Option<u64>,
}

/// AppState-scoped authoring actor registry.
///
/// Each `AppState` owns its own `Arc<AuthoringActorRegistry>`, so independent
/// `AppState` instances never share or invalidate each other's authoring actor
/// state — even when they reuse identical `session_id`/`thread_id` strings (as
/// parallel test harnesses and concurrent agent sessions routinely do). Within
/// a single `AppState`/registry, a UI mutation still invalidates **every**
/// session's actor for that thread (same-app behavior preserved). The actor
/// generation counter remains process-global (`NEXT_ACTOR_GENERATION`) so a
/// recreated actor is always newer than any superseded one.
#[derive(Debug, Default)]
pub struct AuthoringActorRegistry {
    actors: StdMutex<HashMap<AuthoringActorKey, Arc<Mutex<AuthoringActorState>>>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuthoringActorRevision {
    generation: u64,
    revision: u64,
}

pub struct AuthoringActorPublishPermit {
    token: AuthoringActorRevision,
    state: OwnedMutexGuard<AuthoringActorState>,
}

impl AuthoringActorPublishPermit {
    pub(crate) fn mark_published(&mut self) {
        self.state.published_revision = Some(self.token.revision);
    }
}

impl AuthoringActorRegistry {
    fn actor_state(&self, session_id: &str, thread_id: &str) -> Arc<Mutex<AuthoringActorState>> {
        let key = AuthoringActorKey {
            session_id: session_id.to_string(),
            thread_id: thread_id.to_string(),
        };
        self.actors
            .lock()
            .unwrap()
            .entry(key)
            .or_insert_with(|| {
                Arc::new(Mutex::new(AuthoringActorState {
                    generation: NEXT_ACTOR_GENERATION.fetch_add(1, Ordering::Relaxed),
                    current_revision: 0,
                    published_revision: None,
                }))
            })
            .clone()
    }

    pub(crate) async fn reserve_authoring_actor_revision(
        &self,
        session_id: &str,
        thread_id: &str,
    ) -> AuthoringActorRevision {
        let state = self.actor_state(session_id, thread_id);
        let mut state = state.lock().await;
        state.current_revision = state.current_revision.saturating_add(1).max(1);
        AuthoringActorRevision {
            generation: state.generation,
            revision: state.current_revision,
        }
    }

    pub(crate) async fn acquire_authoring_actor_publish_permit(
        &self,
        session_id: &str,
        thread_id: &str,
        token: AuthoringActorRevision,
    ) -> AppResult<AuthoringActorPublishPermit> {
        let state = self.actor_state(session_id, thread_id);
        let state = state.lock_owned().await;
        // Publishing is serialized, never rejected. Superseded work finalizes
        // its own immutable version; append order alone determines thread head.
        Ok(AuthoringActorPublishPermit { token, state })
    }

    pub(crate) fn forget_authoring_actors_for_session(&self, session_id: &str) {
        self.actors
            .lock()
            .unwrap()
            .retain(|key, _| key.session_id != session_id);
    }

    /// Invalidate every authoring actor for `thread_id` across **all** sessions
    /// in this registry. Because the registry is AppState-scoped, this only
    /// affects the owning `AppState`'s actors — never an independent
    /// `AppState`'s registry, even if it reuses the same `thread_id` string.
    pub(crate) async fn invalidate_authoring_actors_for_thread(&self, thread_id: &str) {
        let states = {
            let mut actors = self.actors.lock().unwrap();
            let states = actors
                .iter()
                .filter(|(key, _)| key.thread_id == thread_id)
                .map(|(_, state)| state.clone())
                .collect::<Vec<_>>();
            actors.retain(|key, _| key.thread_id != thread_id);
            states
        };

        for state in states {
            let mut state = state.lock().await;
            state.generation = NEXT_ACTOR_GENERATION.fetch_add(1, Ordering::Relaxed);
            state.current_revision = 0;
            state.published_revision = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn separate_authoring_actors_do_not_share_publish_lock() {
        let registry = AuthoringActorRegistry::default();
        let revision_a = registry
            .reserve_authoring_actor_revision("session-1", "thread-a")
            .await;
        let revision_b = registry
            .reserve_authoring_actor_revision("session-1", "thread-b")
            .await;
        let _permit_a = registry
            .acquire_authoring_actor_publish_permit("session-1", "thread-a", revision_a)
            .await
            .expect("actor A permit");

        let permit_b = tokio::time::timeout(
            Duration::from_millis(50),
            registry.acquire_authoring_actor_publish_permit("session-1", "thread-b", revision_b),
        )
        .await
        .expect("actor B lock is independent")
        .expect("actor B permit");

        drop(permit_b);
        registry.forget_authoring_actors_for_session("session-1");
    }

    #[tokio::test]
    async fn old_actor_generation_still_publishes_after_registry_recovery() {
        let registry = AuthoringActorRegistry::default();
        let old = registry
            .reserve_authoring_actor_revision("session-1", "thread-a")
            .await;
        registry.forget_authoring_actors_for_session("session-1");
        let current = registry
            .reserve_authoring_actor_revision("session-1", "thread-a")
            .await;

        registry
            .acquire_authoring_actor_publish_permit("session-1", "thread-a", old)
            .await
            .expect("older append result remains publishable");

        registry
            .acquire_authoring_actor_publish_permit("session-1", "thread-a", current)
            .await
            .expect("current actor generation publishes");
    }

    #[tokio::test]
    async fn ui_mutation_invalidates_every_session_actor_for_only_its_thread() {
        let registry = AuthoringActorRegistry::default();
        let stale_a = registry
            .reserve_authoring_actor_revision("session-a", "thread-a")
            .await;
        let stale_b = registry
            .reserve_authoring_actor_revision("session-b", "thread-a")
            .await;
        let other_thread = registry
            .reserve_authoring_actor_revision("session-a", "thread-b")
            .await;

        registry
            .invalidate_authoring_actors_for_thread("thread-a")
            .await;

        registry
            .acquire_authoring_actor_publish_permit("session-a", "thread-a", stale_a)
            .await
            .expect("session A append remains publishable");
        registry
            .acquire_authoring_actor_publish_permit("session-b", "thread-a", stale_b)
            .await
            .expect("session B append remains publishable");
        registry
            .acquire_authoring_actor_publish_permit("session-a", "thread-b", other_thread)
            .await
            .expect("other thread remains publishable");
    }

    /// Independent `AppState` instances own distinct registries. Even when they
    /// reuse identical `(session_id, thread_id)` strings, a UI mutation (thread
    /// invalidation) in one AppState's registry never reaches the other
    /// registry's actors. This is the production-safe isolation the old
    /// process-global registry could not provide, because it keyed actors only
    /// by `thread_id` and a thread-id collision across AppStates cross-fired.
    #[tokio::test]
    async fn independent_registries_do_not_invalidate_each_other_for_same_thread() {
        let app_a = AuthoringActorRegistry::default();
        let app_b = AuthoringActorRegistry::default();
        let revision_a = app_a
            .reserve_authoring_actor_revision("session-1", "thread-1")
            .await;

        // App B is an unrelated AppState, but performs a UI mutation that
        // invalidates its own thread-1 authoring actors.
        app_b
            .invalidate_authoring_actors_for_thread("thread-1")
            .await;

        let outcome = app_a
            .acquire_authoring_actor_publish_permit("session-1", "thread-1", revision_a)
            .await;
        assert!(
            outcome.is_ok(),
            "independent AppState registries must not invalidate each other's \
             authoring actors for the same thread id, but app A's reserved revision \
             was invalidated by app B's UI mutation: {:?}",
            outcome.err()
        );
    }

    /// Independent `AppState` registries must not collide even with identical
    /// `(session_id, thread_id)` strings: app B's later reservation must not
    /// supersede app A's in-flight render. The old process-global registry
    /// failed this because two AppStates sharing the same strings keyed to one
    /// actor and B's reservation bumped the revision out from under A.
    #[tokio::test]
    async fn independent_registries_isolate_authoring_actor_state_for_identical_keys() {
        let app_a = AuthoringActorRegistry::default();
        let app_b = AuthoringActorRegistry::default();

        let revision_a = app_a
            .reserve_authoring_actor_revision("session-1", "thread-1")
            .await;
        // App B is an unrelated AppState, but reuses the same (session, thread).
        let _revision_b = app_b
            .reserve_authoring_actor_revision("session-1", "thread-1")
            .await;

        let outcome = app_a
            .acquire_authoring_actor_publish_permit("session-1", "thread-1", revision_a)
            .await;
        assert!(
            outcome.is_ok(),
            "independent AppState must isolate its authoring actor registry, but \
             app A's reserved revision was superseded by app B sharing the same \
             (session_id, thread_id) string: {:?}",
            outcome.err()
        );
    }
}
