import { derived, get } from 'svelte/store';
import {
  normalizeDesignParams,
  normalizeUiSpec,
  type DesignOutput,
  type DesignParams,
  type UiSpec,
} from '../types/domain';
import { workingCopy } from './workingCopy';

export type ParamPanelStateSnapshot = {
  versionId: string | null;
  uiSpec: UiSpec;
  params: DesignParams;
};

const projection = derived(workingCopy, (copy): ParamPanelStateSnapshot => ({
  versionId: copy.sourceVersionId,
  uiSpec: copy.uiSpec,
  params: copy.params,
}));

function hydratePanel(payload: ParamPanelStateSnapshot) {
  const current = get(workingCopy);
  workingCopy.patch({
    sourceVersionId: payload.versionId,
    uiSpec: normalizeUiSpec(payload.uiSpec),
    params: normalizeDesignParams(payload.params),
    dirty: current.dirty,
  });
}

/** Compatibility facade. Working copy owns parameter values and UI schema. */
export const paramPanelState = {
  subscribe: projection.subscribe,

  reset() {
    hydratePanel({ versionId: null, uiSpec: { fields: [] }, params: {} });
  },

  hydrate(payload: {
    versionId?: string | null;
    uiSpec?: UiSpec;
    params?: DesignParams;
  }) {
    hydratePanel({
      versionId: payload.versionId ?? null,
      uiSpec: normalizeUiSpec(payload.uiSpec),
      params: normalizeDesignParams(payload.params),
    });
  },

  hydrateFromVersion(design: DesignOutput | null | undefined, versionId: string | null) {
    hydratePanel({
      versionId,
      uiSpec: normalizeUiSpec(design?.uiSpec),
      params: normalizeDesignParams(design?.initialParams),
    });
  },

  setVersionId(versionId: string | null) {
    const current = get(workingCopy);
    workingCopy.patch({ sourceVersionId: versionId, dirty: current.dirty });
  },

  setUiSpec(uiSpec: UiSpec) {
    workingCopy.patch({ uiSpec: normalizeUiSpec(uiSpec) });
  },

  setParams(params: DesignParams) {
    workingCopy.patch({ params: normalizeDesignParams(params) });
  },

  patchParams(partialParams: DesignParams) {
    const current = get(workingCopy);
    workingCopy.patch({
      params: { ...current.params, ...normalizeDesignParams(partialParams) },
    });
  },
};
