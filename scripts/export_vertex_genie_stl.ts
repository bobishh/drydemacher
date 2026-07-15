// Export the VertexGenie (Ecky mascot, seed 1) to a binary STL file.
//
// This produces a realistic, organic, displaced triangle mesh — the exact same
// geometry rendered in three.js on the landing page — for use as a poly BRep
// bridge test fixture. The triangulation is lifted verbatim from
// `sites/landing/src/EckyMascot.svelte::buildScene` so the STL IS the mascot.
//
// AFTER the face carving test, the mascot STL is also plain printable.
//
// Usage:
//   npx tsx scripts/export_vertex_genie_stl.ts <output.stl>

import * as THREE from 'three';
import { STLExporter } from 'three/examples/jsm/exporters/STLExporter.js';
import { writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import {
  DEFAULT_GENIE_TRAITS,
  resolveModeTraits,
  seededSigned,
  seededUnit,
} from '../src/lib/genie/traits';
import { buildStoneGeometry, type StonePoint3 } from '../src/lib/genie/stoneGeometry';

const outPath = process.argv[2];
if (!outPath) {
  console.error('Usage: npx tsx scripts/export_vertex_genie_stl.ts <output.stl>');
  process.exit(1);
}

// The canonical Ecky genome: default traits, idle mode. Seed 1 is Ecky.
const profile = resolveModeTraits(DEFAULT_GENIE_TRAITS, 'idle');

function clamp(value: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, value));
}

const stone = buildStoneGeometry(profile);
const frontCount = stone.front.length;
const toVector = (point: StonePoint3) => new THREE.Vector3(point.x, point.y, point.z);
const front = stone.front.map(toVector);
const rim = stone.rim.map(toVector);
const back = stone.back.map(toVector);
const backCenter = new THREE.Vector3(
  back.reduce((sum, point) => sum + point.x, 0) / frontCount + seededSigned(profile.seed, 1486) * 0.04,
  back.reduce((sum, point) => sum + point.y, 0) / frontCount,
  -0.92,
);
const sideMid = front.map((point, index) => {
  const next = (index + 1) % frontCount;
  const prev = (index + frontCount - 1) % frontCount;
  const clump =
    0.02 +
    seededUnit(profile.seed, 1470 + index) * 0.13 +
    (index % 4 === 0 ? 0.08 : index % 4 === 2 ? 0.05 : 0);
  const basePoint = point.clone().lerp(rim[index], 0.5 + seededUnit(profile.seed, 1478 + index) * 0.16);
  const ridgeSource = rim[index].clone().lerp(rim[next], 0.22 + seededUnit(profile.seed, 1480 + index) * 0.22);
  const shoulder = rim[prev].clone().lerp(rim[index], 0.62);
  return basePoint
    .lerp(ridgeSource, clump)
    .lerp(shoulder, seededUnit(profile.seed, 1484 + index) * 0.08)
    .setZ(basePoint.z + clump * (0.46 + seededUnit(profile.seed, 1488 + index) * 0.28));
});
const rearMid = rim.map((point, index) => {
  const prev = (index + frontCount - 1) % frontCount;
  const next = (index + 1) % frontCount;
  const clump =
    0.03 +
    seededUnit(profile.seed, 1490 + index) * 0.16 +
    (index % 5 === 1 ? 0.1 : index % 5 === 3 ? 0.06 : 0);
  const basePoint = point.clone().lerp(back[index], 0.44 + seededUnit(profile.seed, 1498 + index) * 0.16);
  const ridgeSource = point
    .clone()
    .lerp(rim[next], 0.18 + seededUnit(profile.seed, 1502 + index) * 0.18)
    .lerp(back[prev], 0.18 + seededUnit(profile.seed, 1506 + index) * 0.16);
  return basePoint
    .lerp(ridgeSource, clump)
    .setZ(basePoint.z - clump * (0.28 + seededUnit(profile.seed, 1510 + index) * 0.24));
});
const deformRing = (ring: THREE.Vector3[], sourceOffset: number, depthBase: number, depthRange: number, zBias: number) =>
  ring.map((point, index) => {
    const outward = point.clone().sub(backCenter).setZ(0);
    if (outward.lengthSq() < 0.01) return point.clone();
    outward.normalize();
    const tangent = new THREE.Vector3(-outward.y, outward.x, 0);
    const majorPeak = index % 5 === sourceOffset % 5 || seededUnit(profile.seed, sourceOffset + index) > 0.72;
    const minorPeak = index % 5 === (sourceOffset + 2) % 5 || seededUnit(profile.seed, sourceOffset + 50 + index) > 0.56;
    const peakDepth = majorPeak
      ? depthBase + seededUnit(profile.seed, sourceOffset + 100 + index) * depthRange
      : minorPeak
        ? depthBase * 0.48 + seededUnit(profile.seed, sourceOffset + 100 + index) * depthRange * 0.48
        : depthBase * 0.12 + seededUnit(profile.seed, sourceOffset + 100 + index) * depthRange * 0.16;
    return point
      .clone()
      .add(outward.multiplyScalar(peakDepth))
      .add(tangent.multiplyScalar(seededSigned(profile.seed, sourceOffset + 150 + index) * (majorPeak ? 0.1 : 0.04)))
      .setZ(point.z + zBias * (majorPeak ? 1 : 0.45) + seededSigned(profile.seed, sourceOffset + 200 + index) * Math.abs(zBias) * 0.28);
  });
const sideShell = deformRing(sideMid, 1516, 0.16, 0.22, 0.1);
const rimShell = deformRing(rim, 1540, 0.28, 0.34, 0.14);
const rearShell = deformRing(rearMid, 1564, 0.3, 0.36, -0.14);
const backShell = deformRing(back, 1588, 0.32, 0.42, -0.18);
const backCrown = back.map((point, index) =>
  point
    .clone()
    .lerp(backCenter, 0.42 + seededUnit(profile.seed, 1552 + index) * 0.14)
    .setZ(-0.72 - seededUnit(profile.seed, 1560 + index) * 0.18),
);
const crownShell = deformRing(backCrown, 1630, 0.26, 0.36, -0.22);
const center = toVector(stone.center);
const positions: number[] = [];
const pushTri = (a: THREE.Vector3, b: THREE.Vector3, c: THREE.Vector3) => {
  for (const point of [a, b, c]) {
    positions.push(point.x, point.y, point.z);
  }
};
const pushQuad = (a: THREE.Vector3, b: THREE.Vector3, c: THREE.Vector3, d: THREE.Vector3) => {
  pushTri(a, b, c);
  pushTri(a, c, d);
};
for (let index = 0; index < frontCount; index++) {
  const next = (index + 1) % frontCount;
  pushTri(center, front[index], front[next]);
  pushQuad(front[index], sideShell[index], sideShell[next], front[next]);
  pushQuad(sideShell[index], rimShell[index], rimShell[next], sideShell[next]);
  pushQuad(rimShell[index], rearShell[index], rearShell[next], rimShell[next]);
  pushQuad(rearShell[index], backShell[index], backShell[next], rearShell[next]);
  pushQuad(backShell[index], crownShell[index], crownShell[next], backShell[next]);
  pushTri(backCenter, crownShell[next], crownShell[index]);
}

const geometry = new THREE.BufferGeometry();
geometry.setAttribute('position', new THREE.Float32BufferAttribute(positions, 3));
geometry.computeVertexNormals();

const baseScaleX = 0.86;
const baseScaleY = 0.94;
const baseScaleZ = 0.86;
const mesh = new THREE.Mesh(geometry);
mesh.scale.set(baseScaleX, baseScaleY, baseScaleZ);
mesh.updateMatrixWorld(true);

const exporter = new STLExporter();
const parsed = exporter.parse(mesh, { binary: true }) as unknown as DataView;
const buffer = Buffer.from(parsed.buffer);

const triCount = buffer.readUInt32LE(80);
console.error(`VertexGenie (seed 1, idle): ${triCount} triangles, ${buffer.length} bytes (binary STL)`);
const resolved = resolve(process.cwd(), outPath);
writeFileSync(resolved, buffer);
console.error(`→ ${resolved}`);
