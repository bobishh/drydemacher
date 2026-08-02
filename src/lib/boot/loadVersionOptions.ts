import type { LoadVersionOptions } from '../stores/history';

export const BOOT_LOAD_VERSION_OPTIONS = {
  rebuildMissingRuntime: false,
} satisfies LoadVersionOptions;
