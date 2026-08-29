import assert from 'node:assert/strict';
import { test } from 'node:test';
import { buildOwnershipSections, provenanceOverlayControls, provenanceOverlayPatch } from './ownershipSections';

const fields = Array.from({ length: 49 }, (_, index) => ({
  type: 'number' as const,
  key: `dryer_param_${index}`,
  label: `Dryer Param ${index}`,
  frozen: false,
}));

const manifest = {
  modelId: 'dryer',
  sourceKind: 'generated',
  engineKind: 'ecky',
  sourceLanguage: 'ecky',
  geometryBackend: 'directOcct',
  parts: [
    { partId: 'drum', label: 'Drum', editable: true, parameterKeys: fields.slice(0, 12).map((field) => field.key) },
    { partId: 'shell', label: 'Shell', editable: true, parameterKeys: [fields[0]!.key, ...fields.slice(12, 30).map((field) => field.key)] },
    { partId: 'air-path', label: 'Air Path', editable: true, parameterKeys: fields.slice(30, 49).map((field) => field.key) },
  ],
  parameterGroups: [],
} as any;

test('Given a 49-parameter Ecky manifest When ownership projects Then shared keys render once and dense parts collapse', () => {
  const sections = buildOwnershipSections({ manifest, fields, selectedTarget: null, searchQuery: '' });

  assert.deepEqual(sections.map((section) => section.sectionId), [
    'model:parameters',
    'part:drum',
    'part:shell',
    'part:air-path',
  ]);
  assert.deepEqual(sections[0]?.fields.map((field) => field.key), ['dryer_param_0']);
  assert.equal(
    sections.flatMap((section) => section.fields).filter((field) => field.key === 'dryer_param_0').length,
    1,
  );
  assert.equal(sections.find((section) => section.sectionId === 'part:drum')?.collapsed, true);
  assert.equal(sections.find((section) => section.sectionId === 'part:shell')?.collapsed, true);
});

test('Given exact drum selection When ownership projects Then drum foregrounds expanded and unrelated sections collapse', () => {
  const sections = buildOwnershipSections({
    manifest,
    fields,
    selectedTarget: {
      targetId: 'drum:face:bore',
      aliasIds: [],
      kind: 'face',
      partId: 'drum',
      label: 'Drum Bore',
      editable: true,
      viewerNodeId: 'drum-node',
      parameterKeys: ['dryer_param_2'],
      primitiveIds: [],
      viewIds: [],
    },
    searchQuery: '',
  });

  assert.equal(sections[0]?.sectionId, 'part:drum');
  assert.equal(sections[0]?.selected, true);
  assert.equal(sections[0]?.collapsed, false);
  assert.deepEqual(sections[0]?.visibleFields.map((field) => field.key), ['dryer_param_2']);
  assert.ok(sections.slice(1).every((section) => section.collapsed));
});

test('Given named shape groups When ownership projects Then specific groups consume controls once before part remainder', () => {
  const sections = buildOwnershipSections({
    manifest: {
      ...manifest,
      parameterGroups: [{
        groupId: 'shape:drum:bore',
        label: 'Drum Bore',
        partIds: ['drum'],
        parameterKeys: ['dryer_param_2', 'dryer_param_3'],
        editable: true,
        order: 1,
      }],
    },
    fields,
    selectedTarget: null,
    searchQuery: '',
  });

  assert.deepEqual(
    sections.find((section) => section.sectionId === 'shape:drum:bore')?.fields.map((field) => field.key),
    ['dryer_param_2', 'dryer_param_3'],
  );
  assert.ok(
    sections.find((section) => section.sectionId === 'part:drum')?.fields.every(
      (field) => field.key !== 'dryer_param_2' && field.key !== 'dryer_param_3',
    ),
  );
  assert.equal(
    sections.flatMap((section) => section.fields).filter((field) => field.key === 'dryer_param_2').length,
    1,
  );
});

test('Given ambiguous face When ownership projects Then no part controls foreground as fallback', () => {
  const sections = buildOwnershipSections({
    manifest,
    fields,
    selectedTarget: {
      targetId: 'drum:face:ambiguous', aliasIds: [], kind: 'face', partId: 'drum', label: 'Ambiguous',
      editable: true, viewerNodeId: 'drum-node', parameterKeys: [], primitiveIds: [], viewIds: [],
    },
    searchQuery: '',
  });
  assert.ok(sections.every((section) => !section.selected && section.collapsed));
});

test('Given exact generated Ecky provenance When overlay controls project Then only exact fields render', () => {
  const controls = provenanceOverlayControls({
    manifest,
    fields,
    parameters: Object.fromEntries(fields.map((field, index) => [field.key, index])),
    target: {
      targetId: 'drum:face:bore', aliasIds: [], kind: 'face', partId: 'drum', label: 'Drum Bore',
      editable: true, viewerNodeId: 'drum-node', parameterKeys: ['dryer_param_2'], primitiveIds: [], viewIds: [],
    },
  });
  assert.deepEqual(controls.map((control) => control.rawField?.key), ['dryer_param_2']);

  const ambiguous = provenanceOverlayControls({
    manifest,
    fields,
    parameters: {},
    target: {
      targetId: 'drum:face:ambiguous', aliasIds: [], kind: 'face', partId: 'drum', label: 'Ambiguous',
      editable: true, viewerNodeId: 'drum-node', parameterKeys: [], primitiveIds: [], viewIds: [],
    },
  });
  assert.deepEqual(ambiguous, []);
  assert.deepEqual(provenanceOverlayPatch(controls, 'ast-param:dryer_param_2', 77), {
    dryer_param_2: 77,
  });
  assert.deepEqual(provenanceOverlayPatch(controls, 'ast-param:dryer_param_3', 77), {});
});
