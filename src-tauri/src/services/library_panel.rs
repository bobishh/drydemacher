use std::path::Path;

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::component_package_runtime;
use crate::contracts::{
    AppError, AppResult, ComponentPackageHeader, Config, FreecadLibraryItem,
    FreecadLibrarySearchRequest,
};
use crate::models::PathResolver;

const FREECAD_PAGE_SIZE: u32 = 100;

#[derive(Debug, Clone, Deserialize, Serialize, Type, PartialEq, Eq)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum LibraryPanelIntent {
    LoadComponents,
    #[specta(rename_all = "camelCase")]
    InstallPackage {
        archive_path: String,
    },
    #[specta(rename_all = "camelCase")]
    LoadFreecad {
        query: String,
        page: u32,
    },
    #[specta(rename_all = "camelCase")]
    SetFreecadRoot {
        root: String,
        query: String,
    },
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum LibraryPanelProjection {
    #[specta(rename_all = "camelCase")]
    ComponentPackages {
        package_headers: Vec<ComponentPackageHeader>,
    },
    #[specta(rename_all = "camelCase")]
    FreecadLibrary {
        freecad_library_roots: Vec<String>,
        items: Vec<FreecadLibraryItem>,
        page: u32,
        has_more: bool,
    },
}

pub fn load_component_packages(app: &dyn PathResolver) -> AppResult<LibraryPanelProjection> {
    Ok(LibraryPanelProjection::ComponentPackages {
        package_headers: component_package_runtime::list_installed_component_package_headers(app)?,
    })
}

pub fn install_component_package(
    app: &dyn PathResolver,
    archive_path: &str,
) -> AppResult<LibraryPanelProjection> {
    let archive_path = archive_path.trim();
    if archive_path.is_empty() {
        return Err(AppError::validation(
            "Component package archive path is required.",
        ));
    }
    component_package_runtime::install_component_package_to_store(app, Path::new(archive_path))?;
    load_component_packages(app)
}

pub fn config_with_freecad_root(config: &Config, root: &str) -> AppResult<Config> {
    let root = root.trim();
    if root.is_empty() {
        return Err(AppError::validation("FreeCAD library root is required."));
    }
    let mut updated = config.clone();
    updated.freecad_library_roots = vec![root.to_string()];
    Ok(updated)
}

pub fn load_freecad_page(
    config: Config,
    query: String,
    page: u32,
) -> AppResult<LibraryPanelProjection> {
    let mut items = crate::freecad_library::search_freecad_library(
        &FreecadLibrarySearchRequest {
            query,
            roots: Vec::new(),
            limit: Some(FREECAD_PAGE_SIZE + 1),
            offset: page.saturating_mul(FREECAD_PAGE_SIZE),
            include_architecture: false,
        },
        &config.freecad_library_roots,
    )?;
    let has_more = items.len() > FREECAD_PAGE_SIZE as usize;
    items.truncate(FREECAD_PAGE_SIZE as usize);
    Ok(LibraryPanelProjection::FreecadLibrary {
        freecad_library_roots: config.freecad_library_roots,
        items,
        page,
        has_more,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intent_boundary_is_tagged_and_camel_case() {
        let value = serde_json::to_value(LibraryPanelIntent::SetFreecadRoot {
            root: "/library".to_string(),
            query: "bolt".to_string(),
        })
        .expect("serialize");

        assert_eq!(value["kind"], "setFreecadRoot");
        assert_eq!(value["root"], "/library");
        assert!(value.get("archive_path").is_none());
    }

    #[test]
    fn root_update_preserves_config_and_replaces_roots() {
        let config: Config = serde_json::from_value(serde_json::json!({
            "engines": [],
            "selectedEngineId": "engine-1",
            "freecadCmd": "",
            "cadTextFontPath": "",
            "assets": [],
            "voice": {},
            "mcp": {},
            "femCompute": {},
            "providerModels": {},
            "defaultEngineKind": "eckyIrV0",
            "defaultSourceLanguage": "eckyIrV0",
            "defaultGeometryBackend": "eckyRust",
            "maxGenerationAttempts": 3,
            "maxVerifyAttempts": 2
        }))
        .expect("config");
        let updated = config_with_freecad_root(&config, " /library ").expect("root");

        assert_eq!(updated.freecad_library_roots, vec!["/library"]);
        assert_eq!(updated.selected_engine_id, config.selected_engine_id);
    }
}
