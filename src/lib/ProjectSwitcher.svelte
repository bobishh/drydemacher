<script lang="ts">
  import { onMount } from 'svelte';
  import { convertFileSrc } from '@tauri-apps/api/core';
  import { open } from '@tauri-apps/plugin-dialog';
  import {
    activeThreadLoadingId,
    createNewThread,
    deleteThread,
    finalizeThread,
    loadFromHistory,
    loadInventory,
    openInventoryThread,
    refreshHistory,
    rememberLatestThreadVersion,
    renameThread,
    reopenThread,
  } from './stores/history';
  import {
    activeThreadIdStore as activeThreadId,
    historyStore as history,
  } from './stores/domainState';
  import {
    formatBackendError,
    getDeletedThreadPreview,
    getDeletedThreadsPage,
    getThreadLatestVersion,
    getThreadMessagesPage,
    restoreDeletedThread,
  } from './tauri/client';
  import type { DeletedThreadSummary } from './tauri/contracts';
  import type { Message, Thread } from './types/domain';
  import { selectThreadPreviewImage, type ProjectPreviewImage } from './projectPreview';
  import ManualImportModal from './ManualImportModal.svelte';
  import Modal from './Modal.svelte';

  let {
    onImportFcstd,
    onOpenNewProjectChooser,
    freecadUnavailableReason = null,
  }: {
    onImportFcstd?: (sourcePath: string) => void;
    onOpenNewProjectChooser?: () => void;
    freecadUnavailableReason?: string | null;
  } = $props();

  type Tab = 'active' | 'completed' | 'trash';

  const TRASH_PAGE_SIZE = 24;
  const queuedPreviewThreadIds = new Set<string>();

  let activeTab = $state<Tab>('active');
  let searchQuery = $state('');
  let isLoading = $state(false);
  let loadError = $state<string | null>(null);

  let completedProjects = $state<Thread[]>([]);
  let completedLoaded = $state(false);
  let deletedProjects = $state<DeletedThreadSummary[]>([]);
  let deletedProjectPreviews = $state<Record<string, string | null>>({});
  let trashLoaded = $state(false);
  let trashNextBefore = $state<string | null>(null);
  let trashHasMore = $state(false);
  let trashLoadMoreBusy = $state(false);

  let latestVersions = $state<Record<string, Message | null>>({});
  let previewImages = $state<Record<string, ProjectPreviewImage | null>>({});
  let previewFetchVersionIds = $state<Record<string, string | null>>({});
  let previewPumpActive = false;

  let showNewChooser = $state(false);
  let showImport = $state(false);
  let projectToDelete = $state<Thread | null>(null);
  let editingProjectId = $state<string | null>(null);
  let editingTitle = $state('');
  let renameBusy = $state(false);
  let pendingActionId = $state<string | null>(null);
  let openActionsProjectId = $state<string | null>(null);

  onMount(() => {
    const onPreviewUpdated = (event: Event) => {
      const detail = (event as CustomEvent<{
        threadId?: string;
        messageId?: string;
        imageData?: string;
      }>).detail;
      if (!detail?.threadId || !detail.imageData) return;

      if (detail.messageId) {
        previewImages = {
          ...previewImages,
          [detail.threadId]: {
            messageId: detail.messageId,
            imageData: detail.imageData,
          },
        };
        previewFetchVersionIds = {
          ...previewFetchVersionIds,
          [detail.threadId]: detail.messageId,
        };
      }

      const latest = latestVersions[detail.threadId];
      if (latest && latest.id === detail.messageId) {
        latestVersions = {
          ...latestVersions,
          [detail.threadId]: { ...latest, imageData: detail.imageData },
        };
      }
    };

    window.addEventListener('ecky:version-preview-updated', onPreviewUpdated);
    return () => window.removeEventListener('ecky:version-preview-updated', onPreviewUpdated);
  });

  function previewSrc(raw: string | null | undefined): string | null {
    const value = raw?.trim();
    if (!value) return null;
    if (/^(data:image\/|blob:|https?:|asset:|tauri:)/i.test(value)) return value;
    try {
      return convertFileSrc(value);
    } catch {
      return value;
    }
  }

  function threadPreviewImage(thread: Thread): string | null {
    return previewSrc(
      selectThreadPreviewImage(
        thread,
        latestVersions[thread.id],
        previewImages[thread.id],
      ),
    );
  }

  function newestPreviewImage(messages: Message[]): ProjectPreviewImage | null {
    for (const message of [...messages].reverse()) {
      if (
        message.role === 'assistant' &&
        message.status === 'success' &&
        message.artifactBundle &&
        message.imageData?.trim()
      ) {
        return { messageId: message.id, imageData: message.imageData };
      }
    }
    return null;
  }

  async function loadActiveTab() {
    if (
      activeTab === 'active' ||
      (activeTab === 'completed' && completedLoaded) ||
      (activeTab === 'trash' && trashLoaded)
    ) {
      return;
    }

    isLoading = true;
    loadError = null;
    try {
      if (activeTab === 'completed') {
        completedProjects = await loadInventory();
        completedLoaded = true;
      } else {
        const page = await getDeletedThreadsPage(null, TRASH_PAGE_SIZE);
        deletedProjects = page.items;
        trashNextBefore = page.nextBefore;
        trashHasMore = page.hasMore;
        trashLoaded = true;
      }
    } catch (error) {
      loadError = formatBackendError(error);
    } finally {
      isLoading = false;
    }
  }

  $effect(() => {
    void loadActiveTab();
  });

  async function fetchLatestVersion(threadId: string) {
    let version = latestVersions[threadId];
    if (version === undefined) {
      try {
        version = await getThreadLatestVersion(threadId);
        latestVersions = { ...latestVersions, [threadId]: version };
        if (version) rememberLatestThreadVersion(threadId, version);
        if (version?.imageData) {
          previewImages = {
            ...previewImages,
            [threadId]: { messageId: version.id, imageData: version.imageData },
          };
          previewFetchVersionIds = {
            ...previewFetchVersionIds,
            [threadId]: version.id,
          };
          return;
        }
      } catch (error) {
        console.error(`Failed to fetch latest version for ${threadId}:`, error);
        latestVersions = { ...latestVersions, [threadId]: null };
      }
    }

    const latestMessageId = version?.id ?? null;
    if (
      previewImages[threadId] !== undefined &&
      previewFetchVersionIds[threadId] === latestMessageId
    ) {
      return;
    }

    try {
      let before: number | null = null;
      let preview: ProjectPreviewImage | null = null;
      let hasMore = true;
      while (!preview && hasMore) {
        const page = await getThreadMessagesPage(threadId, before, 24, true);
        preview = newestPreviewImage(page.messages);
        before = page.nextBefore;
        hasMore = page.hasMore && before !== null;
      }
      if ((latestVersions[threadId]?.id ?? null) !== latestMessageId) return;
      previewImages = { ...previewImages, [threadId]: preview };
      previewFetchVersionIds = {
        ...previewFetchVersionIds,
        [threadId]: latestMessageId,
      };
    } catch (error) {
      console.error(`Failed to fetch preview history for ${threadId}:`, error);
      previewImages = { ...previewImages, [threadId]: null };
      previewFetchVersionIds = {
        ...previewFetchVersionIds,
        [threadId]: latestMessageId,
      };
    }
  }

  function queuePreviewFetch(threadId: string) {
    if (
      latestVersions[threadId] !== undefined &&
      previewImages[threadId] !== undefined &&
      previewFetchVersionIds[threadId] === (latestVersions[threadId]?.id ?? null)
    ) {
      return;
    }
    if (queuedPreviewThreadIds.has(threadId)) return;
    queuedPreviewThreadIds.add(threadId);
    if (previewPumpActive) return;

    previewPumpActive = true;
    queueMicrotask(async () => {
      try {
        while (queuedPreviewThreadIds.size > 0) {
          const threadId = queuedPreviewThreadIds.values().next().value;
          if (!threadId) break;
          queuedPreviewThreadIds.delete(threadId);
          await fetchLatestVersion(threadId);
        }
      } finally {
        previewPumpActive = false;
      }
    });
  }

  async function warmProjectPreviews(projects: Thread[]) {
    for (const project of projects) {
      if ((project.versionCount ?? 0) <= 0) continue;
      queuePreviewFetch(project.id);
      await new Promise((resolve) => setTimeout(resolve, 0));
    }
  }

  const activeProjects = $derived(
    $history.filter(
      (project: Thread) =>
        project.status !== 'finalized' &&
        (
          project.title.toLowerCase().includes(searchQuery.toLowerCase()) ||
          Boolean(project.summary?.toLowerCase().includes(searchQuery.toLowerCase()))
        ),
    ),
  );
  const visibleCompletedProjects = $derived(
    completedProjects.filter(
      (project) =>
        project.title.toLowerCase().includes(searchQuery.toLowerCase()) ||
        Boolean(project.summary?.toLowerCase().includes(searchQuery.toLowerCase())),
    ),
  );
  const visibleDeletedProjects = $derived(
    deletedProjects.filter(
      (project) =>
        project.title.toLowerCase().includes(searchQuery.toLowerCase()) ||
        Boolean(project.summary?.toLowerCase().includes(searchQuery.toLowerCase())),
    ),
  );

  $effect(() => {
    if (activeTab === 'active') {
      void warmProjectPreviews(activeProjects);
    } else if (activeTab === 'completed') {
      void warmProjectPreviews(visibleCompletedProjects);
    }
  });

  function trashPreviewCard(node: HTMLElement, project: DeletedThreadSummary) {
    let currentProject = project;
    const fetch = async () => {
      if (deletedProjectPreviews[currentProject.id] !== undefined) return;
      deletedProjectPreviews = {
        ...deletedProjectPreviews,
        [currentProject.id]: null,
      };
      try {
        const imageData = await getDeletedThreadPreview(currentProject.id);
        deletedProjectPreviews = {
          ...deletedProjectPreviews,
          [currentProject.id]: imageData,
        };
      } catch (error) {
        console.error(
          `Failed to fetch deleted project preview for ${currentProject.id}:`,
          formatBackendError(error),
        );
      }
    };

    if (typeof IntersectionObserver === 'undefined') {
      void fetch();
      return {
        update(nextProject: DeletedThreadSummary) {
          currentProject = nextProject;
          void fetch();
        },
      };
    }

    const observer = new IntersectionObserver(
      (entries) => {
        if (entries.some((entry) => entry.isIntersecting)) void fetch();
      },
      { root: node.closest('.scrollable'), rootMargin: '240px 0px' },
    );
    observer.observe(node);

    return {
      update(nextProject: DeletedThreadSummary) {
        currentProject = nextProject;
      },
      destroy() {
        observer.disconnect();
      },
    };
  }

  function formatDate(timestamp: number) {
    return new Date(timestamp * 1000).toLocaleString(undefined, {
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    });
  }

  function attentionText(project: Thread): string | null {
    const queued = Number(project.queuedCount || 0);
    const confirm = project.pendingConfirm?.trim();
    if (confirm && queued > 0) return `ACTION REQUIRED · ${queued} queued`;
    if (confirm) return 'ACTION REQUIRED';
    if (queued > 0) return `${queued} queued ${queued === 1 ? 'message' : 'messages'}`;
    return null;
  }

  function selectProject(project: Thread) {
    if (editingProjectId === project.id || $activeThreadLoadingId === project.id) return;
    if (activeTab === 'completed') {
      void openInventoryThread(project.id);
    } else {
      void loadFromHistory(project);
    }
  }

  async function completeProject(id: string) {
    openActionsProjectId = null;
    pendingActionId = id;
    try {
      await finalizeThread(id);
      completedProjects = await loadInventory();
      completedLoaded = true;
    } finally {
      pendingActionId = null;
    }
  }

  async function reopenProject(id: string) {
    openActionsProjectId = null;
    pendingActionId = id;
    try {
      await reopenThread(id);
      completedProjects = completedProjects.filter((project) => project.id !== id);
    } finally {
      pendingActionId = null;
    }
  }

  async function recoverProject(id: string) {
    pendingActionId = id;
    try {
      await restoreDeletedThread(id);
      deletedProjects = deletedProjects.filter((project) => project.id !== id);
      await refreshHistory();
    } finally {
      pendingActionId = null;
    }
  }

  async function loadMoreTrash() {
    if (!trashHasMore || !trashNextBefore || trashLoadMoreBusy) return;
    trashLoadMoreBusy = true;
    try {
      const page = await getDeletedThreadsPage(trashNextBefore, TRASH_PAGE_SIZE);
      const seen = new Set(deletedProjects.map((project) => project.id));
      deletedProjects = [
        ...deletedProjects,
        ...page.items.filter((project) => !seen.has(project.id)),
      ];
      trashNextBefore = page.nextBefore;
      trashHasMore = page.hasMore;
    } catch (error) {
      loadError = formatBackendError(error);
    } finally {
      trashLoadMoreBusy = false;
    }
  }

  async function confirmDeleteProject() {
    if (!projectToDelete) return;
    const id = projectToDelete.id;
    pendingActionId = id;
    try {
      await deleteThread(id);
      trashLoaded = false;
    } finally {
      projectToDelete = null;
      pendingActionId = null;
    }
  }

  function startRename(project: Thread) {
    editingProjectId = project.id;
    editingTitle = project.title;
  }

  function cancelRename() {
    editingProjectId = null;
    editingTitle = '';
  }

  async function commitRename(project: Thread) {
    if (renameBusy) return;
    const title = editingTitle.trim();
    if (!title || title === project.title) {
      cancelRename();
      return;
    }

    renameBusy = true;
    try {
      await renameThread(project.id, title);
      cancelRename();
    } finally {
      renameBusy = false;
    }
  }

  async function handleImportFcstd() {
    if (freecadUnavailableReason) return;
    showNewChooser = false;
    const selected = await open({
      multiple: false,
      filters: [{ name: 'FreeCAD Document', extensions: ['fcstd'] }],
    });
    if (typeof selected === 'string' && selected.trim()) {
      onImportFcstd?.(selected);
    }
  }

  function retryActiveTab() {
    loadError = null;
    if (activeTab === 'completed') completedLoaded = false;
    if (activeTab === 'trash') trashLoaded = false;
    void loadActiveTab();
  }
</script>

<div class="project-switcher">
  <header class="switcher-header">
    <nav class="tabs" aria-label="Project lifecycle">
      <button class:active={activeTab === 'active'} onclick={() => activeTab = 'active'}>ACTIVE</button>
      <button class:active={activeTab === 'completed'} onclick={() => activeTab = 'completed'}>COMPLETED</button>
      <button class:active={activeTab === 'trash'} onclick={() => activeTab = 'trash'}>TRASH</button>
    </nav>
    <div class="header-actions">
      <input class="search-input" type="search" placeholder="Search..." bind:value={searchQuery} />
      <button
        class="new-btn"
        onclick={() => onOpenNewProjectChooser ? onOpenNewProjectChooser() : showNewChooser = true}
      >
        + NEW
      </button>
    </div>
  </header>

  <main class="switcher-content scrollable">
    {#if isLoading}
      <div class="loading-state">
        {activeTab === 'completed' ? 'LOADING COMPLETED...' : 'LOADING TRASH...'}
      </div>
    {:else if loadError}
      <div class="error-state">
        <strong>
          {activeTab === 'completed' ? 'COMPLETED LOAD ERROR' : 'TRASH LOAD ERROR'}
        </strong>
        <pre>{loadError}</pre>
        <button onclick={retryActiveTab}>RETRY</button>
      </div>
    {:else}
      <div class="project-grid">
        {#if activeTab === 'active'}
          {#if activeProjects.length === 0}
            <div class="empty-state">
              {searchQuery ? 'NO MATCHING ACTIVE PROJECTS' : 'NO ACTIVE PROJECTS'}
            </div>
          {/if}
          {#each activeProjects as project (project.id)}
            {@const attention = attentionText(project)}
            <article
              class="project-card"
              class:active={$activeThreadId === project.id}
              data-project-id={project.id}
            >
              <div class="card-thumb">
                {#if threadPreviewImage(project)}
                  <img src={threadPreviewImage(project) ?? ''} alt={`${project.title} preview`} />
                {:else}
                  <div class="no-thumb">NO PREVIEW</div>
                {/if}
              </div>
              <div class="card-body">
                <div class="card-header">
                  {#if editingProjectId === project.id}
                    <input
                      class="rename-input"
                      bind:value={editingTitle}
                      onkeydown={(event) => event.key === 'Enter' && commitRename(project)}
                    />
                  {:else}
                    <h3>{project.title}</h3>
                  {/if}
                  <span>{formatDate(project.updatedAt)}</span>
                </div>
                {#if project.summary}
                  <p class="summary">{project.summary}</p>
                {/if}
                {#if attention}
                  <p class="attention">{attention}</p>
                {/if}
                <div class="card-footer">
                  <span>{project.versionCount || 0} versions</span>
                  <div class="actions">
                    <button onclick={() => selectProject(project)}>OPEN</button>
                    <button
                      aria-label="MORE ACTIONS"
                      aria-expanded={openActionsProjectId === project.id}
                      onclick={() => openActionsProjectId = openActionsProjectId === project.id ? null : project.id}
                    >
                      •••
                    </button>
                  </div>
                </div>
                {#if openActionsProjectId === project.id}
                  <div class="card-action-menu">
                    <button onclick={() => completeProject(project.id)}>COMPLETE</button>
                    <button onclick={() => { openActionsProjectId = null; startRename(project); }}>RENAME</button>
                    <button class="danger" onclick={() => { openActionsProjectId = null; projectToDelete = project; }}>DELETE</button>
                  </div>
                {/if}
              </div>
            </article>
          {/each}
        {:else if activeTab === 'completed'}
          {#if visibleCompletedProjects.length === 0}
            <div class="empty-state">
              {searchQuery ? 'NO MATCHING COMPLETED PROJECTS' : 'NO COMPLETED PROJECTS'}
            </div>
          {/if}
          {#each visibleCompletedProjects as project (project.id)}
            <article class="project-card" data-project-id={project.id}>
              <div class="card-thumb">
                {#if threadPreviewImage(project)}
                  <img src={threadPreviewImage(project) ?? ''} alt={`${project.title} preview`} />
                {:else}
                  <div class="no-thumb">NO PREVIEW</div>
                {/if}
              </div>
              <div class="card-body">
                <div class="card-header">
                  <h3>{project.title}</h3>
                  <span>{formatDate(project.finalizedAt ?? project.updatedAt)}</span>
                </div>
                {#if project.summary}
                  <p class="summary">{project.summary}</p>
                {/if}
                <div class="card-footer">
                  <span>{project.versionCount || 0} versions</span>
                  <div class="actions">
                    <button onclick={() => selectProject(project)}>VIEW</button>
                    <button
                      aria-label="MORE ACTIONS"
                      aria-expanded={openActionsProjectId === project.id}
                      onclick={() => openActionsProjectId = openActionsProjectId === project.id ? null : project.id}
                    >
                      •••
                    </button>
                  </div>
                </div>
                {#if openActionsProjectId === project.id}
                  <div class="card-action-menu">
                    <button onclick={() => reopenProject(project.id)}>REOPEN</button>
                  </div>
                {/if}
              </div>
            </article>
          {/each}
        {:else}
          {#if visibleDeletedProjects.length === 0}
            <div class="empty-state">
              {searchQuery ? 'NO MATCHING DELETED PROJECTS' : 'TRASH IS EMPTY'}
            </div>
          {/if}
          {#each visibleDeletedProjects as project (project.id)}
            <article
              class="project-card"
              data-project-id={project.id}
              use:trashPreviewCard={project}
            >
              <div class="card-thumb">
                {#if previewSrc(deletedProjectPreviews[project.id])}
                  <img
                    src={previewSrc(deletedProjectPreviews[project.id]) ?? ''}
                    alt={`${project.title} preview`}
                  />
                {:else}
                  <div class="no-thumb">NO PREVIEW</div>
                {/if}
              </div>
              <div class="card-body">
                <div class="card-header">
                  <h3>{project.title}</h3>
                  <span>{formatDate(project.deletedAt)}</span>
                </div>
                {#if project.summary}
                  <p class="summary">{project.summary}</p>
                {/if}
                <div class="card-footer">
                  <span>{project.versionCount} versions</span>
                  <button
                    onclick={() => recoverProject(project.id)}
                    disabled={pendingActionId === project.id}
                  >
                    {pendingActionId === project.id ? 'RECOVERING...' : 'RECOVER'}
                  </button>
                </div>
              </div>
            </article>
          {/each}
          {#if trashHasMore}
            <div class="trash-pagination">
              <button onclick={loadMoreTrash} disabled={trashLoadMoreBusy}>
                {trashLoadMoreBusy ? 'LOADING...' : 'LOAD MORE'}
              </button>
            </div>
          {/if}
        {/if}
      </div>
    {/if}
  </main>

  {#if !onOpenNewProjectChooser && showNewChooser}
    <Modal title="Start New Project" onclose={() => showNewChooser = false}>
      <div class="new-chooser">
        <button onclick={() => { createNewThread({ mode: 'blank' }); showNewChooser = false; }}>
          Blank Project
        </button>
        <button
          onclick={handleImportFcstd}
          disabled={Boolean(freecadUnavailableReason)}
          title={freecadUnavailableReason ?? undefined}
        >
          Import FreeCAD
        </button>
        <button onclick={() => { showImport = true; showNewChooser = false; }}>Import Macro</button>
      </div>
    </Modal>
  {/if}

  {#if !onOpenNewProjectChooser && showImport}
    <ManualImportModal
      bind:show={showImport}
      onImport={(data) => {
        createNewThread({ mode: 'macro', ...data });
        showImport = false;
      }}
    />
  {/if}

  {#if projectToDelete}
    <Modal title="Trash Project" onclose={() => projectToDelete = null}>
      <div class="confirm-delete">
        <p>Move <strong>{projectToDelete.title}</strong> to trash?</p>
        <p>You can recover the complete project and its versions from <strong>TRASH</strong>.</p>
        <div class="confirm-actions">
          <button onclick={() => projectToDelete = null}>CANCEL</button>
          <button
            class="danger"
            onclick={confirmDeleteProject}
            disabled={pendingActionId === projectToDelete.id}
          >
            MOVE TO TRASH
          </button>
        </div>
      </div>
    </Modal>
  {/if}
</div>

<style>
  .project-switcher {
    container-type: inline-size;
    display: flex;
    flex-direction: column;
    height: 100%;
    min-width: 0;
    background: var(--bg-100);
    color: var(--text);
    overflow: hidden;
  }

  .switcher-header {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 12px;
    border-bottom: 1px solid var(--bg-300);
    overflow: hidden;
  }

  .tabs,
  .header-actions,
  .actions,
  .confirm-actions {
    display: flex;
    gap: 6px;
    min-width: 0;
    overflow: hidden;
  }

  button {
    border: 1px solid var(--bg-400);
    background: var(--bg-300);
    color: var(--text);
    font-family: var(--font-mono);
    font-size: 0.62rem;
    font-weight: 700;
    letter-spacing: 0.06em;
    padding: 5px 8px;
    cursor: pointer;
  }

  button:hover:not(:disabled),
  .tabs button.active {
    border-color: var(--primary);
    color: var(--primary);
  }

  button:disabled {
    opacity: 0.55;
    cursor: wait;
  }

  button.danger {
    border-color: var(--red);
    color: var(--red);
  }

  .tabs button {
    background: transparent;
    border-color: transparent;
  }

  .search-input,
  .rename-input {
    min-width: 0;
    border: 1px solid var(--bg-300);
    background: var(--bg-200);
    color: var(--text);
  }

  .search-input {
    flex: 1;
    padding: 7px 9px;
  }

  .new-btn {
    flex: 0 0 auto;
    border-color: var(--primary);
    background: var(--primary);
    color: var(--bg-100);
  }

  .switcher-content {
    flex: 1;
    min-width: 0;
    min-height: 0;
    padding: 14px;
    overflow-y: auto;
  }

  .project-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(min(240px, 100%), 1fr));
    gap: 14px;
    min-width: 0;
    overflow: hidden;
  }

  .project-card {
    display: flex;
    flex-direction: column;
    min-width: 0;
    border: 1px solid var(--bg-300);
    background: var(--bg-200);
    overflow: hidden;
  }

  .project-card.active {
    border-color: var(--primary);
    box-shadow: inset 0 0 0 1px var(--primary);
  }

  .card-thumb {
    display: grid;
    height: 120px;
    place-items: center;
    border-bottom: 1px solid var(--bg-300);
    background: #000;
    overflow: hidden;
  }

  .card-thumb img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    opacity: 0.78;
  }

  .no-thumb {
    color: var(--bg-400);
    font-size: 0.58rem;
    letter-spacing: 0.1em;
  }

  .card-body {
    display: flex;
    flex: 1;
    flex-direction: column;
    gap: 8px;
    min-width: 0;
    padding: 11px;
    overflow: hidden;
  }

  .card-header,
  .card-footer {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 8px;
    min-width: 0;
    overflow: hidden;
  }

  .card-header h3 {
    flex: 1;
    min-width: 0;
    margin: 0;
    overflow: hidden;
    color: var(--text);
    font-size: 0.84rem;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .card-header span,
  .card-footer > span {
    flex: 0 0 auto;
    color: var(--text-dim);
    font-size: 0.6rem;
  }

  .summary,
  .attention {
    margin: 0;
    overflow: hidden;
    font-size: 0.7rem;
    line-height: 1.35;
  }

  .summary {
    display: -webkit-box;
    color: var(--text-dim);
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 2;
    line-clamp: 2;
  }

  .attention {
    color: var(--secondary);
  }

  .card-footer {
    align-items: center;
    margin-top: auto;
  }

  .card-action-menu {
    display: flex;
    justify-content: flex-end;
    gap: 5px;
    padding-top: 8px;
    border-top: 1px solid var(--bg-300);
    overflow: hidden;
  }

  .rename-input {
    flex: 1;
    width: 100%;
    padding: 3px 5px;
  }

  .trash-pagination {
    grid-column: 1 / -1;
    display: flex;
    justify-content: center;
    padding: 8px;
    overflow: hidden;
  }

  .loading-state,
  .empty-state,
  .error-state {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 10px;
    padding: 24px;
    color: var(--text-dim);
    overflow: hidden;
  }

  .error-state {
    border: 1px solid var(--red);
    color: var(--red);
  }

  .error-state pre {
    width: 100%;
    margin: 0;
    white-space: pre-wrap;
    overflow: hidden;
  }

  .new-chooser,
  .confirm-delete {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 16px;
    overflow: hidden;
  }

  .new-chooser button {
    padding: 11px;
    text-align: left;
  }

  .confirm-delete p {
    margin: 0;
    color: var(--text);
    font-size: 0.78rem;
    line-height: 1.45;
  }

  .confirm-actions {
    justify-content: flex-end;
  }

  @container (max-width: 359px) {
    .switcher-header {
      padding: 9px;
    }

    .tabs {
      display: grid;
      grid-template-columns: repeat(3, minmax(0, 1fr));
    }

    .tabs button {
      padding-inline: 3px;
    }

    .switcher-content {
      padding: 9px;
    }

    .project-grid {
      grid-template-columns: 1fr;
      gap: 9px;
    }

    .project-card {
      display: grid;
      grid-template-columns: 82px minmax(0, 1fr);
    }

    .card-thumb {
      height: auto;
      min-height: 104px;
      border-right: 1px solid var(--bg-300);
      border-bottom: 0;
    }

    .card-body {
      padding: 9px;
    }

    .summary {
      -webkit-line-clamp: 1;
      line-clamp: 1;
    }
  }
</style>
