# Delta for macro-ast-map

## ADDED Requirements

### Requirement: Macro source projects to additive params map

The system SHALL project `.ecky` macro source into a structured `New Params`
visual map without making the visual map the source of truth or removing the
existing parameter panel.

#### Scenario: Existing macro renders in New Params

- GIVEN an existing `.ecky` macro with a model, nested part, input param, and
  numeric parameter
- WHEN the author opens `New Params`
- THEN the editor shows the model as the root structure
- AND the nested part is visible as a child structure
- AND the input param is visible as a port
- AND the numeric parameter has an inline control anchor
- AND the existing parameter panel entrypoint remains available.

#### Scenario: Source remains canonical

- GIVEN a macro AST map is visible
- WHEN the source is parsed, projected to map, serialized, and reparsed
- THEN the reparsed program preserves the same authored macro semantics
- AND no renderer-only state is required to recover the source.

### Requirement: AST nodes have stable identity

The system SHALL assign stable ids to map nodes so selection, patches,
verification links, preview results, and diagnostics can target source-backed
entities.

#### Scenario: Formatting change preserves node ids

- GIVEN `.ecky` source with stable model, part, input, and param forms
- WHEN whitespace, indentation, or comments change without semantic edits
- THEN the same source-backed entities keep the same map node ids.

#### Scenario: Unrelated branch edit preserves node selection

- GIVEN an author has selected a parameter node in one branch
- WHEN another branch receives a valid AST patch
- THEN the selected parameter keeps its node id
- AND selection remains on that parameter.

#### Scenario: Ambiguous identity reports diagnostic

- GIVEN a source edit creates two unnamed sibling forms that cannot receive
  deterministic distinct semantic ids
- WHEN the backend projects the AST map
- THEN the response includes a deterministic identity diagnostic
- AND the frontend can display that diagnostic at the affected parent node.

### Requirement: Map renderer follows workbench UI boundaries

The system SHALL render the AST map inside existing workbench theme and layout
constraints.

#### Scenario: Tactical map shell renders

- GIVEN the AST map editor is open
- WHEN the map shell renders
- THEN it uses Tactical Midnight theme colors
- AND square borders
- AND `--primary` or `--secondary` bronze accents for selected or active map
  elements
- AND uses futuristic blob, glow, or port styling without locking the product to
  literal molecular biology.

#### Scenario: Map containers constrain overflow

- GIVEN a macro map contains many nested structures or long labels
- WHEN the viewport renders on desktop or mobile size
- THEN major map containers keep `overflow: hidden`
- AND controls or labels do not bleed into unrelated workbench regions.

### Requirement: Scene uses layered SVG and HTML

The system SHALL render the macro map as an SVG-led structural scene with HTML
controls overlaid at shared layout coordinates.

#### Scenario: Structural layer stays separate from controls

- GIVEN `New Params` renders a source-backed macro map
- WHEN the scene is drawn
- THEN SVG renders the structural nodes, ports, connectors, focus rings, and
  glow-safe shapes
- AND the scene reflects syntax types visually through node variants, badges,
  or shape cues for model, part, port, and value nodes
- AND HTML renders the interactive inputs, buttons, and search result anchors
  on top of the same layout model
- AND canvas, if present, only serves decoration or background underlay.

#### Scenario: Compact modules wrap around parent parts

- GIVEN a macro map shows several numeric parameters under one part
- WHEN the scene lays out those parameters
- THEN each parameter appears as a compact module attached to the owning part
- AND the scene does not expand the entire stack into full-width rows
- AND the owning part remains visually grouped as a single mechanism blob.

### Requirement: Search focuses map regions

The system SHALL use search in `New Params` as spatial navigation to
source-backed map regions.

#### Scenario: Parameter search focuses owning region

- GIVEN `New Params` shows several source-backed parameter controls
- WHEN the author searches for a parameter by name or visible label
- THEN matching results are listed
- AND choosing a result selects the matching node
- AND the map focuses or frames the owning region.

#### Scenario: Find then apply keeps source-backed identity

- GIVEN `New Params` shows a matching parameter node after search
- WHEN the author applies a valid inline edit from that focused result
- THEN the backend applies a structured AST patch at the selected node
- AND the node keeps its stable id if semantics did not change
- AND the updated source can be reparsed into the same map region.

#### Scenario: No-match search preserves state

- GIVEN `New Params` has a selected parameter node
- WHEN the author searches for a string with no matches
- THEN the view shows a no-match state
- AND source remains unchanged
- AND the current selection is not mutated.

### Requirement: New Params extends into typed geometry projection

The system SHALL extend the shipped `New Params` scene from parameter ownership
into a typed visual projection using existing source-backed AST, authoring graph,
shape graph, and Core operation metadata. The projection SHALL remain derived
frontend state and SHALL NOT introduce another authoring or geometry truth.

#### Scenario: Existing shell renders typed structure

- GIVEN `.ecky` source contains named scalar and geometry bindings
- WHEN the author opens typed geometry mode in `New Params`
- THEN the existing viewport, search, focus, SVG structure, and HTML overlay
  remain in use
- AND nodes are enriched from backend stable keys, value kinds, operations, and
  addressability
- AND TypeScript does not parse Lisp text to infer geometry operations.

#### Scenario: Param-only projection remains fallback

- GIVEN typed AST projection is unavailable for legacy source
- WHEN `New Params` opens
- THEN existing parameter ownership projection remains usable
- AND UI reports why typed structure is unavailable
- AND it does not fabricate editable geometry nodes.

### Requirement: Projection collapses values by typed role

The system SHALL project scalar bindings as inline expressions or controls and
geometry-valued bindings as structural nodes. It SHALL use backend `valueKind`
instead of frontend naming heuristics.

#### Scenario: Scalar let stays inline

- GIVEN a named `let` binding compiles as `Number`
- WHEN typed map projects its owning part
- THEN binding appears as inline expression or control under that owner
- AND it does not occupy a top-level geometry node
- AND expanding math detail does not mutate source.

#### Scenario: Geometry let becomes structural node

- GIVEN a named `let` binding compiles as `Solid`
- WHEN typed map projects its owning part
- THEN binding appears as structural node with authored name
- AND references to binding become graph connections
- AND selecting it resolves backend stable key and source context.

#### Scenario: Unknown kind remains honest

- GIVEN a node has `Any`, unsupported, or ambiguous value kind
- WHEN typed map projects it
- THEN node appears collapsed with exact kind and diagnostic
- AND frontend does not guess scalar or geometry edit semantics.

### Requirement: Operation ports use canonical typed roles

The system SHALL label operation inputs with role, value kind, order, and
cardinality from canonical backend operation/signature registry. Frontend code
SHALL NOT maintain an independent operation signature table.

#### Scenario: Difference exposes base and tools

- GIVEN a source-backed `difference` operation
- WHEN its visual node renders
- THEN one input is labelled `base`
- AND cutter input is labelled `tools` with supported cardinality
- AND node retains exact source name `difference` when display copy says
  `Subtract`.

#### Scenario: Extrude exposes profile and height

- GIVEN a source-backed `extrude` operation
- WHEN its visual node renders
- THEN inputs identify `profile` and `height`
- AND each port exposes backend-declared value kind
- AND incompatible connection is rejected before source mutation.

### Requirement: Non-addressable nodes are collapsed and read-only

The system SHALL keep Core-only or macro-expanded nodes visible as collapsed
read-only structure when `sourceAddressable=false`. It SHALL show backend
`nonEditableReason` and SHALL NOT synthesize source paths or patch operations.

#### Scenario: Macro-expanded operation cannot be edited directly

- GIVEN a projected operation has no exact authored source target
- WHEN author selects it
- THEN map shows a collapsed read-only node
- AND displays backend-provided non-editable reason
- AND edit, reconnect, rename, and delete remain unavailable.

#### Scenario: Source-backed ancestor remains actionable

- GIVEN a read-only expanded node belongs to a source-addressable named binding
- WHEN author requests source context
- THEN map may focus nearest exact authored ancestor
- AND it does not claim expanded child itself is editable.

### Requirement: Visual layout is separate non-authoring state

The system SHALL derive automatic layout. MVP visual state SHALL contain only
selection and expansion keyed by backend stable node key and SHALL remain
session-local. Visual state SHALL NOT enter `.ecky`, Core IR, render inputs,
version identity, artifacts, or exports. Manual node positioning and durable
layout persistence are outside MVP scope.

#### Scenario: Expansion survives source-preserving projection

- GIVEN author expanded and selected a named geometry node
- WHEN unrelated source edit preserves that node's stable key
- THEN expansion and selection remain attached to that node
- AND generated geometry is unaffected.

#### Scenario: Stale visual state never attaches by position

- GIVEN anonymous nested node disappears or receives a different stable key
- WHEN map re-projects
- THEN stale selection and expansion state are ignored
- AND it is not assigned to a nearby or same-index replacement.

#### Scenario: MVP emits no manual coordinates

- GIVEN author opens typed geometry projection
- WHEN automatic scene layout is built
- THEN no manual node coordinates are loaded from or written to source, config,
  version data, or project files
- AND map remains fully reconstructable from projection plus session expansion
  state.

### Requirement: Named identity survives unrelated reorder

The system SHALL preserve stable keys for named params, parts, shapes, and let
bindings across formatting changes and insertion or reorder of unrelated
siblings. Anonymous nested operation identity MAY reset when structural paths
change, but SHALL never silently resolve to a different source operation.

#### Scenario: Named geometry binding keeps selection

- GIVEN named geometry binding `holes` is selected and has layout override
- WHEN another named sibling is inserted before it
- THEN `holes` retains its stable key
- AND selection and layout remain attached to `holes`.

#### Scenario: Anonymous operation reports identity reset

- GIVEN anonymous nested operation is selected
- WHEN sibling reorder changes its structural stable key
- THEN prior selection/layout is cleared or reported stale
- AND frontend does not apply it to another anonymous operation.

### Requirement: Typed visual edits use guarded AST patches

The system SHALL translate typed node gestures into backend AST patches guarded
by source and node digests. It SHALL NOT mutate Core IR directly or use frontend
byte splicing for typed geometry operations.

#### Scenario: Visual argument edit roundtrips through source

- GIVEN a source-addressable `extrude` node with editable height argument
- WHEN author changes height through visual control
- THEN frontend sends allowed AST patch with stable key/path and digests
- AND backend validates and patches canonical source
- AND recompilation returns same structural owner with updated value
- AND preview uses accepted source.

#### Scenario: Stale visual gesture fails safely

- GIVEN visual node was projected from older source digest
- WHEN edit reaches backend after source changed
- THEN patch rejects with exact stale identity detail
- AND source and viewport are not silently overwritten.
