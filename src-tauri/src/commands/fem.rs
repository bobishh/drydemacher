use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex as StdMutex, OnceLock};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Emitter, State};

use ecky_fem::{
    FemApplicabilityCheckKind, FemApplicabilityStatus, FemBudgetLimits,
    FemEngineeringEvidenceLedger, FemEvidenceAuthority, FemEvidenceSubject,
    FemIdealizationArtifact, FemIdealizationKind, FemResultExtremum, FemResultFieldKind,
    FemSafetyFactor, FemStudyAssumptionCategory, FemStudyAssumptionStatus,
    FemValidationEvidenceKind, FEM_SCHEMA_VERSION,
};

use crate::contracts::{
    AppError, AppResult, FemAcceptanceEvaluationDto, FemAcceptanceEvidenceChainDto,
    FemApplicabilityCheckDto, FemAssumptionDto, FemCancelResponse, FemConvergenceLevelDto,
    FemConvergenceRequest, FemConvergenceResponse, FemEngineeringEvidenceDto,
    FemEngineeringQuestionDto, FemExtremumDto, FemIdealizationDto, FemInputEvidenceDto,
    FemMeshPreviewResponse, FemResultArrayDto, FemResultReadRequest, FemResultReadResponse,
    FemResultSummaryDto, FemRunResponse, FemSensitivityEvidenceDto, FemSensitivityMetricDto,
    FemStudyRequest, FemStudyValidationResponse, FemSupportReactionDto, FemValidationEvidenceDto,
    FemVerificationLayerDto, FemVtuExportResponse,
};
use crate::ecky_cad_host::analysis_boundary::{
    load_direct_occt_analysis_boundary_surface, AnalysisBoundarySurface,
};
use crate::fem_engineering::{
    authored_study_from_core, engineering_ledger_from_core, resolve_fem_face_tags,
};
use crate::fem_mesher::{probe_ftetwild_runtime, FTetWildRuntimeRequirement};
use crate::models::{AppState, PathResolver};
use crate::services::fem::{
    execute_fem_mesh_pipeline, execute_fem_pipeline_with_mesh_size, FemPipelineControl,
    FemPipelineStage, FemProgressEvent,
};
use crate::services::fem_artifacts::{
    export_fem_result_vtu as write_fem_result_vtu, load_fem_mesh_asset, load_fem_result_asset,
    publish_fem_mesh_asset, publish_fem_result_asset, FemMeshAsset, FemResultAsset, FemScalarType,
};

const FTETWILD_RUNTIME_VERSION: &str = "0.1.0-ecky.1";
const FTETWILD_SOURCE_REVISION: &str = "d7d99bb4387a07895b9adce058dc7305f6b6e5ab";
const FEM_REQUEST_CACHE_ROOT: &str = "fem-request-cache-v1";
const FEM_MESH_REQUEST_CACHE_ROOT: &str = "fem-mesh-request-cache-v1";
const FEM_REQUEST_CACHE_SCHEMA_VERSION: u32 = 2;
const FEM_SINGLEFLIGHT_LIMIT: usize = 256;
const FEM_REQUEST_CACHE_ENTRY_LIMIT: usize = 128;
const FEM_REQUEST_CACHE_BYTE_LIMIT: u64 = 2 * 1024 * 1024;
static FEM_REQUEST_CACHE_NONCE: AtomicU64 = AtomicU64::new(1);
static FEM_RUN_SINGLEFLIGHT: OnceLock<StdMutex<HashMap<String, Arc<FemSharedJob>>>> =
    OnceLock::new();

struct FemSharedJob {
    gate: StdMutex<()>,
    execution_cancelled: AtomicBool,
    subscribers: StdMutex<HashMap<u64, Arc<AtomicBool>>>,
}

struct FemSharedSubscription {
    job: Arc<FemSharedJob>,
    subscriber_id: u64,
}

struct FemCancellationMonitor {
    stopped: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl FemSharedJob {
    fn new() -> Self {
        Self {
            gate: StdMutex::new(()),
            execution_cancelled: AtomicBool::new(false),
            subscribers: StdMutex::new(HashMap::new()),
        }
    }

    fn subscribe(
        self: &Arc<Self>,
        cancellation: Arc<AtomicBool>,
    ) -> AppResult<FemSharedSubscription> {
        let mut subscribers = self
            .subscribers
            .lock()
            .map_err(|_| AppError::internal("FEM subscriber registry was poisoned."))?;
        if subscribers.is_empty() {
            self.execution_cancelled.store(false, Ordering::Release);
        }
        let subscriber_id = FEM_REQUEST_CACHE_NONCE.fetch_add(1, Ordering::Relaxed);
        subscribers.insert(subscriber_id, cancellation);
        drop(subscribers);
        self.refresh_cancellation()?;
        Ok(FemSharedSubscription {
            job: self.clone(),
            subscriber_id,
        })
    }

    fn refresh_cancellation(&self) -> AppResult<()> {
        let subscribers = self
            .subscribers
            .lock()
            .map_err(|_| AppError::internal("FEM subscriber registry was poisoned."))?;
        if !subscribers.is_empty()
            && subscribers
                .values()
                .all(|cancelled| cancelled.load(Ordering::Acquire))
        {
            self.execution_cancelled.store(true, Ordering::Release);
        }
        Ok(())
    }
}

impl FemSharedSubscription {
    fn start_monitor(&self) -> FemCancellationMonitor {
        let stopped = Arc::new(AtomicBool::new(false));
        let monitor_stopped = stopped.clone();
        let job = self.job.clone();
        let thread = std::thread::spawn(move || {
            while !monitor_stopped.load(Ordering::Acquire) {
                let _ = job.refresh_cancellation();
                std::thread::sleep(Duration::from_millis(5));
            }
            let _ = job.refresh_cancellation();
        });
        FemCancellationMonitor {
            stopped,
            thread: Some(thread),
        }
    }
}

impl Drop for FemSharedSubscription {
    fn drop(&mut self) {
        if let Ok(mut subscribers) = self.job.subscribers.lock() {
            subscribers.remove(&self.subscriber_id);
        }
        let _ = self.job.refresh_cancellation();
    }
}

impl Drop for FemCancellationMonitor {
    fn drop(&mut self) {
        self.stopped.store(true, Ordering::Release);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct FemRequestCacheEntry {
    schema_version: u32,
    request_digest: String,
    analysis_identity_digest: String,
    solution_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct FemMeshRequestCacheEntry {
    schema_version: u32,
    request_digest: String,
    analysis_identity_digest: String,
    mesh_content_digest: String,
}

struct ResolvedFemRequest {
    program: ecky_render::core_ir::CoreProgram,
    boundary: AnalysisBoundarySurface,
    manifest: crate::contracts::ModelManifest,
    budgets: FemBudgetLimits,
    control: FemPipelineControl,
}

#[tauri::command]
#[specta::specta]
pub fn validate_fem_study(
    request: FemStudyRequest,
    app: AppHandle,
) -> AppResult<FemStudyValidationResponse> {
    validate_fem_study_with_resolver(request, &app)
}

pub(crate) fn validate_fem_study_with_resolver(
    request: FemStudyRequest,
    app: &dyn PathResolver,
) -> AppResult<FemStudyValidationResponse> {
    let resolved = resolve_request(&request, app)?;
    let faces = resolve_fem_face_tags(&resolved.manifest.tagged_anchors, &resolved.boundary)?;
    let study = authored_study_from_core(
        &resolved.program,
        &request.analysis_name,
        &faces,
        resolved.budgets,
    )?;
    let ledger = engineering_ledger_from_core(
        &resolved.program,
        &request.analysis_name,
        &resolved.boundary.source_geometry_digest,
        &resolved.boundary.source_geometry_digest,
    )?;
    Ok(FemStudyValidationResponse {
        job_id: request.job_id,
        model_id: request.model_id,
        analysis_name: request.analysis_name,
        part_id: study.part_id,
        source_digest: crate::services::render_snapshot::canonical_source_digest(&request.source),
        source_geometry_digest: resolved.boundary.source_geometry_digest,
        boundary_digest: resolved.boundary.content_digest,
        boundary_node_count: resolved.boundary.vertices.len() as u64,
        boundary_triangle_count: resolved.boundary.triangles.len() as u64,
        face_group_count: resolved.boundary.face_groups.len() as u64,
        decision_readiness_error: ledger
            .validate_decision_readiness()
            .err()
            .map(|error| error.to_string()),
    })
}

#[tauri::command]
#[specta::specta]
pub async fn run_fem_study(
    request: FemStudyRequest,
    app: AppHandle,
    state: State<'_, AppState>,
) -> AppResult<FemRunResponse> {
    validate_job_id(&request.job_id)?;
    let cancellation = Arc::new(AtomicBool::new(false));
    {
        let mut jobs = state.fem_cancellations.lock().await;
        if jobs.contains_key(&request.job_id) {
            return Err(AppError::conflict(format!(
                "FEM job '{}' is already running.",
                request.job_id
            )));
        }
        jobs.insert(request.job_id.clone(), cancellation.clone());
    }
    let job_id = request.job_id.clone();
    let jobs = state.fem_cancellations.clone();
    let worker_app = app.clone();
    let joined = tauri::async_runtime::spawn_blocking(move || {
        let job_id = request.job_id.clone();
        run_fem_study_with_resolver_subscribed(request, &worker_app, cancellation, |progress| {
            let _ = worker_app.emit(
                "fem-progress",
                serde_json::json!({"jobId": job_id, "progress": progress}),
            );
        })
    })
    .await;
    jobs.lock().await.remove(&job_id);
    joined.map_err(|error| AppError::internal(format!("FEM job thread failed: {error}")))?
}

#[tauri::command]
#[specta::specta]
pub async fn preview_fem_mesh(
    request: FemStudyRequest,
    app: AppHandle,
    state: State<'_, AppState>,
) -> AppResult<FemMeshPreviewResponse> {
    validate_job_id(&request.job_id)?;
    let cancellation = Arc::new(AtomicBool::new(false));
    {
        let mut jobs = state.fem_cancellations.lock().await;
        if jobs.contains_key(&request.job_id) {
            return Err(AppError::conflict(format!(
                "FEM job '{}' is already running.",
                request.job_id
            )));
        }
        jobs.insert(request.job_id.clone(), cancellation.clone());
    }
    let job_id = request.job_id.clone();
    let jobs = state.fem_cancellations.clone();
    let worker_app = app.clone();
    let joined = tauri::async_runtime::spawn_blocking(move || {
        let event_job_id = request.job_id.clone();
        preview_fem_mesh_with_resolver_subscribed(request, &worker_app, cancellation, |progress| {
            let _ = worker_app.emit(
                "fem-progress",
                serde_json::json!({"jobId": event_job_id, "progress": progress}),
            );
        })
    })
    .await;
    jobs.lock().await.remove(&job_id);
    joined.map_err(|error| AppError::internal(format!("FEM mesh thread failed: {error}")))?
}

#[tauri::command]
#[specta::specta]
pub async fn cancel_fem_study(
    job_id: String,
    state: State<'_, AppState>,
) -> AppResult<FemCancelResponse> {
    validate_job_id(&job_id)?;
    let jobs = state.fem_cancellations.lock().await;
    let cancellation_requested = jobs.get(&job_id).is_some_and(|cancelled| {
        cancelled.store(true, Ordering::Release);
        true
    });
    Ok(FemCancelResponse {
        job_id,
        cancellation_requested,
    })
}

#[tauri::command]
#[specta::specta]
pub async fn run_fem_convergence(
    request: FemConvergenceRequest,
    app: AppHandle,
    state: State<'_, AppState>,
) -> AppResult<FemConvergenceResponse> {
    validate_convergence_request(&request)?;
    let job_id = request.study.job_id.clone();
    let cancellation = Arc::new(AtomicBool::new(false));
    {
        let mut jobs = state.fem_cancellations.lock().await;
        if jobs.contains_key(&job_id) {
            return Err(AppError::conflict(format!(
                "FEM job '{job_id}' is already running."
            )));
        }
        jobs.insert(job_id.clone(), cancellation.clone());
    }
    let jobs = state.fem_cancellations.clone();
    let worker_app = app.clone();
    let event_job_id = job_id.clone();
    let joined = tauri::async_runtime::spawn_blocking(move || {
        run_fem_convergence_with_resolver(request, &worker_app, cancellation.as_ref(), |progress| {
            let _ = worker_app.emit(
                "fem-progress",
                serde_json::json!({"jobId": event_job_id, "progress": progress}),
            );
        })
    })
    .await;
    jobs.lock().await.remove(&job_id);
    joined.map_err(|error| AppError::internal(format!("FEM convergence thread failed: {error}")))?
}

#[tauri::command]
#[specta::specta]
pub fn read_fem_result(
    request: FemResultReadRequest,
    app: AppHandle,
) -> AppResult<FemResultReadResponse> {
    read_fem_result_with_resolver(request, &app)
}

pub(crate) fn read_fem_result_with_resolver(
    request: FemResultReadRequest,
    app: &dyn PathResolver,
) -> AppResult<FemResultReadResponse> {
    if request.maximum_result_bytes == 0 {
        return Err(AppError::validation(
            "FEM result read byte budget must be positive.",
        ));
    }
    let asset = load_fem_result_asset(
        app,
        &request.analysis_identity_digest,
        &request.solution_digest,
        request.maximum_result_bytes,
    )?;
    Ok(asset_response(asset))
}

#[tauri::command]
#[specta::specta]
pub fn export_fem_result_vtu(
    request: FemResultReadRequest,
    target_path: String,
    app: AppHandle,
) -> AppResult<FemVtuExportResponse> {
    let asset = load_fem_result_asset(
        &app,
        &request.analysis_identity_digest,
        &request.solution_digest,
        request.maximum_result_bytes,
    )?;
    let (byte_length, sha256) =
        write_fem_result_vtu(&asset, &target_path, request.maximum_result_bytes)?;
    Ok(FemVtuExportResponse {
        path: target_path,
        byte_length,
        sha256,
        result_digest: asset.result_digest,
    })
}

#[cfg(test)]
pub(crate) fn run_fem_study_with_resolver<F>(
    request: FemStudyRequest,
    app: &dyn PathResolver,
    cancellation: &AtomicBool,
    progress: F,
) -> AppResult<FemRunResponse>
where
    F: FnMut(FemProgressEvent),
{
    run_fem_study_with_resolver_and_mesh_size_and_subscription(
        request,
        app,
        cancellation,
        None,
        None,
        progress,
    )
}

pub(crate) fn run_fem_study_with_resolver_subscribed<F>(
    request: FemStudyRequest,
    app: &dyn PathResolver,
    cancellation: Arc<AtomicBool>,
    progress: F,
) -> AppResult<FemRunResponse>
where
    F: FnMut(FemProgressEvent),
{
    run_fem_study_with_resolver_and_mesh_size_and_subscription(
        request,
        app,
        cancellation.as_ref(),
        Some(cancellation.clone()),
        None,
        progress,
    )
}

#[cfg(test)]
pub(crate) fn preview_fem_mesh_with_resolver<F>(
    request: FemStudyRequest,
    app: &dyn PathResolver,
    cancellation: &AtomicBool,
    progress: F,
) -> AppResult<FemMeshPreviewResponse>
where
    F: FnMut(FemProgressEvent),
{
    preview_fem_mesh_with_resolver_and_subscription(request, app, cancellation, None, progress)
}

pub(crate) fn preview_fem_mesh_with_resolver_subscribed<F>(
    request: FemStudyRequest,
    app: &dyn PathResolver,
    cancellation: Arc<AtomicBool>,
    progress: F,
) -> AppResult<FemMeshPreviewResponse>
where
    F: FnMut(FemProgressEvent),
{
    preview_fem_mesh_with_resolver_and_subscription(
        request,
        app,
        cancellation.as_ref(),
        Some(cancellation.clone()),
        progress,
    )
}

fn preview_fem_mesh_with_resolver_and_subscription<F>(
    request: FemStudyRequest,
    app: &dyn PathResolver,
    cancellation: &AtomicBool,
    subscriber_cancellation: Option<Arc<AtomicBool>>,
    mut progress: F,
) -> AppResult<FemMeshPreviewResponse>
where
    F: FnMut(FemProgressEvent),
{
    let started = Instant::now();
    let resolved = resolve_request(&request, app)?;
    let runtime_root = crate::runtime_capabilities::resolve_ftetwild_runtime_root(app)?;
    let runtime = probe_ftetwild_runtime(
        runtime_root,
        &FTetWildRuntimeRequirement {
            runtime_version: FTETWILD_RUNTIME_VERSION.to_string(),
            source_revision: FTETWILD_SOURCE_REVISION.to_string(),
        },
    )?;
    let request_digest = fem_request_cache_digest(&request, &resolved, &runtime, None)?;
    let singleflight = fem_singleflight_job(&format!("mesh:{request_digest}"))?;
    let subscription = subscriber_cancellation
        .map(|cancellation| singleflight.subscribe(cancellation))
        .transpose()?;
    let _cancellation_monitor = subscription
        .as_ref()
        .map(FemSharedSubscription::start_monitor);
    let execution_cancellation = subscription
        .as_ref()
        .map(|subscription| &subscription.job.execution_cancelled)
        .unwrap_or(cancellation);
    let _singleflight_guard = singleflight
        .gate
        .lock()
        .map_err(|_| AppError::internal("FEM mesh singleflight lock was poisoned."))?;
    if execution_cancellation.load(Ordering::Acquire) {
        return Err(AppError::conflict(format!(
            "FEM mesh preview '{}' was cancelled while waiting for an identical run.",
            request.analysis_name
        )));
    }
    if let Some(asset) =
        load_fem_mesh_request_cache(app, &request_digest, request.budgets.result_bytes)?
    {
        progress(FemProgressEvent {
            stage: FemPipelineStage::Publish,
            elapsed_ms: started.elapsed().as_millis().min(u64::MAX as u128) as u64,
            node_count: asset
                .arrays
                .iter()
                .find(|array| array.name == "nodesMm")
                .and_then(|array| array.shape.first())
                .copied(),
            tet4_cell_count: asset
                .arrays
                .iter()
                .find(|array| array.name == "tet4Cells")
                .and_then(|array| array.shape.first())
                .copied(),
            detail: "Loaded exact immutable FEM mesh cache; fTetWild skipped.".to_string(),
            cancellation_boundary: true,
        });
        return subscriber_result(
            mesh_asset_response(&request, asset),
            cancellation,
            &request.analysis_name,
        );
    }
    let scratch = app.app_data_dir().join("fem-scratch").join(&request.job_id);
    if scratch.exists() {
        return Err(AppError::conflict(format!(
            "FEM scratch directory '{}' already exists.",
            scratch.display()
        )));
    }
    fs::create_dir_all(&scratch).map_err(|error| {
        AppError::persistence(format!(
            "FEM scratch directory '{}' could not be created: {error}",
            scratch.display()
        ))
    })?;
    let result = execute_fem_mesh_pipeline(
        &resolved.program,
        &request.analysis_name,
        &resolved.manifest.tagged_anchors,
        &resolved.boundary,
        resolved.budgets,
        &runtime,
        &scratch,
        &resolved.control,
        None,
        execution_cancellation,
        &mut progress,
    );
    let cleanup_error = fs::remove_dir_all(&scratch).err();
    let result = result?;
    if let Some(error) = cleanup_error {
        return Err(AppError::persistence(format!(
            "FEM scratch cleanup '{}' failed: {error}",
            scratch.display()
        )));
    }
    let asset = publish_fem_mesh_asset(app, &result, request.budgets.result_bytes)?;
    store_fem_mesh_request_cache(app, &request_digest, &asset)?;
    progress(FemProgressEvent {
        stage: FemPipelineStage::Publish,
        elapsed_ms: started.elapsed().as_millis().min(u64::MAX as u128) as u64,
        node_count: Some(result.mesh.nodes.len() as u64),
        tet4_cell_count: Some(result.mesh.cells.len() as u64),
        detail: "Published immutable FEM mesh artifact.".to_string(),
        cancellation_boundary: true,
    });
    subscriber_result(
        mesh_asset_response(&request, asset),
        cancellation,
        &request.analysis_name,
    )
}

pub(crate) fn run_fem_study_with_resolver_and_mesh_size<F>(
    request: FemStudyRequest,
    app: &dyn PathResolver,
    cancellation: &AtomicBool,
    mesh_size_override_mm: Option<f64>,
    progress: F,
) -> AppResult<FemRunResponse>
where
    F: FnMut(FemProgressEvent),
{
    run_fem_study_with_resolver_and_mesh_size_and_subscription(
        request,
        app,
        cancellation,
        None,
        mesh_size_override_mm,
        progress,
    )
}

fn run_fem_study_with_resolver_and_mesh_size_and_subscription<F>(
    request: FemStudyRequest,
    app: &dyn PathResolver,
    cancellation: &AtomicBool,
    subscriber_cancellation: Option<Arc<AtomicBool>>,
    mesh_size_override_mm: Option<f64>,
    mut progress: F,
) -> AppResult<FemRunResponse>
where
    F: FnMut(FemProgressEvent),
{
    let started = Instant::now();
    let resolved = resolve_request(&request, app)?;
    let runtime_root = crate::runtime_capabilities::resolve_ftetwild_runtime_root(app)?;
    let runtime = probe_ftetwild_runtime(
        runtime_root,
        &FTetWildRuntimeRequirement {
            runtime_version: FTETWILD_RUNTIME_VERSION.to_string(),
            source_revision: FTETWILD_SOURCE_REVISION.to_string(),
        },
    )?;
    let request_digest =
        fem_request_cache_digest(&request, &resolved, &runtime, mesh_size_override_mm)?;
    let singleflight = fem_singleflight_job(&request_digest)?;
    let subscription = subscriber_cancellation
        .map(|cancellation| singleflight.subscribe(cancellation))
        .transpose()?;
    let _cancellation_monitor = subscription
        .as_ref()
        .map(FemSharedSubscription::start_monitor);
    let execution_cancellation = subscription
        .as_ref()
        .map(|subscription| &subscription.job.execution_cancelled)
        .unwrap_or(cancellation);
    let _singleflight_guard = singleflight
        .gate
        .lock()
        .map_err(|_| AppError::internal("FEM singleflight lock was poisoned."))?;
    if execution_cancellation.load(Ordering::Acquire) {
        return Err(AppError::conflict(format!(
            "FEM study '{}' was cancelled while waiting for an identical run.",
            request.analysis_name
        )));
    }
    if let Some(asset) = load_fem_request_cache(app, &request_digest, request.budgets.result_bytes)?
    {
        progress(FemProgressEvent {
            stage: FemPipelineStage::Publish,
            elapsed_ms: started.elapsed().as_millis().min(u64::MAX as u128) as u64,
            node_count: asset
                .arrays
                .iter()
                .find(|array| array.name == "nodesMm")
                .and_then(|array| array.shape.first())
                .copied(),
            tet4_cell_count: asset
                .arrays
                .iter()
                .find(|array| array.name == "tet4Cells")
                .and_then(|array| array.shape.first())
                .copied(),
            detail: "Loaded exact immutable FEM result cache; mesh and solve skipped.".to_string(),
            cancellation_boundary: true,
        });
        return subscriber_result(
            result_response(&request, asset),
            cancellation,
            &request.analysis_name,
        );
    }
    let scratch = app.app_data_dir().join("fem-scratch").join(&request.job_id);
    if scratch.exists() {
        return Err(AppError::conflict(format!(
            "FEM scratch directory '{}' already exists.",
            scratch.display()
        )));
    }
    fs::create_dir_all(&scratch).map_err(|error| {
        AppError::persistence(format!(
            "FEM scratch directory '{}' could not be created: {error}",
            scratch.display()
        ))
    })?;
    let result = execute_fem_pipeline_with_mesh_size(
        &resolved.program,
        &request.analysis_name,
        &resolved.manifest.tagged_anchors,
        &resolved.boundary,
        resolved.budgets,
        &runtime,
        &scratch,
        &resolved.control,
        mesh_size_override_mm,
        execution_cancellation,
        &mut progress,
    );
    let cleanup_error = fs::remove_dir_all(&scratch).err();
    let result = result?;
    if let Some(error) = cleanup_error {
        return Err(AppError::persistence(format!(
            "FEM scratch cleanup '{}' failed: {error}",
            scratch.display()
        )));
    }
    let source_digest = crate::services::render_snapshot::canonical_source_digest(&request.source);
    let asset =
        publish_fem_result_asset(app, &result, &source_digest, request.budgets.result_bytes)?;
    store_fem_request_cache(app, &request_digest, &asset)?;
    progress(FemProgressEvent {
        stage: FemPipelineStage::Publish,
        elapsed_ms: started.elapsed().as_millis().min(u64::MAX as u128) as u64,
        node_count: Some(result.mesh.nodes.len() as u64),
        tet4_cell_count: Some(result.mesh.cells.len() as u64),
        detail: "Published immutable FEM result artifact.".to_string(),
        cancellation_boundary: true,
    });
    subscriber_result(
        result_response(&request, asset),
        cancellation,
        &request.analysis_name,
    )
}

fn subscriber_result<T>(value: T, cancellation: &AtomicBool, analysis_name: &str) -> AppResult<T> {
    if cancellation.load(Ordering::Acquire) {
        return Err(AppError::conflict(format!(
            "FEM subscriber for study '{analysis_name}' was cancelled."
        )));
    }
    Ok(value)
}

fn fem_request_cache_digest(
    request: &FemStudyRequest,
    resolved: &ResolvedFemRequest,
    runtime: &crate::fem_mesher::FTetWildRuntimeIdentity,
    mesh_size_override_mm: Option<f64>,
) -> AppResult<String> {
    fem_request_cache_digest_components(
        request,
        &resolved.boundary.content_digest,
        &resolved.boundary.source_geometry_digest,
        &resolved.manifest.tagged_anchors,
        runtime,
        mesh_size_override_mm,
    )
}

fn fem_request_cache_digest_components<T: Serialize>(
    request: &FemStudyRequest,
    boundary_digest: &str,
    source_geometry_digest: &str,
    tagged_anchors: &T,
    runtime: &crate::fem_mesher::FTetWildRuntimeIdentity,
    mesh_size_override_mm: Option<f64>,
) -> AppResult<String> {
    let value = serde_json::json!({
        "schemaVersion": FEM_REQUEST_CACHE_SCHEMA_VERSION,
        "modelId": request.model_id,
        "sourceDigest": crate::services::render_snapshot::canonical_source_digest(&request.source),
        "analysisName": request.analysis_name,
        "boundaryDigest": boundary_digest,
        "sourceGeometryDigest": source_geometry_digest,
        "taggedAnchors": tagged_anchors,
        "budgets": request.budgets,
        "control": request.control,
        "meshSizeOverrideMm": mesh_size_override_mm,
        "runtime": {
            "runtimeName": runtime.runtime_name,
            "runtimeVersion": runtime.runtime_version,
            "sourceRevision": runtime.source_revision,
            "platform": runtime.platform,
            "arch": runtime.arch,
            "workerProtocol": runtime.worker_protocol,
            "executableSha256": runtime.executable_sha256,
            "sourceSha256": runtime.source_sha256,
            "licenseSha256": runtime.license_sha256,
            "noticeSha256": runtime.notice_sha256,
            "transitiveLicenseInventorySha256": runtime.transitive_license_inventory_sha256,
            "capabilities": runtime.capabilities,
        }
    });
    let bytes = serde_json::to_vec(&value).map_err(|error| {
        AppError::internal(format!(
            "FEM request cache identity serialization failed: {error}"
        ))
    })?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

fn fem_singleflight_job(request_digest: &str) -> AppResult<Arc<FemSharedJob>> {
    let registry = FEM_RUN_SINGLEFLIGHT.get_or_init(|| StdMutex::new(HashMap::new()));
    let mut registry = registry
        .lock()
        .map_err(|_| AppError::internal("FEM singleflight registry was poisoned."))?;
    if registry.len() >= FEM_SINGLEFLIGHT_LIMIT {
        registry.retain(|_, job| Arc::strong_count(job) > 1);
    }
    if registry.len() >= FEM_SINGLEFLIGHT_LIMIT && !registry.contains_key(request_digest) {
        return Err(AppError::conflict(format!(
            "FEM singleflight budget exceeded: observed {}, allowed {FEM_SINGLEFLIGHT_LIMIT}.",
            registry.len() + 1
        )));
    }
    let replace_cancelled_generation = registry
        .get(request_digest)
        .is_some_and(|job| job.execution_cancelled.load(Ordering::Acquire));
    if replace_cancelled_generation {
        registry.insert(request_digest.to_string(), Arc::new(FemSharedJob::new()));
    }
    Ok(registry
        .entry(request_digest.to_string())
        .or_insert_with(|| Arc::new(FemSharedJob::new()))
        .clone())
}

fn load_fem_request_cache(
    app: &dyn PathResolver,
    request_digest: &str,
    maximum_result_bytes: u64,
) -> AppResult<Option<FemResultAsset>> {
    let path = fem_request_cache_path(app, request_digest)?;
    if !path.exists() {
        return Ok(None);
    }
    let bytes = fs::read(&path).map_err(|error| {
        AppError::persistence(format!(
            "FEM request cache '{}' read failed: {error}",
            path.display()
        ))
    })?;
    if bytes.len() > 16 * 1024 {
        return Err(AppError::validation(
            "FEM request cache entry exceeds 16 KiB.",
        ));
    }
    let entry: FemRequestCacheEntry = serde_json::from_slice(&bytes).map_err(|error| {
        AppError::validation(format!(
            "FEM request cache '{}' is invalid: {error}",
            path.display()
        ))
    })?;
    if entry.schema_version != FEM_REQUEST_CACHE_SCHEMA_VERSION
        || entry.request_digest != request_digest
    {
        return Err(AppError::conflict("FEM request cache identity is stale."));
    }
    load_fem_result_asset(
        app,
        &entry.analysis_identity_digest,
        &entry.solution_digest,
        maximum_result_bytes,
    )
    .map(Some)
}

fn load_fem_mesh_request_cache(
    app: &dyn PathResolver,
    request_digest: &str,
    maximum_result_bytes: u64,
) -> AppResult<Option<FemMeshAsset>> {
    let path = fem_cache_path(app, FEM_MESH_REQUEST_CACHE_ROOT, request_digest)?;
    if !path.exists() {
        return Ok(None);
    }
    let bytes = fs::read(&path).map_err(|error| {
        AppError::persistence(format!(
            "FEM mesh request cache '{}' read failed: {error}",
            path.display()
        ))
    })?;
    if bytes.len() > 16 * 1024 {
        return Err(AppError::validation(
            "FEM mesh request cache entry exceeds 16 KiB.",
        ));
    }
    let entry: FemMeshRequestCacheEntry = serde_json::from_slice(&bytes).map_err(|error| {
        AppError::validation(format!(
            "FEM mesh request cache '{}' is invalid: {error}",
            path.display()
        ))
    })?;
    if entry.schema_version != FEM_REQUEST_CACHE_SCHEMA_VERSION
        || entry.request_digest != request_digest
    {
        return Err(AppError::conflict(
            "FEM mesh request cache identity is stale.",
        ));
    }
    load_fem_mesh_asset(
        app,
        &entry.analysis_identity_digest,
        &entry.mesh_content_digest,
        maximum_result_bytes,
    )
    .map(Some)
}

fn store_fem_request_cache(
    app: &dyn PathResolver,
    request_digest: &str,
    asset: &FemResultAsset,
) -> AppResult<()> {
    let path = fem_request_cache_path(app, request_digest)?;
    let parent = path.parent().expect("FEM request cache path has parent");
    fs::create_dir_all(parent).map_err(|error| {
        AppError::persistence(format!(
            "FEM request cache root '{}' create failed: {error}",
            parent.display()
        ))
    })?;
    let bytes = serde_json::to_vec_pretty(&FemRequestCacheEntry {
        schema_version: FEM_REQUEST_CACHE_SCHEMA_VERSION,
        request_digest: request_digest.to_string(),
        analysis_identity_digest: asset.analysis_identity_digest.clone(),
        solution_digest: asset.solution_digest.clone(),
    })
    .map_err(|error| {
        AppError::internal(format!("FEM request cache serialization failed: {error}"))
    })?;
    let temporary = parent.join(format!(
        ".publishing-{}-{}",
        std::process::id(),
        FEM_REQUEST_CACHE_NONCE.fetch_add(1, Ordering::Relaxed)
    ));
    let mut file = fs::File::create(&temporary).map_err(|error| {
        AppError::persistence(format!(
            "FEM request cache temporary create failed: {error}"
        ))
    })?;
    file.write_all(&bytes).map_err(|error| {
        AppError::persistence(format!("FEM request cache temporary write failed: {error}"))
    })?;
    file.sync_all().map_err(|error| {
        AppError::persistence(format!("FEM request cache temporary sync failed: {error}"))
    })?;
    fs::rename(&temporary, &path).map_err(|error| {
        let _ = fs::remove_file(&temporary);
        AppError::persistence(format!("FEM request cache atomic publish failed: {error}"))
    })?;
    prune_fem_cache_directory(
        parent,
        FEM_REQUEST_CACHE_ENTRY_LIMIT,
        FEM_REQUEST_CACHE_BYTE_LIMIT,
    )
}

fn store_fem_mesh_request_cache(
    app: &dyn PathResolver,
    request_digest: &str,
    asset: &FemMeshAsset,
) -> AppResult<()> {
    let bytes = serde_json::to_vec_pretty(&FemMeshRequestCacheEntry {
        schema_version: FEM_REQUEST_CACHE_SCHEMA_VERSION,
        request_digest: request_digest.to_string(),
        analysis_identity_digest: asset.analysis_identity_digest.clone(),
        mesh_content_digest: asset.mesh_content_digest.clone(),
    })
    .map_err(|error| {
        AppError::internal(format!(
            "FEM mesh request cache serialization failed: {error}"
        ))
    })?;
    store_fem_cache_bytes(app, FEM_MESH_REQUEST_CACHE_ROOT, request_digest, &bytes)
}

fn fem_request_cache_path(app: &dyn PathResolver, request_digest: &str) -> AppResult<PathBuf> {
    fem_cache_path(app, FEM_REQUEST_CACHE_ROOT, request_digest)
}

fn fem_cache_path(app: &dyn PathResolver, root: &str, request_digest: &str) -> AppResult<PathBuf> {
    let hex = request_digest.strip_prefix("sha256:").ok_or_else(|| {
        AppError::validation("FEM request cache identity must use sha256 prefix.")
    })?;
    if hex.len() != 64 || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(AppError::validation(
            "FEM request cache identity must contain 64 hexadecimal characters.",
        ));
    }
    Ok(app.app_data_dir().join(root).join(format!("{hex}.json")))
}

fn store_fem_cache_bytes(
    app: &dyn PathResolver,
    root: &str,
    request_digest: &str,
    bytes: &[u8],
) -> AppResult<()> {
    let path = fem_cache_path(app, root, request_digest)?;
    let parent = path.parent().expect("FEM cache path has parent");
    fs::create_dir_all(parent).map_err(|error| {
        AppError::persistence(format!(
            "FEM cache root '{}' create failed: {error}",
            parent.display()
        ))
    })?;
    let temporary = parent.join(format!(
        ".publishing-{}-{}",
        std::process::id(),
        FEM_REQUEST_CACHE_NONCE.fetch_add(1, Ordering::Relaxed)
    ));
    let mut file = fs::File::create(&temporary).map_err(|error| {
        AppError::persistence(format!("FEM cache temporary create failed: {error}"))
    })?;
    file.write_all(bytes).map_err(|error| {
        AppError::persistence(format!("FEM cache temporary write failed: {error}"))
    })?;
    file.sync_all().map_err(|error| {
        AppError::persistence(format!("FEM cache temporary sync failed: {error}"))
    })?;
    fs::rename(&temporary, &path).map_err(|error| {
        let _ = fs::remove_file(&temporary);
        AppError::persistence(format!("FEM cache atomic publish failed: {error}"))
    })?;
    prune_fem_cache_directory(
        parent,
        FEM_REQUEST_CACHE_ENTRY_LIMIT,
        FEM_REQUEST_CACHE_BYTE_LIMIT,
    )
}

fn prune_fem_cache_directory(
    directory: &std::path::Path,
    maximum_entries: usize,
    maximum_bytes: u64,
) -> AppResult<()> {
    let mut entries = fs::read_dir(directory)
        .map_err(|error| {
            AppError::persistence(format!(
                "FEM cache root '{}' read failed: {error}",
                directory.display()
            ))
        })?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            if path.extension().and_then(|value| value.to_str()) != Some("json") {
                return None;
            }
            let metadata = entry.metadata().ok()?;
            let modified = metadata.modified().unwrap_or(std::time::UNIX_EPOCH);
            Some((modified, path, metadata.len()))
        })
        .collect::<Vec<_>>();
    entries.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(&right.1)));
    let mut retained_bytes = 0_u64;
    for (index, (_, path, bytes)) in entries.into_iter().enumerate() {
        let retain = index < maximum_entries
            && retained_bytes
                .checked_add(bytes)
                .is_some_and(|total| total <= maximum_bytes);
        if retain {
            retained_bytes += bytes;
        } else {
            fs::remove_file(&path).map_err(|error| {
                AppError::persistence(format!(
                    "FEM cache entry '{}' prune failed: {error}",
                    path.display()
                ))
            })?;
        }
    }
    Ok(())
}

pub(crate) fn run_fem_convergence_with_resolver<F>(
    request: FemConvergenceRequest,
    app: &dyn PathResolver,
    cancellation: &AtomicBool,
    mut progress: F,
) -> AppResult<FemConvergenceResponse>
where
    F: FnMut(FemProgressEvent),
{
    validate_convergence_request(&request)?;
    run_fem_convergence_sequence(request, cancellation, |study, mesh_size_mm| {
        run_fem_study_with_resolver_and_mesh_size(
            study,
            app,
            cancellation,
            Some(mesh_size_mm),
            &mut progress,
        )
    })
}

fn run_fem_convergence_sequence<R>(
    request: FemConvergenceRequest,
    cancellation: &AtomicBool,
    mut run_level: R,
) -> AppResult<FemConvergenceResponse>
where
    R: FnMut(FemStudyRequest, f64) -> AppResult<FemRunResponse>,
{
    let base_job_id = request.study.job_id.clone();
    let model_id = request.study.model_id.clone();
    let analysis_name = request.study.analysis_name.clone();
    let mut levels = Vec::with_capacity(request.mesh_sizes_mm.len());
    let mut latest_acceptance_evaluations = Vec::new();
    let mut sequence_status = "completed".to_string();
    for (index, mesh_size_mm) in request.mesh_sizes_mm.iter().copied().enumerate() {
        if cancellation.load(Ordering::Acquire) {
            sequence_status = "cancelled".to_string();
            levels.push(failed_convergence_level(
                mesh_size_mm,
                "cancelled",
                format!(
                    "FEM convergence '{base_job_id}' was cancelled before level {}.",
                    index + 1
                ),
            ));
            break;
        }
        let mut study = request.study.clone();
        study.job_id = format!("{base_job_id}-level-{}", index + 1);
        let response = match run_level(study, mesh_size_mm) {
            Ok(response) => response,
            Err(error) => {
                let cancelled = cancellation.load(Ordering::Acquire);
                sequence_status = if cancelled { "cancelled" } else { "failed" }.to_string();
                levels.push(failed_convergence_level(
                    mesh_size_mm,
                    if cancelled { "cancelled" } else { "failed" },
                    error.to_string(),
                ));
                break;
            }
        };
        let previous = levels
            .iter()
            .rev()
            .find(|level| level.status == "completed");
        latest_acceptance_evaluations = response.acceptance_evaluations.clone();
        let displacement_relative_delta = previous.and_then(|level| {
            level
                .maximum_displacement_mm
                .map(|previous| relative_delta(previous, response.summary.maximum_displacement_mm))
        });
        let stress_relative_delta = previous.and_then(|level| {
            level
                .maximum_von_mises_mpa
                .map(|previous| relative_delta(previous, response.summary.maximum_von_mises_mpa))
        });
        levels.push(FemConvergenceLevelDto {
            mesh_size_mm,
            status: "completed".to_string(),
            error: None,
            analysis_identity_digest: Some(response.analysis_identity_digest),
            solution_digest: Some(response.solution_digest),
            result_digest: Some(response.result_digest),
            mesh_content_digest: Some(response.mesh_content_digest),
            node_count: Some(response.summary.node_count),
            tet4_cell_count: Some(response.summary.tet4_cell_count),
            minimum_scaled_jacobian: Some(response.summary.minimum_scaled_jacobian),
            equilibrium_relative_imbalance: Some(response.summary.equilibrium_relative_imbalance),
            solver_relative_residual: Some(response.summary.solver_relative_residual),
            maximum_displacement_mm: Some(response.summary.maximum_displacement_mm),
            maximum_von_mises_mpa: Some(response.summary.maximum_von_mises_mpa),
            displacement_relative_delta,
            stress_relative_delta,
        });
    }
    let failed = sequence_status != "completed";
    let displacement_status = if failed {
        "failed".to_string()
    } else {
        convergence_status(
            levels
                .iter()
                .filter_map(|level| level.displacement_relative_delta),
            request.displacement_relative_tolerance,
            false,
        )
    };
    let stress_values = levels
        .iter()
        .filter_map(|level| level.maximum_von_mises_mpa)
        .collect::<Vec<_>>();
    let stress_rising = stress_values.windows(2).all(|pair| pair[1] > pair[0]);
    let stress_status = if failed {
        "failed".to_string()
    } else {
        convergence_status(
            levels
                .iter()
                .filter_map(|level| level.stress_relative_delta),
            request.stress_relative_tolerance,
            stress_rising,
        )
    };
    let acceptance_evaluations = latest_acceptance_evaluations
        .into_iter()
        .map(|evaluation| {
            complete_converged_acceptance_evaluation(
                evaluation,
                &sequence_status,
                &displacement_status,
                &stress_status,
            )
        })
        .collect();
    Ok(FemConvergenceResponse {
        job_id: base_job_id,
        model_id,
        analysis_name,
        sequence_status,
        levels,
        displacement_status,
        stress_status,
        acceptance_evaluations,
    })
}

fn complete_converged_acceptance_evaluation(
    mut evaluation: FemAcceptanceEvaluationDto,
    sequence_status: &str,
    displacement_status: &str,
    stress_status: &str,
) -> FemAcceptanceEvaluationDto {
    if evaluation.status != "pending" {
        return evaluation;
    }
    let metric_status = match evaluation.field.as_str() {
        "maximum-displacement" | "maximumDisplacement" => Some(displacement_status),
        "von-mises-stress"
        | "vonMisesStress"
        | "maximum-principal-stress"
        | "maximumPrincipalStress" => Some(stress_status),
        _ => None,
    };
    let Some(metric_status) = metric_status else {
        evaluation.detail = format!(
            "FEM acceptance metric '{}' requires convergence, but field '{}' has no convergence series.",
            evaluation.metric_id, evaluation.field
        );
        return evaluation;
    };
    evaluation.convergence_status = Some(metric_status.to_string());
    evaluation.evidence_chain.convergence_status = Some(metric_status.to_string());
    if sequence_status != "completed" || metric_status != "converged" {
        evaluation.detail = format!(
            "FEM acceptance metric '{}' cannot pass: convergence sequence '{}', metric status '{}'.",
            evaluation.metric_id, sequence_status, metric_status
        );
        return evaluation;
    }
    let Some(observed) = evaluation.observed else {
        evaluation.status = "failed".to_string();
        evaluation.detail = format!(
            "FEM acceptance metric '{}' has no current numeric result.",
            evaluation.metric_id
        );
        return evaluation;
    };
    let passed = match evaluation.comparison.as_str() {
        "lessThanOrEqual" => observed <= evaluation.threshold,
        "greaterThanOrEqual" => observed >= evaluation.threshold,
        _ => false,
    };
    evaluation.status = if passed { "passed" } else { "failed" }.to_string();
    evaluation.detail = format!(
        "FEM acceptance metric '{}' {} after '{}' convergence: observed {} {}, threshold {} {}.",
        evaluation.metric_id,
        evaluation.status,
        metric_status,
        observed,
        evaluation.unit,
        evaluation.threshold,
        evaluation.unit
    );
    evaluation
}

fn failed_convergence_level(
    mesh_size_mm: f64,
    status: &str,
    error: String,
) -> FemConvergenceLevelDto {
    FemConvergenceLevelDto {
        mesh_size_mm,
        status: status.to_string(),
        error: Some(error),
        analysis_identity_digest: None,
        solution_digest: None,
        result_digest: None,
        mesh_content_digest: None,
        node_count: None,
        tet4_cell_count: None,
        minimum_scaled_jacobian: None,
        equilibrium_relative_imbalance: None,
        solver_relative_residual: None,
        maximum_displacement_mm: None,
        maximum_von_mises_mpa: None,
        displacement_relative_delta: None,
        stress_relative_delta: None,
    }
}

fn validate_convergence_request(request: &FemConvergenceRequest) -> AppResult<()> {
    validate_job_id(&request.study.job_id)?;
    if request.mesh_sizes_mm.len() < 3
        || request.mesh_sizes_mm.len() as u64 > request.study.budgets.convergence_levels
    {
        return Err(AppError::validation(format!(
            "FEM convergence requires 3..={} explicit mesh sizes; observed {}.",
            request.study.budgets.convergence_levels,
            request.mesh_sizes_mm.len()
        )));
    }
    if request
        .mesh_sizes_mm
        .iter()
        .any(|value| !value.is_finite() || *value <= 0.0)
        || request
            .mesh_sizes_mm
            .windows(2)
            .any(|pair| pair[1] >= pair[0])
    {
        return Err(AppError::validation(
            "FEM convergence meshSizesMm must be finite, positive, and strictly coarse-to-fine.",
        ));
    }
    for (name, tolerance) in [
        (
            "displacementRelativeTolerance",
            request.displacement_relative_tolerance,
        ),
        ("stressRelativeTolerance", request.stress_relative_tolerance),
    ] {
        if !tolerance.is_finite() || tolerance <= 0.0 || tolerance >= 1.0 {
            return Err(AppError::validation(format!(
                "FEM convergence {name} must be finite between 0 and 1."
            )));
        }
    }
    Ok(())
}

fn relative_delta(previous: f64, current: f64) -> f64 {
    (current - previous).abs() / current.abs().max(previous.abs()).max(f64::EPSILON)
}

fn convergence_status(
    deltas: impl Iterator<Item = f64>,
    tolerance: f64,
    suspected_singularity: bool,
) -> String {
    let deltas = deltas.collect::<Vec<_>>();
    if deltas.iter().all(|delta| *delta <= tolerance) {
        "converged".to_string()
    } else if suspected_singularity {
        "suspectedSingularity".to_string()
    } else {
        "unconverged".to_string()
    }
}

fn resolve_request(
    request: &FemStudyRequest,
    app: &dyn PathResolver,
) -> AppResult<ResolvedFemRequest> {
    validate_job_id(&request.job_id)?;
    if request.model_id.trim().is_empty()
        || request.analysis_name.trim().is_empty()
        || request.source.trim().is_empty()
    {
        return Err(AppError::validation(
            "FEM request requires modelId, source, and analysisName.",
        ));
    }
    let (_bundle, manifest) = crate::model_runtime::read_runtime_bundle(app, &request.model_id)?;
    let source_digest = crate::services::render_snapshot::canonical_source_digest(&request.source);
    if manifest.source_digest.as_deref() != Some(source_digest.as_str()) {
        return Err(AppError::conflict(format!(
            "FEM source is stale for model '{}': request digest '{}', manifest digest '{}'.",
            request.model_id,
            source_digest,
            manifest.source_digest.as_deref().unwrap_or("missing")
        )));
    }
    let program = crate::ecky_scheme::compile_to_core_program(&request.source)
        .map_err(|error| AppError::validation(format!("FEM source compilation failed: {error}")))?;
    let analyses = program
        .analyses
        .iter()
        .filter(|analysis| analysis.name == request.analysis_name)
        .collect::<Vec<_>>();
    let analysis = match analyses.as_slice() {
        [analysis] => *analysis,
        [] => {
            return Err(AppError::not_found(format!(
                "FEM analysis '{}' was not found.",
                request.analysis_name
            )))
        }
        _ => {
            return Err(AppError::validation(format!(
                "FEM analysis '{}' is duplicate.",
                request.analysis_name
            )))
        }
    };
    let bundle_dir = crate::model_runtime::runtime_bundle_dir(app, &request.model_id)?;
    let boundary = load_direct_occt_analysis_boundary_surface(&bundle_dir, &analysis.part)?;
    let budgets = FemBudgetLimits {
        schema_version: FEM_SCHEMA_VERSION,
        boundary_triangles: request.budgets.boundary_triangles,
        tet4_cells: request.budgets.tet4_cells,
        nodes: request.budgets.nodes,
        dofs: request.budgets.dofs,
        sparse_nonzeros: request.budgets.sparse_nonzeros,
        result_bytes: request.budgets.result_bytes,
        convergence_levels: request.budgets.convergence_levels,
    };
    budgets
        .validate()
        .map_err(|error| AppError::validation(format!("FEM budgets are invalid: {error}")))?;
    let control = FemPipelineControl {
        envelope_mm: request.control.envelope_mm,
        minimum_scaled_jacobian: request.control.minimum_scaled_jacobian,
        maximum_runtime_ms: request.control.maximum_runtime_ms,
        relative_solver_tolerance: request.control.relative_solver_tolerance,
    };
    control.validate()?;
    Ok(ResolvedFemRequest {
        program,
        boundary,
        manifest,
        budgets,
        control,
    })
}

fn result_response(request: &FemStudyRequest, asset: FemResultAsset) -> FemRunResponse {
    let response = asset_response(asset);
    FemRunResponse {
        job_id: request.job_id.clone(),
        model_id: request.model_id.clone(),
        analysis_name: request.analysis_name.clone(),
        source_digest: response.source_digest.clone(),
        analysis_identity_digest: response.analysis_identity_digest,
        solution_digest: response.solution_digest,
        result_digest: response.result_digest,
        mesh_content_digest: response.mesh_content_digest,
        source_boundary_digest: response.source_boundary_digest,
        decision_ready: response.decision_ready,
        decision_readiness_error: response.decision_readiness_error,
        manifest_path: response.manifest_path,
        arrays: response.arrays,
        summary: response.summary,
        support_reactions: response.support_reactions,
        engineering_evidence: response.engineering_evidence,
        acceptance_evaluations: response.acceptance_evaluations,
    }
}

fn asset_response(asset: FemResultAsset) -> FemResultReadResponse {
    let asset_dir = asset
        .manifest_path
        .parent()
        .map(PathBuf::from)
        .unwrap_or_default();
    let minimum_yield_safety_factor = match asset.summary.minimum_yield_safety_factor {
        FemSafetyFactor::Finite { value } => Some(value),
        FemSafetyFactor::Infinite => None,
    };
    let node_count = asset
        .arrays
        .iter()
        .find(|array| array.name == "nodesMm")
        .and_then(|array| array.shape.first())
        .copied()
        .unwrap_or(0);
    let tet4_cell_count = asset
        .arrays
        .iter()
        .find(|array| array.name == "tet4Cells")
        .and_then(|array| array.shape.first())
        .copied()
        .unwrap_or(0);
    let engineering_evidence = engineering_evidence_response(
        &asset.engineering_evidence,
        &asset.idealization_artifact,
        &asset.idealization_artifact_digest,
    );
    FemResultReadResponse {
        source_digest: asset.source_digest,
        analysis_identity_digest: asset.analysis_identity_digest,
        solution_digest: asset.solution_digest,
        result_digest: asset.result_digest,
        mesh_content_digest: asset.mesh_content_digest,
        source_boundary_digest: asset.source_boundary_digest,
        decision_ready: asset.decision_ready,
        decision_readiness_error: asset.decision_readiness_error,
        manifest_path: asset.manifest_path.to_string_lossy().into_owned(),
        arrays: asset
            .arrays
            .into_iter()
            .map(|array| FemResultArrayDto {
                name: array.name,
                path: asset_dir.join(array.path).to_string_lossy().into_owned(),
                scalar_type: match array.scalar_type {
                    FemScalarType::Float64Le => "float64Le".to_string(),
                    FemScalarType::Uint32Le => "uint32Le".to_string(),
                },
                shape: array.shape,
                byte_length: array.byte_length,
                sha256: array.sha256,
            })
            .collect(),
        summary: FemResultSummaryDto {
            maximum_displacement_mm: asset.summary.maximum_displacement.value,
            maximum_von_mises_mpa: asset.summary.maximum_von_mises.value,
            maximum_principal_stress_mpa: asset.summary.maximum_principal_stress.value,
            volume_mm3: asset.summary.volume_mm3,
            mass_kg: asset.summary.mass_kg,
            minimum_yield_safety_factor,
            equilibrium_relative_imbalance: asset.equilibrium_relative_imbalance,
            solver_relative_residual: asset.solver_relative_residual,
            minimum_scaled_jacobian: asset.mesh_quality.minimum_scaled_jacobian,
            node_count,
            tet4_cell_count,
            extrema: [
                &asset.summary.maximum_displacement,
                &asset.summary.maximum_von_mises,
                &asset.summary.maximum_principal_stress,
            ]
            .into_iter()
            .map(extremum_response)
            .collect(),
        },
        support_reactions: asset
            .support_reactions
            .into_iter()
            .map(|reaction| FemSupportReactionDto {
                name: reaction.name,
                face_group_indices: reaction.face_group_indices,
                resultant_n: reaction.resultant_n,
            })
            .collect(),
        engineering_evidence,
        acceptance_evaluations: asset
            .acceptance_evaluations
            .into_iter()
            .map(|evaluation| {
                let evidence_chain =
                    acceptance_evidence_chain(&asset.engineering_evidence, &evaluation);
                FemAcceptanceEvaluationDto {
                    study_name: evaluation.study_name,
                    metric_id: evaluation.metric_id,
                    field: evaluation.field,
                    status: evaluation.status,
                    observed: evaluation.observed,
                    unit: evaluation.unit,
                    threshold: evaluation.threshold,
                    comparison: evaluation.comparison,
                    mesh_size_mm: evaluation.mesh_size_mm,
                    node_id: evaluation.node_id,
                    element_id: evaluation.element_id,
                    coordinate_mm: evaluation.coordinate_mm,
                    analysis_identity_digest: evaluation.analysis_identity_digest,
                    mesh_content_digest: evaluation.mesh_content_digest,
                    result_digest: evaluation.result_digest,
                    convergence_status: evaluation.convergence_status,
                    evidence_chain,
                    detail: evaluation.detail,
                }
            })
            .collect(),
    }
}

fn engineering_evidence_response(
    ledger: &FemEngineeringEvidenceLedger,
    idealization_artifact: &FemIdealizationArtifact,
    idealization_artifact_digest: &str,
) -> FemEngineeringEvidenceDto {
    let inputs = ledger
        .input_bindings
        .iter()
        .filter_map(|binding| {
            ledger
                .evidence
                .iter()
                .find(|record| record.evidence_id == binding.evidence_id)
                .map(|record| FemInputEvidenceDto {
                    input_name: binding.input_name.clone(),
                    evidence_id: binding.evidence_id.clone(),
                    subject: evidence_subject_name(record.subject).into(),
                    source: record.source.clone(),
                    authority: evidence_authority_name(record.authority).into(),
                    uncertainty_percent: record.uncertainty_percent,
                    decision_critical: record.decision_critical,
                })
        })
        .collect();
    let validation_evidence = ledger
        .validation_evidence
        .iter()
        .map(|evidence| FemValidationEvidenceDto {
            validation_id: evidence.validation_id.clone(),
            kind: validation_kind_name(evidence.kind).into(),
            source: evidence.source.clone(),
            result_digest: evidence.result_digest.clone(),
        })
        .collect::<Vec<_>>();
    FemEngineeringEvidenceDto {
        question: FemEngineeringQuestionDto {
            question_id: ledger.question.question_id.clone(),
            statement: ledger.question.statement.clone(),
            decision: ledger.question.decision.clone(),
            acceptance_metric_ids: ledger.question.acceptance_metric_ids.clone(),
        },
        idealization: FemIdealizationDto {
            artifact_digest: idealization_artifact_digest.to_string(),
            kind: match idealization_artifact.kind {
                FemIdealizationKind::ExactSolid => "exactSolid",
                FemIdealizationKind::DefeaturedSolid => "defeaturedSolid",
            }
            .into(),
            source_geometry_digest: ledger.idealization.source_geometry_digest.clone(),
            analysis_geometry_digest: ledger.idealization.analysis_geometry_digest.clone(),
            manufacturing_geometry_digest: idealization_artifact
                .manufacturing_geometry_digest
                .clone(),
            affected_topology_ids: ledger.idealization.affected_topology_ids.clone(),
            justification: ledger.idealization.justification.clone(),
            expected_influence_percent: ledger.idealization.expected_influence_percent,
            accepted_by_user: ledger.idealization.accepted_by_user,
        },
        inputs,
        assumptions: ledger
            .assumptions
            .iter()
            .map(|assumption| FemAssumptionDto {
                assumption_id: assumption.assumption_id.clone(),
                category: assumption_category_name(assumption.category).into(),
                statement: assumption.statement.clone(),
                status: assumption_status_name(assumption.status).into(),
                evidence_ids: assumption.evidence_ids.clone(),
            })
            .collect(),
        applicability: ledger
            .applicability_checks
            .iter()
            .map(|check| FemApplicabilityCheckDto {
                check_id: check.check_id.clone(),
                kind: applicability_kind_name(check.kind).into(),
                status: applicability_status_name(check.status).into(),
                observed: check.observed,
                limit: check.limit,
                unit: check.unit.clone(),
                evidence_ids: check.evidence_ids.clone(),
                detail: check.detail.clone(),
            })
            .collect(),
        sensitivity: ledger
            .sensitivity
            .as_ref()
            .map(|sensitivity| FemSensitivityEvidenceDto {
                completed: sensitivity.completed,
                case_result_digests: sensitivity.case_result_digests.clone(),
                metric_ranges: sensitivity
                    .metric_ranges
                    .iter()
                    .map(|metric| FemSensitivityMetricDto {
                        metric_id: metric.metric_id.clone(),
                        nominal: metric.nominal,
                        minimum: metric.minimum,
                        maximum: metric.maximum,
                        unit: metric.unit.clone(),
                        dominant_input_name: metric.dominant_input_name.clone(),
                        decision_changed: metric.decision_changed,
                    })
                    .collect(),
            }),
        validation_evidence,
        verification_layers: verification_layers(ledger),
    }
}

fn acceptance_evidence_chain(
    ledger: &FemEngineeringEvidenceLedger,
    evaluation: &crate::services::fem::FemAcceptanceEvaluation,
) -> FemAcceptanceEvidenceChainDto {
    let mut gaps = Vec::new();
    if !ledger.idealization.accepted_by_user {
        gaps.push("analysis idealization is not user accepted".into());
    }
    for evidence in &ledger.evidence {
        if matches!(
            evidence.authority,
            FemEvidenceAuthority::Unknown | FemEvidenceAuthority::Proposed
        ) {
            gaps.push(format!(
                "input evidence '{}' is not authoritative",
                evidence.evidence_id
            ));
        }
    }
    for check in &ledger.applicability_checks {
        if matches!(
            check.status,
            FemApplicabilityStatus::Blocked | FemApplicabilityStatus::NotEvaluated
        ) {
            gaps.push(format!(
                "applicability check '{}' is not passing",
                check.check_id
            ));
        }
    }
    if evaluation.status == "pending" {
        gaps.push("acceptance metric requires unresolved current evidence".into());
    }
    if !ledger.validation_evidence.iter().any(|evidence| {
        matches!(
            evidence.kind,
            FemValidationEvidenceKind::QualifiedReference | FemValidationEvidenceKind::PhysicalTest
        )
    }) {
        gaps.push("physical or qualified-reference validation is missing".into());
    }
    FemAcceptanceEvidenceChainDto {
        source_geometry_digest: ledger.idealization.source_geometry_digest.clone(),
        analysis_geometry_digest: ledger.idealization.analysis_geometry_digest.clone(),
        idealization_accepted: ledger.idealization.accepted_by_user,
        input_evidence_ids: ledger
            .input_bindings
            .iter()
            .map(|binding| binding.evidence_id.clone())
            .collect(),
        applicability_check_ids: ledger
            .applicability_checks
            .iter()
            .map(|check| check.check_id.clone())
            .collect(),
        convergence_status: evaluation.convergence_status.clone(),
        sensitivity_result_digests: ledger
            .sensitivity
            .as_ref()
            .map(|sensitivity| sensitivity.case_result_digests.clone())
            .unwrap_or_default(),
        validation_evidence_ids: ledger
            .validation_evidence
            .iter()
            .map(|evidence| evidence.validation_id.clone())
            .collect(),
        gaps,
    }
}

fn verification_layers(ledger: &FemEngineeringEvidenceLedger) -> Vec<FemVerificationLayerDto> {
    let layer = |name: &str,
                 kind: Option<FemValidationEvidenceKind>,
                 missing_status: &str,
                 passed_detail: &str,
                 missing_detail: &str| {
        let evidence_ids = kind
            .map(|kind| {
                ledger
                    .validation_evidence
                    .iter()
                    .filter(|evidence| evidence.kind == kind)
                    .map(|evidence| evidence.validation_id.clone())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        FemVerificationLayerDto {
            layer: name.into(),
            status: if evidence_ids.is_empty() {
                missing_status.into()
            } else {
                "passed".into()
            },
            evidence_ids,
            detail: if kind.is_some_and(|kind| {
                ledger
                    .validation_evidence
                    .iter()
                    .any(|evidence| evidence.kind == kind)
            }) {
                passed_detail.into()
            } else {
                missing_detail.into()
            },
        }
    };
    vec![
        layer(
            "analyticalUnit",
            Some(FemValidationEvidenceKind::Analytical),
            "missing",
            "Analytical or unit-level mechanics proof is recorded.",
            "No result-bound analytical or unit proof is recorded.",
        ),
        layer(
            "differentialSolver",
            Some(FemValidationEvidenceKind::DifferentialSolver),
            "missing",
            "Independent solver comparison is recorded.",
            "No independent solver comparison is recorded.",
        ),
        layer(
            "meshConvergence",
            None,
            "pending",
            "Current mesh convergence evidence is attached.",
            "No current convergence sequence is attached to this single-run result.",
        ),
        {
            let evidence_ids = ledger
                .validation_evidence
                .iter()
                .filter(|evidence| {
                    matches!(
                        evidence.kind,
                        FemValidationEvidenceKind::QualifiedReference
                            | FemValidationEvidenceKind::PhysicalTest
                    )
                })
                .map(|evidence| evidence.validation_id.clone())
                .collect::<Vec<_>>();
            FemVerificationLayerDto {
                layer: "physicalReference".into(),
                status: if evidence_ids.is_empty() {
                    "missing".into()
                } else {
                    "passed".into()
                },
                detail: if evidence_ids.is_empty() {
                    "No physical or qualified-reference validation is recorded.".into()
                } else {
                    "Physical or qualified-reference validation is recorded.".into()
                },
                evidence_ids,
            }
        },
    ]
}

fn evidence_subject_name(value: FemEvidenceSubject) -> &'static str {
    match value {
        FemEvidenceSubject::Material => "material",
        FemEvidenceSubject::Load => "load",
        FemEvidenceSubject::Support => "support",
        FemEvidenceSubject::Connection => "connection",
        FemEvidenceSubject::Geometry => "geometry",
        FemEvidenceSubject::AcceptanceCriterion => "acceptanceCriterion",
    }
}

fn evidence_authority_name(value: FemEvidenceAuthority) -> &'static str {
    match value {
        FemEvidenceAuthority::Unknown => "unknown",
        FemEvidenceAuthority::Proposed => "proposed",
        FemEvidenceAuthority::UserAccepted => "userAccepted",
        FemEvidenceAuthority::RecordedSource => "recordedSource",
    }
}

fn assumption_category_name(value: FemStudyAssumptionCategory) -> &'static str {
    match value {
        FemStudyAssumptionCategory::Geometry => "geometry",
        FemStudyAssumptionCategory::Physics => "physics",
        FemStudyAssumptionCategory::Material => "material",
        FemStudyAssumptionCategory::Load => "load",
        FemStudyAssumptionCategory::Support => "support",
        FemStudyAssumptionCategory::Connection => "connection",
    }
}

fn assumption_status_name(value: FemStudyAssumptionStatus) -> &'static str {
    match value {
        FemStudyAssumptionStatus::Unknown => "unknown",
        FemStudyAssumptionStatus::Proposed => "proposed",
        FemStudyAssumptionStatus::Accepted => "accepted",
        FemStudyAssumptionStatus::Rejected => "rejected",
    }
}

fn applicability_kind_name(value: FemApplicabilityCheckKind) -> &'static str {
    match value {
        FemApplicabilityCheckKind::OneSolidScope => "oneSolidScope",
        FemApplicabilityCheckKind::UnsupportedInterfaces => "unsupportedInterfaces",
        FemApplicabilityCheckKind::ThinSlenderTet4Risk => "thinSlenderTet4Risk",
        FemApplicabilityCheckKind::NearIncompressibleLocking => "nearIncompressibleLocking",
        FemApplicabilityCheckKind::ConstraintRealism => "constraintRealism",
        FemApplicabilityCheckKind::ConcentratedLoadSingularity => "concentratedLoadSingularity",
        FemApplicabilityCheckKind::DisplacementRatio => "displacementRatio",
        FemApplicabilityCheckKind::ElasticRange => "elasticRange",
        FemApplicabilityCheckKind::HotspotStability => "hotspotStability",
        FemApplicabilityCheckKind::BoundaryConditionSingularity => "boundaryConditionSingularity",
    }
}

fn applicability_status_name(value: FemApplicabilityStatus) -> &'static str {
    match value {
        FemApplicabilityStatus::Pass => "pass",
        FemApplicabilityStatus::Warning => "warning",
        FemApplicabilityStatus::Blocked => "blocked",
        FemApplicabilityStatus::NotEvaluated => "notEvaluated",
    }
}

fn validation_kind_name(value: FemValidationEvidenceKind) -> &'static str {
    match value {
        FemValidationEvidenceKind::Analytical => "analytical",
        FemValidationEvidenceKind::DifferentialSolver => "differentialSolver",
        FemValidationEvidenceKind::QualifiedReference => "qualifiedReference",
        FemValidationEvidenceKind::PhysicalTest => "physicalTest",
    }
}

fn extremum_response(extremum: &FemResultExtremum) -> FemExtremumDto {
    FemExtremumDto {
        field_kind: match extremum.field_kind {
            FemResultFieldKind::DisplacementMagnitude => "displacementMagnitude",
            FemResultFieldKind::VonMisesStress => "vonMisesStress",
            FemResultFieldKind::PrincipalStressMaximum => "principalStressMaximum",
        }
        .to_string(),
        value: extremum.value,
        unit: extremum.unit.clone(),
        node_id: extremum.node_id,
        element_id: extremum.element_id,
        coordinate_mm: [
            extremum.coordinate_mm.x_mm,
            extremum.coordinate_mm.y_mm,
            extremum.coordinate_mm.z_mm,
        ],
        mesh_content_digest: extremum.mesh_content_digest.clone(),
        source_boundary_digest: extremum.source_boundary_digest.clone(),
    }
}

fn mesh_asset_response(request: &FemStudyRequest, asset: FemMeshAsset) -> FemMeshPreviewResponse {
    let asset_dir = asset
        .manifest_path
        .parent()
        .map(PathBuf::from)
        .unwrap_or_default();
    let node_count = asset
        .arrays
        .iter()
        .find(|array| array.name == "nodesMm")
        .and_then(|array| array.shape.first())
        .copied()
        .unwrap_or(0);
    let tet4_cell_count = asset
        .arrays
        .iter()
        .find(|array| array.name == "tet4Cells")
        .and_then(|array| array.shape.first())
        .copied()
        .unwrap_or(0);
    let boundary_triangle_count = asset
        .arrays
        .iter()
        .find(|array| array.name == "boundaryTriangles")
        .and_then(|array| array.shape.first())
        .copied()
        .unwrap_or(0);
    FemMeshPreviewResponse {
        job_id: request.job_id.clone(),
        model_id: request.model_id.clone(),
        analysis_name: request.analysis_name.clone(),
        analysis_identity_digest: asset.analysis_identity_digest,
        mesh_content_digest: asset.mesh_content_digest,
        source_boundary_digest: asset.source_boundary_digest,
        manifest_path: asset.manifest_path.to_string_lossy().into_owned(),
        arrays: asset
            .arrays
            .into_iter()
            .map(|array| FemResultArrayDto {
                name: array.name,
                path: asset_dir.join(array.path).to_string_lossy().into_owned(),
                scalar_type: match array.scalar_type {
                    FemScalarType::Float64Le => "float64Le".to_string(),
                    FemScalarType::Uint32Le => "uint32Le".to_string(),
                },
                shape: array.shape,
                byte_length: array.byte_length,
                sha256: array.sha256,
            })
            .collect(),
        node_count,
        tet4_cell_count,
        boundary_triangle_count,
        face_group_count: asset.face_group_count as u64,
        minimum_scaled_jacobian: asset.mesh_quality.minimum_scaled_jacobian,
        minimum_radius_ratio: asset.mesh_quality.minimum_radius_ratio,
        connected_component_count: asset.mesh_quality.connected_component_count as u64,
        boundary_area_mm2_by_group: asset.mesh_quality.boundary_area_mm2_by_group,
    }
}

fn validate_job_id(job_id: &str) -> AppResult<()> {
    if job_id.is_empty()
        || job_id.len() > 128
        || !job_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        Err(AppError::validation(
            "FEM jobId must contain 1-128 ASCII letters, digits, '-' or '_'.",
        ))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone)]
    struct TestResolver {
        root: PathBuf,
    }

    impl PathResolver for TestResolver {
        fn app_config_dir(&self) -> PathBuf {
            self.root.join("config")
        }

        fn app_data_dir(&self) -> PathBuf {
            self.root.join("data")
        }

        fn resource_path(&self, _path: &str) -> Option<PathBuf> {
            None
        }
    }

    #[test]
    fn shared_fem_job_stops_only_after_final_subscriber_cancels() {
        let job = Arc::new(FemSharedJob::new());
        let first_cancelled = Arc::new(AtomicBool::new(false));
        let second_cancelled = Arc::new(AtomicBool::new(false));
        let first = job.subscribe(first_cancelled.clone()).unwrap();
        let second = job.subscribe(second_cancelled.clone()).unwrap();

        first_cancelled.store(true, Ordering::Release);
        job.refresh_cancellation().unwrap();
        assert!(!job.execution_cancelled.load(Ordering::Acquire));

        second_cancelled.store(true, Ordering::Release);
        job.refresh_cancellation().unwrap();
        assert!(job.execution_cancelled.load(Ordering::Acquire));

        drop(first);
        drop(second);
        let next = job.subscribe(Arc::new(AtomicBool::new(false))).unwrap();
        assert!(!job.execution_cancelled.load(Ordering::Acquire));
        drop(next);
    }

    #[test]
    fn subscriber_after_final_cancellation_gets_a_fresh_job_generation() {
        let digest = format!(
            "subscriber-rollover-{}",
            FEM_REQUEST_CACHE_NONCE.fetch_add(1, Ordering::Relaxed)
        );
        let cancelled_job = fem_singleflight_job(&digest).unwrap();
        let cancelled = Arc::new(AtomicBool::new(true));
        let old_subscription = cancelled_job.subscribe(cancelled).unwrap();
        cancelled_job.refresh_cancellation().unwrap();
        assert!(cancelled_job.execution_cancelled.load(Ordering::Acquire));

        let fresh_job = fem_singleflight_job(&digest).unwrap();

        assert!(!Arc::ptr_eq(&cancelled_job, &fresh_job));
        assert!(!fresh_job.execution_cancelled.load(Ordering::Acquire));
        drop(old_subscription);
    }

    #[test]
    fn request_cache_prunes_to_bounded_immutable_json_entries() {
        let root = std::env::temp_dir().join(format!(
            "ecky-fem-cache-bound-{}-{}",
            std::process::id(),
            FEM_REQUEST_CACHE_NONCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).unwrap();
        for index in 0..5 {
            fs::write(root.join(format!("{index}.json")), vec![b'x'; 32]).unwrap();
        }
        fs::write(root.join("active.tmp"), b"private publication").unwrap();

        prune_fem_cache_directory(&root, 3, 80).unwrap();

        let json_entries = fs::read_dir(&root)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| {
                entry.path().extension().and_then(|value| value.to_str()) == Some("json")
            })
            .collect::<Vec<_>>();
        assert_eq!(json_entries.len(), 2);
        assert!(root.join("active.tmp").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn request_cache_identity_changes_for_every_physics_and_provenance_input() {
        use crate::fem_mesher::{FTetWildRuntimeCapabilities, FTetWildRuntimeIdentity};

        let request = convergence_request().study;
        let selectors = serde_json::json!({
            "mount": {"kind":"face", "durableTargetIds":["body:face:mount"]},
            "load": {"kind":"face", "durableTargetIds":["body:face:load"]}
        });
        let runtime = FTetWildRuntimeIdentity {
            runtime_name: "fTetWild".into(),
            runtime_version: "pinned".into(),
            source_revision: "revision-a".into(),
            platform: "test".into(),
            arch: "test".into(),
            worker_protocol: "protocol-v1".into(),
            executable_path: PathBuf::from("/runtime/ftetwild-worker"),
            runtime_library_paths: vec![],
            executable_sha256: "sha256:executable-a".into(),
            source_sha256: "sha256:source-a".into(),
            license_sha256: "sha256:license".into(),
            notice_sha256: "sha256:notice".into(),
            transitive_license_inventory_sha256: "sha256:inventory".into(),
            capabilities: FTetWildRuntimeCapabilities {
                structured_arrays: true,
                tet4: true,
                wide_surface_tags: true,
                isolated_worker: true,
            },
        };
        let digest = |request: &FemStudyRequest,
                      boundary: &str,
                      geometry: &str,
                      selectors: &serde_json::Value,
                      runtime: &FTetWildRuntimeIdentity,
                      mesh_size: Option<f64>| {
            fem_request_cache_digest_components(
                request, boundary, geometry, selectors, runtime, mesh_size,
            )
            .expect("cache identity")
        };
        let baseline = digest(
            &request,
            "sha256:boundary-a",
            "sha256:geometry-a",
            &selectors,
            &runtime,
            None,
        );

        for (label, source) in [
            ("source", "(model source-b)"),
            ("material", "(model (material steel))"),
            ("load", "(model (load service-load-b))"),
            ("constraint", "(model (constraint mount-b))"),
        ] {
            let mut changed = request.clone();
            changed.source = source.into();
            assert_ne!(
                baseline,
                digest(
                    &changed,
                    "sha256:boundary-a",
                    "sha256:geometry-a",
                    &selectors,
                    &runtime,
                    None
                ),
                "{label} mutation must invalidate cache"
            );
        }

        let mut changed_params = request.clone();
        changed_params.model_id = "model-parameter-snapshot-b".into();
        assert_ne!(
            baseline,
            digest(
                &changed_params,
                "sha256:boundary-a",
                "sha256:geometry-a",
                &selectors,
                &runtime,
                None
            ),
            "parameter snapshot mutation must invalidate cache"
        );
        assert_ne!(
            baseline,
            digest(
                &request,
                "sha256:boundary-b",
                "sha256:geometry-a",
                &selectors,
                &runtime,
                None
            ),
            "boundary mesh mutation must invalidate cache"
        );
        assert_ne!(
            baseline,
            digest(
                &request,
                "sha256:boundary-a",
                "sha256:geometry-b",
                &selectors,
                &runtime,
                None
            ),
            "exact geometry mutation must invalidate cache"
        );
        let mut changed_selectors = selectors.clone();
        changed_selectors["mount"]["durableTargetIds"][0] = serde_json::json!("body:face:mount-b");
        assert_ne!(
            baseline,
            digest(
                &request,
                "sha256:boundary-a",
                "sha256:geometry-a",
                &changed_selectors,
                &runtime,
                None
            ),
            "selector mutation must invalidate cache"
        );
        let mut changed_tolerance = request.clone();
        changed_tolerance.control.relative_solver_tolerance = 1.0e-9;
        assert_ne!(
            baseline,
            digest(
                &changed_tolerance,
                "sha256:boundary-a",
                "sha256:geometry-a",
                &selectors,
                &runtime,
                None
            ),
            "solver tolerance mutation must invalidate cache"
        );
        assert_ne!(
            baseline,
            digest(
                &request,
                "sha256:boundary-a",
                "sha256:geometry-a",
                &selectors,
                &runtime,
                Some(0.5)
            ),
            "volume-mesh control mutation must invalidate cache"
        );
        let mut changed_runtime = runtime.clone();
        changed_runtime.executable_sha256 = "sha256:executable-b".into();
        assert_ne!(
            baseline,
            digest(
                &request,
                "sha256:boundary-a",
                "sha256:geometry-a",
                &selectors,
                &changed_runtime,
                None
            ),
            "runtime binary mutation must invalidate cache"
        );
    }

    #[test]
    fn convergence_status_never_averages_away_red_levels() {
        assert_eq!(
            convergence_status([0.01, 0.02].into_iter(), 0.03, false),
            "converged"
        );
        assert_eq!(
            convergence_status([0.01, 0.08].into_iter(), 0.03, false),
            "unconverged"
        );
        assert_eq!(
            convergence_status([0.04, 0.08].into_iter(), 0.03, true),
            "suspectedSingularity"
        );
    }

    #[test]
    fn convergence_requires_three_strictly_decreasing_bounded_sizes() {
        let base = convergence_request();
        assert!(validate_convergence_request(&base).is_ok());
        let mut invalid = base.clone();
        invalid.mesh_sizes_mm = vec![4.0, 4.0, 1.0];
        assert!(validate_convergence_request(&invalid).is_err());
    }

    fn convergence_request() -> FemConvergenceRequest {
        FemConvergenceRequest {
            study: FemStudyRequest {
                job_id: "convergence".to_string(),
                model_id: "model".to_string(),
                source: "(model)".to_string(),
                analysis_name: "study".to_string(),
                budgets: crate::contracts::FemBudgetLimitsDto {
                    boundary_triangles: 10,
                    tet4_cells: 10,
                    nodes: 10,
                    dofs: 30,
                    sparse_nonzeros: 100,
                    result_bytes: 1024,
                    convergence_levels: 3,
                },
                control: crate::contracts::FemPipelineControlDto {
                    envelope_mm: 0.1,
                    minimum_scaled_jacobian: 1.0e-6,
                    maximum_runtime_ms: 1000,
                    relative_solver_tolerance: 1.0e-8,
                },
            },
            mesh_sizes_mm: vec![4.0, 2.0, 1.0],
            displacement_relative_tolerance: 0.03,
            stress_relative_tolerance: 0.05,
        }
    }

    fn convergence_run_response(job_id: String, value: f64) -> FemRunResponse {
        FemRunResponse {
            job_id,
            model_id: "model".to_string(),
            analysis_name: "study".to_string(),
            source_digest: "sha256:source".to_string(),
            analysis_identity_digest: format!("sha256:analysis-{value}"),
            solution_digest: format!("sha256:solution-{value}"),
            result_digest: format!("sha256:result-{value}"),
            mesh_content_digest: format!("sha256:mesh-{value}"),
            source_boundary_digest: "sha256:boundary".to_string(),
            decision_ready: true,
            decision_readiness_error: None,
            manifest_path: "/tmp/fem-result.json".to_string(),
            arrays: vec![],
            summary: FemResultSummaryDto {
                maximum_displacement_mm: value,
                maximum_von_mises_mpa: value * 100.0,
                maximum_principal_stress_mpa: value * 110.0,
                volume_mm3: 1.0,
                mass_kg: 1.0,
                minimum_yield_safety_factor: Some(2.0),
                equilibrium_relative_imbalance: 1.0e-12,
                solver_relative_residual: 1.0e-13,
                minimum_scaled_jacobian: 0.2,
                node_count: 10,
                tet4_cell_count: 20,
                extrema: vec![],
            },
            support_reactions: vec![],
            engineering_evidence: serde_json::from_value(serde_json::json!({
                "question": {"questionId":"q", "statement":"question", "decision":"decide", "acceptanceMetricIds":["metric"]},
                "idealization": {"artifactDigest":"sha256:idealization", "kind":"exactSolid", "sourceGeometryDigest":"sha256:geometry", "analysisGeometryDigest":"sha256:geometry", "manufacturingGeometryDigest":"sha256:geometry", "affectedTopologyIds":[], "justification":"exact solid", "expectedInfluencePercent":0.0, "acceptedByUser":true},
                "inputs": [], "assumptions": [], "applicability": [], "sensitivity": null,
                "validationEvidence": [], "verificationLayers": []
            })).expect("test engineering evidence DTO"),
            acceptance_evaluations: vec![],
        }
    }

    #[test]
    fn convergence_preserves_completed_levels_when_intermediate_solve_fails() {
        let request = convergence_request();
        let mut invocation = 0;
        let response = run_fem_convergence_sequence(
            request,
            &AtomicBool::new(false),
            |study, _mesh_size_mm| {
                invocation += 1;
                if invocation == 2 {
                    Err(AppError::internal(
                        "Faer factorization failed at refinement level 2",
                    ))
                } else {
                    Ok(convergence_run_response(study.job_id, invocation as f64))
                }
            },
        )
        .expect("partial convergence evidence");
        assert_eq!(response.sequence_status, "failed");
        assert_eq!(response.displacement_status, "failed");
        assert_eq!(response.stress_status, "failed");
        assert_eq!(response.levels.len(), 2);
        assert_eq!(response.levels[0].status, "completed");
        assert!(response.levels[0].result_digest.is_some());
        assert!(response.levels[0].mesh_content_digest.is_some());
        assert_eq!(response.levels[0].solver_relative_residual, Some(1.0e-13));
        assert_eq!(response.levels[1].status, "failed");
        assert!(response.levels[1]
            .error
            .as_deref()
            .unwrap_or_default()
            .contains("Faer factorization failed"));
        assert!(response.levels[1].result_digest.is_none());
    }

    #[test]
    fn three_level_convergence_keeps_identity_quality_residual_extrema_and_separate_metric_status()
    {
        let mut level = 0usize;
        let displacement = [1.0, 1.01, 1.015];
        let stress = [100.0, 110.0, 125.0];
        let response = run_fem_convergence_sequence(
            convergence_request(),
            &AtomicBool::new(false),
            |study, _mesh_size_mm| {
                let mut response = convergence_run_response(study.job_id, displacement[level]);
                response.summary.maximum_von_mises_mpa = stress[level];
                response.summary.node_count = 10 + level as u64;
                response.summary.tet4_cell_count = 20 + level as u64 * 2;
                response.summary.minimum_scaled_jacobian = 0.2 - level as f64 * 0.01;
                level += 1;
                Ok(response)
            },
        )
        .expect("three-level convergence evidence");

        assert_eq!(response.sequence_status, "completed");
        assert_eq!(response.levels.len(), 3);
        assert_eq!(response.displacement_status, "converged");
        assert_eq!(response.stress_status, "suspectedSingularity");
        assert!(response.levels[0].displacement_relative_delta.is_none());
        assert!(response.levels[1..].iter().all(|level| {
            level.analysis_identity_digest.is_some()
                && level.solution_digest.is_some()
                && level.result_digest.is_some()
                && level.mesh_content_digest.is_some()
                && level.node_count.is_some()
                && level.tet4_cell_count.is_some()
                && level.minimum_scaled_jacobian.is_some()
                && level.solver_relative_residual.is_some()
                && level.maximum_displacement_mm.is_some()
                && level.maximum_von_mises_mpa.is_some()
                && level.displacement_relative_delta.is_some()
                && level.stress_relative_delta.is_some()
        }));
    }

    #[test]
    fn convergence_quality_failure_preserves_completed_level_and_raw_gate_detail() {
        let mut level = 0usize;
        let response = run_fem_convergence_sequence(
            convergence_request(),
            &AtomicBool::new(false),
            |study, _mesh_size_mm| {
                level += 1;
                if level == 2 {
                    return Err(AppError::validation(
                        "minimum scaled Jacobian -0.04 is below allowed 0.01",
                    ));
                }
                Ok(convergence_run_response(study.job_id, 1.0))
            },
        )
        .expect("quality failure evidence");

        assert_eq!(response.sequence_status, "failed");
        assert_eq!(response.levels[0].status, "completed");
        assert_eq!(response.levels[1].status, "failed");
        assert!(response.levels[1]
            .error
            .as_deref()
            .unwrap_or_default()
            .contains("scaled Jacobian -0.04"));
    }

    #[test]
    fn convergence_cancellation_returns_explicit_level_without_running_solver() {
        let mut invoked = false;
        let cancellation = AtomicBool::new(true);
        let response = run_fem_convergence_sequence(
            convergence_request(),
            &cancellation,
            |_study, _mesh_size_mm| {
                invoked = true;
                unreachable!("cancelled convergence must not start next level")
            },
        )
        .expect("cancelled sequence evidence");
        assert!(!invoked);
        assert_eq!(response.sequence_status, "cancelled");
        assert_eq!(response.levels.len(), 1);
        assert_eq!(response.levels[0].status, "cancelled");
        assert!(response.levels[0]
            .error
            .as_deref()
            .unwrap_or_default()
            .contains("cancelled"));
    }

    #[test]
    fn required_acceptance_only_passes_with_matching_metric_convergence() {
        let evaluation = FemAcceptanceEvaluationDto {
            study_name: "bracket-static".to_string(),
            metric_id: "tip-limit".to_string(),
            field: "maximum-displacement".to_string(),
            status: "pending".to_string(),
            observed: Some(0.4),
            unit: "mm".to_string(),
            threshold: 0.5,
            comparison: "lessThanOrEqual".to_string(),
            mesh_size_mm: 1.0,
            node_id: Some(7),
            element_id: None,
            coordinate_mm: Some([1.0, 2.0, 3.0]),
            analysis_identity_digest: "sha256:analysis".to_string(),
            mesh_content_digest: "sha256:mesh".to_string(),
            result_digest: "sha256:result".to_string(),
            convergence_status: None,
            evidence_chain: FemAcceptanceEvidenceChainDto {
                source_geometry_digest: "sha256:geometry".into(),
                analysis_geometry_digest: "sha256:geometry".into(),
                idealization_accepted: true,
                input_evidence_ids: vec![],
                applicability_check_ids: vec![],
                convergence_status: None,
                sensitivity_result_digests: vec![],
                validation_evidence_ids: vec![],
                gaps: vec!["convergence pending".into()],
            },
            detail: "single run pending".to_string(),
        };
        let green = complete_converged_acceptance_evaluation(
            evaluation.clone(),
            "completed",
            "converged",
            "unconverged",
        );
        assert_eq!(green.status, "passed");
        assert_eq!(green.convergence_status.as_deref(), Some("converged"));
        assert_eq!(
            green.evidence_chain.convergence_status.as_deref(),
            Some("converged")
        );

        let red = complete_converged_acceptance_evaluation(
            evaluation,
            "completed",
            "unconverged",
            "converged",
        );
        assert_eq!(red.status, "pending");
        assert!(red.detail.contains("cannot pass"));
    }

    #[test]
    fn parameter_sweep_preserves_current_selectors_and_rejects_removed_topology_without_coordinate_rebinding(
    ) {
        use crate::contracts::{DesignParams, ParamValue};
        use crate::ecky_cad_host::direct_occt_runtime::render_core_program_runtime_bundle;
        use crate::ecky_cad_host::direct_occt_sdk::{
            bundled_occt_runtime_root_from_repo, inspect_occt_runtime,
        };

        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("repository root")
            .to_path_buf();
        let layout = inspect_occt_runtime(&bundled_occt_runtime_root_from_repo(&repo_root));
        if !layout.runtime_complete() {
            eprintln!("native FEM topology-transition sweep requires complete OCCT runtime");
            return;
        }
        let source = include_str!("../../tests/fixtures/fem/parameter-topology-sweep.ecky");
        let program = crate::ecky_scheme::compile_to_core_program(source).expect("sweep fixture");
        let root = std::env::temp_dir().join(format!(
            "ecky-fem-parameter-sweep-{}-{}",
            std::process::id(),
            FEM_REQUEST_CACHE_NONCE.fetch_add(1, Ordering::Relaxed)
        ));
        let resolver = TestResolver { root: root.clone() };
        let render_case = |width: f64, cut_radius: f64| {
            let params = DesignParams::from([
                ("width".to_string(), ParamValue::Number(width)),
                ("cut-radius".to_string(), ParamValue::Number(cut_radius)),
            ]);
            let (bundle, manifest) =
                render_core_program_runtime_bundle(&program, source, &params, &layout, &resolver)
                    .expect("parameter sweep exact OCCT render");
            let bundle_dir = crate::model_runtime::runtime_bundle_dir(&resolver, &bundle.model_id)
                .expect("sweep bundle directory");
            let boundary = load_direct_occt_analysis_boundary_surface(&bundle_dir, "bracket")
                .expect("sweep analysis boundary");
            let resolved = resolve_fem_face_tags(&manifest.tagged_anchors, &boundary)
                .expect("current exact selectors survive");
            assert_eq!(resolved["mounting"].len(), 1);
            assert_eq!(resolved["load-pad"].len(), 1);
            assert_eq!(
                boundary.triangle_face_group_indices.len(),
                boundary.triangles.len()
            );
            assert!(boundary
                .face_groups
                .iter()
                .all(|group| group.triangle_count > 0));
            assert!(boundary
                .triangle_face_group_indices
                .iter()
                .all(|index| (*index as usize) < boundary.face_groups.len()));
            (manifest, boundary)
        };

        let _minimum = render_case(8.0, 0.0);
        let (nominal_manifest, nominal_boundary) = render_case(10.0, 0.0);
        let _maximum = render_case(12.0, 0.0);
        let (_transition_manifest, transition_boundary) = render_case(10.0, 2.0);

        let transition_ids = transition_boundary
            .face_groups
            .iter()
            .map(|group| group.canonical_target_id.as_str())
            .collect::<std::collections::BTreeSet<_>>();
        let removed = nominal_boundary
            .face_groups
            .iter()
            .find(|group| !transition_ids.contains(group.canonical_target_id.as_str()))
            .expect("topology transition removes at least one nominal face identity");
        let mut stale_anchors = nominal_manifest.tagged_anchors;
        stale_anchors.insert(
            "removed-transition-face".into(),
            crate::contracts::TaggedAnchorBinding {
                kind: crate::contracts::TaggedAnchorKind::Face,
                authored_selector: "exact removed nominal face".into(),
                target: "bracket".into(),
                target_ids: vec![removed.target_id.clone()],
                durable_target_ids: vec![removed
                    .durable_target_id
                    .clone()
                    .expect("nominal durable face")],
                canonical_target_ids: vec![removed.canonical_target_id.clone()],
                alias_ids: vec![],
            },
        );
        let error = resolve_fem_face_tags(&stale_anchors, &transition_boundary)
            .expect_err("removed topology must fail instead of coordinate rebinding");
        assert!(error.message.contains("resolved to 0"), "{error:?}");

        fs::remove_dir_all(root).expect("cleanup parameter sweep fixture");
    }

    #[test]
    fn exact_warm_run_uses_singleflight_cache_without_meshing_or_solving_again() {
        use crate::contracts::DesignParams;
        use crate::ecky_cad_host::direct_occt_runtime::render_core_program_runtime_bundle;
        use crate::ecky_cad_host::direct_occt_sdk::{
            bundled_occt_runtime_root_from_repo, inspect_occt_runtime,
        };

        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("repository root")
            .to_path_buf();
        let occt_root = bundled_occt_runtime_root_from_repo(&repo_root);
        let ftetwild_root = repo_root.join(".dist/runtime/ftetwild");
        if !occt_root.is_dir() || !ftetwild_root.is_dir() {
            eprintln!("native FEM cache proof is platform-gated");
            return;
        }
        let layout = inspect_occt_runtime(&occt_root);
        if !layout.runtime_complete() {
            eprintln!("native FEM cache proof requires complete OCCT runtime");
            return;
        }
        let source = r#"(model
          (tag-face mounting :faces "bottom" bracket)
          (tag-face load-pad :faces "top" bracket)
          (part bracket (box 10 10 10))
          (analysis bracket-static
            (linear-static :part bracket)
            (question bracket-strength :statement "Does the part remain elastic?" :decision "accept or revise" :acceptance-metrics [stress-limit displacement-limit])
            (acceptance-criterion stress-limit :field von-mises-stress :comparison less-than-or-equal :limit "200" :unit MPa :requires-convergence false)
            (acceptance-criterion displacement-limit :field maximum-displacement :comparison less-than-or-equal :limit "1" :unit mm :requires-convergence true)
            (idealization exact-solid :justification "Exact connected solid." :accepted true)
            (evidence material-source :subject material :source "qualified material record" :authority recorded-source :uncertainty-percent 0 :decision-critical true)
            (evidence load-source :subject load :source "accepted load case" :authority user-accepted :uncertainty-percent 0 :decision-critical true)
            (evidence support-source :subject support :source "accepted fixture" :authority user-accepted :uncertainty-percent 0 :decision-critical true)
            (input-evidence aluminum :evidence material-source)
            (input-evidence applied-load :evidence load-source)
            (input-evidence mounting :evidence support-source)
            (assumption small-strain :category physics :statement "Small displacement linear elasticity." :status accepted :evidence [material-source load-source support-source])
            (validation-evidence fixture :kind physical-test :source "versioned fixture" :result-digest "sha256:fixture")
            (material aluminum :young-modulus 68900MPa :poisson-ratio 0.33 :density 2700kg-per-m3 :yield-strength 276MPa)
            (volume-mesh :element tet4 :size 5mm)
            (fixed :faces (tag mounting))
            (surface-force :faces (tag load-pad) :total [0N 0N -10N])
            (solve :method sparse-direct)))"#;
        let program = crate::ecky_scheme::compile_to_core_program(source).expect("compile");
        let root =
            std::env::temp_dir().join(format!("ecky-fem-command-cache-{}", uuid::Uuid::new_v4()));
        let resolver = TestResolver { root: root.clone() };
        let (bundle, mut manifest) = render_core_program_runtime_bundle(
            &program,
            source,
            &DesignParams::new(),
            &layout,
            &resolver,
        )
        .expect("render exact OCCT bundle");
        manifest.source_digest = Some(crate::services::render_snapshot::canonical_source_digest(
            source,
        ));
        crate::model_runtime::write_runtime_bundle(&resolver, &bundle.model_id, &bundle, &manifest)
            .expect("bind source digest to runtime bundle");
        let make_request = |job_id: &str| FemStudyRequest {
            job_id: job_id.to_string(),
            model_id: bundle.model_id.clone(),
            source: source.to_string(),
            analysis_name: "bracket-static".to_string(),
            budgets: crate::contracts::FemBudgetLimitsDto {
                boundary_triangles: 100_000,
                tet4_cells: 500_000,
                nodes: 150_000,
                dofs: 450_000,
                sparse_nonzeros: 30_000_000,
                result_bytes: 128 * 1024 * 1024,
                convergence_levels: 3,
            },
            control: crate::contracts::FemPipelineControlDto {
                envelope_mm: 0.1,
                minimum_scaled_jacobian: 1.0e-6,
                maximum_runtime_ms: 120_000,
                relative_solver_tolerance: 1.0e-8,
            },
        };
        let mut cold_mesh_progress = Vec::new();
        let cold_mesh = preview_fem_mesh_with_resolver(
            make_request("mesh-cache-cold"),
            &resolver,
            &AtomicBool::new(false),
            |event| cold_mesh_progress.push(event),
        )
        .expect("cold FEM mesh preview");
        assert!(cold_mesh_progress
            .iter()
            .any(|event| event.stage == FemPipelineStage::VolumeMesh));
        assert!(!cold_mesh_progress
            .iter()
            .any(|event| event.stage == FemPipelineStage::Solve));
        let mut warm_mesh_progress = Vec::new();
        let warm_mesh = preview_fem_mesh_with_resolver(
            make_request("mesh-cache-warm"),
            &resolver,
            &AtomicBool::new(false),
            |event| warm_mesh_progress.push(event),
        )
        .expect("warm FEM mesh preview");
        assert_eq!(warm_mesh.mesh_content_digest, cold_mesh.mesh_content_digest);
        assert_eq!(warm_mesh_progress.len(), 1);
        assert!(warm_mesh_progress[0].detail.contains("fTetWild skipped"));

        let mut cold_progress = Vec::new();
        let cold = run_fem_study_with_resolver(
            make_request("cache-cold"),
            &resolver,
            &AtomicBool::new(false),
            |event| cold_progress.push(event),
        )
        .expect("cold FEM run");
        assert_eq!(cold.acceptance_evaluations.len(), 2);
        assert_eq!(cold.acceptance_evaluations[0].metric_id, "stress-limit");
        assert_eq!(cold.acceptance_evaluations[0].status, "passed");
        assert!(cold.acceptance_evaluations[0].element_id.is_some());
        assert_eq!(
            cold.acceptance_evaluations[0].analysis_identity_digest,
            cold.analysis_identity_digest
        );
        assert_eq!(
            cold.acceptance_evaluations[0].result_digest,
            cold.result_digest
        );
        assert_eq!(
            cold.acceptance_evaluations[1].metric_id,
            "displacement-limit"
        );
        assert_eq!(cold.acceptance_evaluations[1].status, "pending");
        assert!(cold.acceptance_evaluations[1]
            .detail
            .contains("requires current convergence evidence"));
        assert!(!cold.decision_ready);
        assert!(cold_progress
            .iter()
            .any(|event| event.stage == FemPipelineStage::VolumeMesh));
        assert!(cold_progress
            .iter()
            .any(|event| event.stage == FemPipelineStage::Solve));

        let mut warm_progress = Vec::new();
        let warm = run_fem_study_with_resolver(
            make_request("cache-warm"),
            &resolver,
            &AtomicBool::new(false),
            |event| warm_progress.push(event),
        )
        .expect("warm FEM run");
        assert_eq!(warm.analysis_identity_digest, cold.analysis_identity_digest);
        assert_eq!(warm.solution_digest, cold.solution_digest);
        assert_eq!(warm_progress.len(), 1);
        assert_eq!(warm_progress[0].stage, FemPipelineStage::Publish);
        assert!(warm_progress[0].detail.contains("mesh and solve skipped"));

        let mut concurrent_a = make_request("singleflight-a");
        concurrent_a.control.relative_solver_tolerance = 5.0e-9;
        let mut concurrent_b = make_request("singleflight-b");
        concurrent_b.control.relative_solver_tolerance = 5.0e-9;
        let resolver_a = resolver.clone();
        let resolver_b = resolver.clone();
        let first = std::thread::spawn(move || {
            let mut progress = Vec::new();
            let result = run_fem_study_with_resolver(
                concurrent_a,
                &resolver_a,
                &AtomicBool::new(false),
                |event| progress.push(event),
            );
            (result, progress)
        });
        let second = std::thread::spawn(move || {
            let mut progress = Vec::new();
            let result = run_fem_study_with_resolver(
                concurrent_b,
                &resolver_b,
                &AtomicBool::new(false),
                |event| progress.push(event),
            );
            (result, progress)
        });
        let (first_result, first_progress) = first.join().expect("first singleflight thread");
        let (second_result, second_progress) = second.join().expect("second singleflight thread");
        let first_result = first_result.expect("first singleflight result");
        let second_result = second_result.expect("second singleflight result");
        assert_eq!(first_result.solution_digest, second_result.solution_digest);
        let combined = first_progress
            .iter()
            .chain(&second_progress)
            .collect::<Vec<_>>();
        assert_eq!(
            combined
                .iter()
                .filter(|event| event.stage == FemPipelineStage::VolumeMesh)
                .count(),
            1
        );
        assert_eq!(
            combined
                .iter()
                .filter(|event| event.stage == FemPipelineStage::Solve)
                .count(),
            1
        );
        assert_eq!(
            combined
                .iter()
                .filter(|event| event.detail.contains("mesh and solve skipped"))
                .count(),
            1
        );
        fs::remove_dir_all(root).expect("cleanup cache fixture");
    }
}
