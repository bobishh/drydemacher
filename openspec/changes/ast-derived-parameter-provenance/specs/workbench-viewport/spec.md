# workbench-viewport Specification

## MODIFIED Requirements

### Requirement: Viewport Controls Require Exact Generated Provenance

The system SHALL keep the viewport free of generic parameter overlays. In Select mode,
it MAY render scoped controls for a generated Ecky selection only when the resolved AST
provenance contains a non-empty parameter set. Orbit and Measure modes SHALL remain free
of editable parameter overlays.

#### Scenario: Proven part selection opens scoped controls

- **GIVEN** a generated Ecky part owns a non-empty inferred parameter set
- **WHEN** the user selects that part in viewport Select mode
- **THEN** the viewport renders controls linked to that set
- **AND** unrelated model parameters are absent
- **AND** edits use the existing parameter apply flow.

#### Scenario: Exact face selection narrows part controls

- **GIVEN** a direct-OCCT face has exact authored-shape parameter provenance
- **WHEN** the user selects that face in viewport Select mode
- **THEN** the viewport renders only the face provenance controls
- **AND** part-only controls outside that set are absent.

#### Scenario: Ambiguous selection shows no editable overlay

- **GIVEN** a selection is ambiguous or resolves to no parameter keys
- **WHEN** the user selects it
- **THEN** no editable viewport overlay is rendered
- **AND** the system does not fall back to all part or model parameters.

#### Scenario: Non-selection modes stay unobscured

- **GIVEN** a rendered model
- **WHEN** the user orbits or measures it
- **THEN** `.viewer-part-overlay` is not rendered.
