<script lang="ts">
  import { convertFileSrc } from '@tauri-apps/api/core';
  import { open } from '@tauri-apps/plugin-dialog';
  import { openInventoryThread } from './stores/history';
  import {
    formatBackendError,
    getAnimalCapCatalog,
    getConfig,
    installComponentPackageArchive,
    listInstalledComponentPackageHeaders,
    saveConfig,
    searchFreecadLibrary,
  } from './tauri/client';
  import type {
    AnimalCapCatalogEntry,
    ComponentPackageHeader,
    FreecadLibraryItem,
  } from './tauri/contracts';

  let {
    onImportFreecadLibraryPart,
  }: {
    onImportFreecadLibraryPart?: (item: FreecadLibraryItem) => Promise<void> | void;
  } = $props();

  type LibraryTab = 'components' | 'freecad' | 'catalog';

  let activeTab = $state<LibraryTab>('components');
  let searchQuery = $state('');
  let loading = $state(false);
  let loadError = $state<string | null>(null);

  let packageHeaders = $state<ComponentPackageHeader[]>([]);
  let packagesLoaded = $state(false);
  let packageImportBusy = $state(false);

  let freecadLibraryRoots = $state<string[]>([]);
  let freecadLibraryResults = $state<FreecadLibraryItem[]>([]);
  let freecadLoaded = $state(false);
  let freecadLibrarySearchBusy = $state(false);
  let freecadLibraryFolderBusy = $state(false);
  let importingFreecadLibraryItemId = $state<string | null>(null);

  let animalCapEntries = $state<AnimalCapCatalogEntry[]>([]);
  let catalogLoaded = $state(false);

  async function loadActiveTab() {
    if (
      (activeTab === 'components' && packagesLoaded) ||
      (activeTab === 'freecad' && freecadLoaded) ||
      (activeTab === 'catalog' && catalogLoaded)
    ) {
      return;
    }

    loading = true;
    loadError = null;
    try {
      if (activeTab === 'components') {
        packageHeaders = await listInstalledComponentPackageHeaders();
        packagesLoaded = true;
      } else if (activeTab === 'freecad') {
        const config = await getConfig();
        freecadLibraryRoots = config.freecadLibraryRoots ?? [];
        freecadLoaded = true;
      } else {
        const catalog = await getAnimalCapCatalog();
        animalCapEntries = catalog?.entries ?? [];
        catalogLoaded = true;
      }
    } catch (error) {
      loadError = formatBackendError(error);
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    void loadActiveTab();
  });

  const filteredPackages = $derived(
    packageHeaders.filter((pkg) =>
      [
        pkg.displayName,
        pkg.packageId,
        pkg.version,
        ...(pkg.tags ?? []),
      ].join(' ').toLowerCase().includes(searchQuery.toLowerCase()),
    ),
  );

  const filteredFreecadResults = $derived(
    freecadLibraryResults.filter((item) =>
      [
        item.name,
        item.categoryPath,
        ...(item.tags ?? []),
        ...(item.formats ?? []),
      ].join(' ').toLowerCase().includes(searchQuery.toLowerCase()),
    ),
  );

  const filteredCatalog = $derived(
    animalCapEntries.filter((entry) =>
      [
        entry.displayName,
        entry.species,
        entry.source.author,
        entry.source.license,
      ].join(' ').toLowerCase().includes(searchQuery.toLowerCase()),
    ),
  );

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

  function packageStats(pkg: ComponentPackageHeader) {
    const count = (value: number, singular: string, plural = `${singular}s`) =>
      `${value} ${value === 1 ? singular : plural}`;
    return [
      count(pkg.components?.length ?? 0, 'component'),
      count(pkg.portTypes?.length ?? 0, 'port type'),
      count(pkg.assemblies?.length ?? 0, 'assembly', 'assemblies'),
    ].join(' / ');
  }

  async function handleImportPackageArchive() {
    if (packageImportBusy) return;
    loadError = null;
    const selected = await open({
      multiple: false,
      filters: [{ name: 'Ecky Package', extensions: ['ecky', 'zip'] }],
    });
    if (typeof selected !== 'string' || !selected.trim()) return;

    packageImportBusy = true;
    try {
      await installComponentPackageArchive(selected);
      packageHeaders = await listInstalledComponentPackageHeaders();
      packagesLoaded = true;
    } catch (error) {
      loadError = formatBackendError(error);
    } finally {
      packageImportBusy = false;
    }
  }

  async function handleSetFreecadLibraryFolder() {
    if (freecadLibraryFolderBusy) return;
    loadError = null;
    const selected = await open({ directory: true, multiple: false });
    if (typeof selected !== 'string' || !selected.trim()) return;

    freecadLibraryFolderBusy = true;
    try {
      const config = await getConfig();
      const roots = [selected.trim()];
      await saveConfig({ ...config, freecadLibraryRoots: roots });
      freecadLibraryRoots = roots;
      freecadLibraryResults = [];
    } catch (error) {
      loadError = formatBackendError(error);
    } finally {
      freecadLibraryFolderBusy = false;
    }
  }

  async function handleSearchFreecadLibrary() {
    if (freecadLibrarySearchBusy) return;
    loadError = null;
    freecadLibrarySearchBusy = true;
    try {
      freecadLibraryResults = await searchFreecadLibrary({
        query: searchQuery,
        roots: freecadLibraryRoots,
        limit: 60,
      });
    } catch (error) {
      loadError = formatBackendError(error);
      freecadLibraryResults = [];
    } finally {
      freecadLibrarySearchBusy = false;
    }
  }

  async function importFreecadLibraryItem(item: FreecadLibraryItem) {
    if (!onImportFreecadLibraryPart || importingFreecadLibraryItemId) return;
    importingFreecadLibraryItemId = item.id;
    try {
      await onImportFreecadLibraryPart(item);
    } finally {
      importingFreecadLibraryItemId = null;
    }
  }

  function retry() {
    loadError = null;
    if (activeTab === 'components') packagesLoaded = false;
    if (activeTab === 'freecad') freecadLoaded = false;
    if (activeTab === 'catalog') catalogLoaded = false;
    void loadActiveTab();
  }
</script>

<div class="library-panel">
  <header class="library-header">
    <nav class="library-tabs" aria-label="Library sections">
      <button class:active={activeTab === 'components'} onclick={() => activeTab = 'components'}>
        COMPONENT PACKAGES
      </button>
      <button class:active={activeTab === 'freecad'} onclick={() => activeTab = 'freecad'}>
        FREECAD PARTS
      </button>
      <button class:active={activeTab === 'catalog'} onclick={() => activeTab = 'catalog'}>
        CATALOG
      </button>
    </nav>
    <div class="library-tools">
      <input class="search-input" type="search" placeholder="Search library..." bind:value={searchQuery} />
      {#if activeTab === 'components'}
        <button class="primary-action" onclick={handleImportPackageArchive} disabled={packageImportBusy}>
          {packageImportBusy ? 'IMPORTING...' : 'IMPORT PACKAGE'}
        </button>
      {:else if activeTab === 'freecad'}
        <button class="primary-action" onclick={handleSearchFreecadLibrary} disabled={freecadLibrarySearchBusy}>
          {freecadLibrarySearchBusy ? 'SEARCHING...' : 'SEARCH'}
        </button>
      {/if}
    </div>
  </header>

  <main class="library-content">
    {#if loading}
      <div class="state">LOADING LIBRARY...</div>
    {:else if loadError}
      <div class="error-state">
        <strong>LIBRARY ERROR</strong>
        <pre>{loadError}</pre>
        <button onclick={retry}>RETRY</button>
      </div>
    {:else if activeTab === 'components'}
      {#if filteredPackages.length === 0}
        <div class="state">NO COMPONENT PACKAGES</div>
      {:else}
        <div class="library-grid">
          {#each filteredPackages as pkg (pkg.packageId + pkg.version)}
            <article class="library-card">
              <div class="card-title">
                <h3>{pkg.displayName}</h3>
                <span>{pkg.visibility}</span>
              </div>
              <p>{pkg.packageId} / {pkg.version}</p>
              <div class="card-stats">{packageStats(pkg)}</div>
              {#if pkg.tags?.length}
                <div class="tag-list">
                  {#each pkg.tags as tag}
                    <span>{tag}</span>
                  {/each}
                </div>
              {/if}
            </article>
          {/each}
        </div>
      {/if}
    {:else if activeTab === 'freecad'}
      <section class="source-config">
        <div>
          <strong>LOCAL SOURCE</strong>
          <p>{freecadLibraryRoots[0] || 'NO LOCAL FOLDER CONFIGURED'}</p>
        </div>
        <button onclick={handleSetFreecadLibraryFolder} disabled={freecadLibraryFolderBusy}>
          {freecadLibraryRoots.length ? 'CHANGE FOLDER' : 'SET FOLDER'}
        </button>
      </section>
      {#if filteredFreecadResults.length === 0}
        <div class="state">SEARCH LOCAL FREECAD PARTS</div>
      {:else}
        <div class="result-list">
          {#each filteredFreecadResults as item (item.id)}
            <article class="result-row">
              <div>
                <h3>{item.name}</h3>
                <p>{item.categoryPath}</p>
              </div>
              <button
                aria-label={`${importingFreecadLibraryItemId === item.id ? 'IMPORTING' : 'IMPORT'} ${item.name}`}
                onclick={() => importFreecadLibraryItem(item)}
                disabled={Boolean(importingFreecadLibraryItemId)}
              >
                {importingFreecadLibraryItemId === item.id ? 'IMPORTING...' : 'IMPORT'}
              </button>
            </article>
          {/each}
        </div>
      {/if}
    {:else}
      {#if filteredCatalog.length === 0}
        <div class="state">NO CATALOG ITEMS</div>
      {:else}
        <div class="library-grid">
          {#each filteredCatalog as entry (entry.id)}
            <article class="library-card catalog-card">
              {#if previewSrc(entry.artifact?.previewPath)}
                <img src={previewSrc(entry.artifact?.previewPath) ?? ''} alt={`${entry.displayName} preview`} />
              {/if}
              <div class="catalog-copy">
                <div class="card-title">
                  <h3>{entry.displayName}</h3>
                  <span>{entry.artifact?.verificationStatus === 'passed' ? 'VERIFIED' : 'CANDIDATE'}</span>
                </div>
                <p>{entry.species} · {entry.source.license}</p>
                {#if entry.recipe?.boreProfileId}
                  <div class="card-stats">{entry.recipe.boreProfileId}</div>
                {/if}
                {#if entry.artifact?.threadId}
                  <button onclick={() => openInventoryThread(entry.artifact?.threadId ?? '')}>VIEW MODEL</button>
                {/if}
              </div>
            </article>
          {/each}
        </div>
      {/if}
    {/if}
  </main>
</div>

<style>
  .library-panel {
    container-type: inline-size;
    display: flex;
    flex-direction: column;
    height: 100%;
    min-width: 0;
    background: var(--bg-100);
    color: var(--text);
    overflow: hidden;
  }

  .library-header {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 12px;
    border-bottom: 1px solid var(--bg-300);
    overflow: hidden;
  }

  .library-tabs,
  .library-tools {
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
    font-size: 0.64rem;
    font-weight: 700;
    letter-spacing: 0.06em;
    padding: 6px 9px;
    cursor: pointer;
  }

  button:hover:not(:disabled),
  button.active {
    border-color: var(--primary);
    color: var(--primary);
  }

  button:disabled {
    opacity: 0.55;
    cursor: wait;
  }

  .search-input {
    flex: 1;
    min-width: 0;
    padding: 7px 9px;
    border: 1px solid var(--bg-300);
    background: var(--bg-200);
    color: var(--text);
  }

  .primary-action {
    flex: 0 0 auto;
    border-color: var(--secondary);
    color: var(--secondary);
  }

  .library-content {
    flex: 1;
    min-width: 0;
    min-height: 0;
    padding: 14px;
    overflow-y: auto;
  }

  .library-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(230px, 1fr));
    gap: 12px;
    overflow: hidden;
  }

  .library-card,
  .source-config,
  .result-row,
  .error-state {
    border: 1px solid var(--bg-300);
    background: var(--bg-200);
    overflow: hidden;
  }

  .library-card {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 12px;
  }

  .card-title,
  .source-config,
  .result-row {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 10px;
  }

  h3,
  p {
    margin: 0;
  }

  h3 {
    font-size: 0.82rem;
  }

  p,
  .card-stats {
    color: var(--text-dim);
    font-size: 0.68rem;
    line-height: 1.4;
  }

  .card-title span {
    color: var(--secondary);
    font-size: 0.56rem;
  }

  .tag-list {
    display: flex;
    flex-wrap: wrap;
    gap: 5px;
    overflow: hidden;
  }

  .tag-list span {
    padding: 2px 5px;
    border: 1px solid var(--bg-400);
    color: var(--text-dim);
    font-size: 0.58rem;
  }

  .source-config {
    align-items: center;
    padding: 12px;
    margin-bottom: 12px;
  }

  .result-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
    overflow: hidden;
  }

  .result-row {
    align-items: center;
    padding: 10px 12px;
  }

  .catalog-card {
    padding: 0;
  }

  .catalog-card img {
    width: 100%;
    height: 140px;
    object-fit: cover;
    background: #000;
  }

  .catalog-copy {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 12px;
    overflow: hidden;
  }

  .state {
    display: grid;
    min-height: 160px;
    place-items: center;
    color: var(--text-dim);
    font-size: 0.72rem;
    overflow: hidden;
  }

  .error-state {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 10px;
    padding: 12px;
    color: var(--red);
  }

  .error-state pre {
    width: 100%;
    margin: 0;
    white-space: pre-wrap;
    overflow: hidden;
  }

  @container (max-width: 399px) {
    .library-tabs {
      display: grid;
      grid-template-columns: 1fr;
    }

    .library-tools {
      flex-wrap: wrap;
    }

    .primary-action {
      width: 100%;
    }

    .library-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
