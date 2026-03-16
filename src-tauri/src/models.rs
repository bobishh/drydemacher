use std::collections::{HashMap, VecDeque};
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use tauri::{Emitter, Manager};
use tokio::sync::oneshot;

pub use crate::contracts::*;

pub trait PathResolver: Send + Sync {
    fn app_config_dir(&self) -> PathBuf;
    fn app_data_dir(&self) -> PathBuf;
    fn resource_path(&self, path: &str) -> Option<PathBuf>;
}

impl PathResolver for tauri::AppHandle {
    fn app_config_dir(&self) -> PathBuf {
        self.path().app_config_dir().unwrap()
    }
    fn app_data_dir(&self) -> PathBuf {
        self.path().app_data_dir().unwrap()
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
    pub last_target: Option<McpTargetRef>,
}

impl McpSessionState {
    pub fn new(client_kind: String, host_label: String) -> Self {
        Self {
            client_kind,
            agent_label: host_label.clone(),
            host_label,
            llm_model_id: None,
            llm_model_label: None,
            last_target: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PromptResumeState {
    pub pgid: Option<i32>,
    pub agent_label: String,
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
pub type PendingAgentSessionTraces = Arc<Mutex<HashMap<String, VecDeque<AgentSessionTraceEntry>>>>;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Mutex<Config>>,
    pub last_snapshot: Arc<Mutex<Option<LastDesignSnapshot>>>,
    pub db: Arc<tokio::sync::Mutex<rusqlite::Connection>>,
    pub render_lock: Arc<tokio::sync::Mutex<()>>,
    pub mcp_status: Arc<Mutex<McpServerStatus>>,
    pub mcp_sessions: Arc<tokio::sync::Mutex<HashMap<String, McpSessionState>>>,
    /// Pending user-confirmation requests keyed by requestId.
    pub confirm_channels: Arc<tokio::sync::Mutex<HashMap<String, oneshot::Sender<String>>>>,
    /// Pending user-prompt requests keyed by requestId (agent waits for text/attachments from UI).
    pub prompt_channels:
        Arc<tokio::sync::Mutex<HashMap<String, oneshot::Sender<ResolveAgentPromptInput>>>>,
    /// Runtime state machine for active-mode MCP agents.
    pub auto_agent_runtime: Arc<Mutex<crate::mcp::runtime::AutoAgentRuntimeRegistry>>,
    /// Maps prompt request_id → process control for agents SIGSTOP'd while waiting on the user.
    pub prompt_waits: Arc<Mutex<HashMap<String, PromptResumeState>>>,
    /// Pending viewport screenshot requests keyed by requestId.
    pub viewport_screenshot_channels: PendingViewportScreenshotChannels,
    /// Ring buffer of in-app log entries (latest 200 entries).
    pub app_logs: Arc<Mutex<VecDeque<AppLogEntry>>>,
    /// Structured per-session trace buffers for active MCP agents.
    pub agent_session_traces: PendingAgentSessionTraces,
    /// Active PTY-backed terminal bridges for interactive auto-agents.
    pub agent_terminals: PendingAgentTerminalSessions,
    /// App handle for emitting runtime PTY events back into the frontend.
    pub app_handle: Arc<Mutex<Option<tauri::AppHandle>>>,
}

impl AppState {
    pub fn new(
        config: Config,
        last_snapshot: Option<LastDesignSnapshot>,
        conn: rusqlite::Connection,
    ) -> Self {
        Self {
            config: Arc::new(Mutex::new(config)),
            last_snapshot: Arc::new(Mutex::new(last_snapshot)),
            db: Arc::new(tokio::sync::Mutex::new(conn)),
            render_lock: Arc::new(tokio::sync::Mutex::new(())),
            mcp_status: Arc::new(Mutex::new(McpServerStatus {
                running: false,
                endpoint_url: "http://127.0.0.1:39249/mcp".to_string(),
                last_startup_error: None,
            })),
            mcp_sessions: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
            confirm_channels: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
            prompt_channels: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
            auto_agent_runtime: Arc::new(Mutex::new(
                crate::mcp::runtime::AutoAgentRuntimeRegistry::default(),
            )),
            prompt_waits: Arc::new(Mutex::new(HashMap::new())),
            viewport_screenshot_channels: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
            app_logs: Arc::new(Mutex::new(VecDeque::new())),
            agent_session_traces: Arc::new(Mutex::new(HashMap::new())),
            agent_terminals: Arc::new(Mutex::new(HashMap::new())),
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

    pub fn push_agent_session_trace(&self, entry: AgentSessionTraceEntry) {
        {
            let mut traces = self.agent_session_traces.lock().unwrap();
            let buffer = traces.entry(entry.session_id.clone()).or_default();
            if buffer.len() >= 200 {
                buffer.pop_front();
            }
            buffer.push_back(entry.clone());
        }

        let handle = self.app_handle.lock().unwrap().clone();
        if let Some(handle) = handle {
            let _ = handle.emit("agent-session-trace-updated", entry);
        }
    }

    pub fn agent_session_trace(&self, session_id: &str) -> Vec<AgentSessionTraceEntry> {
        self.agent_session_traces
            .lock()
            .unwrap()
            .get(session_id)
            .map(|entries| entries.iter().cloned().collect())
            .unwrap_or_default()
    }

    pub fn latest_agent_session_trace_summary(&self, session_id: &str) -> Option<String> {
        self.agent_session_traces
            .lock()
            .unwrap()
            .get(session_id)
            .and_then(|entries| entries.back())
            .map(|entry| entry.summary.clone())
    }

    pub fn emit_history_updated(&self) {
        let handle = self.app_handle.lock().unwrap().clone();
        if let Some(handle) = handle {
            let _ = handle.emit("history-updated", ());
        }
    }
}
