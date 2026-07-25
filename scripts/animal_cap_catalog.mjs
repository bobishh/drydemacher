import { createHash } from 'node:crypto';
import { existsSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';

const SHA256 = /^[a-f0-9]{64}$/;

function isRecord(value) {
  return Boolean(value) && typeof value === 'object' && !Array.isArray(value);
}

function requireText(errors, value, label) {
  if (typeof value !== 'string' || value.trim() === '') errors.push(`${label} must be non-empty`);
}

function sha256(path) {
  return createHash('sha256').update(readFileSync(path)).digest('hex');
}

function verifyArtifactFile(errors, rootDir, entryId, relativePath, expectedHash, label) {
  if (typeof relativePath !== 'string' || !relativePath.trim()) {
    errors.push(`entry ${entryId} ${label} path must be non-empty`);
    return;
  }
  const path = resolve(rootDir, relativePath);
  if (!existsSync(path)) {
    errors.push(`entry ${entryId} ${label} is missing: ${relativePath}`);
    return;
  }
  if (expectedHash && sha256(path) !== expectedHash) {
    errors.push(`entry ${entryId} ${label} hash does not match ${relativePath}`);
  }
}

export function validateAnimalCapCatalog(catalog, options = {}) {
  const errors = [];
  const verifyFiles = options.verifyFiles ?? true;
  const rootDir = options.rootDir ?? process.cwd();

  if (!isRecord(catalog)) return ['catalog must be an object'];
  if (catalog.schemaVersion !== 1) errors.push('catalog schemaVersion must equal 1');
  if (!isRecord(catalog.boreProfiles)) errors.push('catalog boreProfiles must be an object');
  if (!Array.isArray(catalog.entries)) return [...errors, 'catalog entries must be an array'];

  const ids = new Set();
  for (const entry of catalog.entries) {
    if (!isRecord(entry)) {
      errors.push('catalog entry must be an object');
      continue;
    }
    requireText(errors, entry.id, 'entry id');
    const id = typeof entry.id === 'string' && entry.id ? entry.id : '<unknown>';
    if (ids.has(id)) errors.push(`duplicate entry id ${id}`);
    ids.add(id);
    requireText(errors, entry.displayName, `entry ${id} displayName`);
    requireText(errors, entry.species, `entry ${id} species`);
    if (!['candidate', 'published'].includes(entry.state)) {
      errors.push(`entry ${id} state must be candidate or published`);
    }
    if (!isRecord(entry.surfaces)) {
      errors.push(`entry ${id} surfaces must be an object`);
    }
    if (!isRecord(entry.source)) {
      errors.push(`entry ${id} source must be an object`);
      continue;
    }
    requireText(errors, entry.source.author, `entry ${id} source author`);
    requireText(errors, entry.source.pageUrl, `entry ${id} source pageUrl`);
    requireText(errors, entry.source.downloadUrl, `entry ${id} source downloadUrl`);
    requireText(errors, entry.source.license, `entry ${id} source license`);
    requireText(errors, entry.source.licenseUrl, `entry ${id} source licenseUrl`);

    if (entry.state !== 'published') {
      if (entry.surfaces?.engine || entry.surfaces?.landing) {
        errors.push(`candidate entry ${id} cannot be exposed on product surfaces`);
      }
      continue;
    }

    if (!isRecord(entry.recipe)) {
      errors.push(`entry ${id} published recipe must be an object`);
      continue;
    }
    if (!isRecord(catalog.boreProfiles) || !catalog.boreProfiles[entry.recipe.boreProfileId]) {
      errors.push(`entry ${id} references unknown bore profile ${entry.recipe.boreProfileId}`);
    }
    if (
      typeof entry.recipe.uniformScale !== 'number' ||
      entry.recipe.uniformScale <= 0 ||
      ['scaleX', 'scaleY', 'scaleZ', 'nonUniformScale'].some((key) => key in entry.recipe)
    ) {
      errors.push(`entry ${id} recipe must use uniformScale only`);
    }
    if (!['x', 'y', 'z'].includes(entry.recipe.boreAxis)) {
      errors.push(`entry ${id} boreAxis must be x, y, or z`);
    }
    if (typeof entry.recipe.boreMouthSourceCoordinate !== 'number') {
      errors.push(`entry ${id} boreMouthSourceCoordinate must be numeric`);
    }
    if (typeof entry.recipe.boreAxisHeightMm !== 'number') {
      errors.push(`entry ${id} boreAxisHeightMm must be numeric`);
    }

    if (!isRecord(entry.artifact)) {
      errors.push(`entry ${id} published artifact must be an object`);
      continue;
    }
    if (entry.artifact.verificationStatus !== 'passed') {
      errors.push(`entry ${id} published artifact must have passed verification`);
    }
    if (entry.artifact.verifiedNonManifoldEdgeCount !== 0) {
      errors.push(`entry ${id} published artifact must have zero non-manifold edges`);
    }
    for (const key of ['modelId', 'threadId', 'messageId']) {
      requireText(errors, entry.artifact[key], `entry ${id} artifact ${key}`);
    }
    for (const [value, label] of [
      [entry.source.sourceSha256, 'sourceSha256'],
      [entry.source.ingestedStlSha256, 'ingestedStlSha256'],
      [entry.artifact.sourceSha256, 'artifact sourceSha256'],
      [entry.artifact.stlSha256, 'artifact stlSha256'],
    ]) {
      if (typeof value !== 'string' || !SHA256.test(value)) {
        errors.push(`entry ${id} ${label} must be lowercase SHA-256`);
      }
    }

    if (verifyFiles) {
      verifyArtifactFile(
        errors,
        rootDir,
        id,
        entry.source.sourceMeshPath,
        entry.source.sourceSha256,
        'source mesh',
      );
      verifyArtifactFile(
        errors,
        rootDir,
        id,
        entry.source.ingestedStlPath,
        entry.source.ingestedStlSha256,
        'ingested STL',
      );
      verifyArtifactFile(
        errors,
        rootDir,
        id,
        entry.artifact.sourcePath,
        entry.artifact.sourceSha256,
        'published source',
      );
      verifyArtifactFile(
        errors,
        rootDir,
        id,
        entry.artifact.stlPath,
        entry.artifact.stlSha256,
        'published STL',
      );
      verifyArtifactFile(
        errors,
        rootDir,
        id,
        entry.artifact.previewPath,
        null,
        'published preview',
      );
    }
  }

  return errors;
}

function identifier(id) {
  return id.replace(/[^a-zA-Z0-9]+(.)/g, (_, next) => next.toUpperCase()).replace(/^[^a-zA-Z_]/, '_');
}

export function generateLandingModule(catalog) {
  const entries = catalog.entries.filter(
    (entry) => entry.state === 'published' && entry.surfaces?.landing === true,
  );
  const imports = [];
  const rows = [];

  for (const entry of entries) {
    const name = identifier(entry.id);
    const prefix = '../../../../catalogs/animal-caps/';
    imports.push(`import ${name}StlUrl from '${prefix}${entry.artifact.stlPath}?url';`);
    imports.push(`import ${name}SourceUrl from '${prefix}${entry.artifact.sourcePath}?url';`);
    imports.push(`import ${name}PreviewUrl from '${prefix}${entry.artifact.previewPath}?url';`);
    rows.push(`  {
    id: ${JSON.stringify(entry.id)},
    displayName: ${JSON.stringify(entry.displayName)},
    species: ${JSON.stringify(entry.species)},
    boreProfileId: ${JSON.stringify(entry.recipe.boreProfileId)},
    boreAxis: ${JSON.stringify(entry.recipe.boreAxis)},
    boreAxisHeightMm: ${JSON.stringify(entry.recipe.boreAxisHeightMm)},
    uniformScale: ${JSON.stringify(entry.recipe.uniformScale)},
    license: ${JSON.stringify(entry.source.license)},
    sourceAuthor: ${JSON.stringify(entry.source.author)},
    sourcePageUrl: ${JSON.stringify(entry.source.pageUrl)},
    modelId: ${JSON.stringify(entry.artifact.modelId)},
    verificationStatus: ${JSON.stringify(entry.artifact.verificationStatus)},
    verifiedTriangleCount: ${JSON.stringify(entry.artifact.verifiedTriangleCount)},
    stlUrl: ${name}StlUrl,
    stlDownloadName: ${JSON.stringify(`${entry.id.replace(/^quaternius-/, '').replace(/-presta$/, '')}-presta-valve-cap.stl`)},
    sourceUrl: ${name}SourceUrl,
    sourceDownloadName: ${JSON.stringify(`${entry.id.replace(/^quaternius-/, '').replace(/-presta$/, '')}-presta-valve-cap.ecky`)},
    previewUrl: ${name}PreviewUrl,
  }`);
  }

  return `// GENERATED FILE. Run: node scripts/sync_animal_cap_catalog.mjs
${imports.join('\n')}

export type AnimalCapShowcaseEntry = {
  id: string;
  displayName: string;
  species: string;
  boreProfileId: string;
  boreAxis: 'x' | 'y' | 'z';
  boreAxisHeightMm: number;
  uniformScale: number;
  license: string;
  sourceAuthor: string;
  sourcePageUrl: string;
  modelId: string;
  verificationStatus: 'passed';
  verifiedTriangleCount: number;
  stlUrl: string;
  stlDownloadName: string;
  sourceUrl: string;
  sourceDownloadName: string;
  previewUrl: string;
};

export const animalCapShowcaseEntries: AnimalCapShowcaseEntry[] = [
${rows.join(',\n')}
];
`;
}

