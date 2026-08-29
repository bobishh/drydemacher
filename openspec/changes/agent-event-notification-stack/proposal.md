# Proposal: Agent Event Notification Stack

## Intent

Make every user-relevant agent lifecycle event observable without turning Ecky
into a terminal mirror. Replace the current single-winner, non-expiring bubble
with a chronological notification stack backed by a complete session activity
journal.

Today rapid agent actions disappear between one-second polls, simultaneous
sources suppress each other, and most bubbles remain until replacement or
manual dismissal. The activity window cannot recover those lost transitions
because `App.svelte` synthesizes only the current bubble while MCP trace events
go only to the app log.

## Findings

- `src-tauri/src/contracts/agent.rs` exposes `ThreadAgentState` as one mutable
  snapshot (`phase`, `statusText`, `activityLabel`, `busy`, attention fields).
- `src/App.svelte` polls `getThreadAgentState` every second. Multiple state
  changes inside one polling interval collapse into the last snapshot.
- `src-tauri/src/mcp/handlers/mod.rs::push_trace_event` formats MCP lifecycle
  events into app logs only. It does not publish or journal typed UI events.
- `src/lib/agents/activity.ts::resolveGenieBubblePresentation` uses one ordered
  `if / else` chain. Exactly one source wins; other concurrent messages vanish
  from the bubble surface.
- `src/App.svelte` derives one `bubbleSessionEvent` from the winning string.
  Existing session activity events cover selected render/manual/preview paths,
  not the complete agent lifecycle.
- `src/lib/VertexGenie.svelte` accepts one `bubble: string`. Its component
  contract cannot render multiple notifications.
- No general bubble TTL exists. The five-minute `assistantFresh` check only
  gates assistant replies. Agent activity, errors, prompts, and status text
  live until replacement, state change, or manual dismissal.
- Dismissal is keyed by normalized text. A later distinct event with identical
  text can remain hidden.

## Scope

- Add a typed, globally ordered agent activity event contract at the Rust/Tauri
  boundary.
- Route MCP trace events and agent runtime transitions through one backend-owned
  journal-and-emit path.
- Provide cursor-based catch-up so push races, reloads, and temporary listener
  gaps do not lose events from any thread.
- Normalize backend agent events and existing frontend session events into one
  activity projection.
- Replace the single bubble winner with one app-global chronological
  notification queue rendered by a dedicated notification center.
- Give completed informational notifications deterministic TTLs; keep active,
  error, question, and action-required notifications until resolution or
  explicit dismissal.
- Preserve every event in session activity after its notification expires.
- Open the exact activity event when its notification is clicked.
- Preserve raw provider/backend error bodies in event detail.
- Keep notifications from every thread in the same center. Visually emphasize
  current-thread cards without filtering other threads.
- Remove notification ownership and notification props from `VertexGenie`.
- Mount the single center beside Ecky at the former bubble locus. Existing
  onboarding, confirmation, queue-error, and other local interaction copy uses
  that same stack instead of creating another toast or bubble surface.
- Remove periodic `getThreadAgentState` polling and its frontend snapshot
  dependency. Do not retain a polling fallback.

## Out Of Scope

- A separate agent status bar.
- Rendering live terminal stdout/stderr in bubbles or app logs.
- Treating every terminal byte, token, polling tick, or repeated identical
  snapshot as a user notification.
- Replacing dialogue history or the dedicated interactive terminal modal.
- Cross-device collaboration or a CRDT event log.
- Redesigning Ecky mascot geometry.

## Product Rules

- "Every event" means every distinct user-relevant lifecycle transition and
  tool outcome: queued, picked up, started, progress label changed, preview or
  mutation produced, waiting for input, completed, failed, canceled,
  disconnected, or superseded. Raw transport noise is excluded.
- Activity journal is complete. Notification stack is a timed projection, never
  the only copy.
- Notification center is app-global. Active-thread selection changes styling,
  not membership, order, lifetime, or history.
- Maximum four agent activity notifications render at once. Additional agent
  events wait in FIFO order and are promoted; no event is silently dropped by
  overflow.
- One current local interaction card MAY share the same visual stack. It is not
  an `AgentActivityEvent`, receives no fake backend cursor, and does not enter
  activity history.
- Active lifecycle updates reuse one notification card by `lifecycleKey`; every
  underlying transition remains a separate activity event.
- Chronological order is oldest at top, newest at bottom. Stable event identity
  prevents visual reordering during updates.
- Completed info/success cards expire after 8 seconds of visible time. Warnings
  expire after 12 seconds. Active work has no TTL. Errors, questions, and
  action-required cards remain until resolved or dismissed.
- Hover, keyboard focus, and hidden document time pause TTL countdown.
- Dismissal targets `eventId`, never message text. A later identical message is
  still visible.

## Proof Gates

- A burst of at least six ordered agent events reaches activity with no gaps and
  each event becomes visible in the four-card FIFO notification surface.
- Concurrent messages from different threads render in the same center as
  separate cards, vertically stacked, with thread/actor attribution and stable
  global order. Current-thread cards are visually emphasized.
- A completed info card expires on schedule while its activity record remains.
- Hover/focus and hidden-document time pause expiry.
- Pending work does not expire. Completion starts TTL. Error stays until
  resolution or dismissal and shows raw error detail in activity.
- Reload/listener-gap catch-up resumes from global cursor without duplicates.
- Initial load, reconnect, and gap repair use the same cursor-based event
  protocol; no snapshot polling fallback runs.
- No terminal stream text appears in notification copy or general app logs.
- Targeted unit tests, Playwright happy plus pending/error scenarios,
  `npm run typecheck`, strict OpenSpec validation, and
  `cd src-tauri && cargo check` pass.
