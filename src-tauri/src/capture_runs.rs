use crate::contracts::{
    AppError, AppResult, CaptureCropBounds, CaptureReconstructionGuide,
    CaptureReconstructionGuideState, CaptureRun, CaptureSessionInfo, CaptureSessionState,
};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;

pub fn ensure_schema(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS capture_runs (
            id TEXT PRIMARY KEY,
            target_thread_id TEXT NOT NULL,
            target_message_id TEXT,
            title TEXT NOT NULL,
            state TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL,
            accepted_frame_count INTEGER NOT NULL DEFAULT 0,
            mesh_preview_json TEXT,
            derived_stl_path TEXT,
            crop_bounds_json TEXT,
            preview_scale REAL NOT NULL DEFAULT 0.05,
            target_source TEXT NOT NULL DEFAULT '',
            target_source_language TEXT NOT NULL DEFAULT 'ecky',
            started_from_empty INTEGER NOT NULL DEFAULT 0,
            raw_error TEXT,
            guide_json TEXT,
            guide_revision INTEGER,
            guide_mesh_digest TEXT,
            guide_state TEXT,
            guide_stale_reason TEXT,
            guided_request_id TEXT,
            guided_request_state TEXT,
            guided_request_error TEXT
        );
        CREATE INDEX IF NOT EXISTS idx_capture_runs_thread_updated
            ON capture_runs(target_thread_id, updated_at DESC);
        CREATE TABLE IF NOT EXISTS capture_guide_versions (
            run_id TEXT NOT NULL,
            revision INTEGER NOT NULL,
            base_revision INTEGER NOT NULL,
            client_expected_revision INTEGER NOT NULL,
            client_expected_mesh_digest TEXT NOT NULL,
            current_mesh_digest TEXT NOT NULL,
            guide_json TEXT NOT NULL,
            guide_state TEXT NOT NULL,
            raw_evidence_json TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            PRIMARY KEY(run_id, revision),
            FOREIGN KEY(run_id) REFERENCES capture_runs(id) ON DELETE CASCADE
        );
        CREATE INDEX IF NOT EXISTS idx_capture_guide_versions_run_revision
            ON capture_guide_versions(run_id, revision DESC);",
    )?;
    ensure_column(conn, "guide_json", "TEXT")?;
    ensure_column(conn, "guide_revision", "INTEGER")?;
    ensure_column(conn, "guide_mesh_digest", "TEXT")?;
    ensure_column(conn, "guide_state", "TEXT")?;
    ensure_column(conn, "guide_stale_reason", "TEXT")?;
    ensure_column(conn, "guided_request_id", "TEXT")?;
    ensure_column(conn, "guided_request_state", "TEXT")?;
    ensure_column(conn, "guided_request_error", "TEXT")?;
    ensure_column(conn, "guided_result_json", "TEXT")?;
    ensure_column(conn, "guided_deviation_json", "TEXT")?;
    ensure_column(conn, "guided_message_id", "TEXT")?;
    ensure_column(conn, "guided_model_id", "TEXT")?;
    Ok(())
}

pub fn append_capture_guide_version(
    conn: &Connection,
    run_id: &str,
    base_revision: u64,
    client_expected_revision: u64,
    client_expected_mesh_digest: &str,
    current_mesh_digest: &str,
    guide: &CaptureReconstructionGuide,
    state: &CaptureReconstructionGuideState,
    raw_evidence: &[String],
) -> AppResult<()> {
    let guide_json = serde_json::to_string(guide).map_err(|error| {
        AppError::persistence(format!("Capture guide encoding failed: {error}"))
    })?;
    let raw_evidence_json = serde_json::to_string(raw_evidence).map_err(|error| {
        AppError::persistence(format!("Capture guide evidence encoding failed: {error}"))
    })?;
    let transaction = conn
        .unchecked_transaction()
        .map_err(|error| AppError::persistence(error.to_string()))?;
    transaction
        .execute(
            "INSERT INTO capture_guide_versions (
                run_id, revision, base_revision, client_expected_revision,
                client_expected_mesh_digest, current_mesh_digest, guide_json,
                guide_state, raw_evidence_json, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                run_id,
                guide.revision as i64,
                base_revision as i64,
                client_expected_revision as i64,
                client_expected_mesh_digest,
                current_mesh_digest,
                guide_json,
                guide_state_text(state),
                raw_evidence_json,
                now_secs() as i64,
            ],
        )
        .map_err(|error| AppError::persistence(error.to_string()))?;
    transaction
        .execute(
            "UPDATE capture_runs SET guide_json=?2, guide_revision=?3, guide_mesh_digest=?4,
             guide_state=?5, guide_stale_reason=?6, updated_at=?7 WHERE id=?1",
            params![
                run_id,
                guide_json,
                guide.revision as i64,
                current_mesh_digest,
                guide_state_text(state),
                guide_stale_reason(Some(state)),
                now_secs() as i64,
            ],
        )
        .map_err(|error| AppError::persistence(error.to_string()))?;
    transaction
        .commit()
        .map_err(|error| AppError::persistence(error.to_string()))
}

pub fn capture_guide_version_count(conn: &Connection, run_id: &str) -> AppResult<u64> {
    conn.query_row(
        "SELECT COUNT(*) FROM capture_guide_versions WHERE run_id=?1",
        [run_id],
        |row| row.get::<_, i64>(0),
    )
    .map(|count| count as u64)
    .map_err(|error| AppError::persistence(error.to_string()))
}

fn ensure_column(conn: &Connection, name: &str, declaration: &str) -> rusqlite::Result<()> {
    let mut statement = conn.prepare("PRAGMA table_info(capture_runs)")?;
    let names = statement
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    if !names.iter().any(|existing| existing == name) {
        conn.execute(
            &format!("ALTER TABLE capture_runs ADD COLUMN {name} {declaration}"),
            [],
        )?;
    }
    Ok(())
}

pub fn insert(conn: &Connection, run: &CaptureRun) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO capture_runs (
            id, target_thread_id, target_message_id, title, state, created_at, updated_at,
            accepted_frame_count, mesh_preview_json, derived_stl_path, crop_bounds_json,
            preview_scale, target_source, target_source_language, started_from_empty, raw_error,
            guide_json, guide_revision, guide_mesh_digest, guide_state, guide_stale_reason
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)
        ON CONFLICT(id) DO UPDATE SET
            target_thread_id=excluded.target_thread_id,
            target_message_id=excluded.target_message_id,
            title=excluded.title,
            state=excluded.state,
            updated_at=excluded.updated_at,
            accepted_frame_count=excluded.accepted_frame_count,
            mesh_preview_json=excluded.mesh_preview_json,
            derived_stl_path=excluded.derived_stl_path,
            crop_bounds_json=excluded.crop_bounds_json,
            preview_scale=excluded.preview_scale,
            target_source=excluded.target_source,
            target_source_language=excluded.target_source_language,
            started_from_empty=excluded.started_from_empty,
            raw_error=excluded.raw_error,
            guide_json=COALESCE(excluded.guide_json, capture_runs.guide_json),
            guide_revision=COALESCE(excluded.guide_revision, capture_runs.guide_revision),
            guide_mesh_digest=COALESCE(excluded.guide_mesh_digest, capture_runs.guide_mesh_digest),
            guide_state=COALESCE(excluded.guide_state, capture_runs.guide_state),
            guide_stale_reason=COALESCE(excluded.guide_stale_reason, capture_runs.guide_stale_reason)",
        params![
            run.id,
            run.target_thread_id,
            run.target_message_id,
            run.title,
            state_text(&run.state),
            run.created_at as i64,
            run.updated_at as i64,
            run.accepted_frame_count as i64,
            to_json(run.mesh_preview.as_ref())?,
            run.derived_stl_path,
            to_json(run.crop_bounds.as_ref())?,
            run.preview_scale,
            run.target_source,
            run.target_source_language,
            i64::from(run.started_from_empty),
            run.raw_error,
            to_json(run.reconstruction_guide.as_ref())?,
            run.reconstruction_guide.as_ref().map(|guide| guide.revision as i64),
            run.reconstruction_guide.as_ref().map(|guide| guide.source_mesh.content_digest.as_str()),
            run.reconstruction_guide_state.as_ref().map(guide_state_text),
            guide_stale_reason(run.reconstruction_guide_state.as_ref()),
        ],
    )?;
    Ok(())
}

pub fn update_from_session(
    conn: &Connection,
    session: &CaptureSessionInfo,
) -> rusqlite::Result<()> {
    let updated_at = now_secs();
    let mesh_preview_json = to_json(session.mesh_preview.as_ref())?;
    let changed = conn.execute(
        "UPDATE capture_runs SET
            state=?2, updated_at=?3, accepted_frame_count=?4,
            guide_state=CASE
                WHEN guide_json IS NOT NULL AND ?5 IS NOT NULL
                 AND COALESCE(mesh_preview_json, '') != COALESCE(?5, '') THEN 'stale'
                ELSE guide_state END,
            guide_stale_reason=CASE
                WHEN guide_json IS NOT NULL AND ?5 IS NOT NULL
                 AND COALESCE(mesh_preview_json, '') != COALESCE(?5, '')
                THEN 'Guide is stale: selected crop/source mesh digest changed.'
                ELSE guide_stale_reason END,
            mesh_preview_json=COALESCE(?5, mesh_preview_json), raw_error=?6
         WHERE id=?1",
        params![
            session.session_id,
            state_text(&session.state),
            updated_at as i64,
            session.accepted_frame_count as i64,
            mesh_preview_json,
            session.raw_error,
        ],
    )?;
    if changed > 0 {
        conn.execute(
            "UPDATE threads SET updated_at=?2 WHERE id=(SELECT target_thread_id FROM capture_runs WHERE id=?1)",
            params![session.session_id, updated_at as i64],
        )?;
    }
    Ok(())
}

pub fn update_preview_settings(
    conn: &Connection,
    run_id: &str,
    derived_stl_path: Option<&str>,
    crop_bounds: Option<CaptureCropBounds>,
    preview_scale: f64,
) -> rusqlite::Result<()> {
    let updated_at = now_secs();
    let crop_bounds_json = to_json(crop_bounds.as_ref())?;
    conn.execute(
        "UPDATE capture_runs SET
         guide_state=CASE
            WHEN guide_json IS NOT NULL AND (
                COALESCE(derived_stl_path, '') != COALESCE(?2, '') OR
                COALESCE(crop_bounds_json, '') != COALESCE(?3, '')
            ) THEN 'stale'
            ELSE guide_state END,
         guide_stale_reason=CASE
            WHEN guide_json IS NOT NULL AND (
                COALESCE(derived_stl_path, '') != COALESCE(?2, '') OR
                COALESCE(crop_bounds_json, '') != COALESCE(?3, '')
            ) THEN 'Guide is stale: selected crop/source mesh digest changed.'
            ELSE guide_stale_reason END,
         derived_stl_path=?2, crop_bounds_json=?3,
         preview_scale=?4, updated_at=?5 WHERE id=?1",
        params![
            run_id,
            derived_stl_path,
            crop_bounds_json,
            preview_scale,
            updated_at as i64,
        ],
    )?;
    conn.execute(
        "UPDATE threads SET updated_at=?2 WHERE id=(SELECT target_thread_id FROM capture_runs WHERE id=?1)",
        params![run_id, updated_at as i64],
    )?;
    Ok(())
}

pub fn get(conn: &Connection, id: &str) -> rusqlite::Result<Option<CaptureRun>> {
    conn.query_row(
        "SELECT id, target_thread_id, target_message_id, title, state, created_at,
         updated_at, accepted_frame_count, mesh_preview_json, derived_stl_path,
         crop_bounds_json, preview_scale, target_source, target_source_language,
         started_from_empty, raw_error, guide_json, guide_revision, guide_mesh_digest,
         guide_state, guide_stale_reason, guided_message_id, guided_model_id,
         guided_result_json, guided_deviation_json FROM capture_runs WHERE id=?1",
        [id],
        capture_run_from_row,
    )
    .optional()
}

pub fn list_for_thread(conn: &Connection, thread_id: &str) -> rusqlite::Result<Vec<CaptureRun>> {
    let mut statement = conn.prepare(
        "SELECT id, target_thread_id, target_message_id, title, state, created_at,
         updated_at, accepted_frame_count, mesh_preview_json, derived_stl_path,
         crop_bounds_json, preview_scale, target_source, target_source_language,
         started_from_empty, raw_error, guide_json, guide_revision, guide_mesh_digest,
         guide_state, guide_stale_reason, guided_message_id, guided_model_id,
         guided_result_json, guided_deviation_json FROM capture_runs
         WHERE target_thread_id=?1 ORDER BY updated_at DESC, id DESC",
    )?;
    let rows = statement.query_map([thread_id], capture_run_from_row)?;
    rows.collect()
}

pub fn contains(conn: &Connection, id: &str) -> rusqlite::Result<bool> {
    Ok(conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM capture_runs WHERE id=?1)",
        [id],
        |row| row.get::<_, i64>(0),
    )? != 0)
}

pub fn selected_source_identity(
    conn: &Connection,
    run_id: &str,
) -> AppResult<crate::contracts::CaptureGuideSourceMesh> {
    let (path, selection) = selected_source_path(conn, run_id)?;
    crate::capture_guidance::inspect_capture_source_mesh(&path, selection)
}

pub fn selected_source_path(
    conn: &Connection,
    run_id: &str,
) -> AppResult<(std::path::PathBuf, crate::contracts::CaptureMeshSelection)> {
    let run = get(conn, run_id)
        .map_err(|error| AppError::persistence(error.to_string()))?
        .ok_or_else(|| AppError::not_found("Capture run not found."))?;
    let (path, selection) = if let Some(path) = run.derived_stl_path.as_deref() {
        (
            std::path::PathBuf::from(path),
            crate::contracts::CaptureMeshSelection::Crop,
        )
    } else {
        (
            std::path::PathBuf::from(
                run.mesh_preview
                    .as_ref()
                    .map(|preview| preview.stl_path.as_str())
                    .ok_or_else(|| AppError::not_found("Capture run has no source mesh."))?,
            ),
            crate::contracts::CaptureMeshSelection::Raw,
        )
    };
    Ok((path, selection))
}

pub fn guide_context(
    conn: &Connection,
    run_id: &str,
) -> AppResult<crate::contracts::CaptureGuideContext> {
    let run = get(conn, run_id)
        .map_err(|error| AppError::persistence(error.to_string()))?
        .ok_or_else(|| AppError::not_found("Capture run not found."))?;
    let source_mesh = selected_source_identity(conn, run_id)?;
    Ok(crate::contracts::CaptureGuideContext {
        source_mesh,
        target_source_digest: crate::services::render_snapshot::canonical_source_digest(
            &run.target_source,
        ),
        target_version_id: run.target_message_id,
    })
}

fn capture_run_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<CaptureRun> {
    let state: String = row.get(4)?;
    Ok(CaptureRun {
        id: row.get(0)?,
        target_thread_id: row.get(1)?,
        target_message_id: row.get(2)?,
        title: row.get(3)?,
        state: parse_state(&state)?,
        created_at: row.get::<_, i64>(5)? as u64,
        updated_at: row.get::<_, i64>(6)? as u64,
        accepted_frame_count: row.get::<_, i64>(7)? as u32,
        mesh_preview: from_json(row.get(8)?)?,
        derived_stl_path: row.get(9)?,
        crop_bounds: from_json(row.get(10)?)?,
        preview_scale: row.get(11)?,
        target_source: row.get(12)?,
        target_source_language: row.get(13)?,
        started_from_empty: row.get::<_, i64>(14)? != 0,
        raw_error: row.get(15)?,
        reconstruction_guide: from_json(row.get(16)?)?,
        reconstruction_guide_state: parse_guide_state(row.get(19)?, row.get(20)?)?,
        guided_reconstruction_message_id: row.get(21)?,
        guided_reconstruction_model_id: row.get(22)?,
        guided_reconstruction_result: from_json(row.get(23)?)?,
        guided_reconstruction_deviation: from_json(row.get(24)?)?,
    })
}

pub fn save_reconstruction_guide(
    conn: &Connection,
    run_id: &str,
    expected_revision: u64,
    expected_mesh_digest: &str,
    mut guide: CaptureReconstructionGuide,
    state: CaptureReconstructionGuideState,
) -> AppResult<CaptureReconstructionGuide> {
    let run = get(conn, run_id)
        .map_err(|error| AppError::persistence(error.to_string()))?
        .ok_or_else(|| AppError::not_found("Capture run not found."))?;
    let current_revision = run
        .reconstruction_guide
        .as_ref()
        .map(|current| current.revision)
        .unwrap_or(0);
    if expected_revision != current_revision {
        return Err(AppError::conflict(format!(
            "Capture guide revision conflict: expected {expected_revision}, current {current_revision}."
        )));
    }
    if guide.capture_run_id != run.id
        || guide.target_thread_id != run.target_thread_id
        || guide.target_message_id != run.target_message_id
    {
        return Err(AppError::conflict(
            "Capture guide ownership differs from capture run.",
        ));
    }
    let run_target_source_digest =
        crate::services::render_snapshot::canonical_source_digest(&run.target_source);
    if guide.target_source_digest != run_target_source_digest
        || guide.target_version_id != run.target_message_id
    {
        return Err(AppError::conflict(
            "Capture guide target source/version differs from capture run.",
        ));
    }
    if guide.source_mesh.content_digest != expected_mesh_digest {
        return Err(AppError::conflict(
            "Capture guide source mesh digest differs from save guard.",
        ));
    }
    let selected_path = run
        .derived_stl_path
        .as_deref()
        .or_else(|| {
            run.mesh_preview
                .as_ref()
                .map(|preview| preview.stl_path.as_str())
        })
        .ok_or_else(|| AppError::not_found("Capture run has no selected source mesh."))?;
    let actual_mesh_digest =
        crate::capture_guidance::source_mesh_content_digest(Path::new(selected_path))?;
    if actual_mesh_digest != expected_mesh_digest {
        return Err(AppError::conflict(
            "Guide is stale: selected crop/source mesh digest changed.",
        ));
    }
    guide.revision = current_revision + 1;
    match &state {
        CaptureReconstructionGuideState::Ready => {
            crate::capture_guidance::recompute_guide_geometry_from_stl(
                Path::new(selected_path),
                &mut guide,
            )?;
            crate::capture_guidance::validate_computed_reconstruction_evidence(&guide)?;
        }
        CaptureReconstructionGuideState::Draft => {
            crate::capture_guidance::validate_guide_draft_from_stl(
                Path::new(selected_path),
                &mut guide,
            )?;
        }
        CaptureReconstructionGuideState::Stale { .. } => {
            return Err(AppError::validation(
                "A stale guide state is backend-owned and cannot be saved by caller.",
            ));
        }
    }
    guide.canonical_digest = guide
        .compute_canonical_digest()
        .map_err(AppError::validation)?;
    let guide_json = serde_json::to_string(&guide).map_err(|error| {
        AppError::persistence(format!("Capture guide encoding failed: {error}"))
    })?;
    let transaction = conn
        .unchecked_transaction()
        .map_err(|error| AppError::persistence(error.to_string()))?;
    let changed = transaction
        .execute(
            "UPDATE capture_runs SET guide_json=?2, guide_revision=?3, guide_mesh_digest=?4,
             guide_state=?5, guide_stale_reason=?6, updated_at=?7
             WHERE id=?1 AND COALESCE(guide_revision, 0)=?8",
            params![
                run_id,
                guide_json,
                guide.revision as i64,
                expected_mesh_digest,
                guide_state_text(&state),
                guide_stale_reason(Some(&state)),
                now_secs() as i64,
                expected_revision as i64,
            ],
        )
        .map_err(|error| AppError::persistence(error.to_string()))?;
    if changed != 1 {
        return Err(AppError::conflict(
            "Capture guide changed concurrently during save.",
        ));
    }
    transaction
        .commit()
        .map_err(|error| AppError::persistence(error.to_string()))?;
    Ok(guide)
}

pub fn evaluate_reconstruction_guide(
    conn: &Connection,
    run_id: &str,
    expected_mesh_digest: &str,
    mut guide: CaptureReconstructionGuide,
) -> AppResult<CaptureReconstructionGuide> {
    let run = get(conn, run_id)
        .map_err(|error| AppError::persistence(error.to_string()))?
        .ok_or_else(|| AppError::not_found("Capture run not found."))?;
    if guide.capture_run_id != run.id
        || guide.target_thread_id != run.target_thread_id
        || guide.target_message_id != run.target_message_id
    {
        return Err(AppError::conflict(
            "Capture guide ownership differs from capture run.",
        ));
    }
    let target_digest =
        crate::services::render_snapshot::canonical_source_digest(&run.target_source);
    if guide.target_source_digest != target_digest
        || guide.target_version_id != run.target_message_id
    {
        return Err(AppError::conflict(
            "Capture guide target source/version differs from capture run.",
        ));
    }
    if guide.source_mesh.content_digest != expected_mesh_digest {
        return Err(AppError::conflict(
            "Capture guide source mesh digest differs from evaluation guard.",
        ));
    }
    let selected_path = run
        .derived_stl_path
        .as_deref()
        .or_else(|| {
            run.mesh_preview
                .as_ref()
                .map(|preview| preview.stl_path.as_str())
        })
        .ok_or_else(|| AppError::not_found("Capture run has no selected source mesh."))?;
    let actual_mesh_digest =
        crate::capture_guidance::source_mesh_content_digest(Path::new(selected_path))?;
    if actual_mesh_digest != expected_mesh_digest {
        return Err(AppError::conflict(
            "Guide is stale: selected crop/source mesh digest changed.",
        ));
    }
    guide.revision = run
        .reconstruction_guide
        .as_ref()
        .map(|current| current.revision)
        .unwrap_or(0)
        + 1;
    crate::capture_guidance::recompute_guide_geometry_from_stl(
        Path::new(selected_path),
        &mut guide,
    )?;
    guide.canonical_digest = guide
        .compute_canonical_digest()
        .map_err(AppError::validation)?;
    Ok(guide)
}

#[derive(Debug, Clone)]
pub struct PendingGuidedReconstruction {
    pub run_id: String,
    pub request_id: String,
    pub guide: CaptureReconstructionGuide,
}

pub fn mark_guided_reconstruction_pending(
    conn: &Connection,
    run_id: &str,
    request_id: &str,
) -> AppResult<()> {
    if request_id.trim().is_empty() {
        return Err(AppError::validation(
            "Guided reconstruction request ID must not be empty.",
        ));
    }
    let run = get(conn, run_id)
        .map_err(|error| AppError::persistence(error.to_string()))?
        .ok_or_else(|| AppError::not_found("Capture run not found."))?;
    if !matches!(
        run.reconstruction_guide_state,
        Some(CaptureReconstructionGuideState::Ready)
    ) {
        return Err(AppError::conflict(
            "Guided reconstruction request requires a ready capture guide.",
        ));
    }
    let transaction = conn
        .unchecked_transaction()
        .map_err(|error| AppError::persistence(error.to_string()))?;
    transaction
        .execute(
            "UPDATE capture_runs SET guided_request_state='superseded'
             WHERE target_thread_id=?1 AND guided_request_state='pending' AND id<>?2",
            params![run.target_thread_id, run_id],
        )
        .map_err(|error| AppError::persistence(error.to_string()))?;
    transaction
        .execute(
            "UPDATE capture_runs SET guided_request_id=?2, guided_request_state='pending',
             guided_request_error=NULL, guided_result_json=NULL, guided_deviation_json=NULL,
             guided_message_id=NULL, guided_model_id=NULL,
             updated_at=?3 WHERE id=?1",
            params![run_id, request_id, now_secs() as i64],
        )
        .map_err(|error| AppError::persistence(error.to_string()))?;
    transaction
        .commit()
        .map_err(|error| AppError::persistence(error.to_string()))
}

pub fn pending_guided_reconstruction_for_thread(
    conn: &Connection,
    thread_id: &str,
) -> AppResult<Option<PendingGuidedReconstruction>> {
    conn.query_row(
        "SELECT id, guided_request_id, guide_json FROM capture_runs
         WHERE target_thread_id=?1 AND guided_request_state='pending'
         ORDER BY updated_at DESC, id DESC LIMIT 1",
        [thread_id],
        |row| {
            let guide_json: String = row.get(2)?;
            let guide = serde_json::from_str(&guide_json).map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(
                    2,
                    rusqlite::types::Type::Text,
                    Box::new(error),
                )
            })?;
            Ok(PendingGuidedReconstruction {
                run_id: row.get(0)?,
                request_id: row.get(1)?,
                guide,
            })
        },
    )
    .optional()
    .map_err(|error| AppError::persistence(error.to_string()))
}

pub fn record_guided_reconstruction_validation_error(
    conn: &Connection,
    request_id: &str,
    error: &str,
) -> AppResult<()> {
    conn.execute(
        "UPDATE capture_runs SET guided_request_error=?2, guided_result_json=NULL,
         guided_deviation_json=NULL, updated_at=?3
         WHERE guided_request_id=?1 AND guided_request_state='pending'",
        params![request_id, error, now_secs() as i64],
    )
    .map_err(|error| AppError::persistence(error.to_string()))?;
    Ok(())
}

pub fn record_guided_reconstruction_validation_success(
    conn: &Connection,
    request_id: &str,
    provenance: &crate::contracts::CaptureGuideResultProvenance,
    deviation: &crate::contracts::CaptureObservedDeviationReport,
) -> AppResult<()> {
    let provenance_json = serde_json::to_string(provenance).map_err(|error| {
        AppError::persistence(format!(
            "Guided reconstruction provenance serialization failed: {error}"
        ))
    })?;
    let deviation_json = serde_json::to_string(deviation).map_err(|error| {
        AppError::persistence(format!(
            "Guided reconstruction deviation serialization failed: {error}"
        ))
    })?;
    let changed = conn
        .execute(
            "UPDATE capture_runs SET guided_result_json=?2, guided_deviation_json=?3,
             guided_request_error=NULL, updated_at=?4
             WHERE guided_request_id=?1 AND guided_request_state='pending'",
            params![
                request_id,
                provenance_json,
                deviation_json,
                now_secs() as i64
            ],
        )
        .map_err(|error| AppError::persistence(error.to_string()))?;
    if changed != 1 {
        return Err(AppError::conflict(
            "Guided reconstruction request is no longer pending.",
        ));
    }
    Ok(())
}

pub fn guided_reconstruction_deviation_result(
    conn: &Connection,
    request_id: &str,
) -> AppResult<Option<crate::contracts::CaptureObservedDeviationReport>> {
    let result_json = conn
        .query_row(
            "SELECT guided_deviation_json FROM capture_runs WHERE guided_request_id=?1",
            [request_id],
            |row| row.get::<_, Option<String>>(0),
        )
        .optional()
        .map_err(|error| AppError::persistence(error.to_string()))?
        .flatten();
    result_json
        .map(|json| {
            serde_json::from_str(&json).map_err(|error| {
                AppError::persistence(format!(
                    "Stored guided reconstruction deviation is invalid: {error}"
                ))
            })
        })
        .transpose()
}

pub fn guided_reconstruction_validation_result(
    conn: &Connection,
    request_id: &str,
) -> AppResult<Option<crate::contracts::CaptureGuideResultProvenance>> {
    let result_json = conn
        .query_row(
            "SELECT guided_result_json FROM capture_runs WHERE guided_request_id=?1",
            [request_id],
            |row| row.get::<_, Option<String>>(0),
        )
        .optional()
        .map_err(|error| AppError::persistence(error.to_string()))?
        .flatten();
    result_json
        .map(|json| {
            serde_json::from_str(&json).map_err(|error| {
                AppError::persistence(format!(
                    "Stored guided reconstruction provenance is invalid: {error}"
                ))
            })
        })
        .transpose()
}

pub fn complete_guided_reconstruction(
    conn: &Connection,
    request_id: &str,
    message_id: &str,
    model_id: Option<&str>,
) -> AppResult<()> {
    if message_id.trim().is_empty() {
        return Err(AppError::validation(
            "Guided reconstruction committed message ID must not be empty.",
        ));
    }
    let changed = conn
        .execute(
            "UPDATE capture_runs SET guided_request_state='completed',
             guided_request_error=NULL, guided_message_id=?2, guided_model_id=?3, updated_at=?4
             WHERE guided_request_id=?1 AND guided_request_state='pending'",
            params![request_id, message_id, model_id, now_secs() as i64],
        )
        .map_err(|error| AppError::persistence(error.to_string()))?;
    if changed != 1 {
        return Err(AppError::conflict(
            "Guided reconstruction request is no longer pending.",
        ));
    }
    Ok(())
}

fn state_text(state: &CaptureSessionState) -> &'static str {
    match state {
        CaptureSessionState::Pairing => "pairing",
        CaptureSessionState::Capturing => "capturing",
        CaptureSessionState::Reconstructing => "reconstructing",
        CaptureSessionState::Preview => "preview",
        CaptureSessionState::Failed => "failed",
        CaptureSessionState::Cancelled => "cancelled",
    }
}

fn parse_state(value: &str) -> rusqlite::Result<CaptureSessionState> {
    match value {
        "pairing" => Ok(CaptureSessionState::Pairing),
        "capturing" => Ok(CaptureSessionState::Capturing),
        "reconstructing" => Ok(CaptureSessionState::Reconstructing),
        "preview" => Ok(CaptureSessionState::Preview),
        "failed" => Ok(CaptureSessionState::Failed),
        "cancelled" => Ok(CaptureSessionState::Cancelled),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}

fn guide_state_text(state: &CaptureReconstructionGuideState) -> &'static str {
    match state {
        CaptureReconstructionGuideState::Draft => "draft",
        CaptureReconstructionGuideState::Ready => "ready",
        CaptureReconstructionGuideState::Stale { .. } => "stale",
    }
}

fn guide_stale_reason(state: Option<&CaptureReconstructionGuideState>) -> Option<&str> {
    match state {
        Some(CaptureReconstructionGuideState::Stale { reason }) => Some(reason.as_str()),
        _ => None,
    }
}

fn parse_guide_state(
    state: Option<String>,
    stale_reason: Option<String>,
) -> rusqlite::Result<Option<CaptureReconstructionGuideState>> {
    match state.as_deref() {
        None => Ok(None),
        Some("draft") => Ok(Some(CaptureReconstructionGuideState::Draft)),
        Some("ready") => Ok(Some(CaptureReconstructionGuideState::Ready)),
        Some("stale") => Ok(Some(CaptureReconstructionGuideState::Stale {
            reason: stale_reason.unwrap_or_else(|| {
                "Guide is stale: selected crop/source mesh digest changed.".into()
            }),
        })),
        Some(_) => Err(rusqlite::Error::InvalidQuery),
    }
}

fn to_json<T: serde::Serialize>(value: Option<&T>) -> rusqlite::Result<Option<String>> {
    value
        .map(serde_json::to_string)
        .transpose()
        .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))
}

fn from_json<T: serde::de::DeserializeOwned>(value: Option<String>) -> rusqlite::Result<Option<T>> {
    value
        .map(|json| serde_json::from_str(&json))
        .transpose()
        .map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                0,
                rusqlite::types::Type::Text,
                Box::new(error),
            )
        })
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::{
        CaptureCropBounds, CaptureMeshPreview, CaptureRun, CaptureSessionState,
    };

    #[test]
    fn capture_run_survives_connection_reopen_with_preview_settings() {
        let root = std::env::temp_dir().join(format!("ecky-capture-run-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("history.sqlite");
        {
            let conn = crate::db::init_db(&path).unwrap();
            let run = CaptureRun {
                id: "capture-1".into(),
                target_thread_id: "thread-1".into(),
                target_message_id: None,
                title: "Finger scan".into(),
                state: CaptureSessionState::Preview,
                created_at: 10,
                updated_at: 20,
                accepted_frame_count: 48,
                mesh_preview: Some(CaptureMeshPreview {
                    stl_path: "/tmp/raw.stl".into(),
                    triangle_count: 100,
                    bounds_mm: [10.0, 20.0, 30.0],
                    scale_label: "restored".into(),
                    warnings: vec![],
                }),
                derived_stl_path: Some("/tmp/cropped.stl".into()),
                crop_bounds: Some(CaptureCropBounds {
                    min: [1.0, 2.0, 3.0],
                    max: [4.0, 5.0, 6.0],
                }),
                preview_scale: 0.05,
                target_source: "(solid blank)".into(),
                target_source_language: "ecky".into(),
                started_from_empty: false,
                raw_error: None,
                reconstruction_guide: None,
                reconstruction_guide_state: None,
                guided_reconstruction_message_id: None,
                guided_reconstruction_model_id: None,
                guided_reconstruction_result: None,
                guided_reconstruction_deviation: None,
            };
            insert(&conn, &run).unwrap();
        }

        let conn = crate::db::init_db(&path).unwrap();
        let restored = get(&conn, "capture-1").unwrap().unwrap();
        assert_eq!(restored.id, "capture-1");
        assert_eq!(restored.crop_bounds.unwrap().max, [4.0, 5.0, 6.0]);
        assert_eq!(restored.preview_scale, 0.05);
        assert_eq!(list_for_thread(&conn, "thread-1").unwrap(), vec![restored]);
        drop(conn);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn guide_save_is_revision_and_mesh_guarded_then_crop_marks_it_stale() {
        let root =
            std::env::temp_dir().join(format!("ecky-capture-guide-run-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let db_path = root.join("history.sqlite");
        let stl_path = root.join("source.stl");
        std::fs::write(
            &stl_path,
            "solid source\nfacet normal 0 0 1\nouter loop\nvertex 0 0 0\nvertex 1 0 0\nvertex 0 1 0\nendloop\nendfacet\nendsolid source\n",
        )
        .unwrap();
        let mesh_digest = crate::capture_guidance::source_mesh_content_digest(&stl_path).unwrap();
        let conn = crate::db::init_db(&db_path).unwrap();
        let run = CaptureRun {
            id: "run-guide".into(),
            target_thread_id: "thread-guide".into(),
            target_message_id: Some("message-guide".into()),
            title: "Guide".into(),
            state: CaptureSessionState::Preview,
            created_at: 1,
            updated_at: 1,
            accepted_frame_count: 3,
            mesh_preview: Some(CaptureMeshPreview {
                stl_path: stl_path.to_string_lossy().into_owned(),
                triangle_count: 1,
                bounds_mm: [1.0, 1.0, 0.0],
                scale_label: "source".into(),
                warnings: vec![],
            }),
            derived_stl_path: None,
            crop_bounds: None,
            preview_scale: 1.0,
            target_source: "(solid blank)".into(),
            target_source_language: "ecky".into(),
            started_from_empty: false,
            raw_error: None,
            reconstruction_guide: None,
            reconstruction_guide_state: None,
            guided_reconstruction_message_id: None,
            guided_reconstruction_model_id: None,
            guided_reconstruction_result: None,
            guided_reconstruction_deviation: None,
        };
        insert(&conn, &run).unwrap();
        let mut guide = crate::contracts::CaptureReconstructionGuide::test_fixture();
        guide.capture_run_id = run.id.clone();
        guide.target_thread_id = run.target_thread_id.clone();
        guide.target_message_id = run.target_message_id.clone();
        guide.target_source_digest =
            crate::services::render_snapshot::canonical_source_digest(&run.target_source);
        guide.target_version_id = run.target_message_id.clone();
        guide.source_mesh.content_digest = mesh_digest.clone();
        guide.source_mesh.triangle_count = 1;
        for landmark in &mut guide.landmarks {
            landmark.anchor.source_mesh_content_digest = mesh_digest.clone();
            landmark.anchor.triangle_index = 0;
        }
        guide.landmarks[0].anchor.barycentric = [1.0, 0.0, 0.0];
        guide.landmarks[1].anchor.barycentric = [0.0, 1.0, 0.0];
        guide.landmarks[2].anchor.barycentric = [0.0, 0.0, 1.0];
        guide.calibration.measurements = vec![crate::contracts::CaptureKnownDistanceMeasurement {
            measurement_id: "calibration-1".into(),
            label: "known edge".into(),
            first_landmark_id: "landmark-1".into(),
            second_landmark_id: "landmark-2".into(),
            known_distance_mm: 1.0,
            fitted_distance_mm: 0.0,
            residual_mm: 0.0,
            accepted_tolerance_mm: 0.01,
        }];
        guide.measurements = vec![crate::contracts::CaptureNamedMeasurement {
            measurement_id: "depth".into(),
            label: "extrusion depth".into(),
            landmark_ids: vec!["landmark-1".into(), "landmark-2".into()],
            value: 1.0,
            unit: "mm".into(),
            fit_critical: true,
            authored_parameter_name: Some("insert-depth".into()),
            constraint_kind: Some(crate::contracts::CaptureConstraintKind::Extent),
        }];

        let evaluated =
            evaluate_reconstruction_guide(&conn, &run.id, &mesh_digest, guide.clone()).unwrap();
        assert!(evaluated.reconstruction_readiness.ready);
        assert_eq!(evaluated.revision, 1);
        assert!(get(&conn, &run.id)
            .unwrap()
            .unwrap()
            .reconstruction_guide
            .is_none());

        let saved = save_reconstruction_guide(
            &conn,
            &run.id,
            0,
            &mesh_digest,
            guide,
            crate::contracts::CaptureReconstructionGuideState::Ready,
        )
        .unwrap();
        assert_eq!(saved.revision, 1);
        assert!(saved.canonical_digest.starts_with("sha256:"));
        let restored = get(&conn, &run.id).unwrap().unwrap();
        assert_eq!(restored.reconstruction_guide, Some(saved.clone()));
        assert_eq!(
            restored.reconstruction_guide_state,
            Some(crate::contracts::CaptureReconstructionGuideState::Ready)
        );

        let conflict = save_reconstruction_guide(
            &conn,
            &run.id,
            0,
            &mesh_digest,
            saved.clone(),
            crate::contracts::CaptureReconstructionGuideState::Ready,
        )
        .unwrap_err();
        assert_eq!(
            conflict.message,
            "Capture guide revision conflict: expected 0, current 1."
        );
        assert_eq!(
            get(&conn, &run.id).unwrap().unwrap().reconstruction_guide,
            Some(saved.clone())
        );

        mark_guided_reconstruction_pending(&conn, &run.id, "request-1").unwrap();
        let pending = pending_guided_reconstruction_for_thread(&conn, &run.target_thread_id)
            .unwrap()
            .unwrap();
        assert_eq!(pending.run_id, run.id);
        assert_eq!(pending.request_id, "request-1");
        record_guided_reconstruction_validation_error(&conn, "request-1", "wrong kind").unwrap();
        let provenance = crate::contracts::CaptureGuideResultProvenance {
            guide_id: saved.guide_id.clone(),
            guide_revision: saved.revision,
            guide_canonical_digest: saved.canonical_digest.clone(),
            source_mesh_artifact_digest: saved.source_mesh.artifact_digest.clone(),
            source_mesh_content_digest: saved.source_mesh.content_digest.clone(),
            target_source_digest: saved.target_source_digest.clone(),
            target_version_id: saved.target_version_id.clone(),
            generated_source_digest: "sha256:generated".into(),
            geometry_digest: "sha256:geometry".into(),
            assumptions: Vec::new(),
            inferred_regions: Vec::new(),
            selected_feature_plan_id: None,
            feature_operation_traces: Vec::new(),
            correspondences: Vec::new(),
        };
        let deviation = crate::contracts::CaptureObservedDeviationReport {
            schema_version: 1,
            guide_id: saved.guide_id.clone(),
            guide_revision: saved.revision,
            guide_canonical_digest: saved.canonical_digest.clone(),
            source_mesh_content_digest: saved.source_mesh.content_digest.clone(),
            generated_geometry_digest: "sha256:geometry".into(),
            parts: vec![crate::contracts::CaptureDeviationPartIdentity {
                part_id: "part-1".into(),
                source_geometry_digest: "sha256:part-geometry".into(),
                analysis_boundary_digest: "sha256:boundary".into(),
            }],
            source_vertex_count: 3,
            sample_count: 3,
            maximum_mm: 0.1,
            rms_mm: 0.05,
            percentile_95_mm: 0.1,
            outlier_threshold_mm: 0.2,
            outlier_count: 0,
            evidence_scope: "observedRegionOnly".into(),
            display_samples: vec![],
            content_digest: "sha256:deviation".into(),
        };
        record_guided_reconstruction_validation_success(
            &conn,
            "request-1",
            &provenance,
            &deviation,
        )
        .unwrap();
        assert_eq!(
            guided_reconstruction_validation_result(&conn, "request-1").unwrap(),
            Some(provenance.clone())
        );
        assert_eq!(
            guided_reconstruction_deviation_result(&conn, "request-1").unwrap(),
            Some(deviation.clone())
        );
        record_guided_reconstruction_validation_error(&conn, "request-1", "new red result")
            .unwrap();
        assert_eq!(
            guided_reconstruction_validation_result(&conn, "request-1").unwrap(),
            None
        );
        assert_eq!(
            guided_reconstruction_deviation_result(&conn, "request-1").unwrap(),
            None
        );
        record_guided_reconstruction_validation_success(
            &conn,
            "request-1",
            &provenance,
            &deviation,
        )
        .unwrap();
        complete_guided_reconstruction(
            &conn,
            "request-1",
            "generated-message-1",
            Some("generated-model-1"),
        )
        .unwrap();
        assert!(
            pending_guided_reconstruction_for_thread(&conn, &run.target_thread_id)
                .unwrap()
                .is_none()
        );
        let completed = get(&conn, &run.id).unwrap().unwrap();
        assert_eq!(
            completed.guided_reconstruction_message_id.as_deref(),
            Some("generated-message-1")
        );
        assert_eq!(
            completed.guided_reconstruction_model_id.as_deref(),
            Some("generated-model-1")
        );
        assert_eq!(completed.guided_reconstruction_result, Some(provenance));
        assert_eq!(completed.guided_reconstruction_deviation, Some(deviation));

        update_preview_settings(
            &conn,
            &run.id,
            Some(stl_path.to_string_lossy().as_ref()),
            Some(CaptureCropBounds {
                min: [0.0, 0.0, 0.0],
                max: [0.5, 1.0, 0.0],
            }),
            1.0,
        )
        .unwrap();
        assert_eq!(
            get(&conn, &run.id)
                .unwrap()
                .unwrap()
                .reconstruction_guide_state,
            Some(crate::contracts::CaptureReconstructionGuideState::Stale {
                reason: "Guide is stale: selected crop/source mesh digest changed.".into(),
            })
        );
        drop(conn);
        std::fs::remove_dir_all(root).unwrap();
    }
}
