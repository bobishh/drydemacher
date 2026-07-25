import type { DesignOutput } from './types/domain';
import type { WorkingCopyState } from './stores/workingCopy';

type ManualDraftSeedInput = Pick<
  WorkingCopyState,
  | 'title'
  | 'versionName'
  | 'macroDialect'
  | 'engineKind'
  | 'sourceLanguage'
  | 'geometryBackend'
  | 'uiSpec'
  | 'params'
  | 'postProcessing'
>;

export function buildFailedDraftSeed(
  failedDesign: DesignOutput,
  workingDraft: ManualDraftSeedInput,
): DesignOutput {
  const failedUiSpec = failedDesign.uiSpec;
  const failedParams = failedDesign.initialParams;

  return {
    ...failedDesign,
    title: failedDesign.title || workingDraft.title || 'Manual Edit',
    versionName: failedDesign.versionName || workingDraft.versionName || 'Draft',
    macroDialect: failedDesign.macroDialect ?? workingDraft.macroDialect,
    engineKind: failedDesign.engineKind ?? workingDraft.engineKind,
    sourceLanguage: failedDesign.sourceLanguage ?? workingDraft.sourceLanguage,
    geometryBackend: failedDesign.geometryBackend ?? workingDraft.geometryBackend,
    uiSpec:
      failedUiSpec?.fields.length
        ? failedUiSpec
        : workingDraft.uiSpec,
    initialParams:
      failedParams && Object.keys(failedParams).length > 0
        ? failedParams
        : workingDraft.params,
    postProcessing: failedDesign.postProcessing ?? workingDraft.postProcessing ?? null,
  };
}
