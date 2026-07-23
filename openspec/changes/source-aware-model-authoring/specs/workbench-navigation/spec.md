## MODIFIED Requirements

### Requirement: Bottom Icon Workbench Dock

The system SHALL render workbench navigation as a bottom-positioned icon dock
using Ecky Tactical Midnight visual language. Detached Sketch Workspace SHALL
remain hidden while source-aware model authoring replaces its product role.

#### Scenario: Dock appears at bottom

- GIVEN the workbench route is loaded
- WHEN the workbench navigation renders
- THEN the dock is positioned in the bottom half of the viewport
- AND the dock remains inside the viewport bounds

#### Scenario: Dock controls are accessible

- GIVEN the dock renders icon-first controls
- WHEN assistive queries read controls by role and name
- THEN Projects, Parameters, Dialogue, Ecky IR docs, Code inspector, audio,
  draw, and settings controls are available
- AND no Sketch Workspace launcher is available

#### Scenario: Stale Sketch layout stays hidden

- GIVEN persisted thread layout marks Sketch Workspace visible
- WHEN workbench restores that layout
- THEN Sketch Workspace window is not mounted or displayed
- AND persisted visibility for supported windows still restores

#### Scenario: Saved Sketch preview cannot replace current model

- GIVEN current thread has accepted model and dormant saved Sketch preview
- WHEN workbench boots
- THEN accepted model remains viewport source
- AND Sketch preview status is absent
- AND saved Sketch draft is neither loaded into viewport nor deleted
