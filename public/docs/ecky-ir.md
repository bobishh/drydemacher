# Ecky Language Reference

Dry syntax, signatures, selectors, backend support, and verification grammar.
For project-driven learning, use the campaign under `/learn/ecky-ir`.

Use this section after the lessons when you need exact forms, signatures, helper names, selector strings, and verification grammar. The reference is intentionally dense; the earlier chapters show when each piece matters.

## Operation Index

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
