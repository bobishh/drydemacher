# Design: Intent-level raster extrusion and protrusion

## Public contract

The operator states geometric intent. The source representation selects an internal
decoder/lowerer.

```scheme
(extrude sketch height [:symmetric #t|#f])

(extrude image-path height
  [:width physical-width]
  [:depth physical-depth]
  [:fit contain|stretch]
  :threshold 0.5
  :foreground dark
  [:symmetric #t|#f])

(protrude image-path height
  [:width physical-width]
  [:depth physical-depth]
  [:fit contain|stretch]
  :foreground dark)
```

Raster calls require at least one physical dimension. One supplied dimension derives
the other from source pixel aspect ratio. Two supplied dimensions define a containing
box: `contain` is the default and centers uniformly scaled geometry; `stretch` is the
explicit non-uniform option. Raster `cover` remains out of scope until geometry
clipping has a matching deterministic contract.

Raster `extrude` computes coverage, thresholds it, traces closed planar regions, then
uses the normal sketch extrusion path. Raster `protrude` samples continuous coverage
as height and lowers through the internal planar heightfield mesh builder. Width and
depth are explicit; the language never silently stretches from an inferred physical
calibration.

## Coverage and polarity

The decoder preserves alpha. Alpha means material coverage and is never inverted:

```text
dark foreground:  coverage = alpha * (1 - luminance)
light foreground: coverage = alpha * luminance
```

`alpha=0` contributes no profile and no relief. `extrude` classifies coverage at the
explicit threshold; default threshold is `0.5`. `protrude` uses continuous coverage.
Default foreground is `dark`; light-on-dark artwork opts into `:foreground light`.

## Protrusion closure

`protrude` establishes local `Z=0` as the target/base plane. Positive coverage rises
above that plane. Internal watertight closure may extend by a bounded epsilon below
the plane so a translated protrusion overlaps its target, but no rectangular carrier
may rise above `Z=0`. The closure thickness is backend-owned and absent from public
syntax.

## Core and backend ownership

- `extrude` remains a surface operation with a typed sketch-or-image first argument.
- `protrude` is a surface operation returning a solid/mesh.
- Raster extrusion and protrusion partition as mesh-derived geometry for backends that
  cannot consume the raster-derived profile directly.
- The existing heightfield builder becomes an internal protrusion lowerer.
- Legacy `heightfield` remains accepted by parser/runtime tests only. It is omitted
  from portable operation manifests, agent prompts, generated public references, and
  editor completions.

### Mixed Boolean execution

Closed raster-extrusion meshes used by `union`, `difference`, or `intersection`
remain indexed in memory. Operand position and Boolean arity do not force an STL
round trip: when every consumer path reaches the part root through transforms or
Boolean operations, the native runner converts analytic peers once and evaluates the
mixed closure in the Manifold domain. N-ary union uses one batch Boolean. A
non-Boolean topology consumer still blocks this route and retains the explicit OCCT
solidification fallback.

Raster contour extrusion resolves diagonal pixel contacts without changing the
threshold mask into a filled morphology: repeated grid corners receive a bounded
one-eighth-pixel bevel. Caps use constrained Delaunay triangulation with authored
outer and hole rings as hard edges. Together these rules make the pre-Boolean raster
asset survive the canonical evaluated-mesh weld while remaining closed, consistently wound, and
admissible to Manifold.

## Asset and UI behavior

Empty image parameters referenced by `extrude` or `protrude` remain pending and block
render. Decode and tracing failures preserve raw path/decoder evidence and the last
good artifact. Frontend detection becomes operation-neutral image-geometry detection.

## Migration

Repository-authored `.ecky` models, missions, examples, and editor fixtures migrate:

- binary/logo images -> `extrude image-path ...`
- continuous luminance relief -> `protrude image-path ...`

Legacy syntax remains only in explicit compatibility tests and archived historical
OpenSpec records. No migration rewrites user files automatically.

## Rejected paths

- Keep `heightfield` public: exposes implementation and carrier thickness.
- Infer mask versus relief from pixel distribution: antialiasing and real grayscale
  are indistinguishable.
- Invert alpha with foreground polarity: transparent pixels must always mean absence.
- Remove legacy parsing immediately: breaks durable thread/model history.
