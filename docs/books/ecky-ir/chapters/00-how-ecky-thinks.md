## How Ecky Thinks

Ecky has three layers. Knowing their boundaries makes compiler and renderer errors easier to diagnose.

**Surface language.** You write parenthesized `.ecky` forms such as `(model (part ...))`. This syntax describes authoring intent; the renderer does not execute it directly.

**Core IR.** The compiler lowers surface forms into a fixed vocabulary of primitives, booleans, selectors, placements, repeats, and typed mesh operations. The kernel receives this finite data model, not arbitrary Scheme. That boundary makes models reproducible and statically checkable.

**Geometry runtime.** Exact solids render on the native **OCCT** B-rep kernel. Typed polygon data renders in the bounded Rust mesh runtime; a closed mesh crosses into OCCT only through the explicit faceted poly-BRep bridge. **build123d** and **FreeCAD** are supported interop backends with smaller operation sets.

Classify a failure before changing geometry: surface syntax, Core IR validation, or backend support. Diagnostics name that boundary whenever possible.
