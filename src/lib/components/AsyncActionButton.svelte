<script lang="ts">
  let {
    action,
    label,
    pendingLabel,
    disabled = false,
    className = '',
    title = undefined,
    ariaLabel = undefined,
    onerror = undefined,
  }: {
    action: () => Promise<void> | void;
    label: string;
    pendingLabel: string;
    disabled?: boolean;
    className?: string;
    title?: string;
    ariaLabel?: string;
    onerror?: (error: unknown) => void;
  } = $props();

  let pending = $state(false);

  async function run() {
    if (pending || disabled) return;
    pending = true;
    try {
      await action();
    } catch (error) {
      onerror?.(error);
    } finally {
      pending = false;
    }
  }
</script>

<button
  type="button"
  class={className}
  {title}
  aria-label={ariaLabel}
  aria-busy={pending}
  disabled={disabled || pending}
  onclick={run}
>
  {pending ? pendingLabel : label}
</button>

<style>
  button {
    border: 1px solid var(--bg-400);
    border-radius: 0;
    background: var(--bg-300);
    color: var(--text);
    font-family: var(--font-mono);
    font-size: 0.62rem;
    font-weight: 700;
    letter-spacing: 0.06em;
    padding: 5px 8px;
    cursor: pointer;
  }

  button:disabled {
    opacity: 0.55;
    cursor: wait;
  }

  button.danger,
  button.btn-danger {
    border-color: var(--red);
    color: var(--red);
  }

  button.compact {
    min-height: 26px;
    padding: 4px 6px;
    font-size: 0.58rem;
  }
</style>
