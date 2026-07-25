use serde::{Deserialize, Serialize};
use specta::Type;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AnimalCapCatalog {
    pub schema_version: u32,
    pub bore_profiles: BTreeMap<String, AnimalCapBoreProfile>,
    pub entries: Vec<AnimalCapCatalogEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AnimalCapBoreProfile {
    pub presta_major_diameter_mm: f64,
    pub thread_depth_mm: f64,
    pub base_thread_clearance_mm: f64,
    pub free_bore_clearance_mm: f64,
    pub thread_start_mm: f64,
    pub thread_length_mm: f64,
    pub inner_cone_start_mm: f64,
    pub blind_depth_mm: f64,
    pub entry_lead_mm: f64,
    pub entry_flare_mm: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AnimalCapState {
    Candidate,
    Published,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AnimalCapAxis {
    X,
    Y,
    Z,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AnimalCapVerificationStatus {
    Passed,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AnimalCapSurfaces {
    pub engine: bool,
    pub landing: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AnimalCapCatalogEntry {
    pub id: String,
    pub display_name: String,
    pub species: String,
    pub state: AnimalCapState,
    pub surfaces: AnimalCapSurfaces,
    pub source: AnimalCapSource,
    pub source_bounds: AnimalCapSourceBounds,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recipe: Option<AnimalCapRecipe>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact: Option<AnimalCapArtifact>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AnimalCapSource {
    pub author: String,
    pub page_url: String,
    pub download_url: String,
    pub archive_member: String,
    pub license: String,
    pub license_url: String,
    pub source_format: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_mesh_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ingested_stl_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ingested_stl_sha256: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AnimalCapSourceBounds {
    pub min: [f64; 3],
    pub max: [f64; 3],
    pub size: [f64; 3],
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AnimalCapRecipe {
    pub bore_profile_id: String,
    pub bore_axis: AnimalCapAxis,
    pub bore_mouth_source_coordinate: f64,
    pub bore_axis_height_mm: f64,
    pub uniform_scale: f64,
    pub floor_offset_source_coordinate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AnimalCapArtifact {
    pub verification_status: AnimalCapVerificationStatus,
    pub verified_part_count: u32,
    pub verified_component_count: u32,
    pub verified_non_manifold_edge_count: u32,
    pub verified_triangle_count: u32,
    pub model_id: String,
    pub thread_id: String,
    pub message_id: String,
    pub source_path: String,
    pub stl_path: String,
    pub preview_path: String,
    pub source_sha256: String,
    pub stl_sha256: String,
}
