//! Filesystem project mirror: exposes one thread's active macro as a plain
//! folder (`model.ecky` + `ecky-project.json`) so external editors and LLM
//! file skills can author source directly, while threads/versions remain the
//! canonical record. See `openspec/changes/filesystem-project-mirror`.
//!
//! The folder is a mirror, never an alternate database: edits re-enter the
//! app only through compile -> preview -> commit (wired in `mcp::handlers`).

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use specta::Type;

use crate::component_package_runtime::{
    compute_package_payload_digest, encode_validated_payload_archive, payload_store_dir,
    read_payload_inventory, validate_payload_archive, ValidatedPayload, ValidatedPayloadEntry,
};
use crate::contracts::{
    AppError, AppResult, ComponentDependencyLock, PackagePayloadInventoryEntry,
};
use crate::models::PathResolver;

pub const PROJECT_SOURCE_FILE_NAME: &str = "model.ecky";
pub const PROJECT_MANIFEST_FILE_NAME: &str = "ecky-project.json";
/// Canonical dependency lock mirrored beside `model.ecky` for a live-reference
/// version. It is never inferred from installed packages during apply.
pub const PROJECT_LOCK_FILE_NAME: &str = "ecky.lock.json";
const PORTABLE_DEPENDENCIES_DIR_NAME: &str = "dependencies";
const PROJECT_MANIFEST_SCHEMA_VERSION: u32 = 1;
const PROJECTS_DIR_NAME: &str = "projects";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ProjectManifest {
    pub schema_version: u32,
    pub project_id: String,
    pub thread_id: String,
    pub message_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,
    /// Digest of the `model.ecky` bytes Ecky last wrote or applied. The only
    /// thing distinguishing "user edited the file" from "clean".
    pub source_digest: String,
    pub exported_at: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum ProjectSyncState {
    /// No `model.ecky` or no manifest in the folder.
    Missing,
    /// File matches the manifest digest and the thread head is still the
    /// bound message.
    Clean,
    /// File was edited externally; thread head unchanged. Safe to apply.
    FileChanged,
    /// Thread gained versions past the binding; folder is stale. Re-export.
    ThreadAdvanced,
    /// Both sides moved. Applying requires an explicit force.
    Conflict,
}

#[derive(Clone, Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ProjectFolderStatus {
    pub state: ProjectSyncState,
    pub folder: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manifest: Option<ProjectManifest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_digest: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thread_head_message_id: Option<String>,
}

pub fn source_digest(source: &str) -> String {
    format!("sha256:{:x}", Sha256::digest(source.as_bytes()))
}

/// Projects root: the configured `projectsRoot` override when set to a
/// non-blank path, otherwise `<app_data>/projects`.
pub fn projects_root(app: &dyn PathResolver, configured: Option<&str>) -> PathBuf {
    match configured {
        Some(path) if !path.trim().is_empty() => PathBuf::from(path.trim()),
        _ => app.app_data_dir().join(PROJECTS_DIR_NAME),
    }
}

/// Deterministic folder slug: human prefix from the title plus a stable
/// thread-id suffix so renames and collisions cannot cross-wire folders.
pub fn project_slug(title: &str, thread_id: &str) -> String {
    let mut prefix: String = title
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    prefix.truncate(40);
    let suffix: String = thread_id
        .chars()
        .rev()
        .filter(|c| c.is_ascii_alphanumeric())
        .take(8)
        .collect::<String>()
        .chars()
        .rev()
        .collect();
    if prefix.is_empty() {
        format!("project-{suffix}")
    } else {
        format!("{prefix}-{suffix}")
    }
}

pub fn project_dir(
    app: &dyn PathResolver,
    configured_root: Option<&str>,
    slug: &str,
) -> AppResult<PathBuf> {
    if slug.is_empty()
        || !slug
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(AppError::validation(format!(
            "Project slug `{slug}` is not a safe directory name."
        )));
    }
    let root = projects_root(app, configured_root);
    if !root.is_absolute() {
        return Err(AppError::validation(format!(
            "Projects root '{}' must be an absolute path.",
            root.display()
        )));
    }
    if root.exists() && !root.is_dir() {
        return Err(AppError::validation(format!(
            "Projects root '{}' must be a directory.",
            root.display()
        )));
    }
    Ok(root.join(slug))
}

pub fn read_manifest(dir: &Path) -> AppResult<Option<ProjectManifest>> {
    let path = dir.join(PROJECT_MANIFEST_FILE_NAME);
    if !path.is_file() {
        return Ok(None);
    }
    let raw = fs::read_to_string(&path).map_err(|err| {
        AppError::persistence(format!("Failed to read '{}': {}", path.display(), err))
    })?;
    serde_json::from_str(&raw)
        .map(Some)
        .map_err(|err| AppError::persistence(format!("Invalid '{}': {}", path.display(), err)))
}

/// Atomically replace `dest` with `bytes`: stage a unique temp file in the
/// same directory, `write_all` + `sync_all`, then `rename` it into place as
/// the final step. Readers only ever observe the previous complete file or
/// the new complete one, never a partially-written file, and a failed write
/// never truncates the existing destination. On any error after the temp
/// exists it is removed so no residue lingers. When `dest` already exists,
/// its Unix mode is copied onto the temp so external permissions survive
/// re-exports. `rename` stays within one directory, so it is a single atomic
/// entry swap on POSIX and on same-volume Windows.
///
/// This is the source-write primitive required by the thread-source-binding
/// spec (`atomically refresh a clean bound source file`); `export_project`
/// and `write_manifest` route both mirror files through it.
fn atomic_write(dest: &Path, bytes: &[u8]) -> AppResult<()> {
    let parent = dest.parent().ok_or_else(|| {
        AppError::persistence(format!(
            "Cannot atomic-write '{}': no parent directory",
            dest.display()
        ))
    })?;
    let file_name = dest.file_name().ok_or_else(|| {
        AppError::persistence(format!(
            "Cannot atomic-write '{}': not a file path",
            dest.display()
        ))
    })?;
    // Unique temp name beside the destination. The `.atomic-` infix is what
    // residue checks key on, so it stays stable.
    let temp_path = parent.join(format!(
        "{}.atomic-{}",
        file_name.to_string_lossy(),
        uuid::Uuid::new_v4().simple()
    ));

    // Stage the temp first. `create_new` refuses to clobber a stray temp left
    // by a crashed prior run; we still clean up our own on any failure below.
    let mut file = match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp_path)
    {
        Ok(file) => file,
        Err(err) => {
            return Err(AppError::persistence(format!(
                "Failed to stage temp for '{}': {}",
                dest.display(),
                err
            )));
        }
    };
    // Best-effort: copy the existing destination's mode so an externally
    // chmod'd source/manifest keeps its permissions across re-exports.
    copy_existing_permissions(dest, &file);

    if let Err(err) = file.write_all(bytes) {
        let _ = fs::remove_file(&temp_path);
        return Err(AppError::persistence(format!(
            "Failed to write '{}': {}",
            temp_path.display(),
            err
        )));
    }
    if let Err(err) = file.sync_all() {
        let _ = fs::remove_file(&temp_path);
        return Err(AppError::persistence(format!(
            "Failed to fsync '{}': {}",
            temp_path.display(),
            err
        )));
    }
    // Release the handle before rename (Windows requires no open handles).
    drop(file);

    if let Err(err) = publish_atomic(&temp_path, dest) {
        let _ = fs::remove_file(&temp_path);
        return Err(AppError::persistence(format!(
            "Failed to publish '{}': {}",
            dest.display(),
            err
        )));
    }
    Ok(())
}

/// Persist the user-visible canonical source without mutating history or the
/// last-applied manifest digest. The next render/commit decides whether those
/// derived records advance.
pub(crate) fn write_bound_source(path: &Path, source: &str) -> AppResult<()> {
    atomic_write(path, source.as_bytes())
}

#[cfg(not(windows))]
fn publish_atomic(temp_path: &Path, dest: &Path) -> std::io::Result<()> {
    fs::rename(temp_path, dest)
}

#[cfg(windows)]
fn publish_atomic(temp_path: &Path, dest: &Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt;

    const MOVEFILE_REPLACE_EXISTING: u32 = 0x1;
    const MOVEFILE_WRITE_THROUGH: u32 = 0x8;

    extern "system" {
        fn MoveFileExW(
            existing_file_name: *const u16,
            new_file_name: *const u16,
            flags: u32,
        ) -> i32;
    }

    let source: Vec<u16> = temp_path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let destination: Vec<u16> = dest
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let moved = unsafe {
        MoveFileExW(
            source.as_ptr(),
            destination.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if moved == 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

/// Copies the existing destination's permissions onto a staged temp file so
/// re-exports do not reset an externally-chosen file mode. Best-effort: a
/// missing destination (first write) or any failure is ignored.
#[cfg(unix)]
fn copy_existing_permissions(dest: &Path, temp: &fs::File) {
    if let Ok(existing) = fs::metadata(dest) {
        let _ = temp.set_permissions(existing.permissions());
    }
}

#[cfg(not(unix))]
fn copy_existing_permissions(_dest: &Path, _temp: &fs::File) {}

pub fn write_manifest(dir: &Path, manifest: &ProjectManifest) -> AppResult<()> {
    let path = dir.join(PROJECT_MANIFEST_FILE_NAME);
    let json = serde_json::to_string_pretty(manifest)
        .map_err(|err| AppError::internal(format!("Failed to serialize manifest: {err}")))?;
    atomic_write(&path, json.as_bytes())
}

/// Write the canonical dependency lock as the exact compact JSON bytes that
/// define its digest. The lock is a version input, never a mutable discovery
/// record, so callers must supply it from the version artifact bundle.
pub fn write_project_lock(dir: &Path, lock: &ComponentDependencyLock) -> AppResult<()> {
    lock.validate()?;
    let bytes = lock.clone().canonical().canonical_bytes()?;
    atomic_write(&dir.join(PROJECT_LOCK_FILE_NAME), &bytes)
}

/// Read a project lock for project apply. Callers pass this value as
/// `expected_lock` to host pre-resolution; absence means the project has no
/// live package dependencies.
pub fn read_project_lock(dir: &Path) -> AppResult<Option<ComponentDependencyLock>> {
    let path = dir.join(PROJECT_LOCK_FILE_NAME);
    if !path.is_file() {
        return Ok(None);
    }
    let bytes = fs::read(&path).map_err(|err| {
        AppError::persistence(format!("Failed to read '{}': {}", path.display(), err))
    })?;
    let lock: ComponentDependencyLock = serde_json::from_slice(&bytes)
        .map_err(|err| AppError::persistence(format!("Invalid '{}': {}", path.display(), err)))?;
    lock.validate()?;
    let canonical = lock.clone().canonical();
    if bytes != canonical.canonical_bytes()? {
        return Err(AppError::validation(format!(
            "Project lock '{}' is not canonical; re-export the committed version instead of rewriting its dependency lock.",
            path.display()
        )));
    }
    Ok(Some(canonical))
}

/// Storage boundary used by portable project export/import. The project-mirror
/// layer verifies raw payload bytes against the lock before publication; the
/// application-owned implementation may then publish the validated payload to
/// the global component CAS without creating a per-model dependency tree.
pub trait PortablePayloadStorage {
    fn read_payload(&self, package_digest: &str) -> AppResult<Vec<u8>>;
    fn publish_validated_payload(
        &self,
        package_digest: &str,
        payload: &ValidatedPayload,
        inventory: Vec<PackagePayloadInventoryEntry>,
    ) -> AppResult<()>;
}

pub struct GlobalComponentPayloadStorage<'a> {
    pub app: &'a dyn PathResolver,
}

impl PortablePayloadStorage for GlobalComponentPayloadStorage<'_> {
    fn read_payload(&self, package_digest: &str) -> AppResult<Vec<u8>> {
        let store_dir = payload_store_dir(self.app, package_digest)?;
        let inventory = read_payload_inventory(&store_dir)?;
        if inventory.package_digest != package_digest {
            return Err(AppError::validation(format!(
                "Stored package inventory digest '{}' differs from requested portable digest '{}'.",
                inventory.package_digest, package_digest
            )));
        }
        let mut entries = Vec::with_capacity(inventory.entries.len());
        for item in inventory.entries {
            entries.push(ValidatedPayloadEntry {
                path: item.path.clone(),
                content: fs::read(store_dir.join(&item.path)).map_err(|error| {
                    AppError::persistence(format!(
                        "Failed to read stored package payload '{}': {}",
                        item.path, error
                    ))
                })?,
            });
        }
        encode_validated_payload_archive(&ValidatedPayload { entries })
    }

    fn publish_validated_payload(
        &self,
        package_digest: &str,
        payload: &ValidatedPayload,
        inventory: Vec<PackagePayloadInventoryEntry>,
    ) -> AppResult<()> {
        crate::component_package_runtime::publish_validated_payload(
            self.app,
            payload,
            package_digest,
            inventory,
        )
        .map(|_| ())
    }
}

fn dependency_payload_digests(lock: &ComponentDependencyLock) -> AppResult<Vec<String>> {
    lock.validate()?;
    let canonical = lock.clone().canonical();
    let mut digests = Vec::with_capacity(canonical.dependencies.len());
    for dependency in canonical.dependencies {
        if dependency.package_digest.trim().is_empty() {
            return Err(AppError::validation(format!(
                "Dependency lock entry '{}@{}' has an empty package digest.",
                dependency.package_id, dependency.version
            )));
        }
        digests.push(dependency.package_digest);
    }
    digests.sort();
    digests.dedup();
    Ok(digests)
}

fn portable_payload_path(dir: &Path, package_digest: &str) -> AppResult<PathBuf> {
    let hex = package_digest.strip_prefix("sha256:").ok_or_else(|| {
        AppError::validation(format!(
            "Package payload digest '{package_digest}' must start with 'sha256:'."
        ))
    })?;
    if hex.is_empty() || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(AppError::validation(format!(
            "Package payload digest '{package_digest}' is not a safe SHA-256 digest."
        )));
    }
    Ok(dir
        .join(PORTABLE_DEPENDENCIES_DIR_NAME)
        .join(format!("{hex}.eckypkg")))
}

/// Explicit portable export writes verified immutable *inner payload* archives
/// keyed by digest. It never mutates or re-resolves `lock`.
pub fn export_portable_dependencies(
    dir: &Path,
    lock: &ComponentDependencyLock,
    storage: &dyn PortablePayloadStorage,
) -> AppResult<()> {
    let digests = dependency_payload_digests(lock)?;
    let mut verified = Vec::with_capacity(digests.len());
    for digest in &digests {
        let bytes = storage.read_payload(digest)?;
        let payload = validate_payload_archive(&bytes)?;
        let (actual, _) = compute_package_payload_digest(&payload);
        if &actual != digest {
            return Err(AppError::validation(format!(
                "Portable export payload digest '{}' does not match locked digest '{}'.",
                actual, digest
            )));
        }
        verified.push((digest, bytes));
    }
    for (digest, bytes) in verified {
        let path = portable_payload_path(dir, digest)?;
        let parent = path.parent().expect("portable payload has parent");
        fs::create_dir_all(parent).map_err(|err| {
            AppError::persistence(format!("Failed to create '{}': {}", parent.display(), err))
        })?;
        atomic_write(&path, &bytes)?;
    }
    Ok(())
}

/// Import portable dependency payloads only after every archive verifies to
/// the immutable project lock. Invalid input leaves the storage seam untouched.
pub fn import_portable_dependencies(
    dir: &Path,
    lock: &ComponentDependencyLock,
    storage: &dyn PortablePayloadStorage,
) -> AppResult<()> {
    let digests = dependency_payload_digests(lock)?;
    let mut verified = Vec::with_capacity(digests.len());
    for digest in &digests {
        let path = portable_payload_path(dir, digest)?;
        let bytes = fs::read(&path).map_err(|err| {
            AppError::persistence(format!(
                "Failed to read portable dependency '{}': {}",
                path.display(),
                err
            ))
        })?;
        let payload = validate_payload_archive(&bytes)?;
        let (actual, inventory) = compute_package_payload_digest(&payload);
        if &actual != digest {
            return Err(AppError::validation(format!(
                "Portable dependency '{}' digest '{}' does not match locked digest '{}'.",
                path.display(),
                actual,
                digest
            )));
        }
        verified.push((digest, payload, inventory));
    }
    for (digest, payload, inventory) in verified {
        storage.publish_validated_payload(digest, &payload, inventory)?;
    }
    Ok(())
}

pub fn read_project_source(dir: &Path) -> AppResult<Option<String>> {
    let path = dir.join(PROJECT_SOURCE_FILE_NAME);
    if !path.is_file() {
        return Ok(None);
    }
    fs::read_to_string(&path).map(Some).map_err(|err| {
        AppError::persistence(format!("Failed to read '{}': {}", path.display(), err))
    })
}

/// Project-apply seam: read the source and the canonical expected lock from
/// one mirror folder. The caller supplies `expected_lock.as_ref()` to
/// `ResolveAuthoringSourceRequest`; it must never replace this value with the
/// currently installed coordinate index.
pub fn read_project_apply_input(
    dir: &Path,
) -> AppResult<Option<(String, Option<ComponentDependencyLock>)>> {
    let Some(source) = read_project_source(dir)? else {
        return Ok(None);
    };
    Ok(Some((source, read_project_lock(dir)?)))
}

pub struct ExportProjectRequest<'a> {
    pub slug: &'a str,
    pub thread_id: &'a str,
    pub message_id: &'a str,
    pub model_id: Option<&'a str>,
    pub source: &'a str,
    /// Configured `projectsRoot` override; `None` uses `<app_data>/projects`.
    pub projects_root: Option<&'a str>,
}

/// Writes/refreshes the mirror folder from a bound version. Keeps the
/// existing projectId across re-exports so external references stay valid.
pub fn export_project(
    app: &dyn PathResolver,
    request: &ExportProjectRequest<'_>,
) -> AppResult<(PathBuf, ProjectManifest)> {
    let dir = project_dir(app, request.projects_root, request.slug)?;
    fs::create_dir_all(&dir).map_err(|err| {
        AppError::persistence(format!("Failed to create '{}': {}", dir.display(), err))
    })?;
    let project_id = read_manifest(&dir)?
        .map(|existing| existing.project_id)
        .unwrap_or_else(|| format!("proj-{}", uuid::Uuid::new_v4()));

    let source_path = dir.join(PROJECT_SOURCE_FILE_NAME);
    atomic_write(&source_path, request.source.as_bytes())?;

    let manifest = ProjectManifest {
        schema_version: PROJECT_MANIFEST_SCHEMA_VERSION,
        project_id,
        thread_id: request.thread_id.to_string(),
        message_id: request.message_id.to_string(),
        model_id: request.model_id.map(str::to_string),
        source_digest: source_digest(request.source),
        exported_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|elapsed| elapsed.as_secs())
            .unwrap_or(0),
    };
    write_manifest(&dir, &manifest)?;
    Ok((dir, manifest))
}

/// Export a version source and its lock together. This is the project-mirror
/// integration seam for version handlers: it writes `ecky.lock.json` from the
/// committed bundle lock, and removes a stale lock when exporting a vendored
/// (lock-free) version.
pub fn export_project_with_lock(
    app: &dyn PathResolver,
    request: &ExportProjectRequest<'_>,
    lock: Option<&ComponentDependencyLock>,
) -> AppResult<(PathBuf, ProjectManifest)> {
    let (dir, manifest) = export_project(app, request)?;
    let lock_path = dir.join(PROJECT_LOCK_FILE_NAME);
    match lock {
        Some(lock) => write_project_lock(&dir, lock)?,
        None if lock_path.exists() => fs::remove_file(&lock_path).map_err(|err| {
            AppError::persistence(format!(
                "Failed to remove stale project lock '{}': {}",
                lock_path.display(),
                err
            ))
        })?,
        None => {}
    }
    Ok((dir, manifest))
}

/// Explicit portable mode: mirror source + canonical lock, then vendor each
/// immutable locked payload by digest. Normal project export never copies
/// dependency trees.
pub fn export_portable_project(
    app: &dyn PathResolver,
    request: &ExportProjectRequest<'_>,
    lock: &ComponentDependencyLock,
) -> AppResult<(PathBuf, ProjectManifest)> {
    let (dir, manifest) = export_project_with_lock(app, request, Some(lock))?;
    export_portable_dependencies(&dir, lock, &GlobalComponentPayloadStorage { app })?;
    Ok((dir, manifest))
}

/// Portable import verifies every vendored payload before publishing anything
/// to the global CAS, then returns authored source plus the unchanged lock.
pub fn import_portable_project(
    app: &dyn PathResolver,
    dir: &Path,
) -> AppResult<(String, ComponentDependencyLock)> {
    let (source, lock) = read_project_apply_input(dir)?.ok_or_else(|| {
        AppError::validation(format!(
            "Portable project '{}' has no model.ecky.",
            dir.display()
        ))
    })?;
    let lock = lock.ok_or_else(|| {
        AppError::validation(format!(
            "Portable project '{}' has no ecky.lock.json.",
            dir.display()
        ))
    })?;
    import_portable_dependencies(dir, &lock, &GlobalComponentPayloadStorage { app })?;
    Ok((source, lock))
}

/// Pure classification over (file digest, manifest, thread head).
pub fn classify_sync_state(
    file_digest: Option<&str>,
    manifest: Option<&ProjectManifest>,
    thread_head_message_id: Option<&str>,
) -> ProjectSyncState {
    let (Some(file_digest), Some(manifest)) = (file_digest, manifest) else {
        return ProjectSyncState::Missing;
    };
    let file_changed = file_digest != manifest.source_digest;
    let thread_advanced = thread_head_message_id.is_some_and(|head| head != manifest.message_id);
    match (file_changed, thread_advanced) {
        (false, false) => ProjectSyncState::Clean,
        (true, false) => ProjectSyncState::FileChanged,
        (false, true) => ProjectSyncState::ThreadAdvanced,
        (true, true) => ProjectSyncState::Conflict,
    }
}

/// Read-only folder status; thread head is supplied by the caller (handlers
/// own history lookups).
pub fn folder_status(
    app: &dyn PathResolver,
    configured_root: Option<&str>,
    slug: &str,
    thread_head_message_id: Option<&str>,
) -> AppResult<ProjectFolderStatus> {
    let dir = project_dir(app, configured_root, slug)?;
    let manifest = read_manifest(&dir)?;
    let file_digest = read_project_source(&dir)?.map(|source| source_digest(&source));
    let state = classify_sync_state(
        file_digest.as_deref(),
        manifest.as_ref(),
        thread_head_message_id,
    );
    Ok(ProjectFolderStatus {
        state,
        folder: dir.to_string_lossy().to_string(),
        manifest,
        file_digest,
        thread_head_message_id: thread_head_message_id.map(str::to_string),
    })
}

/// Slugs of all project folders under the projects root that look like
/// mirrors (have a manifest). Used by the folder watcher.
pub fn list_project_slugs(
    app: &dyn PathResolver,
    configured_root: Option<&str>,
) -> AppResult<Vec<String>> {
    let root = projects_root(app, configured_root);
    if !root.is_dir() {
        return Ok(Vec::new());
    }
    let mut slugs = Vec::new();
    let entries = fs::read_dir(&root).map_err(|err| {
        AppError::persistence(format!("Failed to read '{}': {}", root.display(), err))
    })?;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() || !path.join(PROJECT_MANIFEST_FILE_NAME).is_file() {
            continue;
        }
        if let Some(name) = path.file_name().and_then(|name| name.to_str()) {
            slugs.push(name.to_string());
        }
    }
    slugs.sort();
    Ok(slugs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TestResolver {
        root: PathBuf,
    }

    impl PathResolver for TestResolver {
        fn app_config_dir(&self) -> PathBuf {
            self.root.clone()
        }

        fn app_data_dir(&self) -> PathBuf {
            self.root.clone()
        }

        fn resource_path(&self, _path: &str) -> Option<PathBuf> {
            None
        }
    }

    fn temp_resolver(name: &str) -> TestResolver {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        TestResolver {
            root: std::env::temp_dir().join(format!("ecky-project-mirror-{name}-{nonce}")),
        }
    }

    fn sample_request<'a>(slug: &'a str, source: &'a str) -> ExportProjectRequest<'a> {
        ExportProjectRequest {
            slug,
            thread_id: "thread-1",
            message_id: "msg-1",
            model_id: Some("model-1"),
            source,
            projects_root: None,
        }
    }

    #[test]
    fn projects_root_honors_config_override_else_defaults() {
        let resolver = temp_resolver("root");
        let default_root = resolver.app_data_dir().join(PROJECTS_DIR_NAME);

        // No override -> `<app_data>/projects`.
        assert_eq!(projects_root(&resolver, None), default_root);

        // Configured override wins.
        let custom = std::env::temp_dir().join("ecky-custom-projects-root");
        assert_eq!(
            projects_root(&resolver, Some(custom.to_str().unwrap())),
            custom
        );

        // Blank/whitespace override is ignored, falling back to the default.
        assert_eq!(projects_root(&resolver, Some("   ")), default_root);
    }

    #[test]
    fn project_dir_rejects_relative_or_non_directory_projects_root() {
        let resolver = temp_resolver("invalid-root");
        let relative =
            project_dir(&resolver, Some("relative/projects"), "bracket-abc123").unwrap_err();
        assert!(
            relative.message.contains("absolute"),
            "{}",
            relative.message
        );

        fs::create_dir_all(&resolver.root).unwrap();
        let file_root = resolver.root.join("not-a-directory");
        fs::write(&file_root, "occupied").unwrap();
        let not_directory =
            project_dir(&resolver, file_root.to_str(), "bracket-abc123").unwrap_err();
        assert!(
            not_directory.message.contains("directory"),
            "{}",
            not_directory.message
        );
    }

    #[test]
    fn export_writes_source_and_manifest_round_trip() {
        let resolver = temp_resolver("export");
        let source = "(model (part body (box 1 2 3)))";

        let (dir, manifest) =
            export_project(&resolver, &sample_request("bracket-abc123", source)).expect("export");

        assert_eq!(
            fs::read_to_string(dir.join(PROJECT_SOURCE_FILE_NAME)).expect("source"),
            source
        );
        let reread = read_manifest(&dir).expect("read").expect("manifest");
        assert_eq!(reread, manifest);
        assert_eq!(reread.schema_version, 1);
        assert_eq!(reread.thread_id, "thread-1");
        assert_eq!(reread.source_digest, source_digest(source));
        assert!(reread.project_id.starts_with("proj-"));

        let raw = fs::read_to_string(dir.join(PROJECT_MANIFEST_FILE_NAME)).expect("raw");
        assert!(
            raw.contains("\"sourceDigest\""),
            "camelCase manifest: {raw}"
        );
        assert!(raw.contains("\"threadId\""), "camelCase manifest: {raw}");
    }

    #[test]
    fn project_lock_round_trips_canonical_bytes() {
        use crate::contracts::{
            ComponentDependencyLockComponent, ComponentDependencyLockEntry,
            COMPONENT_DEPENDENCY_LOCK_SCHEMA_VERSION,
        };

        let resolver = temp_resolver("lock");
        let (dir, _) = export_project(
            &resolver,
            &sample_request("lock-abc123", "(model (part body (box 1 1 1)))"),
        )
        .expect("project export");
        let lock = ComponentDependencyLock {
            schema_version: COMPONENT_DEPENDENCY_LOCK_SCHEMA_VERSION,
            dependencies: vec![ComponentDependencyLockEntry {
                package_id: "bike.kit".to_string(),
                version: "1.2.0".to_string(),
                package_digest: "sha256:aaaa".to_string(),
                components: vec![ComponentDependencyLockComponent {
                    component_id: "cage".to_string(),
                    entry_symbol: Some("cage-v2".to_string()),
                    payload_digest: "sha256:aaaa".to_string(),
                    payload_kind: None,
                    geometry_representation: None,
                }],
            }],
        };

        write_project_lock(&dir, &lock).expect("write canonical lock");
        let raw = fs::read(dir.join(PROJECT_LOCK_FILE_NAME)).expect("lock bytes");
        assert_eq!(
            raw,
            lock.clone()
                .canonical()
                .canonical_bytes()
                .expect("canonical bytes")
        );
        assert_eq!(
            read_project_lock(&dir).expect("read lock"),
            Some(lock.clone().canonical())
        );
        let (_, expected_lock) = read_project_apply_input(&dir)
            .expect("apply input")
            .expect("source present");
        assert_eq!(expected_lock, Some(lock.canonical()));
    }

    #[test]
    fn portable_dependencies_verify_before_storage_publication() {
        use std::cell::RefCell;
        use std::collections::BTreeMap;
        use std::io::Cursor;
        use zip::write::FileOptions;
        use zip::{CompressionMethod, ZipWriter};

        #[derive(Default)]
        struct MemoryStorage {
            payloads: BTreeMap<String, Vec<u8>>,
            published: RefCell<Vec<String>>,
        }

        impl PortablePayloadStorage for MemoryStorage {
            fn read_payload(&self, package_digest: &str) -> AppResult<Vec<u8>> {
                self.payloads.get(package_digest).cloned().ok_or_else(|| {
                    AppError::not_found(format!("missing test payload {package_digest}"))
                })
            }

            fn publish_validated_payload(
                &self,
                package_digest: &str,
                _payload: &ValidatedPayload,
                _inventory: Vec<PackagePayloadInventoryEntry>,
            ) -> AppResult<()> {
                self.published.borrow_mut().push(package_digest.to_string());
                Ok(())
            }
        }

        fn payload_bytes(body: &[u8]) -> Vec<u8> {
            let cursor = Cursor::new(Vec::new());
            let mut zip = ZipWriter::new(cursor);
            zip.start_file(
                "component.ecky",
                FileOptions::default().compression_method(CompressionMethod::Stored),
            )
            .expect("entry");
            zip.write_all(body).expect("body");
            zip.finish().expect("finish").into_inner()
        }

        let resolver = temp_resolver("portable");
        let (dir, _) = export_project(
            &resolver,
            &sample_request("portable-abc123", "(model (part body (box 1 1 1)))"),
        )
        .expect("project export");
        let payload_bytes = payload_bytes(b"(define-component cage () (box 1 1 1))");
        let payload = validate_payload_archive(&payload_bytes).expect("valid payload");
        let (digest, _) = compute_package_payload_digest(&payload);
        let lock = ComponentDependencyLock {
            schema_version: 1,
            dependencies: vec![crate::contracts::ComponentDependencyLockEntry {
                package_id: "bike.kit".to_string(),
                version: "1.2.0".to_string(),
                package_digest: digest.clone(),
                components: vec![crate::contracts::ComponentDependencyLockComponent {
                    component_id: "cage".to_string(),
                    entry_symbol: None,
                    payload_digest: digest.clone(),
                    payload_kind: None,
                    geometry_representation: None,
                }],
            }],
        };
        let mut exporter = MemoryStorage::default();
        exporter.payloads.insert(digest.clone(), payload_bytes);
        export_portable_dependencies(&dir, &lock, &exporter).expect("portable export");
        assert!(portable_payload_path(&dir, &digest)
            .expect("path")
            .is_file());

        let importer = MemoryStorage::default();
        import_portable_dependencies(&dir, &lock, &importer).expect("portable import");
        assert_eq!(importer.published.borrow().as_slice(), [digest.as_str()]);

        fs::write(
            portable_payload_path(&dir, &digest).expect("path"),
            b"not a zip",
        )
        .expect("corrupt portable payload");
        let err = import_portable_dependencies(&dir, &lock, &importer)
            .expect_err("corrupt payload must not publish");
        assert!(
            err.message.contains("parse package payload"),
            "{}",
            err.message
        );
        assert_eq!(
            importer.published.borrow().as_slice(),
            [digest.as_str()],
            "failed verification must not reach the storage seam"
        );
    }

    #[test]
    fn portable_project_round_trips_between_real_global_cas_instances() {
        let source_resolver = temp_resolver("portable-global-source");
        let target_resolver = temp_resolver("portable-global-target");
        let payload = ValidatedPayload {
            entries: vec![
                ValidatedPayloadEntry {
                    path: "components/cage.ecky".to_string(),
                    content: b"(define-component cage () (box 1 2 3))".to_vec(),
                },
                ValidatedPayloadEntry {
                    path: crate::component_package_runtime::COMPONENT_PACKAGE_FILE_NAME.to_string(),
                    content: serde_json::to_vec(&serde_json::json!({
                        "schemaVersion": 1,
                        "packageId": "fixture.portable",
                        "version": "1.0.0",
                        "displayName": "Portable fixture",
                        "visibility": "source",
                        "components": [{
                            "componentId": "cage",
                            "version": "1.0.0",
                            "displayName": "Cage",
                            "sourceRef": "components/cage.ecky",
                            "entrySymbol": "cage"
                        }]
                    }))
                    .expect("manifest bytes"),
                },
            ],
        };
        let (digest, inventory) = compute_package_payload_digest(&payload);
        let source_storage = GlobalComponentPayloadStorage {
            app: &source_resolver,
        };
        source_storage
            .publish_validated_payload(&digest, &payload, inventory)
            .expect("seed source CAS");
        let lock = ComponentDependencyLock {
            schema_version: crate::contracts::COMPONENT_DEPENDENCY_LOCK_SCHEMA_VERSION,
            dependencies: vec![crate::contracts::ComponentDependencyLockEntry {
                package_id: "fixture.portable".to_string(),
                version: "1.0.0".to_string(),
                package_digest: digest.clone(),
                components: vec![crate::contracts::ComponentDependencyLockComponent {
                    component_id: "cage".to_string(),
                    entry_symbol: Some("cage".to_string()),
                    payload_digest: digest.clone(),
                    payload_kind: Some(crate::contracts::ComponentPayloadKind::Source),
                    geometry_representation: None,
                }],
            }],
        };
        let (dir, _) = export_project_with_lock(
            &source_resolver,
            &sample_request(
                "portable-global-abc123",
                r#"(import-component "fixture.portable" :version "1.0.0" :component "cage" :as cage)"#,
            ),
            Some(&lock),
        )
        .expect("project export");
        export_portable_dependencies(&dir, &lock, &source_storage)
            .expect("portable dependency export");

        let target_storage = GlobalComponentPayloadStorage {
            app: &target_resolver,
        };
        import_portable_dependencies(&dir, &lock, &target_storage)
            .expect("verified target CAS import");
        let imported_bytes = target_storage
            .read_payload(&digest)
            .expect("target payload");
        let imported = validate_payload_archive(&imported_bytes).expect("target payload validates");
        assert_eq!(compute_package_payload_digest(&imported).0, digest);
        assert_eq!(
            read_project_lock(&dir).expect("project lock"),
            Some(lock.canonical())
        );

        fs::remove_dir_all(source_resolver.root).ok();
        fs::remove_dir_all(target_resolver.root).ok();
    }

    #[test]
    fn re_export_keeps_project_id_and_rebases_digest() {
        let resolver = temp_resolver("reexport");
        let (_, first) = export_project(
            &resolver,
            &sample_request("kit-abc123", "(model (part a (box 1 1 1)))"),
        )
        .expect("export");
        let mut second_request = sample_request("kit-abc123", "(model (part a (box 2 2 2)))");
        second_request.message_id = "msg-2";
        let (_, second) = export_project(&resolver, &second_request).expect("re-export");

        assert_eq!(first.project_id, second.project_id);
        assert_eq!(second.message_id, "msg-2");
        assert_ne!(first.source_digest, second.source_digest);
    }

    #[test]
    fn export_replacement_leaves_no_atomic_temp_residue() {
        let resolver = temp_resolver("atomic-residue");
        // Two exports back-to-back exercise the replacement path: the staged
        // temp must be renamed away each time, never left beside the file.
        export_project(
            &resolver,
            &sample_request("residue-abc12345", "(model (part a (box 1 1 1)))"),
        )
        .expect("first export");
        let (dir, _) = export_project(
            &resolver,
            &sample_request("residue-abc12345", "(model (part a (box 2 2 2)))"),
        )
        .expect("re-export replaces source");

        let source = fs::read_to_string(dir.join(PROJECT_SOURCE_FILE_NAME)).expect("source");
        assert_eq!(source, "(model (part a (box 2 2 2)))");

        let residue: Vec<_> = fs::read_dir(&dir)
            .expect("read dir")
            .flatten()
            .map(|entry| entry.file_name())
            .filter(|name| name.to_string_lossy().contains("atomic"))
            .collect();
        assert!(
            residue.is_empty(),
            "atomic temp residue left behind: {residue:?}"
        );
        // The mirror folder holds exactly the two canonical files.
        let mut names: Vec<_> = fs::read_dir(&dir)
            .expect("read dir")
            .flatten()
            .map(|entry| entry.file_name().to_string_lossy().to_string())
            .collect();
        names.sort();
        assert_eq!(
            names,
            vec![PROJECT_MANIFEST_FILE_NAME, PROJECT_SOURCE_FILE_NAME],
            "mirror folder must hold only the source and manifest"
        );
    }

    #[cfg(unix)]
    #[test]
    fn export_failure_leaves_existing_source_intact() {
        use std::os::unix::fs::PermissionsExt;

        // A failed mirror write must never truncate or replace an existing
        // source file. Only a temp+rename write has this property:
        // truncate-in-place (fs::write) opens the existing file and destroys
        // its bytes before any later step can fail.
        let resolver = temp_resolver("atomic-fail");
        let dir = projects_root(&resolver, None).join("keep-abc12345");
        fs::create_dir_all(&dir).expect("mkdir");
        let source_path = dir.join(PROJECT_SOURCE_FILE_NAME);
        fs::write(&source_path, "ORIGINAL").expect("seed source");

        // Read-only folder: a temp+rename write cannot stage its temp, so it
        // fails before touching the destination. A truncate-in-place write,
        // by contrast, still opens the existing (writable) file and clobbers
        // it even though the directory is not writable.
        fs::set_permissions(&dir, fs::Permissions::from_mode(0o500)).expect("chmod r-x");

        // Guard against privileged runs (root) where chmod does not restrict.
        let probe = dir.join(".ecky-atomic-probe");
        let restricted = fs::write(&probe, b"").is_err();
        let _ = fs::remove_file(&probe);

        let result = if restricted {
            let r = export_project(&resolver, &sample_request("keep-abc12345", "REPLACEMENT"));
            // Restore writability before assertions so the dir stays manageable.
            let _ = fs::set_permissions(&dir, fs::Permissions::from_mode(0o700));
            r
        } else {
            let _ = fs::set_permissions(&dir, fs::Permissions::from_mode(0o700));
            eprintln!(
                "skipped export_failure_leaves_existing_source_intact: running \
                 with privileges that bypass directory permissions"
            );
            return;
        };

        assert!(result.is_err(), "writing into a read-only folder must fail");

        let after = fs::read_to_string(&source_path).expect("read survivor");
        assert_eq!(
            after, "ORIGINAL",
            "existing source must survive a failed atomic write byte-for-byte"
        );

        let residue: Vec<_> = fs::read_dir(&dir)
            .expect("read dir")
            .flatten()
            .map(|entry| entry.file_name())
            .filter(|name| name.to_string_lossy().contains("atomic"))
            .collect();
        assert!(
            residue.is_empty(),
            "atomic temp residue left behind: {residue:?}"
        );
    }

    #[test]
    fn atomic_write_replaces_bytes_and_preserves_existing_mode() {
        let resolver = temp_resolver("atomic-helper");
        let dir = projects_root(&resolver, None).join("helper-abc12345");
        fs::create_dir_all(&dir).expect("mkdir");
        let dest = dir.join(PROJECT_SOURCE_FILE_NAME);

        atomic_write(&dest, b"first").expect("create via temp+rename");
        assert_eq!(fs::read(&dest).unwrap(), b"first");

        atomic_write(&dest, b"second").expect("replace via temp+rename");
        assert_eq!(fs::read(&dest).unwrap(), b"second");

        let residue: Vec<_> = fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .map(|entry| entry.file_name())
            .filter(|name| name.to_string_lossy().contains("atomic"))
            .collect();
        assert!(residue.is_empty(), "no temp residue: {residue:?}");

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&dest, fs::Permissions::from_mode(0o600)).expect("chmod");
            atomic_write(&dest, b"third").expect("replace preserves perms");
            let mode = fs::metadata(&dest).unwrap().permissions().mode();
            assert_eq!(
                mode & 0o777,
                0o600,
                "external source mode must survive an atomic re-write"
            );

            // A failed write leaves the destination and its mode untouched.
            let read_only_dir = dir.join("locked");
            fs::create_dir_all(&read_only_dir).expect("mkdir locked");
            let locked_dest = read_only_dir.join(PROJECT_SOURCE_FILE_NAME);
            atomic_write(&locked_dest, b"seed").expect("seed locked");
            fs::set_permissions(&read_only_dir, fs::Permissions::from_mode(0o500))
                .expect("chmod r-x");
            let probe = read_only_dir.join(".probe");
            let restricted = fs::write(&probe, b"").is_err();
            let _ = fs::remove_file(&probe);
            if restricted {
                let err = atomic_write(&locked_dest, b"clobber").expect_err("must fail");
                let _ = fs::set_permissions(&read_only_dir, fs::Permissions::from_mode(0o700));
                assert!(
                    err.message.contains("Failed to stage temp"),
                    "{}",
                    err.message
                );
                assert_eq!(fs::read(&locked_dest).unwrap(), b"seed");
                let locked_residue: Vec<_> = fs::read_dir(&read_only_dir)
                    .unwrap()
                    .flatten()
                    .map(|entry| entry.file_name())
                    .filter(|name| name.to_string_lossy().contains("atomic"))
                    .collect();
                assert!(
                    locked_residue.is_empty(),
                    "residue after failure: {locked_residue:?}"
                );
            } else {
                let _ = fs::set_permissions(&read_only_dir, fs::Permissions::from_mode(0o700));
                eprintln!(
                    "skipped atomic_write failure branch: running with privileges \
                     that bypass directory permissions"
                );
            }
        }
    }

    #[test]
    fn slug_is_deterministic_and_safe() {
        assert_eq!(
            project_slug("Film Adapter v2!", "thread-12345678"),
            project_slug("Film Adapter v2!", "thread-12345678")
        );
        let slug = project_slug("Película / адаптер", "thread-XYZ99876");
        assert!(
            slug.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'),
            "{slug}"
        );
        assert!(slug.ends_with("XYZ99876"), "{slug}");
        assert!(project_slug("", "thread-1").starts_with("project-"));
    }

    #[test]
    fn classify_covers_the_full_matrix() {
        let manifest = ProjectManifest {
            schema_version: 1,
            project_id: "proj-x".into(),
            thread_id: "thread-1".into(),
            message_id: "msg-1".into(),
            model_id: None,
            source_digest: source_digest("a"),
            exported_at: 0,
        };
        let clean = source_digest("a");
        let edited = source_digest("b");

        assert_eq!(
            classify_sync_state(None, Some(&manifest), Some("msg-1")),
            ProjectSyncState::Missing
        );
        assert_eq!(
            classify_sync_state(Some(&clean), None, Some("msg-1")),
            ProjectSyncState::Missing
        );
        assert_eq!(
            classify_sync_state(Some(&clean), Some(&manifest), Some("msg-1")),
            ProjectSyncState::Clean
        );
        assert_eq!(
            classify_sync_state(Some(&edited), Some(&manifest), Some("msg-1")),
            ProjectSyncState::FileChanged
        );
        assert_eq!(
            classify_sync_state(Some(&clean), Some(&manifest), Some("msg-2")),
            ProjectSyncState::ThreadAdvanced
        );
        assert_eq!(
            classify_sync_state(Some(&edited), Some(&manifest), Some("msg-2")),
            ProjectSyncState::Conflict
        );
        assert_eq!(
            classify_sync_state(Some(&edited), Some(&manifest), None),
            ProjectSyncState::FileChanged
        );
    }

    #[test]
    fn folder_status_reports_missing_then_clean() {
        let resolver = temp_resolver("status");
        let status =
            folder_status(&resolver, None, "ghost-abc12345", Some("msg-1")).expect("status");
        assert_eq!(status.state, ProjectSyncState::Missing);

        let source = "(model (part body (box 1 2 3)))";
        export_project(&resolver, &sample_request("live-abc12345", source)).expect("export");
        let status =
            folder_status(&resolver, None, "live-abc12345", Some("msg-1")).expect("status");
        assert_eq!(status.state, ProjectSyncState::Clean);
        assert_eq!(
            status.file_digest.as_deref(),
            Some(source_digest(source).as_str())
        );

        fs::write(
            projects_root(&resolver, None)
                .join("live-abc12345")
                .join(PROJECT_SOURCE_FILE_NAME),
            "(model (part body (box 9 9 9)))",
        )
        .expect("external edit");
        let status =
            folder_status(&resolver, None, "live-abc12345", Some("msg-1")).expect("status");
        assert_eq!(status.state, ProjectSyncState::FileChanged);
    }

    #[test]
    fn unsafe_slugs_are_rejected() {
        let resolver = temp_resolver("unsafe");
        let err = folder_status(&resolver, None, "../escape", None).expect_err("unsafe slug");
        assert!(err.message.contains("not a safe"), "{}", err.message);
    }
}
