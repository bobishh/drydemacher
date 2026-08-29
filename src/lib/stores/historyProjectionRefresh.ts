type RefreshState<T> = {
  requestedRevision: number;
  completedRevision: number;
  inFlight: Promise<void> | null;
  fetch: (revision: number) => Promise<T>;
  apply: (value: T, revision: number) => void;
};

export type RevisionedSingleflight<T> = {
  request(
    key: string,
    revision: number,
    fetch: (revision: number) => Promise<T>,
    apply: (value: T, revision: number) => void,
  ): Promise<void>;
};

export function createRevisionedSingleflight<T>(): RevisionedSingleflight<T> {
  const states = new Map<string, RefreshState<T>>();

  return {
    request(key, revision, fetch, apply) {
      const normalizedRevision = Math.max(0, Math.floor(revision));
      const state = states.get(key) ?? {
        requestedRevision: 0,
        completedRevision: 0,
        inFlight: null,
        fetch,
        apply,
      };
      state.requestedRevision = Math.max(state.requestedRevision, normalizedRevision);
      state.fetch = fetch;
      state.apply = apply;
      states.set(key, state);
      if (state.inFlight) return state.inFlight;

      state.inFlight = (async () => {
        while (state.completedRevision < state.requestedRevision) {
          const targetRevision = state.requestedRevision;
          const value = await state.fetch(targetRevision);
          state.completedRevision = targetRevision;
          if (targetRevision < state.requestedRevision) continue;
          state.apply(value, targetRevision);
        }
      })().finally(() => {
        state.inFlight = null;
        if (state.completedRevision >= state.requestedRevision) states.delete(key);
      });
      return state.inFlight;
    },
  };
}
