use super::{
    ArtifactBundle, CaptureGuideResultProvenance, CaptureObservedDeviationReport,
    CaptureReconstructionGuide, ModelManifest,
};
use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum CaptureSessionState {
    Pairing,
    Capturing,
    Reconstructing,
    Preview,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExistingCaptureTarget {
    pub thread_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message_id: Option<String>,
    pub source: String,
    pub source_language: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CaptureSessionInfo {
    pub session_id: String,
    pub target_thread_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_message_id: Option<String>,
    pub target_title: String,
    pub target_source: String,
    pub target_source_language: String,
    pub started_from_empty: bool,
    pub pairing_token: String,
    pub pairing_url: String,
    pub trust_url: String,
    pub protocol_version: u16,
    pub client_capabilities: CaptureClientCapabilities,
    pub state: CaptureSessionState,
    pub created_at: u64,
    pub expires_at: u64,
    #[serde(default)]
    pub accepted_frame_count: u32,
    #[serde(default)]
    pub coverage_percent: u8,
    #[serde(default)]
    pub guidance: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw_error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reconstruction_progress: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mesh_preview: Option<CaptureMeshPreview>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CaptureMeshPreview {
    pub stl_path: String,
    pub triangle_count: u64,
    pub bounds_mm: [f64; 3],
    pub scale_label: String,
    #[serde(default)]
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CapturePreparedPreview {
    pub artifact_bundle: ArtifactBundle,
    pub model_manifest: ModelManifest,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CaptureCropBounds {
    pub min: [f64; 3],
    pub max: [f64; 3],
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CaptureRun {
    pub id: String,
    pub target_thread_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_message_id: Option<String>,
    pub title: String,
    pub state: CaptureSessionState,
    pub created_at: u64,
    pub updated_at: u64,
    pub accepted_frame_count: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mesh_preview: Option<CaptureMeshPreview>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub derived_stl_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub crop_bounds: Option<CaptureCropBounds>,
    pub preview_scale: f64,
    pub target_source: String,
    pub target_source_language: String,
    pub started_from_empty: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw_error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reconstruction_guide: Option<CaptureReconstructionGuide>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reconstruction_guide_state: Option<CaptureReconstructionGuideState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guided_reconstruction_message_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guided_reconstruction_model_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guided_reconstruction_result: Option<CaptureGuideResultProvenance>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guided_reconstruction_deviation: Option<CaptureObservedDeviationReport>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "status",
    deny_unknown_fields
)]
pub enum CaptureReconstructionGuideState {
    Draft,
    Ready,
    Stale { reason: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ReopenedCaptureRun {
    pub run: CaptureRun,
    pub session: CaptureSessionInfo,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CaptureClientCapabilities {
    #[serde(default)]
    pub metric_depth: bool,
    #[serde(default)]
    pub camera_intrinsics: bool,
    #[serde(default)]
    pub camera_pose: bool,
    #[serde(default)]
    pub depth_sidecars: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CaptureFrameMetrics {
    pub luminance: f32,
    pub sharpness: f32,
    pub subject_coverage: f32,
    pub motion: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CaptureFrameManifestEntry {
    pub frame_id: String,
    pub content_digest: String,
    pub captured_at: u64,
    pub mime_type: String,
    pub width: u32,
    pub height: u32,
    pub image_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_metrics: Option<CaptureFrameMetrics>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub camera_intrinsics: Option<[f32; 9]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub camera_transform: Option<[f32; 16]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub depth_digest: Option<String>,
    #[serde(default)]
    pub visual_signature: Vec<u8>,
    #[serde(default)]
    pub server_assessment: CaptureServerAssessment,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CapturePairRequest {
    #[serde(default)]
    pub protocol_version: u16,
    #[serde(default)]
    pub capabilities: CaptureClientCapabilities,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CaptureServerAssessment {
    pub feature_overlap: f32,
    pub coverage_percent: u8,
    pub guidance: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub missing_view: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safari_frame_without_native_sidecars_remains_valid() {
        let json = r#"{"frameId":"f1","contentDigest":"abc","capturedAt":1,"mimeType":"image/jpeg","width":2,"height":2,"imagePath":"source/abc.jpg"}"#;
        let frame: CaptureFrameManifestEntry = serde_json::from_str(json).unwrap();
        assert_eq!(frame.camera_intrinsics, None);
        assert_eq!(frame.camera_transform, None);
        assert_eq!(frame.depth_digest, None);
    }

    #[test]
    fn native_frame_accepts_versioned_pose_and_depth_sidecars() {
        let frame = CaptureFrameManifestEntry {
            frame_id: "f1".into(),
            content_digest: "abc".into(),
            captured_at: 1,
            mime_type: "image/jpeg".into(),
            width: 2,
            height: 2,
            image_path: "source/abc.jpg".into(),
            client_metrics: None,
            camera_intrinsics: Some([1.0; 9]),
            camera_transform: Some([1.0; 16]),
            depth_digest: Some("depth-abc".into()),
            visual_signature: vec![1, 2, 3],
            server_assessment: CaptureServerAssessment::default(),
        };
        let restored: CaptureFrameManifestEntry =
            serde_json::from_value(serde_json::to_value(&frame).unwrap()).unwrap();
        assert_eq!(restored, frame);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CaptureFrameManifest {
    pub session_id: String,
    pub frames: Vec<CaptureFrameManifestEntry>,
}
