---
id: mission-02-bottle-cage-dovetail
title: Mount a bottle cage without support
---

# Mount a bottle cage without support

Build a removable bottle cage as two independent prints. First derive a clamp
from the bicycle tube. Then make a small male/female rail coupon. Finally use
the same interface in the cage and mount. The print rule is concrete: every
new roof layer needs material beneath it, so a flat roof becomes a peaked one.

## Fit is a named relation {#fit-first}

Clearance belongs to both mating profiles. Name it before shaping either rail.
The male profile is nominal. The female profile grows by clearance, not by an
anonymous offset hidden in a cutter.
## Make the frame interface {#clamp}

The clamp derives from measured tube diameter. `frame_dia` controls its inside
radius; `clip_t` controls material outside that measurement. The opening makes
the clamp printable and lets it flex onto the tube. Change diameter once, then
read the derived radii before rendering.
## Remove the unsupported roof {#roof}

The starter rail has a flat roof. Replace its roof cutter with the peaked cutter
from the next step. Preserve base width, top width, height, and length: only
the roof changes. The acceptance compares Core IR, not source spelling.
## Reveal the peaked rail {#peaked-roof}

The rail coupon makes the fit visible. `papa` is nominal. `mama` derives width
and height from `fit_clearance`, then adds a roof rise. Print both short coupons
flat, slide them by hand, and adjust clearance only after observing the actual
material and printer.
## Use the complete mount {#full-cage}

Review clamp and cage as separately printable parts. The mount owns a male rail
and frame clamp; the cage owns a female channel. Their shared names—base width,
rail height, rail length, and clearance—are the contract. Test rail engagement
without a bottle, then test retention under intended load. Geometry matching is
not evidence of safe ride vibration behavior.
