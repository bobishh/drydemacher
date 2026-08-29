# Change: Native FEM topology optimization

## Why

Current FEM evaluates authored solids but cannot derive load-path material layouts. This encourages decorative geometry instead of material placed from authored supports, contacts, protected regions, and load cases.

## What Changes

- Add deterministic SIMP compliance minimization on the existing native Tet4 mesh and sparse solver.
- Add weighted multi-load compliance, volume fraction, penalization, minimum density, volume-aware density filtering, convergence, and bounded iteration controls.
- Add generic passive-solid and passive-void regions resolved from authored durable face tags.
- Resolve the selected authored FEM study and build or reuse its immutable Tet4 mesh inside one topology-run operation.
- Publish immutable iteration traces, checkpoints, density fields, and diagnostic previews.
- Use automatic exact-BRep meshing through external Gmsh HXT with durable face mapping and immutable mesh identity.
- Use a bounded parallel SPD solve with measured backend identity and explicit runtime evidence.
- Restore immutable convergence evidence for the current source/model/study after panel or app restart.
- Keep density evidence and generic support-graph reconstruction distinct from authored production geometry.

## Scope

This change operates on any admitted Tet4 design domain with one or more authored linear-static load cases. Geometry, parameters, materials, supports, loads, protected regions, and topology targets come from the selected `.ecky` study.

Application config controls only model-independent runtime policy: resource limits, solver/mesher backends, algorithm selection, executable paths, and global safety bounds.

The topology runtime does not generate, recognize, validate, or publish product-specific geometry. Exact production geometry remains authored model state and requires independent FEM verification after any reconstruction or edit.

Implementation is independent. Open-source projects inform equations, boundaries, and tests; no third-party optimizer source is vendored or copied.
