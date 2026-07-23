use super::AgentContext;
use crate::models::{AppError, AppResult};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex as StdMutex, OnceLock};
use tokio::sync::{Mutex, OwnedMutexGuard};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct AuthoringActorRevision {
    generation: u64,
    revision: u64,
}

pub(super) struct AuthoringActorPublishPermit {
    token: AuthoringActorRevision,
    state: OwnedMutexGuard<AuthoringActorState>,
}

impl AuthoringActorPublishPermit {
    pub(super) fn mark_published(&mut self) {
        self.state.published_revision = Some(self.token.revision);
    }
}

static NEXT_ACTOR_GENERATION: AtomicU64 = AtomicU64::new(1);

static AUTHORING_ACTORS: OnceLock<
    StdMutex<HashMap<AuthoringActorKey, Arc<Mutex<AuthoringActorState>>>>,
> = OnceLock::new();

fn authoring_actors(
) -> &'static StdMutex<HashMap<AuthoringActorKey, Arc<Mutex<AuthoringActorState>>>> {
    AUTHORING_ACTORS.get_or_init(|| StdMutex::new(HashMap::new()))
}

fn actor_key(ctx: &AgentContext, thread_id: &str) -> AuthoringActorKey {
    AuthoringActorKey {
        session_id: ctx.session_id.clone(),
        thread_id: thread_id.to_string(),
    }
}

fn actor_state(ctx: &AgentContext, thread_id: &str) -> Arc<Mutex<AuthoringActorState>> {
    let key = actor_key(ctx, thread_id);
    authoring_actors()
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

pub(super) async fn reserve_authoring_actor_revision(
    ctx: &AgentContext,
    thread_id: &str,
) -> AuthoringActorRevision {
    let state = actor_state(ctx, thread_id);
    let mut state = state.lock().await;
    state.current_revision = state.current_revision.saturating_add(1).max(1);
    AuthoringActorRevision {
        generation: state.generation,
        revision: state.current_revision,
    }
}

pub(super) async fn acquire_authoring_actor_publish_permit(
    ctx: &AgentContext,
    thread_id: &str,
    token: AuthoringActorRevision,
) -> AppResult<AuthoringActorPublishPermit> {
    let actor_id = format!("{}:{}", ctx.session_id, thread_id);
    let state = actor_state(ctx, thread_id);
    let state = state.lock_owned().await;
    let current_revision = state.current_revision;
    let current_generation = state.generation;
    let already_published = state.published_revision == Some(token.revision);
    if token.generation != current_generation
        || token.revision != current_revision
        || already_published
    {
        let reason = if already_published {
            "already published"
        } else {
            "superseded"
        };
        return Err(AppError::with_details(
            crate::contracts::AppErrorCode::Conflict,
            format!(
                "Authoring actor render result {reason}: requested revision {}, current revision {current_revision}.",
                token.revision
            ),
            format!(
                "actorId={actor_id} requestedGeneration={} currentGeneration={current_generation} requestedRevision={} currentRevision={current_revision}",
                token.generation, token.revision
            ),
        )
        .with_operation("authoring_actor_publish"));
    }

    Ok(AuthoringActorPublishPermit { token, state })
}

pub(super) fn forget_authoring_actors_for_session(session_id: &str) {
    authoring_actors()
        .lock()
        .unwrap()
        .retain(|key, _| key.session_id != session_id);
}

pub(crate) async fn invalidate_authoring_actors_for_thread(thread_id: &str) {
    let states = {
        let mut actors = authoring_actors().lock().unwrap();
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn context(session_id: &str) -> AgentContext {
        AgentContext {
            session_id: session_id.to_string(),
            client_kind: "test".to_string(),
            host_label: "test".to_string(),
            agent_label: "test".to_string(),
            llm_model_id: None,
            llm_model_label: None,
        }
    }

    #[tokio::test]
    async fn separate_authoring_actors_do_not_share_publish_lock() {
        let ctx = context("parallel-actor-session");
        let revision_a = reserve_authoring_actor_revision(&ctx, "thread-a").await;
        let revision_b = reserve_authoring_actor_revision(&ctx, "thread-b").await;
        let _permit_a = acquire_authoring_actor_publish_permit(&ctx, "thread-a", revision_a)
            .await
            .expect("actor A permit");

        let permit_b = tokio::time::timeout(
            Duration::from_millis(50),
            acquire_authoring_actor_publish_permit(&ctx, "thread-b", revision_b),
        )
        .await
        .expect("actor B lock is independent")
        .expect("actor B permit");

        drop(permit_b);
        forget_authoring_actors_for_session(&ctx.session_id);
    }

    #[tokio::test]
    async fn old_actor_generation_cannot_publish_after_registry_recovery() {
        let ctx = context("actor-generation-session");
        let old = reserve_authoring_actor_revision(&ctx, "thread-a").await;
        forget_authoring_actors_for_session(&ctx.session_id);
        let current = reserve_authoring_actor_revision(&ctx, "thread-a").await;

        let error = match acquire_authoring_actor_publish_permit(&ctx, "thread-a", old).await {
            Ok(_) => panic!("old actor generation published"),
            Err(error) => error,
        };
        assert_eq!(error.code, crate::contracts::AppErrorCode::Conflict);
        assert!(error
            .details
            .as_deref()
            .unwrap_or_default()
            .contains("requestedGeneration"));

        acquire_authoring_actor_publish_permit(&ctx, "thread-a", current)
            .await
            .expect("current actor generation publishes");
        forget_authoring_actors_for_session(&ctx.session_id);
    }

    #[tokio::test]
    async fn ui_mutation_invalidates_every_session_actor_for_only_its_thread() {
        let ctx_a = context("ui-invalidation-session-a");
        let ctx_b = context("ui-invalidation-session-b");
        let stale_a = reserve_authoring_actor_revision(&ctx_a, "thread-a").await;
        let stale_b = reserve_authoring_actor_revision(&ctx_b, "thread-a").await;
        let other_thread = reserve_authoring_actor_revision(&ctx_a, "thread-b").await;

        invalidate_authoring_actors_for_thread("thread-a").await;

        assert!(
            acquire_authoring_actor_publish_permit(&ctx_a, "thread-a", stale_a)
                .await
                .is_err()
        );
        assert!(
            acquire_authoring_actor_publish_permit(&ctx_b, "thread-a", stale_b)
                .await
                .is_err()
        );
        acquire_authoring_actor_publish_permit(&ctx_a, "thread-b", other_thread)
            .await
            .expect("other thread remains publishable");

        forget_authoring_actors_for_session(&ctx_a.session_id);
        forget_authoring_actors_for_session(&ctx_b.session_id);
    }
}
