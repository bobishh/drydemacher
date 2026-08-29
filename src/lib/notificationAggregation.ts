type NotificationCardLike = {
  threadId: string | null;
  actorLabel: string;
  local: boolean;
  activityKind: string | null;
};

type NotificationEventLike = {
  sessionId: string;
  actor: { label: string };
  severity: string;
  state: string;
  requiresAttention: boolean;
};

const PROJECT_FOLDER_WATCHER_SESSION_ID = 'project-folder-watcher';

function isProjectFolderWatcher(card: NotificationCardLike): boolean {
  return /^folder[-_ ]?sync$/i.test(card.actorLabel.trim());
}

export function shouldProjectAgentNotification(event: NotificationEventLike): boolean {
  if (event.sessionId !== PROJECT_FOLDER_WATCHER_SESSION_ID) return true;
  if (event.requiresAttention) return true;
  if (event.severity === 'error' || event.severity === 'question') return true;
  return event.state === 'failed';
}

export function collapseProjectFolderRenderCards<T extends NotificationCardLike>(
  cards: T[],
  projectFolderThreadId: string | null,
): T[] {
  if (!projectFolderThreadId) return cards;

  return cards.filter((card) => (
    card.local
    || card.threadId !== projectFolderThreadId
    || !isProjectFolderWatcher(card)
  ));
}
