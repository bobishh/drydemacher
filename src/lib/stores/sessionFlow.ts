import { writable, get } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import { convertFileSrc } from '@tauri-apps/api/core';
import { workingCopy } from './workingCopy';
import { history, activeThreadId, activeVersionId } from './domainState';
import { refreshHistory } from './history';
import { showCodeModal } from './viewState';

/** 
 * @typedef {import('../types/phase').Phase} Phase 
 * 
 * @typedef {Object} RequestState
 * @property {string} id
 * @property {string} originalPrompt
 * @property {string} currentPrompt
 * @property {number} attempt
 * @property {number} maxAttempts
 * @property {boolean} questionMode
 * @property {string|null} screenshot
 * @property {Array} attachments
 */

function createSessionStore() {
  const { subscribe, set, update } = writable({
    phase: 'booting',
    status: 'System ready.',
    error: null,
    stlUrl: null,
    /** @type {RequestState|null} */
    request: null
  });

  return {
    subscribe,
    setPhase: (p) => update(s => ({ ...s, phase: p })),
    setStatus: (msg) => update(s => ({ ...s, status: msg })),
    setError: (err) => update(s => ({ ...s, error: err })),
    setStlUrl: (url) => update(s => ({ ...s, stlUrl: url })),
    startRequest: (req) => update(s => ({ ...s, request: req, error: null })),
    updateRequest: (patch) => update(s => ({ ...s, request: s.request ? { ...s.request, ...patch } : null })),
    finishRequest: () => update(s => ({ ...s, request: null }))
  };
}

export const session = createSessionStore();

// Accessors for convenience
export const phase = { subscribe: (fn) => session.subscribe(s => fn(s.phase)), set: session.setPhase };
export const status = { subscribe: (fn) => session.subscribe(s => fn(s.status)), set: session.setStatus };
export const error = { subscribe: (fn) => session.subscribe(s => fn(s.error)), set: session.setError };
export const stlUrl = { subscribe: (fn) => session.subscribe(s => fn(s.stlUrl)), set: session.setStlUrl };

// Global app state reference for callbacks (microwave, etc)
let appState = null;
export function initSessionFlow(state) {
  appState = state;
}

// Hard lock to prevent multiple simultaneous requests
let generationInFlight = false;

const REPAIR_PHRASES = [
  "FreeCAD blinked first. Asking the LLM for a cleaner retry.",
  "Repair cycle engaged. Convincing the macro to respect causality.",
  "Patching the geometry after a Boolean tantrum.",
  "Render failed. Rewriting the macro before the solver notices.",
  "Running emergency emotional support for a wounded BRep.",
  "The mesh has unionized. Negotiating a repair attempt.",
  "Reconstructing dignity after a FreeCAD traceback.",
  "The model broke character. Sending it back with notes.",
  "Second pass active: less chaos, more solids.",
  "Repairing the macro with the confidence of a forged permit."
];

function pickRetryMessage(nextAttempt, maxAttempts) {
  const phrase = REPAIR_PHRASES[Math.floor(Math.random() * REPAIR_PHRASES.length)];
  return `${phrase} Retry ${nextAttempt} of ${maxAttempts}.`;
}

export function startLightReasoning() {
  if (appState?.startLightReasoning) appState.startLightReasoning();
}

export function stopLightReasoning() {
  if (appState?.stopLightReasoning) appState.stopLightReasoning();
}

export function startCooking() {
  if (appState?.startCooking) appState.startCooking();
}

export function stopCooking(success) {
  if (appState?.stopCooking) appState.stopCooking(success);
}

export async function handleGenerate(initialPrompt, attachments = []) {
  const runId = Math.random().toString(36).substring(7);
  
  if (generationInFlight) {
    console.warn(`[SessionFlow:${runId}] IGNORING request: Another generation is already in flight.`);
    return;
  }

  const currentSession = get(session);
  if (currentSession.phase !== 'idle' && currentSession.phase !== 'error') {
    console.warn(`[SessionFlow:${runId}] IGNORING request: Current phase is ${currentSession.phase}`);
    return;
  }

  try {
    generationInFlight = true;
    console.log(`[SessionFlow:${runId}] Request started. Prompt: "${initialPrompt.substring(0, 30)}...", Attachments: ${attachments.length}`);

    session.startRequest({
      id: runId,
      originalPrompt: initialPrompt,
      currentPrompt: initialPrompt,
      attempt: 1,
      maxAttempts: 3,
      questionMode: false,
      screenshot: null,
      attachments
    });

    const { 
      isQuestionIntent,
      viewerComponent
    } = appState;

    function buildLightReasoningContext() {
      const context = [];
      const wc = get(workingCopy);
      if (wc.title) context.push(`Title: ${wc.title}`);
      if (wc.versionName) context.push(`Version: ${wc.versionName}`);
      if (wc.macroCode) context.push(`Current FreeCAD Macro:\n\`\`\`python\n${wc.macroCode}\n\`\`\``);
      if (wc.uiSpec) context.push(`Current UI Spec:\n\`\`\`json\n${JSON.stringify(wc.uiSpec, null, 2)}\n\`\`\``);
      if (wc.params && Object.keys(wc.params).length > 0) {
        context.push(`Current Parameters:\n\`\`\`json\n${JSON.stringify(wc.params, null, 2)}\n\`\`\``);
      }
      return context.join('\n\n');
    }

    function buildWorkingDesignSnapshot() {
      const wc = get(workingCopy);
      if (!wc.macroCode) return null;
      return {
        title: wc.title || 'Untitled Design',
        version_name: wc.versionName || 'Working Copy',
        response: '',
        interaction_mode: 'design',
        macro_code: wc.macroCode,
        ui_spec: wc.uiSpec || { fields: [] },
        initial_params: wc.params || {}
      };
    }

    session.setPhase('classifying');
    startLightReasoning();
    let isQuestion = isQuestionIntent(initialPrompt);
    let lightResponse = '';
    
    try {
      console.log(`[SessionFlow:${runId}] Classifying intent...`);
      const intent = await invoke('classify_intent', { 
        prompt: initialPrompt, 
        threadId: get(activeThreadId),
        context: buildLightReasoningContext()
      });
      if (intent?.intent_mode === 'question' || intent?.intent_mode === 'design') {
        isQuestion = intent.intent_mode === 'question';
      }
      if (intent?.response) {
        lightResponse = intent.response;
        appState.setCookingPhrase(lightResponse);
      }
      session.updateRequest({ questionMode: isQuestion });
      console.log(`[SessionFlow:${runId}] Classified as: ${isQuestion ? 'question' : 'design'}`);
    } catch (e) {
      console.warn(`[SessionFlow:${runId}] Intent classification fallback:`, e);
    }

    let currentScreenshot = null;
    if (viewerComponent && get(stlUrl)) {
      currentScreenshot = viewerComponent.captureScreenshot();
      session.updateRequest({ screenshot: currentScreenshot });
      console.log(`[SessionFlow:${runId}] Captured 3D viewport screenshot.`);
    }

    if (isQuestion) {
      console.log(`[SessionFlow:${runId}] Entering lightweight Q&A flow.`);
      session.setPhase('answering');
      session.setStatus('Answering question...');
      
      const questionReplyText = lightResponse || 'Question answered. Geometry unchanged.';
      
      const result = await invoke('answer_question_light', {
        prompt: initialPrompt,
        response: questionReplyText,
        threadId: get(activeThreadId),
        titleHint: get(activeThreadId) ? undefined : 'Question Session',
        imageData: currentScreenshot,
        attachments: attachments
      });
      
      activeThreadId.set(result.thread_id);
      await refreshHistory();
      
      session.setStatus(result.response || questionReplyText);
      session.setPhase('idle');
      session.finishRequest();
      stopLightReasoning();
      console.log(`[SessionFlow:${runId}] Q&A complete. Geometry unchanged.`);
      return;
    }

    stopLightReasoning();
    startCooking();

    let attempt = 1;
    let currentPrompt = initialPrompt;

    while (attempt <= 3) {
      session.setPhase('generating');
      session.setStatus(`Consulting LLM (Attempt ${attempt}/3)...`);
      session.updateRequest({ attempt, currentPrompt });
      console.log(`[SessionFlow:${runId}] LLM Generation Attempt ${attempt}/3 started.`);

      try {
        const result = await invoke('generate_design', { 
          prompt: currentPrompt,
          threadId: get(activeThreadId),
          parentMacroCode: get(workingCopy).macroCode || null,
          workingDesign: buildWorkingDesignSnapshot(),
          isRetry: attempt > 1,
          imageData: currentScreenshot,
          attachments: attachments,
          questionMode: false
        });
        
        console.log(`[SessionFlow:${runId}] LLM responded successfully.`);
        const data = result.design;
        const interactionMode = `${data.interaction_mode ?? ''}`.toLowerCase();

        if (interactionMode === 'question') {
          console.log(`[SessionFlow:${runId}] LLM decided this is just a question after all.`);
          const qResult = await invoke('answer_question_light', {
            prompt: currentPrompt,
            response: data.response || 'Question answered.',
            threadId: result.thread_id,
            imageData: currentScreenshot,
            attachments: attachments
          });
          activeThreadId.set(qResult.thread_id);
          await refreshHistory();
          session.setStatus(data.response || 'Question answered.');
          session.setPhase('idle');
          session.finishRequest();
          stopLightReasoning();
          stopCooking(true);
          break;
        }

        session.setStatus('Executing FreeCAD engine...');
        console.log(`[SessionFlow:${runId}] Validating geometry via FreeCAD...`);
        let absolutePath = null;
        try {
          session.setPhase('rendering');
          absolutePath = await invoke('render_stl', { 
            macroCode: data.macro_code, 
            parameters: data.initial_params || {}
          });
          session.setStlUrl(convertFileSrc(absolutePath));
          console.log(`[SessionFlow:${runId}] FreeCAD render SUCCESS. STL path: ${absolutePath}`);
        } catch (renderError) {
          console.error(`[SessionFlow:${runId}] FreeCAD render FAILED:`, renderError);
          if (attempt < 3) {
            console.log(`[SessionFlow:${runId}] Entering repair cycle...`);
            session.setPhase('repairing');
            const repairMsg = pickRetryMessage(attempt + 1, 3);
            appState.setRepairMessage(repairMsg);
            currentPrompt = `The previous code failed in FreeCAD with this error:\n${renderError}\n\nPlease fix it.`;
            attempt++;
            continue; // Go back to the start of the while loop
          } else {
            console.error(`[SessionFlow:${runId}] Max attempts reached. Giving up.`);
            session.setError(`Render Error: ${renderError}`);
            session.setPhase('error');
            workingCopy.patch({
              macroCode: data.macro_code,
              uiSpec: data.ui_spec,
              params: data.initial_params || {}
            });
            appState.openCodeModalManual(data);
            stopCooking(false);
            break;
          }
        }

        // If we reach here, render succeeded. Now try to commit.
        try {
          activeThreadId.set(result.threadId);
          activeVersionId.set(result.messageId);
          workingCopy.loadVersion(data, result.messageId);
          
          await refreshHistory();
          
          session.setStatus('Design synthesized successfully.');
          session.setPhase('idle');
          session.finishRequest();
          stopCooking(true);
          break; // Successfully finished
        } catch (commitError) {
          console.error(`[SessionFlow:${runId}] Commit to database FAILED:`, commitError);
          session.setError(`Database Error: ${commitError}`);
          session.setPhase('error');
          // Update working copy so the user doesn't lose the generated code
          workingCopy.patch({
            macroCode: data.macro_code,
            uiSpec: data.ui_spec,
            params: data.initial_params || {}
          });
          stopCooking(false);
          break;
        }
      } catch (e) {
        console.error(`[SessionFlow:${runId}] LLM API or network error:`, e);
        session.setError(`Generation Failed: ${e}`);
        session.setPhase('error');
        stopCooking(false);
        break;
      }
    }
  } catch (err) {
    console.error(`[SessionFlow:${runId}] Critical pipeline error:`, err);
    session.setError(`Pipeline Error: ${err}`);
    session.setPhase('error');
    stopCooking(false);
  } finally {
    stopLightReasoning();
    generationInFlight = false;
  }
}

export async function handleParamChange(newParams, forcedCode = null) {
  const wc = get(workingCopy);
  const currentParams = { ...wc.params, ...newParams };
  workingCopy.updateParams(newParams);
  
  const codeToUse = forcedCode || wc.macroCode;
  if (!codeToUse) return;

  console.log('[SessionFlow] Parameters updated, triggering re-render...');

  if (wc.sourceVersionId) {
    try {
      await invoke('update_parameters', { messageId: wc.sourceVersionId, parameters: currentParams });
      await refreshHistory();
    } catch (e) {
      console.error('[SessionFlow] Failed to persist parameters:', e);
    }
  }
  
  session.setStatus('Executing FreeCAD engine...');
  try {
    session.setPhase('rendering');
    const absolutePath = await invoke('render_stl', { 
      macroCode: codeToUse, 
      parameters: currentParams 
    });
    session.setStlUrl(convertFileSrc(absolutePath));
    session.setPhase('idle');
    console.log('[SessionFlow] Param-driven re-render SUCCESS.');
  } catch (e) {
    console.error('[SessionFlow] Param-driven re-render FAILED:', e);
    session.setError(`Render Error: ${e}`);
    session.setPhase('error');
  }
}

export async function commitManualVersion(editedCode) {
  const wc = get(workingCopy);
  const tid = get(activeThreadId);
  
  if (!tid) {
    session.setError("Cannot commit manual version: No active thread. Please generate first.");
    session.setPhase('error');
    return;
  }
  
  session.setStatus("Validating manual edit...");
  try {
    session.setPhase('rendering');
    const absolutePath = await invoke('render_stl', { 
      macroCode: editedCode, 
      parameters: wc.params 
    });
    session.setStlUrl(convertFileSrc(absolutePath));
    
    await invoke('add_manual_version', {
      threadId: tid,
      title: wc.title || "Manual Edit",
      versionName: "V-manual",
      macroCode: editedCode,
      parameters: wc.params,
      uiSpec: wc.uiSpec
    });
    
    workingCopy.loadVersion({ ...wc, macro_code: editedCode }, "manual");
    showCodeModal.set(false);
    session.setStatus("Manual version committed successfully.");
    session.setPhase('idle');
    
    await refreshHistory();
    
  } catch (e) {
    session.setError(`Manual Commit Failed: ${e}`);
    session.setStatus("Validation failed. Check your Python code.");
    session.setPhase('error');
  }
}
