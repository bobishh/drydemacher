# Delta for session-collaboration

## MODIFIED Requirements

### Requirement: Session event log for visible collaboration

The system SHALL record every distinct user-relevant user, agent, and system
lifecycle action affecting any session as a globally ordered typed event. The
backend SHALL support cursor-based catch-up so temporary listener gaps and
reloads do not lose events from any thread.

#### Scenario: Agent proposes a macro patch

- GIVEN an agent produces a macro change
- WHEN the patch is accepted into the working design or preview draft
- THEN the session event log contains a macro patch event
- AND the event records actor, timestamp, summary, old source reference, and new
  source reference.

#### Scenario: User changes parameters

- GIVEN the user applies parameter changes
- WHEN render starts from those parameters
- THEN the session event log contains a params changed event
- AND the event includes old and new values for changed keys.

#### Scenario: System renders a model

- GIVEN a render is requested
- WHEN rendering starts and completes
- THEN the session event log contains render started and render succeeded events
- AND the success event links to the runtime artifact bundle.

#### Scenario: Render fails

- GIVEN a render fails
- WHEN the backend returns an error
- THEN the session event log contains a render failed event
- AND the event includes the raw backend error detail.

#### Scenario: Listener misses a cross-thread event burst

- GIVEN the frontend last consumed cursor 10
- AND backend cursors 11 through 15 contain events from multiple threads
- WHEN the frontend reconnects and requests activity after cursor 10
- THEN it receives cursors 11 through 15 in global order
- AND overlap with live push delivery is deduplicated by event ID.

#### Scenario: Frontend starts or reconnects

- GIVEN journaled lifecycle events already describe current agent state
- WHEN the frontend starts or reconnects
- THEN it subscribes and catches up through the global event cursor
- AND mascot, connection, phase, busy, and attention projections reconstruct
  from those events
- AND the frontend does not poll or invoke `getThreadAgentState`.

### Requirement: Session projections drive collaboration UI

The system SHALL derive notification stack, activity, preview, and code-diff UI
state from session events. Event severity MAY control notification lifetime and
style but SHALL NOT suppress another distinct event.

#### Scenario: Multiple important events exist

- GIVEN multiple warning, error, question, or agent-action events exist
- WHEN notification projection runs
- THEN it retains each distinct event in FIFO order
- AND renders up to four concurrent cards
- AND queues remaining cards without deleting their activity records.

#### Scenario: Lifecycle progress changes

- GIVEN multiple events share one lifecycle key
- WHEN notification projection runs
- THEN the visible card updates in place with stable identity
- AND activity retains every underlying lifecycle transition.

#### Scenario: Activity contains work from every thread

- GIVEN events from multiple threads or versions exist
- WHEN the session activity window opens from the active workbench
- THEN the window contains events from every thread in global order
- AND active-thread events are visually emphasized rather than exclusively
  filtered
- AND the user can inspect event detail without losing current viewport state.
