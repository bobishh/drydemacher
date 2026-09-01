use super::{Config, RuntimeCapabilities, Thread, WorkspaceProjection};
use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BootProjection {
    pub config: Config,
    pub history: Vec<Thread>,
    pub workspace: Option<WorkspaceProjection>,
    pub selected_part_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BootRuntimeProjection {
    pub config: Config,
    pub capabilities: RuntimeCapabilities,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ModelCatalogProjection {
    pub config: Config,
    pub models: Vec<String>,
}
