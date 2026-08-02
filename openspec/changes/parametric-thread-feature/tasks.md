# Tasks: Parametric Thread Primitive

Builds on the landed radial-thread fix (`1a58364`: Frenet + RightCorner sweep).
Author tests first (BDD red→green) per task; verify native ↔ build123d parity.

## 1. Fix the `thread` coincident-face / hollow bug (highest priority)

- [x] 1.1 (test) external `thread` with coarse/deep params (e.g. d12 pitch3
  depth1.2) produces ONE solid with the expected solid-core volume — not a
  hollow spiral. Guard against regression: assert `volume` ≈ core+ridge, and
  `connected-component-count = 1`. DONE 2026-07-06: fix (1.2) landed in
  `64af169` without an automated test — only manual verification in the
  commit message. Added
  `thread_union_stays_solid_on_coarse_deep_params_regression` in
  `direct_occt_executor.rs` (radius 6/pitch 3/length 10/depth 1.2, matching
  the bug's repro shape); confirmed it actually catches the regression by
  temporarily zeroing the fix's `overlap` term (hard failure — runner
  couldn't even triangulate the degenerate shape) then restoring it (green:
  1 component, volume 1426 ≥ 0.9× bare-core 1131). STL facet-adjacency
  non-manifold counting is NOT used as a signal here — helical sweep
  tessellation produces legitimate seam T-vertices the heuristic flags on a
  valid solid; volume + component count are the real hollow-vs-solid proof.
- [x] 1.2 Build the core with an internal overlap in `expand_thread_node`
  (native) and `_ecky_thread` (build123d): `core_radius = radius + overlap`,
  ridge `:depth = depth + overlap`. No coincident core/ridge face. DONE in
  `64af169` (both native `direct_occt.rs::expand_thread_node` and
  `build123d_lowering.rs::_ecky_thread`); now proven by 1.1's test.
- [x] 1.3 (test) geometric-sanity verify catches a hollow result (a valid but
  wrong solid): expected-volume / core-present check, not only `IsValid`.
  Distinct from 1.1: this wants the app's own `verify_core_program`/authored
  `(verify)` pipeline to catch it as a diagnostic, not an external test
  harness. DONE 2026-07-31: authored `(thread core-volume-ratio radius length)`
  compares manifest volume with the expected cylindrical core. Focused test
  proves a topologically green, single-component sample still goes red when
  its volume is below 90% of the expected core.

## 2. Intent-derived profile (`:flank`, derivation)

- [x] 2.1 (test) `(thread … :flank 45deg :depth d :crest c)` derives
  `base = c + 2·d·tan(45°)` and renders; changing `:flank` changes `base`.
  Proven: build123d focused filter 6/6 green; native `thread_profile` filter
  2/2 green.
- [x] 2.2 Implement derivation in the op (build123d + native); explicit
  `:base-width`/`:crest-width` still override. Proven by the same focused tests.
- [x] 2.3 (test + diagnostic) `pitch ≤ base + clearance` emits a printability
  diagnostic (turns merge) without hard-failing. Proven: warning reaches existing
  build123d report and native manifest warning channels; focused tests green.

## 3. `tapped-hole` cutter (manifold by construction)

- [x] 3.1 (test) `(difference wall (tapped-hole :iso "M8" :length L))` yields a
  manifold body (`non-manifold-edge-count = 0`) with a through bore at minor and
  helical relief out to major. DONE 2026-07-30: runtime test
  `tapped_hole_in_wall_yields_connected_through_bore_and_relief_to_major` is
  green. The cutter reaches radial 4.00002 mm versus ISO M8 major 4.0 mm; wall
  volume is 5159.07 mm³, removed volume 240.9269 mm³ versus the 197.05 mm³
  bore floor, one connected component, and zero wall non-manifold edges. The
  reverse relief→bore binary-cut chain produces the same 240.9269 mm³ on both
  runner-first and forced fresh-emit paths. `expand_tapped_hole_node` emits
  relief before bore to preserve that measured removal.
- [x] 3.2 Implement `tapped-hole` = `union(bore@minor, female-relief)` with the
  relief radius inset below the bore (overlap) so no coincident face. DONE
  2026-07-30: backend parity closed across all three lowerers. Native
  `direct_occt.rs::expand_tapped_hole_node` (already green) + now
  build123d `_ecky_tapped_hole` (`bore = Cylinder(minor ...)` union
  `bore + relief`, named `overlap = min(0.3, minor * 0.5, depth)`, relief
  `radius = minor - overlap`, `depth + overlap`) and FreeCAD
  `_ecky_tapped_hole` (`bore = _ecky_cylinder(float(minor) ...)` union
  `bore.fuse(relief)`, same named overlap rule). Reuses `_ecky_helical_ridge`
  for the relief, no anonymous fit offsets, no new op/format. Proven by
  `lower_to_build123d_tapped_hole_emits_tapped_hole_helper`,
  `lower_to_build123d_thread_and_tapped_hole_of_equal_nominal_mate`,
  `freecad_lowering_emits_tapped_hole_helper` + native parity tests
  `plans_tapped_hole_as_union_of_bore_and_relief_for_direct_occt`,
  `expands_tapped_hole_into_bore_and_relief_ridge_union_for_direct_occt`,
  `thread_and_tapped_hole_of_equal_nominal_share_iso_minor_for_direct_occt`.
  3.1 (rendered `non-manifold-edge-count = 0`) and 3.3 (rendered bbox/fit
  engagement) are GREEN (see their evidence) — both executed at runtime
  2026-07-30. Mating DIMENSIONAL parity (equal
  nominal → shared ISO minor) was already green; 3.3 adds the literal bbox/fit
  engagement.
- [x] 3.3 (test) mating: an external `thread` and a `tapped-hole` of equal
  nominal with complementary clearance engage (bbox/fit check). DONE 2026-07-30:
  `equal_nominal_thread_and_tapped_hole_render_to_shared_major_bbox` in
  `direct_occt_executor.rs` renders `(thread :iso "M8" :length 14)` and
  `(tapped-hole :iso "M8" :length 14)` via the bundled runner-first harness and
  proves the literal bbox engagement (not just lowering parity): male radial
  bbox = 4.00000, female radial bbox = 4.00000, both = ISO M8 major (4.0),
  shared envelope Δ = 0.0 ≤ 0.1, male axial span 14.9375 ≥ length 14 (helix
  present end-to-end; the +0.9375 is the swept trapezoid base-width = pitch·0.75),
  both single solids, major (4.0) > minor (3.23325) so the bolt cannot pass the
  bore without its ridges riding the relief.

## 4. Asymmetric (buttress) profile — op enhancement

- [x] 4.1 `helical-ridge` accepts an asymmetric profile (independent
  upper/lower flanks, or an axial crest offset). DONE 2026-07-31: native and
  build123d ridge lowering accept paired `:lower-flank` / `:upper-flank` values
  and construct matching asymmetric profile vertices.
- [x] 4.2 `thread :profile 'buttress :load-flank … :return-flank …` maps to it.
  DONE 2026-07-31: both lowerers map load→lower and return→upper, with build123d
  degree surface values converted to radians exactly once.
- [x] 4.3 (test) buttress overhang flank ≤ 45° from vertical while the load flank
  stays steep; parity native ↔ build123d. DONE 2026-07-31: focused filters
  `buttress_profile`, `plans_buttress_thread`, and
  `lower_to_build123d_buttress_thread` are each 1/1 green; `cargo check` green.

## 5. Printability verify clauses

- [ ] 5.1 Reusable verify set for a printed thread: single-solid, manifold,
  overhang within budget, `pitch > base`. Author once, reuse.
- [ ] 5.2 (test) a thread with too-shallow flank (overhang > budget) goes red on
  the overhang clause; loosening `:flank` goes green.

## 6. Placement + boolean (reuse, document)

- [ ] 6.1 Confirm `place`/`location` positions a `thread`/`tapped-hole` on an
  arbitrary axis; document the "thread into a wall" pattern.
- [x] 6.2 Cone/tapered support (pipe/NPT): thread on a conical core. DONE
  2026-07-31: optional named `:top-radius` tapers both core and helical path;
  build123d emits `Cone` + conical `Edge.make_helix`, native emits `Cone` +
  `Geom_ConicalSurface`. Focused conical tests are green on both lowerers;
  existing cylindrical thread tests remain green.

## 7. Actualize consumers

- [x] 7.1 Migrate the helicoid (`Film scanning adapter - Ecky helicoid top
  cover`) to the intent primitive: replace the two hardcoded `crest = base*0.58`
  helical-ridges with `:flank`, expose the flank as a model param. DONE
  2026-07-31: both starts now use `thread` with the existing radii, pitch,
  lengths, depths, clips, translations, and 180-degree phase; `thread_flank`
  controls the profile while existing widths remain crest inputs. Canonical
  Core IR angle degrees now convert once at native geometry math, closing the
  first symbolic angle-param parity gap. Focused source/lowering, angle
  equivalence, native-build123d bbox/volume, two-part render + STEP, and
  `cargo check` proofs are green.
- [x] 7.2 Point `language-convenience-stdlib` fasteners (3.3) at `thread` +
  `tapped-hole`.

## Notes / gotchas captured this session

- Param retention: `macro_preview_render` keeps the target's current param
  VALUES; a new source's defaults do NOT override them. A coarse-pitch thread
  rendered with a retained fine pitch (1.25) makes `base > pitch` → turns merge →
  2 solids / non-manifold. Pass params explicitly or reset on a new design.
- A valid single solid is not proof of correctness — a hollow spiral passed
  `IsValid` + `single-solid`. Always also check volume / core presence.
- Native render goes runner-first; the runner needs the `:frenet` keyword
  (landed) and the cpp rebuilt into the runtime the app actually resolves.
