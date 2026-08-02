import { openProjectInEditor, revealProjectFolder, formatBackendError } from './tauri/client';

// thread-source-binding §3 — frontend source-action seam.
//
// OPEN FILE is served by the existing `open_project_in_editor` backend, which
// mirrors the bound source and opens `model.ecky` in the system editor while
// reporting the exact absolute `folder` + `file`. REVEAL FOLDER is a frontend
// backend. Both commands return the exact persisted binding paths. The seam
// never invents a generic error — it surfaces the raw backend reason verbatim.

export type SourceActionKind = 'open' | 'reveal';

export interface SourceLink {
  slug: string;
  folder: string;
  file: string;
}

export type SourceActionOutcome =
  | {
      kind: SourceActionKind;
      ok: true;
      link: SourceLink;
    }
  | {
      kind: SourceActionKind;
      ok: false;
      error: string;
    };

export interface SourceActionDeps {
  /**
   * Resolves AND opens the bound source file in the OS editor; returns the
   * exact absolute folder + file paths. Backed by open_project_in_editor.
   */
  openInEditor?: (threadId: string | null, messageId: string | null) => Promise<SourceLink>;
  /**
   * Resolves and reveals the bound folder through the native backend.
   */
  revealInFileManager?: (
    threadId: string | null,
    messageId: string | null,
  ) => Promise<SourceLink>;
}

export function createSourceActions(deps: SourceActionDeps = {}) {
  const openInEditor = deps.openInEditor ?? defaultOpenInEditor;
  const revealInFileManager = deps.revealInFileManager ?? defaultRevealInFileManager;

  async function openSourceFile(
    threadId: string | null,
    messageId: string | null,
  ): Promise<SourceActionOutcome> {
    try {
      const link = await openInEditor(threadId, messageId);
      return { kind: 'open', ok: true, link };
    } catch (error) {
      return { kind: 'open', ok: false, error: formatBackendError(error) };
    }
  }

  async function revealSourceFolder(
    threadId: string | null,
    messageId: string | null,
    _knownLink?: SourceLink | null,
  ): Promise<SourceActionOutcome> {
    try {
      const link = await revealInFileManager(threadId, messageId);
      return { kind: 'reveal', ok: true, link };
    } catch (error) {
      return { kind: 'reveal', ok: false, error: formatBackendError(error) };
    }
  }

  return { openSourceFile, revealSourceFolder };
}

async function defaultOpenInEditor(
  threadId: string | null,
  messageId: string | null,
): Promise<SourceLink> {
  return openProjectInEditor(threadId, messageId);
}

async function defaultRevealInFileManager(
  threadId: string | null,
  messageId: string | null,
): Promise<SourceLink> {
  return revealProjectFolder(threadId, messageId);
}
