import assert from 'node:assert/strict';
import test from 'node:test';
import { buildMacroAstMapProjection } from './macroAstMap';

test('projects typed source-map nodes into existing part regions', () => {
  const projection = buildMacroAstMapProjection({
    modelManifest: {
      parts: [
        {
          partId: 'input-port',
          label: 'Input Port',
          parameterKeys: [],
          freecadObjectName: 'InputPort',
          kind: 'solid',
          editable: true,
        },
      ],
    } as any,
    uiSpec: { fields: [] },
    parameters: {},
    sourceNodes: [
      {
        id: 'verify:wall-thickness',
        kind: 'verify',
        label: 'wall-thickness',
        startByte: 12,
        endByte: 40,
      },
    ],
  });

  const part = projection.root.children.find((node) => node.id === 'part:input-port');
  assert.ok(part);
  const verify = projection.root.children.find((node) => node.id === 'verify:wall-thickness');

  assert.equal(part.kind, 'part');
  assert.deepEqual(part.children, []);
  assert.equal(verify?.kind, 'verify');
  assert.deepEqual(verify?.sourceRange, { startByte: 12, endByte: 40 });
});
