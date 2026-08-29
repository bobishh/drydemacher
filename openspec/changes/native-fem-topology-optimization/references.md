# Open-source design review

Research date: 2026-08-17. References guide an independent Rust implementation. No source is vendored or copied.

## Exact-BRep tetrahedral meshing

- Gmsh 4.15.2 HXT: official parallel Delaunay tetrahedral mesher with OpenCASCADE STEP import, size fields, entity-preserving boundary mesh, and explicit thread controls. GPL runtime remains external and is not linked or redistributed.
- Netgen 6.2.2606: LGPL OCC/STEP tetrahedral mesher measured as alternative.
- Representative analytic STEP, global size 2.4 mm, Apple M5 development host: HXT 0.62-0.67 s wall, about 14.2k nodes and 41.8k Tet4, 300/300 BRep faces tagged; Netgen warm run 5.54 s wall, 25.7k nodes and 89.8k Tet4, 300/300 faces tagged; prior triangle-soup meshing exceeded 600 s at the same global size.
- Decision: exact-BRep FEM uses automatic external HXT. Netgen remains measured reference. fTetWild is outside this change; STL repair/solidification requires its own future capability and spec.

## Scikit-Topt

- Source: https://github.com/kevin-tofu/scikit-topt
- License: Apache-2.0.
- Relevant: unstructured meshes, multiple load cases, SIMP/RAMP sensitivities, OC, sparse solves, iteration tests.
- Adopt: separate solver/filter/update/history boundaries; weighted compliance; finite-difference and step-level tests.
- Reject: Python runtime dependency; current project maturity as authoritative production proof.

## TopOpt.jl

- Source: https://github.com/JuliaTopOpt/TopOpt.jl
- License: MIT.
- Relevant: 2D/3D unstructured topology optimization, volume-aware differentiable density filters, projection, fixed/non-design domains, multiple loads.
- Adopt: sparse linear filter plus transpose sensitivity; actual cell-volume weighting; explicit fixed-domain model.
- Defer: projection continuation, RAMP, BESO, TOBS, uncertainty models.

## FEniTop

- Source: https://github.com/missionlab/fenitop
- License: GPL-3.0.
- Relevant: 2D/3D unstructured meshes; explicit filter -> projection -> FEM -> sensitivity -> backward-filter -> OC/MMA pipeline; passive bounds; Helmholtz filter.
- Adopt concept only: pipeline ordering and passive-region invariants.
- Reject: source reuse; FEniCSx/PETSc/MPI dependency; Helmholtz filter in v1.

## DTU Top88

- Source: https://www.topopt.mek.dtu.dk/apps-and-software/efficient-topology-optimization-in-matlab
- Relevant: canonical compact SIMP/OC and density-filter benchmark; passive and multiple-load extensions.
- Adopt: benchmark expectations and equation cross-check.
- Reject: structured 2D grid assumptions and MATLAB implementation.

## NonconvexMMA.jl

- Source: https://github.com/JuliaNonconvex/NonconvexMMA.jl, inspected revision `fa9352c434e9d4bd7a2f55bae3f82af757116e2a`.
- License: MIT, copyright Mohamed Tarek and contributors.
- Relevant: MMA87 moving-asymptote update; MMA02/GCMMA objective/constraint curvature lifts; exact-function conservative-approximation test; bounded inner loop; KKT convergence.
- Adopt: clean-room single-affine-constraint Rust specialization with scalar dual bisection, exact compliance re-evaluation, checkpointed lifts, and equation fixtures.
- Convergence: package defaults to `KKTCriteria`; its MMA tests set positive KKT tolerance with objective-delta tolerance disabled. Ecky additionally requires stability of the filtered physical-density field used by FEM and reconstruction, not raw movement of a volume-negligible design variable.
- Reject: Julia/Optim runtime dependency and general multi-constraint allocation model.

## Alternatives not selected

- ToPy: https://github.com/williamhunter/topy — useful older 2D/3D reference; Python-2-era architecture and licensing/dependency fit inferior.
- PyTopo3D: https://github.com/jihoonkim888/PyTopo3D — useful 3D SIMP example; structured hexahedral domain does not match native Tet4 pipeline.

## Decision

V1: native Rust, existing Tet4 FEM, measured parallel sparse backend, SIMP compliance, deterministic OC, weighted multiple loads, compact-support volume-aware density filter, passive solid/void cells, immutable bounded trace. Density preview remains evidence. Printable geometry and exact BRep remain later gates.

## Parallel sparse backend spike

Representative immutable fixture: 11,700 Tet4 before reduction, 9,255 reduced SPD degrees of freedom, 269,985 stored nonzeros, and five identical RHS. Release/`-O3` runs on the development Apple-silicon host produced:

| Backend | Factor | Five-RHS solve | Maximum relative residual |
| --- | ---: | ---: | ---: |
| Apple Accelerate Sparse Cholesky | 16.6 ms | 3.0 ms | 1.32e-12 |
| SuiteSparse/CHOLMOD 7.14 supernodal | 17.2 ms | 2.3 ms | 1.69e-12 |
| Faer 0.24.4 sequential | 33.9 ms | 3.4 ms | 1.49e-12 |
| Faer 0.24.4 Rayon, requested 8 threads | 56.9 ms | 49.6 ms | 1.49e-12 |

This fixture rejects Faer/Rayon as an assumed shortcut: parallel overhead makes it slower at this size. Accelerate and CHOLMOD remain candidates. Selection waits for the production-scale approximately 50k-DOF replay plus observed multicore and peak-memory evidence; configured thread count alone is not proof.

Production replay used immutable mesh `sha256:bc9ec72fb51add1876d8014fe64928e0133190f9d446e26f66399f91faa4ffca`: 50,287 Tet4, 46,110 reduced DOF, 1,527,064 stored nonzeros, constrained support groups, and five RHS. Accelerate averaged 32.2 ms factor and 7.8 ms solve over 100 factor replays, residual 5.13e-12, 156 MB maximum RSS, and 6.92 CPU seconds over 4.02 wall seconds (1.72 observed effective cores). CHOLMOD averaged 41.4 ms factor and 6.7 ms solve, residual 6.33e-12, 146 MB maximum RSS, and 5.19 CPU seconds over 5.04 wall seconds (1.03 effective cores). Faer sequential measured 66.1 ms factor plus 8.3 ms solve; Faer/Rayon-8 measured 128.8 ms plus 8.1 ms. Accelerate wins latency and observed-parallel gates. Its supported API selects multi-threaded mode while Accelerate manages the numeric count; runtime identity must not claim an invented count.

## Topology preprocessing profile

Run date: 2026-08-18, Apple M5, identical deterministic workloads. The 50,000-element spatial filter produced 1,552,432 canonical neighbor links with a conservative 58,477,824-byte estimate. A clean dev build with `ecky-fem` forced to opt-level 0 took 637.1 ms; the workspace opt-level 3 profile took 95.3 ms, a 6.69x reduction. Exact all-pairs equivalence, canonical neighbor order, cancellation, and wall-limit-before-first-solve tests passed.

The 50,287-element iteration-preprocessing replay used 150,870 DOF, 3,168,162 canonical nonzeros, and five RHS. Reusable adjacency-plan construction took 34.7 ms, numeric stiffness accumulation 13.9 ms, one shared Dirichlet reduction 19.6 ms, and 96 exact affine-volume evaluations 4.0 ms. Zero/nonzero prescribed-value equivalence, explicit filtered-volume equivalence with passive rows, adjacency-assembly equivalence, reusable-pattern equivalence, passive invariants, and fixed bisection count passed.

The Accelerate adapter borrows canonical row-major entries and extracts symmetric-upper coordinates directly; only noncanonical compatibility owns rebuilt entries. Borrowed/owned path, noncanonical solve, symmetry, and residual gates passed. The production sparse replay remains 46,110 reduced DOF and 1,527,064 stored nonzeros with 32.2 ms factor plus 7.8 ms five-RHS solve.

Expanded-AABB passive-region selection processed 50,000 points against 300 triangles in 20.5 ms and selected 3,050 cells. Exhaustive exact point-triangle membership equivalence passed independently.
