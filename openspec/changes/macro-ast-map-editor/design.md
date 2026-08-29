# Design: Macro AST Map Editor

## Goal

Create an additive `New Params` visual view first, backed by `.ecky` source
projection, then extend that surface into broader macro structure, insertion,
and verification authoring while keeping source-backed AST as the authority.

## Decisions

- `.ecky` source remains canonical.
- Backend owns parse, validate, AST identity assignment, AST patch application,
  source serialization, and verification evaluation.
- Frontend owns new params view shell, map layout, selection, search focus,
  inline controls, insertion affordances, keyboard focus, and visual state.
- Tauri boundary structs use `#[serde(rename_all = "camelCase")]`.
- Tauri invoke payloads use camelCase in TypeScript.
- Rust command arguments and structs use snake_case internally.
- Major map containers use `overflow: hidden`.
- UI follows Tactical Midnight theme, square borders, and `--primary` /
  `--secondary` bronze accents.
- Visual edits are expressed as AST patch requests.
- String splicing is forbidden for source-changing map edits.
- Stable AST node identity is required before selection, undo, or verification
  links ship.
- Existing parameter panel remains available until the new params view proves
  parity for covered flows.
- SVG is the primary structural layer for the map scene.
- HTML is the editable overlay layer for inputs, buttons, and search anchors.
- Canvas is optional and may only provide background, glow, or perf underlay
  decoration.
- The shared layout model must expose `nodeId -> x/y/w/h/path/ports/controlAnchor`
  so SVG and HTML layers stay aligned.
- The shipped `New Params` projection is the frontend shell to extend. A second
  visual document model SHALL NOT be introduced.
- Typed geometry projection consumes existing `EckyAstNode`, authoring graph,
  shape graph, and Core operation signature data. It does not add a parallel
  Rust IR hierarchy.
- Operation port roles come from the canonical backend operation/signature
  registry. Frontend code SHALL NOT maintain a second operation arity/type map.
- Automatic layout is derived. MVP stores only selection and expansion state as
  session UI state keyed by stable node key. Manual node positioning and durable
  layout persistence are deferred.
- Core nodes with `sourceAddressable=false` remain visible as collapsed
  read-only nodes with the backend-provided reason.
- Search result selection focuses and frames the matching source-backed map
  region instead of acting as text-only filtering.
- Visual direction is Vertex/futuristic blobs with luminous contours and ports,
  not literal molecular biology.
- Verification state is source-backed authored data plus runtime result overlay.
- Physical fit relations must use named constraints or named bindings.
- Repeated authored structures must use `repeat` or `instance`.

## Rejected Paths

- Replacing the detached parameter side panel immediately. Rejected because the
  new view must prove behavior without removing fallback.
- Canvas document as source of truth. Rejected because source roundtrip and LLM
  editing would become secondary.
- String patching from click/type interactions. Rejected because nested macro
  edits need parse-safe behavior and diagnostics.
- Mockup-only biological visualization. Rejected because visual form must prove
  AST identity, persistence, and preview updates, and preferred direction is
  futuristic blobs rather than literal cell anatomy.
- UI-only verification markers. Rejected because verification intent must compile
  and roundtrip as `.ecky` source.

## Architecture

```text
.ecky source
  -> backend parse
  -> AST with stable node ids
  -> frontend map projection
  -> inline edit / insert / verify action
  -> AST patch request
  -> backend validate + apply
  -> serialized source
  -> render / preview / verify
  -> frontend map projection update
```

The frontend never mutates source text directly for map interactions. It submits
intent:

```text
PatchParamValue(nodeId, value)
InsertNode(parentNodeId, insertionKind, payload)
CreateBinding(name, sourceNodeId, targetNodeId, relation)
CreateVerifyClause(anchorNodeId, payload)
RenameNode(nodeId, name)
DeleteNode(nodeId)
```

Backend responses include updated source, updated AST map projection, preview
state, and diagnostics tied to node ids when possible.

## AST Identity

Each source-backed node receives a stable id derived from semantic path plus
parse-local disambiguation:

```text
model/body_shell/part:front_wall/param:width
model/body_shell/repeat:shelf_rows/item:3/part:shelf
model/body_shell/verify:door_gap
```

Identity rules:

- keep ids stable when sibling formatting changes
- keep ids stable when unrelated branches change
- preserve identity across parse, patch, serialize, reparse
- create deterministic ids for unnamed legacy forms
- return conflict diagnostics when an edit would make identity ambiguous

## Visual Model

Map projection is a structured view model, not raw renderer state:

```text
AstMap
  root: MapNode
  layers: structure | ports | controls | verification
  diagnostics: MapDiagnostic[]

MapNode
  id
  kind
  label
  valueKind
  operation?
  sourceRange?
  sourceAddressable
  editableOps
  nonEditableReason?
  children
  ports: role + valueKind + cardinality + connection
  controls
  verification
  layoutHints
```

Visual metaphor is pluggable. First production renderer should bias toward
Vertex/futuristic regions: dark irregular plates, neon edge glow, bronze
accents, ports, traces, and focused regions. SVG owns these shapes. HTML owns
the live control surface. Canvas may only draw underlay glow or background
noise. Contract stays same.

## Typed Geometry Projection

`New Params` currently projects parameter ownership from `ModelManifest`,
`UiSpec`, current values, and a shallow source byte map. That shipped projection
remains the fallback and supplies proven scene, search, focus, density, and
inline-control behavior. Typed geometry mode enriches it with existing backend
AST/authoring packets instead of inferring operations from source text in
TypeScript.

### Current implementation gap

Current frontend projection is not a partial Core graph. Its node kinds are
limited to `model`, `part`, `port`, `param`, and `verify`; construction uses
`ModelManifest`, `UiSpec`, parameter values, and `macro_ast_source_map`. The
source-map command provides only top-level ids, labels, and byte ranges.

The stronger backend path exists but is disconnected:

- `get_authoring_graph` compiles current source and returns AST, feature,
  dependency, constraint, target, and handle projections;
- frontend `AuthoringGraph` decodes targets but leaves AST nodes, features,
  dependencies, constraints, and handles as `unknown[]`;
- no workbench consumer calls `getAuthoringGraph`;
- Core verifier already stores argument names in `ArgSpec`, but this metadata is
  private validator implementation and is not a public visual descriptor;
- current map connectors represent root/part layout, not typed value flow;
- current CAD tone logic infers dimensions from labels such as `width`, `x`,
  `height`, and `y` instead of backend type/unit metadata.

MVP closes this bridge. It extends existing `AuthoringGraphAstNode` projection
and TypeScript decoder with addressability, child/input edges, and typed port
roles. It does not create a new persisted graph, new Core node hierarchy, or a
frontend Lisp parser.

Projection inputs:

```text
EckyAstNode[]
AuthoringGraph
ShapeGraphPacket
canonical operation descriptors
current values
VisualLayoutState
```

Projection output remains a frontend view model. It is regenerated after every
accepted patch and never becomes geometry or source truth.

Collapse policy follows `CoreValueKind` and source addressability:

- `Number`, `Boolean`, and `Text` bindings render as inline expressions or
  controls under the nearest structural owner.
- `Point2`, `Point3`, and scalar `List` values render inline by default and may
  expand through a math/details lens.
- `Sketch`, `Path`, `Frame`, `Mesh`, `Compound`, and `Solid` bindings render as
  structural nodes.
- named `part`, `shape`, geometry-valued `let`, and geometry-valued `let*`
  bindings preserve authored names as labels and identity anchors.
- anonymous nested operations may render inside the nearest named structural
  node instead of occupying the top-level scene.
- nodes without authored source mapping render as collapsed read-only nodes;
  UI exposes `nonEditableReason` and never synthesizes an edit path.

This policy is projection-only. It does not constant-fold or rewrite authored
Lisp. Expanding inline math changes view state only.

Operation nodes use typed role-labelled ports. Examples:

```text
difference: base: Solid, tools: Solid+
union: solids: Solid+
extrude: profile: Sketch, height: Number
translate: x/y/z: Number, shape: geometry
place: frame: Frame, shape: geometry
repeat-union: count/index inputs, body: Solid
```

Exact roles, kinds, ordering, and cardinality come from canonical backend
operation descriptors. Frontend may display `Subtract` while retaining exact
source operation `difference`.

For MVP, backend may attach resolved port descriptors directly to each projected
call node. A standalone operation-catalog command is unnecessary. Existing
`ArgSpec` names and expected kinds become reusable descriptor data instead of
remaining private validation-only constants.

Visual gestures produce the same digest-guarded AST patches used by agents. The
scoped raw-source pane may remain a legacy escape hatch, but typed node editing
SHALL NOT use byte splicing or write Core IR directly.

Stable identity policy:

- named params, parts, shapes, and let bindings retain layout/selection across
  formatting changes and unrelated sibling insertion or reorder;
- anonymous nested operation identity is best-effort and may reset when its
  structural path changes;
- when identity cannot be preserved, stale layout is discarded instead of
  attaching it to another operation;
- backend remains authority for `stableNodeKey`, `sourceAddressable`, and
  editable operations.

## New Params View

The first product surface is additive:

- entry label: `New Params`
- old parameter panel stays reachable
- source projection provides visible params and owning AST context
- inline control edits use AST patches
- search indexes param names, labels, node ids, and visible source-backed labels
- selecting a search result pans, scrolls, or zooms to the owning region
- focus state uses Tactical Midnight accents, not a separate results panel as
  primary feedback

The view may omit insertion and verification controls until their phases start.
The first usable shape should favor compact modules around the owning part,
not full-width nested list rows. Search focus should zoom or pan the scene and
pulse the target contour.

## Subagent Role

Subagents may help with disjoint work such as AST projection review, search
index tuning, layout proof, and e2e proof. They do not own the map source of
truth, patch application, or acceptance of the `find -> apply` loop.

## Inline Parameters

Inline controls are generated from AST param metadata:

- number: stepper, slider when bounds exist, text fallback
- boolean: toggle
- enum/symbol set: menu
- text: inline text field
- reference: port selector

Control changes are draft edits until backend accepts. Failure state remains at
the control and displays raw backend/provider error body.

## Search Focus

Search is spatial navigation:

```text
query -> matching MapNode[] -> select result -> focusRegion(nodeId)
```

Focus behavior:

- keep result list compact
- select the matching AST node
- pan or scroll the map so the region is visible
- emphasize the containing blob and direct control
- preserve inline edit state when search focus changes
- show no-match state without mutating source or selection

## Authoring From Map

Insertion starts from explicit anchor selection:

- click empty region inside model or part
- choose insertion kind or type text into insertion prompt
- frontend sends structured intent
- backend validates legal position
- backend applies AST patch and serializes source

Typed intent resolution is deterministic:

- known `part` template creates part node
- known param declaration creates input/param node
- known `verify` intent creates verify clause
- unknown text stays pending with raw parser diagnostic

## Verification Layer

Verification authoring uses existing verify syntax and future named constraints:

- selected geometry or AST nodes can create a named distance/fit/overlap check
- verify clauses render as map nodes anchored to related structure
- pending checks render separately from pass/fail/error checks
- runtime result overlays never become export geometry
- raw backend/provider errors display at the verify node

## State And Persistence

- Source edits persist through existing version/history flows.
- Configuration changes persist through `save_config` to
  `app_config_dir/config.edn`.
- Deterministic automatic layout is not persisted.
- MVP visual state contains selection and expansion only. It is keyed by
  `stableNodeKey`, remains session-local, and is excluded from `.ecky`, render
  inputs, version digests, artifacts, and exports.
- Manual node positioning, grouping overrides, and restart persistence are out
  of MVP scope. If added later, they remain non-authoring UI metadata keyed by
  thread/workspace plus stable node key.
- Stale visual-state keys are ignored. They never retarget by array index or
  nearest visual position.
- Authored params, parts, relations, repeats, instances, and verify clauses are
  source.
- No SQLite file is written directly.

## UI Boundary

- No separate agent status bar.
- No live auto-agent terminal output in app logs.
- Agent state belongs in Ecky bubble copy.
- Interactive agent stdout/stderr stays in dedicated terminal modal.
- Major editor, map, overlay, and modal containers use `overflow: hidden`.

## BDD Proof Strategy

Every UI increment starts with Playwright on a real route:

```gherkin
Given an existing macro with nested parts and params
When the author opens New Params
Then model, part, input port, and inline param controls are visible
```

Each shipped UI increment must prove:

- one happy path
- one failure or pending state
- source or config persistence when behavior changes persisted state
- old parameter panel remains available during rollout

Rust changes require `cd src-tauri && cargo check` before success report.

## Phase Boundaries

### Phase 1: Additive New Params Projection

- backend params map projection command
- stable node ids
- parse/serialize roundtrip proof for params and owner context
- readonly `New Params` renderer
- existing parameter panel still available

### Phase 2: Inline Param Editing

- inline controls
- AST param patch command
- source/version persistence
- preview update
- raw error at node
- old parameter panel still available

### Phase 3: Search Focus

- search index over params and source-backed labels
- result list
- focus selected region
- no-match state

### Phase 4: Typed Geometry Projection

- reuse shipped `New Params` SVG/HTML scene and interaction shell
- consume backend AST, authoring graph, and shape graph packets
- project scalar bindings inline and geometry bindings as structural nodes
- derive operation-specific typed ports from canonical signatures
- show non-source-addressable nodes as collapsed read-only structure
- retain layout and selection for stable named nodes across unrelated reorder
- keep automatic layout; persist no manual coordinates in MVP

### Phase 5: Map Insertion

- click/type insertion anchor
- insert part/input/relation/repeat/instance
- parser diagnostics at pending node
- source roundtrip

### Phase 6: Verification Authoring

- create named constraints/bindings from selections
- render verify nodes/overlays
- pass/fail/error state
- production export excludes diagnostics

### Phase 7: Panel Retirement Decision

- evaluate whether to replace old params panel
- remove detached params dependency only for proven migrated flows
- preserve legacy fallback only where not yet migrated
- prove no regression in version apply/commit flows

## Open Questions

- Final art metaphor: exact Vertex/futuristic blob language, glow intensity,
  density, and motion level.
- Whether keyboard-first command palette is required for all insertion kinds.
- Whether source text editor stays side-by-side, modal, or secondary drawer after
  map editor reaches parity.
