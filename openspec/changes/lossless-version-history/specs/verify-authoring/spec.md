# Delta for verify-authoring

## MODIFIED Requirements

### Requirement: Code inspector can seed authored verify template

The system SHALL let authors append one starter verify clause from the existing
code inspector when the visible source is a model-shaped `.ecky` buffer. Every
changed version-mode draft SHALL append an immutable version before validation
or render; docs-opened scratch snippets remain local and non-versioned.

#### Scenario: Insert verify template into model source

- GIVEN the code inspector shows model-shaped `.ecky` source without a verify
  clause
- WHEN the author triggers verify insertion
- THEN the visible source gains one top-level verify template before the model
  close
- AND the inspector reports that verify insertion succeeded.

#### Scenario: Duplicate verify insertion stays blocked

- GIVEN the code inspector shows model-shaped `.ecky` source that already
  contains a verify clause
- WHEN the author views code inspector actions
- THEN the verify insertion action is disabled
- AND existing authored verify source stays unchanged.

#### Scenario: Docs-opened snippet stays scratch-only

- GIVEN the docs window opens a snippet inside the existing code inspector
- WHEN the snippet modal is visible
- THEN apply, fork, and commit version actions are not available
- AND verify insertion action is not available
- AND source-mode or scratch-status badges are not shown
- AND the snippet remains local scratch content instead of a live version edit.

#### Scenario: Version-mode apply keeps inserted verify source

- GIVEN the workbench code inspector opens real `.ecky` version source without a
  verify clause
- WHEN the author inserts a verify template and applies the draft
- THEN exact changed source is appended as one immutable version before render
- AND head advances to that version regardless of render outcome
- AND success or raw failure evidence is attached to that version
- AND render uses source that includes the inserted top-level verify clause.

#### Scenario: Version-mode verification does not duplicate unchanged draft

- GIVEN version-mode Apply already appended the current verify-bearing source
- WHEN the author verifies without changing source again
- THEN verification attaches result metadata to that version automatically
- AND no content-identical duplicate version is appended.

#### Scenario: Version-mode Ecky source is highlighted as Ecky

- GIVEN the workbench code inspector opens real `.ecky` source
- WHEN the modal renders the editor
- THEN Ecky comments, keywords, numbers, strings, and atoms are highlighted as
  Ecky tokens
- AND the editor does not present that source as Python mode.

#### Scenario: Docs-opened Ecky snippet shows visible Ecky colors

- GIVEN the docs window opens an Ecky tutorial snippet in the code inspector
- WHEN the scratch modal renders
- THEN Ecky keyword and number tokens use the shipped tactical highlight colors
- AND the docs scratch modal remains copy-only.
