export type ViewerTopologyMaterialization = {
  materialize: boolean;
  reason: 'targetBudgetExceeded' | null;
};

// Per-target Three.js objects are useful for small analytic models. Faceted
// imported BReps can expose tens of thousands of targets; those stay in the
// manifest/query layer instead of becoming one scene object each.
const MAX_MATERIALIZED_TOPOLOGY_TARGETS = 1_000;

export function materializeViewerTopology(
  edgeTargetCount: number,
  faceTargetCount: number,
): ViewerTopologyMaterialization {
  if (edgeTargetCount + faceTargetCount > MAX_MATERIALIZED_TOPOLOGY_TARGETS) {
    return { materialize: false, reason: 'targetBudgetExceeded' };
  }
  return { materialize: true, reason: null };
}
