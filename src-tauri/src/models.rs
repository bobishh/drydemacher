use crate::contracts::{
    AgentTerminalSnapshot, AgentWorkingVersionEvent, AppError, AppLogEntry, AppResult, Config,
    LastDesignSnapshot, McpServerStatus, ResolveAgentPromptInput, ViewportCameraState,
};
#[cfg(unix)]
use libc;
use std::collections::{HashMap, HashSet, VecDeque};
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use tauri::{Emitter, Manager};
use tokio::sync::oneshot;

/// Pending user-prompt requests keyed by requestId.
type PromptChannels = Arc<
    tokio::sync::Mutex<HashMap<String, oneshot::Sender<Result<ResolveAgentPromptInput, String>>>>,
>;

pub trait PathResolver: Send + Sync {
    fn app_config_dir(&self) -> PathBuf;
    fn try_app_config_dir(&self) -> AppResult<PathBuf> {
        Ok(self.app_config_dir())
    }
    fn app_data_dir(&self) -> PathBuf;
    fn resource_path(&self, path: &str) -> Option<PathBuf>;
}

fn env_path_override(name: &str) -> Option<PathBuf> {
    std::env::var_os(name)
        .map(PathBuf::from)
        .filter(|path| !path.as_os_str().is_empty())
}

impl PathResolver for tauri::AppHandle {
    fn app_config_dir(&self) -> PathBuf {
        env_path_override("ECKY_APP_CONFIG_DIR")
            .unwrap_or_else(|| self.path().app_config_dir().unwrap())
    }
    fn try_app_config_dir(&self) -> AppResult<PathBuf> {
        if let Some(path) = env_path_override("ECKY_APP_CONFIG_DIR") {
            return Ok(path);
        }
        self.path()
            .app_config_dir()
            .map_err(|_| AppError::persistence("config path resolution failed: app-config-dir"))
    }
    fn app_data_dir(&self) -> PathBuf {
        env_path_override("ECKY_APP_DATA_DIR")
            .or_else(|| env_path_override("ECKY_APP_CONFIG_DIR"))
            .unwrap_or_else(|| self.path().app_data_dir().unwrap())
    }
    fn resource_path(&self, path: &str) -> Option<PathBuf> {
        self.path()
            .resolve(path, tauri::path::BaseDirectory::Resource)
            .ok()
    }
}

impl<T: PathResolver + ?Sized> PathResolver for std::sync::Arc<T> {
    fn app_config_dir(&self) -> PathBuf {
        (**self).app_config_dir()
    }
    fn try_app_config_dir(&self) -> AppResult<PathBuf> {
        (**self).try_app_config_dir()
    }
    fn app_data_dir(&self) -> PathBuf {
        (**self).app_data_dir()
    }
    fn resource_path(&self, path: &str) -> Option<PathBuf> {
        (**self).resource_path(path)
    }
}

#[derive(Debug, Clone)]
pub struct McpTargetRef {
    pub thread_id: String,
    pub message_id: String,
    pub model_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct McpSessionState {
    pub client_kind: String,
    pub host_label: String,
    pub agent_label: String,
    pub llm_model_id: Option<String>,
    pub llm_model_label: Option<String>,
    pub bound_thread_id: Option<String>,
    pub last_target: Option<McpTargetRef>,
    pub phase: Option<String>,
    pub status_text: Option<String>,
    pub busy: bool,
    pub activity_label: Option<String>,
    pub activity_started_at: Option<u64>,
    pub attention_kind: Option<String>,
    pub waiting_on_prompt: bool,
    pub current_turn_id: Option<String>,
    pub current_turn_thread_id: Option<String>,
    pub current_turn_working_message_ids: Vec<String>,
    pub current_turn_working_version_message_id: Option<String>,
    pub updated_at: u64,
}

impl McpSessionState {
    fn now_secs() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    pub fn new(client_kind: String, host_label: String) -> Self {
        Self {
            client_kind,
            agent_label: host_label.clone(),
            host_label,
            llm_model_id: None,
            llm_model_label: None,
            bound_thread_id: None,
            last_target: None,
            phase: None,
            status_text: None,
            busy: false,
            activity_label: None,
            activity_started_at: None,
            attention_kind: None,
            waiting_on_prompt: false,
            current_turn_id: None,
            current_turn_thread_id: None,
            current_turn_working_message_ids: Vec::new(),
            current_turn_working_version_message_id: None,
            updated_at: Self::now_secs(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PromptResumeState {
    pub pgid: Option<i32>,
    pub agent_label: String,
    pub session_id: String,
    pub thread_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ViewportScreenshotCapture {
    pub data_url: String,
    pub width: u32,
    pub height: u32,
    pub camera: ViewportCameraState,
    pub source: String,
    pub thread_id: String,
    pub message_id: String,
    pub model_id: Option<String>,
    pub include_overlays: bool,
}

pub type ViewportScreenshotSender = oneshot::Sender<Result<ViewportScreenshotCapture, String>>;
pub type PendingViewportScreenshotChannels =
    Arc<tokio::sync::Mutex<HashMap<String, ViewportScreenshotSender>>>;
pub type AgentTerminalWriter = Arc<Mutex<Box<dyn Write + Send>>>;
pub type AgentTerminalPty = Arc<Mutex<Box<dyn portable_pty::MasterPty + Send>>>;

pub struct AgentTerminalRuntime {
    pub snapshot: AgentTerminalSnapshot,
    pub writer: AgentTerminalWriter,
    pub pty: AgentTerminalPty,
    pub pending_utf8: Vec<u8>,
    pub pending_escape: String,
    pub last_emitted_at: Option<Instant>,
}

pub type PendingAgentTerminalSessions = Arc<Mutex<HashMap<String, AgentTerminalRuntime>>>;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ConfigPersistenceStatus {
    pub cleanup_pending: bool,
    pub warnings: Vec<String>,
}

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Mutex<Config>>,
    pub config_persistence_status: Arc<Mutex<ConfigPersistenceStatus>>,
    pub last_snapshot: Arc<Mutex<Option<LastDesignSnapshot>>>,
    pub db: Arc<tokio::sync::Mutex<rusqlite::Connection>>,
    pub db_read: Option<Arc<tokio::sync::Mutex<rusqlite::Connection>>>,
    pub render_lock: Arc<tokio::sync::Mutex<()>>,
    pub mcp_status: Arc<Mutex<McpServerStatus>>,
    pub mcp_sessions: Arc<tokio::sync::Mutex<HashMap<String, McpSessionState>>>,
    /// MCP guide/resource URIs read by each live session.
    pub mcp_session_read_resources: Arc<tokio::sync::Mutex<HashMap<String, HashSet<String>>>>,
    /// OpenSpec `agent-context-budgeting` §5: capability groups explicitly enabled
    /// for each compact-managed MCP session (group ids, e.g. `target-reads`).
    /// Core workflow tools are always available; specialist groups load on demand
    /// via the `capability_enable` control. Kept as a side map (mirroring
    /// `mcp_session_read_resources`) so `McpSessionState` literals need no change.
    pub mcp_session_enabled_groups: Arc<tokio::sync::Mutex<HashMap<String, HashSet<String>>>>,
    /// OpenSpec `agent-context-budgeting` §5.3: queued MCP server→client
    /// notifications (e.g. `notifications/tools/list_changed`) keyed by session
    /// id. This Streamable-HTTP server answers each request with one JSON-RPC
    /// object, so server-initiated notifications are recorded here for delivery
    /// to managed agents on their next poll rather than as an out-of-band push.
    pub mcp_session_pending_notifications:
        Arc<tokio::sync::Mutex<HashMap<String, Vec<serde_json::Value>>>>,
    /// Pending user-confirmation requests keyed by requestId.
    pub confirm_channels: Arc<tokio::sync::Mutex<HashMap<String, oneshot::Sender<String>>>>,
    /// Pending user-prompt requests keyed by requestId (agent waits for text/attachments from UI).
    pub prompt_channels: PromptChannels,
    /// Runtime state machine for active-mode MCP agents.
    pub auto_agent_runtime: Arc<Mutex<crate::mcp::runtime::AutoAgentRuntimeRegistry>>,
    /// Maps prompt request_id → process control for agents SIGSTOP'd while waiting on the user.
    pub prompt_waits: Arc<Mutex<HashMap<String, PromptResumeState>>>,
    /// Pending viewport screenshot requests keyed by requestId.
    pub viewport_screenshot_channels: PendingViewportScreenshotChannels,
    /// Ring buffer of in-app log entries (latest 200 entries).
    pub app_logs: Arc<Mutex<VecDeque<AppLogEntry>>>,
    /// Active PTY-backed terminal bridges for interactive auto-agents.
    pub agent_terminals: PendingAgentTerminalSessions,
    /// AppState-scoped authoring actor registry. Each `AppState` owns its own
    /// registry so independent instances never share or invalidate each
    /// other's authoring actor state (even with identical `session_id`/
    /// `thread_id` strings), while a UI mutation still invalidates every
    /// session's actor for that thread within this `AppState`.
    pub authoring_actor_registry: Arc<crate::mcp::handlers::AuthoringActorRegistry>,
    /// App handle for emitting runtime PTY events back into the frontend.
    pub app_handle: Arc<Mutex<Option<tauri::AppHandle>>>,
}

impl AppState {
    pub fn new(
        config: Config,
        last_snapshot: Option<LastDesignSnapshot>,
        conn: rusqlite::Connection,
    ) -> Self {
        Self::new_with_read_connection(config, last_snapshot, conn, None)
    }

    pub fn new_with_read_connection(
        config: Config,
        last_snapshot: Option<LastDesignSnapshot>,
        conn: rusqlite::Connection,
        read_conn: Option<rusqlite::Connection>,
    ) -> Self {
        Self {
            config: Arc::new(Mutex::new(config)),
            config_persistence_status: Arc::new(Mutex::new(ConfigPersistenceStatus::default())),
            last_snapshot: Arc::new(Mutex::new(last_snapshot)),
            db: Arc::new(tokio::sync::Mutex::new(conn)),
            db_read: read_conn.map(|conn| Arc::new(tokio::sync::Mutex::new(conn))),
            render_lock: Arc::new(tokio::sync::Mutex::new(())),
            mcp_status: Arc::new(Mutex::new(McpServerStatus {
                running: false,
                endpoint_url: "http://127.0.0.1:39249/mcp".to_string(),
                last_startup_error: None,
            })),
            mcp_sessions: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
            mcp_session_read_resources: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
            mcp_session_enabled_groups: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
            mcp_session_pending_notifications: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
            confirm_channels: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
            prompt_channels: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
            auto_agent_runtime: Arc::new(Mutex::new(
                crate::mcp::runtime::AutoAgentRuntimeRegistry::default(),
            )),
            prompt_waits: Arc::new(Mutex::new(HashMap::new())),
            viewport_screenshot_channels: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
            app_logs: Arc::new(Mutex::new(VecDeque::new())),
            agent_terminals: Arc::new(Mutex::new(HashMap::new())),
            authoring_actor_registry: Arc::new(
                crate::mcp::handlers::AuthoringActorRegistry::default(),
            ),
            app_handle: Arc::new(Mutex::new(None)),
        }
    }

    pub fn set_app_handle(&self, handle: tauri::AppHandle) {
        *self.app_handle.lock().unwrap() = Some(handle);
    }

    pub fn push_log(&self, message: String) {
        let ts_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        let entry = AppLogEntry { ts_ms, message };
        let mut logs = self.app_logs.lock().unwrap();
        if logs.len() >= 200 {
            logs.pop_front();
        }
        logs.push_back(entry);
    }

    /// Record content-free context-budget telemetry to the in-app profiler
    /// (OpenSpec `agent-context-budgeting`, section 4.2). The record is derived
    /// from the shape-only [`ContextEnvelope`] plus optional numeric provider
    /// usage, then serialized to one profiler line. Because the telemetry is
    /// content-free by construction, no source, references, prompt, image bytes,
    /// API keys, authorization headers, or full paths can leak into the log.
    /// This reuses the existing session-activity/profiler ring buffer — it adds
    /// no status bar and no new UI surface.
    pub fn record_context_telemetry(
        &self,
        envelope: &crate::context_envelope::ContextEnvelope,
        usage: Option<&crate::contracts::UsageSummary>,
    ) {
        let telemetry_usage = usage.map(|u| crate::context_envelope::TelemetryUsage {
            input_tokens: u.input_tokens,
            output_tokens: u.output_tokens,
            total_tokens: u.total_tokens,
            cached_input_tokens: u.cached_input_tokens,
            reasoning_tokens: u.reasoning_tokens,
        });
        let telemetry = crate::context_envelope::envelope_telemetry(envelope, telemetry_usage);
        let json = serde_json::to_string(&telemetry).unwrap_or_else(|_| "{}".to_string());
        self.push_log(format!("[CONTEXT] {json}"));
    }

    pub fn set_mcp_status(&self, running: bool, last_startup_error: Option<String>) {
        let mut status = self.mcp_status.lock().unwrap();
        status.running = running;
        status.last_startup_error = last_startup_error;
    }

    pub fn mcp_status(&self) -> McpServerStatus {
        self.mcp_status.lock().unwrap().clone()
    }

    pub fn emit_agent_terminal_update(&self, snapshot: &AgentTerminalSnapshot) {
        let handle = self.app_handle.lock().unwrap().clone();
        if let Some(handle) = handle {
            let _ = handle.emit("agent-terminal-updated", snapshot);
        }
    }

    pub fn emit_agent_working_version_created(&self, event: &AgentWorkingVersionEvent) {
        let handle = self.app_handle.lock().unwrap().clone();
        if let Some(handle) = handle {
            let _ = handle.emit("agent-working-version-created", event);
        }
    }

    pub fn emit_agent_draft_preview_updated(
        &self,
        event: &crate::contracts::AgentDraftPreviewUpdatedEvent,
    ) {
        let handle = self.app_handle.lock().unwrap().clone();
        if let Some(handle) = handle {
            let _ = handle.emit("agent-draft-preview-updated", event);
        }
    }

    pub fn emit_history_updated(&self) {
        let handle = self.app_handle.lock().unwrap().clone();
        if let Some(handle) = handle {
            let _ = handle.emit("history-updated", ());
        }
    }

    /// Close a single pending prompt: SIGCONT any frozen process, send Err to unblock the handler,
    /// clear waiting_on_prompt, and emit agent-prompt-closed to the frontend.
    pub async fn close_single_prompt(
        &self,
        request_id: &str,
        session_id: &str,
        thread_id: Option<String>,
        reason: &str,
    ) {
        let pgid = {
            let mut waits = self.prompt_waits.lock().unwrap();
            waits.remove(request_id).and_then(|ctrl| ctrl.pgid)
        };
        #[cfg(unix)]
        if let Some(pgid) = pgid {
            unsafe {
                libc::kill(-pgid, libc::SIGCONT);
            }
        }
        {
            let mut channels = self.prompt_channels.lock().await;
            if let Some(tx) = channels.remove(request_id) {
                let _ = tx.send(Err(reason.to_string()));
            }
        }
        {
            let mut sessions = self.mcp_sessions.lock().await;
            if let Some(session) = sessions.get_mut(session_id) {
                session.waiting_on_prompt = false;
            }
        }
        let handle = self.app_handle.lock().unwrap().clone();
        if let Some(handle) = handle {
            let _ = handle.emit(
                "agent-prompt-closed",
                serde_json::json!({
                    "requestId": request_id,
                    "sessionId": session_id,
                    "threadId": thread_id,
                    "reason": reason,
                }),
            );
        }
    }

    /// Close all pending prompts for a session (e.g. on disconnect or logout).
    pub async fn close_prompts_for_session(&self, session_id: &str, reason: &str) {
        let targets: Vec<(String, Option<String>)> = {
            let waits = self.prompt_waits.lock().unwrap();
            waits
                .iter()
                .filter(|(_, ctrl)| ctrl.session_id == session_id)
                .map(|(req_id, ctrl)| (req_id.clone(), ctrl.thread_id.clone()))
                .collect()
        };
        for (request_id, thread_id) in targets {
            self.close_single_prompt(&request_id, session_id, thread_id, reason)
                .await;
        }
    }

    /// Close all pending prompts for an agent label (e.g. when the agent process is stopped).
    pub async fn close_prompts_for_agent_label(&self, agent_label: &str, reason: &str) {
        let targets: Vec<(String, String, Option<String>)> = {
            let waits = self.prompt_waits.lock().unwrap();
            waits
                .iter()
                .filter(|(_, ctrl)| ctrl.agent_label == agent_label)
                .map(|(req_id, ctrl)| {
                    (
                        req_id.clone(),
                        ctrl.session_id.clone(),
                        ctrl.thread_id.clone(),
                    )
                })
                .collect()
        };
        for (request_id, session_id, thread_id) in targets {
            self.close_single_prompt(&request_id, &session_id, thread_id, reason)
                .await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context_envelope::{
        assemble_envelope, envelope_telemetry, section_id, ContextSection, EnvelopeStage,
        SectionPriority, Sensitivity, TelemetryUsage,
    };
    use crate::contracts::{
        Config, Engine, EngineKind, GeometryBackend, SourceLanguage, UsageSummary,
    };

    fn minimal_config() -> Config {
        Config {
            engines: vec![Engine {
                id: "test".to_string(),
                name: "Test".to_string(),
                provider: "gemini".to_string(),
                api_key: String::new(),
                model: "gemini-2.5-flash".to_string(),
                light_model: "gemini-2.5-flash-lite".to_string(),
                base_url: String::new(),
                enabled: true,
                vision_overrides: std::collections::HashMap::new(),
            }],
            selected_engine_id: "test".to_string(),
            freecad_cmd: String::new(),
            cad_text_font_path: String::new(),
            freecad_library_roots: Vec::new(),
            assets: vec![],
            microwave: None,
            voice: crate::contracts::VoiceConfig::default(),
            mcp: crate::contracts::McpConfig::default(),
            has_seen_onboarding: false,
            connection_type: None,
            default_engine_kind: EngineKind::Freecad,
            default_source_language: SourceLanguage::LegacyPython,
            default_geometry_backend: GeometryBackend::Freecad,
            max_generation_attempts: 3,
            max_verify_attempts: 2,
            projects_root: None,
        }
    }

    fn state() -> AppState {
        let conn = rusqlite::Connection::open_in_memory().expect("in-memory db");
        AppState::new(minimal_config(), None, conn)
    }

    /// 4.2: `record_context_telemetry` emits the envelope shape plus provider
    /// cache/input/output usage through the existing profiler/session-activity
    /// path (the `app_logs` ring buffer surfaced by `get_app_logs`). The
    /// recorded line is content-free: even though the budgeted sections carry
    /// secret source/prompt/API-key/path content, none of it reaches the log.
    #[test]
    fn record_context_telemetry_emits_content_free_shape_line() {
        let state = state();
        let request = ContextSection::new(
            section_id::REQUEST,
            SectionPriority::Mandatory,
            Sensitivity::Sensitive,
            "PROMPT-SECRET-DO-NOT-LEAK",
        );
        let source = ContextSection::new(
            section_id::CURRENT_SOURCE,
            SectionPriority::Authoritative,
            Sensitivity::Sensitive,
            "SOURCE-SECRET sk-leak-AAAAAAAA Bearer zzz /Users/secret/x.ecky",
        );
        let envelope = assemble_envelope(EnvelopeStage::Generation, vec![request, source])
            .expect("envelope assembles");
        let usage = UsageSummary {
            input_tokens: 100,
            output_tokens: 40,
            total_tokens: 140,
            cached_input_tokens: 60,
            reasoning_tokens: 0,
            estimated_cost_usd: None,
            segments: Vec::new(),
        };

        state.record_context_telemetry(&envelope, Some(&usage));

        let logs = state.app_logs.lock().unwrap();
        assert_eq!(logs.len(), 1, "exactly one profiler line recorded");
        let line = &logs[0].message;
        assert!(line.starts_with("[CONTEXT] "), "tagged as a context line");
        let json = line.trim_start_matches("[CONTEXT] ");
        let parsed: serde_json::Value =
            serde_json::from_str(json).expect("recorded payload is valid JSON");

        // Shape + provider usage are present.
        assert_eq!(parsed["stage"], "generation");
        assert_eq!(parsed["ceilingChars"], 64_000);
        assert_eq!(parsed["usage"]["inputTokens"], 100);
        assert_eq!(parsed["usage"]["cachedInputTokens"], 60);
        assert_eq!(parsed["usage"]["outputTokens"], 40);
        assert!(parsed["sections"].is_array());
        // Inclusion decisions are carried.
        let decisions: Vec<&str> = parsed["sections"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s["decision"].as_str().unwrap())
            .collect();
        assert!(decisions.contains(&"included"));

        // No sensitive content leaks into the profiler line.
        for needle in [
            "PROMPT-SECRET-DO-NOT-LEAK",
            "SOURCE-SECRET",
            "sk-leak-AAAAAAAA",
            "Bearer zzz",
            "/Users/secret/x.ecky",
        ] {
            assert!(!line.contains(needle), "profiler line leaked: {needle}");
        }
    }

    /// 4.2: recording without provider usage still emits a valid, content-free
    /// shape line (the `usage` field is omitted entirely).
    #[test]
    fn record_context_telemetry_without_usage_omits_usage_field() {
        let state = state();
        let request = ContextSection::new(
            section_id::REQUEST,
            SectionPriority::Mandatory,
            Sensitivity::Sensitive,
            "NEVER-LEAK-THIS-PROMPT",
        );
        let envelope = assemble_envelope(EnvelopeStage::Classifier, vec![request]).unwrap();
        state.record_context_telemetry(&envelope, None);
        let line = &state.app_logs.lock().unwrap()[0].message;
        let json = line.trim_start_matches("[CONTEXT] ");
        let parsed: serde_json::Value = serde_json::from_str(json).unwrap();
        assert!(parsed.get("usage").is_none() || parsed["usage"].is_null());
        assert!(!line.contains("NEVER-LEAK-THIS-PROMPT"));
    }

    /// Guards the derivation contract used by the emission path: the recorded
    /// JSON carries exactly the shape fields (stage, ceiling, totals, sections,
    /// usage) and deserializes as a well-formed object.
    #[test]
    fn recorded_telemetry_is_a_well_formed_shape_object() {
        let envelope = assemble_envelope(
            EnvelopeStage::Generation,
            vec![ContextSection::new(
                section_id::REQUEST,
                SectionPriority::Mandatory,
                Sensitivity::Sensitive,
                "secret",
            )],
        )
        .unwrap();
        let telemetry = envelope_telemetry(
            &envelope,
            Some(TelemetryUsage {
                input_tokens: 1,
                output_tokens: 2,
                total_tokens: 3,
                cached_input_tokens: 0,
                reasoning_tokens: 0,
            }),
        );
        let parsed: serde_json::Value =
            serde_json::from_str(&serde_json::to_string(&telemetry).unwrap()).unwrap();
        assert_eq!(parsed["stage"], "generation");
        assert_eq!(parsed["ceilingChars"], 64_000);
        assert_eq!(parsed["sections"][0]["id"], "request");
        assert_eq!(parsed["sections"][0]["decision"], "included");
        assert_eq!(parsed["usage"]["inputTokens"], 1);
        // camelCase boundary, no snake_case leaks.
        let json = parsed.to_string();
        assert!(!json.contains("observed_chars"));
        assert!(!json.contains("input_tokens"));
    }
}
