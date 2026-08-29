export type ViewerBusyPhase = 'generating' | 'repairing' | 'rendering' | 'committing' | null;

type ViewerBusyState = {
  showViewerBusyMask: boolean;
  viewerBusyPhase: ViewerBusyPhase;
  viewerBusyText: string | null;
};

export function deriveViewerBusyState(input: { geometryRenderActive: boolean }): ViewerBusyState {
  return input.geometryRenderActive
    ? {
        showViewerBusyMask: true,
        viewerBusyPhase: 'rendering',
        viewerBusyText: 'Rendering geometry.',
      }
    : {
        showViewerBusyMask: false,
        viewerBusyPhase: null,
        viewerBusyText: null,
      };
}
