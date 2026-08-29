## ADDED Requirements

### Requirement: Components use a canonical local frame

The system SHALL author geometry inside every `define-component` in that
component's local coordinate system. Component placement SHALL be expressed at
the instance/mate boundary; a component body SHALL NOT require parent dimensions
or world coordinates merely to select a mounting face.

#### Scenario: One latch geometry moves between enclosure faces

- **GIVEN** a latch component authored once around its local mounting port
- **WHEN** one instance mates to a front-wall port and another mates to a
  side-wall port
- **THEN** both instances use byte-identical latch body source
- **AND** only the target port reference differs
- **AND** the instances render on the requested orthogonal faces

### Requirement: Named ports carry complete local frames

The system SHALL support named ports on source-backed components and output
parts. Each port SHALL carry a stable id, compatibility type, origin, local
`xAxis`, local `zAxis`, derived orthogonal `yAxis`, and optional interface/fit
metadata. The compiler SHALL reject non-finite, zero-length, non-orthogonal, or
left-handed frames with component and port source context.

#### Scenario: Invalid port axes fail before render

- **WHEN** a port declares parallel `xAxis` and `zAxis`
- **THEN** compilation fails before backend execution
- **AND** the diagnostic names the component, port, axes, and source span

#### Scenario: Port parameters preserve physical fit intent

- **WHEN** a mounting port declares clearance or insertion-depth metadata
- **THEN** the metadata remains named on placement evidence and package export
- **AND** no anonymous placement offset replaces the declared fit value

### Requirement: A mate solves one deterministic rigid placement

The system SHALL place a component instance by mapping a named source port to a
named target port. The solved transform SHALL be
`targetFrame * mateModifiers * inverse(sourceFrame)` and SHALL lower to existing
geometry placement operations before backend execution.

#### Scenario: Source port maps exactly to target port

- **WHEN** an instance mates its `mount` port to `enclosure.side-left-latch`
- **THEN** the transformed source-port origin equals the target-port origin
- **AND** transformed port axes satisfy the requested normal/roll relation within
  the numeric placement tolerance

#### Scenario: Mate graph conflict is explicit

- **WHEN** multiple mates require inconsistent transforms for one rigid instance
- **THEN** placement fails before render
- **AND** the diagnostic names every conflicting mate and resolved frame

### Requirement: Mate orientation is semantic and explicit

The mate surface SHALL support aligned or opposed port normals, roll about the
target port axis, port-local offset, and optional mirroring across an explicitly
named source-port axis. Callers SHALL NOT need to derive Euler rotations from
the target wall orientation.

#### Scenario: Orthogonal wall move needs no Euler rewrite

- **WHEN** a latch mate target changes from a front-wall port to a side-wall port
- **THEN** the solver derives the 90-degree world rotation from the port frames
- **AND** source geometry, source port, roll, and fit parameters remain unchanged

#### Scenario: Mirrored counterpart keeps a right-handed placement frame

- **WHEN** an instance requests mirroring across source-port local `x`
- **THEN** geometry is reflected in local space before rigid mate placement
- **AND** emitted `placementFrame` remains orthonormal and right-handed
- **AND** manifest evidence records the mirror operation separately

### Requirement: Placement evidence is durable and inspectable

Every solved inline component instance SHALL emit instance id, source component,
source/target port references, solved placement frame, orientation modifiers,
and mate status into runtime manifests. Preview, export, verification, and error
reporting SHALL consume the same evidence.

#### Scenario: Agent can inspect why an instance is rotated

- **WHEN** a mated component renders successfully
- **THEN** target metadata exposes its solved frame and mate inputs
- **AND** an agent can distinguish target-port orientation from authored local
  geometry without reverse-engineering triangle coordinates

### Requirement: Placement is backend and export invariant

Native OCCT, portable Core planning, FreeCAD, mesh preview, STEP, STL, and 3MF paths SHALL
apply the same solved transform. Preview-only exploded offsets SHALL compose
after component placement and SHALL NOT change manufacturing geometry.

#### Scenario: Native and portable planners agree on side placement

- **WHEN** the front/side latch fixture lowers through native OCCT and portable Core planners
- **THEN** corresponding placed-part bounds and port-frame origins agree within
  parity tolerance
- **AND** each manufacturing export contains the same rigid placement

#### Scenario: Exploded preview does not alter export

- **WHEN** an exploded view offsets a mated latch for inspection
- **THEN** preview displays the extra offset
- **AND** STL/STEP digests remain those of the solved manufacturing placement
