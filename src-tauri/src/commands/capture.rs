use tauri::{AppHandle, State};

use crate::contracts::{
    AppError, AppResult, CaptureCropBounds, CaptureFrameManifest, CaptureMeshPreview,
    CaptureReconstructionGuide, CaptureReconstructionGuideState, CaptureRun, CaptureSessionState,
    ReopenedCaptureRun,
};
use crate::models::{AppState, PathResolver};

const CAPTURE_SESSION_TTL_SECS: u64 = 60 * 60;

#[tauri::command]
#[specta::specta]
pub async fn start_capture_session(
    thread_id: String,
    message_id: Option<String>,
    title: String,
    target_source: String,
    target_source_language: String,
    started_from_empty: bool,
    state: State<'_, AppState>,
) -> AppResult<crate::contracts::CaptureSessionInfo> {
    let session = state
        .start_capture_session(CAPTURE_SESSION_TTL_SECS, thread_id, message_id)
        .await?;
    let run = CaptureRun {
        id: session.session_id.clone(),
        target_thread_id: session.target_thread_id.clone(),
        target_message_id: session.target_message_id.clone(),
        title: title.clone(),
        state: session.state.clone(),
        created_at: session.created_at,
        updated_at: session.created_at,
        accepted_frame_count: 0,
        mesh_preview: None,
        derived_stl_path: None,
        crop_bounds: None,
        preview_scale: 0.05,
        target_source,
        target_source_language,
        started_from_empty,
        raw_error: None,
        reconstruction_guide: None,
        reconstruction_guide_state: None,
        guided_reconstruction_message_id: None,
        guided_reconstruction_model_id: None,
        guided_reconstruction_result: None,
        guided_reconstruction_deviation: None,
    };
    let db = state.db.lock().await;
    crate::db::create_or_update_thread(&db, &run.target_thread_id, &title, run.created_at, None)
        .map_err(|error| AppError::persistence(error.to_string()))?;
    crate::capture_runs::insert(&db, &run)
        .map_err(|error| AppError::persistence(error.to_string()))?;
    Ok(session)
}

#[tauri::command]
#[specta::specta]
pub async fn list_capture_runs(
    thread_id: String,
    state: State<'_, AppState>,
) -> AppResult<Vec<CaptureRun>> {
    let db = state.db.lock().await;
    crate::capture_runs::list_for_thread(&db, &thread_id)
        .map_err(|error| AppError::persistence(error.to_string()))
}

#[tauri::command]
#[specta::specta]
pub async fn reopen_capture_run(
    run_id: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> AppResult<ReopenedCaptureRun> {
    let run = {
        let db = state.db.lock().await;
        crate::capture_runs::get(&db, &run_id)
            .map_err(|error| AppError::persistence(error.to_string()))?
            .ok_or_else(|| AppError::not_found("Capture run not found."))?
    };
    reopen_run(&state, &app, run).await
}

#[tauri::command]
#[specta::specta]
pub async fn adopt_latest_capture_run(
    thread_id: String,
    message_id: Option<String>,
    title: String,
    target_source: String,
    target_source_language: String,
    started_from_empty: bool,
    state: State<'_, AppState>,
    app: AppHandle,
) -> AppResult<ReopenedCaptureRun> {
    let captures_root = app.app_data_dir().join("captures");
    let raw_stl = latest_legacy_capture_stl(&captures_root)?
        .ok_or_else(|| AppError::not_found("No previous capture STL exists."))?;
    let run_id = raw_stl
        .parent()
        .and_then(std::path::Path::parent)
        .and_then(std::path::Path::file_name)
        .and_then(std::ffi::OsStr::to_str)
        .ok_or_else(|| AppError::validation("Previous capture path has no run identity."))?
        .to_string();
    let existing = {
        let db = state.db.lock().await;
        crate::capture_runs::get(&db, &run_id)
            .map_err(|error| AppError::persistence(error.to_string()))?
    };
    let run = if let Some(existing) = existing {
        existing
    } else {
        let preview = inspect_capture_stl(&raw_stl)?;
        let updated_at = raw_stl
            .metadata()
            .and_then(|metadata| metadata.modified())
            .ok()
            .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|duration| duration.as_secs())
            .unwrap_or_else(now_secs);
        let run = CaptureRun {
            id: run_id,
            target_thread_id: thread_id,
            target_message_id: message_id,
            title: title.clone(),
            state: CaptureSessionState::Preview,
            created_at: updated_at,
            updated_at,
            accepted_frame_count: read_capture_manifest(&captures_root, raw_stl_run_id(&raw_stl)?)
                .map(|manifest| manifest.frames.len() as u32)
                .unwrap_or(0),
            mesh_preview: Some(preview),
            derived_stl_path: None,
            crop_bounds: None,
            preview_scale: 0.05,
            target_source,
            target_source_language,
            started_from_empty,
            raw_error: None,
            reconstruction_guide: None,
            reconstruction_guide_state: None,
            guided_reconstruction_message_id: None,
            guided_reconstruction_model_id: None,
            guided_reconstruction_result: None,
            guided_reconstruction_deviation: None,
        };
        let db = state.db.lock().await;
        crate::db::create_or_update_thread(&db, &run.target_thread_id, &title, updated_at, None)
            .map_err(|error| AppError::persistence(error.to_string()))?;
        crate::capture_runs::insert(&db, &run)
            .map_err(|error| AppError::persistence(error.to_string()))?;
        run
    };
    reopen_run(&state, &app, run).await
}

#[tauri::command]
#[specta::specta]
pub async fn save_capture_preview_settings(
    run_id: String,
    crop_bounds: Option<CaptureCropBounds>,
    preview_scale: f64,
    state: State<'_, AppState>,
) -> AppResult<()> {
    if !preview_scale.is_finite() || preview_scale <= 0.0 {
        return Err(AppError::validation(
            "Capture preview scale must be positive and finite.",
        ));
    }
    let db = state.db.lock().await;
    let run = crate::capture_runs::get(&db, &run_id)
        .map_err(|error| AppError::persistence(error.to_string()))?
        .ok_or_else(|| AppError::not_found("Capture run not found."))?;
    crate::capture_runs::update_preview_settings(
        &db,
        &run_id,
        run.derived_stl_path.as_deref(),
        crop_bounds,
        preview_scale,
    )
    .map_err(|error| AppError::persistence(error.to_string()))
}

#[tauri::command]
#[specta::specta]
pub async fn get_capture_reconstruction_guide(
    run_id: String,
    state: State<'_, AppState>,
) -> AppResult<Option<CaptureReconstructionGuide>> {
    let db = state.db.lock().await;
    Ok(crate::capture_runs::get(&db, &run_id)
        .map_err(|error| AppError::persistence(error.to_string()))?
        .and_then(|run| run.reconstruction_guide))
}

#[tauri::command]
#[specta::specta]
pub async fn get_capture_guide_source_identity(
    run_id: String,
    state: State<'_, AppState>,
) -> AppResult<crate::contracts::CaptureGuideSourceMesh> {
    let db = state.db.lock().await;
    crate::capture_runs::selected_source_identity(&db, &run_id)
}

#[tauri::command]
#[specta::specta]
pub async fn get_capture_guide_context(
    run_id: String,
    state: State<'_, AppState>,
) -> AppResult<crate::contracts::CaptureGuideContext> {
    let db = state.db.lock().await;
    crate::capture_runs::guide_context(&db, &run_id)
}

#[tauri::command]
#[specta::specta]
pub async fn save_capture_reconstruction_guide(
    run_id: String,
    expected_revision: u64,
    expected_mesh_digest: String,
    guide: CaptureReconstructionGuide,
    guide_state: CaptureReconstructionGuideState,
    state: State<'_, AppState>,
) -> AppResult<CaptureReconstructionGuide> {
    let db = state.db.lock().await;
    crate::capture_runs::save_reconstruction_guide(
        &db,
        &run_id,
        expected_revision,
        &expected_mesh_digest,
        guide,
        guide_state,
    )
}

#[tauri::command]
#[specta::specta]
pub async fn evaluate_capture_reconstruction_guide(
    run_id: String,
    expected_mesh_digest: String,
    guide: CaptureReconstructionGuide,
    state: State<'_, AppState>,
) -> AppResult<CaptureReconstructionGuide> {
    let db = state.db.lock().await;
    crate::capture_runs::evaluate_reconstruction_guide(&db, &run_id, &expected_mesh_digest, guide)
}

pub(crate) async fn queue_capture_guided_reconstruction_impl(
    run_id: &str,
    expected_guide_revision: u64,
    expected_target_source_digest: &str,
    state: &AppState,
) -> AppResult<crate::contracts::QueuedCaptureGuidedReconstruction> {
    let (guide, target_version_id) = {
        let db = state.db.lock().await;
        let run = crate::capture_runs::get(&db, run_id)
            .map_err(|error| AppError::persistence(error.to_string()))?
            .ok_or_else(|| AppError::not_found("Capture run not found."))?;
        let guide = run
            .reconstruction_guide
            .clone()
            .ok_or_else(|| AppError::validation("Capture run has no reconstruction guide."))?;
        match run.reconstruction_guide_state.as_ref() {
            Some(CaptureReconstructionGuideState::Ready) => {}
            Some(CaptureReconstructionGuideState::Stale { reason }) => {
                return Err(AppError::conflict(reason.clone()));
            }
            _ => {
                return Err(AppError::validation(
                    "Capture reconstruction guide is not ready.",
                ));
            }
        }
        if guide.revision != expected_guide_revision {
            return Err(AppError::conflict(format!(
                "Capture guide revision conflict: expected {expected_guide_revision}, current {}.",
                guide.revision
            )));
        }
        let capture_target_digest =
            crate::services::render_snapshot::canonical_source_digest(&run.target_source);
        if capture_target_digest != expected_target_source_digest {
            return Err(AppError::conflict(
                "Capture target source changed since guide creation.",
            ));
        }
        let (current_target_digest, target_version_id) =
            if let Some(message_id) = run.target_message_id.as_ref() {
                let (output, owner_thread_id) =
                    crate::db::get_message_output_and_thread(&db, message_id)
                        .map_err(|error| AppError::persistence(error.to_string()))?
                        .ok_or_else(|| {
                            AppError::not_found(format!(
                                "Capture target message {message_id} not found."
                            ))
                        })?;
                if owner_thread_id != run.target_thread_id {
                    return Err(AppError::conflict(
                        "Capture target message no longer belongs to owning thread.",
                    ));
                }
                (
                    crate::services::render_snapshot::canonical_source_digest(&output.macro_code),
                    Some(message_id.clone()),
                )
            } else if run.started_from_empty {
                (capture_target_digest.clone(), None)
            } else {
                return Err(AppError::validation(
                    "Capture run has no exact target message identity.",
                ));
            };
        if current_target_digest != capture_target_digest {
            return Err(AppError::conflict(
                "Capture target source changed since guide creation.",
            ));
        }
        (guide, target_version_id)
    };
    let request = crate::capture_guidance::build_guided_reconstruction_request(
        &guide,
        expected_target_source_digest,
        target_version_id,
    )?;
    let prompt = crate::capture_guidance::guided_reconstruction_prompt(&request)?;
    let queued = crate::commands::session::queue_agent_prompt_impl(
        crate::contracts::QueueAgentPromptInput {
            thread_id: Some(guide.target_thread_id.clone()),
            prompt_text: prompt,
            attachments: Vec::new(),
        },
        state,
    )
    .await?;
    if queued.thread_id != guide.target_thread_id {
        return Err(AppError::internal(
            "Guided reconstruction queued into wrong thread.",
        ));
    }
    {
        let db = state.db.lock().await;
        crate::capture_runs::mark_guided_reconstruction_pending(&db, run_id, &request.request_id)?;
    }
    Ok(crate::contracts::QueuedCaptureGuidedReconstruction {
        request_id: request.request_id,
        thread_id: queued.thread_id,
        message_id: queued.message_id,
    })
}

#[tauri::command]
#[specta::specta]
pub async fn queue_capture_guided_reconstruction(
    run_id: String,
    expected_guide_revision: u64,
    expected_target_source_digest: String,
    state: State<'_, AppState>,
) -> AppResult<crate::contracts::QueuedCaptureGuidedReconstruction> {
    queue_capture_guided_reconstruction_impl(
        &run_id,
        expected_guide_revision,
        &expected_target_source_digest,
        &state,
    )
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn get_capture_session_status(
    token: String,
    state: State<'_, AppState>,
) -> AppResult<Option<crate::contracts::CaptureSessionInfo>> {
    Ok(state.get_capture_session(&token).await)
}

#[tauri::command]
#[specta::specta]
pub async fn pair_capture_session(
    token: String,
    state: State<'_, AppState>,
) -> AppResult<crate::contracts::CaptureSessionInfo> {
    state
        .pair_capture_session(&token, 0, Default::default())
        .await
}

#[tauri::command]
#[specta::specta]
pub async fn cancel_capture_session(
    token: String,
    state: State<'_, AppState>,
) -> AppResult<crate::contracts::CaptureSessionInfo> {
    state.cancel_capture_session(&token).await
}

#[tauri::command]
#[specta::specta]
pub async fn resume_capture_session(
    token: String,
    state: State<'_, AppState>,
) -> AppResult<crate::contracts::CaptureSessionInfo> {
    state.resume_capture_session(&token).await
}

#[tauri::command]
#[specta::specta]
pub async fn retry_capture_reconstruction(
    token: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> AppResult<crate::contracts::CaptureSessionInfo> {
    crate::capture_server::begin_reconstruction(
        state.inner().clone(),
        &token,
        app.app_data_dir().join("captures"),
        app.app_data_dir().join("capture-tools"),
    )
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn prepare_capture_preview(
    token: String,
    crop_bounds: Option<crate::contracts::CaptureCropBounds>,
    state: State<'_, AppState>,
    app: AppHandle,
) -> AppResult<crate::contracts::CapturePreparedPreview> {
    let session = state.get_capture_session(&token).await.ok_or_else(|| {
        crate::contracts::AppError::not_found("Capture session not found or expired.")
    })?;
    if session.state != CaptureSessionState::Preview {
        return Err(crate::contracts::AppError::validation(
            "Capture mesh preview is not ready.",
        ));
    }
    let preview = session
        .mesh_preview
        .ok_or_else(|| crate::contracts::AppError::not_found("Capture mesh preview is missing."))?;
    let source_path = std::path::Path::new(&preview.stl_path);
    let selected_path = if let Some(bounds) = crop_bounds {
        let output_path = source_path.with_file_name("preview-box-crop.stl");
        crate::capture_mesh_crop::write_capture_box_crop(
            source_path,
            &output_path,
            crate::capture_mesh_crop::MeshCropBounds {
                min: bounds.min,
                max: bounds.max,
            },
        )?;
        output_path
    } else {
        source_path.to_path_buf()
    };
    let prepared = crate::freecad_library::import_generated_capture_mesh(
        &selected_path,
        &format!(
            "Capture {}",
            &session.session_id[..8.min(session.session_id.len())]
        ),
        &session.session_id,
        &app,
    )?;
    {
        let db = state.db.lock().await;
        if let Some(run) = crate::capture_runs::get(&db, &session.session_id)
            .map_err(|error| AppError::persistence(error.to_string()))?
        {
            crate::capture_runs::update_preview_settings(
                &db,
                &session.session_id,
                crop_bounds
                    .map(|_| selected_path.to_string_lossy().into_owned())
                    .as_deref(),
                crop_bounds,
                run.preview_scale,
            )
            .map_err(|error| AppError::persistence(error.to_string()))?;
        }
    }
    Ok(prepared)
}

async fn reopen_run(
    state: &AppState,
    app: &AppHandle,
    run: CaptureRun,
) -> AppResult<ReopenedCaptureRun> {
    let raw_path = run
        .mesh_preview
        .as_ref()
        .map(|preview| std::path::PathBuf::from(&preview.stl_path))
        .ok_or_else(|| AppError::not_found("Capture run has no reconstructed STL."))?;
    if !raw_path.is_file() {
        return Err(AppError::not_found(format!(
            "Capture STL is missing: {}",
            raw_path.display()
        )));
    }
    let frames = read_capture_manifest(&app.app_data_dir().join("captures"), &run.id)
        .map(|manifest| manifest.frames)
        .unwrap_or_default();
    let session = state
        .reopen_capture_session(&run, frames, CAPTURE_SESSION_TTL_SECS)
        .await?;
    Ok(ReopenedCaptureRun { run, session })
}

fn read_capture_manifest(root: &std::path::Path, run_id: &str) -> Option<CaptureFrameManifest> {
    let bytes = std::fs::read(root.join(run_id).join("manifest.json")).ok()?;
    serde_json::from_slice(&bytes).ok()
}

fn latest_legacy_capture_stl(root: &std::path::Path) -> AppResult<Option<std::path::PathBuf>> {
    if !root.is_dir() {
        return Ok(None);
    }
    let mut candidates = Vec::new();
    for entry in std::fs::read_dir(root)
        .map_err(|error| AppError::persistence(format!("Capture storage read failed: {error}")))?
    {
        let run_root = entry
            .map_err(|error| AppError::persistence(error.to_string()))?
            .path();
        let reconstruction = run_root.join("reconstruction");
        for name in ["preview.stl", "model.stl"] {
            let candidate = reconstruction.join(name);
            if !candidate.is_file() {
                continue;
            }
            let modified = candidate
                .metadata()
                .and_then(|metadata| metadata.modified())
                .unwrap_or(std::time::UNIX_EPOCH);
            candidates.push((modified, candidate));
        }
    }
    candidates.sort_by_key(|candidate| std::cmp::Reverse(candidate.0));
    Ok(candidates.into_iter().next().map(|(_, path)| path))
}

fn raw_stl_run_id(path: &std::path::Path) -> AppResult<&str> {
    path.parent()
        .and_then(std::path::Path::parent)
        .and_then(std::path::Path::file_name)
        .and_then(std::ffi::OsStr::to_str)
        .ok_or_else(|| AppError::validation("Capture STL path has no run identity."))
}

fn inspect_capture_stl(path: &std::path::Path) -> AppResult<CaptureMeshPreview> {
    use crate::ecky_ir::mesh_asset::{IndexedMeshAsset, MeshAssetSource};
    let mesh = IndexedMeshAsset::from_stl(MeshAssetSource::Imported, path)?;
    let mut minimum = [f64::INFINITY; 3];
    let mut maximum = [f64::NEG_INFINITY; 3];
    for vertex in mesh.vertices() {
        for axis in 0..3 {
            minimum[axis] = minimum[axis].min(vertex[axis]);
            maximum[axis] = maximum[axis].max(vertex[axis]);
        }
    }
    let topology = mesh.topology();
    let mut warnings = Vec::new();
    if !topology.closed {
        warnings.push(format!(
            "Mesh is open: {} boundary edges, {} non-manifold edges.",
            topology.boundary_edge_count, topology.non_manifold_edge_count
        ));
    }
    Ok(CaptureMeshPreview {
        stl_path: path.to_string_lossy().into_owned(),
        triangle_count: mesh.triangles().len() as u64,
        bounds_mm: [
            maximum[0] - minimum[0],
            maximum[1] - minimum[1],
            maximum[2] - minimum[2],
        ],
        scale_label: "Restored capture coordinates".into(),
        warnings,
    })
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod guided_tests {
    use super::*;

    fn write_triangle_stl() -> std::path::PathBuf {
        let root =
            std::env::temp_dir().join(format!("ecky-guided-queue-mesh-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("guide.stl");
        std::fs::write(
            &path,
            "solid guide\nfacet normal 0 0 1\nouter loop\nvertex 0 0 0\nvertex 2 0 0\nvertex 0 2 0\nendloop\nendfacet\nendsolid guide\n",
        )
        .unwrap();
        path
    }

    fn test_config() -> crate::contracts::Config {
        crate::contracts::Config {
            engines: Vec::new(),
            selected_engine_id: String::new(),
            freecad_cmd: String::new(),
            cad_text_font_path: String::new(),
            freecad_library_roots: Vec::new(),
            assets: Vec::new(),
            microwave: None,
            voice: crate::contracts::VoiceConfig::default(),
            mcp: crate::contracts::McpConfig::default(),
            has_seen_onboarding: true,
            connection_type: None,
            default_engine_kind: crate::contracts::EngineKind::EckyIrV0,
            default_source_language: crate::contracts::SourceLanguage::EckyIrV0,
            default_geometry_backend: crate::contracts::GeometryBackend::EckyRust,
            max_generation_attempts: 3,
            max_verify_attempts: 0,
            projects_root: None,
        }
    }

    #[tokio::test]
    async fn guided_request_queues_only_into_capture_owning_thread_with_source_guard() {
        let db_path =
            std::env::temp_dir().join(format!("ecky-guided-queue-{}", uuid::Uuid::new_v4()));
        let conn = crate::db::init_db(&db_path).unwrap();
        let state = AppState::new(test_config(), None, conn);
        let target_source = "(solid blank)".to_string();
        let target_source_digest =
            crate::services::render_snapshot::canonical_source_digest(&target_source);
        let mut guide = CaptureReconstructionGuide::test_fixture();
        guide.capture_run_id = "run-owner".into();
        guide.target_thread_id = "thread-owner".into();
        guide.target_message_id = None;
        guide.target_source_digest = target_source_digest.clone();
        guide.target_version_id = None;
        guide.calibration.method =
            crate::contracts::CaptureCalibrationMethod::TrustedMetricMetadata {
                provenance: "fixture-metric".into(),
                accepted_by_user: true,
            };
        let mesh_path = write_triangle_stl();
        let mesh_digest = crate::capture_guidance::source_mesh_content_digest(&mesh_path).unwrap();
        guide.source_mesh.content_digest = mesh_digest.clone();
        guide.source_mesh.triangle_count = 1;
        let positions = [[0.0, 0.0, 0.0], [2.0, 0.0, 0.0], [0.0, 2.0, 0.0]];
        for (landmark, position) in guide.landmarks.iter_mut().zip(positions) {
            landmark.anchor.source_mesh_content_digest = mesh_digest.clone();
            landmark.anchor.triangle_index = 0;
            landmark.anchor.source_position = position;
            landmark.anchor.barycentric = match position {
                [0.0, 0.0, 0.0] => [1.0, 0.0, 0.0],
                [2.0, 0.0, 0.0] => [0.0, 1.0, 0.0],
                _ => [0.0, 0.0, 1.0],
            };
        }
        guide
            .measurements
            .push(crate::contracts::CaptureNamedMeasurement {
                measurement_id: "depth".into(),
                label: "extrusion depth".into(),
                landmark_ids: vec!["landmark-1".into(), "landmark-2".into()],
                value: 12.0,
                unit: "mm".into(),
                fit_critical: true,
                authored_parameter_name: Some("insert-depth".into()),
                constraint_kind: Some(crate::contracts::CaptureConstraintKind::Extent),
            });
        crate::capture_guidance::recompute_guide_geometry_from_stl(&mesh_path, &mut guide).unwrap();
        guide.canonical_digest = guide.compute_canonical_digest().unwrap();
        let round_tripped: CaptureReconstructionGuide =
            serde_json::from_str(&serde_json::to_string(&guide).unwrap()).unwrap();
        assert_eq!(
            guide.canonical_digest,
            round_tripped.compute_canonical_digest().unwrap(),
            "capture guide digest must survive persistence JSON"
        );
        let run = CaptureRun {
            id: guide.capture_run_id.clone(),
            target_thread_id: guide.target_thread_id.clone(),
            target_message_id: None,
            title: "Owning thread".into(),
            state: CaptureSessionState::Preview,
            created_at: 1,
            updated_at: 1,
            accepted_frame_count: 3,
            mesh_preview: None,
            derived_stl_path: None,
            crop_bounds: None,
            preview_scale: 1.0,
            target_source,
            target_source_language: "ecky".into(),
            started_from_empty: true,
            raw_error: None,
            reconstruction_guide: Some(guide.clone()),
            reconstruction_guide_state: Some(CaptureReconstructionGuideState::Ready),
            guided_reconstruction_message_id: None,
            guided_reconstruction_model_id: None,
            guided_reconstruction_result: None,
            guided_reconstruction_deviation: None,
        };
        {
            let db = state.db.lock().await;
            crate::capture_runs::insert(&db, &run).unwrap();
        }

        let queued = queue_capture_guided_reconstruction_impl(
            &run.id,
            guide.revision,
            &target_source_digest,
            &state,
        )
        .await
        .unwrap();
        assert_eq!(queued.thread_id, "thread-owner");
        let stored = {
            let db = state.db.lock().await;
            crate::services::history::get_thread(&db, "thread-owner").unwrap()
        };
        assert_eq!(stored.messages.len(), 1);
        assert_eq!(
            stored.messages[0].status,
            crate::contracts::MessageStatus::Pending
        );
        assert!(stored.messages[0].content.contains(&queued.request_id));
        let pending = {
            let db = state.db.lock().await;
            crate::capture_runs::pending_guided_reconstruction_for_thread(&db, "thread-owner")
                .unwrap()
                .unwrap()
        };
        assert_eq!(pending.run_id, run.id);
        assert_eq!(pending.request_id, queued.request_id);

        let error = queue_capture_guided_reconstruction_impl(
            &run.id,
            guide.revision,
            "sha256:diverged",
            &state,
        )
        .await
        .unwrap_err();
        assert_eq!(
            error.message,
            "Capture target source changed since guide creation."
        );
        let _ = std::fs::remove_file(db_path);
        let _ = std::fs::remove_dir_all(mesh_path.parent().unwrap());
    }
}
