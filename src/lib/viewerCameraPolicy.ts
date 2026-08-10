import * as THREE from 'three';

export type ViewerClipPlanes = {
  near: number;
  far: number;
};

/** Keep depth precision tied to the displayed model, not an unrelated scene-size floor. */
export function resolveViewerClipPlanes(
  bounds: THREE.Box3,
  cameraPosition: THREE.Vector3,
): ViewerClipPlanes {
  const size = bounds.getSize(new THREE.Vector3());
  const maxDim = Math.max(size.x, size.y, size.z, Number.EPSILON);
  const sphere = bounds.getBoundingSphere(new THREE.Sphere());
  const distance = cameraPosition.distanceTo(sphere.center);
  const usableFrontDistance = Math.max(distance - sphere.radius, maxDim * 0.001);
  const backDistance = distance + sphere.radius;
  const minimumNear = maxDim * 0.000001;

  return {
    near: Math.max(
      minimumNear,
      Math.min(distance * 0.1, sphere.radius * 0.1, usableFrontDistance * 0.5),
    ),
    far: backDistance * 1.05,
  };
}
