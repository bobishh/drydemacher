import assert from 'node:assert/strict';
import test from 'node:test';

import { get, writable } from 'svelte/store';

import { createOnboardingStore, shouldAutoStartOnboarding } from './onboarding';

test('persisted completed or skipped onboarding never restarts after config reload', () => {
  assert.equal(shouldAutoStartOnboarding({
    configLoaded: true,
    isBooting: false,
    hasSeenOnboarding: true,
    isActive: false,
    isSuppressed: false,
  }), false);
});

test('fresh config starts onboarding only after canonical config has loaded', () => {
  const fresh = {
    isBooting: false,
    hasSeenOnboarding: false,
    isActive: false,
    isSuppressed: false,
  };
  assert.equal(shouldAutoStartOnboarding({ ...fresh, configLoaded: false }), false);
  assert.equal(shouldAutoStartOnboarding({ ...fresh, configLoaded: true }), true);
});

test('onboarding store advances through floating-window targets and finishes cleanly', async () => {
  const configStore = writable({ hasSeenOnboarding: false });
  let saveCount = 0;
  const onboarding = createOnboardingStore({
    configStore,
    saveConfig: async () => {
      saveCount += 1;
    },
  });

  onboarding.start();
  assert.deepEqual(get(onboarding), {
    isActive: true,
    currentStepIndex: 0,
    currentStepId: 'intro',
    highlightTarget: null,
    windowIdToOpen: null,
    text: "Welcome to Ecky CAD! I'm Ecky, your AI design assistant. Let me give you a quick tour of how we can build things together.",
  });

  await onboarding.next();
  assert.equal(get(onboarding).currentStepId, 'dialogue');
  assert.equal(get(onboarding).highlightTarget, 'dialogue');
  assert.equal(get(onboarding).windowIdToOpen, 'dialogue');

  await onboarding.next();
  assert.equal(get(onboarding).currentStepId, 'viewport');
  assert.equal(get(onboarding).highlightTarget, 'viewport');
  assert.equal(get(onboarding).windowIdToOpen, null);

  await onboarding.next();
  assert.equal(get(onboarding).currentStepId, 'params');
  assert.equal(get(onboarding).windowIdToOpen, 'params');

  await onboarding.next();
  assert.equal(get(onboarding).currentStepId, 'projects');
  assert.equal(get(onboarding).windowIdToOpen, 'projects');

  await onboarding.next();
  assert.equal(get(onboarding).currentStepId, 'finish');
  assert.equal(get(onboarding).highlightTarget, null);

  await onboarding.next();
  assert.equal(get(onboarding).isActive, false);
  assert.equal(get(configStore).hasSeenOnboarding, true);
  assert.equal(saveCount, 1);
});

test('onboarding skip marks config complete immediately', async () => {
  const configStore = writable({ hasSeenOnboarding: false });
  let saveCount = 0;
  const onboarding = createOnboardingStore({
    configStore,
    saveConfig: async () => {
      saveCount += 1;
    },
  });

  onboarding.start();
  await onboarding.skip();

  assert.equal(get(onboarding).isActive, false);
  assert.equal(get(configStore).hasSeenOnboarding, true);
  assert.equal(saveCount, 1);
});
