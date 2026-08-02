<script lang="ts">
  import * as THREE from 'three';
  import { STLLoader } from 'three/examples/jsm/loaders/STLLoader.js';

  // Renders real Ecky output: the actual STL meshes produced by the OCCT kernel
  // from a committed .ecky model. Not a mockup — these are the exported parts.
  type Part = { url: string; color: string; opacity?: number };

  let {
    parts,
    size = 360,
    interactive = true,
    initialYaw = 0.12,
    initialPitch = -0.08,
    label = 'A real model rendered by Ecky — drag to rotate',
  }: {
    parts: Part[];
    size?: number;
    interactive?: boolean;
    initialYaw?: number;
    initialPitch?: number;
    label?: string;
  } = $props();

  let canvas = $state<HTMLCanvasElement | null>(null);
  let status = $state<'loading' | 'ready' | 'error'>('loading');
  let errorMessage = $state('');
  let failedAsset = $state('');
  let retryKey = $state(0);

  const rt = { userYaw: 0.12, userPitch: -0.08 };
  let renderScene: (() => void) | null = null;
  let dragPointerId: number | null = null;
  let dragLastX = 0;
  let dragLastY = 0;

  function clamp(v: number, min: number, max: number) {
    return Math.max(min, Math.min(max, v));
  }

  $effect(() => {
    if (!canvas) return;
    retryKey;
    status = 'loading';
    errorMessage = '';
    failedAsset = '';
    rt.userYaw = initialYaw;
    rt.userPitch = initialPitch;

    let active = true;
    renderScene = null;
    let r: THREE.WebGLRenderer;
    try {
      r = new THREE.WebGLRenderer({ canvas, alpha: true, antialias: true });
      r.setPixelRatio(Math.min(window.devicePixelRatio || 1, 2));
      r.outputColorSpace = THREE.SRGBColorSpace;
    } catch (error) {
      status = 'error';
      errorMessage = error instanceof Error ? error.message : String(error);
      return;
    }

    // CSS caps the viewer at `size`, but the workbench can be narrower on a
    // phone. Keep the WebGL drawing buffer aligned with the visible square.
    const resize = () => {
      const side = Math.max(1, Math.round(canvas.getBoundingClientRect().width));
      r.setSize(side, side, false);
      renderScene?.();
    };
    const resizeObserver = new ResizeObserver(resize);
    resizeObserver.observe(canvas);
    resize();

    const scene = new THREE.Scene();
    const group = new THREE.Group();
    scene.add(group);
    const camera = new THREE.PerspectiveCamera(30, 1, 0.1, 2000);
    // Visible height at z=180 ≈ 2*180*tan(15°) ≈ 96 units; fits a 64-unit model.
    camera.position.set(0, 0, 180);
    camera.lookAt(0, 0, 0);

    scene.add(new THREE.AmbientLight(0x9fb7a6, 1.1));
    const key = new THREE.DirectionalLight(0xdce8db, 2.2);
    key.position.set(-60, 80, 100);
    scene.add(key);
    const rim = new THREE.DirectionalLight(0x6fae8a, 1.0);
    rim.position.set(70, -40, 60);
    scene.add(rim);

    const render = () => {
      group.rotation.y = rt.userYaw;
      group.rotation.x = rt.userPitch;
      r.render(scene, camera);
    };
    renderScene = render;
    render();

    // Load every part into one shared coordinate frame (they align as the
    // kernel emitted them), then fit the whole group into view.
    const loader = new STLLoader();
    const meshes: THREE.Mesh[] = [];
    let pending = parts.length;
    let failures = 0;
    const finalize = () => {
      if (!active || failures > 0 || meshes.length === 0) return;
      const box = new THREE.Box3();
      for (const m of meshes) box.expandByObject(m);
      // The STLs share one coordinate frame (same as the kernel emitted them),
      // so do NOT per-part center — that would collapse the assembly. Offset
      // the whole group by the combined centroid and scale to fit.
      const center = box.getCenter(new THREE.Vector3());
      const dims = box.getSize(new THREE.Vector3());
      const maxDim = Math.max(dims.x, dims.y, dims.z) || 1;
      // Fit longest dimension to ~64 units of camera space (~67% of view).
      const scale = 64 / maxDim;
      group.scale.setScalar(scale);
      // Move the group's centroid to the origin (apply in local units, pre-scale).
      group.position.copy(center).multiplyScalar(-scale);
      status = 'ready';
      render();
    };

    if (parts.length === 0) {
      status = 'error';
      errorMessage = 'No STL parts were published for this preset.';
    }

    for (const part of parts) {
      loader.load(
        part.url,
        (geometry) => {
          if (!active) {
            geometry.dispose();
            return;
          }
          geometry.computeVertexNormals();
          const material = new THREE.MeshStandardMaterial({
            color: new THREE.Color(part.color),
            metalness: 0.08,
            roughness: 0.55,
            transparent: (part.opacity ?? 1) < 1,
            opacity: part.opacity ?? 1,
            flatShading: false,
          });
          const mesh = new THREE.Mesh(geometry, material);
          group.add(mesh);
          meshes.push(mesh);
          pending -= 1;
          if (pending === 0) finalize();
        },
        undefined,
        (err) => {
          if (!active) return;
          failures += 1;
          pending -= 1;
          failedAsset = decodeURIComponent(part.url.split('/').pop()?.split('?')[0] ?? part.url);
          errorMessage = err instanceof Error ? err.message : String(err || 'Unknown loader failure');
          status = 'error';
        },
      );
    }

    return () => {
      active = false;
      resizeObserver.disconnect();
      if (renderScene === render) renderScene = null;
      for (const m of meshes) {
        m.geometry.dispose();
        (m.material as THREE.Material).dispose();
      }
      r.dispose();
    };
  });

  function retry() {
    retryKey += 1;
  }

  function startDrag(e: PointerEvent) {
    if (!interactive) return;
    dragPointerId = e.pointerId;
    dragLastX = e.clientX;
    dragLastY = e.clientY;
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
  }
  function moveDrag(e: PointerEvent) {
    if (!interactive || dragPointerId !== e.pointerId) return;
    rt.userYaw = clamp(rt.userYaw + (e.clientX - dragLastX) * 0.01, -2.4, 2.4);
    rt.userPitch = clamp(rt.userPitch + (e.clientY - dragLastY) * 0.008, -2.6, 2.6);
    dragLastX = e.clientX;
    dragLastY = e.clientY;
    renderScene?.();
  }
  function endDrag(e: PointerEvent) {
    if (dragPointerId !== e.pointerId) return;
    dragPointerId = null;
    (e.currentTarget as HTMLElement).releasePointerCapture(e.pointerId);
  }

</script>

<div
  class="viewer"
  style={`--viewer-size:${size}px;`}
  class:interactive
  role="img"
  aria-label={label}
>
  <canvas
    bind:this={canvas}
    width={size}
    height={size}
    onpointerdown={startDrag}
    onpointermove={moveDrag}
    onpointerup={endDrag}
    onpointercancel={endDrag}
  ></canvas>
  {#if status === 'loading'}
    <div class="viewer-load" role="status">rendering…</div>
  {:else if status === 'error'}
    <div class="viewer-error" role="alert">
      <strong>STL LOAD FAILED</strong>
      <span>{failedAsset || 'unpublished STL'}</span>
      <small>{errorMessage}</small>
      <button type="button" onclick={retry}>RETRY STL</button>
    </div>
  {/if}
</div>

<style>
  .viewer {
    position: relative;
    display: block;
    width: min(100%, var(--viewer-size));
    aspect-ratio: 1;
    overflow: hidden;
  }
  .viewer canvas {
    display: block;
    width: 100%;
    height: 100%;
    touch-action: none;
  }
  .viewer.interactive canvas {
    cursor: grab;
  }
  .viewer.interactive canvas:active {
    cursor: grabbing;
  }
  .viewer-load {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    font: 500 0.8rem var(--font-mono, monospace);
    color: var(--text-dim, #7a8a7a);
    pointer-events: none;
  }

  .viewer-error {
    position: absolute;
    inset: 12%;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 7px;
    padding: 12px;
    border: 1px solid var(--red, #ff6b6b);
    background: color-mix(in srgb, var(--bg-100, #16213e) 94%, transparent);
    color: var(--text, #e0e0e0);
    text-align: center;
    overflow: hidden;
  }

  .viewer-error strong {
    color: var(--red, #ff6b6b);
    font: 700 0.74rem var(--font-mono, monospace);
    letter-spacing: 0.08em;
  }

  .viewer-error span,
  .viewer-error small {
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    font-family: var(--font-mono, monospace);
  }

  .viewer-error span {
    font-size: 0.72rem;
  }

  .viewer-error small {
    color: var(--text-dim, #888);
    font-size: 0.68rem;
    white-space: nowrap;
  }

  .viewer-error button {
    padding: 5px 8px;
    border: 1px solid var(--red, #ff6b6b);
    background: var(--bg-200, #1a1a2e);
    color: var(--red, #ff6b6b);
    font: 700 0.58rem var(--font-mono, monospace);
    cursor: pointer;
  }
</style>
