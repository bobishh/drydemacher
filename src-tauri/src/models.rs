use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::Manager;
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
    /// Pending user-prompt requests keyed by requestId (agent waits for free-text from UI).
    pub prompt_channels: Arc<tokio::sync::Mutex<HashMap<String, oneshot::Sender<String>>>>,
    /// Active-mode auto-agent process group IDs keyed by agent label.
    /// Used to SIGSTOP the agent while it waits for user input.
    pub auto_agent_pids: Arc<Mutex<HashMap<String, i32>>>,
    /// Maps prompt request_id → pgid for agents that were SIGSTOP'd.
    /// Cleared and SIGCONT'd when the user resolves the prompt.
    pub prompt_pgids: Arc<Mutex<HashMap<String, i32>>>,
    /// Per-agent wake notifiers. Fired by `wake_auto_agent` to unblock
    /// the supervisor loop waiting to respawn a dead agent.
    pub agent_wake: Arc<Mutex<HashMap<String, Arc<tokio::sync::Notify>>>>,
    /// Ring buffer of in-app log entries (latest 200 entries).
    pub app_logs: Arc<Mutex<VecDeque<AppLogEntry>>>,
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
            auto_agent_pids: Arc::new(Mutex::new(HashMap::new())),
            prompt_pgids: Arc::new(Mutex::new(HashMap::new())),
            agent_wake: Arc::new(Mutex::new(HashMap::new())),
            app_logs: Arc::new(Mutex::new(VecDeque::new())),
        }
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
}
