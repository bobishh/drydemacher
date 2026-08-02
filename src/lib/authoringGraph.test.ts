import assert from 'node:assert/strict';
import test from 'node:test';
import { decodeAuthoringGraph } from './authoringGraph';

const graph = {
  sourceDigest: 'sha256:source',
  coreDigest: 'sha256:core',
  astNodes: [],
  features: [],
  dependencies: [],
  constraints: [],
  handles: [],
};

test('rejects editable target without exact feature and AST binding', () => {
  assert.throws(
    () =>
      decodeAuthoringGraph({
        ...graph,
        targets: [
          {
            targetId: 'face:derived:17',
            partId: 'body',
            viewerNodeId: 'body',
            label: 'Derived face',
            kind: 'face',
            editable: true,
            featureIds: [],
            sourceStableNodeKeys: [],
          },
        ],
      }),
    /Editable authoring target 'face:derived:17' lacks exact feature and AST binding\./,
  );
});

test('preserves backend non-editable reason without inferring editability', () => {
  const decoded = decodeAuthoringGraph({
    ...graph,
    targets: [
      {
        targetId: 'face:derived:17',
        partId: 'body',
        viewerNodeId: 'body',
        label: 'Derived face',
        kind: 'face',
        editable: false,
        nonEditableReason: 'No exact source binding for derived face.',
        featureIds: [],
        sourceStableNodeKeys: [],
      },
    ],
  });

  assert.equal(decoded.targets[0]?.editable, false);
  assert.equal(
    decoded.targets[0]?.nonEditableReason,
    'No exact source binding for derived face.',
  );
});
