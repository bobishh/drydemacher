# Proposal: Foreign Component Import Lifecycle

## Why

FreeCAD-library import currently spreads one imported component across message,
runtime, manifest, project-source, and Viewer state. Code can ask a macro API for
an imported message with no macro. Project switching can discard a valid cached
preview before the target is ready. Treating the calculated STL as authored
`(import-stl ...)` source then destroys FreeCAD component identity, bindings, and
parameter evidence.

Import needs one durable imported-component aggregate. Its typed
`(freecad-component ...)` descriptor identifies donor, runtime, manifest,
bindings, and parameters. Calculated STL remains immutable Viewer cache, never
authoring source. A later verified Ecky rewrite is a new ordinary Ecky version.

## What Changes

- Recursively discover supported FCStd and STEP files below
  a selected root. Persist and display the exact root.
- Preserve donor bytes, digest, format, measured bounds, and unit evidence in the
  managed project asset store.
- Persist the calculated STL and native donor/runtime files as version-owned,
  content-addressed assets referenced by one imported-component record.
- Display read-only evidence beside a typed `(freecad-component ...)` descriptor.
- Route descriptor Apply/Commit through one Rust imported-parameter intent that
  owns `apply_imported_model`, semantic carry-forward, optional immutable version
  append, and runtime snapshot projection; never through macro rendering.
- Load cached runtime directly by content identity. Project reopen does not render
  or reopen FCStd/STEP when persisted runtime files exist.
- Keep the previous model visible until the target artifact is ready or loading
  fails; project switching never emits an intentional empty Viewer frame.
- Record conversion lifecycle as durable structured state consumed by the normal
  agent authoring loop. Do not synthesize chat commands or call a hidden LLM path.
- Remove numeric confidence from deterministic import heuristics. Show provenance,
  facts, and warnings only.

## Out of Scope

- Automatic reconstruction of donor BRep features into native Ecky primitives.
- Blocking print export on pending or failed Ecky conversion.
- Changing standalone STL/OBJ/3MF import behavior.
- Deleting donor assets still referenced by history.
