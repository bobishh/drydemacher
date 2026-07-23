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
