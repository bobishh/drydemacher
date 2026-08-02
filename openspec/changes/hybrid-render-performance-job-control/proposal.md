# Proposal: Hybrid Render Performance and Job Control

## Why

Reduce latency and memory cost of dense hybrid renders while adding explicit
progress, cancellation, deterministic reuse, and representation-aware kernel
routing.

## Problem

The Poly BRep bridge is functionally complete, but dense imported or generated
meshes can still trigger expensive faceted-BRep conversion, repeated kernel
execution, oversized preview payloads, and uninterruptible child processes.
Those runtime concerns are independent from correctness of the bridge itself.

## What Changes

- Benchmark dense imported-mesh workloads by fixed render stages.
- Preserve indexed manifold meshes until an operation or export contract
  requires BRep conversion.
- Optimize OCCT Boolean planning without changing authored semantics.
- Reuse verified immutable artifacts and coalesce identical concurrent work.
- Add typed progress and subscriber-aware cancellation for kernel jobs.
- Bound preview payload construction and dense topology exposure.
- Consider explicit decoration-only simplification only after benchmarks prove
  it is needed.

## Out of Scope

- Poly BRep partition, `import-stl`, `solidify`, hybrid dispatch, or STEP
  correctness. Those belong to the completed `poly-brep-bridge` change.
- Product-specific geometry acceptance. A benchmark mesh is load data only.
- Silent geometry simplification or kernel fallback.
- Global fuzzy tolerances or glue settings without a named policy.

## Proof Gates

- Cold dense-mesh render meets a recorded threshold.
- Warm identical render performs no kernel execution.
- Concurrent identical renders execute one kernel job.
- Cancellation leaves no orphan process or partial cache entry.
- Output topology, bounds, volume, and configured deviation remain within
  fixture tolerances.
- Disk-cache and process-hot preview responses meet payload timing budgets.
