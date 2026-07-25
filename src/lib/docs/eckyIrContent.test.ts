import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import test from 'node:test';

import { projectEckyIrContent } from '../../../scripts/ecky_ir_content';

const root = process.cwd();

test('canonical corpus projects campaign, human reference, and LLM reference without overlap', () => {
  const corpus = fs.readFileSync(
    path.join(root, 'docs/books/ecky-ir/ecky-ir-corpus.md'),
    'utf8',
  );
  const projected = projectEckyIrContent(corpus);

  assert.match(projected.campaign, /^# Ecky Campaign$/m);
  assert.match(projected.campaign, /^## Level 01:/m);
  assert.match(projected.campaign, /^## Level 05: Perforated Toothbrush Holder$/m);
  assert.doesNotMatch(projected.campaign, /Appendix: Language Reference/);
  assert.doesNotMatch(projected.campaign, /ECKY_AGENT_REFERENCE/);

  assert.match(projected.reference, /^# Ecky Language Reference$/m);
  assert.match(projected.reference, /^## Operation Index$/m);
  assert.match(projected.reference, /^## Primitive Signatures$/m);
  assert.doesNotMatch(projected.reference, /First Solid: Ball on a Base/);
  assert.doesNotMatch(projected.reference, /ECKY_AGENT_REFERENCE/);

  assert.match(projected.agentReference, /^# Ecky language reference$/m);
  assert.match(projected.agentReference, /Return one complete `\(model \.\.\.\)` program/);
  assert.doesNotMatch(projected.agentReference, /Rendered output/);
});

test('published campaign and references equal their canonical projections', () => {
  const corpus = fs.readFileSync(
    path.join(root, 'docs/books/ecky-ir/ecky-ir-corpus.md'),
    'utf8',
  );
  const projected = projectEckyIrContent(corpus);

  assert.equal(
    fs.readFileSync(path.join(root, 'public/tutorials/ecky-campaign.md'), 'utf8'),
    projected.campaign,
  );
  assert.equal(
    fs.readFileSync(path.join(root, 'public/docs/ecky-ir.md'), 'utf8'),
    projected.reference,
  );
  assert.equal(
    fs.readFileSync(path.join(root, 'public/docs/ecky-agent-reference.md'), 'utf8'),
    projected.agentReference,
  );
});
