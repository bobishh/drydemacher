# Design

## Authored-study authority

The selected `.ecky` analysis is the sole source of model-specific inputs:

- design-domain part and current parameter values;
- material;
- volume-mesh and refinement controls;
- fixed, prescribed, and loaded durable face tags;
- passive-solid and passive-void regions;
- load cases and weights;
- topology targets and authored reconstruction constraints.

Rust consumes compiled generic FEM contracts. It must not contain product names, dimensions, geometry source templates, default mounting layouts, or hard-coded load cases.

Application config may bound or select model-independent runtime behavior only: CPU/thread count, memory and wall time, solver/mesher backend, executable paths, numeric tolerances, safety budgets, and generic algorithm choice. Config limits authored requests; it does not replace authored model inputs.

## Kernel

Each free design element owns design density `x in [rho_min, 1]`. A sparse row-normalized physical filter maps `x -> rho`. Element stiffness uses `E(rho) = E_min + rho^p (E_0 - E_min)`. Objective is weighted compliance across authored load cases. Volume uses physical density and actual Tet4 volume.

Sensitivity uses element strain energy from each converged displacement field. Weighted load-case sensitivities are summed in deterministic authored order. Physical-density sensitivities pass through the transpose filter before the constrained optimizer update.

Passive-solid and passive-void cells never become optimizer variables. Their prescribed physical volume enters the affine volume constraint explicitly. Passive sets must be unique, disjoint, in range, and derived from authored durable face tags.

The tested OC implementation remains a reference path. Production updates use deterministic MMA/GCMMA with bounded conservative inner evaluations. Convergence requires sustained filtered-density stability plus bounded KKT residual. Objective delta or fixed iteration count cannot substitute for convergence evidence.

Pipeline:

`authored study -> exact mesh -> design x -> physical rho -> assemble/solve all RHS -> compliance/sensitivity -> MMA/GCMMA -> checkpoint/result`.

## Parallel sparse solve

One symmetric positive-definite stiffness matrix is assembled and factorized per topology iteration. All authored load-case right-hand sides reuse that factorization.

Runtime evidence records backend/version, matrix digest, RHS count, thread-control mode, observed worker use, factor/solve time, memory, and residuals. Unsupported requested parallel execution fails explicitly; no silent sequential relabeling.

Backend selection uses generic representative SPD and multi-RHS fixtures. Numerical equivalence precedes performance ranking. Product geometry is never embedded in backend selection or runtime API.

## Exact meshing and identity

Exact analytic BRep studies use external Gmsh HXT from generated STEP, not viewer tessellation. The runtime resolves durable tagged faces, invokes bounded meshing automatically, validates boundary mapping and mesh quality, and assigns the checked mesh an immutable content digest.

MCP callers identify the authored thread/model/study. They do not supply internal cell indices or fabricate analysis/mesh identities.

## Reconstruction boundary

Density fields, threshold surfaces, tetrahedral edges, and support graphs are analysis evidence.

Generic reconstruction may:

- retain a dominant support-connected component;
- reject and report disconnected density islands;
- derive a sparse graph between authored named anchors;
- smooth or mirror graph geometry using authored generic controls;
- emit data suitable for model authoring.

Generic reconstruction must not emit a product-specific `.ecky` source, invent geometry, infer semantic roles from anchor names, or publish an exact product automatically. Model-specific conversion and fit relations remain in authored `.ecky` source.

Any exact geometry produced later must be previewed, structurally verified, remeshed, and independently solved against its authored studies before engineering acceptance.

## Persistence and cancellation

Every iteration is bounded by iteration, solve, element, memory, result-size, cancellation, and wall-clock budgets. State checkpoints are immutable and sufficient for exact pause/resume.

FEM evidence is keyed by current source digest, model identity, authored study, mesh sequence, controls, and result digests. Restore is read-only and never silently reuses stale evidence or starts a solve.

## Proof strategy

- Finite-difference sensitivity on a tiny Tet4 mesh.
- Filter normalization, transpose-gradient, nonuniform-volume, and passive-mask tests.
- OC reference and MMA/GCMMA parity/KKT/checkpoint tests.
- Generic cantilever and multi-load design-domain acceptance fixtures.
- Step/resume equivalence and cancellation boundaries.
- Exact-BRep Gmsh face mapping and immutable mesh identity.
- Parallel sparse multi-RHS numerical/performance replay.
- Generic support-graph connectivity, island rejection, smoothing, and symmetry tests.
- Independent remeshed FEM required for any authored exact-geometry result.
