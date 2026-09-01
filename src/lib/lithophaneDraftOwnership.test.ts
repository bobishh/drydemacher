import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const panelSource = readFileSync(new URL('./ParamPanel.svelte', import.meta.url), 'utf8');
const manualServiceSource = readFileSync(
  new URL('../../src-tauri/src/services/manual_code.rs', import.meta.url),
  'utf8',
);
const bindingsSource = readFileSync(
  new URL('../../src-tauri/src/bindings.rs', import.meta.url),
  'utf8',
);

test('lithophane UI mints local draft keys while manual apply allocates canonical IDs', () => {
  assert.match(panelSource, /return `draft-litho-\$\{crypto\.randomUUID\(\)\}`/);
  assert.doesNotMatch(panelSource, /return `litho-\$\{crypto\.randomUUID/);
  assert.match(manualServiceSource, /canonicalize_manual_post_processing/);
  assert.match(manualServiceSource, /format!\("litho-\{\}", uuid::Uuid::new_v4\(\)\)/);
  assert.doesNotMatch(bindingsSource, /update_post_processing/);
});
