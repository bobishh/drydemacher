import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

import type { DesignOutput } from '../src/lib/types/domain.js';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const generatedPromptPath = path.resolve(
  __dirname,
  '../docs/generated/ecky-agent-system-prompt.md',
);

const DESIGN_JSON_CONTRACT = `Return a JSON object with these keys:
1. "title": short descriptive title.
2. "versionName": short iteration label.
3. "response": concise user summary.
4. "interactionMode": "design" or "question".
5. "macroCode": one complete .ecky (model ...) program.
6. "uiSpec": object with a "fields" array.
7. "initialParams": object whose keys exactly match uiSpec.

Use camelCase keys. Never emit snake_case transport keys.`;

export const MODEL_SYSTEM_PROMPT = `${readFileSync(generatedPromptPath, 'utf8').trim()}

# Response envelope

${DESIGN_JSON_CONTRACT}`;

export function buildUserPrompt(userPrompt: string): string {
  return `User request: ${userPrompt}\n\nGenerate the design JSON exactly in the required schema.`;
}

export type ServerModelOutput = Pick<
  DesignOutput,
  'title' | 'versionName' | 'response' | 'interactionMode' | 'macroCode' | 'uiSpec' | 'initialParams'
>;
