import { convertFileSrc } from '@tauri-apps/api/core';

type FileSourceConverter = (path: string) => string;

const DIRECT_PREVIEW_SOURCE = /^(data:image\/|blob:|https?:|asset:|tauri:)/i;

export function toPreviewSrc(
  raw: string | null | undefined,
  convert: FileSourceConverter = convertFileSrc,
): string | null {
  const value = raw?.trim();
  if (!value) return null;
  if (DIRECT_PREVIEW_SOURCE.test(value)) return value;

  try {
    return convert(value);
  } catch {
    return value;
  }
}
