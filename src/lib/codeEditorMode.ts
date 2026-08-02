export function usesPythonEditorMode(sourceLanguage: string | null | undefined): boolean {
  return sourceLanguage === 'legacyPython';
}
