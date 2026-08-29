# Proposal: Parameterized STL Import Preparation

## Why

Dense STL donors currently enter Ecky at full triangle count. A user can import,
preview, crop, and solidify them, but cannot bound import detail from canonical
`.ecky` source. Agents therefore reach for external Python tools and create
untracked derivative STL files. That bypasses source provenance, cache identity,
the External Shapes workflow, and visible render progress.

## What Changes

- Extend `import-stl` with optional, explicit preparation keywords for triangle
  target and absolute geometric error.
- Keep raw source bytes immutable and content-addressed.
- Produce a deterministic derived indexed mesh inside Ecky's cache; never make a
  hidden rewritten STL the geometry authority.
- Preserve raw source identity for mesh anchors, Crop, Guides, and validation.
- Expose Original/Prepared detail controls on the selected External Shapes Import
  source and apply them through an AST patch.
- Record requested and achieved triangle count, achieved error, algorithm version,
  raw digest, and derived digest in artifact provenance.
- Let `solidify` consume the prepared mesh when a later BRep operation or STEP
  export requires a solid.
- Show import, validation, preparation, cache, and failure state in the active
  task instead of running an invisible agent-side conversion.

## Canonical Example

```lisp
(model
  (params
    (number donor-triangles 40000
      :label "Donor triangle target"
      :min 5000 :max 250000 :step 5000)
    (number donor-max-error 0.05mm
      :label "Donor maximum deviation"
      :min 0.01mm :max 0.5mm :step 0.01mm :unit length))
  (part donor
    (solidify
      (import-stl "hydrant.stl"
        :target-triangles donor-triangles
        :max-error donor-max-error
        :preserve-boundaries #t))))
```

Without keywords, `import-stl` retains current exact indexed import behavior.

## Out of Scope

- Silent simplification.
- Automatic geometry repair or topology-changing fallback.
- Combining `solidify` with `import-stl`.
- Editing the original STL file.
- Python, Blender, Meshlab, or another external preprocessing dependency.
- Treating a simplified decorative donor as exact fit geometry without an
  authored deviation bound.

## Proof Gates

- Existing one-argument `import-stl` models render identically.
- Prepared import remains one oriented manifold component when the source is one.
- Requested error is never exceeded; an unreachable triangle target reports the
  achieved count without weakening the error bound.
- Raw and prepared digests, counts, and error appear in artifact provenance.
- Identical source digest and policy reuse one immutable cache entry.
- External Shapes can preview Original and Prepared states and apply parameters to
  the exact selected AST node.
- `solidify(import-stl(...prepared...))` reaches the hybrid/BRep pipeline without
  fabricating analytic geometry.
- No external derivative STL is required.
