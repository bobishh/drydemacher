use crate::contracts::WebContentRecoveryState;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RecoveryAction {
    Reload,
    Block,
}

#[derive(Debug, Default)]
struct RecoveryGuard {
    termination_count: u32,
    automatic_reload_used: bool,
    blocked: bool,
    raw_error: Option<String>,
    occurred_at: Option<u64>,
}

impl RecoveryGuard {
    fn terminate(&mut self, raw_error: String) -> RecoveryAction {
        self.termination_count = self.termination_count.saturating_add(1);
        self.raw_error = Some(raw_error);
        self.occurred_at = Some(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        );
        if self.automatic_reload_used {
            self.blocked = true;
            RecoveryAction::Block
        } else {
            self.automatic_reload_used = true;
            RecoveryAction::Reload
        }
    }

    fn snapshot(&self) -> WebContentRecoveryState {
        WebContentRecoveryState {
            termination_count: self.termination_count,
            automatic_reload_used: self.automatic_reload_used,
            blocked: self.blocked,
            raw_error: self.raw_error.clone(),
            occurred_at: self.occurred_at,
        }
    }

    fn acknowledge_stable(&mut self) {
        *self = Self::default();
    }
}

static RECOVERY_GUARD: OnceLock<Mutex<RecoveryGuard>> = OnceLock::new();

fn guard() -> &'static Mutex<RecoveryGuard> {
    RECOVERY_GUARD.get_or_init(|| Mutex::new(RecoveryGuard::default()))
}

pub fn state() -> WebContentRecoveryState {
    guard().lock().unwrap().snapshot()
}

pub fn acknowledge_stable() {
    guard().lock().unwrap().acknowledge_stable();
}

fn record_termination(raw_error: String) -> (RecoveryAction, WebContentRecoveryState) {
    let mut state = guard().lock().unwrap();
    let action = state.terminate(raw_error);
    (action, state.snapshot())
}

#[cfg(debug_assertions)]
#[doc(hidden)]
pub fn simulate_termination_for_integration_test(
    raw_error: impl Into<String>,
) -> WebContentRecoveryState {
    record_termination(raw_error.into()).1
}

#[cfg(target_os = "macos")]
mod macos {
    use super::{record_termination, RecoveryAction};
    use objc::runtime::{
        class_getInstanceMethod, method_setImplementation, object_getClass, Imp, Method, Object,
        Sel,
    };
    use objc::{msg_send, sel, sel_impl};
    use std::sync::OnceLock;
    use tauri::{AppHandle, Emitter, Manager};

    static APP_HANDLE: OnceLock<AppHandle> = OnceLock::new();
    static ORIGINAL_IMPLEMENTATION: OnceLock<usize> = OnceLock::new();

    unsafe extern "C" fn web_content_process_did_terminate(
        receiver: *mut Object,
        selector: Sel,
        webview: *mut Object,
    ) {
        if let Some(address) = ORIGINAL_IMPLEMENTATION.get().copied() {
            let original: unsafe extern "C" fn(*mut Object, Sel, *mut Object) =
                std::mem::transmute(address);
            original(receiver, selector, webview);
        }

        let raw_error = "WKWebView web content process terminated".to_string();
        let (action, snapshot) = record_termination(raw_error.clone());
        if let Some(app) = APP_HANDLE.get() {
            let _ = app.emit("web-content-terminated", &snapshot);
            if let Some(state) = app.try_state::<crate::models::AppState>() {
                state.push_log(format!("[WEB_CONTENT] {raw_error}"));
            }
        }
        match action {
            RecoveryAction::Reload => {
                let _: *mut Object = msg_send![webview, reload];
            }
            RecoveryAction::Block => {
                let _ = rfd::MessageDialog::new()
                    .set_level(rfd::MessageLevel::Error)
                    .set_title("Ecky WebContent recovery stopped")
                    .set_description(format!(
                        "{raw_error}. Automatic reload already ran once. Reopen the window after reducing the active history/render load."
                    ))
                    .show();
            }
        }
    }

    pub fn install(app: &AppHandle) -> Result<(), String> {
        let _ = APP_HANDLE.set(app.clone());
        let window = app
            .get_webview_window("main")
            .ok_or_else(|| "main webview window not found".to_string())?;
        window
            .with_webview(|platform| unsafe {
                let webview = platform.inner().cast::<Object>();
                let delegate: *mut Object = msg_send![webview, navigationDelegate];
                if delegate.is_null() || ORIGINAL_IMPLEMENTATION.get().is_some() {
                    return;
                }
                let class = object_getClass(delegate);
                let method =
                    class_getInstanceMethod(class, sel!(webViewWebContentProcessDidTerminate:))
                        as *mut Method;
                if method.is_null() {
                    return;
                }
                let replacement: Imp = std::mem::transmute(
                    web_content_process_did_terminate
                        as unsafe extern "C" fn(*mut Object, Sel, *mut Object),
                );
                let original = method_setImplementation(method, replacement);
                let _ = ORIGINAL_IMPLEMENTATION.set(original as usize);
            })
            .map_err(|error| error.to_string())
    }
}

#[cfg(target_os = "macos")]
pub use macos::install;

#[cfg(not(target_os = "macos"))]
pub fn install(_app: &tauri::AppHandle) -> Result<(), String> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crash_loop_guard_reloads_once_then_blocks_with_raw_reason() {
        let mut guard = RecoveryGuard::default();
        assert_eq!(
            guard.terminate("first raw reason".to_string()),
            RecoveryAction::Reload
        );
        assert_eq!(
            guard.terminate("second raw reason".to_string()),
            RecoveryAction::Block
        );
        let state = guard.snapshot();
        assert_eq!(state.termination_count, 2);
        assert!(state.blocked);
        assert_eq!(state.raw_error.as_deref(), Some("second raw reason"));
        guard.acknowledge_stable();
        assert_eq!(guard.terminate("later".to_string()), RecoveryAction::Reload);
    }
}
