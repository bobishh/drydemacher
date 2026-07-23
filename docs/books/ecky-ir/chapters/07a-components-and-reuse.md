## Components and Reuse: Lift a Proven Part

Use a component when geometry must be reused across parts or models. A component packages a closed parameter signature, its geometry, and verification clauses; each instance reuses that definition without copying source.

This component defines a bored mounting standoff and carries its minimum-wall check:

```scheme
(define-component standoff
  ((number height 12 :label "Standoff height" :min 6 :max 30)
   (number bore 3.2))
  (verify (tag bore_open) (metric min_wall_thickness "body") (expect (>= value 1.2)))
  (difference
    (cylinder 6 height 96)
    (cylinder bore (+ height 2) 96)))

(model
  (part front_left (standoff :height 16))
  (part rear_right (translate 40 0 0 (standoff))))
```

Three rules define component behavior.

**Reuse by reference, override by keyword.** `(standoff :height 16)` instantiates the component and overrides one signature key; `(standoff)` takes every default. Omitted keys fall back to the signature, and a missing _required_ key (one with no default) is a compile error that names the component and lists its signature. There is no copy-paste, so there is no drift: change the body once and both parts move together.

**Closedness makes reuse reliable.** A component body sees only signature keys and local bindings (`let`, `let*`, repeat indices, and build shapes). Referencing a model parameter or outer binding is a compile error. Therefore the component can be copied into another model without hidden dependencies.

**Verification expands per instance.** The component's `verify` clause is namespaced by part key, producing tags such as `front_left/bore_open` and `rear_right/bore_open`. Every instance runs the same wall-thickness requirement.

For the exact signature grammar, nesting limits, and verify-travel rules, see **`define-component`** in the language reference appendix.

### The library loop (MCP)

Components do not have to live in one file. Agents lift proven parts into a shared library and pull them back by source:

1. `component_extract` — hand it a model and a `partKey`. Referenced model params become the signature (metadata preserved); scalar outer bindings become plain defaults; any non-scalar free reference is reported as a blocker so you cannot extract something that secretly depends on its context. `save: true` stores it.
2. `component_search` — compact headers only (name, one-liner, param keys, tags). Bodies never come back from search, so the library stays browsable.
3. `component_get` — the full, self-contained `define-component` source for one name. Paste it into the model and instantiate.

The loop is copy-inline by design: what you get back is closed source, not a hidden registry link. A part proven in one project becomes a building block in the next, checks and all.
