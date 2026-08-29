## Why

Ecky already computes parameter dependencies from Core IR, but manifest lowering
replaces every part dependency set with the complete model parameter list. A large
macro therefore renders one flat control dump and makes every part appear to own every
knob. The filament dryer reproduces this defect with 49 parameters, 8 parts, and 106
named build shapes.

STEP cannot recover authoring intent. Ecky AST and direct-runtime authored topology
bindings are the only deterministic provenance sources available for generated models.

## What Changes

- Preserve inferred Core IR parameter dependencies on each generated Ecky part instead
  of assigning all model parameters to every part.
- Derive stable parameter groups from model-level parameters, parts, explicit features,
  and reachable named `build/shape` stages. Never generate or persist Ecky
  `controlViews` for this purpose.
- Carry direct-OCCT authored shape bindings into selection targets so an exact tagged or
  authored topology hit can expose its narrow parameter set. Mesh-only output falls
  back to part ownership.
- Render deterministic ownership sections directly below Params search. Keep dense or
  unrelated sections collapsed instead of presenting one flat list.
- Make viewport Select mode reveal and edit only controls linked to the selected target,
  shape, feature, or part. Ambiguous targets never fall back to all model parameters.
- Add authoring guidance for stable part, feature, shape, and interaction-critical
  topology names. Compiler inference remains authoritative; prompt compliance is not.
- Apply settled project-folder edits within two seconds instead of waiting for a
  minute-scale poll.
- Split direct-OCCT geometry and semantic cache identity: tag-only edits reuse the
  existing BRep/topology and rebuild only tags plus manifest. Authored bindings do not
  disable geometry reuse.
- Bind project-card thumbnails to the current head. Never label an older raster preview
  with a newer version timestamp, and do not reject intentional separated print layout
  as a failed source apply.
- Rebuild and verify the bound large filament-dryer macro through MCP-first authoring.

## Capabilities

### New Capabilities

- `ast-control-provenance`: deterministic parameter ownership and grouping from Ecky
  Core IR through runtime manifests.
- `project-sync-performance`: bounded project-folder latency, semantic-only native
  rerenders, and head-accurate project preview state.

### Modified Capabilities

- `workbench-viewport`: exact generated-model selection may open scoped controls; the
  previous blanket overlay prohibition is replaced by provenance-gated behavior.

## Impact

- Backend: Ecky native and direct-OCCT manifest lowering, topology report decoding,
  feature graph/group derivation, geometry/semantic cache boundaries, filesystem watch,
  verification policy, manifest validation tests.
- Frontend: Params ownership sections, selected-scope expansion, viewport overlay gate,
  AST map projection tests, head-accurate project-card preview state.
- Authoring: Ecky prompt/card guidance; no new authored `Views` data.
- Product proof: current bound filament-dryer target and real workbench route.
- No STEP semantic inference, FreeCAD control-view change, direct SQLite write, or
  source-control checkpoint.
