# Ecky IR Field Guide

Learn Ecky by making parts. Each chapter changes visible geometry, explains why
that change works, and names the failure worth remembering. The later reference
exists for lookup; it is not the recommended reading order. This Markdown file
is canonical: web docs, EPUB/HTML, split rendering chapters, and the Ecky agent
language reference are projections of it.

Read the main lessons in order. The app sidebar exposes the same book one chapter at a time. Use **OPEN IN CODE** on any chapter to load its first runnable `.ecky` snippet into the code window. (You write parenthesized forms; they compile to a fixed set of operations, and the default render is an exact B-rep solid.)

## How Ecky Thinks

Start with the useful loop: write a small model, preview it, inspect the result,
then change one source relation. Ecky keeps those steps deterministic. When an
error appears, three boundaries explain where it came from:

**Surface language.** You write parenthesized `.ecky` forms such as `(model (part ...))`. This syntax describes authoring intent; the renderer does not execute it directly.

**Core IR.** The compiler lowers surface forms into a fixed vocabulary of primitives, booleans, selectors, placements, repeats, and typed mesh operations. The kernel receives this finite data model, not arbitrary Scheme. That boundary makes models reproducible and statically checkable.

**Geometry runtime.** Exact solids render on the native **OCCT** B-rep kernel. Typed polygon data renders in the bounded Rust mesh runtime; a closed mesh crosses into OCCT only through the explicit faceted poly-BRep bridge. **build123d** and **FreeCAD** are supported interop backends with smaller operation sets.

Classify a failure before changing geometry: surface syntax, Core IR validation, or backend support. Diagnostics name that boundary whenever possible.


## First Solid: Ball on a Base

A renderable file needs a `model`, a named `part`, and geometry. We are making
a marker: first its ball, then the base. Starting with one primitive keeps each
added transform or boolean obvious.

```scheme
(model
  (part marker
    (sphere 10)))
```

![Rendered output for First Solid: Ball on a Base, example 1](assets/01-first-solid-01.png)

`model` is the root. `part` gives the geometry a stable id. `sphere` produces the solid.

Add another primitive with `union` when two solids should become one part.

```scheme
(model
  (part marker
    (union
      (box 28 28 4)
      (translate 0 0 10
        (sphere 10)))))
```

![Rendered output for First Solid: Ball on a Base, example 2](assets/01-first-solid-02.png)

`box` makes the base. `translate` moves the ball up so it sits on the base instead of overlapping the center.

Use this pattern for first tests: primitive first, then one transform, then one boolean.

> **Watch for:** primitives start at the origin. Two untransformed solids overlap instead of stacking. If a union has the right members but the wrong silhouette, inspect placement before changing the boolean.

## Sketch to Solid: Plate from a Profile

Many parts begin as a closed 2D region. `extrude` turns that region into a solid with a specified height.

```scheme
(model
  (part plate
    (extrude
      (rounded-rect 70 42 5)
      4)))
```

![Rendered output for Sketch to Solid: Plate from a Profile, example 1](assets/02-sketch-extrude-01.png)

`rounded-rect` is the closed 2D profile. `extrude` gives it thickness.

Use `profile` when the shape has holes.

```scheme
(model
  (part washer_plate
    (extrude
      (profile
        :outer (rounded-rect 70 42 5)
        :holes (circle 9 64))
      4)))
```

![Rendered output for Sketch to Solid: Plate from a Profile, example 2](assets/02-sketch-extrude-02.png)

The outer profile defines material. The hole profile removes material during the extrusion.

The modeling sequence stays explicit: define a region, then choose how it becomes three-dimensional.

Use `offset` to grow or shrink a 2D outline by a fixed distance before extruding. A positive distance pushes the outline outward.

```scheme
(model
  (part gasket
    (extrude
      (profile
        :outer (offset 3 (rounded-rect 30 18 4))
        :holes (rounded-rect 30 18 4))
      4)))
```

![Rendered output for Sketch to Solid: Plate from a Profile, example 3](assets/02-sketch-extrude-03.png)

`offset 3` creates the outer boundary; the original outline becomes the hole. The resulting gasket has a uniform 3 mm wall.

`scale` stretches a profile by separate x, y, z factors. Scale a circle in one axis and it becomes an ellipse, so you reach for `scale` instead of a separate ellipse primitive.

```scheme
(model
  (part oval_plate
    (extrude (scale 1.6 1 1 (circle 10 48)) 5)))
```

![Rendered output for Sketch to Solid: Plate from a Profile, example 4](assets/02-sketch-extrude-04.png)

> **Watch for:** `extrude` only works on a _closed_ region. An open polyline or a profile whose `:holes` poke through the `:outer` edge has no well-defined inside, and the extrude fails or produces junk. Keep holes strictly inside the outer boundary, and reach for `profile` (not a raw shape) the moment material needs to be removed — the `:outer`/`:holes` split is what tells Ecky which side is solid.

## Convenience Shapes: Stop Hand-Building Common Outlines

Use named convenience shapes when they express the design directly. They avoid repeated outline math and preserve analytic geometry where the backend supports it.

A **torus** is a ring: major radius to the tube centre, minor radius of the tube.

```scheme
(model
  (part ring
    (torus 20 5)))
```

![Rendered output for Convenience Shapes: Stop Hand-Building Common Outlines, example 1](assets/02a-convenience-shapes-01.png)

An **ellipse** is a 2D profile — give it the x and y radii, then `extrude` it like any sketch. When the y radius is larger, the long axis simply swings to y; you do not rotate anything yourself.

```scheme
(model
  (part oval
    (extrude (ellipse 18 10) 4)))
```

![Rendered output for Convenience Shapes: Stop Hand-Building Common Outlines, example 2](assets/02a-convenience-shapes-02.png)

A **regular-polygon** takes a side count and a circumradius (optionally `:rotation`).

```scheme
(model
  (part hex
    (extrude (regular-polygon 6 12) 5)))
```

![Rendered output for Convenience Shapes: Stop Hand-Building Common Outlines, example 3](assets/02a-convenience-shapes-03.png)

A **trapezoid** takes the bottom width, top width, and height; add `:skew` to slide the top sideways.

```scheme
(model
  (part wedge_plate
    (extrude (trapezoid 40 24 18 :skew 4) 5)))
```

![Rendered output for Convenience Shapes: Stop Hand-Building Common Outlines, example 4](assets/02a-convenience-shapes-04.png)

A **wedge** is the 3D ramp: a `dx × dy × dz` box whose top face shrinks to the rectangle `xmin..xmax` by `zmin..zmax`.

```scheme
(model
  (part ramp
    (wedge 40 20 30 10 5 30 25)))
```

![Rendered output for Convenience Shapes: Stop Hand-Building Common Outlines, example 5](assets/02a-convenience-shapes-05.png)

### Slots

A slot is an obround — a rectangle capped by two semicircles. Four front-ends describe the same shape from whatever you happen to know.

`slot-overall` takes the tip-to-tip length and the width.

```scheme
(model
  (part track
    (extrude (slot-overall 50 12) 4)))
```

![Rendered output for Convenience Shapes: Stop Hand-Building Common Outlines, example 6](assets/02a-convenience-shapes-06.png)

`slot-center-to-center` takes the distance between the two end-arc centres and the width.

```scheme
(model
  (part track_c2c
    (extrude (slot-center-to-center 38 12) 4)))
```

![Rendered output for Convenience Shapes: Stop Hand-Building Common Outlines, example 7](assets/02a-convenience-shapes-07.png)

`slot-center-point` takes the slot centre `(cx cy)`, the centre of one end arc `(px py)`, and the width — handy when you already know where the holes go. It orients itself along the line between the two points.

```scheme
(model
  (part track_cp
    (extrude (slot-center-point 0 0 30 0 12) 4)))
```

![Rendered output for Convenience Shapes: Stop Hand-Building Common Outlines, example 8](assets/02a-convenience-shapes-08.png)

`slot-arc` curves the slot along a circular arc: centreline radius, start and end angle (degrees), and width.

```scheme
(model
  (part curved_track
    (extrude (slot-arc 30 0 120 10) 4)))
```

![Rendered output for Convenience Shapes: Stop Hand-Building Common Outlines, example 9](assets/02a-convenience-shapes-09.png)

> **Watch for:** the slot, ellipse, regular-polygon, and trapezoid examples here are 2D profiles — they need an `extrude` (or `revolve`) to become a solid. `torus` and `wedge` are already solids, so they stand alone.

### Threads

`thread` builds a screw thread by sweeping a ridge along a helix around a core cylinder — you do not hand-build the helix. Give it a radius, pitch, length, and depth.

```scheme
(model
  (part screw
    (thread :radius 6 :pitch 1.5 :length 18 :depth 0.9)))
```

![Rendered output for Convenience Shapes: Stop Hand-Building Common Outlines, example 10](assets/02a-convenience-shapes-10.png)

For standard hardware, `:iso "M…"` decodes an ISO metric coarse-pitch designation into the radius, pitch, and depth for you — pass only the length.

```scheme
(model
  (part bolt
    (thread :iso "M8" :length 20)))
```

![Rendered output for Convenience Shapes: Stop Hand-Building Common Outlines, example 11](assets/02a-convenience-shapes-11.png)

`:female #t` makes the matching cutter instead of a solid screw. Subtract it from a bore to tap a hole; `:clearance` widens the envelope so the parts actually mate.

```scheme
(model
  (part nut
    (difference
      (cylinder 10 8)
      (thread :iso "M8" :length 8 :female #t :clearance 0.2))))
```

![Rendered output for Convenience Shapes: Stop Hand-Building Common Outlines, example 12](assets/02a-convenience-shapes-12.png)

`:lefthand #t` reverses the helix. Unknown ISO designations (e.g. `"M7"`) fail with a clear error rather than guessing.

## Parameters: Make the Plate Editable

Parameters separate design inputs from derived geometry. Declare editable dimensions once under `params`; the UI reads their metadata and the model reads their keys.

```scheme
(model
  (params
    (number plate_w 70 :label "Plate width" :min 40 :max 120 :step 1)
    (number plate_h 42 :label "Plate height" :min 20 :max 80 :step 1)
    (number corner_r 5 :label "Corner radius" :min 0 :max 12 :step 0.5)
    (number thickness 4 :label "Thickness" :min 1 :max 12 :step 0.5))
  (part plate
    (extrude
      (rounded-rect plate_w plate_h corner_r)
      thickness)))
```

![Rendered output for Parameters: Make the Plate Editable, example 1](assets/03-parameters-01.png)

The geometry reads the parameter names directly. The UI reads labels, min/max, and step from the declarations.

Keep parameters physical: widths, heights, clearances, radii. Put derived math near the geometry.

```scheme
(shape hole_r (/ bore_d 2))
```

That line is better than repeating `(/ bore_d 2)` through cuts and selectors.

### Units: bare numbers already have one

Ecky uses **millimeters for length** and **degrees for angles**. Bare numbers already use those base units: `(box 70 42 4)` is 70 × 42 × 4 mm, while `(rotate 90 0 0 ...)` rotates 90 degrees.

When you do write one, the suffix is a **conversion into that base unit** — nothing more:

| Suffix | Family | Becomes |
| --- | --- | --- |
| `mm` | length | itself (`12mm` → `12`) |
| `cm` | length | ×10 (`1cm` → `10`) |
| `in` | length | ×25.4 (`1in` → `25.4`) |
| `deg` | angle | itself (`90deg` → `90`) |
| `rad` | angle | ×(180/π) (`1.5708rad` → `90`) |

So `(box 12mm 1cm 1in)` is exactly `(box 12 10 25.4)`, and `(rotate 1.5708rad 0 0 ...)` is the same 90-degree turn as `(rotate 90 0 0 ...)`. Suffixes exist so you can author in the unit a spec is written in and let Ecky normalize.

**Some numbers stay unitless on purpose.** Counts (`(repeat 5 ...)`), ratios, segment counts on a cylinder (`(cylinder 6 12 96)` — that `96` is facets, not millimeters), and indices are pure numbers. A suffix on them is meaningless; leave them bare.

**Unit suffixes convert values; they do not type-check dimensions.** `45deg` in a width slot becomes the number `45`, then the box reads it as 45 mm. Use length suffixes for lengths, angle suffixes for angles, and bare values for counts and ratios.

## Cut and Join: Mounting Plate

Use `build` when a part needs several boolean stages. Each `shape` names an intermediate result; `result` identifies the final geometry. Names keep cutters and later selectors readable.

```scheme
(model
  (params
    (number plate_w 80)
    (number plate_h 48)
    (number thickness 5)
    (number hole_r 4))
  (part mount
    (build
      (shape blank
        (extrude (rounded-rect plate_w plate_h 4) thickness))
      (shape hole_left
        (translate -24 0 -0.5
          (cylinder hole_r (+ thickness 1))))
      (shape hole_right
        (translate 24 0 -0.5
          (cylinder hole_r (+ thickness 1))))
      (result
        (difference blank hole_left hole_right)))))
```

![Rendered output for Cut and Join: Mounting Plate, example 1](assets/04-cut-and-join-01.png)

`build` names each step. `difference` subtracts cutters from the blank. The cutters are slightly taller than the plate so the cut passes fully through.

Add material with `union` or `fuse`.

```scheme
(result
  (union
    (difference blank hole_left hole_right)
    (translate 0 0 thickness
      (cylinder 12 8))))
```

The result is still one part, but the intent stays readable.

> **Watch for:** make cutters cross the stock completely; coincident cutter and stock faces can leave unstable slivers. Booleans also rebuild topology, so raw face and edge indices are not durable selectors. Use geometric selectors or tags after the boolean.

## Round, Chamfer, Shell: Select Edges and Faces

Finishing operations need two things: a radius or thickness, and a stable way to identify target topology. This chapter pairs `fillet`, `chamfer`, and `shell` with geometric selectors, tags, and native provenance selectors.

Edge operations happen after the main solid exists.

```scheme
(model
  (part soft_block
    (fillet 2
      :edges "top"
      (box 60 36 16))))
```

![Rendered output for Round, Chamfer, Shell: Select Edges and Faces, example 1](assets/05-round-shell-select-01.png)

`:edges "top"` selects top boundary edges. Use `chamfer` when the edge should become flat instead of rounded.

```scheme
(model
  (part beveled_block
    (chamfer 1.5
      :edges "bottom"
      (box 60 36 16))))
```

![Rendered output for Round, Chamfer, Shell: Select Edges and Faces, example 2](assets/05-round-shell-select-02.png)

Use `shell` to hollow a solid by removing selected faces.

```scheme
(model
  (part open_tray
    (shell 2
      :faces "top"
      (box 70 44 22))))
```

![Rendered output for Round, Chamfer, Shell: Select Edges and Faces, example 3](assets/05-round-shell-select-03.png)

Selectors should describe a physical feature: top, bottom, planar normal, or a stable target id. Avoid anonymous offsets for fit-critical faces.

Tag any fit-critical selector. The tag records intended topology in the manifest, so param changes can rebind the same seat, lip, or opening instead of chasing backend face indexes.

```scheme
(model
  (tag-face tray_opening :faces "top" tray)
  (part tray
    (shell 2
      :faces (tag tray_opening)
      (box 70 44 22))))
```

![Rendered output for Round, Chamfer, Shell: Select Edges and Faces, example 4](assets/05-round-shell-select-04.png)

When a `build` introduces helper solids, use `:created-by <shape>` to keep clause selectors scoped to topology from that intermediate shape only.

```scheme
(model
  (part body
    (build
      (shape blank (box 70 44 22))
      (shape pocket (translate 0 0 10 (box 30 18 12)))
      (shape tray (difference blank pocket))
      (result
        (shell 2
          :faces "planar+normal-z+area-max"
          :created-by pocket
          tray)))))
```

Here `:created-by pocket` limits face candidates to the cavity created from `pocket`, not every planar top-facing face on `tray`.

> **Native-only.** `:created-by` is a provenance selector: it relies on the
> originating-slot index that the native OCCT kernel tracks for every face and
> edge. It resolves only on the native backend (Ecky's default). The build123d
> and FreeCAD interop backends have no slot-provenance index, so they reject
> `:created-by` rather than guess. If you lower a model through an interop
> backend (including `ecky check`, which uses build123d today), drop the
> `:created-by` clause and lean on the geometric predicates (`planar`,
> `normal-z`, `area-max`) or a `tag-face` instead.

### Tapered fillets

A normal `fillet` uses one radius. Add `:to-radius` and the radius varies along each selected edge — it starts at the base radius and eases to the second one. Handy for blends that need to grow or shrink along a run.

```scheme
(model
  (part p
    (fillet 4 :to-radius 1 :edges "top" (box 40 40 20))))
```

> **Backend note:** tapered fillets are an OCCT capability rendered by the native and FreeCAD backends. The build123d backend only does single-radius fillets, so it rejects `:to-radius` with a clear error rather than silently giving you a uniform fillet — render tapered fillets on native or FreeCAD.

### Draft

`draft` tilts the side walls of a solid by an angle so a molded part can release from its tool. It tapers every vertical face about a neutral plane (the level that stays the original size); pass `:neutral-z` to move that plane, otherwise it sits at `z = 0`.

```scheme
(model
  (part p
    (draft 8 (box 30 30 20))))
```

![Rendered output for Round, Chamfer, Shell: Select Edges and Faces, example 7](assets/05-round-shell-select-07.png)

> **Backend note:** draft is rendered by the native and build123d backends (both OpenCASCADE). The FreeCAD backend has no Part draft API, so it rejects `draft` with a clear error. This first cut drafts *all* vertical faces; targeting specific faces with a `:faces` selector is a planned extension.

## Paths and Surfaces: Revolve and Sweep

Choose a surface operation from the motion of a profile: rotate it around an axis, carry it along a path, or interpolate between several sections.

Use `revolve` when a 2D profile turns around an axis.

```scheme
(model
  (part knob
    (revolve
      (make-face
        (path
          (12 0 0)
          (18 0 0)
          (18 18 0)
          (10 24 0)
          (12 0 0)))
      360)))
```

![Rendered output for Paths and Surfaces: Revolve and Sweep, example 1](assets/06-paths-and-surfaces-01.png)

`path` creates the outline. `make-face` turns the closed outline into a face. `revolve` spins it into a solid.

Use `sweep` when a profile follows a path.

```scheme
(model
  (part handle
    (sweep
      (circle 2.2 32)
      (bezier-path
        ((-24 0 0) (-10 18 6) (10 18 6) (24 0 0))))))
```

![Rendered output for Paths and Surfaces: Revolve and Sweep, example 2](assets/06-paths-and-surfaces-02.png)

The circle is the cross-section. The bezier path is the centerline. Sweep keeps those responsibilities separate.

Use `loft` when one profile needs to become another profile across height or distance.

```scheme
(model
  (part nozzle
    (loft 24
      (circle 14 32)
      (circle 5 32))))
```

![Rendered output for Paths and Surfaces: Revolve and Sweep, example 3](assets/06-paths-and-surfaces-03.png)

The first profile is the base, the last is the cap, and `loft` skins a smooth wall between them. The leading number is the total height; profiles stack evenly along it, so the wide circle sits at the bottom and the narrow one at the top.

### Ribs and grooves

`rib` and `groove` are the two-step "sweep a profile, then combine" move rolled into one op. Both take a solid, a profile, and a path: `rib` sweeps the profile along the path and fuses the result onto the solid (a reinforcing rib); `groove` sweeps it and cuts it away (a channel).

```scheme
(model
  (part p
    (rib
      (box 20 20 20)
      (circle 3)
      (path (0 0 0) (0 0 30)))))
```

![Rendered output for Paths and Surfaces: Revolve and Sweep, example 4](assets/06-paths-and-surfaces-04.png)

Swap `rib` for `groove` to subtract the same swept run instead of adding it. They lower to `sweep` + `union`/`difference`, so they render on every backend.

## Repetition: Ribs, Slots, and Patterns

Represent repeated geometry with one body and an index. This keeps count, spacing, and fit math editable in one place.

```scheme
(model
  (part ribbed_plate
    (build
      (shape base
        (box 90 40 4))
      (shape ribs
        (repeat-union i 5
          (translate (- (* i 18) 36) 0 5
            (box 4 34 6))))
      (result
        (union base ribs)))))
```

![Rendered output for Repetition: Ribs, Slots, and Patterns, example 1](assets/07-repetition-01.png)

`repeat-union` makes one merged body from repeated solids. The index `i` is local to the repeat body.

When repeated features share the same fit math, hoist derived values once instead of repeating arithmetic at every call site. Use model-level `let*` for dependent dimensions, a helper `define` for placement math, and `define-component` when one repeated body needs the same closed geometry everywhere.

```scheme
(define (divider-depth tray_d wall)
  (- tray_d (* 2 wall)))

(define-component divider
  ((number height 12) (number depth 34))
  (box 4 depth height))

(model
  (let* ((tray_d 40)
         (wall 3)
         (pitch 18)
         (slot_w 6)
         (rib_h 12)
         (divider_d (divider-depth tray_d wall)))
    (part tray
      (difference
        (union
          (box 80 tray_d 18)
          (repeat-union i 4
            (translate (- (* i pitch) 27) 0 9
              (divider :height rib_h :depth divider_d))))
        (repeat-union i 4
          (translate (- (* i pitch) 27) 0 0
            (box slot_w 30 20)))))))
```

Here `pitch`, `slot_w`, and `wall` each have one definition. `divider-depth` owns the offset calculation, while `divider` owns the repeated body. Lift shared math or geometry as soon as a second call site appears.

Use `repeat-compound` when repeated items should stay grouped instead of merged.

```scheme
(shape rollers
  (repeat-compound i 4
    (translate (- (* i 16) 24) 0 8
      (cylinder 3 8))))
```

Use `repeat-pick` when only some indices should produce geometry.

```scheme
(shape end_stop
  (repeat-pick i 5 (= i 4)
    (translate 36 0 12
      (sphere 4))))
```

### Common mistake: `(define ...)` inside `(model ...)`

`(define ...)` is only valid at the **top level** (outside `(model ...)`), where it
defines reusable helper functions like `divider-depth` above. Inside `(model ...)`,
Steel evaluates `define` eagerly — before params have values — so any arithmetic
on a param produces a misleading `TypeMismatch` error instead of a clear message.

**Wrong** — define inside model:
```scheme
(model
  (params (number frame_length 160))
  (define half_len (/ frame_length 2))   ; ← TypeMismatch at runtime
  (part body (box half_len 10 10)))
```

**Right** — `let*` inside the part:
```scheme
(model
  (params (number frame_length 160))
  (part body
    (let* ((half_len (/ frame_length 2)))
      (box half_len 10 10))))
```

The rule is simple: **`define` for top-level helper functions, `let*` for computed
values inside parts.** If a derived value needs to reference a param, it belongs
in a `let*` binding scoped to the part (or a `let*` wrapping model clauses that
spans multiple parts).

## Components and Reuse: Lift a Proven Part

Use a component when geometry must be reused across parts or models. A component packages a closed parameter signature, its geometry, and verification clauses; each instance reuses that definition without copying source.

This component defines a bored mounting standoff and carries its minimum-wall check:

```scheme
(define-component standoff
  ((number height 12 :label "Standoff height" :min 6 :max 30)
   (number bore 3.2))
  (verify (tag bore_open) (metric min_wall_thickness "body") (expect (>= value 1.2)))
  (difference
    (cylinder 6 height 96)
    (cylinder bore (+ height 2) 96)))

(model
  (part front_left (standoff :height 16))
  (part rear_right (translate 40 0 0 (standoff))))
```

Three rules define component behavior.

**Reuse by reference, override by keyword.** `(standoff :height 16)` instantiates the component and overrides one signature key; `(standoff)` takes every default. Omitted keys fall back to the signature, and a missing _required_ key (one with no default) is a compile error that names the component and lists its signature. There is no copy-paste, so there is no drift: change the body once and both parts move together.

**Closedness makes reuse reliable.** A component body sees only signature keys and local bindings (`let`, `let*`, repeat indices, and build shapes). Referencing a model parameter or outer binding is a compile error. Therefore the component can be copied into another model without hidden dependencies.

**Verification expands per instance.** The component's `verify` clause is namespaced by part key, producing tags such as `front_left/bore_open` and `rear_right/bore_open`. Every instance runs the same wall-thickness requirement.

For the exact signature grammar, nesting limits, and verify-travel rules, see **`define-component`** in the language reference appendix.

### The library loop (MCP)

Components do not have to live in one file. Agents lift proven parts into a shared library and pull them back by source:

1. `component_extract` — hand it a model and a `partKey`. Referenced model params become the signature (metadata preserved); scalar outer bindings become plain defaults; any non-scalar free reference is reported as a blocker so you cannot extract something that secretly depends on its context. `save: true` stores it.
2. `component_search` — compact headers only (name, one-liner, param keys, tags). Bodies never come back from search, so the library stays browsable.
3. `component_get` — the full, self-contained `define-component` source for one name. Paste it into the model and instantiate.

The loop is copy-inline by design: what you get back is closed source, not a hidden registry link. A part proven in one project becomes a building block in the next, checks and all.

## Placement and Frames: Put Geometry Where It Belongs

Use direct transforms for fixed world-axis placement. Use a named frame when several shapes share a local coordinate system or must follow a path.

```scheme
(translate 20 0 0 (box 10 10 10))
(rotate 0 0 45 (box 10 10 10))
(mirror 'x 0 (box 10 10 10))
```

Use frames when placement should be named and reused.

```scheme
(model
  (part angled_pin
    (build
      (shape pin_pose
        (plane
          :origin (20 0 4)
          :normal (0 1 1)
          :x (1 0 0)))
      (shape pin
        (cylinder 3 24))
      (result
        (place pin_pose pin)))))
```

![Rendered output for Placement and Frames: Put Geometry Where It Belongs, example 1](assets/08-placement-and-frames-01.png)

`plane` describes a local coordinate system. `place` moves geometry into it.

For path-driven models, `path-frame` can sample a location and tangent along a path. Use it when attachments must follow a curve instead of a fixed world axis.

## Verification: State What Must Stay True

`verify` stores measurable requirements with the model. Write the requirement before tuning geometry, run verification, and keep the clause unchanged while repairing a failed result.

Start with the invariant, not the fix. This model says the lid must keep at least `0.3` mm clearance above the body:

```scheme
(model
  (verify
    (tag lid_clearance body.lid_gap)
    (metric gap (clearance min-distance body lid))
    (expect gap (>= 0.3)))
  (part body (box 80 50 20))
  (part lid
    (translate 0 0 20.4
      (box 78 48 3))))
```

![Rendered output for Verification: State What Must Stay True, example 1](assets/09-verification-01.png)

`tag` names the concern. `metric` measures it. `expect` sets the condition.

### Red to green: lid clearance

Red state: the required clearance is `0.3` mm, but the lid sits only `0.2` mm above the body. Verification reports the measured delta.

```text
(model
  (verify
    (tag lid_clearance body.lid_gap)
    (metric gap (clearance min-distance body lid))
    (expect gap (>= 0.3)))
  (part body (box 80 50 20))
  (part lid
    (translate 0 0 20.2
      (box 78 48 3))))
```

Green state: keep the same `verify` block and move the lid to `20.4`. Re-render and run verification again. Geometry changes; the requirement does not.

```text
(part lid
  (translate 0 0 20.4
    (box 78 48 3)))
```

Worked red-to-green loop:

1. Write one `verify` clause from one physical requirement.
2. Run `verify_generated_model` and confirm the failure names the violated promise.
3. Change geometry, parameters, or named constraints. Do not weaken the requirement to get green.
4. Fix the model and re-render.
5. Run `verify_generated_model` again until the original clause passes.

Use verification for:

- minimum clearances
- expected part count
- STL triangle or component checks
- required STEP or preview artifacts

Do not delete a failing verification clause to make a render pass. Fix the model or the stated requirement.

## Real Model Patterns: Procedural Cuts and Arrayed Frames

These fixtures combine generated cutter lists, deterministic fields, path frames, arrays, and parameter-driven cavities. Focus on how each model separates generation math from final boolean intent.

### Procedural perforated panel

This model uses `map` and `range` to generate cutters, `hash-signed` to jitter each cutter, `voronoi2` to vary cutter radius, and `apply union` to turn the generated list into one cutter body.

<!-- render-source: ../examples/voronoi-perforated-panel.ecky -->

![Rendered output for Real Model Patterns: Procedural Cuts and Arrayed Frames, example 1](assets/10-real-model-patterns-01.png)

The important line is the result expression:

```scheme
(result
  (difference
    panel
    (apply union
      (map
        (lambda (cell)
          (let* ((col (- cell (* 4 (floor (/ cell 4)))))
                 (row (floor (/ cell 4)))
                 (x (* (- col 1.5) 14))
                 (y (* (- row 1.0) 12))
                 (jx (+ x (* 2.4 (hash-signed col row 23))))
                 (jy (+ y (* 2.4 (hash-signed (+ col 19.19) (+ row 7.73) 54))))
                 (r (+ 2.2 (* 1.1 (voronoi2 (/ jx 14.0) (/ jy 12.0) 23)))))
            (translate jx jy 0
              (cylinder r 8 24))))
        (range 0 cell-count)))))
```

`range` decides how many cutters exist. `map` builds one cylinder per cell. `let*` is required because `jx`, `jy`, and `r` depend on earlier bindings. `apply union` converts the list of cylinders into one boolean operand for `difference`.

This is the pattern to use when the count is parametric but the result is still one printable part.

### Frame and array bracket

This fixture combines curve-driven placement with arrays. The rib is swept along a bezier path. The pad is placed at a sampled path frame. The base holes, locator posts, and fan stops use three array helpers.

<!-- render-source: ../examples/frame-array-bracket.ecky -->

![Rendered output for Real Model Patterns: Procedural Cuts and Arrayed Frames, example 2](assets/10-real-model-patterns-02.png)

The model has three distinct placement styles:

```scheme
(shape rail
  (bezier-path ((-18 0 4) (-8 7 9) (8 -7 12) (18 0 16))))
(shape rib
  (sweep (circle 1.1) rail))
(shape end-frame
  (path-frame rail :at end :up (0 0 1)))
(shape placed-pad
  (place end-frame pad :offset (0 0 -1.5) :rotate (0 0 18)))
```

`sweep` makes geometry follow the path. `path-frame` samples a pose from the path. `place` uses that pose to attach another solid.

The array helpers do the repeated work:

```scheme
(linear-array 3 14 0 0
  (translate -14 0 -2 (cylinder 2.1 10)))

(grid-array 2 3 16 10
  (translate -16 -5 4 (cylinder 1.2 8)))

(radial-array 6 60 11
  (translate 0 0 4 (cone 1.8 0.8 5)))
```

Use these when the pattern is regular. Use `map` and `range` when each instance needs custom math.

### Woodlouse hotel

This habitat uses one generated entrance list plus repeated shelves and dividers. Shared dimensions keep openings aligned when chamber count or overall width changes.

<!-- render-source: ../examples/woodlouse-hotel.ecky -->

![Rendered output for Real Model Patterns: Procedural Cuts and Arrayed Frames, example 3](assets/10-real-model-patterns-03.png)

The entrances are generated from one parametric chamber count:

```scheme
(shape entrances
  (apply union
    (map
      (lambda (cell)
        (let* ((col (- cell (* chamber_cols (floor (/ cell chamber_cols)))))
               (row (floor (/ cell chamber_cols)))
               (x (+ (* -0.5 hotel_w) wall (* (+ col 0.5) col_gap)))
               (z (+ wall (* (+ row 0.55) floor_gap))))
          (translate x (* -0.5 hotel_d) z
            (rotate 90 0 0
              (cylinder entrance_r (+ hotel_d 6) 24)))))
      (range 0 (* chamber_cols 3)))))
```

`chamber_cols` drives both cutter count and divider spacing. `col_gap` is derived from `hotel_w` and `chamber_cols`, so openings stay centered when the model is resized.

## Worked Project: Perforated Toothbrush Holder

Small examples teach syntax. This project teaches control: preserve one useful
body while shelling it, closing it, proving a custom cutter, then generating
dozens of cutters without turning the source into copied geometry.

Every stage is a complete `.ecky` file. Run a checkpoint before reading the
next one with `src-tauri/target/debug/ecky check <checkpoint.ecky>`.

The manifest at
`docs/books/ecky-ir/examples/toothbrush-holder/manifest.json` keeps the stages
ordered and machine-checkable.

### Stage 1: subtract one named cavity

The outside is a union of a centered cylinder and a rear box. The cavity uses
the same construction with dimensions derived from `wall_thickness`.

```scheme
(model
  (params
    (number width 36.0 :min 20.0 :max 60.0 :label "Width")
    (number depth 25.0 :min 15.0 :max 50.0 :label "Depth")
    (number wall_thickness 2.6 :min 1.5 :max 5.0 :label "Wall thickness")
    (number height 85.0 :min 40.0 :max 150.0 :label "Height"))
  (part toothbrush_holder
    (let* ((radius (/ width 2.0))
           (inner_width (- width (* 2.0 wall_thickness)))
           (inner_depth (- depth wall_thickness))
           (inner_radius (- radius wall_thickness))
           (outer
             (union
               (box width depth height :align '(center max center))
               (cylinder radius height :align '(center center center))))
           (cavity
             (union
               (box inner_width inner_depth (+ height 10.0)
                 :align '(center max center))
               (cylinder inner_radius (+ height 10.0)
                 :align '(center center center)))))
      (difference outer cavity))))
```

Canonical checkpoint:
`docs/books/ecky-ir/examples/toothbrush-holder/01-shell.ecky`.

The cavity is taller than the body. That deliberate overrun avoids coincident
top and bottom faces. `wall_thickness` owns every fit-critical offset; there is
no second anonymous wall value waiting to drift.

### Stage 2: close the base, then reopen a drain

Stage 1 is a tube. Stage 2 adds a bottom blank at `(- (/ height 2.0))`, cuts a
drain through it, then unions the drained bottom with the shell.

```text
(bottom_blank
  (translate 0 0 (- (/ height 2.0))
    (union
      (box width depth wall_thickness :align '(center max min))
      (cylinder radius wall_thickness :align '(center center min)))))
(drain
  (translate 0 0 (- (/ height 2.0) 1.0)
    (cylinder drain_radius (+ wall_thickness 2.0)
      :align '(center center min))))
(bottom (difference bottom_blank drain))
```

Canonical checkpoint:
`docs/books/ecky-ir/examples/toothbrush-holder/02-drained-base.ecky`.

The drain begins 1 mm below the bottom and ends 1 mm above it. Cutters should
cross material completely; exact tangency is not a useful manufacturing
constraint.

### Stage 3: prove one custom cutter

The wall pattern starts as a closed `polygon`, becomes a solid with `extrude`,
then receives a frame with `location` and `place`. Only one cutter is
subtracted.

```text
(spade_profile
  (polygon
    ((0.0 4.7) (-3.05 1.5) (-3.25 -0.8) (-2.15 -1.95)
     (-0.55 -3.35) (-1.15 -5.35) (1.15 -5.35)
     (0.55 -3.35) (2.15 -1.95) (3.25 -0.8) (3.05 1.5))))
(spade
  (place
    (location (plane :origin '(0.0 0.0 0.0))
      :rotate '(-90.0 0.0 180.0))
    (extrude spade_profile cutter_depth)))
(front_cutter (translate 0.0 front_cutter_y 0.0 spade))
```

Canonical checkpoint:
`docs/books/ecky-ir/examples/toothbrush-holder/03-single-cutter.ecky`.

Do not start with the full pattern. A wrong profile, axis, or cutter depth
multiplied fifty times only makes the failure slower to understand.

### Stage 4: generate cutters, subtract once

The production model uses nested `repeat-union` forms for staggered front,
left, and right rows. Row and column indices feed local `let*` bindings for
angle, side offset, and height. Those groups become `spade_holes`.

The final intent stays small:

```text
(spade_holes (union front_spades left_spades right_spades))
(final (difference (union shell bottom) spade_holes))
```

The planner flattens the generated tool union into one n-ary `difference`, so
independent cutters can be prepared in parallel before OCCT performs the
boolean. Source still describes one semantic cut; runtime chooses the execution
schedule.

Canonical complete model:
`src-tauri/tests/fixtures/cad/perf/toothbrush_holder_versions.ecky`.

This is the reusable project shape:

**1. Expose physical controls.** Parameters describe user decisions.

**2. Derive dimensions once.** Put dependent math in `let*`.

**3. Name body and cutter stages.** Final expressions should read as intent.

**4. Prove one repeated unit.** Debug one profile, axis, and overrun.

**5. Generate the remaining units.** Use `repeat` or arrays.

**6. Keep the final boolean boring.** One body, one cutter group, one cut.

**Watch for:** a repeated cutter that only touches a wall may disappear or
split topology unpredictably. Give cutter depth explicit overrun and keep safe
top/bottom margins as named bindings.

## Projects as Folders: Edit Anywhere, Stay Canonical

A project folder mirrors one thread's active source onto disk. Edit `model.ecky` with any file-based tool; Ecky validates the changed file and records accepted updates as new thread versions. The thread remains canonical history.

`project_folder_export` writes two files:

```text
<projectsRoot>/<slug>/
  model.ecky          edit this with anything
  ecky-project.json   binding manifest, owned by Ecky — never edit by hand
```

Edit `model.ecky` in any editor. A polling watcher detects a digest change, compiles the source, renders a preview, and commits a `folder-sync` version on the bound thread. Two safeguards prevent partial or repeated failures:

- **Two-tick settle.** A changed file must read identical on two consecutive polls before the compiler sees it. A half-written save — the editor flushing in chunks — never reaches Ecky mid-write.
- **A broken save fails once, loudly, then waits.** If the edited source does not compile, the watcher reports the failure once for that exact content and then goes quiet until you change the file again. It does not re-render the same mistake every tick.

When you need to reason about the folder explicitly, `project_folder_status` classifies it:

- `clean` — file matches the bound version; nothing to do.
- `fileChanged` — you edited the file; the watcher will apply it (or you can).
- `threadAdvanced` — the thread moved on without the folder; the folder is stale. Re-export to refresh it.
- `conflict` — both sides moved. The watcher will **not** auto-resolve this; applying requires an explicit force, and the previous head stays available as a version so nothing is lost.
- `missing` — no folder or no manifest yet.

**The folder is a mirror, not a second database.** Threads and versions remain authoritative. Do not edit `ecky-project.json`; refresh a stale mirror or resolve a conflict explicitly.

## Final Model: Integrated Film Adapter Open Helicoid v9

The final example is a multi-part film adapter with sliding rail joints and a two-start helicoid. Its base, insert stack, tunnel, cover, and lens carrier share fit dimensions but remain separate printable parts.

<!-- render-source: ../examples/ecky-integrated-film-adapter-open-helicoid-v9.ecky -->

![Rendered output for Final Model: Integrated Film Adapter Open Helicoid v9, example 1](assets/11-complex-film-adapter-01.png)

Full source: `docs/books/ecky-ir/examples/ecky-integrated-film-adapter-open-helicoid-v9.ecky`. The sections below isolate the six mechanical subsystems.

### 1. Public controls define physical fit

The first block exposes dimensions that matter after printing: film format, aperture, rail geometry, insert stack, film gap, lens bore, and helicoid thread geometry.

```scheme
(params
  (select film_format "120_645" :label "film format"
    :options (("120 6x9" "120_6x9") ("120 6x6" "120_6x6")
              ("120 6x4.5" "120_645") ("135 36x24" "135") ("110" "110")))
  (number rail_tip_w 5.4 :label "joint max W" :min 3.5 :max 8 :step 0.1)
  (number rail_h 4.2 :label "joint H" :min 2 :max 6 :step 0.1)
  (number fit_clearance 0.25 :label "fit clearance" :min 0 :max 0.8 :step 0.05)
  (number film_gap 0.6 :label "film velvet gap" :min 0.1 :max 1.5 :step 0.05)
  (number lens_bore_d 59.6 :label "lens bore D" :min 50 :max 68 :step 0.1)
  (number thread_turns 3.2 :label "helicoid turns" :min 1.5 :max 5 :step 0.1)
  (number thread_clearance 0.25 :label "helicoid clearance" :min 0.15 :max 0.6 :step 0.05))
```

This is the same habit as earlier chapters: public parameters are physical, not arbitrary. `fit_clearance` appears in rail channels and detents. `film_gap` controls the clamp stack. `lens_bore_d`, `thread_turns`, and `thread_clearance` drive the helicoid interface.

### 2. Base makes recessed pockets and male rails

The base starts as a rounded plate, removes the aperture and insert pocket, then adds male triangular rail profiles on both long sides.

```scheme
(part base_recessed_male_rails
  (build
    (shape raw_plate
      (extrude (rounded-rect outer_w outer_h corner_r) base_h))
    (shape aperture_cut
      (translate 0 0 -0.1
        (box aperture_w aperture_h (+ base_h 0.2))))
    (shape frame_pocket
      (translate 0 0 (- base_h pocket_depth)
        (extrude
          (rounded-rect (+ holder_w (* 2 fit_clearance))
                        (+ holder_h (* 2 fit_clearance))
                        holder_corner_r)
          (+ pocket_depth 0.2))))
    (shape plate
      (difference raw_plate aperture_cut frame_pocket film_path_cut))
    (shape rail_left
      (translate (- (/ outer_w 2)) rail_y rail_z
        (rotate 0 90 0
          (extrude rail_profile_pos outer_w))))
    (result
      (fuse plate rail_left rail_right detent_top_left detent_top_right
            detent_bottom_left detent_bottom_right))))
```

`rail_profile_pos` and `rail_profile_neg` are small triangular sketches. They become long rails by `extrude`, then get fused onto the base. This is the same sketch-to-extrude move from chapter 2, applied to sliding joints.

### 3. Film insert is a two-piece stack

The lower insert carries the film guides. The upper insert clamps above the film gap. Both use the selected film format to derive `frame_w`, `frame_h`, and `film_strip_w`.

```scheme
(shape frame_w
  (if (= film_format "135") 36
    (if (= film_format "110") 17
      (if (= film_format "120_645") 42
        (if (= film_format "120_6x6") 56 84)))))
(shape guide_top
  (translate 0 (/ film_channel_h 2) (- (+ holder_thickness (/ film_guide_h 2)) 0.24)
    (box (- holder_w 8) film_guide_rail_w film_guide_h)))
(shape lower_frame
  (difference
    lower_raw
    aperture_cut
    notch_top_left
    notch_top_right
    notch_bottom_left
    notch_bottom_right))
```

The insert stack is why the model has `holder_thickness`, `film_gap`, and `insert_lid_thickness` as separate controls. Those are real Z layers, not a single magic height.

### 4. Tunnel joins bottom and top modules

The tunnel module has both sides of the sliding interface. Its bottom cuts female channels so it can slide onto the base rails. Its top adds male rails so the top cover can slide onto the tunnel.

```scheme
(part tunnel_female_bottom_male_top
  (build
    (shape channel_profile_pos
      (polygon
        (((/ (+ rail_h (* 2 fit_clearance)) 2) 0)
         (0 (/ (+ rail_tip_w (* 2 fit_clearance)) 2))
         ((- (/ (+ rail_h (* 2 fit_clearance)) 2)) 0))))
    (shape body
      (difference body_blank tunnel_cut))
    (shape channel_left
      (translate (- (+ (/ outer_w 2) lead_in)) rail_y channel_z
        (rotate 0 90 0
          (extrude channel_profile_pos (+ outer_w (* 2 lead_in))))))
    (shape rail_left
      (translate (- (/ outer_w 2)) rail_y rail_z
        (rotate 0 90 0
          (extrude rail_profile_pos outer_w))))
    (result
      (fuse
        (difference body channel_left channel_right)
        rail_left
        rail_right))))
```

This is the sliding-joint core. Female channels are oversized by `fit_clearance`; male rails use the nominal profile. The book built these ideas earlier as sketches, cuts, and named clearances. Here they become a printable mechanical interface.

### 5. Top cover is open and owns the female helicoid

The cover removes matching rail channels and opens the center so the helicoid socket is visible. The female thread is modeled as two clipped helical ridges subtracted from a sleeve.

```scheme
(shape female_thread_a_raw
  (translate 0 0 (+ socket_base_z thread_z0)
    (helical-ridge
      :radius female_root_r
      :pitch thread_pitch
      :height thread_len
      :base-width female_axial_width
      :crest-width (* female_axial_width 0.58)
      :depth female_depth)))
(shape female_thread_a
  (clip-box female_thread_a_raw
    :x ((- female_thread_clip_r) female_thread_clip_r)
    :y ((- female_thread_clip_r) female_thread_clip_r)
    :z ((+ socket_base_z 0.05) (+ socket_base_z sleeve_h 1))))
(shape female_thread_b
  (rotate 0 0 180 female_thread_a))
(shape socket_threaded_shell
  (difference
    (translate 0 0 socket_base_z
      (cylinder socket_outer_r sleeve_h))
    female_thread_a
    female_thread_b))
```

`thread_pitch` comes from carrier height and turn count. `female_thread_b` is the second start, made by rotating the first. The clipped ends keep the helix printable and bounded inside the socket height.

### 6. Moving lens carrier matches the cover

The carrier is separate and previewed to the side with `carrier_preview_x`. It uses the same thread pitch, height, and clearance math, but its ridges are fused onto the carrier body instead of cut out of the socket.

```scheme
(shape male_thread_a_raw
  (translate 0 0 thread_z0
    (helical-ridge
      :radius ridge_root_r
      :pitch thread_pitch
      :height thread_len
      :base-width thread_width
      :crest-width (* thread_width 0.58)
      :depth ridge_sweep_depth)))
(shape male_thread_a
  (clip-box male_thread_a_raw
    :x ((- thread_clip_r) thread_clip_r)
    :y ((- thread_clip_r) thread_clip_r)
    :z (0 carrier_h)))
(shape carrier_outer
  (fuse carrier_body male_thread_a male_thread_b))
(result
  (translate carrier_preview_x 0 socket_base_z
    (difference carrier_outer stop_aperture lens_slip_bore)))
```

That last `translate` is preview layout, not fit math. The carrier is offset so the reader can see both halves of the helicoid in one render.

### Combined mechanism

The mechanism combines earlier patterns directly: profiles become rails and channels; named clearances control sliding fits; repeated structures stay parametric; frames place mating geometry; verification records fit requirements. The carrier threads into the cover while the remaining modules stack through rail interfaces.

## Mesh and Image Geometry: Polygons in 3D

Ecky supports typed triangle geometry alongside analytic B-rep operations. Mesh execution is bounded and deterministic; it does not run Blender Python or arbitrary scripts.

### Open surfaces and closed solids

Use `mesh` for a triangle surface. Use `polyhedron` when the triangles form a printable solid.

```scheme
(define vertices
  '((0 0 0) (20 0 0) (0 20 0) (0 0 20)))

(define triangles
  '((0 2 1) (0 1 3) (1 2 3) (2 0 3)))

(model
  (verify
    (tag mesh_clean)
    (metric bad_edges (stl non-manifold-edge-count))
    (expect bad_edges (= 0)))
  (part tetrahedron
    (polyhedron
      :vertices vertices
      :triangles triangles)))
```

`mesh` permits boundaries and previews them honestly as an open surface. `polyhedron` requires one closed orientable component with nonzero volume. Both reject invalid indices, repeated vertices, zero-area faces, duplicates, inconsistent winding, and resource-budget overflow before render.

Prefer formula-generated vertex/triangle lists for repeated or mathematical geometry. Keep one binding for each list instead of expanding thousands of copied triangles into source.

### Heightmaps become dimensioned relief

`heightfield` samples image luminance into a closed planar mesh. Physical dimensions remain explicit.

```scheme
(model
  (verify
    (tag relief_closed)
    (metric bad_edges (stl non-manifold-edge-count))
    (expect bad_edges (= 0)))
  (part relief
    (heightfield image-path
      :width 100
      :depth 70
      :relief-height 4
      :base-thickness 1.2
      :invert #f)))
```

The image path points to a staged local asset. Empty selection is pending, not fake geometry. Decode errors retain raw path/error evidence. Width, depth, relief height, and base thickness must be positive.

### Orthographic images become reviewed sketches

Front, Top, and Side line art follows a different route:

1. select each raster and enter physical calibration;
2. tune threshold/inversion;
3. extract closed contour candidates;
4. review a candidate into an editable sketch primitive;
5. run existing preview-hull and exact candidate validation.

Raster provenance records asset digest, view, calibration, threshold, inversion, contour id, and extractor version. Failed extraction preserves the last reviewed sketch. Preview hull remains diagnostic until STEP and hidden-line validation pass.

### Export truth follows representation

Pure mesh output offers STL. Multipart viewer assets also enable 3MF or multipart STL export. Pure mesh does not offer STEP.

A closed mesh may enter the hybrid `import-stl -> solidify` bridge before a supported BRep boolean. Successful STEP from that route is labeled **Faceted poly-BRep** and carries source mesh digests/topology evidence. It is triangle-derived, not analytic source CAD.

Reference photos are another route: a vision model can propose inferred `.ecky` source, then normal compilation and verification run. One perspective photo remains an inferred approximation; response text alone cannot mark it reconstructed or accepted CAD.


## Appendix: Language Reference

Use this section after the lessons when you need exact forms, signatures, helper names, selector strings, and verification grammar. The reference is intentionally dense; the earlier chapters show when each piece matters.

### Generated Operation Index

Generated from the runtime surface registry. Do not edit this table by hand.

<!-- ECKY_GENERATED_OP_INDEX_START -->
| Form | Available backends |
| --- | --- |
| `*` | build123d, ecky-rust, freecad |
| `+` | build123d, ecky-rust, freecad |
| `-` | build123d, ecky-rust, freecad |
| `/` | build123d, ecky-rust, freecad |
| `<` | build123d, ecky-rust, freecad |
| `<=` | build123d, ecky-rust, freecad |
| `=` | build123d, ecky-rust, freecad |
| `>` | build123d, ecky-rust, freecad |
| `>=` | build123d, ecky-rust, freecad |
| `abs` | build123d, ecky-rust, freecad |
| `and` | build123d, ecky-rust, freecad |
| `append` | build123d, ecky-rust, freecad |
| `apply` | build123d, ecky-rust, freecad |
| `arc-array` | build123d, ecky-rust, freecad |
| `atan` | build123d, ecky-rust, freecad |
| `atan2` | build123d, ecky-rust, freecad |
| `attractor-field` | ecky-rust |
| `begin` | build123d, ecky-rust, freecad |
| `bezier-path` | build123d, ecky-rust, freecad |
| `box` | build123d, ecky-rust, freecad |
| `bspline` | build123d, ecky-rust, freecad |
| `build` | build123d, ecky-rust, freecad |
| `cell-distance2` | build123d, ecky-rust, freecad |
| `cellular` | ecky-rust |
| `chamfer` | build123d, ecky-rust, freecad |
| `circle` | build123d, ecky-rust, freecad |
| `clamp` | build123d, ecky-rust, freecad |
| `clip-box` | build123d, ecky-rust, freecad |
| `common` | build123d, ecky-rust, freecad |
| `compound` | build123d, ecky-rust, freecad |
| `concat-map` | build123d, ecky-rust, freecad |
| `cone` | build123d, ecky-rust, freecad |
| `cos` | build123d, ecky-rust, freecad |
| `cut` | build123d, ecky-rust, freecad |
| `cylinder` | build123d, ecky-rust, freecad |
| `define` | build123d, ecky-rust, freecad |
| `deg` | build123d, ecky-rust, freecad |
| `deg->rad` | build123d, ecky-rust, freecad |
| `diamond` | ecky-rust |
| `diamond-field` | ecky-rust |
| `difference` | build123d, ecky-rust, freecad |
| `ellipse` | build123d, ecky-rust, freecad |
| `empty?` | build123d, ecky-rust, freecad |
| `enumerate` | build123d, ecky-rust, freecad |
| `even?` | build123d, ecky-rust, freecad |
| `extrude` | build123d, ecky-rust, freecad |
| `fbm` | ecky-rust |
| `fbm2` | build123d, ecky-rust, freecad |
| `fillet` | build123d, ecky-rust, freecad |
| `filter` | build123d, ecky-rust, freecad |
| `flat-map` | build123d, ecky-rust, freecad |
| `floor` | build123d, ecky-rust, freecad |
| `fold` | build123d, ecky-rust, freecad |
| `for-compound` | build123d, ecky-rust, freecad |
| `for-union` | build123d, ecky-rust, freecad |
| `fourier` | ecky-rust |
| `fuse` | build123d, ecky-rust, freecad |
| `grid-array` | build123d, ecky-rust, freecad |
| `groove` | build123d, ecky-rust, freecad |
| `gyroid` | ecky-rust |
| `hammered` | ecky-rust |
| `hash-signed` | build123d, ecky-rust, freecad |
| `hash01` | build123d, ecky-rust, freecad |
| `heightfield` | ecky-rust |
| `helical-ridge` | build123d, ecky-rust, freecad |
| `henon-points` | build123d, ecky-rust, freecad |
| `hull` | ecky-rust |
| `if` | build123d, ecky-rust, freecad |
| `import-stl` | build123d, ecky-rust, freecad |
| `intersection` | build123d, ecky-rust, freecad |
| `jitter2` | build123d, ecky-rust, freecad |
| `jittered-grid` | build123d, ecky-rust, freecad |
| `lambda` | build123d, ecky-rust, freecad |
| `lerp` | build123d, ecky-rust, freecad |
| `let` | build123d, ecky-rust, freecad |
| `let*` | build123d, ecky-rust, freecad |
| `linear-array` | build123d, ecky-rust, freecad |
| `linspace` | build123d, ecky-rust, freecad |
| `list` | build123d, ecky-rust, freecad |
| `list?` | build123d, ecky-rust, freecad |
| `location` | build123d, ecky-rust, freecad |
| `loft` | build123d, ecky-rust, freecad |
| `logistic-bifurcation-points` | build123d, ecky-rust, freecad |
| `lorenz-points` | build123d, ecky-rust, freecad |
| `make-face` | build123d, ecky-rust, freecad |
| `map` | build123d, ecky-rust, freecad |
| `max` | build123d, ecky-rust, freecad |
| `mesh` | ecky-rust |
| `meta` | build123d, ecky-rust, freecad |
| `min` | build123d, ecky-rust, freecad |
| `mirror` | build123d, ecky-rust, freecad |
| `neovius` | ecky-rust |
| `noise2` | build123d, ecky-rust, freecad |
| `not` | build123d, ecky-rust, freecad |
| `null?` | build123d, ecky-rust, freecad |
| `odd?` | build123d, ecky-rust, freecad |
| `offset` | build123d, ecky-rust, freecad |
| `offset-rounded` | build123d, ecky-rust, freecad |
| `or` | build123d, ecky-rust, freecad |
| `organic-loop` | build123d, ecky-rust, freecad |
| `params` | build123d, ecky-rust, freecad |
| `part` | build123d, ecky-rust, freecad |
| `path` | build123d, ecky-rust, freecad |
| `path-frame` | build123d, ecky-rust, freecad |
| `place` | build123d, ecky-rust, freecad |
| `plane` | build123d, ecky-rust, freecad |
| `polar-points` | build123d, ecky-rust, freecad |
| `polygon` | build123d, ecky-rust, freecad |
| `polyhedron` | ecky-rust |
| `polyline` | build123d, ecky-rust, freecad |
| `profile` | build123d, ecky-rust, freecad |
| `quote` | build123d, ecky-rust, freecad |
| `rad` | build123d, ecky-rust, freecad |
| `rad->deg` | build123d, ecky-rust, freecad |
| `radial-array` | build123d, ecky-rust, freecad |
| `range` | build123d, ecky-rust, freecad |
| `rectangle` | build123d, ecky-rust, freecad |
| `reduce` | build123d, ecky-rust, freecad |
| `regular-polygon` | build123d, ecky-rust, freecad |
| `repeat` | build123d, ecky-rust, freecad |
| `repeat-compound` | build123d, ecky-rust, freecad |
| `repeat-pick` | build123d, ecky-rust, freecad |
| `repeat-union` | build123d, ecky-rust, freecad |
| `result` | build123d, ecky-rust, freecad |
| `reverse` | build123d, ecky-rust, freecad |
| `revolve` | build123d, ecky-rust, freecad |
| `rib` | build123d, ecky-rust, freecad |
| `ribs` | ecky-rust |
| `ring` | build123d, ecky-rust, freecad |
| `rings` | ecky-rust |
| `rossler-points` | build123d, ecky-rust, freecad |
| `rotate` | build123d, ecky-rust, freecad |
| `rounded-polygon` | build123d, ecky-rust, freecad |
| `rounded-rect` | build123d, ecky-rust, freecad |
| `sampled-radial-loft` | build123d, ecky-rust, freecad |
| `scale` | build123d, ecky-rust, freecad |
| `schwarz-d` | ecky-rust |
| `schwarz-p` | ecky-rust |
| `shape` | build123d, ecky-rust, freecad |
| `shell` | build123d, ecky-rust, freecad |
| `sin` | build123d, ecky-rust, freecad |
| `slot-arc` | build123d, ecky-rust, freecad |
| `slot-center-point` | build123d, ecky-rust, freecad |
| `slot-center-to-center` | build123d, ecky-rust, freecad |
| `slot-overall` | build123d, ecky-rust, freecad |
| `smoothstep` | build123d, ecky-rust, freecad |
| `sphere` | build123d, ecky-rust, freecad |
| `spiral` | ecky-rust |
| `superellipse-point` | build123d, ecky-rust, freecad |
| `svg` | build123d, ecky-rust, freecad |
| `sweep` | build123d, ecky-rust, freecad |
| `tan` | build123d, ecky-rust, freecad |
| `taper` | build123d, ecky-rust, freecad |
| `text` | build123d, ecky-rust, freecad |
| `thread` | build123d, ecky-rust, freecad |
| `torus` | build123d, ecky-rust, freecad |
| `translate` | build123d, ecky-rust, freecad |
| `trapezoid` | build123d, ecky-rust, freecad |
| `twist` | build123d, ecky-rust, freecad |
| `union` | build123d, ecky-rust, freecad |
| `voronoi-cells` | build123d, ecky-rust, freecad |
| `voronoi2` | build123d, ecky-rust, freecad |
| `wall-pattern` | ecky-rust |
| `wave-loop` | build123d, ecky-rust, freecad |
| `wedge` | build123d, ecky-rust, freecad |
| `xor` | build123d, ecky-rust, freecad |
| `zero?` | build123d, ecky-rust, freecad |
| `zip` | build123d, ecky-rust, freecad |
<!-- ECKY_GENERATED_OP_INDEX_END -->

## Language Overview

Scope here:

- `ecky/cad` exported CAD forms and ops
- `ecky/core` helper functions shipped with Ecky
- `ecky/params` parameter forms
- lowerer-visible keywords people otherwise guess from source

Out of scope here:

- full Steel standard library reference
- backend implementation internals
- UI behavior outside `.ecky` authoring

Mental model:

- `.ecky` is Scheme surface syntax
- compiler lowers it into Core IR
- verifier checks value kinds and op signatures
- lowerers map Core IR into build123d, FreeCAD, or direct OCCT execution

Read this order if new:

- `Forms and Structure`
- `Params and Controls`
- `Primitive Signatures`
- `Boolean and Transform Signatures`
- `Surface and Path Signatures`
- `Array and Frame Signatures`
- `Special / Custom Operations`
- `Selector Strings and Named Keywords`
- `Cookbook`

## Forms and Structure

This is top-level authoring grammar. If source feels mysterious, start here.

### `model`

```scheme
(model
  ...)
```

- root form for one design
- source must start with `(model ...)`
- contains `params`, `part`, `feature`, helper `define`s, and local setup

### `part`

```scheme
(part body expr)
(part body "Human Label" expr)
```

- positional 1: part id symbol
- positional 2: optional display label text
- final positional: expression producing geometry

### `feature`

Two forms exist:

```scheme
(feature body :role shell expr)
(feature body :role shell :params (width height) expr)
```

- positional 1: feature id symbol
- required keyword: `:role`
- optional keyword: `:params`
- final positional: expression producing geometry

Use `feature` when geometry needs explicit semantic identity, role, and parameter-key tracking.

### `build`

```scheme
(build
  (shape outer expr)
  (shape cavity expr)
  (result expr))
```

- local binding block
- accepts `shape` bindings plus one `result`
- `result` must come once
- do not place new `shape` bindings after `result`

### `shape`

```scheme
(shape ribs expr)
```

`shape` is not geometry op. It is bind statement inside `build`.

- positional 1: local binding name
- positional 2: expression producing value

Read it as:

- bind intermediate value
- give later code a name
- keep boolean stacks readable

### `result`

```scheme
(result expr)
```

- final value returned by `build`

### `assembly` (planned)

Reserved shape sketch:

```scheme
(model
  (assembly exploded_preview
    ...))
```

- planned top-level clause for explicit multi-part assembly recipes
- spelling reserved in book now; runtime/compiler support deferred
- spec'd grammar reserved now; implementation deferred until views prove the display/manufacturing split
- intended to formalize what component packages already do at the package layer
- assemblies stay placement-based as today; no mate/joint solver implied
- examples here mark intent only, not accepted source today
- until implementation lands, keep physical bodies as `part`s, use `view` for preview-only offsets, and use component packages for solved assembly workflows

### `export` (planned)

Reserved shape sketch:

```scheme
(model
  (export manufacturing
    ...))
```

- planned top-level clause for authored export/manufacturing policy
- spelling reserved in book now; runtime/compiler support deferred
- reserved until views prove the display/manufacturing split
- preview transforms never affect STL or STEP artifacts
- examples here mark intent only, not accepted source today
- until implementation lands, use current export commands, artifact manifests, and package output modes outside `.ecky` source

## Components

A component is a named, parameterized, closed geometry unit. Define once,
instantiate anywhere, override knobs at the call site. `model` and `part`
stay valid forever; components add reuse on top without changing them.

### `define-component`

```scheme
(define-component knuckle
  ((number pin_d 8 :label "Pin diameter" :min 4 :max 12 :step 0.5)
   (number clearance 0.3))
  (difference
    (cylinder (* 2 pin_d) 10 96)
    (cylinder (+ pin_d clearance) 12 96)))
```

- positional 1: component name symbol
- positional 2: signature list; entries use the same grammar as `params`
  entries (kind, key, optional default, keyword metadata)
- final positional: one geometry expression
- optional `(verify ...)` clauses may sit alongside the geometry expression
- valid at top level or as a direct `model` clause

### Instantiation

```scheme
(part hinge_a (knuckle :pin_d 6))   ; override pin_d, clearance defaults
(part hinge_b (knuckle))            ; all defaults apply
```

- arguments are keywords only: `(name :key value ...)`
- omitted keys take their signature defaults
- a signature entry without a default is required at every call site
- unknown keyword or missing required key fails compile with the component
  name and its signature listed
- components instantiate other components; cycles are rejected and nesting
  is capped at depth 32

### Closedness

A component body sees its signature keys plus bindings made inside the body
(`let`, `let*`, lambda parameters, `repeat` indices, `build` shapes) and
nothing else. Referencing a model param or outer binding is a compile error
naming the variable and the component. Closedness is what makes a component
copy-inlineable: paste the `define-component` into any model and it works.

### Verify travel

`verify` clauses inside a component expand once per instantiation, with the
tag namespaced by the instantiating part key:

```scheme
(define-component pin ((number d 2))
  (verify (tag pin_ok) (metric min_wall_thickness "body") (expect (>= value 1)))
  (cylinder d 10 48))

(part left (pin :d 3))   ; verify tag becomes left/pin_ok
```

A pasted component therefore carries its own checks — reuse includes proof.

### Component Library Workflow (MCP)

Agents lift proven parts into the shared library and reuse them by source:

1. `component_extract` — pass the model source and a `partKey`. Referenced
   model params become the signature with metadata preserved; scalar outer
   `let`/`let*` bindings become plain defaults; non-scalar free references
   are reported as blockers. Set `save: true` to store the component.
2. `component_search` — compact headers only (name, one-liner, param keys,
   tags). Bodies are never returned by search.
3. `component_get` — full copy-inline `define-component` source for one
   component by name. Paste it into the model and instantiate it.

Extraction is copy-inline only: the returned source is self-contained and
no registry reference is created implicitly.

## Projects As Folders

A project can live as a plain folder on disk: edit `model.ecky` with any
editor or LLM file skill; Ecky stays the renderer, validator, and history.

```text
<projectsRoot>/<slug>/
  model.ecky          edit this with anything
  ecky-project.json   binding manifest, owned by Ecky
```

- `project_folder_export` writes the folder from a thread's active version
- `project_folder_status` classifies it: `clean`, `fileChanged`,
  `threadAdvanced` (stale; re-export), `conflict`, or `missing`
- `project_folder_apply` compiles the edited file, renders a preview, and
  commits it as a new version on the bound thread, then rebases the manifest

Rules:

- the folder is a mirror; threads and versions remain the record
- a stale folder never silently clobbers the thread: re-export to refresh
- a conflict (both sides moved) applies only with an explicit force, and the
  previous head stays available as a version
- never edit `ecky-project.json` by hand

## Verify Clauses

Use `verify` when source should declare structural expectations explicitly.

```scheme
(model
  (verify
    (tag front_gap body.front_window_1)
    (metric gap (clearance min-distance body lid))
    (expect gap (>= 3)))
  (part body (box 10 10 10))
  (part lid (box 10 10 10)))
```

- model verification is top-level under `model`
- component-owned verification is the one exception: a `verify` clause may
  sit directly inside `define-component`, before its geometry expression, and
  expands once per instance
- one verify clause requires three sections in order:
  - `tag`
  - `metric`
  - `expect`
- nested `verify` inside geometry or helper expressions is rejected
- empty `(verify)` is rejected

### `tag`

```scheme
(tag body_shell body.front_window_1)
```

- carries authored labels, ids, or references
- payload stays opaque to compiler/core IR
- useful for human grouping and later diagnostics

### `metric`

```scheme
(metric check (manifest has-step))
(metric triangles (stl triangle-count))
(metric gap (clearance min-distance body.front_window_1 lid.front_skirt))
```

- first item usually names local check alias
- second item is metric expression
- current runtime metric namespaces:
  - `manifest`
  - `stl`
  - `clearance`

Current shipped metric keys:

- `manifest has-step`
- `manifest has-preview-stl`
- `manifest edge-target-count`
- `manifest face-target-count`
- `manifest export-format-count`
- `manifest part-count`
- `stl triangle-count`
- `stl connected-component-count`
- `stl non-manifold-edge-count`
- `stl overhang-face-count`
- `clearance min-distance`

`clearance min-distance` compares the minimum distance between two named
selectors.

- selectors can name parts, selection targets, or correspondence outputs
- part selectors use manifest bounds
- edge and face selectors use runtime mesh target geometry when available
- unresolved selectors fail authored verify with a raw runtime error

### `expect`

```scheme
(expect check (= true))
(expect triangles (> 100))
```

- first item should reference the metric alias used above
- second item is comparator form
- current shipped comparators:
  - `=`
  - `>`
  - `>=`
  - `<`
  - `<=`

Authoring rule:

- fix geometry or exports until `verify` passes
- do not remove `verify` clauses to bypass authored requirements

## Params and Controls

Parameter forms live in `ecky/params`.

### `params`

```scheme
(params
  decl
  decl
  :relations ((<= wall shell) (>= shell 1.6)))
```

- container for parameter declarations
- optional `:relations` list attaches cross-parameter constraints

Supported relation operators:

- `<`
- `<=`
- `>`
- `>=`

### `number`

```scheme
(number wall 2.4
  :label "Wall"
  :min 0.8
  :max 8
  :step 0.1
  :unit length
  :frozen #f)
```

- positional 1: parameter key symbol
- positional 2: default number
- keywords:
  - `:label` text
  - `:min` number
  - `:max` number
  - `:step` number
  - `:unit` one of `length | angle | ratio | count | text`
  - `:frozen` boolean

### Units and suffixed literals

Humans may use bare numbers because Ecky's base units are millimetres and
degrees. Agent-generated physical dimensions should use suffixed literals like
mm/cm/in/deg/rad when the suffix makes intent clearer.

Examples:

- `12mm`
- `2.54cm`
- `0.25in`
- `45deg`
- `1.5708rad`

Prompt generators use suffixed literals for physical lengths and angles. Bare
numbers remain appropriate for counts, ratios, segments, and unitless math.

### `toggle`

```scheme
(toggle useFillet #t
  :label "Use fillet"
  :frozen #f)
```

- positional 1: parameter key symbol
- positional 2: default boolean
- keywords:
  - `:label`
  - `:frozen`

### `select`

```scheme
(select material "PLA"
  :label "Material"
  :unit text
  :options
    ((option "PLA" "PLA")
     (option "PETG" "PETG")
     (option "ABS" "ABS"))
  :frozen #f)
```

- positional 1: parameter key symbol
- positional 2: default choice value
- required keyword for practical use: `:options`
- optional keywords:
  - `:label`
  - `:unit`
  - `:frozen`

### `image`

```scheme
(image decal "assets/logo.svg"
  :label "Decal"
  :frozen #f)
```

- positional 1: parameter key symbol
- positional 2: default image path text
- optional keywords:
  - `:label`
  - `:frozen`

### `option`

```scheme
(option "Large" 42)
(option "PLA" "PLA")
```

- positional 1: display label
- positional 2: value
- valid value kinds:
  - number
  - string / text symbol

## Core Helper Library

Helpers here come from `ecky/core`.

### Constructors and Symbols

#### `vec2`

- signature: `vec2 x y`
- returns: 2D point

#### `vec3`

- signature: `vec3 x y z`
- returns: 3D point

#### `start`

- constant anchor symbol for path/frame usage

#### `end`

- constant anchor symbol for path/frame usage

#### `xy`

- constant plane symbol

#### `yz`

- constant plane symbol

#### `xz`

- constant plane symbol

#### `true`

- constant boolean alias for `#t`

#### `false`

- constant boolean alias for `#f`

### Sequence Helpers

#### `zip`

- signature: `zip list1 list2 ...`
- returns: list of tuples

#### `enumerate`

- signature: `enumerate list`
- signature: `enumerate start-index list`
- returns: list of `(index item)` pairs

#### `flat-map`

- signature: `flat-map fn list1 list2 ...`
- returns: concatenated mapped list

#### `concat-map`

- signature: `concat-map fn list1 list2 ...`
- same behavior as `flat-map`

#### `linspace`

- signature: `linspace start stop count`
- returns: evenly spaced number list
- special cases:
  - `count <= 0` -> empty list
  - `count == 1` -> single-item list containing `start`

### Scalar Math Helpers

#### `pi`

- constant `3.141592653589793`

#### `tau`

- constant `6.283185307179586`

#### `clamp`

- signature: `clamp value lower upper`
- returns: value clamped into `[lower, upper]`

#### `lerp`

- signature: `lerp start end t`
- returns: linear interpolation

#### `invlerp`

- signature: `invlerp start end value`
- returns: normalized interpolation factor

#### `remap`

- signature: `remap value in-start in-end out-start out-end`
- returns: value remapped from one range into another

#### `deg`

- signature: `deg degrees`
- returns: radians

#### `rad`

- signature: `rad radians`
- returns: degrees

#### `deg->rad`

- signature: `deg->rad degrees`
- returns: radians

#### `rad->deg`

- signature: `rad->deg radians`
- returns: degrees

#### `smoothstep`

- signature: `smoothstep edge0 edge1 x`
- returns: smoothed `0..1` interpolation

#### `square`

- signature: `square x`
- returns: `x * x`

#### `cube`

- signature: `cube x`
- returns: `x * x * x`

### Noise and Field Helpers

#### `hash01`

- signature: `hash01 x y seed`
- returns: deterministic `0..1` scalar

#### `hash-signed`

- signature: `hash-signed x y seed`
- returns: deterministic `-1..1` scalar

#### `noise2`

- signature: `noise2 x y seed`
- returns: smoothed 2D value noise

#### `fbm2`

- signature: `fbm2 x y seed octaves lacunarity gain`
- returns: fractal Brownian motion sample

#### `voronoi2`

- signature: `voronoi2 x y seed`
- returns: Voronoi-style scalar field

#### `cell-distance2`

- signature: `cell-distance2 x y seed`
- returns: normalized cell distance field

#### `jitter2`

- signature: `jitter2 x y amount seed`
- returns: jittered 2D point

#### `jittered-grid`

- signature: `jittered-grid rows cols dx dy amount seed`
- returns: list of jittered 2D points

### Shape-Driving Point Generators

#### `polar-points`

- signature: `polar-points count radius`
- returns: closed-style circular 2D sample list

#### `organic-loop`

- signature: `organic-loop count radius amount seed`
- returns: noisy radial 2D loop

#### `wave-loop`

- signature: `wave-loop count rx ry amp waves seed`
- returns: wavy ellipse-like 2D loop

#### `superellipse-point`

- signature: `superellipse-point rx ry n t`
- returns: single 2D point on superellipse

#### `voronoi-cells`

- signature: `voronoi-cells rows cols dx dy amount seed`
- returns: jittered cell-center point list

### Chaotic / Generative Point Clouds

#### `lorenz-points`

- signature: `lorenz-points count dt scale`
- returns: list of 3D points

#### `rossler-points`

- signature: `rossler-points count dt scale`
- returns: list of 3D points

#### `logistic-bifurcation-points`

- signature: `logistic-bifurcation-points count seed scale`
- returns: list of 2D points

#### `henon-points`

- signature: `henon-points count seed scale`
- returns: list of 2D points

Use helper outputs as inputs to `polygon`, `bspline`, `path`, `bezier-path`, `map`, and repetition logic.

## Value Kinds and IR Nodes

Verifier-backed value kinds:

- `Any`
- `Number`
- `Boolean`
- `Text`
- `List`
- `Point2`
- `Point3`
- `Sketch`
- `Path`
- `Frame`
- `Compound`
- `Solid`

Core node kinds:

- `Literal`
- `Reference`
- `Build`
- `Let`
- `If`
- `Call`
- `Range`
- `Map`
- `Apply`
- `List`
- `Group`

If typecheck fails, compiler is checking these kinds, not backend Python text.

## Primitive Signatures

These are explicit authored calls. When backend diverges, caveat is spelled out.

### `box`

- signature: `box width depth height`
- result: `Solid`
- keywords:
  - `:align (x y z)` with each axis one of `min | center | max`

### `sphere`

- signature: `sphere radius`
- result: `Solid`
- keywords:
  - `:align (x y z)`

### `cylinder`

- signature: `cylinder radius height`
- signature: `cylinder radius height segments`
- result: `Solid`
- keywords:
  - `:align (x y z)`

### `cone`

- signature: `cone radius1 radius2 height`
- signature: `cone radius1 radius2 height segments`
- result: `Solid`
- keywords:
  - `:align (x y z)`

### `circle`

- signature: `circle radius`
- signature: `circle radius segments`
- result: `Sketch`

### `rectangle`

- signature: `rectangle width height`
- result: `Sketch`

### `rounded-rect`

- signature: `rounded-rect width height radius`
- result: `Sketch`

### `rounded-polygon`

- signature: `rounded-polygon points radius`
- signature: `rounded-polygon points radius segments`
- `points`: list of 2D points
- result: `Sketch`

### `polygon`

- signature: `polygon points`
- `points`: list of 2D points
- result: `Sketch`

### `profile`

- signature: `profile loop1 loop2 ...`
- signature: `profile :outer outer-loop :holes hole-loop-or-list`
- result: `Sketch`

Rules:

- positional form treats every argument as sketch/wire loop
- keyword form accepts `:outer` and `:holes` only
- current hole-aware lowerers expect exactly one outer loop when `:holes` is used

### `make-face`

- signature: `make-face wire1 wire2 ...`
- result: `Sketch`
- use when you already have wire-like geometry and need face/sketch result

### `text`

- signature: `text string size`
- result: `Sketch`
- normal use: feed into `extrude`

Example:

```scheme
(extrude (text "HELLO" 12) 2)
```

### `svg`

- build123d-authored signature: `svg path`
- FreeCAD-authored signature: `svg path [target-width] [target-height] [fit-mode]`
- result: `Sketch`

Known fit modes from lowerers/tests:

- `"contain"`
- `"cover"`
- `"stretch"`
- `"fill"`

### `import-stl`

- signature: `import-stl path`
- result: imported solid/mesh-like geometry

### `ring`

- signature: `ring outer-radius inner-radius`
- signature: `ring outer-radius inner-radius segments`
- result: `Sketch`
- lowering behavior: alias for profile-with-hole semantics

## Boolean and Transform Signatures

### `union`

- signature: `union shape1 shape2 ...`
- result: shape-like value

### `fuse`

- alias of `union`

### `difference`

- signature: `difference base cut1 cut2 ...`
- result: shape-like value

### `cut`

- alias of `difference`

### `intersection`

- signature: `intersection shape1 shape2 ...`
- result: shape-like value

### `common`

- alias of `intersection`

### `xor`

- signature: `xor shape1 shape2 ...`
- result: shape-like value

Boolean rule:

- minimum arity: one shape

### `translate`

- signature: `translate x y z shape`
- result kind follows input shape kind

### `rotate`

- signature: `rotate x y z shape`
- result kind follows input shape kind

### `scale`

- verifier accepts:
  - `scale factor shape`
  - `scale x y z shape`
- build123d lowerer supports both
- FreeCAD lowerer currently expects explicit `x y z shape`
- result kind follows input shape kind

### `mirror`

- signature: `mirror axis offset shape`
- `axis`: string or symbol naming mirror axis
- `offset`: numeric plane offset
- result kind follows input shape kind

Examples:

```scheme
(translate 20 0 0 (box 10 10 10))
(rotate 0 0 45 (box 10 10 10))
(scale 2 2 1 (circle 10))
(mirror 'x 0 (box 10 10 10))
```

## Surface and Path Signatures

### `extrude`

- signature: `extrude profile distance`
- result: `Solid`
- backend keyword:
  - `:symmetric` boolean

### `revolve`

- signature: `revolve profile angle`
- result: `Solid`

### `loft`

- signature: `loft distance profile1 profile2 ...`
- requires at least two profiles after distance
- result: `Solid`

### `sweep`

- signature: `sweep profile path`
- result: `Solid`

### `shell`

- signature: `shell thickness solid`
- result: `Solid`
- optional keyword:
  - `:faces selector`

### `offset`

- signature: `offset amount profile`
- result: `Sketch`
- optional keyword:
  - `:openings sketch-or-sketch-list`

### `offset-rounded`

- signature: `offset-rounded amount profile`
- result: `Sketch`
- optional keyword:
  - `:openings sketch-or-sketch-list`

### `fillet`

- signature: `fillet radius solid`
- result: `Solid`
- optional keyword:
  - `:edges selector`

### `chamfer`

- signature: `chamfer distance solid`
- result: `Solid`
- optional keyword:
  - `:edges selector`

### `taper`

- signature: `taper height scale profile`
- signature: `taper height scale-x scale-y profile`
- result: `Solid`
- FreeCAD caveat: non-uniform taper currently rejected

### `twist`

- signature: `twist height angle profile`
- result: `Solid`
- verifier-backed form is 3 positional args

### `path`

- signature: `path point1 point2 ...`
- signature: `path point-list`
- each point is 3D
- result: `Path`

### `polyline`

- alias of `path`

### `bezier-path`

- signature: `bezier-path point-list`
- point list must be 3D
- result: `Path`

### `bspline`

- signature: `bspline point-list`
- optional second positional in lowerers: `closed`
- optional keywords:
  - `:closed` boolean
  - `:tangents` point-list
  - `:tangent-scalars` numeric list
- result: `Sketch`

Notes:

- verifier only requires point-list first
- lowerers accept tangent hints
- tangents list may use 2 entries or one per point in build123d path

Example:

```scheme
(model
  (part latch
    (translate 0 -17 5
      (sweep
        (circle 1.4)
        (bezier-path ((-18 0 0) (-8 -8 4) (8 -8 4) (18 0 0)))))))
```

## Array and Frame Signatures

### `linear-array`

- signature: `linear-array count x y z shape`
- result: same geometry family as input

### `radial-array`

- signature: `radial-array count angle radius shape`
- result: same geometry family as input

### `grid-array`

- signature: `grid-array rows cols x y shape`
- result: same geometry family as input

### `arc-array`

- signature: `arc-array count radius start-angle end-angle shape`
- result: same geometry family as input

### `repeat`

- signature: `repeat index count expr`
- verifier recognizes form
- geometry lowerers do not currently expose dedicated authored lowering path like `repeat-union` / `repeat-compound` / `repeat-pick`

### `repeat-union`

- signature: `repeat-union index count expr`
- index must be symbol
- body should produce geometry
- result: union/fused geometry

### `repeat-compound`

- signature: `repeat-compound index count expr`
- index must be symbol
- body should produce geometry
- result: compound geometry
- build123d caveat: currently solid-only

### `repeat-pick`

- signature: `repeat-pick index count predicate expr`
- index must be symbol
- predicate decides whether current body instance is selected
- result: last matching geometry

### `for-union`

- macro alias:
  - `for-union (index count) body`
- lowers to `repeat-union`

### `for-compound`

- macro alias:
  - `for-compound (index count) body`
- lowers to `repeat-compound`

### `plane`

- signature: `plane`
- keywords:
  - `:origin (x y z)`
  - `:x (x y z)`
  - `:normal (x y z)`
- result: `Frame`

Defaults:

- origin `(0 0 0)`
- x direction `(1 0 0)`
- normal `(0 0 1)`

### `location`

- verifier signature: `location [frame]`
- authored backend-safe signature: `location frame`
- optional keywords:
  - `:offset (x y z)`
  - `:rotate (x y z)`
- result: `Frame`

### `path-frame`

- signature: `path-frame path`
- optional keywords:
  - `:at start | end | number`
  - `:up (x y z)`
- result: `Frame`

### `place`

- signature: `place frame shape`
- optional keywords:
  - `:offset (x y z)`
  - `:rotate (x y z)`
- result: placed shape

### `clip-box`

- signature: `clip-box shape`
- required keywords:
  - `:x (min max)`
  - `:y (min max)`
  - `:z (min max)`
- result: clipped shape

Example:

```scheme
(model
  (part body
    (build
      (shape rail (path (0 0 0) (20 0 10) (20 10 10)))
      (shape peg (box 4 2 6 :align '(min min min)))
      (shape frame (path-frame rail :at 0.5))
      (result (place frame peg :offset (1 2 3) :rotate (10 20 30))))))
```

## Special / Custom Operations

These are exported authored ops outside generic primitive/boolean/surface families.

### `hole`

Typed placeholder op. Use to mark missing geometry intentionally.

- signature: `hole :type kind`
- signature: `hole :type kind :goal "why this hole exists"`
- required keyword:
  - `:type`
- optional keyword:
  - `:goal`

Allowed `:type` values:

- `solid`
- `sketch`
- `path`
- `shape`

Current behavior:

- compiler accepts it as typed placeholder
- lowerers reject it until replaced with real geometry

### `compound`

- signature: `compound shape1 shape2 ...`
- groups shapes without boolean merge semantics

### `helical-ridge`

Keyword-only thread-like ridge generator.

- required keywords:
  - `:radius`
  - `:pitch`
  - `:height`
  - `:base-width`
  - `:crest-width`
  - `:depth`
- optional keywords:
  - `:female`
  - `:clearance`
  - `:lefthand`

Example:

```scheme
(helical-ridge
  :radius 10
  :pitch 2
  :height 18
  :base-width 1.2
  :crest-width 0.4
  :depth 0.7
  :female #t
  :clearance 0.15
  :lefthand #t)
```

### `sampled-radial-loft`

Procedural sampled shell / loft op.

```scheme
(sampled-radial-loft
  (theta z fz)
  :height 40
  :z-steps 6
  :theta-steps 24
  :radius expr
  :z-map expr)
```

- binder list must be exactly `(theta z fz)`
- required keywords:
  - `:height`
  - `:z-steps`
  - `:theta-steps`
  - `:radius`
- optional keyword:
  - `:z-map`

### `wall-pattern`

Pattern op applied to shell/solid target.

Pattern shape seen in repo:

```scheme
(wall-pattern
  (:mode gyroid :depth 0.6 :uFreq 4 :vFreq 5 :phase 0.2)
  shape)
```

Observed options:

- `:mode`
- `:depth`
- `:uFreq`
- `:vFreq`
- `:phase`

Observed modes:

- `gyroid`
- `cellular`
- `fbm`
- `ribs`

Backend caveat:

- build123d lowerer currently rejects `wall-pattern`
- use direct Rust/OCCT path when pattern op matters

## Selector Strings and Named Keywords

This is where people waste time guessing.

### Shared keyword value expectations

Verifier enforces:

- `:offset` -> 3D point
- `:rotate` -> 3D point
- `:origin` -> 3D point
- `:x` -> 3D point on frame ops
- `:normal` -> 3D point
- `clip-box :x/:y/:z` -> 2-item numeric list
- `:openings` -> sketch or sketch-list
- `:edges` -> edge selector payload
- `:faces` -> face selector payload

### `:align`

Supported on:

- `box`
- `sphere`
- `cylinder`
- `cone`

Example:

```scheme
(box 4 4 4 :align '(min center max))
```

Rules:

- expects 3-axis tuple
- each axis must be `min`, `center`, or `max`

### Edge selectors

Used by ops like `fillet` and `chamfer`.

Examples:

- `:edges top`
- `:edges "bottom"`
- `:edges "left+vertical"`
- `:edges "target-id:body:edge:0:0-0-0_10-0-0"`

Observed canonical meaning:

- `top` -> boundary `z max`
- `bottom` -> boundary `z min`
- `left+vertical` -> `x-min + axis-z`

### Face selectors

Used by ops like `shell`.

Examples:

- `:faces "top"`
- `:faces "planar+normal-z+area-max"`
- `:faces "target-id:body:face:5:0-0-10:100"`

### `path-frame :at`

Accepted anchor values:

- `start`
- `end`
- numeric position

## Cookbook

### Cube

```scheme
(model
  (part body
    (box 20 20 20)))
```

### Rotate a part

```scheme
(model
  (part body
    (rotate 0 0 45
      (box 20 20 10))))
```

### Box with named intermediate shapes

```scheme
(model
  (part body
    (build
      (shape outer (box 80 60 24))
      (shape cavity (translate 2 2 2 (box 76 56 22)))
      (result (difference outer cavity)))))
```

### Profile with hole

```scheme
(model
  (part body
    (extrude
      (profile :outer (circle 20 96) :holes (circle 10 96))
      10)))
```

### Repeat ribs and rollers

```scheme
(model
  (part body
    (build
      (shape ribs
        (repeat-union i 4
          (translate (* i 10) 0 0 (box 4 8 6))))
      (shape rollers
        (repeat-compound i 4
          (translate (+ (* i 10) 5) 0 0 (cylinder 2 6))))
      (result (compound ribs rollers)))))
```

### Cup from real fixture

```scheme
(model
  (part cup
    (fillet 1.47
      (union
        (shell 3
          (revolve
            (make-face
              (union
                (bspline ((30 10) (69 105)) #f
                  :tangents ((1 0.5) (0.7 1))
                  :tangent-scalars (1.75 1))
                (path (30 10 0) (40 0 0) (0 0 0) (0 105 0) (69 105 0))))
            360))
        (translate 0 0 10
          (cylinder 30 3))))))
```

## Tutorial: Loop to Profile

Sample points, close loop, extrude profile.

```scheme
(define control-points
  (map
    (lambda (angle)
      (list
        (* 26 (cos (* pi (/ angle 180.0))))
        (* 16 (sin (* pi (/ angle 180.0))))))
    (linspace 0 315 8)))

(model
  (part body
    (extrude (bspline control-points :closed #t) 10)))
```

What to notice:

- `linspace` drives repeatable sampling
- point list becomes curve
- curve becomes profile
- profile becomes solid

## Tutorial: Path to Solid

Separate motion logic from body logic.

```scheme
(model
  (part latch
    (translate 0 -17 5
      (sweep
        (circle 1.4)
        (bezier-path ((-18 0 0) (-8 -8 4) (8 -8 4) (18 0 0)))))))
```

What to notice:

- profile is tiny and stable
- path carries shape motion
- latch stays separate from any main body

## Tutorial: Repeat Logic

Author repeated geometry as structure, not copy-paste.

```scheme
(model
  (part body
    (build
      (shape ribs
        (repeat-union i 4
          (translate (* i 10) 0 0 (box 4 8 6))))
      (shape rollers
        (repeat-compound i 4
          (translate (+ (* i 10) 5) 0 0 (cylinder 2 6))))
      (shape marker
        (repeat-pick i 4 (= i 3)
          (translate (+ (* i 10) 5) 0 12 (sphere 3))))
      (result (compound ribs rollers marker)))))
```

What to notice:

- index symbol `i` becomes body-local numeric binding
- repetition lives in one source block
- final boolean/compound intent stays obvious

## Constraint Dojo

Use this section as fit/tolerance checklist when a model crosses from “looks right” into “must assemble”:

- named clearances
- relation constraints
- lower/upper bounds
- failure examples
- why anonymous offsets are garbage for physical fit

<!-- ECKY_AGENT_REFERENCE_START -->
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
<!-- ECKY_AGENT_REFERENCE_END -->
