<script lang="ts">
  let {
    searchQuery = '',
    onSearchQueryChange,
  }: {
    searchQuery?: string;
    onSearchQueryChange?: (value: string) => void;
  } = $props();
</script>

<div class="panel-search">
  <div class="search-box">
    <input
      type="text"
      placeholder="Search controls..."
      value={searchQuery}
      oninput={(event) => onSearchQueryChange?.((event.currentTarget as HTMLInputElement).value)}
      class="search-input"
    />
    {#if searchQuery}
      <button class="clear-search" onclick={() => onSearchQueryChange?.('')} aria-label="Clear search">✕</button>
    {/if}
  </div>
</div>

<style>
  .panel-search {
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .search-box {
    position: relative;
    width: 100%;
    overflow: hidden;
  }

  .search-input {
    width: 100%;
    min-height: 42px;
    padding: 10px 36px 10px 12px;
    background: var(--bg-100);
    border: 1px solid var(--bg-300);
    color: var(--text);
    font-size: 0.86rem;
    font-weight: 600;
    line-height: 1.2;
    outline: none;
    transition:
      border-color 0.2s,
      background-color 0.2s;
  }

  .search-input:focus {
    border-color: var(--primary);
    background: color-mix(in srgb, var(--bg-100) 88%, var(--primary) 12%);
  }

  .clear-search {
    position: absolute;
    right: 10px;
    top: 50%;
    transform: translateY(-50%);
    background: none;
    border: none;
    color: var(--text-dim);
    cursor: pointer;
    font-size: 0.95rem;
    padding: 0;
  }

  .clear-search:hover {
    color: var(--text);
  }
</style>
