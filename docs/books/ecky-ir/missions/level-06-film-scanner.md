---
id: mission-06-film-scanner
title: Keep a film scanner adjustable
---

# Keep a film scanner adjustable

The scanner is an assembly of printable parts, not a single hero mesh. Rail,
channel, film insert, tunnel, cover, and moving lens carrier meet at named
interfaces. Read each source as a contract between parts before treating it as
a finished object.

## The interfaces are the model {#interfaces}

Three relations govern this mission. `fit_clearance` expands a female rail
channel around its male rail. The film-format branch derives aperture dimensions
from one selected value. `thread_clearance` separates the helicoid ridge from
its matching socket. Keep those names at the point where the two features are
derived; copying a number into each part makes later adjustment unsafe.

## Make the rail and channel agree {#rail-fit}

The first source is a small two-part coupon. A triangular rail profile is
extruded into the base part. The tunnel part derives `channel_h` and `channel_w`
by adding twice the shared clearance before extruding the matching cutter.
This is deliberately smaller than the final scanner: it lets you inspect one
male/female relation without confusing it with film format or focus motion.

Change rail tip width, rail height, or clearance only through its named
parameter. A changed rail with an unchanged channel is a broken interface even
if each part still renders.

## Branch by film format {#format}

`film_format` is one `select` control. Nested `if` expressions derive a frame
width and height for 135, 120 6×4.5, and 120 6×9; the insert blank and aperture
cut then consume those derived values. There is no copied 135 insert or copied
120 insert. This keeps the aperture centered on the same rail and tunnel datum.

The worked scanner subassembly expands that pattern with insert clearance and a
base plate. Read the branch first, then follow `frame_w` and `frame_h` into the
two cutter dimensions. Do not change a downstream box dimension to make a
format fit: correct the branch relation instead.

## Inspect the final scanner {#scanner-final}

The final scanner retains independently printable base rails, lower guides,
upper clamp, tunnel, top cover, and moving lens carrier. The film insert is a
two-piece stack: lower guides carry the supportless male rails; the upper clamp
derives the matching female channels with named join clearance. The base/tunnel
and top-cover rails use the same clearance relation introduced in the coupon.

The helicoid is also a pair. The top cover cuts its socket with a female
`helical-ridge`; the carrier fuses matching male ridges, both using pitch,
depth, and thread clearance derived from the same controls. The preview proves
part placement and branch structure only. It cannot prove a smooth sliding rail
or a turning thread.

## Calibrate the moving interface {#coupon}

The calibration source is a bounded mechanism study. It exposes one female
helicoid ridge, a central bore, repeated radial stops, and a sampled focus knob.
`repeat-union` makes stop geometry; `repeat-compound` keeps witness ticks
grouped without fusing them. The `common`, `intersection`, and `xor` shapes are
comparison witnesses, not extra scanner parts.

Start with pitch and thread clearance. Print a short threaded sample and turn it
through a few stops before changing the full carrier. Do not infer a production
fit from a preview or from a Boolean that happens to compile.

## Print fit before finish {#print}

Print two small coupons before the complete scanner: a rail/channel section and
a short helicoid/socket section. Use the intended material, nozzle, layer
height, and orientation. Record the clearance that slides and the thread
clearance that turns, then set those named controls in the full assembly. This
is the handoff from geometric intent to a machine-specific fit.
