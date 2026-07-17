import assert from 'node:assert/strict';
import test from 'node:test';

import { resolveDraftPreviewDesign } from './draftPreviewParams';
import type { DesignOutput } from '../types/domain';

function design(initialParams: Record<string, unknown>): DesignOutput {
  return {
    title: 'Woodlouse hotel',
    versionName: 'draft',
    response: '',
    interactionMode: 'design',
    macroCode: '(model)',
    macroDialect: 'ecky',
    engineKind: 'ecky',
    sourceLanguage: 'ecky',
    geometryBackend: 'freecad',
    uiSpec: { fields: [] },
    initialParams,
    postProcessing: null,
  } as DesignOutput;
}

test('Given an MCP preview for the active thread When it becomes current Then the panel uses the rendered preview params', () => {
  const preview = design({ length: 150, width: 92, svg_icon_width: 8 });
  const sameThread = resolveDraftPreviewDesign({
    design: preview,
  });
  const otherThread = resolveDraftPreviewDesign({
    design: preview,
  });

  assert.deepEqual(sameThread.initialParams, { length: 150, width: 92, svg_icon_width: 8 });
  assert.deepEqual(otherThread.initialParams, { length: 150, width: 92, svg_icon_width: 8 });
});
