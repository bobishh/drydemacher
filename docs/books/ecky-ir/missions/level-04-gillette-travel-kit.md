---
id: mission-04-gillette-travel-kit
title: Pack a razor into three printed parts
---

# Pack a razor into three printed parts

Build a real assembly: base holds the handle and blade box, lid slides on rails, blade cover retains one consumable. Three printed parts means three interfaces to reason about.

## Parts have different jobs {#separate-parts}

The base carries the handle clips and blade box. The lid protects the assembly and slides over rails. The blade cover is a small removable retention part. Keep them separate: each has different movement, wear, print orientation, and fit tolerance.

## Make the shell {#shell}

Start with outer shell, then overshooting cavity. `wall` sets side thickness; `floor` sets the bottom. The cavity starts at `floor` and deliberately extends beyond the top, so the subtraction cannot leave a skin over the opening.

Render the supplied shell. Change `wall` from `3` to `2.4`, render, and inspect that the external envelope stays fixed while only the cavity grows. Restore it before continuing.

## Complete the cover detents {#detents}

The blade cover needs two identical pockets. `detent_engagement` is one named decision: it moves both pockets equally from the cover edges. Keep `pocket_radius` derived from `detent_radius` plus clearance; do not type two unrelated pocket positions.

Render the starter. Change engagement from `0.2` to `0.3` and identify both pockets moving together. Check Solution compares lowered Ecky IR, not source text.

## Reveal the finished kit {#kit-solution}

The finished kit joins the airy base, rail lid, and blade cover as separate printable parts. Read each `part` boundary first; only then inspect detents, handle snaps, and dovetail rails. This prevents an assembly from turning into one unprintable solid.

## Study snap and rail fit {#fit-coupon}

Use this handle clip as a coupon before printing the full kit. `snap_clearance` controls clip bore; `clip_wall` controls spring material; `clip_width` controls contact length. Test one short coupon with your filament and orientation, then carry measured clearance into the full model. A nominal number is not a physical guarantee.
