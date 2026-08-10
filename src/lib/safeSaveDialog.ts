import { showSafeSaveDialog } from './tauri/client';

export type SaveDialogOptions = {
  defaultPath: string;
  filters: Array<{ name: string; extensions: string[] }>;
};

export function createSaveDialogGate(
  show: (options: SaveDialogOptions) => Promise<string | null>,
) {
  let active = false;
  return async (options: SaveDialogOptions): Promise<string | null> => {
    if (active) throw new Error('A file dialog is already open. Close it before exporting again.');
    const filter = options.filters[0];
    if (!filter) throw new Error('Save dialog requires one file filter.');
    active = true;
    try {
      return await show(options);
    } finally {
      active = false;
    }
  };
}

export const safeSaveDialog = createSaveDialogGate(async (options) => {
  const filter = options.filters[0];
  return showSafeSaveDialog(options.defaultPath, filter.name, filter.extensions);
});
