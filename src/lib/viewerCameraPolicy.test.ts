import assert from 'node:assert/strict';
import test from 'node:test';
import * as THREE from 'three';
import { resolveViewerClipPlanes } from './viewerCameraPolicy';

test('resolveViewerClipPlanes keeps clip range proportional to model bounds', () => {
  const small = resolveViewerClipPlanes(
    new THREE.Box3(new THREE.Vector3(-0.5, -0.5, -0.5), new THREE.Vector3(0.5, 0.5, 0.5)),
    new THREE.Vector3(2, 2, 2),
  );
  const large = resolveViewerClipPlanes(
    new THREE.Box3(new THREE.Vector3(-500, -500, -500), new THREE.Vector3(500, 500, 500)),
    new THREE.Vector3(2000, 2000, 2000),
  );

  assert.ok(small.near > 0);
  assert.ok(small.far > small.near);
  assert.ok(large.near > small.near * 500);
  assert.ok(large.far < 10_000);
  assert.ok(large.far / large.near < 100);
});

test('resolveViewerClipPlanes leaves margin before model and after model', () => {
  const bounds = new THREE.Box3(new THREE.Vector3(-2, -1, -3), new THREE.Vector3(2, 1, 3));
  const planes = resolveViewerClipPlanes(bounds, new THREE.Vector3(8, 6, 10));
  const sphere = bounds.getBoundingSphere(new THREE.Sphere());
  const distance = new THREE.Vector3(8, 6, 10).distanceTo(sphere.center);

  assert.ok(planes.near < distance - sphere.radius);
  assert.ok(planes.far > distance + sphere.radius);
});

test('resolveViewerClipPlanes keeps a close or inside camera in front of the bounds', () => {
  const bounds = new THREE.Box3(new THREE.Vector3(-10, -2, -1), new THREE.Vector3(10, 2, 1));
  const cameraPosition = new THREE.Vector3(9.5, 0, 0);
  const sphere = bounds.getBoundingSphere(new THREE.Sphere());
  const distance = cameraPosition.distanceTo(sphere.center);
  const usableFrontDistance = Math.max(distance - sphere.radius, bounds.getSize(new THREE.Vector3()).x * 0.001);
  const backDistance = distance + sphere.radius;
  const planes = resolveViewerClipPlanes(bounds, cameraPosition);

  assert.ok(planes.near < usableFrontDistance);
  assert.ok(planes.far > backDistance);
});
