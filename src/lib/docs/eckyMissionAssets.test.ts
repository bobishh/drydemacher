import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import test from 'node:test';


/**
 * repair-ecky-learning-campaign — source-bound render asset invariants.
 *
 * Campaign BRIEF images are not hand-drawn or AI-generated pictures. Each is the
 * actual tessellation of a committed Ecky source, produced by native Ecky
 * (direct-occt) -> STL -> the existing Three/WebGL browser renderer. These build
 * checks bind every published PNG to its canonical Ecky source so a source edit
 * without a re-render fails the build.
 *
 * They do NOT re-run CAD operations: geometry correctness stays owned by the
 * Ecky runtime/model tests. This only asserts provenance, presence, and binding.
 */

const root = process.cwd();

type RenderAsset = {
  name: string;
  ecky: string;
  png: string;
  /** Minimal structural sanity for the canonical source (NOT a CAD test). */
  sourceAsserts: Array<{ snippet: string; why: string }>;
};

const RENDER_ASSETS: RenderAsset[] = [
  {
    name: 'Corner Bracket',
    ecky: 'docs/books/ecky-ir/examples/corner-bracket.ecky',
    png: 'public/docs/assets/corner-bracket.png',
    sourceAsserts: [
      { snippet: '(union', why: 'bracket overlap+union relation' },
      { snippet: '(box 40 8 6)', why: 'foot' },
      { snippet: '(box 8 40 6)', why: 'flange' },
    ],
  },
  {
    name: 'Dovetail Fit',
    ecky: 'docs/books/ecky-ir/examples/dovetail-fit.ecky',
    png: 'public/docs/assets/dovetail-fit.png',
    sourceAsserts: [
      { snippet: '(polygon', why: 'dovetail profile extracted from film adapter' },
      { snippet: 'fit_clearance', why: 'named shared-clearance binding' },
      { snippet: '(extrude', why: 'rail/channel extrusion' },
      { snippet: '(difference', why: 'female channel cut' },
    ],
  },
];

function sha256(filePath: string): string {
  return createHash('sha256').update(fs.readFileSync(filePath)).digest('hex');
}

const PNG_SIGNATURE = Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]);

for (const asset of RENDER_ASSETS) {
  test(`canonical ${asset.name} Ecky source exists and carries its key forms`, () => {
    const sourcePath = path.join(root, asset.ecky);
    assert.ok(fs.existsSync(sourcePath), `canonical source missing: ${asset.ecky}`);
    const source = fs.readFileSync(sourcePath, 'utf8');
    assert.ok(source.includes('(model'), `${asset.ecky} must be a model`);
    for (const { snippet, why } of asset.sourceAsserts) {
      assert.ok(source.includes(snippet), `${asset.ecky} must contain ${why}: ${snippet}`);
    }
  });

  test(`published ${asset.name} PNG exists, is a real PNG, and has content`, () => {
    const pngPath = path.join(root, asset.png);
    assert.ok(fs.existsSync(pngPath), `published PNG missing: ${asset.png}`);
    const bytes = fs.readFileSync(pngPath);
    assert.deepEqual(
      bytes.subarray(0, 8),
      PNG_SIGNATURE,
      `${asset.png} must start with the PNG signature`,
    );
    assert.ok(bytes.length > 5000, `${asset.png} is suspiciously small (${bytes.length} bytes)`);
  });

  test(`published ${asset.name} PNG is bound to its canonical Ecky source`, () => {
    const manifestPath = `${path.join(root, asset.png)}.manifest.json`;
    assert.ok(fs.existsSync(manifestPath), `source-binding manifest missing: ${manifestPath}`);

    const manifest = JSON.parse(fs.readFileSync(manifestPath, 'utf8')) as {
      sourcePath: string;
      sourceSha256: string;
      eckyContentHash: string;
      stlBytes: number;
      renderer: string;
      backend: string;
      nonBackgroundCoverage?: number;
    };

    // Binding: manifest points at the committed source and its recorded sha256
    // matches the actual source bytes. Source drift without a re-render fails.
    assert.equal(manifest.sourcePath, asset.ecky);
    assert.equal(manifest.sourceSha256, sha256(path.join(root, asset.ecky)));

    // Provenance: native Ecky geometry through the Three/WebGL renderer — never OpenSCAD.
    assert.equal(manifest.backend, 'direct-occt');
    assert.ok(manifest.renderer.includes('three'), 'renderer must be the Three/WebGL pipeline');
    assert.ok(manifest.stlBytes > 0, 'manifest must record a non-empty STL');
    assert.ok(
      typeof manifest.eckyContentHash === 'string' && manifest.eckyContentHash.length > 0,
      'manifest must record the native ecky contentHash',
    );

    if (typeof manifest.nonBackgroundCoverage === 'number') {
      assert.ok(
        manifest.nonBackgroundCoverage > 0.01,
        `${asset.name} rendered mesh coverage too low (${(manifest.nonBackgroundCoverage * 100).toFixed(2)}%)`,
      );
    }
  });
}
