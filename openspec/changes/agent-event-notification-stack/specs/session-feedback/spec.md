# Delta for session-feedback

## MODIFIED Requirements

### Requirement: Errors surface through the Ecky bubble only

The workbench SHALL surface error state through the Ecky notification stack and
SHALL NOT render a separate error banner. Session-level errors (render, export,
config, import, provider, and agent runtime) SHALL reach a sticky Ecky error
notification so no error is lost. Full raw error detail SHALL remain available
in session activity after notification dismissal or resolution.

#### Scenario: No standalone error banner

- GIVEN any error state is active
- WHEN the workbench renders
- THEN no `.error-banner` element is present.

#### Scenario: Session error appears in the notification stack

- GIVEN a session-level error is set
- WHEN notification presentation resolves
- THEN one Ecky card carries the error summary
- AND other concurrent notifications remain present or queued.

#### Scenario: Error card is dismissed

- GIVEN the user dismisses a sticky error notification
- WHEN the card leaves the stack
- THEN the corresponding activity event remains
- AND its raw backend or provider body remains inspectable.
