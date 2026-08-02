---
id: mission-01-bracket-enclosure
title: Make a bracket close an enclosure
---

# Make a bracket close an enclosure

Build relations before details. This mission starts with two detached solids,
then carries the same named dimensions into two separately printable enclosure
parts and one bounded closure choice.

## Join the corner bracket {#worked-bracket}

The foot sits on the bed. The flange begins at `X=60`, deliberately detached,
so placement is visible in the bundled preview. Move that translation to `X=0`,
change `compound` to `union`, then render your edit. `compound` merely groups
solids; `union` makes overlapping solids one load-bearing bracket.

## Inspect the connected bracket {#bracket-solution}

The flange now overlaps the foot by `stock_t`. The same named stock controls the
contact and the plate thickness. A union is appropriate only after that physical
overlap exists.

## Read the modeling scaffold {#build-forms}

`params` declares adjustable inputs. `let*` names local derived dimensions such
as `flange_h` and `overlap`. Inside `build`, each `shape` gives an intermediate
solid a readable name, and `result` selects the final solid. Read this scaffold
before adding a second printable part: it makes a relation inspectable instead
of hiding it in copied offsets.

## Separate body from lid {#enclosure-shell}

The enclosure uses two `part` forms because body and lid print, move, and wear
independently. Both consume the same case dimensions. The body first defines an
outer box, then removes a cavity with `difference`; the lid remains a separate
plate at `body_h`. No joint decision is needed yet.

## Switch one interface between snap and bolt {#joint-branch}

This small male/female coupon names `fit_clearance` once. `if` chooses between
two complete solids: snap tab plus slot, or bolt boss plus bore. That boundary
matters: pass a finished solid from each branch, never the name of a prior
`shape`. The female cut uses the same clearance. Change `joint_type`, render,
then compare both assemblies before asking the enclosure to make the same
choice.

## Choose a closure {#joint}

The starter is a complete snap enclosure, so it renders before any edit. Replace
each marked fixed snap block with the same complete-solid `if` pattern from the
coupon: snap geometry for `snap`, bolt geometry for `bolted`; retain optional
countersinks inside the bolted branch. Keep body and lid fixed as separate
parts. No new joint pattern. No unnamed fit offset.

## Reveal the configurable enclosure {#configurable-enclosure}

The finished body adds snap hooks or bored bolt bosses. The finished lid cuts
the matching slots or bores and may add recesses. `fit_clearance` expands only
the receiving geometry; named case dimensions place both corners. Either
closure remains one readable interface, not two unrelated models.

## Tune hardware after the structure {#print-choice}

Use slots for installation adjustment and threads for a defined fastener. Cut
only where remaining wall can carry load. Test fit in a small coupon before
changing a full enclosure.

## Finish the printable bracket {#finish}

Mounting slots, ribs, and lightening cuts are finish details. They come after
the foot-to-flange load path and enclosure interface are correct. Inspect each
detail as a consequence of named stock and clearance, not as a substitute for
them.
