<script lang="ts">
  import StlViewer from '../StlViewer.svelte';
  import {
    animalCapShowcaseEntries,
    type AnimalCapShowcaseEntry,
  } from './animalCapShowcase.generated';

  let selectedId = $state(animalCapShowcaseEntries[0]?.id ?? '');
  const selected = $derived(
    animalCapShowcaseEntries.find((entry) => entry.id === selectedId) ?? animalCapShowcaseEntries[0],
  );

  function selectEntry(entry: AnimalCapShowcaseEntry) {
    selectedId = entry.id;
  }
</script>

{#if selected}
  <div
    class="animal-workbench"
    data-testid="animal-cap-workbench"
    data-selected-animal-cap={selected.id}
  >
    <header class="animal-head">
      <div>
        <span>CURATED ANIMAL CAP</span>
        <strong>{selected.displayName.toUpperCase()}</strong>
      </div>
      <div class="verification">
        <span>{selected.verificationStatus.toUpperCase()}</span>
        <strong>{selected.verifiedTriangleCount} TRIANGLES</strong>
      </div>
    </header>

    {#if animalCapShowcaseEntries.length > 1}
      <div class="animal-picker" role="group" aria-label="Animal valve cap">
        {#each animalCapShowcaseEntries as entry}
          <button
            type="button"
            aria-pressed={entry.id === selected.id}
            class:active={entry.id === selected.id}
            onclick={() => selectEntry(entry)}
          >
            {entry.species}
          </button>
        {/each}
      </div>
    {/if}

    <div class="animal-body">
      <div class="animal-viewport">
        <StlViewer
          size={560}
          parts={[{ url: selected.stlUrl, color: '#b99c60' }]}
        />
        <span class="orbit-hint">DRAG TO ORBIT</span>
      </div>

      <aside class="animal-meta">
        <div>
          <span>FIT PROFILE</span>
          <strong>{selected.boreProfileId.toUpperCase()}</strong>
        </div>
        <div>
          <span>TRANSFORM</span>
          <strong>UNIFORM SCALE {selected.uniformScale}</strong>
        </div>
        <div>
          <span>BORE</span>
          <strong>{selected.boreAxis.toUpperCase()} AXIS · {selected.boreAxisHeightMm} MM HIGH</strong>
        </div>
        <div>
          <span>SOURCE</span>
          <a href={selected.sourcePageUrl} target="_blank" rel="noreferrer">
            {selected.sourceAuthor} · {selected.license}
          </a>
        </div>
        <div>
          <span>MODEL</span>
          <strong>{selected.modelId}</strong>
        </div>
      </aside>
    </div>

    <footer class="animal-downloads">
      <a href={selected.stlUrl} download={selected.stlDownloadName}>DOWNLOAD {selected.species.toUpperCase()} STL</a>
      <a href={selected.sourceUrl} download={selected.sourceDownloadName}>DOWNLOAD {selected.species.toUpperCase()} SOURCE</a>
    </footer>
  </div>
{/if}

<style>
  .animal-workbench {
    border: 1px solid rgba(199, 169, 91, 0.52);
    background: #0b0e19;
    box-shadow: 12px 12px 0 rgba(4, 6, 12, 0.72);
    overflow: hidden;
  }

  .animal-head {
    min-height: 68px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 18px;
    padding: 14px 18px;
    border-bottom: 1px solid rgba(199, 169, 91, 0.3);
    background: #12172a;
    overflow: hidden;
  }

  .animal-head > div {
    display: grid;
    gap: 5px;
    min-width: 0;
    overflow: hidden;
  }

  .animal-head span,
  .animal-meta span,
  .orbit-hint {
    color: #c7a95b;
    font: 700 0.67rem/1.1 ui-monospace, SFMono-Regular, Menlo, monospace;
    letter-spacing: 0.12em;
  }

  .animal-head strong {
    color: #f1ead9;
    font: 700 0.88rem/1.25 ui-monospace, SFMono-Regular, Menlo, monospace;
    overflow-wrap: anywhere;
  }

  .verification {
    text-align: right;
  }

  .verification span {
    color: #77b887;
  }

  .animal-picker {
    display: flex;
    gap: 8px;
    padding: 10px 14px;
    border-bottom: 1px solid rgba(199, 169, 91, 0.22);
    overflow: auto hidden;
  }

  .animal-picker button {
    border: 1px solid #363c59;
    background: #151a30;
    color: #aea891;
    padding: 8px 12px;
    font: 700 0.72rem ui-monospace, SFMono-Regular, Menlo, monospace;
  }

  .animal-picker button.active {
    border-color: #c7a95b;
    color: #f1ead9;
  }

  .animal-body {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 260px;
    min-height: 470px;
    overflow: hidden;
  }

  .animal-viewport {
    position: relative;
    min-width: 0;
    min-height: 430px;
    border-right: 1px solid rgba(199, 169, 91, 0.22);
    overflow: hidden;
  }

  .animal-viewport :global(.viewer) {
    width: 100% !important;
    height: 100% !important;
    min-height: 430px;
  }

  .orbit-hint {
    position: absolute;
    right: 14px;
    bottom: 12px;
    color: #777b91;
  }

  .animal-meta {
    display: grid;
    align-content: start;
    background: #101426;
    overflow: hidden;
  }

  .animal-meta > div {
    display: grid;
    gap: 7px;
    padding: 16px;
    border-bottom: 1px solid rgba(199, 169, 91, 0.16);
    min-width: 0;
    overflow: hidden;
  }

  .animal-meta strong,
  .animal-meta a {
    color: #e1dac9;
    font: 650 0.76rem/1.45 ui-monospace, SFMono-Regular, Menlo, monospace;
    overflow-wrap: anywhere;
  }

  .animal-meta a {
    color: #aab7d3;
  }

  .animal-downloads {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
    padding: 13px 14px;
    border-top: 1px solid rgba(199, 169, 91, 0.3);
    background: #12172a;
    overflow: hidden;
  }

  .animal-downloads a {
    border: 1px solid #4a506b;
    color: #e1dac9;
    padding: 10px 13px;
    font: 700 0.72rem ui-monospace, SFMono-Regular, Menlo, monospace;
    text-decoration: none;
  }

  .animal-downloads a:first-child {
    border-color: #c7a95b;
    background: #c7a95b;
    color: #111522;
  }

  @media (max-width: 760px) {
    .animal-head {
      align-items: flex-start;
    }

    .animal-body {
      grid-template-columns: 1fr;
    }

    .animal-viewport {
      min-height: 360px;
      border-right: 0;
      border-bottom: 1px solid rgba(199, 169, 91, 0.22);
    }

    .animal-viewport :global(.viewer) {
      min-height: 360px;
    }

    .animal-meta {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }

  @media (max-width: 480px) {
    .animal-head {
      display: grid;
    }

    .verification {
      text-align: left;
    }

    .animal-meta {
      grid-template-columns: 1fr;
    }

    .animal-downloads {
      display: grid;
    }
  }
</style>
