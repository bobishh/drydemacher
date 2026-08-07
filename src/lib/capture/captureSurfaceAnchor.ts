import * as THREE from 'three';

export interface CaptureSurfaceAnchorValue {
  sourceMeshContentDigest: string;
  triangleIndex: number;
  barycentric: [number, number, number];
  sourcePosition: [number, number, number];
  sourceNormal: [number, number, number];
}

export function captureSurfaceAnchorFromIntersection(
  sourceMeshContentDigest: string,
  sourceGeometry: THREE.BufferGeometry,
  sourceMesh: THREE.Mesh,
  intersection: THREE.Intersection<THREE.Object3D>,
): CaptureSurfaceAnchorValue {
  if (!sourceMeshContentDigest.startsWith('sha256:')) {
    throw new Error('Capture source mesh content digest is invalid.');
  }
  if (intersection.faceIndex == null) {
    throw new Error('Capture intersection has no source triangle index.');
  }
  const triangleIndex = intersection.faceIndex;
  const positions = sourceGeometry.getAttribute('position');
  if (!(positions instanceof THREE.BufferAttribute) && !(positions instanceof THREE.InterleavedBufferAttribute)) {
    throw new Error('Capture source geometry has no position attribute.');
  }
  const offset = triangleIndex * 3;
  const index = sourceGeometry.getIndex();
  const vertexIndices = index
    ? [index.getX(offset), index.getX(offset + 1), index.getX(offset + 2)]
    : [offset, offset + 1, offset + 2];
  if (vertexIndices.some(vertexIndex => !Number.isInteger(vertexIndex) || vertexIndex < 0 || vertexIndex >= positions.count)) {
    throw new Error('Capture source triangle index is out of bounds.');
  }

  const a = new THREE.Vector3().fromBufferAttribute(positions, vertexIndices[0]);
  const b = new THREE.Vector3().fromBufferAttribute(positions, vertexIndices[1]);
  const c = new THREE.Vector3().fromBufferAttribute(positions, vertexIndices[2]);
  const normal = new THREE.Vector3()
    .crossVectors(new THREE.Vector3().subVectors(b, a), new THREE.Vector3().subVectors(c, a));
  if (!Number.isFinite(normal.lengthSq()) || normal.lengthSq() <= 1e-24) {
    throw new Error('Capture source triangle is degenerate.');
  }
  normal.normalize();

  sourceMesh.updateMatrixWorld(true);
  const localPoint = sourceMesh.worldToLocal(intersection.point.clone());
  const barycentricVector = THREE.Triangle.getBarycoord(
    localPoint,
    a,
    b,
    c,
    new THREE.Vector3(),
  );
  if (!barycentricVector) {
    throw new Error('Capture source triangle is degenerate.');
  }
  const barycentric: [number, number, number] = [
    barycentricVector.x,
    barycentricVector.y,
    barycentricVector.z,
  ];
  if (
    barycentric.some(value => !Number.isFinite(value) || value < -1e-7 || value > 1 + 1e-7)
    || Math.abs(barycentric[0] + barycentric[1] + barycentric[2] - 1) > 1e-7
  ) {
    throw new Error('Capture hit lies outside selected source triangle.');
  }
  const sourcePosition = new THREE.Vector3()
    .addScaledVector(a, barycentric[0])
    .addScaledVector(b, barycentric[1])
    .addScaledVector(c, barycentric[2]);

  return {
    sourceMeshContentDigest,
    triangleIndex,
    barycentric,
    sourcePosition: [sourcePosition.x, sourcePosition.y, sourcePosition.z],
    sourceNormal: [normal.x, normal.y, normal.z],
  };
}
