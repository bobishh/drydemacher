import type { ThreadWindowLayout } from '../tauri/contracts';
import {
  clearActiveProjectNavigation,
  createCampaignRun,
  deleteCampaignRun,
  getActiveProjectNavigation,
  getAppWindowLayout,
  getCampaignRun,
  listCampaignRuns,
  saveActiveProjectNavigation,
  saveAppWindowLayout,
  saveCampaignRun,
} from '../tauri/client';

/** Driver-facing contract. Campaign runs are projects, but never design threads. */
export type CampaignRun = {
  id: string;
  title: string;
  definitionId: string;
  kind: 'campaignRun';
  definitionVersion: string;
  currentStepId: string;
  completedStepIds: string[];
  passedChallengeIds: string[];
  draftOverridesByStepId: Record<string, string>;
  createdAt: number;
  updatedAt: number;
};

export type CreateCampaignRunInput = Pick<CampaignRun, 'title' | 'definitionId' | 'definitionVersion' | 'currentStepId'>;

export type ActiveProjectNavigation = {
  kind: 'design' | 'campaign';
  id: string;
  view: 'workbench' | 'campaign';
};

export const campaignRunClient = {
  create(input: CreateCampaignRunInput) {
    return createCampaignRun(input).then((run) => run as CampaignRun);
  },
  list() {
    return listCampaignRuns().then((runs) => runs as CampaignRun[]);
  },
  get(id: string) {
    return getCampaignRun(id).then((run) => run as CampaignRun);
  },
  save(run: CampaignRun) {
    return saveCampaignRun(run).then((saved) => saved as CampaignRun);
  },
  delete(id: string) {
    return deleteCampaignRun(id);
  },
  getActiveProjectNavigation() {
    return getActiveProjectNavigation().then((navigation) => navigation as ActiveProjectNavigation | null);
  },
  saveActiveProjectNavigation(navigation: ActiveProjectNavigation) {
    return saveActiveProjectNavigation(navigation).then((saved) => saved as ActiveProjectNavigation);
  },
  clearActiveProjectNavigation() {
    return clearActiveProjectNavigation();
  },
  getAppWindowLayout() {
    return getAppWindowLayout();
  },
  saveAppWindowLayout(layout: ThreadWindowLayout) {
    return saveAppWindowLayout(layout);
  },
};
