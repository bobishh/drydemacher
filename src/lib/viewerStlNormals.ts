import type * as THREE from 'three';
import { toCreasedNormals } from 'three/examples/jsm/utils/BufferGeometryUtils.js';

export const STL_NORMAL_CREASE_ANGLE = Math.PI / 6;

export function prepareStlDisplayGeometry(
  geometry: THREE.BufferGeometry,
): THREE.BufferGeometry {
  return toCreasedNormals(geometry, STL_NORMAL_CREASE_ANGLE);
}
