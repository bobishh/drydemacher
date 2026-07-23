import test from 'node:test';
import assert from 'node:assert/strict';

import * as THREE from 'three';

import { prepareStlDisplayGeometry } from './viewerStlNormals';

function twoFacesAtAngle(angle: number): THREE.BufferGeometry {
  const sin = Math.sin(angle);
  const cos = Math.cos(angle);
  const geometry = new THREE.BufferGeometry();
  geometry.setAttribute(
    'position',
    new THREE.Float32BufferAttribute(
      [
        0, 0, 0,
        1, 0, 0,
        0, 1, 0,
        0, 0, 0,
        1, 0, 0,
        0, cos, sin,
      ],
      3,
    ),
  );
  return geometry;
}

function normalsAtOrigin(geometry: THREE.BufferGeometry): THREE.Vector3[] {
  const positions = geometry.getAttribute('position');
  const normals = geometry.getAttribute('normal');
  const result: THREE.Vector3[] = [];
  for (let index = 0; index < positions.count; index += 1) {
    if (
      Math.abs(positions.getX(index)) < 1e-6 &&
      Math.abs(positions.getY(index)) < 1e-6 &&
      Math.abs(positions.getZ(index)) < 1e-6
    ) {
      result.push(
        new THREE.Vector3(
          normals.getX(index),
          normals.getY(index),
          normals.getZ(index),
        ).normalize(),
      );
    }
  }
  return result;
}

test('STL viewer keeps a 45 degree chamfer boundary hard', () => {
  const display = prepareStlDisplayGeometry(twoFacesAtAngle(Math.PI / 4));
  const normals = normalsAtOrigin(display);

  assert.equal(normals.length, 2);
  assert.ok(normals[0].dot(normals[1]) < 0.8);
});

test('STL viewer still smooths adjacent 14 degree curve facets', () => {
  const display = prepareStlDisplayGeometry(twoFacesAtAngle(0.25));
  const normals = normalsAtOrigin(display);

  assert.equal(normals.length, 2);
  assert.ok(normals[0].dot(normals[1]) > 0.999);
});
