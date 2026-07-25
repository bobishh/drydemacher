import { readFileSync, writeFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import { generateLandingModule, validateAnimalCapCatalog } from './animal_cap_catalog.mjs';

const scriptDir = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(scriptDir, '..');
const catalogRoot = resolve(repoRoot, 'catalogs', 'animal-caps');
const catalogPath = resolve(catalogRoot, 'catalog.json');
const outputPath = resolve(
  repoRoot,
  'sites',
  'landing',
  'src',
  'showcase',
  'animalCapShowcase.generated.ts',
);
const check = process.argv.includes('--check');
const catalog = JSON.parse(readFileSync(catalogPath, 'utf8'));
const errors = validateAnimalCapCatalog(catalog, { rootDir: catalogRoot, verifyFiles: true });

if (errors.length > 0) {
  process.stderr.write(`${errors.map((error) => `- ${error}`).join('\n')}\n`);
  process.exitCode = 1;
} else {
  const next = generateLandingModule(catalog);
  if (check) {
    const current = readFileSync(outputPath, 'utf8');
    if (current !== next) {
      process.stderr.write('Animal cap landing projection is stale.\n');
      process.exitCode = 1;
    }
  } else {
    writeFileSync(outputPath, next);
    process.stdout.write(`Wrote ${outputPath}\n`);
  }
}
