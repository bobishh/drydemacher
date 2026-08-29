import test from 'node:test';
import assert from 'node:assert/strict';

import type { ArtifactBundle, Message, ModelManifest } from '../types/domain';
import {
  activeThreadLoadingId,
  activeThreadMessagesLoading,
  activeThreadVersionLoading,
  createNewThread,
  evictSupersededVersionDetails,
  resolveCommittedVersionAfterHistoryRefresh,
  threadMessagePageState,
} from './history';
import type { Thread } from '../types/domain';
import { activeThreadIdStore, activeVersionId } from './domainState';
import { historyStore } from './domainState';
import { session } from './sessionStore';
import { get } from 'svelte/store';
import { activeVersionTimelineIndex, versionTimelineMessages } from '../threadTimeline';
import { rememberCommittedVersionMessage, rememberLatestThreadVersion } from './history';

function sampleBundle(modelId: string, modelStlPath: string): ArtifactBundle {
  return {
    schemaVersion: 1,
    modelId,
    sourceKind: 'generated',
    engineKind: 'ecky',
    sourceLanguage: 'ecky',
    geometryBackend: 'build123d',
    contentHash: `hash-${modelId}`,
    artifactVersion: 1,
    fcstdPath: '',
    manifestPath: `/tmp/${modelId}.json`,
    macroPath: `/tmp/${modelId}.ecky`,
    modelStlPath,
    viewerAssets: [],
    edgeTargets: [],
    calloutAnchors: [],
    measurementGuides: [],
    exportArtifacts: [],
  };
}

function sampleManifest(modelId: string): ModelManifest {
  return {
    modelId,
    sourceKind: 'generated',
    sourceLanguage: 'ecky',
    geometryBackend: 'build123d',
    document: {
      documentName: 'Test',
      documentLabel: 'Test',
      objectCount: 1,
      warnings: [],
    },
    taggedAnchors: {},
    analysisDeclarations: [],
    parts: [],
    parameterGroups: [],
    selectionTargets: [],
    warnings: [],
    controlPrimitives: [],
    controlViews: [],
    controlRelations: [],
    enrichmentState: { status: 'none', proposals: [] },
  };
}

function sampleMessage(
  id: string,
  artifactBundle: ArtifactBundle,
  modelManifest: ModelManifest,
): Message {
  return {
    id,
    role: 'assistant',
    content: 'Version',
    status: 'success',
    output: null,
    usage: null,
    artifactBundle,
    modelManifest,
    agentOrigin: null,
    imageData: null,
    visualKind: null,
    attachmentImages: [],
    timestamp: Date.now(),
  };
}

function sampleThread(id: string, messages: Message[] = []): Thread {
  return {
    id,
    title: id,
    summary: '',
    messages,
    updatedAt: 1,
    versionCount: messages.length,
    pendingCount: 0,
    queuedCount: 0,
    errorCount: 0,
    status: 'active',
  };
}

test('history refresh replaces a cleared draft target with the newest committed exact version', () => {
  const older = sampleMessage(
    'message-old',
    sampleBundle('model-old', '/tmp/model-old.stl'),
    sampleManifest('model-old'),
  );
  const exactHead = sampleMessage(
    'message-exact-head',
    sampleBundle('model-exact-head', '/tmp/model-exact-head.stl'),
    sampleManifest('model-exact-head'),
  );

  const resolved = resolveCommittedVersionAfterHistoryRefresh(
    'cleared-draft-preview-id',
    [older, exactHead],
  );

  assert.equal(resolved?.id, 'message-exact-head');
  assert.equal(resolved?.artifactBundle?.modelStlPath, '/tmp/model-exact-head.stl');
});

test('history refresh preserves an explicitly selected committed version', () => {
  const selected = sampleMessage(
    'message-selected',
    sampleBundle('model-selected', '/tmp/model-selected.stl'),
    sampleManifest('model-selected'),
  );
  const newer = sampleMessage(
    'message-newer',
    sampleBundle('model-newer', '/tmp/model-newer.stl'),
    sampleManifest('model-newer'),
  );

  assert.equal(
    resolveCommittedVersionAfterHistoryRefresh('message-selected', [selected, newer]),
    null,
  );
});

test('history keeps failed artifactless versions and advances head to latest append', () => {
  historyStore.set([]);
  activeVersionId.set(null);

  const first: Message = {
    id: 'failed-version',
    role: 'assistant',
    content: 'provider raw error body',
    status: 'error',
    output: {
      title: 'Draft',
      versionName: 'V-failed',
      response: 'provider raw error body',
      interactionMode: 'design',
      macroCode: 'box()',
      sourceLanguage: 'ecky',
      geometryBackend: 'build123d',
      uiSpec: { fields: [] },
      initialParams: {},
    },
    artifactBundle: null,
    modelManifest: null,
    timestamp: 10,
  };
  const second = { ...first, id: 'pending-version', status: 'pending' as const, timestamp: 11 };

  rememberCommittedVersionMessage('thread-lossless', 'Draft', first);
  rememberLatestThreadVersion('thread-lossless', second);

  const thread = get(historyStore).find((candidate) => candidate.id === 'thread-lossless');
  assert.deepEqual(thread?.messages.map((message) => message.id), ['pending-version', 'failed-version']);
  assert.equal(thread?.versionCount, 2);
  assert.equal(get(activeVersionId), 'pending-version');
});

function mergeThreadMessagesLike(existing: Message[], incoming: Message[]): Message[] {
  const seen = new Set<string>();
  return [...incoming, ...existing].filter((message) => {
    if (seen.has(message.id)) return false;
    seen.add(message.id);
    return true;
  });
}

function mergeThreadMessagePayloadLike(existing: Message | undefined, incoming: Message): Message {
  if (!existing) return incoming;
  return {
    ...existing,
    ...incoming,
    output: incoming.output ?? existing.output,
    artifactBundle: incoming.artifactBundle ?? existing.artifactBundle,
    modelManifest: incoming.modelManifest ?? existing.modelManifest,
  };
}

function versionCountForMessagesLike(messages: Message[], fallback: number): number {
  return Math.max(
    fallback,
    messages.filter((message) => Boolean(message.output || message.artifactBundle || message.modelManifest)).length,
  );
}

function mergeCommittedVersionMessageLike(
  threads: Thread[],
  threadId: string,
  title: string,
  message: Message,
) {
  const existing = threads.find((thread) => thread.id === threadId) ?? null;
  const nextMessages = mergeThreadMessagesLike(existing?.messages ?? [], [message]);
  const nextThread: Thread = existing
    ? {
        ...existing,
        title: title || existing.title,
        messages: nextMessages,
        updatedAt: Math.max(existing.updatedAt ?? 0, message.timestamp),
        versionCount: versionCountForMessagesLike(nextMessages, existing.versionCount ?? 0),
      }
    : {
        id: threadId,
        title,
        summary: '',
        messages: nextMessages,
        updatedAt: message.timestamp,
        versionCount: versionCountForMessagesLike(nextMessages, 0),
        pendingCount: 0,
        queuedCount: 0,
        errorCount: 0,
        status: 'active',
      };

  return [nextThread, ...threads.filter((thread) => thread.id !== threadId)];
}

function mergeActiveThreadMessagesLike(
  existingMessages: Message[],
  incomingMessages: Message[],
  activeMessageId: string | null,
): Message[] {
  const existingById = new Map(existingMessages.map((message) => [message.id, message]));
  const incomingIds = new Set(incomingMessages.map((message) => message.id));
  const mergedIncoming = incomingMessages.map((message) =>
    mergeThreadMessagePayloadLike(existingById.get(message.id), message),
  );

  if (!activeMessageId || incomingIds.has(activeMessageId)) {
    return mergedIncoming;
  }

  const restoredActive = existingById.get(activeMessageId);
  return restoredActive ? [restoredActive, ...mergedIncoming] : mergedIncoming;
}

function beginThreadSwitchLike(targetThreadId: string) {
  activeVersionId.set(null);
  session.setError(null);
  session.setStlUrl(null);
  session.clearModelRuntime();
  activeThreadIdStore.set(targetThreadId);
}

function detachActiveVersionRuntimeLike() {
  activeVersionId.set(null);
  session.setStlUrl(null);
  session.clearModelRuntime();
}

function effectiveActiveVersionIdLike(messages: Message[], currentVersionId: string | null): string | null {
  const versions = versionTimelineMessages(messages);
  const index = activeVersionTimelineIndex(versions, currentVersionId);
  return index >= 0 ? versions[index]?.id ?? null : null;
}

test('mergeCommittedVersionMessage inserts committed fork message into new active thread', () => {
  const bundle = sampleBundle('model-1', '/tmp/model.stl');
  const manifest = sampleManifest('model-1');
  const message = sampleMessage('msg-fork', bundle, manifest);

  const merged = mergeCommittedVersionMessageLike(
    [sampleThread('thread-old')],
    'thread-fork',
    'Forked Box',
    message,
  );

  assert.equal(merged[0].id, 'thread-fork');
  assert.equal(merged[0].title, 'Forked Box');
  assert.equal(merged[0].messages[0]?.id, 'msg-fork');
  assert.equal(merged[0].versionCount, 1);
  assert.equal(merged[1].id, 'thread-old');
});

test('mergeActiveThreadMessages preserves seeded active version when first page omits it', () => {
  const active = sampleMessage('msg-active', sampleBundle('model-active', '/tmp/active.stl'), sampleManifest('model-active'));
  const older = sampleMessage('msg-older', sampleBundle('model-older', '/tmp/older.stl'), sampleManifest('model-older'));

  const merged = mergeActiveThreadMessagesLike([active], [older], 'msg-active');

  assert.deepEqual(merged.map((message) => message.id), ['msg-active', 'msg-older']);
});

test('mergeActiveThreadMessages hydrates skinny page payload from seeded active version', () => {
  const active = sampleMessage('msg-active', sampleBundle('model-active', '/tmp/active.stl'), sampleManifest('model-active'));
  const skinny: Message = {
    ...active,
    output: null,
    artifactBundle: null,
    modelManifest: null,
  };

  const merged = mergeActiveThreadMessagesLike([active], [skinny], 'msg-active');

  assert.equal(merged[0]?.id, 'msg-active');
  assert.equal(merged[0]?.artifactBundle?.modelStlPath, '/tmp/active.stl');
  assert.equal(merged[0]?.modelManifest?.modelId, 'model-active');
});

test('effectiveActiveVersionId falls back to displayed latest version when active id is a draft preview', () => {
  const older = sampleMessage(
    'msg-older',
    sampleBundle('model-older', '/tmp/older.stl'),
    sampleManifest('model-older'),
  );
  const latest = {
    ...sampleMessage(
      'msg-latest',
      sampleBundle('model-latest', '/tmp/latest.stl'),
      sampleManifest('model-latest'),
    ),
    timestamp: older.timestamp + 1,
  };

  const effective = effectiveActiveVersionIdLike([older, latest], 'draft-preview-id');

  assert.equal(effective, 'msg-latest');
});

test('Given thread switch starts When previous model is still loaded Then stale version runtime is detached before new thread becomes active', () => {
  const oldBundle = sampleBundle('model-old', '/tmp/old.stl');
  const oldManifest = sampleManifest('model-old');

  activeThreadIdStore.set('thread-old');
  activeVersionId.set('message-old');
  session.setStlUrl('/tmp/old.stl');
  session.setModelRuntime(oldBundle, oldManifest);

  beginThreadSwitchLike('thread-new');

  assert.equal(get(activeThreadIdStore), 'thread-new');
  assert.equal(get(activeVersionId), null);
  assert.equal(get(session).stlUrl, null);
  assert.equal(get(session).artifactBundle, null);
  assert.equal(get(session).modelManifest, null);
});

test('Given active version is removed When fallback version is still resolving Then stale viewport runtime is detached', () => {
  const oldBundle = sampleBundle('model-old', '/tmp/old.stl');
  const oldManifest = sampleManifest('model-old');

  activeThreadIdStore.set('thread-1');
  activeVersionId.set('message-old');
  session.setStlUrl('/tmp/old.stl');
  session.setModelRuntime(oldBundle, oldManifest);

  detachActiveVersionRuntimeLike();

  assert.equal(get(activeVersionId), null);
  assert.equal(get(session).stlUrl, null);
  assert.equal(get(session).artifactBundle, null);
  assert.equal(get(session).modelManifest, null);
});

test('Given stale thread messages are loading When backend opens the reusable empty thread Then loading state is cleared', async () => {
  activeThreadIdStore.set('thread-old');
  activeVersionId.set('message-old');
  activeThreadLoadingId.set('thread-old');
  activeThreadMessagesLoading.set(true);
  activeThreadVersionLoading.set(true);
  threadMessagePageState.set({
    'thread-old': {
      isLoading: true,
      hasMore: false,
      nextBefore: null,
      error: null,
    },
  });

  await createNewThread(
    { mode: 'blank' },
    {
      openBlank: async () => ({
        threadId: 'thread-empty',
        slug: 'untitled-thread-empty',
        folder: '/tmp/untitled-thread-empty',
        file: '/tmp/untitled-thread-empty/model.ecky',
        source: '(model (part body (box 20 20 20)))',
      }),
    },
  );

  const newThreadId = get(activeThreadIdStore);
  assert.ok(newThreadId);
  assert.notEqual(newThreadId, 'thread-old');
  assert.equal(newThreadId, 'thread-empty');
  assert.equal(get(activeVersionId), null);
  assert.equal(get(activeThreadLoadingId), null);
  assert.equal(get(activeThreadMessagesLoading), false);
  assert.equal(get(activeThreadVersionLoading), false);
  assert.equal(get(threadMessagePageState)[newThreadId!]?.isLoading, false);
});

test('selected detail hydration evicts superseded heavy version payloads', () => {
  const older = sampleMessage('older', sampleBundle('older-model', '/tmp/older.stl'), sampleManifest('older-model'));
  const selected = sampleMessage('selected', sampleBundle('selected-model', '/tmp/selected.stl'), sampleManifest('selected-model'));
  const threads: Thread[] = [{
    id: 'thread-1',
    title: 'Thread',
    summary: '',
    messages: [older, selected],
    updatedAt: 1,
    genieTraits: null,
    versionCount: 2,
    pendingCount: 0,
    queuedCount: 0,
    errorCount: 0,
    isBlank: false,
    status: 'active',
    finalizedAt: null,
    pendingConfirm: null,
  }];

  const projected = evictSupersededVersionDetails(threads, 'thread-1', selected);

  assert.equal(projected[0].messages[0].artifactBundle, null);
  assert.equal(projected[0].messages[0].modelManifest, null);
  assert.equal(projected[0].messages[1].artifactBundle?.modelId, 'selected-model');
});
