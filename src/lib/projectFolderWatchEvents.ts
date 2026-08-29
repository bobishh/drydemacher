type ThreadScopedWatchEvent = {
  threadId: string;
};

export function selectProjectFolderWatchEvent<T extends ThreadScopedWatchEvent>(
  events: T[],
  activeThreadId: string | null,
): T | undefined {
  if (!activeThreadId) return undefined;
  for (let index = events.length - 1; index >= 0; index -= 1) {
    if (events[index]?.threadId === activeThreadId) return events[index];
  }
  return undefined;
}
