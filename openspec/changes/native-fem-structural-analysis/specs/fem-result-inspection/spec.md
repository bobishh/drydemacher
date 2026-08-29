# Delta for fem-result-inspection

## ADDED Requirements

### Requirement: FEM execution is explicit and does not block normal preview

Compiling or rendering a model with an analysis SHALL validate and preserve the
study declaration but SHALL NOT automatically generate a volume mesh or solve.
Mesh generation and solve SHALL be explicit workbench or MCP actions.

#### Scenario: Model with analysis is previewed

- **WHEN** normal geometry preview runs
- **THEN** CAD preview and topology artifacts are produced normally
- **AND** no Gmsh HXT/Netgen worker or FEM solver starts
- **AND** the study is shown as not run or stale as applicable.

#### Scenario: User runs the study

- **WHEN** the user invokes run for a valid current study
- **THEN** the runtime executes the ordered FEM pipeline
- **AND** progress, cancellation, and final result identity are available.

### Requirement: Workbench inspection presents current result evidence

The existing workbench control dock SHALL expose study validation, mesh
quality, run/cancel, field selection, extrema, reactions, convergence, and VTU
export without introducing a separate agent status bar.

#### Scenario: Successful result is inspected

- **GIVEN** a successful current result
- **WHEN** the Analysis section opens
- **THEN** it shows study/result identity, material, loads, constraints, mesh
  counts/quality, residual, equilibrium, mass, extrema, and convergence state
- **AND** selecting an extremum can locate it in the viewport.

#### Scenario: Backend operation fails

- **GIVEN** runtime probing, meshing, assembly, or solution fails
- **WHEN** the Analysis section reports the failure
- **THEN** stage, raw backend detail, observed values, and actionable source or
  selector context remain visible
- **AND** the message is not replaced by generic solver advice.

### Requirement: MCP exposes compact artifact-aware FEM operations

A specialist MCP capability SHALL expose study validation, mesh preview,
analysis run/cancel, result summary/sliced inspection, and convergence run.
Existing parameter/source tools SHALL remain the model mutation path.

#### Scenario: Agent completes a verified study flow

- **WHEN** an agent validates a study, previews its mesh, runs it, reads extrema,
  and invokes generated-model verification
- **THEN** every response carries current model/artifact identity
- **AND** bulk field arrays route as artifacts or bounded slices
- **AND** only a green current required FEM check may support commit.

#### Scenario: Agent requests an old result after parameter edit

- **WHEN** result inspection targets a stale identity
- **THEN** the response labels it stale and returns both old and required current
  identities
- **AND** it does not present the values as current or modify source/history.

### Requirement: Result visualization is display-only

The viewport SHALL support undeformed outline, deformed boundary with explicit
scale, volume-mesh edges or clip view, scalar legend, and current fields such as
displacement magnitude, von Mises stress, principal stress, and safety factor.
Display averaging and deformation scaling SHALL be explicit.

#### Scenario: User displays exaggerated deformation

- **GIVEN** a current result
- **WHEN** deformation scale and von Mises field are selected
- **THEN** the viewport transforms only result-display vertices
- **AND** labels scale, units, range, field location, and averaged/unaveraged
  status
- **AND** exact CAD geometry remains unchanged.

#### Scenario: Display state changes

- **WHEN** field, legend range, deformation scale, clip plane, mesh overlay, or
  undeformed outline changes
- **THEN** BRep, STL, STEP, and manufacturing artifact digests remain identical
- **AND** no debug/result primitive enters production export geometry.

### Requirement: Mesh convergence is first-class evidence

The system SHALL run at least three explicit mesh levels for selected metrics
and SHALL report per-level identity, size controls, node/Tet4 counts, quality,
residual, extrema, and relative deltas. Convergence status SHALL be per metric.

#### Scenario: Consecutive refinements meet tolerance

- **GIVEN** all levels pass quality and solve gates
- **AND** consecutive relative deltas for a selected metric meet its configured
  threshold
- **WHEN** convergence is evaluated
- **THEN** that metric is marked converged with all level evidence preserved.

#### Scenario: Peak stress rises at a singular support edge

- **GIVEN** displacement trends converge but unaveraged peak stress continues to
  rise or move into a shrinking hotspot
- **WHEN** convergence is evaluated
- **THEN** displacement and stress receive independent statuses
- **AND** stress remains unconverged or suspected-singularity
- **AND** smoothing cannot turn it green.

#### Scenario: One refinement fails or is cancelled

- **WHEN** a required level fails quality/solve gates or the sequence is
  cancelled
- **THEN** the study is failed/incomplete rather than converged
- **AND** successful earlier levels remain visible as partial evidence.

### Requirement: Verification and safety presentation are honest

The UI and MCP SHALL distinguish current/stale, solved/failed, and
converged/unconverged states and SHALL identify the MVP as linear-elastic Tet4
analysis rather than engineering certification.

#### Scenario: Required convergence evidence is absent

- **GIVEN** an authored FEM verification check requires convergence
- **AND** only one mesh result exists or the relevant metric is unconverged
- **WHEN** verification and result presentation run
- **THEN** the check cannot pass
- **AND** the missing/unconverged evidence is shown alongside the numerical
  value.

#### Scenario: Result is current but scope is limited

- **GIVEN** a valid converged linear-static result
- **WHEN** it is presented
- **THEN** the interface identifies small-strain isotropic Tet4 assumptions
- **AND** does not claim contact, fatigue, buckling, nonlinear, physical-test,
  or certification coverage.

### Requirement: Inspection exposes the complete engineering evidence chain

Workbench and MCP SHALL present engineering question, idealization, assumption
ledger, input provenance/uncertainty, applicability, numerical verification,
mesh convergence, singularity, sensitivity, and physical-validation state as
separate evidence. A single green solver/result badge SHALL NOT collapse these
states.

#### Scenario: Solve succeeds with incomplete engineering evidence

- **GIVEN** residual, equilibrium, and mesh convergence pass
- **AND** material provenance, load uncertainty, support rationale, or physical
  validation is missing
- **WHEN** user or agent inspects study
- **THEN** numerical solve is shown successful
- **AND** engineering decision remains pending/unsupported with exact missing
  evidence
- **AND** UI does not label part safe

#### Scenario: User traces a decision

- **GIVEN** a study has current result and decision status
- **WHEN** user opens evidence details
- **THEN** every acceptance metric traces to result/mesh/geometry/source digest,
  idealization, material/load/support evidence, applicability gate, convergence,
  sensitivity, and validation record
- **AND** stale or superseded evidence remains distinguishable

### Requirement: FEM activity uses existing agent and terminal UX

Long FEM stages SHALL publish typed activity through the existing Ecky bubble
and SHALL keep interactive worker output in the dedicated terminal/error view,
not in a new status bar or general app logs.

#### Scenario: Meshing and solving take noticeable time

- **WHEN** stages advance
- **THEN** existing activity UX shows current stage and bounded progress facts
- **AND** cancel remains available
- **AND** completion or failure clears the activity state.
