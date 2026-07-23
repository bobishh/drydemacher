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
