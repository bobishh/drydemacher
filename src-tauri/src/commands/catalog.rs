use std::path::{Path, PathBuf};

use tauri::{AppHandle, Manager};

use crate::contracts::{AnimalCapCatalog, AnimalCapState, AppError, AppResult};

fn catalog_root(app: &AppHandle) -> AppResult<PathBuf> {
    let resource_root = app
        .path()
        .resource_dir()
        .map_err(|error| {
            AppError::persistence(format!("Animal cap resource path failed: {error}"))
        })?
        .join("catalogs/animal-caps");
    if resource_root.join("catalog.json").is_file() {
        return Ok(resource_root);
    }
    Ok(Path::new(env!("CARGO_MANIFEST_DIR")).join("../catalogs/animal-caps"))
}

pub(crate) fn load_animal_cap_catalog(root: &Path) -> AppResult<AnimalCapCatalog> {
    let manifest_path = root.join("catalog.json");
    let raw = std::fs::read_to_string(&manifest_path).map_err(|error| {
        AppError::persistence(format!(
            "Animal cap catalog read failed at '{}': {error}",
            manifest_path.display()
        ))
    })?;
    let mut catalog: AnimalCapCatalog = serde_json::from_str(&raw).map_err(|error| {
        AppError::persistence(format!(
            "Animal cap catalog parse failed at '{}': {error}",
            manifest_path.display()
        ))
    })?;

    catalog.entries.retain(|entry| entry.surfaces.engine);
    for entry in &mut catalog.entries {
        if entry.state != AnimalCapState::Published {
            return Err(AppError::validation(format!(
                "Animal cap '{}' is exposed to engine before publication.",
                entry.id
            )));
        }
        let artifact = entry.artifact.as_mut().ok_or_else(|| {
            AppError::validation(format!(
                "Published animal cap '{}' has no artifact metadata.",
                entry.id
            ))
        })?;
        for path in [
            &mut artifact.source_path,
            &mut artifact.stl_path,
            &mut artifact.preview_path,
        ] {
            let resolved = root.join(path.as_str());
            if !resolved.is_file() {
                return Err(AppError::persistence(format!(
                    "Animal cap '{}' artifact is missing: {}",
                    entry.id,
                    resolved.display()
                )));
            }
            *path = resolved.to_string_lossy().into_owned();
        }
        if let Some(path) = entry.source.source_mesh_path.as_mut() {
            *path = root.join(path.as_str()).to_string_lossy().into_owned();
        }
        if let Some(path) = entry.source.ingested_stl_path.as_mut() {
            *path = root.join(path.as_str()).to_string_lossy().into_owned();
        }
    }

    Ok(catalog)
}

#[tauri::command]
#[specta::specta]
pub async fn get_animal_cap_catalog(app: AppHandle) -> AppResult<AnimalCapCatalog> {
    load_animal_cap_catalog(&catalog_root(&app)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repo_catalog_projects_only_published_engine_entries() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../catalogs/animal-caps");
        let catalog = load_animal_cap_catalog(&root).expect("catalog");
        assert_eq!(catalog.schema_version, 1);
        assert_eq!(catalog.entries.len(), 1);
        let pug = &catalog.entries[0];
        assert_eq!(pug.id, "quaternius-pug-presta");
        assert_eq!(pug.recipe.as_ref().unwrap().uniform_scale, 12.0);
        assert!(Path::new(&pug.artifact.as_ref().unwrap().stl_path).is_file());
    }

    #[test]
    fn missing_catalog_keeps_raw_path_context() {
        let error = load_animal_cap_catalog(Path::new("/definitely/missing/animal-caps"))
            .expect_err("missing catalog should fail");
        assert!(error.message.contains("Animal cap catalog read failed"));
        assert!(error
            .message
            .contains("/definitely/missing/animal-caps/catalog.json"));
    }
}
