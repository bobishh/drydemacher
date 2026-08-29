## Context

`.ecky` already has two disconnected placement stories:

1. Inline `define-component` calls expand to geometry. Authors then place that
   geometry with raw `translate`/`rotate` or calculate world coordinates inside
   the component.
2. Installed component packages already expose `ComponentPort`/`PortFrame`, and
   `commands/component_package.rs` solves first-pass rigid mates into per-instance
   `placementFrame` values used by preview and export.

The filament-dryer latch demonstrates the gap. Its component recomputes
enclosure width/depth, rail Y, split Z, catch offsets, and front/rear transforms.
Moving the same mechanism to an orthogonal side changes both placement and
component geometry math. `plane`/`location`/`place` can express the final
transform, but source has no named component interface from which to derive it.

Constraints:

- Existing `.ecky` source and stable keys remain valid.
- `ecky_core_ir` public geometry structs remain unchanged where possible.
- Frontend payloads remain camelCase; Rust contracts remain snake_case with
  serde boundary translation.
- All physical fit values remain named.
- Preview-only transforms never enter manufacturing geometry.

## Goals / Non-Goals

**Goals:**

- Make component geometry local and portable between arbitrary mounting faces.
- Express attachment through stable named port frames and semantic mate options.
- Share one mate solver between inline source and installed packages.
- Lower solved placement to existing transforms, preserving backend parity.
- Preserve ports through extraction/package workflows.
- Provide enough evidence to explain every solved orientation.

**Non-Goals:**

- General mechanical kinematics, flexible bodies, or motion simulation.
- Automatic inference of authoritative ports from bounding boxes or topology.
- Constraint solving for arbitrary geometric equations.
- Automatic collision-free routing or latch-dimension redesign.
- Replacing `translate`, `rotate`, `place`, or package assemblies.
- UI assembly editing in the first increment.

## Decisions

### 1. Local geometry plus explicit interface clauses

A component owns geometry and ports in one canonical local frame. Proposed
surface:

```scheme
(define-component c-latch
  ((number width 64)
   (number clearance 0.25))
  (ports
    (port mount
      :type "mechanical.latch.mount.v1"
      :frame (frame
        :origin '(0 0 0)
        :x-axis '(1 0 0)
        :z-axis '(0 0 1))
      :params ((clearance clearance))))
  (build
    ;; Geometry centered on local mount frame.
    ...))
```

`frame` is metadata, not geometry. `yAxis` is derived as `zAxis × xAxis` after
normalization. Port expressions share the component signature's lexical scope.
Output-role `part` forms may declare ports through the same interface clause.

Alternative: infer origin and axes from component bounds. Rejected: a bbox does
not identify pivot, insertion direction, contact plane, or handedness and changes
when geometry changes.

### 2. One source-native placement form

Placement wraps a component invocation and names both interfaces:

```scheme
(part latch-left
  (place-component
    (c-latch :width latch-width)
    :from mount
    :to (port-ref enclosure-body side-left-latch)
    :normal opposed
    :roll 0deg
    :offset '(0 0 0)
    :mirror none))
```

Changing `side-left-latch` to `front-left-latch` moves the same local geometry.
V1 requires explicit `:normal aligned|opposed`; `:roll` and `:offset` default to
zero, `:mirror` defaults to `none`. `:offset` is expressed in target-port local
coordinates. Physical offsets used for fit must reference named bindings.

Alternative: add convenience rotations such as `:side left`. Rejected: semantic
face names do not generalize to arbitrary angled or imported assemblies.

### 3. Port-frame mapping defines the transform

For source port frame `F_s`, target world frame `F_t`, and mate modifier matrix
`M`, solve:

```text
T_instance = F_t * M * inverse(F_s)
```

`M` contains normal alignment/opposition, roll, and target-local offset. The
solver validates transformed origins and axes after calculation. Multiple mates
for one rigid instance must resolve the same transform within tolerance; they
validate rather than average conflicting placements.

An unplaced output part is rooted at identity. Mated instances form a directed
graph. Cycles are accepted only when all already-solved transforms agree;
underconstrained unrooted graphs fail explicitly.

Alternative: expose Euler angles and let the author calculate them. Rejected:
Euler order, wall normal, and pivot-axis mapping are the current failure mode.

### 4. Mirroring is separate from rigid placement

A `PortFrame` stays orthonormal and right-handed. `:mirror x|y|none` reflects
local geometry and all local port frames across the named source-port axis before
mate solving. Placement evidence records the reflection separately; it never
smuggles a negative determinant into `placementFrame`.

Alternative: encode reflection as a left-handed frame. Rejected: existing frame
validation and exporters assume a rigid right-handed basis.

### 5. Extract the existing package solver into shared infrastructure

Move pure frame validation, compatibility, transform solving, and clearance
checks from `commands/component_package.rs` into a backend-independent module,
tentatively `component_placement.rs`. Both package assembly commands and inline
source compilation call the same solver.

Inline compilation pipeline:

```text
parse source
-> resolve component signatures and interface clauses
-> instantiate geometry + evaluated local ports
-> build rooted instance/mate graph
-> validate types, ids, frames, and named fit values
-> solve placement frames
-> expand mirror/place-component into existing mirror/place Core nodes
-> plan/render through existing backends
-> attach placement evidence to manifest
```

The geometry Core IR remains placement-op based. Interface/mate nodes live in the
expanded AST/compiler evidence layer and disappear after deterministic lowering.
Generated transform nodes retain the `place-component` call as source anchor.

Alternative: call the package command layer from the compiler. Rejected: command
handlers own persistence/runtime concerns and would make pure source compilation
depend on application state.

### 6. Evidence is a sidecar, not synthetic geometry

Compilation returns `ComponentPlacementEvidence` beside CoreProgram:

```text
instanceId
componentId
sourcePortRef
targetPortRef
placementFrame
normalMode
rollDegrees
offset
mirrorAxis
mateStatus / diagnostics
```

ArtifactBundle and ModelManifest persist equivalent camelCase records. Viewer,
MCP inspection, verification, STEP/STL/3MF export, and failure diagnostics use
this one record. Port glyphs may later be drawn as debug overlays; they never
enter STL/STEP.

### 7. Preserve source intent during extraction

`component_extract` carries explicit port clauses when every referenced binding
is closed by the extracted signature/body. A port depending on parent/world
bindings blocks extraction with exact binding evidence. Extraction never freezes
the current evaluated world frame and never heuristically invents a port.

Package headers reuse existing `ComponentPort` and `PortFrame` contracts. Inline
and installed components therefore expose the same interface shape.

### 8. Backend parity comes from lowering once

Mate solving happens before backend planning. Every backend receives ordinary
resolved `mirror`/`place` transforms. No backend implements a mate solver.
Manufacturing export bakes the solved transform exactly as installed assembly
export already bakes `placementFrame`; exploded/view offsets apply afterward in
Viewer metadata only.

## Risks / Trade-offs

- **[Two component systems drift]** → Extract solver and frame validation into one
  shared module before adding inline syntax; package tests remain regression
  coverage.
- **[Stable ids change during inline expansion]** → Anchor generated transforms
  to the authored placement call and add byte-stability fixtures for old source.
- **[Mate graphs become an accidental general solver]** → Limit V1 to rigid
  frame equality, explicit mirror, named clearance checks, and deterministic
  conflict errors.
- **[Ports depend on parameters]** → Evaluate port expressions in the same lexical
  environment as geometry and include resolved values in evidence/cache identity.
- **[Reflection changes thread/latch chirality]** → Keep mirror explicit, test
  normals/winding, and record it outside the right-handed placement frame.
- **[Source syntax adds verbosity]** → Agent guide provides a canonical local-
  component pattern; later UI helpers may author frames without weakening source
  authority.
- **[Placement hides collision]** → V1 proves frame mapping only. Existing or
  later keepout/clearance verification reports physical interference separately.

## Migration Plan

1. Extract and test shared `PortFrame` validation/mate math without behavior
   changes to package assemblies.
2. Add parser/emitter support for component ports and placement syntax behind
   additive forms; old source compiles identically.
3. Add expanded-AST mate graph and lower solved instances to existing transforms.
4. Persist placement evidence and expose it through MCP/runtime manifests.
5. Preserve ports through extraction and package headers.
6. Update canonical language manifest, generated prompts/docs, and authoring card.
7. Migrate one latch fixture: local latch body plus front and side target ports.
   Prove moving it changes only the target `port-ref`.

Rollback removes use of the additive forms while leaving old transform-authored
models untouched. Persisted artifacts remain loadable because their geometry is
already transformed and placement evidence is additive metadata.

## Open Questions

- Whether V1 permits only literal port ids/types or allows computed symbols. The
  recommended answer is literal stable ids/types; frame coordinates may be
  parameter expressions.
- Whether nested mated components expose transformed child ports outside their
  parent. Recommended V1: yes, namespaced by instance path.
- Whether `:mirror` accepts only `x|y|none` or a general local plane. Recommended
  V1: `x|y|none`; arbitrary reflection adds no value to the latch use case.
- Exact epsilon for redundant-mate agreement. Reuse existing package solver
  tolerances unless parity tests show unit-scale sensitivity.
