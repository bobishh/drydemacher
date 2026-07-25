## Level 03: Parametric Pattern

**Mission:** Replace copied geometry with one repeated rule.

**Clear condition:** Changing count or pitch moves the whole pattern without editing shape blocks.

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
