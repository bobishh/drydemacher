/**
 * Projects shell contract. Drivers own type-specific identity and card data;
 * the shell never treats a campaign run as a design thread.
 */
export type ProjectCard = {
  kind: 'design' | 'campaignRun';
  id: string;
  title: string;
  updatedAt: number;
  progress: string;
};

export type DesignProjectInput = Pick<ProjectCard, 'id' | 'title' | 'updatedAt'> & {
  versionCount: number;
};

export type CampaignRunProjectInput = Pick<ProjectCard, 'id' | 'title' | 'updatedAt'> & {
  currentStepId: string;
  completedStepIds: readonly string[];
};

export const designProjectDriver = {
  kind: 'design' as const,
  card(project: DesignProjectInput): ProjectCard {
    return {
      kind: 'design',
      id: project.id,
      title: project.title,
      updatedAt: project.updatedAt,
      progress: `${project.versionCount} version${project.versionCount === 1 ? '' : 's'}`,
    };
  },
};

export const campaignRunProjectDriver = {
  kind: 'campaignRun' as const,
  card(run: CampaignRunProjectInput): ProjectCard {
    const complete = run.completedStepIds.length;
    return {
      kind: 'campaignRun',
      id: run.id,
      title: run.title,
      updatedAt: run.updatedAt,
      progress: `${complete} step${complete === 1 ? '' : 's'} complete`,
    };
  },
};

export const projectDriverRegistry = {
  design: designProjectDriver,
  campaignRun: campaignRunProjectDriver,
} as const;
