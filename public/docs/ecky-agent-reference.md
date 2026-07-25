# Ecky language reference

Current fileExtension: `.ecky`.
Current sourceLanguage: `ecky`.

Return one complete `(model ...)` program. Use millimetres for length and degrees for angles. Suffixed literals convert into those base units; they do not provide dimensional type checking.

## Program structure

- Put reusable pure `(define ...)` helpers and `define-component` declarations before `(model ...)`.
- Put `params`, `verify`, `part`, and `meta` clauses directly inside `model`.
- Never put `define` inside `model`. Use `let*` inside a part when later values depend on earlier values or parameters.
- Give every part a stable key. Use `build`, named `shape` stages, and one `result` when a part needs intermediate geometry.
- Keep `ui_spec`, `initial_params`, and source parameter keys aligned. Use `number`, `select`, `toggle`, or `image`; never invent parameter forms.

```scheme
(model
  (params
    (number width 60 :label "Width" :min 20 :max 120 :step 1)
    (number thickness 4 :label "Thickness" :min 1 :max 12 :step 0.5))
  (part plate
    (let* ((hole-r 3))
      (build
        (shape blank (extrude (rounded-rect width 36 4) thickness))
        (shape bore (translate 0 0 -0.5 (cylinder hole-r (+ thickness 1))))
        (result (difference blank bore))))))
```

## Geometry rules

- Start with primitives or closed 2D profiles. Use `extrude`, `revolve`, `sweep`, or `loft` to create solids.
- Boolean cutters must cross the target completely. Overshoot instead of leaving coincident faces.
- Author repeated shelves, ribs, holes, doors, or clips with `repeat`, array forms, or component instances. Do not copy shape blocks.
- Name every fit-critical dimension or relation: wall thickness, clearance, bore radius, pitch, seat height, and mating axis. Do not hide physical fit in anonymous offsets.
- Prefer selectors based on physical meaning or stable tags. Boolean operations rebuild topology, so raw face or edge indices are not stable design intent.
- Backend support is authoritative. If a diagnostic rejects an operation on the active backend, change the operation or backend; do not retry unchanged source.

## Verification

Write top-level `verify` clauses from measurable requirements. Keep them during repair.

```scheme
(model
  (verify
    (tag mesh-clean)
    (metric bad-edges (stl non-manifold-edge-count))
    (expect bad-edges (= 0)))
  (verify
    (tag preview-exists)
    (metric preview (manifest has-preview-stl))
    (expect preview (= true)))
  (part body (box 30 20 10)))
```

Use `manifest` metrics for artifact and part claims, `stl` metrics for mesh structure, `clearance` for physical gaps, `selector` for measured placement, and `relation` for comparisons between named targets. A failing clause means repair geometry or parameters; never weaken the requirement to manufacture green output.

## Mesh and image geometry

- Use `mesh` and `polyhedron` with bounded vertex and triangle lists. Prefer formula-generated lists over copied face blocks.
- `mesh` may be open. `polyhedron` must be one closed, orientable, nonzero-volume component. Check boundary edges, non-manifold edges, connected components, and volume before solid or printability claims.
- Use `heightfield` for calibrated luminance relief with explicit width, depth, relief height, base thickness, and inversion. Missing image selection is pending input, not substitute geometry.
- Front, Top, and Side raster references require physical calibration, contour review, editable sketch primitives, and normal exact-candidate validation. Raster extraction alone is not accepted CAD.
- A single perspective image provides inferred geometry only. Never call it an exact reconstruction, measured model, or accepted CAD without independent dimensions and validation.
- Pure mesh output supports mesh exports such as STL. STEP produced after solidifying a closed mesh is faceted poly-BRep, not analytic source CAD. Read artifact provenance before making export or exactness claims.

## Operating contract

Output source and required response fields only. Do not claim compilation, rendering, verification, STEP availability, or printability before runtime evidence exists. When the compiler returns a diagnostic, fix the named cause and emit a complete corrected program.
