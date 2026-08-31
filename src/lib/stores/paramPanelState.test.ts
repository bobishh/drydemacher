import assert from 'node:assert/strict';
import test, { afterEach } from 'node:test';
import { get } from 'svelte/store';

import type { DesignOutput } from '../types/domain';
import { paramPanelState } from './paramPanelState';
import { workingCopy } from './workingCopy';

function design(): DesignOutput {
  return {
    title: 'Bracket',
    versionName: 'v1',
    response: '',
    interactionMode: 'design',
    macroCode: '(box :size [10 20 30])',
    macroDialect: 'ecky',
    engineKind: 'ecky',
    sourceLanguage: 'ecky',
    geometryBackend: 'mesh',
    uiSpec: { fields: [{ key: 'width', label: 'Width', type: 'number', frozen: false }] },
    initialParams: { width: 10 },
  };
}

afterEach(() => workingCopy.reset());

test('Given a version enters working copy When panel projects state Then no second hydration is required', () => {
  workingCopy.loadVersion(design(), 'message-1');

  assert.equal(get(paramPanelState).versionId, 'message-1');
  assert.deepEqual(get(paramPanelState).params, { width: 10 });
  assert.equal(get(paramPanelState).uiSpec.fields[0]?.key, 'width');
});

test('Given panel parameters change When working copy is read Then both surfaces share one value', () => {
  workingCopy.loadVersion(design(), 'message-1');
  paramPanelState.setParams({ width: 24 });

  assert.deepEqual(get(workingCopy).params, { width: 24 });
  assert.deepEqual(get(paramPanelState).params, { width: 24 });
});
