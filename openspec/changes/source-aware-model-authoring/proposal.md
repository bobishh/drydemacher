# Proposal: Source-Aware Model Authoring

## Intent

Replace detached Sketch Workspace direction with one source-aware authoring
surface. `.ecky` source remains canonical. Three.js model, AST map, parameters,
features, constraints, and selections become synchronized projections of same
compiled model.

Current Sketch Workspace starts from blank orthographic drawings and produces a
separate draft path. That path cannot reliably manipulate existing authored
models. It implies generic CAD capability while losing source semantics.

## Scope

- Hide Sketch Workspace entry point immediately without deleting dormant code.
- Ignore stale persisted window state that previously opened Sketch Workspace.
- Define shared authoring graph joining AST nodes, parameters, features,
  constraints, viewer targets, and source-backed handles.
- Synchronize selection between Three.js geometry and AST/dataflow view.
- Add deterministic direct-manipulation handles for explicitly supported source
  operations.
- Route LLM-authored manipulation through same guarded AST patch and preview
  path.
- Keep derived tessellation and BRep vertices read-only unless compiler emits an
  exact source binding.

## Out Of Scope

- Generic mesh vertex editing.
- Rebuilding AutoCAD, FreeCAD, or a geometric constraint kernel in frontend.
- LLM-generated authoritative geometry projections.
- Deleting Sketch Workspace backend, tests, contracts, or saved draft data in
  first slice.
- Treating imported geometry without provenance as parametric source.

## Product Direction

Two synchronized lenses share one selection and authoring graph:

- spatial lens: Three.js model, selected geometry, focused dependency traces,
  source-backed handles, compact contextual controls
- source lens: AST/parameter/feature graph currently represented by New Params

Dragging a handle edits named source data. Selecting derived geometry without a
binding explains why it is read-only and reveals nearest owning feature or
parameters. LLM resolves ambiguous language into candidate AST patches; backend
compiler remains projection authority.

## Proof Gates

- Sketch launcher absent on workbench route.
- Stale saved layout cannot reopen Sketch Workspace.
- Geometry selection resolves through stable target and source identities.
- Supported handle drag validates source and node digests before preview.
- Unsupported derived geometry never claims editability.
- Accepted preview updates source and affected targets without committing until
  explicit Apply/Commit.
- Raw backend errors stay attached to responsible handle or source node.
