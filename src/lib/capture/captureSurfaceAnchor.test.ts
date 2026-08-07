import assert from 'node:assert/strict';
import test from 'node:test';
import * as THREE from 'three';
import { captureSurfaceAnchorFromIntersection } from './captureSurfaceAnchor';

function makeIntersection(geometry: THREE.BufferGeometry, localPoint: THREE.Vector3) {
  const mesh = new THREE.Mesh(geometry, new THREE.MeshBasicMaterial());
  mesh.rotation.x = -Math.PI / 2;
  mesh.position.set(10, 20, 30);
  mesh.updateMatrixWorld(true);
  return {
    mesh,
    intersection: {
      distance: 1,
      point: localPoint.clone().applyMatrix4(mesh.matrixWorld),
      object: mesh,
      faceIndex: 0,
    } as THREE.Intersection<THREE.Object3D>,
  };
}

test('non-indexed source hit becomes digest-bound source-coordinate triangle anchor', () => {
  const geometry = new THREE.BufferGeometry();
  geometry.setAttribute('position', new THREE.Float32BufferAttribute([
    0, 0, 0,
    2, 0, 0,
    0, 2, 0,
  ], 3));
  const { mesh, intersection } = makeIntersection(geometry, new THREE.Vector3(0.5, 1, 0));

  const anchor = captureSurfaceAnchorFromIntersection('sha256:mesh', geometry, mesh, intersection);

  assert.equal(anchor.triangleIndex, 0);
  assert.deepEqual(anchor.barycentric.map(value => Number(value.toFixed(9))), [0.25, 0.25, 0.5]);
  assert.deepEqual(anchor.sourcePosition.map(value => Number(value.toFixed(9))), [0.5, 1, 0]);
  assert.deepEqual(anchor.sourceNormal.map(value => Number(value.toFixed(9))), [0, 0, 1]);
});

test('indexed source hit uses source indices and ignores display transform', () => {
  const geometry = new THREE.BufferGeometry();
  geometry.setAttribute('position', new THREE.Float32BufferAttribute([
    0, 0, 0,
    2, 0, 0,
    0, 2, 0,
    10, 10, 10,
  ], 3));
  geometry.setIndex([0, 1, 2]);
  const { mesh, intersection } = makeIntersection(geometry, new THREE.Vector3(1, 0.5, 0));

  const anchor = captureSurfaceAnchorFromIntersection('sha256:indexed', geometry, mesh, intersection);

  assert.deepEqual(anchor.barycentric.map(value => Number(value.toFixed(9))), [0.25, 0.5, 0.25]);
  assert.deepEqual(anchor.sourcePosition.map(value => Number(value.toFixed(9))), [1, 0.5, 0]);
});

test('missing face index and degenerate source triangles fail exactly', () => {
  const geometry = new THREE.BufferGeometry();
  geometry.setAttribute('position', new THREE.Float32BufferAttribute([
    0, 0, 0,
    1, 0, 0,
    2, 0, 0,
  ], 3));
  const { mesh, intersection } = makeIntersection(geometry, new THREE.Vector3(0.5, 0, 0));
  assert.throws(
    () => captureSurfaceAnchorFromIntersection('sha256:mesh', geometry, mesh, intersection),
    /Capture source triangle is degenerate/,
  );
  intersection.faceIndex = null;
  assert.throws(
    () => captureSurfaceAnchorFromIntersection('sha256:mesh', geometry, mesh, intersection),
    /Capture intersection has no source triangle index/,
  );
});
