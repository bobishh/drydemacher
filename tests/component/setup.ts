// Vitest setup for the Svelte component-test harness.
// Guarantees @testing-library/svelte DOM is reset between cases regardless
// of globals detection.
import { afterEach } from 'vitest';
import { cleanup } from '@testing-library/svelte';

afterEach(() => {
  cleanup();
});
