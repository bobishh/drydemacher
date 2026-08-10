use std::panic::{catch_unwind, AssertUnwindSafe};

use tauri::Manager;

use crate::contracts::{AppError, AppResult};

fn recover_dialog_creation<T>(create: impl FnOnce() -> T) -> AppResult<T> {
    catch_unwind(AssertUnwindSafe(create)).map_err(|payload| {
        let detail = payload
            .downcast_ref::<&str>()
            .copied()
            .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
            .unwrap_or("native dialog creation panicked");
        AppError::internal(format!(
            "macOS save panel service failed without closing Ecky: {detail}"
        ))
    })
}

#[tauri::command]
#[specta::specta]
pub async fn safe_save_dialog(
    default_path: String,
    filter_name: String,
    extensions: Vec<String>,
    app: tauri::AppHandle,
) -> AppResult<Option<String>> {
    if default_path.trim().is_empty()
        || filter_name.trim().is_empty()
        || extensions.is_empty()
        || extensions.iter().any(|extension| {
            extension.is_empty() || !extension.bytes().all(|byte| byte.is_ascii_alphanumeric())
        })
    {
        return Err(AppError::validation(
            "Save dialog requires a default path, filter name, and alphanumeric extensions.",
        ));
    }

    let (sender, receiver) = tokio::sync::oneshot::channel();
    let dialog_app = app.clone();
    app.run_on_main_thread(move || {
        let selected = recover_dialog_creation(|| {
            let mut dialog = rfd::FileDialog::new()
                .set_file_name(&default_path)
                .add_filter(&filter_name, &extensions);
            if let Some(parent) = dialog_app.get_webview_window("main") {
                dialog = dialog.set_parent(&parent);
            }
            dialog.save_file()
        })
        .map(|path| path.map(|path| path.to_string_lossy().into_owned()));
        let _ = sender.send(selected);
    })
    .map_err(|error| AppError::internal(format!("Save dialog scheduling failed: {error}")))?;

    receiver
        .await
        .map_err(|_| AppError::internal("Save dialog service ended before returning a result."))?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_dialog_panic_becomes_backend_error() {
        let error = recover_dialog_creation(|| panic!("unexpected NULL returned from NSSavePanel"))
            .expect_err("native panic must not escape command boundary");
        assert!(error.message.contains("without closing Ecky"));
        assert!(error.message.contains("unexpected NULL"));
    }
}
