import { defineConfig } from 'vitest/config';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { svelteTesting } from '@testing-library/svelte/vite';

// Dedicated vitest config for the Svelte component-test harness (T1).
// Kept separate from vite.config.ts so the app build/plugin config is
// untouched; vitest prefers this file when both exist.
//
// Component tests live under tests/component/** so they are NOT picked up by
// the existing `tsx --test src/lib/**/*.test.ts` unit runner, and vitest's
// include is scoped here so it never grabs the unit or e2e suites.
export default defineConfig({
  // svelte() compiles .svelte components; svelteTesting() adds the `browser`
  // resolve condition so Svelte 5's client mount() runtime (not the SSR one)
  // is used under jsdom.
  plugins: [svelte(), svelteTesting()],
  test: {
    environment: 'jsdom',
    globals: true,
    include: ['tests/component/**/*.test.ts'],
    setupFiles: ['./tests/component/setup.ts'],
  },
});
