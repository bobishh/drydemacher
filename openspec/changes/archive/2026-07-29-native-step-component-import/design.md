## Context

This change starts only after `component-package-imports` provides:

- exact package coordinates and immutable payload digests;
- `component_import_runtime` host pre-resolution;
- `ResolvedAuthoringSource` / `ResolvedCompilation`;
- dependency lock storage in `Message.artifactBundle`;
- bundle/manifest component-import origins.

Current native OCCT supports `ImportStl` and `Solidify`, plus STEP export. Current
STEP import and joined assembly use FreeCAD helpers. Direct OCCT has writer
headers/libraries but no `ImportStep` op.

## Goals / Non-Goals

**Goals:**

- Load locked STEP bytes directly into a native OCCT shape slot.
- Preserve STEP BRep instead of tessellating then rebuilding faceted topology.
- Compose the imported shape with existing native transforms, booleans,
  topology, and export.
- Carry truthful package representation provenance and port targets.
- Reuse the existing resolver/lock/provenance seams rather than inventing a
  second package compiler path.

**Non-Goals:**

- STEP repair/healing, parameter recovery, feature-tree reconstruction, or
  editable imported history.
- STL package authoring.
- FreeCAD fallback.
- UI, registry, semver, transitive packages, or compiled/private payloads.

## Decisions

### 1. Extend the existing resolved payload enum

`component_import_runtime` gains:

```rust
pub enum ResolvedComponentPayload {
    EckySource {
        source: String,
        entry_symbol: String,
    },
    StepAsset {
        path: PathBuf,
        payload_digest: String,
        geometry_provenance: GeometryProvenance,
    },
}
```

No compiler context parameter is added. Source imports still materialize
ephemeral Ecky definitions. STEP imports materialize an internal zero-argument
shape definition whose leaf is `(import-step "<resolved-path>")`; persisted
authored source retains only package coordinate and alias.

STEP alias calls reject positional or keyword geometry arguments. Static STEP
bytes cannot truthfully respond to param overrides. Existing component
params/UI metadata remain usable by independent package UX but not authored
geometry calls.

### 2. Require package-carried representation provenance

`ComponentDefinition` and `ComponentHeader` gain optional
`geometryProvenance`. Artifact-bundle packaging copies it for STEP payloads.

Live STEP import accepts only:

- `visibility=source`;
- `.step` / `.stp` package-local sourceRef;
- payload digest matching installed inventory;
- provenance representation `analyticBrep`, `facetedPolyBrep`, or `hybrid`.

Legacy STEP packages without provenance remain independently importable through
compatibility UX but cannot become live authored components until repackaged.
File extension never proves analytic geometry.

Lock component entries extend:

```text
payloadKind: step
payloadDigest: sha256:...
geometryRepresentation: analyticBrep | facetedPolyBrep | hybrid
```

### 3. Add native `ImportStep`

Internal `(import-step path)` maps through existing custom CoreOperation
plumbing to `OcctOp::ImportStep`.

Generated native executor:

```text
STEPControl_Reader::ReadFile
  -> require IFSelect_RetDone
  -> TransferRoots
  -> require transferred root count > 0
  -> OneShape
  -> require non-null shape
  -> require solid/compsolid/compound containing solid
  -> BRepCheck_Analyzer
  -> publish shape slot
```

Missing files, read/transfer failure, null output, invalid BRep, and shell-only
payloads fail before slot publication. Generated path literals use the existing
safe C++ string escaping.

Multiple solid roots remain a compound. No automatic fuse occurs because that
would change STEP product/part structure.

### 4. Never route through FreeCAD or solidify

Primary runtime mapping:

```text
STEP -> import-step             -> declared BRep representation
STL  -> import-stl -> solidify  -> facetedPolyBrep
```

`solidify` exists because `StlAPI_Reader` yields a face compound. STEP transfer
already yields BRep topology. Calling `solidify` would be hidden repair and may
rebuild topology.

No failure path invokes `freecad::import_step` or
`freecad::assemble_step_parts`. Invalid STEP returns the native error. A future
explicit repair operation may opt into healing.

### 5. Merge provenance conservatively

Runtime output representation:

- analytic authored + analytic imported → `analyticBrep`;
- faceted-only imported placement → `facetedPolyBrep`;
- analytic plus faceted/mixed imported → `hybrid`;
- any `hybrid` contributor → `hybrid`.

Bundle, manifest, primary STEP export, dependency lock, and component-origin
evidence must agree. Cache identity already includes dependency lock digest
from the prerequisite change.

### 6. Reuse native topology reporting

Imported shape slots traverse the existing edge/face topology reporter.
Component origin evidence associates topology part/node ids with canonical
package identity. Port target validation uses Direct OCCT targets; it does not
ask FreeCAD to regenerate topology.

`STEPControl_Reader.hxx` becomes a required native SDK header. Existing
`TKSTEP`, `TKSTEPBase`, `TKSTEPAttr`, and `TKXSBase` libraries already used for
STEP writing remain linkage authority; SDK probe tests prove reader linkage.

## Risks / Trade-offs

- [STEP transfers assemblies/shells, not one solid] → Accept solid-containing
  compounds; reject shell-only payloads without implicit healing.
- [Legacy STEP provenance unknown] → Require repackaging for live reference.
- [Resolved absolute path leaks] → Keep path only in ephemeral compiler source
  and native plan; persisted source/origin stores coordinate plus digest.
- [Direct runtime currently defaults to analytic provenance] → Merge explicit
  contributor provenance before bundle/manifest publication.
- [Port ids drift across STEP readers] → Validate package targets against
  locked Direct OCCT topology evidence, never compatibility-runner ids.

## Migration Plan

1. Require `component-package-imports` tests green.
2. Add package STEP provenance fields and payload admission tests.
3. Add native `ImportStep` executor tests.
4. Extend resolved payload lowering and end-to-end render.
5. Enable live STEP references after provenance/topology gates pass.

Rollback disables STEP payload admission. Source live references remain.
Compatibility STEP import remains unchanged.

## Open Questions

None. Explicit STEP repair belongs to a later change.
