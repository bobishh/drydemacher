## ADDED Requirements

### Requirement: Floating windows respect dock safe area

The system SHALL measure the rendered dock and clamp floating windows to a work area
that excludes the dock, bottom offset, and interaction gap.

#### Scenario: Dialogue composer stays unobscured

- **WHEN** Dialogue opens at its default size on a supported viewport
- **THEN** its composer and actions do not intersect the dock rectangle
- **AND** its header and close control remain inside the work area.

#### Scenario: Restored layout is clamped without schema migration

- **WHEN** a remembered window rectangle would overlap the dock after viewport or dock
  size changes
- **THEN** runtime geometry shifts or resizes it into the safe work area
- **AND** current thread-layout persistence remains the only persistence path.

### Requirement: Window launcher activation has three deterministic outcomes

The system SHALL derive launcher behavior from window visibility and foreground focus.

#### Scenario: Hidden window opens focused

- **WHEN** a hidden window launcher is activated
- **THEN** the window opens inside the safe work area and becomes foreground.

#### Scenario: Background window is retrieved

- **WHEN** a visible non-foreground window launcher is activated
- **THEN** the window becomes foreground without closing.

#### Scenario: Foreground window closes

- **WHEN** a visible foreground window launcher is activated
- **THEN** the window closes
- **AND** focus falls to the highest remaining visible window.
