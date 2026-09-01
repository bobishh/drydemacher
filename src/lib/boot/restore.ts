import { get } from 'svelte/store';
import { session } from '../stores/sessionStore';
import { workingCopy } from '../stores/workingCopy';
import { paramPanelState } from '../stores/paramPanelState';
import {
  historyStore as history,
  activeThreadIdStore as activeThreadId,
  activeVersionId,
  config,
  configLoaded,
  availableModels,
  isLoadingModels,
  runtimeCapabilities,
} from '../stores/domainState';
import {
  formatBackendError,
  getBootProjection,
  getBootRuntimeProjection,
  refreshModelCatalog,
  saveConfigProjection,
} from '../tauri/client';
import { activateWorkspaceProjection } from '../stores/history';

const BOOT_RESTORE_TIMEOUT_MS = 6000;
const BOOT_MODEL_LOAD_TIMEOUT_MS = 6000;

type TauriBridgeWindow = Window & typeof globalThis & {
  __TAURI_INTERNALS__?: {
    invoke?: unknown;
  };
};

function hasTauriInvokeBridge(): boolean {
  if (typeof window === 'undefined') return true;
  const bridge = (window as TauriBridgeWindow).__TAURI_INTERNALS__;
  return typeof bridge?.invoke === 'function';
}

/**
 * Main boot sequence for the application.
 * Restores configuration, history, and the last active design.
 */
export async function boot() {
  session.setPhase('booting');
  session.setStatus('Restoring environment...');
  configLoaded.set(false);

  const bootWatchdog = typeof window !== 'undefined'
    ? window.setTimeout(() => {
        if (get(session).phase === 'booting') {
          console.warn('[Boot] restore is still running.');
        }
      }, 1500)
    : 0;

  if (!hasTauriInvokeBridge()) {
    session.setPhase('idle');
    session.setStatus('System ready.');
    if (bootWatchdog) window.clearTimeout(bootWatchdog);
    return;
  }
  
  try {
    // Rust owns config normalization, bounded history, restart-pointer
    // resolution, and fallback-thread selection as one coherent projection.
    const projection = await withBootTimeout(
      getBootProjection(),
      BOOT_RESTORE_TIMEOUT_MS,
      'Boot projection lookup timed out',
    );
    config.set(projection.config);
    configLoaded.set(true);
    history.set(projection.history);

    // Probe runtime capabilities in the background. Cold FreeCAD/native probes
    // can be slow; cached model restore should not wait on them.
    const capabilitiesRefresh = refreshRuntimeCapabilities();

    // Restore Last Design. Runtime rebuild is intentionally skipped here:
    // boot must open the workbench even when old model assets are missing.
    await restoreBootProjection(projection);
    
    session.setPhase('idle');
    session.setStatus('System ready.');
    void capabilitiesRefresh;
  } catch (e) {
    console.error('[Boot] failed:', e);
    session.setPhase('error');
    session.setError('Boot failed: ' + e);
  } finally {
    if (bootWatchdog) window.clearTimeout(bootWatchdog);
  }
}

async function refreshRuntimeCapabilities() {
  try {
    const projection = await getBootRuntimeProjection();
    config.set(projection.config);
    runtimeCapabilities.set(projection.capabilities);
  } catch (e) {
    console.warn('[Boot] Runtime capability probe failed:', e);
  }
}

export async function saveConfig() {
  const currentConfig = get(config);
  session.setGlobalError(null);
  try {
    const projection = await saveConfigProjection(currentConfig);
    config.set(projection.config);
    runtimeCapabilities.set(projection.capabilities);
    session.setStatus('Configuration saved.');
  } catch (e) {
    session.setGlobalError(`Config Save Error: ${formatBackendError(e)}`);
    throw e;
  }
}

export async function fetchModels() {
  isLoadingModels.set(true);
  try {
    const projection = await refreshModelCatalog();
    availableModels.set(projection.models);
    config.set(projection.config);
  } catch (e) {
    console.error("[Config] Failed to fetch models:", e);
    availableModels.set([]);
    throw e;
  } finally {
    isLoadingModels.set(false);
  }
}

async function restoreBootProjection(
  projection: Awaited<ReturnType<typeof getBootProjection>>,
) {
  try {
    if (!projection.workspace) {
      resetToBlankSession();
      return;
    }

    await withBootTimeout(
      activateWorkspaceProjection(projection.workspace),
      BOOT_MODEL_LOAD_TIMEOUT_MS,
      'Last design runtime load timed out',
    ).catch((e) => {
      console.warn('[Boot] Last design runtime was not restored:', e);
      session.setStatus('Last design runtime unavailable.');
      return null;
    });

    if (projection.selectedPartId) {
      session.setSelectedPartId(projection.selectedPartId);
    }
  } catch (e) {
    console.error("[Boot] Failed to restore last design:", e);
    resetToBlankSession();
  }
}

function withBootTimeout<T>(promise: Promise<T>, timeoutMs: number, message: string): Promise<T> {
  let timeoutId: ReturnType<typeof setTimeout> | null = null;
  const timeout = new Promise<never>((_, reject) => {
    timeoutId = setTimeout(() => reject(new Error(message)), timeoutMs);
  });
  return Promise.race([promise, timeout]).finally(() => {
    if (timeoutId) clearTimeout(timeoutId);
  });
}

function resetToBlankSession() {
  activeThreadId.set(null);
  activeVersionId.set(null);
  workingCopy.reset();
  paramPanelState.reset();
  session.setStlUrl(null);
}
