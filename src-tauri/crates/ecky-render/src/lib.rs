//! Platform-neutral Ecky render boundary.
//!
//! Types in this crate describe render input, in-memory output, and the
//! versioned plan sent to a geometry kernel. Host concerns such as filesystem
//! paths, Tauri handles, HTTP, queues, and process spawning belong in adapters.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub mod core_ir;
pub mod deterministic;
mod kernel_planner;
pub mod scheme;

#[doc(hidden)]
pub use core_ir as ecky_core_ir;
pub use kernel_planner::{PortableKernelPlanner, PortablePlanError};

/// Current render request/product schema.
pub const RENDER_SCHEMA_VERSION: u32 = 1;

/// Parameters resolved by the caller before rendering.
pub type RenderParameters = BTreeMap<String, Value>;

/// Platform-neutral render input.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderRequest {
    pub schema_version: u32,
    pub source: String,
    pub parameters: RenderParameters,
    /// Stable backend identifier. String keeps external kernels extensible.
    pub geometry_backend: String,
    pub requested_artifacts: BTreeSet<ArtifactFormat>,
    /// Logical asset id to immutable content. Source uses ids, never host paths.
    pub assets: BTreeMap<String, RenderAsset>,
}

/// Imported source asset available to the renderer without host filesystem IO.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderAsset {
    pub media_type: String,
    pub digest: String,
    pub bytes: Vec<u8>,
}

/// Export formats understood by the render boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ArtifactFormat {
    Step,
    Stl,
    Obj,
    Glb,
    #[serde(rename = "3mf")]
    ThreeMf,
    Json,
}

/// Artifact purpose, independent from storage location.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ArtifactRole {
    Preview,
    Export,
    Manifest,
    Topology,
    Part,
}

/// In-memory artifact. Host adapters decide where or whether to persist it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderArtifact {
    pub role: ArtifactRole,
    pub format: ArtifactFormat,
    pub media_type: String,
    pub digest: String,
    pub bytes: Vec<u8>,
}

/// Render result shared by desktop, worker, CLI, and browser adapters.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderProduct {
    pub schema_version: u32,
    pub source_digest: String,
    pub manifest: Value,
    pub artifacts: Vec<RenderArtifact>,
    pub diagnostics: Vec<RenderDiagnostic>,
}

/// Structured engine diagnostic. Raw engine detail remains available.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderDiagnostic {
    pub severity: DiagnosticSeverity,
    pub code: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DiagnosticSeverity {
    Info,
    Warning,
    Error,
}

/// Host-independent renderer. Native and WASM entrypoints wrap this boundary.
pub trait RenderEngine {
    type Error;

    fn render(&self, request: &RenderRequest) -> Result<RenderProduct, Self::Error>;
}

/// Source-language adapter. Host and WASM compilers share the same Core IR.
pub trait SourceCompiler {
    type Error;

    fn compile(&self, source: &str) -> Result<core_ir::CoreProgram, Self::Error>;
}

/// Core IR lowering port. Host adapters resolve files, fonts, and other assets
/// before calling a portable planner.
pub trait KernelPlanner {
    type Error;

    fn plan(
        &self,
        program: &core_ir::CoreProgram,
        parameters: &RenderParameters,
    ) -> Result<KernelPlan, Self::Error>;
}

/// Versioned command plan consumed by geometry-kernel adapters.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelPlan {
    pub schema_version: u32,
    pub plan_id: String,
    pub parts: Vec<KernelPart>,
    #[serde(default)]
    pub partial_boolean_groups: Vec<KernelPartialBooleanGroupPlan>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelPartialBooleanGroupPlan {
    pub part_key: String,
    pub parent_output: u64,
    pub operation: String,
    pub input_indices: Vec<u32>,
    pub ordinal: u32,
    pub version: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelPart {
    pub key: String,
    pub label: String,
    pub root: u64,
    pub commands: Vec<KernelCommand>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelCommand {
    pub output: u64,
    pub op: String,
    pub args: Vec<KernelArg>,
    pub keywords: Vec<KernelKeyword>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelArg {
    pub kind: String,
    pub value: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelKeyword {
    pub name: String,
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<KernelArg>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<Value>,
}
