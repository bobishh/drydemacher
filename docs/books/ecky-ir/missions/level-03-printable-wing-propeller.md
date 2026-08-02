---
id: mission-03-wing-propeller-study
title: Drive form from stations and parameters
---

# Drive form from stations and parameters

This is a geometry study, not flight hardware. Learn how one profile becomes a controllable form, then reuse that form around a hub.

## A station is a contract {#stations}

A station is one cross-section plus its position and orientation. The wing needs a root station, a tip station, span, taper, and twist. Name those decisions once; do not hand-draw a second almost-identical profile whenever the wing changes.

## Loft the first wing {#wing-worked-stations}

Read the three stages in the source: root profile, tip profile, then `loft`. `root_chord` controls the first section. `tip_chord` controls the second. `twist` rotates only the tip before the loft joins both profiles across `span`.

Render the supplied model. Change `span` from `90` to `120`, render again, and identify the only dimension that should lengthen. Then restore it before continuing.

## Derive the tip station {#wing-tip}

The starter has a hard-coded tip: `26`, `2.6`, `9`, and `-1.2`. Replace those literals with two names: derived tip chord and derived tip thickness. The chord must come from `root_chord * taper_ratio`; thickness comes from that derived chord. Keep the existing rotation, because twist is a transform of the finished tip profile, not a number baked into its points.

Render after each small edit. Check Solution compares the lowered Ecky IR, so equivalent geometry passes even if variable names or whitespace differ.

## Reveal taper and twist {#wing-solution}

The solution adds `taper_ratio` to the parameter block and derives both tip dimensions in `let*`. Compare it with your source. `tip_chord` remains a useful measurement, but the actual profile now follows one relation. Change taper from `0.6` to `0.45` and see the tip shrink without redrawing anything.

## Repeat one blade from controls {#propeller-worked}

The propeller uses one lofted blade and `repeat-union`. `blade_count` selects copies by rotating the same blade through `360 / blade_count`. `diameter` and `hub_radius` define usable blade length; pitch and twist alter the two station profiles.

Render once. Change `blade_count` from `3` to `4`, render, and verify there are four evenly spaced copies rather than four separate blade definitions.

## Finish the hub variant {#hub-variant}

The starter already builds blades and a press-fit hub. Add a second named hub for `split_bolt`: a named split gap, named bolt-bore radius, clamp ears, bore, and slot. Then use `if` to select `split_bolt_hub` only when `hub_type` is `"split_bolt"`.

Do not change blade geometry. This task is one interface choice around a fixed repeated assembly. The bore clearance remains named because it is fit-critical.

## Reveal the repeated propeller study {#propeller-solution}

The solution keeps one blade definition and adds two hub choices. It is a printable geometry study only. It does not prove balance, fatigue life, thrust, RPM limit, motor compatibility, or airworthiness. Treat it as an exercise in loft, repeat, named fit, and conditional composition.
