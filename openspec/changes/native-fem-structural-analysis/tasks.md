# Tasks: Native FEM Structural Analysis

Implementation follows the repository's outer BDD plus inner unit
red-green-refactor loop. No production FEM code lands without the corresponding
failing mathematical or boundary test.

## 1. Outer Contract And Dependency Audit

- [x] 1.1 Add one failing end-to-end integration fixture for a parameterized
  `.ecky` bracket with durable mounting/load tags, one linear-static study,
  aluminum material, Tet4 controls, fixed support, and total surface force.
- [x] 1.2 Assert the outer result contract: native-only execution, non-empty
  volume mesh, finite displacement/stress, reaction equilibrium, mass, exact
  source/geometry/result digests, and no FreeCAD, arbitrary Python, CalculiX, or
  network invocation.
- [x] 1.3 Add failing outer cases for unresolved face tag, open domain,
  underconstrained rigid body, cancelled job, and stale result after a parameter
  change; confirm each failure occurs at the intended stage.
- [x] 1.4 Audit exact versions, APIs, transitive licenses, platform support, and
  maintenance status for Gmsh HXT, Netgen, Fenris, Faer,
  `mshio`, and `vtkio`. Record decision and lock versions before dependencies.
- [x] 1.5 Add runtime/license gate proving probed Gmsh executable and optional
  Netgen interpreter/module digests, protocol, and license evidence. Reject
  TetGen/AGPL and bundled/linked GPL Gmsh without approved licensing evidence.

## 2. Typed `.ecky` Analysis Surface

- [x] 2.1 Red parser tests for top-level `analysis`, `linear-static`, `material`,
  `volume-mesh`, `refine`, `fixed`, `prescribed-displacement`, `surface-force`,
  `traction`, `pressure`, and `solve` forms with source spans and stable node
  identity.
- [x] 2.2 Add the minimal surface AST and Core analysis declarations without
  making analysis a geometry node or shape-producing expression.
- [x] 2.3 Red unit/dimension tests for N, MPa, force vectors, density, stress,
  displacement, strain, and material fields; include dimensionless-load,
  force-as-pressure, and non-finite failures.
- [x] 2.4 Extend unit normalization and diagnostics. Keep FEM fields strict even
  when legacy CAD source is in permissive unit mode.
- [x] 2.5 Red semantic tests for duplicate study/material names, unsupported
  study/element/solver kinds, missing part/material/mesh/constraint/load,
  invalid Poisson ratio/modulus/density/yield, and analysis-to-geometry cycles.
- [x] 2.6 Add minimal semantic validation and preserve analysis declarations in
  source maps, AST reads/patches, dependency inspection, and model manifest.
- [x] 2.7 Red selector tests showing `(tag name)` resolves to exact/durable face
  targets for the declared part and ambiguous/missing/cross-part selectors fail.
- [x] 2.8 Reuse the existing topology selector service; do not add a coordinate
  guessing path or FEM-specific duplicate selector engine.
- [x] 2.9 Refactor grammar/validation tables while all focused parser and
  semantic tests remain green.

## 3. FEM Domain Crate And Contracts

- [x] 3.1 Add `src-tauri/crates/ecky-fem` to the workspace with no Tauri,
  frontend, persistence, FreeCAD, or MCP dependency.
- [x] 3.2 Red serialization/digest tests for material, load, constraint, mesh
  control, analysis identity, runtime identity, and camelCase DTO mirrors at
  the app boundary.
- [x] 3.3 Implement finite, bounded, versioned domain contracts and deterministic
  canonical hashing. Keep Fenris/Faer/Gmsh/Netgen types private to adapters.
- [x] 3.4 Red budget tests for boundary triangles, Tet4 cells, nodes, DOFs,
  sparse nonzeros, result bytes, and convergence levels with observed/allowed
  diagnostics.
- [x] 3.5 Add admission estimates and budget enforcement before expensive
  allocation where possible.

## 4. Provenance-Preserving OCCT Boundary Mesh

- [x] 4.1 Red native-runner test: tessellate a tagged solid and assert every
  oriented triangle carries its source canonical/durable face group and source
  geometry digest.
- [x] 4.2 Add `AnalysisBoundarySurface` and Direct OCCT output support without
  changing `IndexedMeshAsset`, STL, STEP, or normal preview contracts.
- [x] 4.3 Red topology tests for finite vertices, degeneracy, duplicate
  triangles, winding, closedness, one component, positive volume,
  non-manifold/boundary edges, face-group cardinality, and BRep-versus-mesh
  vertex/edge/face/loop incidence equivalence.
- [x] 4.4 Implement boundary validation, outward orientation, named seam weld,
  group area summaries, and deterministic digest.
- [x] 4.5 Red selector-coverage test: selected CAD face area and emitted boundary
  facets agree within tessellation tolerance, including local face refinement.
- [x] 4.6 Reject partial/ambiguous coverage and any STL round-trip; include part,
  selector, target ids, expected/observed area, and tolerance in diagnostics.
- [x] 4.7 Prove normal render artifacts and manufacturing export digests are
  byte/digest-identical with and without requesting an analysis boundary mesh.

## 5. External Gmsh HXT / Netgen Mesher

- [x] 5.1 Red capability-probe tests for valid runtime, missing executable/source,
  wrong platform/arch/ABI, bad digest, unsupported version, and missing license
  metadata.
- [x] 5.2 Add explicit Gmsh HXT and optional Netgen runtime probing, recording
  executable/interpreter/module paths and digests; support configured external
  development/runtime paths only.
- [x] 5.3 Red worker-protocol tests for valid tagged closed surface, malformed
  array lengths, out-of-range indices, open/non-manifold surface, budget excess,
  unsupported element order, native crash, stderr propagation, and cancellation.
- [x] 5.4 Implement dedicated Rust Gmsh HXT worker and narrow adapter using
  exact-BRep STEP input and bounded ASCII MSH2 output; preserve OCC face-group
  identity and keep Netgen fallback behind the same request contract.
- [x] 5.5 Red mesh-generation test proving one closed domain becomes Tet4 cells
  and source boundary groups survive global and local refinement.
- [x] 5.6 Implement tagged input boundary facets, one volume region,
  explicit deterministic envelope/options/thread policy, and typed-array result
  handoff with insertion, source-tag, and approximation evidence.
- [x] 5.7 Red `FemVolumeMesh` tests for finite coordinates, node references,
  duplicate/repeated cells, signed volume, boundary ownership, group coverage,
  connectedness, minimum quality, and worst-element location.
- [x] 5.8 Implement orientation normalization, canonical numbering/digest,
  boundary reconciliation, quality metrics, and success-only atomic output.
- [x] 5.9 Add no-fallback tests proving Gmsh HXT/Netgen failure never invokes
  TetGen, FreeCAD, untagged STL remeshing, or remote service.
- [x] 5.10 Refactor worker lifecycle and ensure cancellation/restart leaves no
  orphan process, global native state, or partial mesh cache entry.

## 6. Tet4 Element And Assembly Inner Loop

- [x] 6.1 Red mathematical tests for tetrahedron signed volume, reference/world
  shape gradients, partition of unity, and affine-coordinate reproduction.
- [x] 6.2 Implement or adapt the minimal Tet4 geometry/basis functions behind
  `ElementAssembler`; pin Fenris and expose none of its public types.
- [x] 6.3 Red constitutive tests for isotropic 3D elasticity tensor symmetry,
  positive energy in valid material range, and known uniaxial stress/strain.
- [x] 6.4 Implement the small-strain constitutive operator with documented Voigt
  ordering and shear convention.
- [x] 6.5 Red constant-strain patch test and rigid-translation/rotation
  zero-strain tests on one and multiple tetrahedra.
- [x] 6.6 Implement Tet4 `B`, `B^T D B V`, element body-independent stiffness,
  and local-to-global sparse assembly.
- [x] 6.7 Red surface integration tests: triangle traction and pressure produce
  exact total resultant/moment within tolerance; total surface force sums to
  the authored vector independent of triangulation density.
- [x] 6.8 Implement fixed, component-wise prescribed displacement, total surface
  force, traction, and inward pressure assembly using boundary group evidence.
- [x] 6.9 Add deterministic parallel/sequential assembly differential tests if
  parallel assembly is enabled; otherwise keep MVP assembly reproducible and
  bounded.

## 7. Sparse Constraint And Solve Inner Loop

- [x] 7.1 Red tests proving Dirichlet elimination preserves symmetry, enforces
  non-zero prescribed values, and does not use penalty stiffness.
- [x] 7.2 Implement reduced-system construction and retain data needed for
  support reactions.
- [x] 7.3 Red Faer-adapter tests for SPD solve, singular matrix, non-SPD matrix,
  non-finite factor/result, residual failure, and budget rejection.
- [x] 7.4 Implement `LinearSolver` with sparse direct Cholesky/LDLT, explicit
  ordering/tolerance identity, residual calculation, and raw backend failure
  details.
- [x] 7.5 Red structural tests for an underfixed rigid body and hidden mechanism;
  require actionable unconstrained-DOF/likely-rigid-mode diagnostics and no
  hidden springs.
- [x] 7.6 Add pre-solve rigid-mode checks where deterministic and preserve
  factorization evidence when only the solver can diagnose singularity.
- [x] 7.7 Red equilibrium/energy tests proving applied plus reaction resultant is
  within tolerance and strain energy is finite/non-negative.
- [x] 7.8 Implement reactions and solve acceptance gates. Publish no result when
  residual, equilibrium, energy, or finite-value checks fail.

## 8. Post-Processing And Result Artifacts

- [x] 8.1 Red tests for element strain/stress, von Mises, principal stress,
  displacement magnitude, volume, mass, yield safety factor, and typed infinite
  safety factor at zero stress.
- [x] 8.2 Implement unaveraged element results and separately labeled
  volume-weighted nodal display averaging; verification maxima use unaveraged
  values.
- [x] 8.3 Red extrema tests requiring value, unit, element/node id, coordinate,
  field kind, mesh identity, and boundary/source provenance when available.
- [x] 8.4 Implement compact summaries, extrema, reactions by support group, and
  a boundary result surface for the viewer.
- [x] 8.5 Red artifact tests for binary-array manifest bounds, scalar type,
  shape/range/digest validation, truncation, corruption, and oversized result.
- [x] 8.6 Implement immutable `FemResultAsset`, versioned bounded sidecars,
  optional VTU export, and atomic success-only publication.
- [x] 8.7 Add exact cache/staleness tests across source, params, geometry,
  selectors, boundary mesh, volume mesh, materials, loads, constraints,
  tolerances, and runtime identities.

## 9. Runtime Orchestration, Progress, And Cache

- [x] 9.1 Red service test proving ordinary geometry preview compiles but does
  not mesh or solve an analysis.
- [x] 9.2 Add explicit validate, mesh-preview, solve, cancel, result-read, and
  convergence service operations with Tauri camelCase DTOs and Rust snake_case
  fields using `#[serde(rename_all = "camelCase")]`.
- [x] 9.3 Red progress test for ordered resolve, boundary-mesh, volume-mesh,
  validate-mesh, assemble, apply-constraints, solve, postprocess, verify, and
  publish stages with counts and elapsed time.
- [x] 9.4 Implement typed activity/progress without a new status bar and without
  dumping native interactive output into general app logs.
- [x] 9.5 Red cancellation tests at Gmsh HXT/Netgen, assembly, factorization, postprocess,
  and convergence levels; require no worker/orphan/partial artifact.
- [x] 9.6 Add cooperative chunk cancellation and move uninterruptible native or
  direct-solve stages behind a killable worker before claiming cancellation.
- [x] 9.7 Red singleflight/cache tests: identical concurrent studies execute one
  job; warm exact identity performs zero mesh/assembly/solve; failures are not
  cached.
- [x] 9.8 Implement bounded immutable cache and subscriber-aware cancellation.
- [x] 9.9 Add one workbench run intent that accepts only current target inputs;
  Rust owns job identity, configured validation/compute policy, validate-run
  sequencing, progress, cancellation registration, and result publication.
- [x] 9.10 Replace workbench validate, mesh preview, convergence run/cache, and
  VTU export chains with action-specific intents. Rust owns job identities,
  configured budgets/control/tolerances, preview validation sequencing, and the
  export byte cap; frontend keeps only user inputs, projection, raw errors, and
  cancellation from Rust-emitted identity.

## 10. Verification And MCP Surface

- [x] 10.1 Red compiler/runtime tests for `fem-max`, `fem-min`, mass, reaction,
  and safety-factor metrics with unit-checked thresholds.
- [x] 10.2 Add FEM metric resolution to authored verification. Missing or stale
  results fail/pending honestly and never reuse a mismatched analysis identity.
- [x] 10.3 Red verification diagnostic test requiring study, field, value, unit,
  threshold, mesh size, location, result digest, and convergence status.
- [x] 10.4 Add a specialist MCP capability group with compact validate,
  mesh-preview, run, result-get, and convergence operations; route bulk arrays
  as artifacts rather than message JSON.
- [x] 10.5 Red MCP flow: inspect `.ecky` -> validate study -> mesh preview -> run
  -> verify green -> commit; a stale/red result must block green commit claims.
- [x] 10.6 Preserve existing parameter preview/AST patch tools as the only model
  mutation path; FEM tools do not write source, manufacture geometry, or create
  speculative history versions.

## 11. Mesh-Convergence Evidence

- [x] 11.1 Red convergence test for three explicit sizes reporting mesh/result
  digests, quality, counts, residual, extrema, and relative metric deltas.
- [x] 11.2 Implement sequential bounded refinement runs with reuse only where
  exact artifact identity permits it.
- [x] 11.3 Red status tests for converged displacement, unconverged stress,
  quality failure, failed intermediate solve, cancelled sequence, and rising
  hotspot/suspected singularity.
- [x] 11.4 Implement configured consecutive-delta criteria and preserve
  per-metric `converged`, `unconverged`, `failed`, or `suspectedSingularity`
  status. Never average away a red level.
- [x] 11.5 Require convergence when an authored verification check explicitly
  requests it; otherwise display the absence of convergence evidence.

## 12. Workbench And Viewport

- [x] 12.1 Red Playwright flow from bracket source through study validation,
  mesh generation, solve, field selection, extrema inspection, and VTU export.
- [x] 12.2 Add an Analysis section to the existing workbench control dock using
  Tactical Midnight, square borders, bronze `--primary`/`--secondary`, and
  `overflow: hidden` on major layout containers.
- [x] 12.3 Red UI failure/pending cases for unavailable Gmsh HXT runtime, invalid
  mesh, singular solve, running/cancelled study, and stale result; assert raw
  backend detail remains visible.
- [x] 12.4 Add result legend, undeformed outline, deformed boundary scale,
  Tet4/clip view, field picker, quality summary, reactions, and convergence
  table without introducing a separate agent status bar.
- [x] 12.5 Red export-boundary test: changing field, deformation scale, mesh
  overlay, clip, or legend leaves BRep/STL/STEP/manufacturing digests unchanged.
- [x] 12.6 Make all FEM overlays preview-only GPU/display data and keep them out
  of production export geometry.
- [x] 12.7 Red stale-state browser case: edit one geometry/load/material param,
  preview, and assert the old result is visibly stale and cannot show a current
  green verification chip.

## 13. Independent Numerical Reference Gates

- [x] 13.1 Add an axial-bar fixture and compare displacement/stress/reaction to
  closed-form values at recorded tolerances.
- [x] 13.2 Add a bending/cantilever refinement fixture; record expected Tet4
  limitations and require the correct convergence trend rather than an
  unjustified exact tolerance.
- [x] 13.3 Add a versioned offline CalculiX bracket input, solver/version
  provenance, and golden displacement/stress/reaction output. The test reads
  checked-in reference data and does not require CalculiX at runtime.
- [x] 13.4 Differentially compare Ecky against the independent reference with
  declared norm/extrema/location tolerances; investigate rather than widening a
  tolerance after a surprise failure.
- [x] 13.5 Add regression fixtures for pressure normal sign, load distribution,
  face-group mapping, stress averaging distinction, and support reactions.
- [x] 13.6 Add parameter-sweep fixture covering nominal, bounds, and one topology
  transition. Record selector/correspondence survival, boundary-group coverage,
  and explicit failure instead of coordinate-based rebinding.

## 14. Engineering Model Adequacy And Validation

- [x] 14.1 Add typed engineering-question, acceptance-criterion, idealization,
  assumption, evidence-provenance, uncertainty, applicability, sensitivity, and
  physical-validation contracts to analysis identity.
- [x] 14.2 Add authoring/semantic tests proving missing material/load/support/
  connection evidence remains proposed or unknown; no agent/default value can
  satisfy green verification without recorded acceptance.
- [x] 14.3 Add explicit analysis-idealization artifact and tests for defeatured
  geometry identity, affected topology, justification, influence threshold,
  user approval, and unchanged manufacturing BRep.
- [x] 14.4 Add deterministic pre-solve applicability audit for one-solid scope,
  unsupported interfaces, thin/slender Tet4 risk, near-incompressible locking,
  constraint realism, and concentrated load/support singularity risk.
- [x] 14.5 Add post-solve applicability gates for displacement/characteristic-size
  ratio, declared elastic range, hotspot movement, and boundary-condition
  singularity classification. Preserve numerical result while blocking green
  engineering decision.
- [x] 14.6 Add load/material/support sensitivity and bounded uncertainty runs
  independent of mesh convergence. Report dominant inputs and decision-changing
  response ranges.
- [x] 14.7 Separate analytical/unit proof, differential solver verification,
  mesh convergence, and physical/reference validation in contracts and UI.
  CalculiX agreement must not become physical validation.
- [x] 14.8 Add evidence-chain inspection from acceptance metric through result,
  mesh, analysis geometry, idealization, material/load/support provenance,
  applicability, convergence, sensitivity, and validation records.
- [x] 14.9 Add outer BDD cases: numerically green but missing engineering
  evidence; converged result outside linear regime; unsupported contact load
  path; uncertainty that reverses acceptance decision.

## 15. Final Gates

- [x] 15.1 Run focused red-green suites after each slice, then all
  `ecky-fem`, compiler, Direct OCCT, service, MCP, artifact, and frontend unit
  tests.
- [x] 15.2 Run the relevant Playwright happy path plus unavailable-runtime,
  singular/stale, and cancellation cases on the isolated test app.
- [x] 15.3 Run `cd src-tauri && cargo check` and `cargo test`; record any
  platform-gated external Gmsh HXT/Netgen tests and their CI matrix evidence.
- [x] 15.4 Run the full bracket convergence and offline reference gates and
  preserve measured counts, quality, residuals, equilibrium, and deltas.
- [x] 15.5 Run MCP inspect -> validate -> mesh -> solve -> verify -> commit on
  the bracket demonstrator; do not commit a red/unconverged required check.
- [x] 15.6 Run `openspec validate native-fem-structural-analysis --strict`.
- [x] 15.7 Confirm no FreeCAD/Python/CalculiX/network/TetGen runtime invocation
  outside the explicit Gmsh HXT/Netgen adapters, no SQLite direct writes, no
  partial artifacts, no manufacturing digest drift, and no engineering-
  certification language.
