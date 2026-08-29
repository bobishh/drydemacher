import type { ArtifactBundle, DesignParams, ModelManifest, UiSpec } from '../types/domain';

type FreecadComponentSourceInput = {
  artifactBundle: ArtifactBundle;
  manifest: ModelManifest;
  parameters: DesignParams;
  uiSpec: UiSpec;
};

function sourceKindKeyword(manifest: ModelManifest): ':fcstd' | ':step' {
  return manifest.sourceKind === 'importedFcstd' ? ':fcstd' : ':step';
}

function encodeValue(value: DesignParams[string]): string {
  if (value === true) return '#t';
  if (value === false) return '#f';
  if (value === null) return 'nil';
  return JSON.stringify(value);
}

function decodeValue(source: string): DesignParams[string] {
  if (source === '#t') return true;
  if (source === '#f') return false;
  if (source === 'nil') return null;
  const value: unknown = JSON.parse(source);
  if (typeof value === 'string' || typeof value === 'number' || typeof value === 'boolean' || value === null) {
    return value;
  }
  throw new Error(`Unsupported component parameter value: ${source}`);
}

function stringField(source: string, field: string): string | null {
  const match = source.match(new RegExp(`:${field}\\s+("(?:[^"\\\\]|\\\\.)*")`));
  if (!match) return null;
  return JSON.parse(match[1]);
}

export function looksLikeFreecadComponentSource(source: string): boolean {
  return /^\s*\(freecad-component(?:\s|\n)/.test(source) && /\)\s*$/.test(source);
}

export function buildFreecadComponentSource(input: FreecadComponentSourceInput): string {
  const { artifactBundle, manifest, parameters, uiSpec } = input;
  const parameterLines = uiSpec.fields.map((field) =>
    `    (param ${field.key} ${encodeValue(parameters[field.key] ?? null)})`,
  );
  const bindingLines = (manifest.parts ?? []).map((part) => {
    const keys = (part.parameterKeys ?? []).map((key) => JSON.stringify(key)).join(' ');
    return `    (part ${JSON.stringify(part.partId)} :object ${JSON.stringify(part.freecadObjectName ?? part.label ?? part.partId)} :editable ${part.editable ? '#t' : '#f'} :parameters (${keys}))`;
  });

  return [
    '(freecad-component',
    `  :model-id ${JSON.stringify(artifactBundle.modelId)}`,
    `  :source-kind ${sourceKindKeyword(manifest)}`,
    `  :source-path ${JSON.stringify(manifest.document.sourcePath ?? artifactBundle.fcstdPath)}`,
    `  :source-digest ${JSON.stringify(manifest.sourceDigest ?? '')}`,
    `  :content-hash ${JSON.stringify(artifactBundle.contentHash)}`,
    '  :parameters (',
    ...parameterLines,
    '  )',
    '  :bindings (',
    ...bindingLines,
    '  ))',
  ].join('\n');
}

export function parseFreecadComponentSource(
  source: string,
  expectedBundle: ArtifactBundle,
  uiSpec: UiSpec,
): DesignParams {
  if (!looksLikeFreecadComponentSource(source)) {
    throw new Error('Expected `(freecad-component ...)` source.');
  }
  const modelId = stringField(source, 'model-id');
  const contentHash = stringField(source, 'content-hash');
  if (modelId !== expectedBundle.modelId || contentHash !== expectedBundle.contentHash) {
    throw new Error('FreeCAD component identity is read-only. Import a different component instead of changing its identity fields.');
  }

  const allowedKeys = new Set(uiSpec.fields.map((field) => field.key));
  const parameters: DesignParams = {};
  const paramPattern = /^\s*\(param\s+([^\s()]+)\s+(.+)\)\s*$/gm;
  for (const match of source.matchAll(paramPattern)) {
    const key = match[1];
    if (!allowedKeys.has(key)) {
      throw new Error(`Unknown FreeCAD component parameter: ${key}`);
    }
    parameters[key] = decodeValue(match[2].trim());
  }
  for (const key of allowedKeys) {
    if (!(key in parameters)) throw new Error(`Missing FreeCAD component parameter: ${key}`);
  }
  return parameters;
}
