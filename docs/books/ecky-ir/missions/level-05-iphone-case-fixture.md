---
id: mission-05-iphone-case-fixture
title: Read a three-print phone case
---

# Read a three-print phone case

This mission is a manufacturing read-through, not a request to redraw a phone
from scratch. Follow one shared camera datum from the TPU shell into the two
PETG pieces. The final source is the current composite camera-frame model; do
not substitute an older open-lattice or single-piece case.

## One design, three printable parts {#materials}

One assembly produces three exports: the flexible TPU case, a phone-side PETG
inner island, and a camera-side PETG snap island. “Two-piece camera frame”
means the two PETG pieces cooperate at one camera cluster; it does not mean the
TPU shell disappears. Keep material jobs separate: TPU supplies enclosure and
capture compliance, while PETG supplies the rigid, flush camera frame.

## Trace the TPU envelope {#phone-shell}

Start with the pocket rather than the decorative rear surface. Phone width,
length, thickness, corner radius, pocket clearance, wall thickness, and rear
panel thickness define the shell contract. Port and button cutters are then
subtracted from that positive body. When a fit changes, change the named
clearance or wall control; do not move a cutter by an unexplained offset.

The current source also cuts the camera-frame seats from this TPU part. Those
seats use the same measured camera, microphone, and flash datums as the PETG
pieces. That shared datum is the relation to inspect before changing a
dimension.

## Distinguish authored from generated pattern {#lattice}

The rear treatment is a positive edge lattice: struts are explicit geometry
joined to the shell, not holes pretending to be a Voronoi algorithm. Read the
`lattice-strut` component as a reusable segment between two declared endpoints.
The finished case may contain many such authored edges, but the source makes no
claim that it can regenerate a new Voronoi graph for another phone.

Treat this as a boundary: changing the shell envelope can preserve the frame
and fit relations, while changing the authored lattice requires new authored
edge data and a printability review.

## Place the PETG islands {#islands}

The inner and outer PETG islands are the two parts of the composite camera
frame. Both are built from the same three lobe centers: camera, microphone, and
flash. The inner island provides the phone-side seat and a female capture
groove. The outer island provides the camera opening, hidden snap skirt, bead,
and repeated relief slots. The microphone and flash remain open through both
pieces.

`lens-clamp-fit-clearance`, capture thickness, snap engagement depth, and snap
interference are named fit controls. They are not cosmetic knobs: changing one
requires inspecting the matching TPU cut and the opposing PETG feature.

## Inspect the complete assembly {#case-solution}

The final source is the current three-export model: one TPU shell plus the two
PETG camera-frame pieces. Its preview positions the PETG pieces beside the shell
so their separate printable roles stay visible; that is an inspection layout,
not a claim that they print as one merged solid. The camera frame itself is a
composite of two mating pieces, not the old simplified one-piece camera pad.

Check the flush-stack relation before trusting the view: captured TPU plus the
inner PETG thickness equals the rear-panel thickness. Then check that the snap
through-cut, inner groove, and outer skirt use their named clearance and
interference values consistently.

## Make a coupon before committing {#fit-warning}

Print a small camera-frame coupon before the full case. Include a short piece
of TPU seat, the inner PETG groove, and the outer PETG snap skirt. Test the
actual filament pair, nozzle, layer height, and slicer settings. The model
preserves the relation; the coupon decides whether the chosen clearance and
interference are usable on this printer.
