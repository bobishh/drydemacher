# Verification: Native FEM Structural Analysis

## Current Proof

- Exact OCCT boundary: welded finite triangles, duplicate/degenerate rejection,
  closed/manifold/single-component/positive-volume gates, BRep vertex-edge-face-loop
  incidence, mesh face adjacency, durable face groups, selected-face area coverage.
- Native meshing: packaged pinned fTetWild probe, typed-array protocol, crash/raw
  stderr propagation, malformed/OOB/open input rejection, kill-on-cancel, no
  FreeCAD/Python/CalculiX/TetGen/Gmsh/network fallback path.
- Mechanics: Tet4 basis/constitutive/patch tests, exact surface load integration,
  Dirichlet elimination, Faer sparse solve, residual/equilibrium/energy gates,
  unaveraged verification stress, display-only nodal averaging, reactions.
- Artifacts: immutable mesh/result manifests, bounded digest-checked sidecars,
  exact singleflight cache, atomic VTU export with Tet4/displacement/nodal and
  element stress arrays.
- Cancellation: fTetWild and Faer run behind killable workers; assembly and
  postprocess poll chunk observers; convergence stops between levels. Shared
  jobs count independent subscribers and raise the worker kill token only when
  the final subscriber cancels. Cancelled work publishes no partial protocol or
  cache file.
- Engineering evidence: strict FEM units, exact result identity, authored
  acceptance evaluation, required-convergence pending state, applicability and
  uncertainty gates. Full ledger survives immutable artifact publication and is
  exposed as separate analytical, differential, convergence, and physical layers.
  Numerical success does not imply engineering readiness.
- Idealization: immutable digest-bound exact/defeatured artifact records source,
  analysis, and unchanged manufacturing geometry identities, affected topology,
  justification, influence threshold, and explicit approval. Reopen rejects
  altered artifact content or any attempt to replace manufacturing BRep.
- Independent reference: checked-in CalculiX 2.20/SPOOLES C3D4 bracket input and
  golden output. Ecky compares every loaded-node displacement, every element
  stress tensor, extrema/location, and support resultant without runtime CalculiX.
- Workbench: explicit validate/mesh/solve/convergence actions, partial failed
  convergence evidence, stale-source lockout, raw backend failures, preview-only
  overlays, VTU export.

## Recorded Commands

```text
cargo test ecky_cad_host::analysis_boundary::tests --lib
7 passed

cargo test commands::fem::tests::convergence_ --lib
6 passed; three-level identity/quality/residual/extrema/delta evidence plus
converged, suspected-singularity, quality-failed, solve-failed, and cancelled states

cargo test compiler_and_runtime_resolve_typed_fem_max_min_metrics_from_current_study --lib
1 passed; compiler-to-current-result fem-max/min stress, displacement, mass,
support reaction, safety-factor, typed-unit rejection, location, stale-study rejection

cargo test commands::fem::tests::exact_warm_run_uses_singleflight_cache_without_meshing_or_solving_again --lib
1 passed; real OCCT + packaged fTetWild; cold/warm/singleflight

cargo test services::fem_artifacts::tests::vtu_export_contains_tet4_displacement_and_separate_nodal_and_element_stress --lib
1 passed

cargo test --test fem_engineering_authoring
5 passed

cargo test -p ecky-render --test native_fem_bracket_contract
3 passed

cargo test --test fem_engineering_authoring
7 passed; strict FEM dimensions/non-finite values, complete missing/duplicate/
unsupported semantic matrix, exact tags, and analysis-to-geometry cycle rejection

cargo test analysis_declaration_is_source_addressable_stable_and_patchable_as_metadata --lib
1 passed; stable span/path/digest, AST read, whole-analysis guarded patch

cargo test render_core_program_manifest_includes_ast_identity --lib
1 passed; manifest carries compact analysis identity, part, element, source span

npx playwright test e2e/native-fem-analysis.spec.ts
14 passed; happy path plus ordinary-preview zero-worker proof, unavailable
fTetWild, invalid mesh, singular solve, cancelled solve, stale source, failed
convergence level, corrupt result array, and preview-only export invariants

npm run typecheck
0 errors, 0 warnings

npm run test:unit
367 passed

cargo test -p ecky-fem -- --test-threads=1
41 passed across contracts, Tet4 math, assembly, boundary loads, solver,
postprocess, applicability, sensitivity, axial oracle, and cantilever trend

cargo test -p ecky-fem --test linear_solver --test linear_static_solution
8 passed; SPD/singular/non-SPD/non-finite/residual/budget solver gates plus
pre-factorization rigid-mode and raw hidden-mechanism diagnostics; no springs

cargo test -p ecky-fem --test calculix_bracket_reference
1 passed; offline CalculiX displacement/stress/reaction differential

cargo test -p ecky-fem --test contracts numerically_green_never_hides_missing_evidence_nonlinearity_contact_or_decision_reversal
1 passed; numerical/convergence green remains blocked by missing provenance,
nonlinear regime, unsupported interface/contact, or decision-reversing uncertainty

cargo test -p ecky-fem --test contracts defeatured_idealization_is_a_digest_bound_artifact_that_cannot_replace_manufacturing_geometry
1 passed; topology, influence, approval, and manufacturing identity gates

cargo test parameter_sweep_preserves_current_selectors_and_rejects_removed_topology_without_coordinate_rebinding --lib
1 passed; live OCCT min/nominal/max plus hole transition, full boundary-group
coverage, current selector survival, removed topology explicit failure

ECKY_FTETWILD_RUNTIME_ROOT=../.dist/runtime/ftetwild cargo test --test fem_pipeline -- --test-threads=1
2 passed; real packaged worker, immutable result artifact, and outer failure
boundaries with no partial publication

cargo test --test fem_pipeline outer_failures_stop_at_resolve_boundary_or_rigid_mode_gate_before_publication
1 passed; unresolved tag/cancel stop at resolve, open solid stops at boundary,
underconstraint stops at rigid-mode gate, no worker/publication side effects

cargo test --test fem_mesher_runtime --test fem_mesher_worker
8 passed

cargo test services::fem_solver_worker::tests --lib
2 passed; finite worker solve plus forced termination without partial response

cargo test subscriber_ --lib
2 passed; first cancellation preserves shared work, final cancellation stops it,
and a later subscriber receives a fresh job generation

cargo test request_cache_identity_changes_for_every_physics_and_provenance_input --lib
1 passed; source, parameter snapshot, exact geometry, selectors, boundary mesh,
material, load, constraint, solver tolerance, meshing control, runtime binary

cargo test -p ecky-fem --test volume_mesh volume_mesh_content_mutation_changes_canonical_result_identity
1 passed; valid volume-mesh coordinate mutation changes canonical mesh identity

cargo test -p ecky-fem --test structural_patch assembly_observer_can_cancel_before_first_sparse_chunk
1 passed

cargo test -p ecky-fem --test postprocess postprocess_observer_can_cancel_before_first_result_chunk
1 passed

cargo test convergence_cancellation_returns_explicit_level_without_running_solver --lib
1 passed

cargo check
passed after generated camelCase contracts

cargo test -- --test-threads=1
1666 passed, 1 ignored; two failures exposed during the full run: legacy warm
cache artifact identity and stale commit-tool copy. Result artifacts were
versioned to v3/schema 7 with mandatory source identity; the stored final
engineering-ledger digest now matches post-solve content. Both exact failed
tests passed after repair. The unique cache fixture now uses a UUID, preventing
PID reuse from reopening abandoned test artifacts.

cargo test fem_verified_commit_rejects_red_stale_or_unbound_result --lib
1 passed; red, stale source, stale OCCT boundary, missing source identity, and
caller-mismatched result digest cannot reach commit

cargo test immutable_result_manifest_rejects_identity_and_readiness_tampering --lib
1 passed; solution identity, evidence digest, and decision-readiness flips are
rejected while immutable arrays remain hash checked

cargo test fem_mcp_surface_exposes_guarded_inspect_to_verified_commit_flow --lib
1 passed; MCP exposes inspect, validate, mesh preview, run, result reload,
structural verify, and exact FEM-verified commit stages. General commit states
that it makes no FEM claim.

cargo test --test architecture_fitness
4 passed; no debug overlay export, direct SQLite writes, local history path, or
frontend invoke boundary violation

static FEM runtime audit
clean for FreeCAD/Python/CalculiX/network/TetGen/Gmsh/rusqlite/history.sqlite;
only pinned fTetWild worker and Ecky-owned killable solver worker spawn

openspec validate native-fem-structural-analysis --strict
valid
```

## Deliberately Open

- Full BRep self-intersection proof beyond OCCT validity plus mesh manifold checks.
- Contact/interfaces, nonlinear material, fatigue, buckling, certification claims.
- Full physical/qualified-reference validation evidence supplied by project owner.
