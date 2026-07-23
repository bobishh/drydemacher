export function shouldApplyDraftPreviewToWorkspace(input: {
  activeThreadId: string | null;
  previewThreadId: string;
}): boolean {
  return input.activeThreadId === input.previewThreadId;
}
