import type { ArtifactBundle } from '../types/domain';
import {
  checkCampaignStep,
  getCampaignStep,
  listCampaignDefinitions,
} from '../tauri/client';
import { formatBackendError } from '../tauri/client';

/**
 * Backend-owned campaign projection. The desktop shell receives one current
 * step only; it must never load campaign Markdown, source files, or preview
 * assets itself.
 */
export type CampaignStepKind = 'explain' | 'worked' | 'challenge' | 'solution';

export type CampaignCanonicalPreview = {
  canonicalSourceDigest: string;
  runtimeDigest: string;
  artifactBundle: ArtifactBundle;
};

export type CampaignCurrentStep = {
  id: string;
  title: string;
  kind: CampaignStepKind;
  prose: string;
  source: string | null;
  canonicalSourceDigest: string | null;
  canonicalPreview: CampaignCanonicalPreview | null;
  acceptance: { mode: 'equivalentCoreIr'; referenceStepId: string } | null;
  nextStepId: string | null;
  previousStep: { id: string } | null;
  missionIndex: number;
  missionCount: number;
  stepIndex: number;
  stepCount: number;
};

export type CampaignCurrentStepPayload = {
  definitionId: string;
  definitionVersion: string;
  currentStep: CampaignCurrentStep | null;
};

export type CampaignDefinitionSummary = {
  definitionId: string;
  sectionSlug: string;
  title: string;
  stepCount: number;
  firstStepId: string;
};

export const campaignDefinitionClient = {
  list: (): Promise<CampaignDefinitionSummary[]> => listCampaignDefinitions(),
  getStep: (definitionId: string, stepId: string): Promise<CampaignCurrentStepPayload> =>
    getCampaignStep(definitionId, stepId).then((step) => step as CampaignCurrentStepPayload),
  checkSolution: async (definitionId: string, stepId: string, candidateSource: string): Promise<CampaignCheckOutcome> => {
    try {
      const result = await checkCampaignStep(definitionId, stepId, candidateSource);
      return { ok: true, matched: result.matched };
    } catch (error) {
      return { ok: false, rawError: formatBackendError(error) };
    }
  },
};

export type CampaignCheckOutcome =
  | { ok: true; matched: boolean }
  | { ok: false; rawError: string };
