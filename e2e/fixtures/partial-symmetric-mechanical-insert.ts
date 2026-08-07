export type Vec3 = readonly [number, number, number];

/**
 * One observed quarter of a symmetric press-fit insert. Coordinates are source
 * mesh units; the guide must derive millimetres from the known 40 mm span.
 */
export const partialSymmetricMechanicalInsert = {
  artifactDigest: 'sha256:partial-insert-source-v1',
  contentDigest: 'sha256:partial-insert-content-v1',
  sourceUnits: 'source-unit',
  knownDimensionsMm: { calibrationSpan: 40, fullEnvelope: [80, 60, 18] as Vec3 },
  expectedBrepEnvelopeMm: [80, 60, 18] as Vec3,
  symmetry: { planes: ['x=0', 'y=0'], observedRegion: 'x>=0,y>=0' },
  landmarks: {
    calibrationStart: [0, 0, 0] as Vec3,
    calibrationEnd: [40, 0, 0] as Vec3,
    frameOrigin: [0, 0, 0] as Vec3,
    frameX: [40, 0, 0] as Vec3,
    frameY: [0, 30, 0] as Vec3,
    profile: [[0, 0, 9] as Vec3, [20, 0, 9] as Vec3, [20, 30, 9] as Vec3],
  },
  triangles: [
    [[0, 0, 0], [40, 0, 0], [0, 30, 0]],
    [[0, 0, 0], [0, 30, 0], [0, 0, 18]],
    [[40, 0, 0], [20, 30, 0], [20, 0, 18]],
    [[0, 30, 0], [20, 30, 0], [0, 0, 18]],
  ] as readonly (readonly Vec3[])[],
} as const;

export function partialSymmetricMechanicalInsertStl(): Buffer {
  const { triangles } = partialSymmetricMechanicalInsert;
  const stl = Buffer.alloc(84 + triangles.length * 50);
  stl.writeUInt32LE(triangles.length, 80);
  triangles.forEach((triangle, triangleIndex) => triangle.forEach((vertex, vertexIndex) => {
    vertex.forEach((value, axis) => stl.writeFloatLE(value, 84 + triangleIndex * 50 + 12 + vertexIndex * 12 + axis * 4));
  }));
  return stl;
}
