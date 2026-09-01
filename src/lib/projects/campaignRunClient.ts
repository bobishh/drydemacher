import type { ThreadWindowLayout } from '../tauri/contracts';
import {
  clearActiveProjectNavigation,
  deleteCampaignRun,
  getActiveProjectNavigation,
  getAppWindowLayout,
  getCampaignRun,
  listCampaignRuns,
  openCampaignProject,
  saveAppWindowLayout,
  transitionCampaignRun,
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

export type ActiveProjectNavigation = {
  kind: 'design' | 'campaign';
  id: string;
  view: 'workbench' | 'campaign';
};

export const campaignRunClient = {
  open(input: import('../tauri/contracts').OpenCampaignProjectIntent) {
    return openCampaignProject(input);
  },
  list() {
    return listCampaignRuns().then((runs) => runs as CampaignRun[]);
  },
  get(id: string) {
    return getCampaignRun(id).then((run) => run as CampaignRun);
  },
  transition(input: import('../tauri/contracts').TransitionCampaignRunInput) {
    return transitionCampaignRun(input);
  },
  delete(id: string) {
    return deleteCampaignRun(id);
  },
  getActiveProjectNavigation() {
    return getActiveProjectNavigation().then((navigation) => navigation as ActiveProjectNavigation | null);
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
