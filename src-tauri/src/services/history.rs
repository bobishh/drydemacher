use crate::db;
use crate::models::{AppError, AppResult, MessageRole, MessageStatus, Thread};
use crate::persist_thread_summary;

pub fn get_history(conn: &rusqlite::Connection) -> AppResult<Vec<Thread>> {
    db::get_all_threads(conn).map_err(|err: rusqlite::Error| AppError::persistence(err.to_string()))
}

pub fn get_thread(conn: &rusqlite::Connection, id: &str) -> AppResult<Thread> {
    let title = db::get_thread_title(conn, id)
        .map_err(|err| AppError::persistence(err.to_string()))?
        .ok_or_else(|| AppError::not_found("Thread not found."))?;
    let summary = db::get_thread_summary(conn, id)
        .map_err(|err| AppError::persistence(err.to_string()))?
        .unwrap_or_default();
    let messages =
        db::get_thread_messages(conn, id).map_err(|err| AppError::persistence(err.to_string()))?;

    let genie_traits = db::get_thread_genie_traits(conn, id)
        .map_err(|err| AppError::persistence(err.to_string()))?;

    let updated_at = messages.last().map(|m| m.timestamp).unwrap_or(0);
    let version_count = messages
        .iter()
        .filter(|m| {
            m.role == MessageRole::Assistant && (m.output.is_some() || m.artifact_bundle.is_some())
        })
        .count();
    let pending_count = messages
        .iter()
        .filter(|m| m.role == MessageRole::Assistant && m.status == MessageStatus::Pending)
        .count();
    let error_count = messages
        .iter()
        .filter(|m| m.role == MessageRole::Assistant && m.status == MessageStatus::Error)
        .count();

    Ok(Thread {
        id: id.to_string(),
        title,
        summary,
        messages,
        updated_at,
        genie_traits,
        version_count,
        pending_count,
        error_count,
    })
}

pub fn delete_version(conn: &rusqlite::Connection, message_id: &str) -> AppResult<()> {
    let thread_id = db::delete_version_cluster(conn, message_id)
        .map_err(|err: rusqlite::Error| AppError::persistence(err.to_string()))?;

    if let Some(thread_id) = thread_id {
        let title = db::get_thread_title(conn, &thread_id)
            .map_err(|err| AppError::persistence(err.to_string()))?
            .unwrap_or_default();
        if db::has_visible_messages(conn, &thread_id)
            .map_err(|err| AppError::persistence(err.to_string()))?
        {
            let _ = persist_thread_summary(conn, &thread_id, &title);
        } else {
            db::update_thread_summary(conn, &thread_id, "")
                .map_err(|err| AppError::persistence(err.to_string()))?;
        }
    }

    Ok(())
}

pub fn restore_version(conn: &rusqlite::Connection, message_id: &str) -> AppResult<()> {
    let thread_id = db::restore_version_cluster(conn, message_id)
        .map_err(|err: rusqlite::Error| AppError::persistence(err.to_string()))?;

    if let Some(thread_id) = thread_id {
        let title = db::get_thread_title(conn, &thread_id)
            .map_err(|err| AppError::persistence(err.to_string()))?
            .unwrap_or_default();
        if db::has_visible_messages(conn, &thread_id)
            .map_err(|err| AppError::persistence(err.to_string()))?
        {
            let _ = persist_thread_summary(conn, &thread_id, &title);
        }
    }

    Ok(())
}
