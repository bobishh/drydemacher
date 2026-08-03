const MISSION_PREVIEWS: Readonly<Record<string, string>> = {
  'mission-01-bracket-enclosure': '/docs/assets/corner-bracket.png',
  'mission-02-bottle-cage-dovetail': '/docs/assets/dovetail-fit.png',
  'mission-03-wing-propeller-study': '/docs/assets/06-paths-and-surfaces-04.png',
  'mission-04-gillette-travel-kit': '/docs/assets/10-real-model-patterns-01.png',
  'mission-05-iphone-case-fixture': '/docs/assets/10-real-model-patterns-02.png',
  'mission-06-film-scanner': '/docs/assets/11-complex-film-adapter-01.png',
};

export function campaignRunPreviewSrc(currentStepId: string): string | null {
  const missionId = currentStepId.split('/', 1)[0]?.trim();
  return missionId ? MISSION_PREVIEWS[missionId] ?? null : null;
}
