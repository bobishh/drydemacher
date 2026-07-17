import type { DesignOutput, DesignParams } from '../types/domain';

export function resolveDraftPreviewDesign(input: {
  design: DesignOutput;
  previewThreadId?: string;
  activeThreadId?: string | null;
  currentParams?: DesignParams | null;
}): DesignOutput {
  return input.design;
}
