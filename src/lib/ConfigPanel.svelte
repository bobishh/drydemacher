<script lang="ts">
  import { onMount } from 'svelte';
  import Dropdown from './Dropdown.svelte';
  import { open } from '@tauri-apps/plugin-dialog';
  import {
    formatBackendError,
    getMcpServerStatus,
    getSystemPrompt,
    saveRecordedAudio,
    uploadAsset,
  } from './tauri/client';
  import type { AppConfig, McpServerStatus } from './types/domain';

  type ActiveSection = 'engines' | 'assets';
  type RecordingTarget = 'hum' | 'ding';
  type MicLoadState = 'idle' | 'loading' | 'ready' | 'error';
  type MicrophoneOption = {
    id: string;
    name: string;
  };

  let {
    config = $bindable(),
    availableModels = [],
    isLoadingModels = false,
    onfetch,
    onsave,
  }: {
    config: AppConfig;
    availableModels?: string[];
    isLoadingModels?: boolean;
    onfetch?: () => Promise<void> | void;
    onsave?: () => Promise<void> | void;
  } = $props();

  let isSaving = $state(false);
  let message = $state('');
  let activeSection = $state<ActiveSection>('engines');

  // Recording state
  let isRecording = $state(false);
  let recordingTarget = $state<RecordingTarget | null>(null);
  let mediaRecorder: MediaRecorder | null = null;
  let audioChunks: Blob[] = [];
  let recordingTimer = $state(0);
  let timerInterval: ReturnType<typeof setInterval> | null = null;
  let micOptions = $state<MicrophoneOption[]>([]);
  let selectedMicId = $state('');
  let micLoadState = $state<MicLoadState>('idle');
  let micStatusMessage = $state('');
  let selectedAssetId = $state('');
  let mcpStatus = $state<McpServerStatus | null>(null);
  let mcpStatusMessage = $state('');

  type McpAgentSnippet = {
    id: string;
    label: string;
    location: string;
    snippet: string;
  };

  const providers = [
    { id: 'gemini', name: 'Google Gemini' },
    { id: 'openai', name: 'OpenAI (or Compatible)' },
    { id: 'ollama', name: 'Ollama (Local)' }
  ];

  const formats = [
    { id: 'MP3', name: 'MP3 (Audio)' },
    { id: 'WAV', name: 'WAV (Audio)' },
    { id: 'STL', name: 'STL (3D Mesh)' },
    { id: 'STEP', name: 'STEP (BRep)' },
    { id: 'PNG', name: 'PNG (Reference)' },
    { id: 'JPG', name: 'JPG (Reference)' },
    { id: 'JSON', name: 'JSON (Data)' }
  ];

  const selectedEngine = $derived(config.engines.find(e => e.id === config.selectedEngineId));

  function asString(value: string | number | null | undefined): string {
    if (typeof value === 'string') return value;
    if (typeof value === 'number') return String(value);
    return '';
  }

  function getMicrowaveConfig(): NonNullable<AppConfig['microwave']> {
    if (!config.microwave) {
      config.microwave = {
        humId: null,
        dingId: null,
        muted: false
      };
    } else if (typeof config.microwave.muted !== 'boolean') {
      config.microwave.muted = false;
    }
    return config.microwave;
  }
  const microwave = $derived.by(() => getMicrowaveConfig());

  if (typeof config.freecadCmd !== 'string') {
    config.freecadCmd = '';
  }

  const mcpEndpoint = $derived(mcpStatus?.endpointUrl || 'http://127.0.0.1:39249/mcp');

  const mcpAgentSnippets = $derived.by<McpAgentSnippet[]>(() => {
    const endpoint = mcpEndpoint;
    return [
      {
        id: 'gemini',
        label: 'GEMINI CLI',
        location: '~/.gemini/settings.json',
        snippet: JSON.stringify(
          {
            mcpServers: {
              ecky_mcp: {
                httpUrl: endpoint,
              },
            },
          },
          null,
          2,
        ),
      },
      {
        id: 'codex',
        label: 'CODEX',
        location: '~/.codex/config.toml',
        snippet: `[mcp_servers.ecky_mcp]\nenabled = true\nurl = "${endpoint}"\n`,
      },
      {
        id: 'claude',
        label: 'CLAUDE CODE',
        location: '.mcp.json or ~/.claude.json',
        snippet: JSON.stringify(
          {
            mcpServers: {
              ecky_mcp: {
                type: 'http',
                url: endpoint,
              },
            },
          },
          null,
          2,
        ),
      },
    ];
  });

  const genericMcpSnippet = $derived.by(() => {
    const endpoint = mcpStatus?.endpointUrl || 'http://127.0.0.1:39249/mcp';
    return JSON.stringify(
      {
        mcpServers: {
          ecky_mcp: {
            httpUrl: endpoint
          }
        }
      },
      null,
      2,
    );
  });

  async function refreshMcpStatus() {
    try {
      mcpStatus = await getMcpServerStatus();
      mcpStatusMessage = mcpStatus.running
        ? 'Local HTTP MCP server is running.'
        : (mcpStatus.lastStartupError || 'Local HTTP MCP server is not running.');
    } catch (e: unknown) {
      mcpStatusMessage = `Failed to read MCP status: ${formatBackendError(e)}`;
    }
  }

  async function copyMcpSnippet(snippet: string, label: string) {
    try {
      await navigator.clipboard.writeText(snippet);
      mcpStatusMessage = `Copied ${label} MCP snippet.`;
    } catch (e: unknown) {
      mcpStatusMessage = `Copy failed: ${formatBackendError(e)}`;
    }
  }

  onMount(() => {
    void refreshMcpStatus();
  });

  async function handleSave() {
    isSaving = true;
    message = 'Saving registry...';
    try {
      if (onsave) await onsave();
      message = 'Registry saved successfully.';
    } catch (e: unknown) {
      message = `Error: ${formatBackendError(e)}`;
    } finally {
      isSaving = false;
    }
  }

  async function addEngine() {
    const id = `engine-${Date.now()}`;
    const defaultPrompt = await getSystemPrompt();
    const newEngine = {
      id,
      name: 'New Engine',
      provider: 'gemini',
      apiKey: '',
      model: '',
      lightModel: '',
      baseUrl: '',
      systemPrompt: defaultPrompt
    };
    config.engines = [...config.engines, newEngine];
    config.selectedEngineId = id;
    activeSection = 'engines';
  }

  async function refreshMicInputs(requestPermission = true) {
    if (!navigator?.mediaDevices) {
      micLoadState = 'error';
      micStatusMessage = 'Media devices are unavailable in this webview.';
      micOptions = [];
      selectedMicId = '';
      return;
    }

    micLoadState = 'loading';
    micStatusMessage = '';

    if (requestPermission) {
      try {
        const temp = await navigator.mediaDevices.getUserMedia({ audio: true });
        temp.getTracks().forEach(track => track.stop());
      } catch (e: unknown) {
        micLoadState = 'error';
        micStatusMessage = `Microphone access failed: ${formatBackendError(e)}`;
        micOptions = [];
        selectedMicId = '';
        return;
      }
    }

    try {
      const devices = await navigator.mediaDevices.enumerateDevices();
      micOptions = devices
        .filter(d => d.kind === 'audioinput')
        .map((d, i) => ({
          id: d.deviceId,
          name: d.label || `Microphone ${i + 1}`
        }));
      selectedMicId = micOptions.some(m => m.id === selectedMicId) ? selectedMicId : (micOptions[0]?.id ?? '');
      micLoadState = 'ready';
      micStatusMessage = micOptions.length === 0
        ? 'No named microphone inputs were returned. Recording will use the system default input if available.'
        : '';
    } catch (e: unknown) {
      micLoadState = 'error';
      micStatusMessage = `Failed to enumerate microphones: ${formatBackendError(e)}`;
      micOptions = [];
      selectedMicId = '';
      console.warn('Failed to enumerate microphones:', e);
    }
  }

  async function loadMicInputs() {
    await refreshMicInputs(true);
  }

  async function uploadMicrowaveAudio(target: RecordingTarget) {
    try {
      const selected = await open({
        multiple: false,
        filters: [
          { name: 'Audio Files', extensions: ['mp3', 'wav', 'webm', 'ogg', 'm4a', 'aac', 'flac'] }
        ]
      });
      if (typeof selected !== 'string') return;

      const path = selected;
      const name = path.split(/[\/\\]/).pop() || path;
      const ext = (name.split('.').pop() || 'WAV').toUpperCase();

      const asset = await uploadAsset({
        sourcePath: path,
        name,
        format: ext
      });

      const microwaveConfig = getMicrowaveConfig();
      config.assets = [...(config.assets || []), asset];
      if (target === 'hum') microwaveConfig.humId = asset.id;
      if (target === 'ding') microwaveConfig.dingId = asset.id;
      message = `Uploaded and assigned ${target.toUpperCase()} sound: ${name}`;
    } catch (e: unknown) {
      message = `Upload failed: ${formatBackendError(e)}`;
    }
  }

  async function startRecording(target: RecordingTarget) {
    try {
      const constraints: MediaStreamConstraints = selectedMicId
        ? { audio: { deviceId: { exact: selectedMicId } } }
        : { audio: true };
      const stream = await navigator.mediaDevices.getUserMedia(constraints);
      mediaRecorder = new MediaRecorder(stream);
      audioChunks = [];
      recordingTarget = target;
      recordingTimer = 0;

      mediaRecorder.ondataavailable = (event) => {
        audioChunks.push(event.data);
      };

      mediaRecorder.onstop = async () => {
        const audioBlob = new Blob(audioChunks, { type: 'audio/webm' });
        const reader = new FileReader();
        reader.readAsDataURL(audioBlob);
        reader.onloadend = async () => {
          if (typeof reader.result !== 'string') {
            message = 'Failed to read recording buffer.';
            return;
          }
          const base64data = reader.result.split(',')[1] ?? '';
          const name = `Recording: ${target.toUpperCase()} (${new Date().toLocaleTimeString()})`;
          
          try {
            const asset = await saveRecordedAudio({
              base64Data: base64data,
              name
            });
            const microwaveConfig = getMicrowaveConfig();
            config.assets = [...(config.assets || []), asset];
            if (target === 'hum') microwaveConfig.humId = asset.id;
            if (target === 'ding') microwaveConfig.dingId = asset.id;
            message = `Recorded ${target} saved and assigned.`;
          } catch (e: unknown) {
            message = `Failed to save recording: ${formatBackendError(e)}`;
          }
        };
      };

      mediaRecorder.start();
      isRecording = true;
      timerInterval = setInterval(() => {
        recordingTimer++;
      }, 1000);
    } catch (e: unknown) {
      message = `Microphone error: ${formatBackendError(e)}`;
    }
  }

  function stopRecording() {
    if (mediaRecorder) {
      mediaRecorder.stop();
      mediaRecorder.stream.getTracks().forEach(track => track.stop());
    }
    isRecording = false;
    if (timerInterval) {
      clearInterval(timerInterval);
      timerInterval = null;
    }
  }

  function removeEngine(id: string) {
    config.engines = config.engines.filter(e => e.id !== id);
    if (config.selectedEngineId === id) {
      config.selectedEngineId = config.engines.length > 0 ? config.engines[0].id : '';
    }
  }

  async function addAsset() {
    try {
      const selected = await open({
        multiple: false,
        filters: [
          { name: 'All Assets', extensions: ['stl', 'step', 'stp', 'png', 'jpg', 'jpeg', 'json'] }
        ]
      });

      if (typeof selected === 'string') {
        const path = selected;
        const ext = (path.split('.').pop() || '').toUpperCase();
        const name = path.split(/[\/\\]/).pop() || path;
        
        const asset = await uploadAsset({
          sourcePath: path,
          name,
          format: ext
        });

        config.assets = [...(config.assets || []), asset];
        selectedAssetId = asset.id;
        activeSection = 'assets';
        message = `Asset ${name} added.`;
      }
    } catch (e: unknown) {
      message = `Upload failed: ${formatBackendError(e)}`;
    }
  }

  function removeAsset(id: string) {
    config.assets = config.assets.filter(a => a.id !== id);
    if (selectedAssetId === id) {
      selectedAssetId = '';
    }
  }

  async function refreshModels() {
    if (!onfetch) return;
    try {
      await onfetch();
    } catch (e: unknown) {
      message = `Model fetch failed: ${formatBackendError(e)}`;
    }
  }

  async function handleProviderChange() {
    if (selectedEngine) {
      selectedEngine.model = '';
      selectedEngine.lightModel = '';
    }
    await refreshModels();
  }
  async function resetPrompt() {
    if (selectedEngine) {
      selectedEngine.systemPrompt = await getSystemPrompt();
    }
  }
</script>

<div class="config-container">
  <aside class="config-sidebar">
    <div class="sidebar-group">
      <div class="list-header">
        <span>ENGINES</span>
        <button class="btn btn-xs" onclick={addEngine}>+ ADD</button>
      </div>
      <div class="list-content">
        {#each config.engines as engine}
          <button 
            class="engine-item {activeSection === 'engines' && config.selectedEngineId === engine.id ? 'active' : ''}"
            onclick={() => { config.selectedEngineId = engine.id; activeSection = 'engines'; }}
          >
            <span class="engine-name">{engine.name || '(unnamed)'}</span>
            <span class="engine-provider">{engine.provider}</span>
          </button>
        {/each}
      </div>
    </div>

    <div class="sidebar-group">
      <div class="list-header">
        <span>GLOBAL MEDIA / SOUNDS</span>
        <button class="btn btn-xs" onclick={addAsset}>+ UPLOAD</button>
      </div>
      <div class="list-content">
        {#each (config.assets || []) as asset}
          <button 
            class="engine-item {activeSection === 'assets' && selectedAssetId === asset.id ? 'active' : ''}"
            onclick={() => { selectedAssetId = asset.id; activeSection = 'assets'; }}
          >
            <span class="engine-name">{asset.name}</span>
            <span class="engine-provider">{asset.format}</span>
          </button>
        {/each}
        {#if !config.assets || config.assets.length === 0}
          <div class="empty-sidebar-msg">No media uploaded.</div>
        {/if}
      </div>
    </div>
    <div class="sidebar-group">
      <div class="list-header">
        <span>MICROWAVE SOUNDS</span>
      </div>
      <div class="list-content microwave-assignments">
        <div class="mic-device-block">
          <div class="mic-device-header">
            <span class="role-label">MIC INPUT</span>
            <button
              class="btn btn-xs btn-ghost"
              onclick={loadMicInputs}
              disabled={isRecording || micLoadState === 'loading'}
            >
              {micLoadState === 'idle' ? 'LOAD INPUTS' : '↻ RESCAN'}
            </button>
          </div>

          {#if micLoadState === 'idle'}
            <div class="mic-status">
              Load microphone inputs only when needed. Recording still works with the system default input.
            </div>
          {:else if micLoadState === 'loading'}
            <div class="mic-status">Scanning microphones...</div>
          {:else if micLoadState === 'error'}
            <div class="mic-status mic-status-error">{micStatusMessage}</div>
          {:else if micOptions.length > 1}
            <div class="mic-device-row">
              <Dropdown
                options={micOptions}
                value={selectedMicId}
                onchange={(val) => selectedMicId = asString(val)}
                placeholder="Select microphone..."
                disabled={isRecording}
              />
            </div>
          {:else if micOptions.length === 1}
            <div class="mic-status">
              Using `{micOptions[0].name}` when recording.
            </div>
          {:else}
            <div class="mic-status">{micStatusMessage}</div>
          {/if}
        </div>

        <div class="sound-role">
          <span class="role-label">COOKING HUM</span>
          <div class="role-actions">
            {#if isRecording && recordingTarget === 'hum'}
              <button class="btn btn-xs btn-danger pulse" onclick={stopRecording}>⏹ STOP ({recordingTimer}s)</button>
            {:else}
              <button class="btn btn-xs" onclick={() => startRecording('hum')} disabled={isRecording}>🎤 RECORD</button>
            {/if}
            <button class="btn btn-xs btn-ghost" onclick={() => uploadMicrowaveAudio('hum')} disabled={isRecording}>📁 UPLOAD HUM</button>
            {#if microwave.humId}
              <button class="btn btn-xs btn-ghost" onclick={() => microwave.humId = null}>✕ CLEAR</button>
            {/if}
          </div>
          {#if microwave.humId}
            {@const asset = config.assets?.find(a => a.id === microwave.humId)}
            <span class="assigned-name">{asset?.name || 'Assigned'}</span>
          {/if}
        </div>

        <div class="sound-role">
          <span class="role-label">DONE DING</span>
          <div class="role-actions">
            {#if isRecording && recordingTarget === 'ding'}
              <button class="btn btn-xs btn-danger pulse" onclick={stopRecording}>⏹ STOP ({recordingTimer}s)</button>
            {:else}
              <button class="btn btn-xs" onclick={() => startRecording('ding')} disabled={isRecording}>🎤 RECORD</button>
            {/if}
            <button class="btn btn-xs btn-ghost" onclick={() => uploadMicrowaveAudio('ding')} disabled={isRecording}>📁 UPLOAD DING</button>
            {#if microwave.dingId}
              <button class="btn btn-xs btn-ghost" onclick={() => microwave.dingId = null}>✕ CLEAR</button>
            {/if}
          </div>
          {#if microwave.dingId}
            {@const asset = config.assets?.find(a => a.id === microwave.dingId)}
            <span class="assigned-name">{asset?.name || 'Assigned'}</span>
          {/if}
        </div>
      </div>
    </div>
  </aside>

  <main class="engine-details">
    <div class="details-scrollable">
      <div class="details-content">
        <div class="field">
          <div class="prompt-header">
            <label for="freecad-cmd">FREECAD COMMAND / APP (GLOBAL)</label>
            <button class="btn btn-xs btn-ghost" onclick={() => config.freecadCmd = ''}>AUTO DISCOVER</button>
          </div>
          <input
            id="freecad-cmd"
            type="text"
            class="input-mono"
            placeholder="/Applications/FreeCAD.app or /Applications/FreeCAD.app/Contents/Resources/bin/freecadcmd"
            bind:value={config.freecadCmd}
          />
          <div class="field-help">
            Leave blank to auto-detect via `FREECAD_CMD`, PATH, or standard macOS FreeCAD locations.
          </div>
        </div>

        <div class="field">
          <div class="prompt-header">
            <div class="field-title">ADD THIS TO YOUR AGENTIC TOOL</div>
            <div class="inline-actions">
              <button class="btn btn-xs btn-ghost" onclick={refreshMcpStatus}>REFRESH MCP</button>
              <button class="btn btn-xs" onclick={() => copyMcpSnippet(genericMcpSnippet, 'generic JSON')}>COPY GENERIC JSON</button>
            </div>
          </div>
          <div class="mcp-status-row">
            <span class:mcp-running={mcpStatus?.running} class:mcp-stopped={!mcpStatus?.running}>
              {mcpStatus?.running ? 'RUNNING' : 'STOPPED'}
            </span>
            <span class="mcp-endpoint">{mcpStatus?.endpointUrl || 'http://127.0.0.1:39249/mcp'}</span>
          </div>
          <div class="field-help">
            Canonical local MCP endpoint for agent clients. Pick your host and copy the right config format directly.
          </div>
          <div class="mcp-agent-grid">
            {#each mcpAgentSnippets as agent (agent.id)}
              <div class="mcp-agent-card">
                <div class="mcp-agent-card__head">
                  <span class="mcp-agent-card__label">{agent.label}</span>
                  <button class="btn btn-xs" onclick={() => copyMcpSnippet(agent.snippet, agent.label)}>
                    COPY
                  </button>
                </div>
                <div class="mcp-agent-card__path">{agent.location}</div>
              </div>
            {/each}
          </div>
          {#if mcpStatusMessage}
            <div class="field-note">{mcpStatusMessage}</div>
          {/if}
          {#if mcpStatus?.lastStartupError}
            <div class="field-note">Last startup error: {mcpStatus.lastStartupError}</div>
          {/if}
        </div>

      {#if activeSection === 'engines' && selectedEngine}
          <div class="field-row">
            <div class="field flex-2">
              <label for="e-name">DISPLAY NAME</label>
              <input 
                id="e-name" 
                type="text" 
                class="input-mono" 
                placeholder="e.g. My Gemini" 
                bind:value={selectedEngine.name}
              />
            </div>
            <div class="field flex-1">
              <label for="e-provider">PROVIDER</label>
              <Dropdown 
                options={providers} 
                value={selectedEngine.provider} 
                onchange={async (val) => { selectedEngine.provider = asString(val); await handleProviderChange(); }} 
              />
            </div>
          </div>

          <div class="field">
            <label for="e-key">API KEY</label>
            <input 
              id="e-key" 
              type="password" 
              class="input-mono" 
              placeholder="Enter API key..." 
              bind:value={selectedEngine.apiKey}
              onblur={refreshModels}
            />
          </div>

          <div class="field-row">
            <div class="field flex-1">
              <div class="prompt-header">
                <label for="e-model">RENDER AND HEAVY REASONING</label>
                <button class="btn btn-xs btn-ghost" onclick={refreshModels} disabled={isLoadingModels}>
                  ↻ FETCH MODELS
                </button>
              </div>
              <Dropdown 
                options={availableModels.length > 0 ? availableModels : (selectedEngine.model ? [selectedEngine.model] : [])} 
                value={selectedEngine.model} 
                placeholder={isLoadingModels ? "Fetching..." : "Fetch models first..."} 
                onchange={(val) => selectedEngine.model = asString(val)}
              />
            </div>
            <div class="field flex-1">
              <label for="e-light-model">LIGHT REASONING</label>
              <Dropdown
                options={availableModels.length > 0 ? availableModels : (selectedEngine.lightModel ? [selectedEngine.lightModel] : (selectedEngine.model ? [selectedEngine.model] : []))}
                value={selectedEngine.lightModel}
                placeholder={isLoadingModels ? "Fetching..." : "Optional (falls back to heavy model)"}
                onchange={(val) => selectedEngine.lightModel = asString(val)}
              />
              <div class="field-note">Used for text-only intent checks. Image-bearing requests fall back to the main model.</div>
            </div>
          </div>

          <div class="field">
            <label for="e-baseurl">BASE URL (OPTIONAL)</label>
            <input 
              id="e-baseurl" 
              type="text" 
              class="input-mono" 
              placeholder="Default" 
              bind:value={selectedEngine.baseUrl}
              onblur={refreshModels}
            />
          </div>

          <div class="field prompt-field">
            <div class="prompt-header">
              <label for="e-prompt">SYSTEM PROMPT (template with $USER_PROMPT)</label>
              <button class="btn btn-xs" onclick={resetPrompt}>RESET TO DEFAULT</button>
            </div>
            <textarea 
              id="e-prompt" 
              class="input-mono system-prompt-input" 
              spellcheck="false"
              bind:value={selectedEngine.systemPrompt}
              placeholder="Template for LLM. Use $USER_PROMPT as placeholder for user intent."
            ></textarea>
          </div>

          <div class="danger-zone">
            <button class="btn btn-xs btn-ghost" onclick={() => removeEngine(selectedEngine.id)}>REMOVE ENGINE</button>
          </div>
      {:else if activeSection === 'assets'}
        {@const selectedAsset = config.assets?.find(a => a.id === selectedAssetId)}
        {#if selectedAsset}
            <div class="field">
              <span>ASSET NAME</span>
              <input type="text" bind:value={selectedAsset.name} class="input-mono" />
            </div>
            <div class="field">
              <span>FORMAT</span>
              <Dropdown options={formats} bind:value={selectedAsset.format} />
            </div>
            
            <div class="field">
              <span>ASSIGN TO MICROWAVE</span>
              <div class="assignment-buttons">
                <button 
                  class="btn btn-xs {microwave.humId === selectedAsset.id ? 'btn-primary' : 'btn-ghost'}"
                  onclick={() => microwave.humId = selectedAsset.id}
                >
                  ASSIGN AS HUM (COOKING)
                </button>
                <button 
                  class="btn btn-xs {microwave.dingId === selectedAsset.id ? 'btn-primary' : 'btn-ghost'}"
                  onclick={() => microwave.dingId = selectedAsset.id}
                >
                  ASSIGN AS DING (DONE)
                </button>
              </div>
            </div>

            <div class="field">
              <span>LOCAL PATH</span>
              <div class="path-display">{selectedAsset.path}</div>
            </div>
            <div class="danger-zone">
              <button class="btn btn-xs btn-ghost" onclick={() => removeAsset(selectedAsset.id)}>REMOVE ASSET</button>
            </div>
        {:else}
          <div class="no-engine">Select media to view details.</div>
        {/if}
      {:else}
        <div class="no-engine">
          <p>No engine selected. Add one to begin.</p>
          <button class="btn btn-primary" onclick={addEngine}>ADD FIRST ENGINE</button>
        </div>
      {/if}
      </div>
    </div>

    <div class="config-footer">
      <span class="status-msg">{message}</span>
      <button class="btn btn-primary" onclick={handleSave} disabled={isSaving || config.engines.length === 0}>
        {isSaving ? 'SAVING...' : 'SAVE REGISTRY'}
      </button>
    </div>
  </main>
</div>

<style>
  .config-container {
    display: flex;
    height: 100%;
    width: 100%;
    background: var(--bg-100);
    overflow: hidden;
  }

  .config-sidebar {
    width: 240px;
    flex-shrink: 0;
    border-right: 1px solid var(--bg-300);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .sidebar-group {
    display: flex;
    flex-direction: column;
    max-height: 50%;
    border-bottom: 2px solid var(--bg-300);
  }

  .sidebar-group:last-child {
    flex: 1;
    border-bottom: none;
  }

  .list-header {
    padding: 12px;
    display: flex;
    justify-content: space-between;
    align-items: center;
    border-bottom: 1px solid var(--bg-300);
    background: var(--bg-200);
  }

  .list-content {
    flex: 1;
    overflow-y: auto;
  }

  .engine-item {
    width: 100%;
    padding: 10px 12px;
    text-align: left;
    background: none;
    border: none;
    border-bottom: 1px solid var(--bg-300);
    display: flex;
    flex-direction: column;
    gap: 4px;
    cursor: pointer;
  }

  .engine-item:hover {
    background: var(--bg-200);
  }

  .engine-item.active {
    background: var(--bg-300);
    border-left: 3px solid var(--primary);
  }

  .engine-name {
    font-size: 0.75rem;
    font-weight: bold;
    color: var(--text);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .engine-provider {
    font-size: 0.6rem;
    color: var(--secondary);
    text-transform: uppercase;
  }

  .empty-sidebar-msg {
    padding: 20px;
    font-size: 0.7rem;
    color: var(--text-dim);
    text-align: center;
    font-style: italic;
  }

  .engine-details {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
    overflow: hidden;
  }

  .details-scrollable {
    flex: 1;
    overflow-y: auto;
  }

  .details-content {
    padding: 24px;
    display: flex;
    flex-direction: column;
    gap: 20px;
  }

  .field-row {
    display: flex;
    gap: 16px;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .field-help {
    font-size: 0.65rem;
    color: var(--text-dim);
    line-height: 1.4;
  }

  .field-note {
    font-size: 0.62rem;
    color: var(--text-dim);
    line-height: 1.35;
  }

  .path-display {
    font-family: var(--font-mono);
    font-size: 0.7rem;
    color: var(--text-dim);
    background: var(--bg-200);
    padding: 8px;
    border: 1px solid var(--bg-300);
    word-break: break-all;
  }

  .flex-1 { flex: 1; }
  .flex-2 { flex: 2; }

  label {
    font-size: 0.65rem;
    color: var(--text-dim);
    font-weight: bold;
    letter-spacing: 0.05em;
  }

  .field-title {
    font-size: 0.65rem;
    color: var(--text-dim);
    font-weight: bold;
    letter-spacing: 0.05em;
  }

  input, textarea {
    padding: 8px 12px;
    background: var(--bg-200);
    border: 1px solid var(--bg-300);
    color: var(--text);
    font-size: 0.8rem;
    outline: none;
    font-family: var(--font-mono);
    width: 100%;
  }

  input:focus, textarea:focus {
    border-color: var(--primary);
  }

  .prompt-field {
    flex: 1;
    min-height: 300px;
    display: flex;
    flex-direction: column;
  }

  .prompt-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 6px;
  }

  .inline-actions {
    display: flex;
    gap: 8px;
  }

  .mcp-status-row {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
  }

  .mcp-running,
  .mcp-stopped {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 74px;
    padding: 2px 8px;
    border: 1px solid var(--bg-300);
    font-size: 0.62rem;
    font-weight: bold;
    letter-spacing: 0.06em;
    background: var(--bg-200);
  }

  .mcp-running {
    border-color: var(--secondary);
    color: var(--secondary);
  }

  .mcp-stopped {
    border-color: var(--primary);
    color: var(--primary);
  }

  .mcp-endpoint {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-family: var(--font-mono);
    font-size: 0.68rem;
    color: var(--text-dim);
  }

  .mcp-agent-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
    gap: 10px;
  }

  .mcp-agent-card {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 10px 12px;
    border: 1px solid var(--bg-300);
    background: var(--bg-200);
    overflow: hidden;
  }

  .mcp-agent-card__head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }

  .mcp-agent-card__label {
    font-size: 0.66rem;
    font-weight: bold;
    letter-spacing: 0.06em;
    color: var(--text);
  }

  .mcp-agent-card__path {
    font-family: var(--font-mono);
    font-size: 0.62rem;
    color: var(--text-dim);
    word-break: break-word;
  }

  .system-prompt-input {
    flex: 1;
    resize: none;
    line-height: 1.5;
  }

  .danger-zone {
    margin-top: 12px;
    padding-top: 12px;
    border-top: 1px solid var(--bg-300);
  }

  .config-footer {
    padding: 16px 24px;
    border-top: 1px solid var(--bg-300);
    display: flex;
    justify-content: space-between;
    align-items: center;
    background: var(--bg-100);
  }

  .no-engine {
    height: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 16px;
    color: var(--text-dim);
    font-size: 0.8rem;
  }

  .btn-xs {
    padding: 2px 6px;
    font-size: 0.6rem;
  }

  .status-msg {
    font-size: 0.75rem;
    color: var(--secondary);
  }

  .microwave-assignments {
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .sound-role {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .mic-device-block {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding-bottom: 8px;
    border-bottom: 1px solid var(--bg-300);
  }

  .mic-device-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }

  .mic-device-row {
    display: flex;
    gap: 6px;
    align-items: center;
  }

  .mic-device-row :global(.custom-select) {
    flex: 1;
  }

  .role-label {
    font-size: 0.6rem;
    font-weight: bold;
    color: var(--text-dim);
    letter-spacing: 0.05em;
  }

  .mic-status {
    font-size: 0.65rem;
    line-height: 1.4;
    color: var(--text-dim);
  }

  .mic-status-error {
    color: var(--red);
    white-space: pre-wrap;
  }

  .role-actions {
    display: flex;
    gap: 6px;
  }

  .assigned-name {
    font-size: 0.6rem;
    color: var(--secondary);
    font-family: var(--font-mono);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .btn-danger {
    background: var(--red);
    color: white;
    border: none;
  }

  .pulse {
    animation: pulse-red 1.5s infinite;
  }

  @keyframes pulse-red {
    0% { opacity: 1; }
    50% { opacity: 0.6; }
    100% { opacity: 1; }
  }
</style>
