## MODIFIED Requirements

### Requirement: Bottom Icon Workbench Dock

The system SHALL render workbench navigation as a bottom-positioned, named horizontal
toolbar using Ecky Tactical Midnight visual language. It SHALL group persistent
workspace windows separately from transient modes and utilities, use one coherent
repo-native vector icon system, and expose labels and state without relying on color or
pointer hover alone.

#### Scenario: Dock appears at bottom

- **WHEN** the workbench route loads
- **THEN** the dock appears in the bottom half of the viewport
- **AND** the dock remains inside viewport bounds
- **AND** its shell and groups prevent content overflow.

#### Scenario: Controls follow task grouping

- **WHEN** the dock renders
- **THEN** Projects, Parameters, Dialogue, Sketch, Code, and Docs appear in the
  persistent-window group
- **AND** Draw, conditional Terminal, and Settings appear after the utility separator.

#### Scenario: Dock controls are accessible

- **WHEN** assistive queries inspect the dock
- **THEN** it is a horizontal toolbar named `Workbench tools`
- **AND** Projects, Parameters, Dialogue, Ecky IR docs, Code inspector, Sketch
  Workspace, draw, conditional terminal, and settings controls have stable full names
- **AND** every visible/open/active state has a non-color indicator.

#### Scenario: Keyboard traverses one toolbar

- **WHEN** keyboard focus enters the dock and the user presses Left, Right, Home, or End
- **THEN** roving focus moves predictably among rendered controls
- **AND** the dock contributes one Tab stop rather than one Tab stop per control.

#### Scenario: Disabled control remains explainable

- **WHEN** Draw is unavailable for the selected model and receives toolbar focus
- **THEN** it exposes `aria-disabled=true`
- **AND** its raw capability reason is available in accessible and visible tooltip copy
- **AND** activation sends no action.

### Requirement: Navigation State Preservation

The system SHALL preserve Projects-owned creation, per-thread layout persistence, and
window visibility while making launcher focus/close behavior deterministic.

#### Scenario: Settings round trip keeps dock visible

- **WHEN** Settings opens and closes from the workbench dock
- **THEN** the dock remains visible
- **AND** Parameters remains available by accessible name
- **AND** Settings pressed state matches its actual visibility.

#### Scenario: Visible background window focuses before closing

- **WHEN** a dock-launched window is visible behind another window and its launcher is
  activated
- **THEN** that window moves to the foreground and remains open
- **AND** a later activation while it is focused closes it.

## ADDED Requirements

### Requirement: Dock labels and icons remain legible across widths

The system SHALL use real DOM labels and one consistent SVG grammar. Wide layouts SHALL
show persistent short labels; compact layouts SHALL preserve full accessible names and
show the focused/open control name without requiring hover.

#### Scenario: Compact dock keeps every control reachable

- **WHEN** viewport width forces compact dock presentation
- **THEN** icons remain inside the viewport without clipping or wrapping over content
- **AND** keyboard and pointer users can identify every control.
