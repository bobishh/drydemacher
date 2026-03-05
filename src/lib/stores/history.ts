import { get } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import { history, activeThreadId, activeVersionId } from './domainState';
import { workingCopy } from './workingCopy';
import { handleParamChange, session } from './sessionFlow';

export async function loadVersion(msg) {
  if (!msg || !msg.output) return;
  console.log("[History] Loading version:", msg.id);
  activeVersionId.set(msg.id);
  workingCopy.loadVersion(msg.output, msg.id);
  session.setStatus(`Loaded Version: ${msg.output.title}`);
  await handleParamChange(msg.output.initial_params || {}, msg.output.macro_code);
}

export async function loadFromHistory(thread) {
  const targetThreadId = thread.id;
  activeThreadId.set(targetThreadId);
  
  const freshHistory = get(history);
  const freshThread = freshHistory.find(t => t.id === targetThreadId) || thread;
  const lastAssistantMsg = [...freshThread.messages].reverse().find(m => m.role === 'assistant' && m.output);
  
  if (lastAssistantMsg) {
    await loadVersion(lastAssistantMsg);
  } else {
    activeVersionId.set(null);
  }
}

export async function deleteThread(id) {
  try {
    await invoke('delete_thread', { id });
    if (get(activeThreadId) === id) {
      activeThreadId.set(null);
      activeVersionId.set(null);
      workingCopy.reset();
      session.setStlUrl(null);
    }
    const freshHistory = await invoke('get_history');
    history.set(freshHistory);
  } catch (e) {
    session.setError(`Delete Error: ${e}`);
  }
}

export async function deleteVersion(messageId) {
  try {
    await invoke('delete_version', { message_id: messageId });
    const currentThreadId = get(activeThreadId);
    if (!currentThreadId) return;

    // Refresh history
    const freshHistory = await invoke('get_history');
    history.set(freshHistory);

    // Update active version if we deleted the current one
    if (get(activeVersionId) === messageId) {
      const updatedThread = freshHistory.find(t => t.id === currentThreadId);
      const remainingVersions = updatedThread ? updatedThread.messages.filter(m => m.role === 'assistant' && m.output) : [];
      
      if (remainingVersions.length > 0) {
        // Load the last available version
        await loadVersion(remainingVersions[remainingVersions.length - 1]);
      } else {
        // No versions left, reset working copy
        activeVersionId.set(null);
        workingCopy.reset();
      }
    }
  } catch (e) {
    session.setError(`Failed to delete version: ${e}`);
  }
}

export function createNewThread() {
  activeThreadId.set(null);
  activeVersionId.set(null);
  workingCopy.reset();
  session.setStlUrl(null);
  session.setStatus('New design session started.');
}

export function forkDesign() {
  activeThreadId.set(null);
  activeVersionId.set(null);
  workingCopy.patch({
    versionName: 'Forked',
    sourceVersionId: null
  });
  session.setStatus('Design forked. Next generation will create a new thread.');
}

export async function refreshHistory() {
  try {
    const freshHistory = await invoke('get_history');
    history.set(freshHistory);
  } catch (e) {
    console.error("[History] Failed to refresh history:", e);
  }
}
