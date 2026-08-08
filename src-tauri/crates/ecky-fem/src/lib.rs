#![forbid(unsafe_code)]

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const FEM_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemValidationError {
    pub field: String,
    pub message: String,
}

impl fmt::Display for FemValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.field, self.message)
    }
}

impl std::error::Error for FemValidationError {}

pub trait CanonicalDigest {
    fn canonical_digest(&self) -> String;
}

fn stable_digest<T: Serialize>(value: &T) -> String {
    let bytes = serde_json::to_vec(value).expect("serializable FEM contract");
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("sha256:{:x}", hasher.finalize())
}

fn schema_version_error() -> FemValidationError {
    FemValidationError {
        field: "schemaVersion".to_string(),
        message: "unsupported schema version".to_string(),
    }
}

fn finite(field: &str, value: f64) -> Result<(), FemValidationError> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(FemValidationError {
            field: field.to_string(),
            message: "must be finite".to_string(),
        })
    }
}

fn positive_finite(field: &str, value: f64) -> Result<(), FemValidationError> {
    finite(field, value)?;
    if value > 0.0 {
        Ok(())
    } else {
        Err(FemValidationError {
            field: field.to_string(),
            message: "must be positive".to_string(),
        })
    }
}

fn sort_face_targets(targets: &mut [FemFaceTarget]) {
    targets.sort_by(|left, right| {
        left.durable_target_id
            .cmp(&right.durable_target_id)
            .then(left.canonical_target_id.cmp(&right.canonical_target_id))
            .then(left.part_id.cmp(&right.part_id))
            .then(
                left.source_geometry_digest
                    .cmp(&right.source_geometry_digest),
            )
    });
}

fn validate_face_targets(targets: &[FemFaceTarget]) -> Result<(), FemValidationError> {
    if targets.is_empty() {
        return Err(FemValidationError {
            field: "faces".to_string(),
            message: "must not be empty".to_string(),
        });
    }
    for target in targets {
        target.validate()?;
    }
    Ok(())
}

macro_rules! digest_impl {
    ($ty:ty) => {
        impl CanonicalDigest for $ty {
            fn canonical_digest(&self) -> String {
                stable_digest(self)
            }
        }
    };
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemMaterial {
    pub schema_version: u32,
    pub name: String,
    pub young_modulus_mpa: f64,
    pub poisson_ratio: f64,
    pub density_kg_per_mm3: f64,
    pub yield_strength_mpa: f64,
}

impl FemMaterial {
    pub fn validate(&self) -> Result<(), FemValidationError> {
        if self.schema_version != FEM_SCHEMA_VERSION {
            return Err(schema_version_error());
        }
        if self.name.trim().is_empty() {
            return Err(FemValidationError {
                field: "name".to_string(),
                message: "must not be empty".to_string(),
            });
        }
        positive_finite("youngModulusMpa", self.young_modulus_mpa)?;
        finite("poissonRatio", self.poisson_ratio)?;
        if !(-1.0 < self.poisson_ratio && self.poisson_ratio < 0.5) {
            return Err(FemValidationError {
                field: "poissonRatio".to_string(),
                message: "must be within (-1, 0.5)".to_string(),
            });
        }
        positive_finite("densityKgPerMm3", self.density_kg_per_mm3)?;
        positive_finite("yieldStrengthMpa", self.yield_strength_mpa)?;
        Ok(())
    }
}

digest_impl!(FemMaterial);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemFaceTarget {
    pub schema_version: u32,
    pub part_id: String,
    pub canonical_target_id: String,
    pub durable_target_id: String,
    pub source_geometry_digest: String,
}

impl FemFaceTarget {
    pub fn validate(&self) -> Result<(), FemValidationError> {
        if self.schema_version != FEM_SCHEMA_VERSION {
            return Err(schema_version_error());
        }
        if self.part_id.trim().is_empty() {
            return Err(FemValidationError {
                field: "partId".to_string(),
                message: "must not be empty".to_string(),
            });
        }
        if self.canonical_target_id.trim().is_empty() {
            return Err(FemValidationError {
                field: "canonicalTargetId".to_string(),
                message: "must not be empty".to_string(),
            });
        }
        if self.durable_target_id.trim().is_empty() {
            return Err(FemValidationError {
                field: "durableTargetId".to_string(),
                message: "must not be empty".to_string(),
            });
        }
        if self.source_geometry_digest.trim().is_empty() {
            return Err(FemValidationError {
                field: "sourceGeometryDigest".to_string(),
                message: "must not be empty".to_string(),
            });
        }
        Ok(())
    }
}

digest_impl!(FemFaceTarget);

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemForceVector {
    pub x_n: f64,
    pub y_n: f64,
    pub z_n: f64,
}

impl FemForceVector {
    pub fn validate(&self) -> Result<(), FemValidationError> {
        finite("xN", self.x_n)?;
        finite("yN", self.y_n)?;
        finite("zN", self.z_n)?;
        Ok(())
    }
}

digest_impl!(FemForceVector);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemStressVector {
    pub x_mpa: f64,
    pub y_mpa: f64,
    pub z_mpa: f64,
}

impl FemStressVector {
    pub fn validate(&self) -> Result<(), FemValidationError> {
        finite("xMpa", self.x_mpa)?;
        finite("yMpa", self.y_mpa)?;
        finite("zMpa", self.z_mpa)?;
        Ok(())
    }
}

digest_impl!(FemStressVector);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemOptionalDisplacement {
    pub x_mm: Option<f64>,
    pub y_mm: Option<f64>,
    pub z_mm: Option<f64>,
}

impl FemOptionalDisplacement {
    pub fn validate(&self) -> Result<(), FemValidationError> {
        if let Some(value) = self.x_mm {
            finite("xMm", value)?;
        }
        if let Some(value) = self.y_mm {
            finite("yMm", value)?;
        }
        if let Some(value) = self.z_mm {
            finite("zMm", value)?;
        }
        Ok(())
    }
}

digest_impl!(FemOptionalDisplacement);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemLocalRefinement {
    pub schema_version: u32,
    pub faces: Vec<FemFaceTarget>,
    pub size_mm: f64,
}

impl FemLocalRefinement {
    pub fn validate(&self) -> Result<(), FemValidationError> {
        if self.schema_version != FEM_SCHEMA_VERSION {
            return Err(schema_version_error());
        }
        validate_face_targets(&self.faces)?;
        positive_finite("sizeMm", self.size_mm)?;
        Ok(())
    }

    fn canonicalized(&self) -> Self {
        let mut faces = self.faces.clone();
        sort_face_targets(&mut faces);
        Self {
            schema_version: self.schema_version,
            faces,
            size_mm: self.size_mm,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind", content = "value")]
pub enum FemLoad {
    SurfaceForce {
        schema_version: u32,
        name: String,
        faces: Vec<FemFaceTarget>,
        total_force_n: FemForceVector,
    },
    Traction {
        schema_version: u32,
        name: String,
        faces: Vec<FemFaceTarget>,
        traction_mpa: FemStressVector,
    },
    Pressure {
        schema_version: u32,
        name: String,
        faces: Vec<FemFaceTarget>,
        pressure_mpa: f64,
    },
}

impl FemLoad {
    pub fn validate(&self) -> Result<(), FemValidationError> {
        match self {
            FemLoad::SurfaceForce {
                schema_version,
                name,
                faces,
                total_force_n,
            } => {
                if *schema_version != FEM_SCHEMA_VERSION {
                    return Err(schema_version_error());
                }
                if name.trim().is_empty() {
                    return Err(FemValidationError {
                        field: "name".to_string(),
                        message: "must not be empty".to_string(),
                    });
                }
                validate_face_targets(faces)?;
                total_force_n.validate()?;
            }
            FemLoad::Traction {
                schema_version,
                name,
                faces,
                traction_mpa,
            } => {
                if *schema_version != FEM_SCHEMA_VERSION {
                    return Err(schema_version_error());
                }
                if name.trim().is_empty() {
                    return Err(FemValidationError {
                        field: "name".to_string(),
                        message: "must not be empty".to_string(),
                    });
                }
                validate_face_targets(faces)?;
                traction_mpa.validate()?;
            }
            FemLoad::Pressure {
                schema_version,
                name,
                faces,
                pressure_mpa,
            } => {
                if *schema_version != FEM_SCHEMA_VERSION {
                    return Err(schema_version_error());
                }
                if name.trim().is_empty() {
                    return Err(FemValidationError {
                        field: "name".to_string(),
                        message: "must not be empty".to_string(),
                    });
                }
                validate_face_targets(faces)?;
                positive_finite("pressureMpa", *pressure_mpa)?;
            }
        }
        Ok(())
    }

    fn canonicalized(&self) -> Self {
        match self {
            FemLoad::SurfaceForce {
                schema_version,
                name,
                faces,
                total_force_n,
            } => {
                let mut faces = faces.clone();
                sort_face_targets(&mut faces);
                FemLoad::SurfaceForce {
                    schema_version: *schema_version,
                    name: name.clone(),
                    faces,
                    total_force_n: *total_force_n,
                }
            }
            FemLoad::Traction {
                schema_version,
                name,
                faces,
                traction_mpa,
            } => {
                let mut faces = faces.clone();
                sort_face_targets(&mut faces);
                FemLoad::Traction {
                    schema_version: *schema_version,
                    name: name.clone(),
                    faces,
                    traction_mpa: traction_mpa.clone(),
                }
            }
            FemLoad::Pressure {
                schema_version,
                name,
                faces,
                pressure_mpa,
            } => {
                let mut faces = faces.clone();
                sort_face_targets(&mut faces);
                FemLoad::Pressure {
                    schema_version: *schema_version,
                    name: name.clone(),
                    faces,
                    pressure_mpa: *pressure_mpa,
                }
            }
        }
    }
}

impl CanonicalDigest for FemLoad {
    fn canonical_digest(&self) -> String {
        stable_digest(&self.canonicalized())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind", content = "value")]
pub enum FemConstraint {
    Fixed {
        schema_version: u32,
        name: String,
        faces: Vec<FemFaceTarget>,
    },
    PrescribedDisplacement {
        schema_version: u32,
        name: String,
        faces: Vec<FemFaceTarget>,
        displacement_mm: FemOptionalDisplacement,
    },
}

impl FemConstraint {
    pub fn validate(&self) -> Result<(), FemValidationError> {
        match self {
            FemConstraint::Fixed {
                schema_version,
                name,
                faces,
            } => {
                if *schema_version != FEM_SCHEMA_VERSION {
                    return Err(schema_version_error());
                }
                if name.trim().is_empty() {
                    return Err(FemValidationError {
                        field: "name".to_string(),
                        message: "must not be empty".to_string(),
                    });
                }
                validate_face_targets(faces)?;
            }
            FemConstraint::PrescribedDisplacement {
                schema_version,
                name,
                faces,
                displacement_mm,
            } => {
                if *schema_version != FEM_SCHEMA_VERSION {
                    return Err(schema_version_error());
                }
                if name.trim().is_empty() {
                    return Err(FemValidationError {
                        field: "name".to_string(),
                        message: "must not be empty".to_string(),
                    });
                }
                validate_face_targets(faces)?;
                displacement_mm.validate()?;
                if displacement_mm.x_mm.is_none()
                    && displacement_mm.y_mm.is_none()
                    && displacement_mm.z_mm.is_none()
                {
                    return Err(FemValidationError {
                        field: "displacementMm".to_string(),
                        message: "must prescribe at least one component".to_string(),
                    });
                }
            }
        }
        Ok(())
    }

    fn canonicalized(&self) -> Self {
        match self {
            FemConstraint::Fixed {
                schema_version,
                name,
                faces,
            } => {
                let mut faces = faces.clone();
                sort_face_targets(&mut faces);
                FemConstraint::Fixed {
                    schema_version: *schema_version,
                    name: name.clone(),
                    faces,
                }
            }
            FemConstraint::PrescribedDisplacement {
                schema_version,
                name,
                faces,
                displacement_mm,
            } => {
                let mut faces = faces.clone();
                sort_face_targets(&mut faces);
                FemConstraint::PrescribedDisplacement {
                    schema_version: *schema_version,
                    name: name.clone(),
                    faces,
                    displacement_mm: displacement_mm.clone(),
                }
            }
        }
    }
}

impl CanonicalDigest for FemConstraint {
    fn canonical_digest(&self) -> String {
        stable_digest(&self.canonicalized())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FemElementKind {
    Tet4,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemBudgetLimits {
    pub schema_version: u32,
    pub boundary_triangles: u64,
    pub tet4_cells: u64,
    pub nodes: u64,
    pub dofs: u64,
    pub sparse_nonzeros: u64,
    pub result_bytes: u64,
    pub convergence_levels: u64,
}

impl FemBudgetLimits {
    pub fn validate(&self) -> Result<(), FemValidationError> {
        if self.schema_version != FEM_SCHEMA_VERSION {
            return Err(schema_version_error());
        }
        if self.boundary_triangles == 0
            || self.tet4_cells == 0
            || self.nodes == 0
            || self.dofs == 0
            || self.sparse_nonzeros == 0
            || self.result_bytes == 0
            || self.convergence_levels == 0
        {
            return Err(FemValidationError {
                field: "budgets".to_string(),
                message: "all budget limits must be positive".to_string(),
            });
        }
        Ok(())
    }
}

digest_impl!(FemBudgetLimits);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemAdmissionEstimate {
    pub schema_version: u32,
    pub boundary_triangles: u64,
    pub tet4_cells: u64,
    pub nodes: u64,
    pub dofs: u64,
    pub sparse_nonzeros: u64,
    pub result_bytes: u64,
    pub convergence_levels: u64,
}

impl FemAdmissionEstimate {
    pub fn diagnostics(&self, limits: &FemBudgetLimits) -> Vec<FemBudgetDiagnostic> {
        vec![
            FemBudgetDiagnostic {
                resource: FemResource::BoundaryTriangles,
                observed: self.boundary_triangles,
                allowed: limits.boundary_triangles,
            },
            FemBudgetDiagnostic {
                resource: FemResource::Tet4Cells,
                observed: self.tet4_cells,
                allowed: limits.tet4_cells,
            },
            FemBudgetDiagnostic {
                resource: FemResource::Nodes,
                observed: self.nodes,
                allowed: limits.nodes,
            },
            FemBudgetDiagnostic {
                resource: FemResource::Dofs,
                observed: self.dofs,
                allowed: limits.dofs,
            },
            FemBudgetDiagnostic {
                resource: FemResource::SparseNonzeros,
                observed: self.sparse_nonzeros,
                allowed: limits.sparse_nonzeros,
            },
            FemBudgetDiagnostic {
                resource: FemResource::ResultBytes,
                observed: self.result_bytes,
                allowed: limits.result_bytes,
            },
            FemBudgetDiagnostic {
                resource: FemResource::ConvergenceLevels,
                observed: self.convergence_levels,
                allowed: limits.convergence_levels,
            },
        ]
    }

    pub fn admit(&self, limits: &FemBudgetLimits) -> Result<(), FemBudgetAdmissionError> {
        if self.schema_version != FEM_SCHEMA_VERSION || limits.schema_version != FEM_SCHEMA_VERSION
        {
            return Err(FemBudgetAdmissionError {
                diagnostics: vec![],
            });
        }

        let diagnostics = self.diagnostics(limits);
        if diagnostics
            .iter()
            .any(|diagnostic| diagnostic.observed > diagnostic.allowed)
        {
            Err(FemBudgetAdmissionError { diagnostics })
        } else {
            Ok(())
        }
    }
}

digest_impl!(FemAdmissionEstimate);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FemResource {
    BoundaryTriangles,
    Tet4Cells,
    Nodes,
    Dofs,
    SparseNonzeros,
    ResultBytes,
    ConvergenceLevels,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemBudgetDiagnostic {
    pub resource: FemResource,
    pub observed: u64,
    pub allowed: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemBudgetAdmissionError {
    pub diagnostics: Vec<FemBudgetDiagnostic>,
}

impl fmt::Display for FemBudgetAdmissionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "budget admission rejected")
    }
}

impl std::error::Error for FemBudgetAdmissionError {}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemMeshControl {
    pub schema_version: u32,
    pub element_kind: FemElementKind,
    pub global_size_mm: f64,
    pub local_refinements: Vec<FemLocalRefinement>,
    pub budgets: FemBudgetLimits,
}

impl FemMeshControl {
    pub fn validate(&self) -> Result<(), FemValidationError> {
        if self.schema_version != FEM_SCHEMA_VERSION {
            return Err(schema_version_error());
        }
        if self.element_kind != FemElementKind::Tet4 {
            return Err(FemValidationError {
                field: "elementKind".to_string(),
                message: "only tet4 is supported".to_string(),
            });
        }
        positive_finite("globalSizeMm", self.global_size_mm)?;
        self.budgets.validate()?;
        for refinement in &self.local_refinements {
            refinement.validate()?;
        }
        Ok(())
    }

    fn canonicalized(&self) -> Self {
        let mut local_refinements = self.local_refinements.clone();
        local_refinements.sort_by(|left, right| {
            left.size_mm
                .to_bits()
                .cmp(&right.size_mm.to_bits())
                .then(left.canonical_digest().cmp(&right.canonical_digest()))
        });
        Self {
            schema_version: self.schema_version,
            element_kind: self.element_kind,
            global_size_mm: self.global_size_mm,
            local_refinements,
            budgets: self.budgets.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemRuntimeIdentity {
    pub schema_version: u32,
    pub platform: String,
    pub architecture: String,
    pub library_name: String,
    pub library_version: String,
    pub library_digest: String,
    pub adapter_protocol_version: u32,
    pub supported_capabilities: Vec<String>,
    pub notice_digest: String,
}

impl FemRuntimeIdentity {
    pub fn validate(&self) -> Result<(), FemValidationError> {
        if self.schema_version != FEM_SCHEMA_VERSION {
            return Err(schema_version_error());
        }
        if self.platform.trim().is_empty()
            || self.architecture.trim().is_empty()
            || self.library_name.trim().is_empty()
            || self.library_version.trim().is_empty()
            || self.library_digest.trim().is_empty()
            || self.notice_digest.trim().is_empty()
        {
            return Err(FemValidationError {
                field: "runtimeIdentity".to_string(),
                message: "must be fully populated".to_string(),
            });
        }
        if self.adapter_protocol_version == 0 {
            return Err(FemValidationError {
                field: "adapterProtocolVersion".to_string(),
                message: "must be positive".to_string(),
            });
        }
        if self.supported_capabilities.is_empty() {
            return Err(FemValidationError {
                field: "supportedCapabilities".to_string(),
                message: "must not be empty".to_string(),
            });
        }
        Ok(())
    }

    fn canonicalized(&self) -> Self {
        let mut supported_capabilities = self.supported_capabilities.clone();
        supported_capabilities.sort();
        Self {
            schema_version: self.schema_version,
            platform: self.platform.clone(),
            architecture: self.architecture.clone(),
            library_name: self.library_name.clone(),
            library_version: self.library_version.clone(),
            library_digest: self.library_digest.clone(),
            adapter_protocol_version: self.adapter_protocol_version,
            supported_capabilities,
            notice_digest: self.notice_digest.clone(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FemEvidenceAuthority {
    Unknown,
    Proposed,
    UserAccepted,
    RecordedSource,
}

impl FemEvidenceAuthority {
    fn is_authoritative(self) -> bool {
        matches!(self, Self::UserAccepted | Self::RecordedSource)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FemEvidenceSubject {
    Material,
    Load,
    Support,
    Connection,
    Geometry,
    AcceptanceCriterion,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemEngineeringQuestion {
    pub question_id: String,
    pub statement: String,
    pub decision: String,
    pub acceptance_metric_ids: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FemAcceptanceComparison {
    LessThanOrEqual,
    GreaterThanOrEqual,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemAcceptanceCriterion {
    pub metric_id: String,
    pub field: String,
    pub comparison: FemAcceptanceComparison,
    pub limit: f64,
    pub unit: String,
    pub requires_convergence: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemIdealizationRecord {
    pub source_geometry_digest: String,
    pub analysis_geometry_digest: String,
    pub affected_topology_ids: Vec<String>,
    pub justification: String,
    pub expected_influence_percent: f64,
    pub accepted_by_user: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FemIdealizationKind {
    ExactSolid,
    DefeaturedSolid,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemIdealizationArtifact {
    pub schema_version: u32,
    pub kind: FemIdealizationKind,
    pub source_geometry_digest: String,
    pub analysis_geometry_digest: String,
    pub manufacturing_geometry_digest: String,
    pub affected_topology_ids: Vec<String>,
    pub justification: String,
    pub expected_influence_percent: f64,
    pub accepted_by_user: bool,
}

impl FemIdealizationArtifact {
    pub fn from_record(record: &FemIdealizationRecord) -> Result<Self, FemValidationError> {
        let artifact = Self {
            schema_version: FEM_SCHEMA_VERSION,
            kind: if record.source_geometry_digest == record.analysis_geometry_digest {
                FemIdealizationKind::ExactSolid
            } else {
                FemIdealizationKind::DefeaturedSolid
            },
            source_geometry_digest: record.source_geometry_digest.clone(),
            analysis_geometry_digest: record.analysis_geometry_digest.clone(),
            manufacturing_geometry_digest: record.source_geometry_digest.clone(),
            affected_topology_ids: record.affected_topology_ids.clone(),
            justification: record.justification.clone(),
            expected_influence_percent: record.expected_influence_percent,
            accepted_by_user: record.accepted_by_user,
        };
        artifact.validate()?;
        Ok(artifact)
    }

    pub fn validate(&self) -> Result<(), FemValidationError> {
        if self.schema_version != FEM_SCHEMA_VERSION {
            return Err(schema_version_error());
        }
        if self.source_geometry_digest.trim().is_empty()
            || self.analysis_geometry_digest.trim().is_empty()
            || self.manufacturing_geometry_digest.trim().is_empty()
            || self.justification.trim().is_empty()
        {
            return Err(FemValidationError {
                field: "idealizationArtifact".into(),
                message:
                    "must define source, analysis, manufacturing identities, and justification"
                        .into(),
            });
        }
        if self.manufacturing_geometry_digest != self.source_geometry_digest {
            return Err(FemValidationError {
                field: "idealizationArtifact.manufacturingGeometryDigest".into(),
                message: "must preserve the original source geometry; analysis idealization cannot replace manufacturing BRep"
                    .into(),
            });
        }
        finite(
            "idealizationArtifact.expectedInfluencePercent",
            self.expected_influence_percent,
        )?;
        if self.expected_influence_percent < 0.0 {
            return Err(FemValidationError {
                field: "idealizationArtifact.expectedInfluencePercent".into(),
                message: "must be non-negative".into(),
            });
        }
        let unique = self
            .affected_topology_ids
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        if unique.len() != self.affected_topology_ids.len()
            || unique.iter().any(|target_id| target_id.trim().is_empty())
        {
            return Err(FemValidationError {
                field: "idealizationArtifact.affectedTopologyIds".into(),
                message: "must contain unique non-empty topology identities".into(),
            });
        }
        match self.kind {
            FemIdealizationKind::ExactSolid => {
                if self.source_geometry_digest != self.analysis_geometry_digest
                    || !self.affected_topology_ids.is_empty()
                    || self.expected_influence_percent != 0.0
                {
                    return Err(FemValidationError {
                        field: "idealizationArtifact.kind".into(),
                        message: "exact solid requires identical geometry, no affected topology, and zero influence"
                            .into(),
                    });
                }
            }
            FemIdealizationKind::DefeaturedSolid => {
                if self.source_geometry_digest == self.analysis_geometry_digest
                    || self.affected_topology_ids.is_empty()
                    || self.expected_influence_percent <= 0.0
                    || !self.accepted_by_user
                {
                    return Err(FemValidationError {
                        field: "idealizationArtifact.kind".into(),
                        message: "defeatured solid requires distinct analysis geometry, affected topology, positive influence threshold, and user approval"
                            .into(),
                    });
                }
            }
        }
        Ok(())
    }

    fn canonicalized(&self) -> Self {
        let mut canonical = self.clone();
        canonical.affected_topology_ids.sort();
        canonical
    }
}

impl CanonicalDigest for FemIdealizationArtifact {
    fn canonical_digest(&self) -> String {
        stable_digest(&self.canonicalized())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemEvidenceRecord {
    pub evidence_id: String,
    pub subject: FemEvidenceSubject,
    pub label: String,
    pub source: String,
    pub authority: FemEvidenceAuthority,
    pub uncertainty_percent: Option<f64>,
    pub decision_critical: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemInputEvidenceBinding {
    pub input_name: String,
    pub evidence_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FemStudyAssumptionCategory {
    Geometry,
    Physics,
    Material,
    Load,
    Support,
    Connection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FemStudyAssumptionStatus {
    Unknown,
    Proposed,
    Accepted,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemStudyAssumption {
    pub assumption_id: String,
    pub category: FemStudyAssumptionCategory,
    pub statement: String,
    pub status: FemStudyAssumptionStatus,
    pub evidence_ids: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FemApplicabilityCheckKind {
    OneSolidScope,
    UnsupportedInterfaces,
    ThinSlenderTet4Risk,
    NearIncompressibleLocking,
    ConstraintRealism,
    ConcentratedLoadSingularity,
    DisplacementRatio,
    ElasticRange,
    HotspotStability,
    BoundaryConditionSingularity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FemApplicabilityStatus {
    Pass,
    Warning,
    Blocked,
    NotEvaluated,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemApplicabilityCheck {
    pub check_id: String,
    pub kind: FemApplicabilityCheckKind,
    pub status: FemApplicabilityStatus,
    pub observed: Option<f64>,
    pub limit: Option<f64>,
    pub unit: Option<String>,
    pub evidence_ids: Vec<String>,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemPreSolveApplicabilityInput {
    pub schema_version: u32,
    pub solid_count: u32,
    pub unsupported_interface_count: u32,
    pub characteristic_size_mm: f64,
    pub minimum_thickness_mm: f64,
    pub poisson_ratio: f64,
    pub constrained_translation_components: u8,
    pub selected_load_area_mm2: f64,
    pub selected_support_area_mm2: f64,
    pub has_point_load_or_support: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemPostSolveApplicabilityInput {
    pub schema_version: u32,
    pub characteristic_size_mm: f64,
    pub maximum_displacement_mm: f64,
    pub maximum_von_mises_mpa: f64,
    pub yield_strength_mpa: f64,
    pub hotspot_movement_mm: f64,
    pub boundary_condition_singularity: bool,
}

pub fn audit_post_solve_applicability(
    input: &FemPostSolveApplicabilityInput,
) -> Result<Vec<FemApplicabilityCheck>, FemValidationError> {
    if input.schema_version != FEM_SCHEMA_VERSION {
        return Err(schema_version_error());
    }
    positive_finite("characteristicSizeMm", input.characteristic_size_mm)?;
    finite("maximumDisplacementMm", input.maximum_displacement_mm)?;
    finite("maximumVonMisesMpa", input.maximum_von_mises_mpa)?;
    positive_finite("yieldStrengthMpa", input.yield_strength_mpa)?;
    finite("hotspotMovementMm", input.hotspot_movement_mm)?;
    if input.maximum_displacement_mm < 0.0
        || input.maximum_von_mises_mpa < 0.0
        || input.hotspot_movement_mm < 0.0
    {
        return Err(FemValidationError {
            field: "postSolveApplicability".to_string(),
            message: "magnitudes must be non-negative".to_string(),
        });
    }
    let displacement_ratio = input.maximum_displacement_mm / input.characteristic_size_mm;
    let elastic_ratio = input.maximum_von_mises_mpa / input.yield_strength_mpa;
    let hotspot_movement_ratio = input.hotspot_movement_mm / input.characteristic_size_mm;
    Ok(vec![
        applicability_check(
            "small-displacement",
            FemApplicabilityCheckKind::DisplacementRatio,
            displacement_ratio <= 0.05,
            displacement_ratio,
            0.05,
            "ratio",
            "Maximum displacement must remain small relative to characteristic size.",
        ),
        applicability_check(
            "elastic-range",
            FemApplicabilityCheckKind::ElasticRange,
            elastic_ratio <= 1.0,
            elastic_ratio,
            1.0,
            "yieldRatio",
            "Linear-elastic result cannot support a green decision beyond declared yield.",
        ),
        applicability_check(
            "hotspot-stability",
            FemApplicabilityCheckKind::HotspotStability,
            hotspot_movement_ratio <= 0.02,
            hotspot_movement_ratio,
            0.02,
            "ratio",
            "Verification hotspot moves excessively across admitted refinements.",
        ),
        applicability_check(
            "boundary-singularity",
            FemApplicabilityCheckKind::BoundaryConditionSingularity,
            !input.boundary_condition_singularity,
            if input.boundary_condition_singularity {
                1.0
            } else {
                0.0
            },
            0.0,
            "flag",
            "Peak response is classified as a boundary-condition singularity.",
        ),
    ])
}

pub fn audit_pre_solve_applicability(
    input: &FemPreSolveApplicabilityInput,
) -> Result<Vec<FemApplicabilityCheck>, FemValidationError> {
    if input.schema_version != FEM_SCHEMA_VERSION {
        return Err(schema_version_error());
    }
    positive_finite("characteristicSizeMm", input.characteristic_size_mm)?;
    positive_finite("minimumThicknessMm", input.minimum_thickness_mm)?;
    finite("poissonRatio", input.poisson_ratio)?;
    if !(-1.0 < input.poisson_ratio && input.poisson_ratio < 0.5) {
        return Err(FemValidationError {
            field: "poissonRatio".to_string(),
            message: "must be within (-1, 0.5)".to_string(),
        });
    }
    finite("selectedLoadAreaMm2", input.selected_load_area_mm2)?;
    finite("selectedSupportAreaMm2", input.selected_support_area_mm2)?;
    if input.selected_load_area_mm2 < 0.0 || input.selected_support_area_mm2 < 0.0 {
        return Err(FemValidationError {
            field: "selectedAreaMm2".to_string(),
            message: "must be non-negative".to_string(),
        });
    }
    if input.constrained_translation_components > 3 {
        return Err(FemValidationError {
            field: "constrainedTranslationComponents".to_string(),
            message: "must be between zero and three".to_string(),
        });
    }

    let slenderness = input.characteristic_size_mm / input.minimum_thickness_mm;
    let characteristic_area = input.characteristic_size_mm * input.characteristic_size_mm;
    let minimum_selected_area_ratio = input
        .selected_load_area_mm2
        .min(input.selected_support_area_mm2)
        / characteristic_area;
    Ok(vec![
        applicability_check(
            "one-solid-scope",
            FemApplicabilityCheckKind::OneSolidScope,
            input.solid_count == 1,
            input.solid_count as f64,
            1.0,
            "solid",
            "Linear-static MVP requires exactly one connected solid.",
        ),
        applicability_check(
            "interfaces",
            FemApplicabilityCheckKind::UnsupportedInterfaces,
            input.unsupported_interface_count == 0,
            input.unsupported_interface_count as f64,
            0.0,
            "interface",
            "Contact, bonded, fastener, and other multi-body interfaces are unsupported.",
        ),
        applicability_check(
            "tet4-slenderness",
            FemApplicabilityCheckKind::ThinSlenderTet4Risk,
            slenderness <= 20.0,
            slenderness,
            20.0,
            "ratio",
            "Linear Tet4 is blocked for thin/slender geometry above the admitted ratio.",
        ),
        applicability_check(
            "locking",
            FemApplicabilityCheckKind::NearIncompressibleLocking,
            input.poisson_ratio <= 0.45,
            input.poisson_ratio,
            0.45,
            "ratio",
            "Linear Tet4 is blocked near incompressibility because volumetric locking is expected.",
        ),
        applicability_check(
            "constraints",
            FemApplicabilityCheckKind::ConstraintRealism,
            input.constrained_translation_components == 3 && input.selected_support_area_mm2 > 0.0,
            input.constrained_translation_components as f64,
            3.0,
            "component",
            "Supports must restrain all translation components through non-zero selected area.",
        ),
        applicability_check(
            "singularity",
            FemApplicabilityCheckKind::ConcentratedLoadSingularity,
            !input.has_point_load_or_support && minimum_selected_area_ratio >= 1.0e-4,
            minimum_selected_area_ratio,
            1.0e-4,
            "areaRatio",
            "Point-like or vanishing-area load/support evidence creates a singular hotspot.",
        ),
    ])
}

fn applicability_check(
    check_id: &str,
    kind: FemApplicabilityCheckKind,
    passed: bool,
    observed: f64,
    limit: f64,
    unit: &str,
    detail: &str,
) -> FemApplicabilityCheck {
    FemApplicabilityCheck {
        check_id: check_id.to_string(),
        kind,
        status: if passed {
            FemApplicabilityStatus::Pass
        } else {
            FemApplicabilityStatus::Blocked
        },
        observed: Some(observed),
        limit: Some(limit),
        unit: Some(unit.to_string()),
        evidence_ids: Vec::new(),
        detail: detail.to_string(),
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemSensitivityInputRange {
    pub input_name: String,
    pub evidence_id: String,
    pub lower_factor: f64,
    pub upper_factor: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemSensitivityMetricRange {
    pub metric_id: String,
    pub nominal: f64,
    pub minimum: f64,
    pub maximum: f64,
    pub unit: String,
    pub dominant_input_name: Option<String>,
    pub decision_changed: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemSensitivityEvidence {
    pub completed: bool,
    pub input_ranges: Vec<FemSensitivityInputRange>,
    pub case_result_digests: Vec<String>,
    pub metric_ranges: Vec<FemSensitivityMetricRange>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemSensitivityCaseResult {
    pub result_digest: String,
    pub metric_values: BTreeMap<String, f64>,
}

pub fn run_bounded_sensitivity<F>(
    input_ranges: &[FemSensitivityInputRange],
    criteria: &[FemAcceptanceCriterion],
    evaluator: F,
) -> Result<FemSensitivityEvidence, FemValidationError>
where
    F: Fn(&BTreeMap<String, f64>) -> Result<FemSensitivityCaseResult, FemValidationError>,
{
    if input_ranges.is_empty() || input_ranges.len() > 16 {
        return Err(FemValidationError {
            field: "sensitivity.inputRanges".to_string(),
            message: "must contain between one and sixteen inputs".to_string(),
        });
    }
    if criteria.is_empty() || criteria.len() > 32 {
        return Err(FemValidationError {
            field: "sensitivity.criteria".to_string(),
            message: "must contain between one and thirty-two metrics".to_string(),
        });
    }
    let mut factors = BTreeMap::new();
    for range in input_ranges {
        finite("sensitivity.lowerFactor", range.lower_factor)?;
        finite("sensitivity.upperFactor", range.upper_factor)?;
        if range.input_name.trim().is_empty()
            || range.evidence_id.trim().is_empty()
            || factors.insert(range.input_name.clone(), 1.0).is_some()
            || range.lower_factor <= 0.0
            || range.lower_factor > 1.0
            || range.upper_factor < 1.0
        {
            return Err(FemValidationError {
                field: "sensitivity.inputRanges".to_string(),
                message: "requires unique inputs with 0 < lower <= 1 <= upper".to_string(),
            });
        }
    }
    let mut metric_ids = std::collections::HashSet::new();
    for criterion in criteria {
        finite("sensitivity.criteria.limit", criterion.limit)?;
        if criterion.metric_id.trim().is_empty()
            || criterion.unit.trim().is_empty()
            || !metric_ids.insert(criterion.metric_id.as_str())
        {
            return Err(FemValidationError {
                field: "sensitivity.criteria".to_string(),
                message: "contains duplicate or incomplete criterion".to_string(),
            });
        }
    }

    let nominal = validate_sensitivity_case(evaluator(&factors)?, criteria)?;
    let mut cases = vec![(None::<String>, nominal.clone())];
    for range in input_ranges {
        for factor in [range.lower_factor, range.upper_factor] {
            factors.insert(range.input_name.clone(), factor);
            let result = validate_sensitivity_case(evaluator(&factors)?, criteria)?;
            cases.push((Some(range.input_name.clone()), result));
        }
        factors.insert(range.input_name.clone(), 1.0);
    }

    let metric_ranges = criteria
        .iter()
        .map(|criterion| {
            let nominal_value = nominal.metric_values[&criterion.metric_id];
            let minimum = cases
                .iter()
                .map(|(_, case)| case.metric_values[&criterion.metric_id])
                .fold(f64::INFINITY, f64::min);
            let maximum = cases
                .iter()
                .map(|(_, case)| case.metric_values[&criterion.metric_id])
                .fold(f64::NEG_INFINITY, f64::max);
            let dominant_input_name = cases
                .iter()
                .filter_map(|(input, case)| {
                    input.as_ref().map(|input| {
                        (
                            input.clone(),
                            (case.metric_values[&criterion.metric_id] - nominal_value).abs(),
                        )
                    })
                })
                .max_by(|left, right| left.1.total_cmp(&right.1))
                .map(|(input, _)| input);
            let nominal_accepted = criterion_accepts(criterion, nominal_value);
            let decision_changed = criterion_accepts(criterion, minimum) != nominal_accepted
                || criterion_accepts(criterion, maximum) != nominal_accepted;
            FemSensitivityMetricRange {
                metric_id: criterion.metric_id.clone(),
                nominal: nominal_value,
                minimum,
                maximum,
                unit: criterion.unit.clone(),
                dominant_input_name,
                decision_changed,
            }
        })
        .collect();

    Ok(FemSensitivityEvidence {
        completed: true,
        input_ranges: input_ranges.to_vec(),
        case_result_digests: cases
            .into_iter()
            .map(|(_, case)| case.result_digest)
            .collect(),
        metric_ranges,
    })
}

fn validate_sensitivity_case(
    result: FemSensitivityCaseResult,
    criteria: &[FemAcceptanceCriterion],
) -> Result<FemSensitivityCaseResult, FemValidationError> {
    if result.result_digest.trim().is_empty() {
        return Err(FemValidationError {
            field: "sensitivity.resultDigest".to_string(),
            message: "must not be empty".to_string(),
        });
    }
    for criterion in criteria {
        let value = result
            .metric_values
            .get(&criterion.metric_id)
            .ok_or_else(|| FemValidationError {
                field: format!("sensitivity.metrics.{}", criterion.metric_id),
                message: "is missing from case result".to_string(),
            })?;
        finite("sensitivity.metricValue", *value)?;
    }
    Ok(result)
}

fn criterion_accepts(criterion: &FemAcceptanceCriterion, value: f64) -> bool {
    match criterion.comparison {
        FemAcceptanceComparison::LessThanOrEqual => value <= criterion.limit,
        FemAcceptanceComparison::GreaterThanOrEqual => value >= criterion.limit,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FemValidationEvidenceKind {
    Analytical,
    DifferentialSolver,
    QualifiedReference,
    PhysicalTest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemValidationEvidence {
    pub validation_id: String,
    pub kind: FemValidationEvidenceKind,
    pub source: String,
    pub result_digest: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemEngineeringEvidenceLedger {
    pub schema_version: u32,
    pub question: FemEngineeringQuestion,
    pub acceptance_criteria: Vec<FemAcceptanceCriterion>,
    pub idealization: FemIdealizationRecord,
    pub evidence: Vec<FemEvidenceRecord>,
    pub input_bindings: Vec<FemInputEvidenceBinding>,
    pub assumptions: Vec<FemStudyAssumption>,
    pub applicability_checks: Vec<FemApplicabilityCheck>,
    pub sensitivity: Option<FemSensitivityEvidence>,
    pub validation_evidence: Vec<FemValidationEvidence>,
}

impl FemEngineeringEvidenceLedger {
    pub fn validate(&self) -> Result<(), FemValidationError> {
        use std::collections::HashSet;

        if self.schema_version != FEM_SCHEMA_VERSION {
            return Err(schema_version_error());
        }
        if self.question.question_id.trim().is_empty()
            || self.question.statement.trim().is_empty()
            || self.question.decision.trim().is_empty()
            || self.question.acceptance_metric_ids.is_empty()
        {
            return Err(FemValidationError {
                field: "question".to_string(),
                message: "must define question, decision, and acceptance metrics".to_string(),
            });
        }
        let expected_metric_ids = self
            .question
            .acceptance_metric_ids
            .iter()
            .map(String::as_str)
            .collect::<HashSet<_>>();
        if expected_metric_ids.len() != self.question.acceptance_metric_ids.len() {
            return Err(FemValidationError {
                field: "question.acceptanceMetricIds".to_string(),
                message: "must be unique".to_string(),
            });
        }
        let mut criterion_ids = HashSet::new();
        for criterion in &self.acceptance_criteria {
            if criterion.metric_id.trim().is_empty()
                || !criterion_ids.insert(criterion.metric_id.as_str())
                || criterion.field.trim().is_empty()
                || criterion.unit.trim().is_empty()
            {
                return Err(FemValidationError {
                    field: "acceptanceCriteria".to_string(),
                    message: "contains duplicate or incomplete criterion".to_string(),
                });
            }
            finite("acceptanceCriteria.limit", criterion.limit)?;
        }
        if criterion_ids != expected_metric_ids {
            return Err(FemValidationError {
                field: "acceptanceCriteria".to_string(),
                message: "must exactly define every question acceptance metric".to_string(),
            });
        }
        if self.idealization.source_geometry_digest.trim().is_empty()
            || self.idealization.analysis_geometry_digest.trim().is_empty()
            || self.idealization.justification.trim().is_empty()
        {
            return Err(FemValidationError {
                field: "idealization".to_string(),
                message: "must define geometry identities and justification".to_string(),
            });
        }
        finite(
            "expectedInfluencePercent",
            self.idealization.expected_influence_percent,
        )?;
        if self.idealization.expected_influence_percent < 0.0 {
            return Err(FemValidationError {
                field: "expectedInfluencePercent".to_string(),
                message: "must be non-negative".to_string(),
            });
        }
        if self.idealization.source_geometry_digest != self.idealization.analysis_geometry_digest
            && !self.idealization.accepted_by_user
        {
            return Err(FemValidationError {
                field: "idealization.acceptedByUser".to_string(),
                message: "changed analysis geometry requires explicit acceptance".to_string(),
            });
        }

        let mut evidence_ids = HashSet::new();
        for record in &self.evidence {
            if record.evidence_id.trim().is_empty()
                || !evidence_ids.insert(record.evidence_id.as_str())
                || record.label.trim().is_empty()
                || record.source.trim().is_empty()
            {
                return Err(FemValidationError {
                    field: "evidence".to_string(),
                    message: "contains duplicate, empty, or untraceable record".to_string(),
                });
            }
            if let Some(uncertainty) = record.uncertainty_percent {
                finite("uncertaintyPercent", uncertainty)?;
                if uncertainty < 0.0 {
                    return Err(FemValidationError {
                        field: "uncertaintyPercent".to_string(),
                        message: "must be non-negative".to_string(),
                    });
                }
            }
        }
        let mut input_names = HashSet::new();
        for binding in &self.input_bindings {
            if binding.input_name.trim().is_empty()
                || !input_names.insert(binding.input_name.as_str())
                || !evidence_ids.contains(binding.evidence_id.as_str())
            {
                return Err(FemValidationError {
                    field: "inputBindings".to_string(),
                    message: "must uniquely reference existing evidence".to_string(),
                });
            }
        }
        let bound_evidence_ids = self
            .input_bindings
            .iter()
            .map(|binding| binding.evidence_id.as_str())
            .collect::<HashSet<_>>();
        if let Some(record) = self.evidence.iter().find(|record| {
            record.decision_critical
                && matches!(
                    record.subject,
                    FemEvidenceSubject::Material
                        | FemEvidenceSubject::Load
                        | FemEvidenceSubject::Support
                        | FemEvidenceSubject::Connection
                )
                && !bound_evidence_ids.contains(record.evidence_id.as_str())
        }) {
            return Err(FemValidationError {
                field: format!("evidence.{}", record.evidence_id),
                message: "decision-critical engineering input is not bound to an authored input"
                    .to_string(),
            });
        }
        let mut assumption_ids = HashSet::new();
        for assumption in &self.assumptions {
            if assumption.assumption_id.trim().is_empty()
                || !assumption_ids.insert(assumption.assumption_id.as_str())
                || assumption.statement.trim().is_empty()
                || assumption
                    .evidence_ids
                    .iter()
                    .any(|id| !evidence_ids.contains(id.as_str()))
            {
                return Err(FemValidationError {
                    field: "assumptions".to_string(),
                    message: "contains duplicate, empty, or untraceable assumption".to_string(),
                });
            }
        }
        let mut applicability_ids = HashSet::new();
        for check in &self.applicability_checks {
            if check.check_id.trim().is_empty()
                || !applicability_ids.insert(check.check_id.as_str())
                || check.detail.trim().is_empty()
                || check
                    .evidence_ids
                    .iter()
                    .any(|id| !evidence_ids.contains(id.as_str()))
            {
                return Err(FemValidationError {
                    field: "applicabilityChecks".to_string(),
                    message: "contains duplicate, incomplete, or untraceable check".to_string(),
                });
            }
            if let Some(observed) = check.observed {
                finite("applicabilityChecks.observed", observed)?;
            }
            if let Some(limit) = check.limit {
                finite("applicabilityChecks.limit", limit)?;
            }
            if check.observed.is_some() != check.limit.is_some() {
                return Err(FemValidationError {
                    field: format!("applicabilityChecks.{}", check.check_id),
                    message: "observed and limit must be recorded together".to_string(),
                });
            }
        }
        if self.applicability_checks.is_empty() {
            return Err(FemValidationError {
                field: "applicabilityChecks".to_string(),
                message: "must not be empty".to_string(),
            });
        }
        if let Some(sensitivity) = &self.sensitivity {
            let mut sensitivity_inputs = HashSet::new();
            for range in &sensitivity.input_ranges {
                if range.input_name.trim().is_empty()
                    || !sensitivity_inputs.insert(range.input_name.as_str())
                    || !evidence_ids.contains(range.evidence_id.as_str())
                {
                    return Err(FemValidationError {
                        field: "sensitivity.inputRanges".to_string(),
                        message: "contains duplicate, incomplete, or untraceable input".to_string(),
                    });
                }
                finite("sensitivity.lowerFactor", range.lower_factor)?;
                finite("sensitivity.upperFactor", range.upper_factor)?;
                if range.lower_factor <= 0.0 || range.upper_factor < range.lower_factor {
                    return Err(FemValidationError {
                        field: format!("sensitivity.inputRanges.{}", range.input_name),
                        message: "requires 0 < lowerFactor <= upperFactor".to_string(),
                    });
                }
            }
            let mut sensitivity_metrics = HashSet::new();
            for range in &sensitivity.metric_ranges {
                if range.metric_id.trim().is_empty()
                    || !sensitivity_metrics.insert(range.metric_id.as_str())
                    || !expected_metric_ids.contains(range.metric_id.as_str())
                    || range.unit.trim().is_empty()
                {
                    return Err(FemValidationError {
                        field: "sensitivity.metricRanges".to_string(),
                        message: "contains duplicate, unknown, or incomplete metric".to_string(),
                    });
                }
                finite("sensitivity.nominal", range.nominal)?;
                finite("sensitivity.minimum", range.minimum)?;
                finite("sensitivity.maximum", range.maximum)?;
                if range.minimum > range.nominal || range.nominal > range.maximum {
                    return Err(FemValidationError {
                        field: format!("sensitivity.metricRanges.{}", range.metric_id),
                        message: "requires minimum <= nominal <= maximum".to_string(),
                    });
                }
            }
            if sensitivity
                .case_result_digests
                .iter()
                .any(|digest| digest.trim().is_empty())
            {
                return Err(FemValidationError {
                    field: "sensitivity.caseResultDigests".to_string(),
                    message: "contains empty result digest".to_string(),
                });
            }
        }
        let mut validation_ids = HashSet::new();
        for evidence in &self.validation_evidence {
            if evidence.validation_id.trim().is_empty()
                || !validation_ids.insert(evidence.validation_id.as_str())
                || evidence.source.trim().is_empty()
                || evidence.result_digest.trim().is_empty()
            {
                return Err(FemValidationError {
                    field: "validationEvidence".to_string(),
                    message: "contains duplicate, empty, or untraceable validation".to_string(),
                });
            }
        }
        Ok(())
    }

    pub fn validate_decision_readiness(&self) -> Result<(), FemValidationError> {
        self.validate()?;
        for (subject, label) in [
            (FemEvidenceSubject::Material, "material"),
            (FemEvidenceSubject::Load, "load"),
            (FemEvidenceSubject::Support, "support"),
        ] {
            if !self.evidence.iter().any(|record| record.subject == subject) {
                return Err(FemValidationError {
                    field: "evidence".to_string(),
                    message: format!("required {label} evidence is missing"),
                });
            }
        }
        for evidence in &self.evidence {
            if evidence.decision_critical && !evidence.authority.is_authoritative() {
                return Err(FemValidationError {
                    field: format!("evidence.{}", evidence.evidence_id),
                    message: format!(
                        "decision-critical evidence '{}' is not authoritative",
                        evidence.evidence_id
                    ),
                });
            }
        }
        for assumption in &self.assumptions {
            if assumption.status != FemStudyAssumptionStatus::Accepted {
                return Err(FemValidationError {
                    field: format!("assumptions.{}", assumption.assumption_id),
                    message: format!("assumption '{}' is not accepted", assumption.assumption_id),
                });
            }
        }
        for check in &self.applicability_checks {
            if matches!(
                check.status,
                FemApplicabilityStatus::Blocked | FemApplicabilityStatus::NotEvaluated
            ) {
                return Err(FemValidationError {
                    field: format!("applicabilityChecks.{}", check.check_id),
                    message: format!(
                        "applicability check '{}' blocks the engineering decision",
                        check.check_id
                    ),
                });
            }
        }
        let needs_sensitivity = self.evidence.iter().any(|record| {
            record.decision_critical && record.uncertainty_percent.is_some_and(|value| value > 0.0)
        });
        if needs_sensitivity {
            let sensitivity = self
                .sensitivity
                .as_ref()
                .ok_or_else(|| FemValidationError {
                    field: "sensitivity".to_string(),
                    message: "decision-critical uncertainty requires sensitivity evidence"
                        .to_string(),
                })?;
            if !sensitivity.completed
                || sensitivity.input_ranges.is_empty()
                || sensitivity.metric_ranges.is_empty()
                || sensitivity.case_result_digests.is_empty()
            {
                return Err(FemValidationError {
                    field: "sensitivity".to_string(),
                    message: "decision-critical sensitivity evidence is incomplete".to_string(),
                });
            }
            if let Some(metric) = sensitivity
                .metric_ranges
                .iter()
                .find(|metric| metric.decision_changed)
            {
                return Err(FemValidationError {
                    field: format!("sensitivity.metricRanges.{}", metric.metric_id),
                    message: format!(
                        "sensitivity range for metric '{}' changes the engineering decision",
                        metric.metric_id
                    ),
                });
            }
        }
        if !self.validation_evidence.iter().any(|evidence| {
            matches!(
                evidence.kind,
                FemValidationEvidenceKind::QualifiedReference
                    | FemValidationEvidenceKind::PhysicalTest
            )
        }) {
            return Err(FemValidationError {
                field: "validationEvidence".to_string(),
                message: "physical or qualified-reference validation is missing".to_string(),
            });
        }
        Ok(())
    }

    fn canonicalized(&self) -> Self {
        let mut canonical = self.clone();
        canonical.question.acceptance_metric_ids.sort();
        canonical
            .acceptance_criteria
            .sort_by(|left, right| left.metric_id.cmp(&right.metric_id));
        canonical.idealization.affected_topology_ids.sort();
        canonical
            .evidence
            .sort_by(|left, right| left.evidence_id.cmp(&right.evidence_id));
        canonical.input_bindings.sort_by(|left, right| {
            left.input_name
                .cmp(&right.input_name)
                .then(left.evidence_id.cmp(&right.evidence_id))
        });
        for assumption in &mut canonical.assumptions {
            assumption.evidence_ids.sort();
        }
        canonical
            .assumptions
            .sort_by(|left, right| left.assumption_id.cmp(&right.assumption_id));
        for check in &mut canonical.applicability_checks {
            check.evidence_ids.sort();
        }
        canonical
            .applicability_checks
            .sort_by(|left, right| left.check_id.cmp(&right.check_id));
        if let Some(sensitivity) = &mut canonical.sensitivity {
            sensitivity.input_ranges.sort_by(|left, right| {
                left.input_name
                    .cmp(&right.input_name)
                    .then(left.evidence_id.cmp(&right.evidence_id))
            });
            sensitivity.case_result_digests.sort();
            sensitivity
                .metric_ranges
                .sort_by(|left, right| left.metric_id.cmp(&right.metric_id));
        }
        canonical
            .validation_evidence
            .sort_by(|left, right| left.validation_id.cmp(&right.validation_id));
        canonical
    }
}

impl CanonicalDigest for FemEngineeringEvidenceLedger {
    fn canonical_digest(&self) -> String {
        stable_digest(&self.canonicalized())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemAnalysisIdentity {
    pub schema_version: u32,
    pub study_name: String,
    pub part_id: String,
    pub geometry_digest: String,
    pub engineering_evidence_digest: String,
    pub material_digest: String,
    pub load_digests: Vec<String>,
    pub constraint_digests: Vec<String>,
    pub mesh_control_digest: String,
    pub runtime_identity_digest: String,
}

impl FemAnalysisIdentity {
    pub fn validate(&self) -> Result<(), FemValidationError> {
        if self.schema_version != FEM_SCHEMA_VERSION {
            return Err(schema_version_error());
        }
        if self.study_name.trim().is_empty()
            || self.part_id.trim().is_empty()
            || self.geometry_digest.trim().is_empty()
            || self.engineering_evidence_digest.trim().is_empty()
            || self.material_digest.trim().is_empty()
            || self.mesh_control_digest.trim().is_empty()
            || self.runtime_identity_digest.trim().is_empty()
        {
            return Err(FemValidationError {
                field: "analysisIdentity".to_string(),
                message: "must be fully populated".to_string(),
            });
        }
        Ok(())
    }

    fn canonicalized(&self) -> Self {
        let mut load_digests = self.load_digests.clone();
        load_digests.sort();
        let mut constraint_digests = self.constraint_digests.clone();
        constraint_digests.sort();
        Self {
            schema_version: self.schema_version,
            study_name: self.study_name.clone(),
            part_id: self.part_id.clone(),
            geometry_digest: self.geometry_digest.clone(),
            engineering_evidence_digest: self.engineering_evidence_digest.clone(),
            material_digest: self.material_digest.clone(),
            load_digests,
            constraint_digests,
            mesh_control_digest: self.mesh_control_digest.clone(),
            runtime_identity_digest: self.runtime_identity_digest.clone(),
        }
    }
}

impl CanonicalDigest for FemLocalRefinement {
    fn canonical_digest(&self) -> String {
        stable_digest(&self.canonicalized())
    }
}

impl CanonicalDigest for FemMeshControl {
    fn canonical_digest(&self) -> String {
        stable_digest(&self.canonicalized())
    }
}

impl CanonicalDigest for FemRuntimeIdentity {
    fn canonical_digest(&self) -> String {
        stable_digest(&self.canonicalized())
    }
}

impl CanonicalDigest for FemAnalysisIdentity {
    fn canonical_digest(&self) -> String {
        stable_digest(&self.canonicalized())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemVolumeMeshInput {
    pub schema_version: u32,
    pub nodes: Vec<FemPoint3>,
    pub cells: Vec<[u32; 4]>,
    pub boundary_triangles: Vec<[u32; 3]>,
    pub boundary_face_group_indices: Vec<u32>,
    pub face_group_count: u32,
    pub face_group_targets: Vec<FemFaceTarget>,
    pub source_boundary_digest: String,
    pub mesher_identity: FemRuntimeIdentity,
    pub meshing_evidence: FemMeshingEvidence,
    pub minimum_scaled_jacobian: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemMeshingEvidence {
    pub schema_version: u32,
    pub source_triangle_count: u64,
    pub inserted_source_triangle_count: u64,
    pub tagged_boundary_triangle_count: u64,
    pub maximum_boundary_deviation_mm: f64,
    pub deterministic_thread_count: u32,
}

impl FemMeshingEvidence {
    pub fn validate(&self) -> Result<(), FemValidationError> {
        if self.schema_version != FEM_SCHEMA_VERSION {
            return Err(schema_version_error());
        }
        if self.source_triangle_count == 0
            || self.inserted_source_triangle_count > self.source_triangle_count
            || self.tagged_boundary_triangle_count == 0
        {
            return Err(FemValidationError {
                field: "meshingEvidence".to_string(),
                message: "source/tag counts are empty or inconsistent".to_string(),
            });
        }
        if !self.maximum_boundary_deviation_mm.is_finite()
            || self.maximum_boundary_deviation_mm < 0.0
        {
            return Err(FemValidationError {
                field: "meshingEvidence.maximumBoundaryDeviationMm".to_string(),
                message: "must be finite and non-negative".to_string(),
            });
        }
        if self.deterministic_thread_count != 1 {
            return Err(FemValidationError {
                field: "meshingEvidence.deterministicThreadCount".to_string(),
                message: "must be exactly 1".to_string(),
            });
        }
        Ok(())
    }
}

digest_impl!(FemMeshingEvidence);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemMeshQuality {
    pub minimum_signed_volume_mm3: f64,
    pub maximum_signed_volume_mm3: f64,
    pub minimum_scaled_jacobian: f64,
    pub minimum_radius_ratio: f64,
    pub worst_cell_index: u32,
    pub worst_cell_centroid_mm: FemPoint3,
    pub connected_component_count: u32,
    pub boundary_area_mm2_by_group: Vec<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemVolumeMesh {
    pub schema_version: u32,
    pub nodes: Vec<FemPoint3>,
    pub cells: Vec<[u32; 4]>,
    pub boundary_triangles: Vec<[u32; 3]>,
    pub boundary_face_group_indices: Vec<u32>,
    pub face_group_count: u32,
    pub face_group_targets: Vec<FemFaceTarget>,
    pub source_boundary_digest: String,
    pub mesher_identity: FemRuntimeIdentity,
    pub meshing_evidence: FemMeshingEvidence,
    pub quality: FemMeshQuality,
    pub content_digest: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FemVolumeMeshDigestView<'a> {
    schema_version: u32,
    nodes: &'a [FemPoint3],
    cells: &'a [[u32; 4]],
    boundary_triangles: &'a [[u32; 3]],
    boundary_face_group_indices: &'a [u32],
    face_group_count: u32,
    face_group_targets: &'a [FemFaceTarget],
    source_boundary_digest: &'a str,
    mesher_identity: &'a FemRuntimeIdentity,
    meshing_evidence: &'a FemMeshingEvidence,
    quality: &'a FemMeshQuality,
}

impl FemVolumeMesh {
    pub fn validate_and_canonicalize(
        input: FemVolumeMeshInput,
    ) -> Result<Self, FemValidationError> {
        validate_volume_mesh_input_header(&input)?;
        let (nodes, old_to_new) = canonicalize_mesh_nodes(input.nodes)?;
        let mut cells = canonicalize_mesh_cells(&nodes, &old_to_new, input.cells)?;
        cells.sort_unstable();

        let (exterior_facets, connected_component_count) =
            derive_exterior_facets_and_components(&cells)?;
        if connected_component_count != 1 {
            return Err(FemValidationError {
                field: "cells".to_string(),
                message: format!(
                    "volume mesh must contain one connected component; observed {connected_component_count}"
                ),
            });
        }

        let input_groups = canonicalize_input_boundary_groups(
            &old_to_new,
            input.boundary_triangles,
            input.boundary_face_group_indices,
            input.face_group_count,
        )?;
        let exterior_keys = exterior_facets.keys().copied().collect::<BTreeSet<_>>();
        let input_keys = input_groups.keys().copied().collect::<BTreeSet<_>>();
        if exterior_keys != input_keys {
            let missing = exterior_keys.difference(&input_keys).count();
            let unexpected = input_keys.difference(&exterior_keys).count();
            return Err(FemValidationError {
                field: "boundaryTriangles".to_string(),
                message: format!(
                    "exterior facet coverage mismatch: missing {missing}, unexpected {unexpected}"
                ),
            });
        }

        let mut boundary_triangles = Vec::with_capacity(exterior_facets.len());
        let mut boundary_face_group_indices = Vec::with_capacity(exterior_facets.len());
        let mut boundary_area_mm2_by_group = vec![0.0; input.face_group_count as usize];
        for (key, triangle) in exterior_facets {
            let group = input_groups[&key];
            boundary_area_mm2_by_group[group as usize] +=
                indexed_triangle_area_mm2(&nodes, triangle)?;
            boundary_triangles.push(triangle);
            boundary_face_group_indices.push(group);
        }
        if let Some(group) = boundary_area_mm2_by_group
            .iter()
            .position(|area| !area.is_finite() || *area <= 0.0)
        {
            return Err(FemValidationError {
                field: "boundaryFaceGroupIndices".to_string(),
                message: format!("face group {group} has no positive exterior area"),
            });
        }

        let mut minimum_signed_volume_mm3 = f64::INFINITY;
        let mut maximum_signed_volume_mm3 = 0.0_f64;
        let mut minimum_scaled_jacobian = f64::INFINITY;
        let mut minimum_radius_ratio = f64::INFINITY;
        let mut worst_cell_index = 0_u32;
        for (cell_index, cell) in cells.iter().copied().enumerate() {
            let volume = indexed_tet_signed_volume_mm3(&nodes, cell)?;
            let quality = indexed_tet_scaled_jacobian(&nodes, cell, volume)?;
            let radius_ratio = indexed_tet_radius_ratio(&nodes, cell, volume)?;
            minimum_signed_volume_mm3 = minimum_signed_volume_mm3.min(volume);
            maximum_signed_volume_mm3 = maximum_signed_volume_mm3.max(volume);
            minimum_radius_ratio = minimum_radius_ratio.min(radius_ratio);
            if quality < minimum_scaled_jacobian {
                minimum_scaled_jacobian = quality;
                worst_cell_index = u32::try_from(cell_index).map_err(|_| FemValidationError {
                    field: "cells".to_string(),
                    message: "cell index exceeds u32 range".to_string(),
                })?;
            }
        }
        if minimum_scaled_jacobian < input.minimum_scaled_jacobian {
            return Err(FemValidationError {
                field: "quality.minimumScaledJacobian".to_string(),
                message: format!(
                    "worst element {worst_cell_index} scaled Jacobian {minimum_scaled_jacobian} is below threshold {}",
                    input.minimum_scaled_jacobian
                ),
            });
        }
        let worst_cell_centroid_mm = indexed_tet_centroid(&nodes, cells[worst_cell_index as usize]);
        let quality = FemMeshQuality {
            minimum_signed_volume_mm3,
            maximum_signed_volume_mm3,
            minimum_scaled_jacobian,
            minimum_radius_ratio,
            worst_cell_index,
            worst_cell_centroid_mm,
            connected_component_count,
            boundary_area_mm2_by_group,
        };
        let mut mesh = Self {
            schema_version: input.schema_version,
            nodes,
            cells,
            boundary_triangles,
            boundary_face_group_indices,
            face_group_count: input.face_group_count,
            face_group_targets: input.face_group_targets,
            source_boundary_digest: input.source_boundary_digest,
            mesher_identity: input.mesher_identity,
            meshing_evidence: input.meshing_evidence,
            quality,
            content_digest: String::new(),
        };
        mesh.content_digest = stable_digest(&FemVolumeMeshDigestView {
            schema_version: mesh.schema_version,
            nodes: &mesh.nodes,
            cells: &mesh.cells,
            boundary_triangles: &mesh.boundary_triangles,
            boundary_face_group_indices: &mesh.boundary_face_group_indices,
            face_group_count: mesh.face_group_count,
            face_group_targets: &mesh.face_group_targets,
            source_boundary_digest: &mesh.source_boundary_digest,
            mesher_identity: &mesh.mesher_identity,
            meshing_evidence: &mesh.meshing_evidence,
            quality: &mesh.quality,
        });
        Ok(mesh)
    }
}

impl CanonicalDigest for FemVolumeMesh {
    fn canonical_digest(&self) -> String {
        self.content_digest.clone()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum FemSafetyFactor {
    Finite { value: f64 },
    Infinite,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FemResultFieldKind {
    DisplacementMagnitude,
    VonMisesStress,
    PrincipalStressMaximum,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemResultExtremum {
    pub field_kind: FemResultFieldKind,
    pub value: f64,
    pub unit: String,
    pub node_id: Option<u32>,
    pub element_id: Option<u32>,
    pub coordinate_mm: FemPoint3,
    pub mesh_content_digest: String,
    pub source_boundary_digest: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemElementResult {
    pub element_id: u32,
    pub centroid_mm: FemPoint3,
    pub volume_mm3: f64,
    pub strain: Tet4VoigtVector,
    pub stress_mpa: Tet4VoigtVector,
    pub von_mises_mpa: f64,
    pub principal_stress_mpa: [f64; 3],
    pub yield_safety_factor: FemSafetyFactor,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemNodalDisplayResult {
    pub node_id: u32,
    pub coordinate_mm: FemPoint3,
    pub displacement_mm: FemPoint3,
    pub displacement_magnitude_mm: f64,
    pub volume_weighted_stress_mpa: Tet4VoigtVector,
    pub volume_weighted_von_mises_mpa: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemResultSummary {
    pub maximum_displacement: FemResultExtremum,
    pub maximum_von_mises: FemResultExtremum,
    pub maximum_principal_stress: FemResultExtremum,
    pub volume_mm3: f64,
    pub mass_kg: f64,
    pub minimum_yield_safety_factor: FemSafetyFactor,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemResultIdentity {
    pub mesh_content_digest: String,
    pub source_boundary_digest: String,
    pub material_digest: String,
    pub displacement_digest: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemPostprocessResult {
    pub schema_version: u32,
    pub identity: FemResultIdentity,
    pub elements: Vec<FemElementResult>,
    pub nodal_display: Vec<FemNodalDisplayResult>,
    pub summary: FemResultSummary,
    pub result_digest: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FemPostprocessDigestView<'a> {
    schema_version: u32,
    identity: &'a FemResultIdentity,
    elements: &'a [FemElementResult],
    nodal_display: &'a [FemNodalDisplayResult],
    summary: &'a FemResultSummary,
}

pub fn postprocess_linear_static(
    mesh: &FemVolumeMesh,
    material: &FemMaterial,
    displacement_dofs: &[f64],
) -> Result<FemPostprocessResult, FemValidationError> {
    postprocess_linear_static_with_observer(mesh, material, displacement_dofs, |_| Ok(()))
}

pub fn postprocess_linear_static_with_observer<F>(
    mesh: &FemVolumeMesh,
    material: &FemMaterial,
    displacement_dofs: &[f64],
    mut observe_chunk: F,
) -> Result<FemPostprocessResult, FemValidationError>
where
    F: FnMut(usize) -> Result<(), FemValidationError>,
{
    material.validate()?;
    if mesh.schema_version != FEM_SCHEMA_VERSION || mesh.content_digest.trim().is_empty() {
        return Err(FemValidationError {
            field: "mesh".to_string(),
            message: "must be a versioned canonical volume mesh".to_string(),
        });
    }
    let expected_dofs = mesh
        .nodes
        .len()
        .checked_mul(3)
        .ok_or_else(|| FemValidationError {
            field: "displacementDofs".to_string(),
            message: "DOF count overflowed".to_string(),
        })?;
    if displacement_dofs.len() != expected_dofs {
        return Err(FemValidationError {
            field: "displacementDofs".to_string(),
            message: format!(
                "length {} differs from expected {expected_dofs}",
                displacement_dofs.len()
            ),
        });
    }
    for value in displacement_dofs {
        finite("displacementDofs.value", *value)?;
    }

    let assembler = ElementAssembler;
    let mut elements = Vec::with_capacity(mesh.cells.len());
    let mut nodal_stress_sum = vec![[0.0; 6]; mesh.nodes.len()];
    let mut nodal_volume = vec![0.0; mesh.nodes.len()];
    let mut total_volume_mm3 = 0.0;
    for (element_index, cell) in mesh.cells.iter().copied().enumerate() {
        if element_index % 256 == 0 {
            observe_chunk(element_index)?;
        }
        let node_indices = cell.map(|index| index as usize);
        let element = Tet4Element::new(node_indices.map(|index| mesh.nodes[index]));
        let nodal_displacements = node_indices.map(|index| {
            FemPoint3::new(
                displacement_dofs[index * 3],
                displacement_dofs[index * 3 + 1],
                displacement_dofs[index * 3 + 2],
            )
        });
        let strain = assembler.strain_from_displacements(&element, &nodal_displacements)?;
        let stress_mpa = assembler.stress_from_strain(material, strain)?;
        let volume_mm3 = assembler.signed_volume_mm3(&element)?;
        if !volume_mm3.is_finite() || volume_mm3 <= 0.0 {
            return Err(FemValidationError {
                field: format!("cells[{element_index}]"),
                message: "post-processing requires positive Tet4 volume".to_string(),
            });
        }
        let von_mises_mpa = von_mises_stress(stress_mpa)?;
        let principal_stress_mpa = principal_stresses(stress_mpa)?;
        let yield_safety_factor = yield_safety_factor(material.yield_strength_mpa, von_mises_mpa)?;
        let centroid_mm = indexed_tet_centroid(&mesh.nodes, cell);
        let element_id = u32::try_from(element_index).map_err(|_| FemValidationError {
            field: "elements".to_string(),
            message: "element index exceeds u32 range".to_string(),
        })?;
        elements.push(FemElementResult {
            element_id,
            centroid_mm,
            volume_mm3,
            strain,
            stress_mpa,
            von_mises_mpa,
            principal_stress_mpa,
            yield_safety_factor,
        });
        total_volume_mm3 += volume_mm3;
        for node_index in node_indices {
            nodal_volume[node_index] += volume_mm3;
            for component in 0..6 {
                nodal_stress_sum[node_index][component] += stress_mpa[component] * volume_mm3;
            }
        }
    }

    let mut nodal_display = Vec::with_capacity(mesh.nodes.len());
    for (node_index, coordinate_mm) in mesh.nodes.iter().copied().enumerate() {
        if nodal_volume[node_index] <= 0.0 {
            return Err(FemValidationError {
                field: format!("nodes[{node_index}]"),
                message: "node is not owned by any Tet4 element".to_string(),
            });
        }
        let displacement_mm = FemPoint3::new(
            displacement_dofs[node_index * 3],
            displacement_dofs[node_index * 3 + 1],
            displacement_dofs[node_index * 3 + 2],
        );
        let displacement_magnitude_mm = vector_norm([
            displacement_mm.x_mm,
            displacement_mm.y_mm,
            displacement_mm.z_mm,
        ]);
        let volume_weighted_stress_mpa =
            nodal_stress_sum[node_index].map(|value| value / nodal_volume[node_index]);
        let volume_weighted_von_mises_mpa = von_mises_stress(volume_weighted_stress_mpa)?;
        nodal_display.push(FemNodalDisplayResult {
            node_id: u32::try_from(node_index).map_err(|_| FemValidationError {
                field: "nodes".to_string(),
                message: "node index exceeds u32 range".to_string(),
            })?,
            coordinate_mm,
            displacement_mm,
            displacement_magnitude_mm,
            volume_weighted_stress_mpa,
            volume_weighted_von_mises_mpa,
        });
    }
    let maximum_displacement_node = nodal_display
        .iter()
        .max_by(|left, right| {
            left.displacement_magnitude_mm
                .total_cmp(&right.displacement_magnitude_mm)
                .then_with(|| right.node_id.cmp(&left.node_id))
        })
        .expect("canonical mesh has nodes");
    let maximum_von_mises_element = elements
        .iter()
        .max_by(|left, right| {
            left.von_mises_mpa
                .total_cmp(&right.von_mises_mpa)
                .then_with(|| right.element_id.cmp(&left.element_id))
        })
        .expect("canonical mesh has elements");
    let maximum_principal_element = elements
        .iter()
        .max_by(|left, right| {
            left.principal_stress_mpa[0]
                .total_cmp(&right.principal_stress_mpa[0])
                .then_with(|| right.element_id.cmp(&left.element_id))
        })
        .expect("canonical mesh has elements");
    let minimum_yield_safety_factor = elements
        .iter()
        .filter_map(|element| match element.yield_safety_factor {
            FemSafetyFactor::Finite { value } => Some(value),
            FemSafetyFactor::Infinite => None,
        })
        .min_by(f64::total_cmp)
        .map(|value| FemSafetyFactor::Finite { value })
        .unwrap_or(FemSafetyFactor::Infinite);
    let extremum =
        |field_kind, value, unit: &str, node_id, element_id, coordinate_mm| FemResultExtremum {
            field_kind,
            value,
            unit: unit.to_string(),
            node_id,
            element_id,
            coordinate_mm,
            mesh_content_digest: mesh.content_digest.clone(),
            source_boundary_digest: mesh.source_boundary_digest.clone(),
        };
    let summary = FemResultSummary {
        maximum_displacement: extremum(
            FemResultFieldKind::DisplacementMagnitude,
            maximum_displacement_node.displacement_magnitude_mm,
            "mm",
            Some(maximum_displacement_node.node_id),
            None,
            maximum_displacement_node.coordinate_mm,
        ),
        maximum_von_mises: extremum(
            FemResultFieldKind::VonMisesStress,
            maximum_von_mises_element.von_mises_mpa,
            "MPa",
            None,
            Some(maximum_von_mises_element.element_id),
            maximum_von_mises_element.centroid_mm,
        ),
        maximum_principal_stress: extremum(
            FemResultFieldKind::PrincipalStressMaximum,
            maximum_principal_element.principal_stress_mpa[0],
            "MPa",
            None,
            Some(maximum_principal_element.element_id),
            maximum_principal_element.centroid_mm,
        ),
        volume_mm3: total_volume_mm3,
        mass_kg: total_volume_mm3 * material.density_kg_per_mm3,
        minimum_yield_safety_factor,
    };
    let identity = FemResultIdentity {
        mesh_content_digest: mesh.content_digest.clone(),
        source_boundary_digest: mesh.source_boundary_digest.clone(),
        material_digest: material.canonical_digest(),
        displacement_digest: stable_digest(&displacement_dofs),
    };
    let mut result = FemPostprocessResult {
        schema_version: FEM_SCHEMA_VERSION,
        identity,
        elements,
        nodal_display,
        summary,
        result_digest: String::new(),
    };
    result.result_digest = stable_digest(&FemPostprocessDigestView {
        schema_version: result.schema_version,
        identity: &result.identity,
        elements: &result.elements,
        nodal_display: &result.nodal_display,
        summary: &result.summary,
    });
    Ok(result)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemSupportDofGroup {
    pub name: String,
    pub face_group_indices: Vec<u32>,
    pub dof_indices: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemBoundaryConditionAssembly {
    pub rhs_n: Vec<f64>,
    pub dirichlet: Vec<FemDirichletConstraint>,
    pub support_groups: Vec<FemSupportDofGroup>,
}

pub fn assemble_boundary_conditions(
    mesh: &FemVolumeMesh,
    loads: &[FemLoad],
    constraints: &[FemConstraint],
) -> Result<FemBoundaryConditionAssembly, FemValidationError> {
    if mesh.schema_version != FEM_SCHEMA_VERSION || mesh.nodes.is_empty() || mesh.cells.is_empty() {
        return Err(FemValidationError {
            field: "mesh".to_string(),
            message: "must be a non-empty canonical volume mesh".to_string(),
        });
    }
    let dof_count = mesh
        .nodes
        .len()
        .checked_mul(3)
        .ok_or_else(|| FemValidationError {
            field: "mesh.nodes".to_string(),
            message: "DOF count overflowed".to_string(),
        })?;
    let mut rhs_n = vec![0.0; dof_count];
    let assembler = ElementAssembler;
    for load in loads {
        load.validate()?;
        match load {
            FemLoad::SurfaceForce {
                faces,
                total_force_n,
                ..
            } => {
                let group_indices = resolve_mesh_face_groups(mesh, faces)?;
                let triangles = selected_surface_triangles(mesh, &group_indices)?;
                let nodal_forces =
                    assembler.distribute_total_surface_force(&triangles, *total_force_n)?;
                add_surface_nodal_forces(mesh, &group_indices, &nodal_forces, &mut rhs_n)?;
            }
            FemLoad::Traction {
                faces,
                traction_mpa,
                ..
            } => {
                let group_indices = resolve_mesh_face_groups(mesh, faces)?;
                for (triangle, indexed) in selected_indexed_triangles(mesh, &group_indices)? {
                    let forces = assembler.integrate_triangle_traction(
                        &triangle,
                        [traction_mpa.x_mpa, traction_mpa.y_mpa, traction_mpa.z_mpa],
                    )?;
                    add_triangle_forces(indexed, forces, &mut rhs_n);
                }
            }
            FemLoad::Pressure {
                faces,
                pressure_mpa,
                ..
            } => {
                let group_indices = resolve_mesh_face_groups(mesh, faces)?;
                for (triangle, indexed) in selected_indexed_triangles(mesh, &group_indices)? {
                    let forces = assembler.integrate_triangle_pressure(&triangle, *pressure_mpa)?;
                    add_triangle_forces(indexed, forces, &mut rhs_n);
                }
            }
        }
    }
    if rhs_n.iter().any(|value| !value.is_finite()) {
        return Err(FemValidationError {
            field: "rhsN".to_string(),
            message: "assembled load vector contains a non-finite value".to_string(),
        });
    }

    let mut constrained_dofs = BTreeMap::<usize, f64>::new();
    let mut support_groups = Vec::with_capacity(constraints.len());
    for constraint in constraints {
        constraint.validate()?;
        let (name, faces, components) = match constraint {
            FemConstraint::Fixed { name, faces, .. } => {
                (name, faces, [Some(0.0), Some(0.0), Some(0.0)])
            }
            FemConstraint::PrescribedDisplacement {
                name,
                faces,
                displacement_mm,
                ..
            } => (
                name,
                faces,
                [
                    displacement_mm.x_mm,
                    displacement_mm.y_mm,
                    displacement_mm.z_mm,
                ],
            ),
        };
        let group_indices = resolve_mesh_face_groups(mesh, faces)?;
        let mut node_indices = BTreeSet::<u32>::new();
        for triangle in mesh
            .boundary_triangles
            .iter()
            .zip(&mesh.boundary_face_group_indices)
            .filter_map(|(triangle, group)| group_indices.contains(group).then_some(triangle))
        {
            node_indices.extend(triangle.iter().copied());
        }
        if node_indices.is_empty() {
            return Err(FemValidationError {
                field: "constraints.faces".to_string(),
                message: format!("constraint '{name}' resolved to no boundary nodes"),
            });
        }
        let mut dof_indices = Vec::new();
        for node_index in node_indices {
            for (component, prescribed) in components.iter().enumerate() {
                let Some(value) = prescribed else {
                    continue;
                };
                let dof_index = node_index as usize * 3 + component;
                if let Some(existing) = constrained_dofs.insert(dof_index, *value) {
                    if existing != *value {
                        return Err(FemValidationError {
                            field: "constraints".to_string(),
                            message: format!(
                                "constraint '{name}' conflicts at DOF {dof_index}: {existing} versus {value} mm"
                            ),
                        });
                    }
                }
                dof_indices.push(dof_index);
            }
        }
        dof_indices.sort_unstable();
        dof_indices.dedup();
        support_groups.push(FemSupportDofGroup {
            name: name.clone(),
            face_group_indices: group_indices.into_iter().collect(),
            dof_indices,
        });
    }
    let dirichlet = constrained_dofs
        .into_iter()
        .map(|(dof_index, value_mm)| FemDirichletConstraint {
            dof_index,
            value_mm,
        })
        .collect();
    Ok(FemBoundaryConditionAssembly {
        rhs_n,
        dirichlet,
        support_groups,
    })
}

fn resolve_mesh_face_groups(
    mesh: &FemVolumeMesh,
    targets: &[FemFaceTarget],
) -> Result<BTreeSet<u32>, FemValidationError> {
    let mut resolved = BTreeSet::new();
    for target in targets {
        target.validate()?;
        let matches = mesh
            .face_group_targets
            .iter()
            .enumerate()
            .filter(|(_, candidate)| *candidate == target)
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        if matches.len() != 1 {
            return Err(FemValidationError {
                field: "faces".to_string(),
                message: format!(
                    "target '{}'/'{}' did not resolve exactly once in volume-mesh face groups; observed {} matches",
                    target.canonical_target_id,
                    target.durable_target_id,
                    matches.len()
                ),
            });
        }
        resolved.insert(matches[0] as u32);
    }
    Ok(resolved)
}

fn selected_surface_triangles(
    mesh: &FemVolumeMesh,
    group_indices: &BTreeSet<u32>,
) -> Result<Vec<FemSurfaceTriangle>, FemValidationError> {
    Ok(selected_indexed_triangles(mesh, group_indices)?
        .into_iter()
        .map(|(triangle, _)| triangle)
        .collect())
}

fn selected_indexed_triangles(
    mesh: &FemVolumeMesh,
    group_indices: &BTreeSet<u32>,
) -> Result<Vec<(FemSurfaceTriangle, [u32; 3])>, FemValidationError> {
    let selected = mesh
        .boundary_triangles
        .iter()
        .zip(&mesh.boundary_face_group_indices)
        .filter_map(|(triangle, group)| group_indices.contains(group).then_some(*triangle))
        .map(|triangle| {
            Ok((
                FemSurfaceTriangle {
                    schema_version: FEM_SCHEMA_VERSION,
                    nodes: triangle.map(|index| mesh.nodes[index as usize]),
                },
                triangle,
            ))
        })
        .collect::<Result<Vec<_>, FemValidationError>>()?;
    if selected.is_empty() {
        return Err(FemValidationError {
            field: "faces".to_string(),
            message: "resolved face groups contain no boundary triangles".to_string(),
        });
    }
    Ok(selected)
}

fn add_surface_nodal_forces(
    mesh: &FemVolumeMesh,
    group_indices: &BTreeSet<u32>,
    nodal_forces: &[[FemForceVector; 3]],
    rhs_n: &mut [f64],
) -> Result<(), FemValidationError> {
    let indexed = selected_indexed_triangles(mesh, group_indices)?;
    if indexed.len() != nodal_forces.len() {
        return Err(FemValidationError {
            field: "surfaceForces".to_string(),
            message: "triangle/force cardinality mismatch".to_string(),
        });
    }
    for ((_, triangle), forces) in indexed.into_iter().zip(nodal_forces) {
        add_triangle_forces(triangle, *forces, rhs_n);
    }
    Ok(())
}

fn add_triangle_forces(triangle: [u32; 3], forces: [FemForceVector; 3], rhs_n: &mut [f64]) {
    for (node_index, force) in triangle.into_iter().zip(forces) {
        let base = node_index as usize * 3;
        rhs_n[base] += force.x_n;
        rhs_n[base + 1] += force.y_n;
        rhs_n[base + 2] += force.z_n;
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemEquilibriumEvidence {
    pub applied_resultant_n: [f64; 3],
    pub reaction_resultant_n: [f64; 3],
    pub imbalance_n: [f64; 3],
    pub relative_imbalance: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemSupportReactionResult {
    pub name: String,
    pub face_group_indices: Vec<u32>,
    pub resultant_n: [f64; 3],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemLinearStaticSolution {
    pub schema_version: u32,
    pub mesh_content_digest: String,
    pub displacement_dofs_mm: Vec<f64>,
    pub linear_solve: FemLinearSolveResult,
    pub dof_reactions: Vec<FemDofReaction>,
    pub support_reactions: Vec<FemSupportReactionResult>,
    pub equilibrium: FemEquilibriumEvidence,
    pub strain_energy_n_mm: f64,
    pub postprocess: FemPostprocessResult,
    pub solution_digest: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FemSolveStage {
    Assemble,
    ApplyConstraints,
    Solve,
    Postprocess,
    Verify,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FemLinearStaticSolutionDigestView<'a> {
    schema_version: u32,
    mesh_content_digest: &'a str,
    displacement_dofs_mm: &'a [f64],
    linear_solve: &'a FemLinearSolveResult,
    dof_reactions: &'a [FemDofReaction],
    support_reactions: &'a [FemSupportReactionResult],
    equilibrium: &'a FemEquilibriumEvidence,
    strain_energy_n_mm: f64,
    postprocess: &'a FemPostprocessResult,
}

pub fn solve_linear_static(
    mesh: &FemVolumeMesh,
    material: &FemMaterial,
    loads: &[FemLoad],
    constraints: &[FemConstraint],
    relative_tolerance: f64,
    maximum_dimension: usize,
) -> Result<FemLinearStaticSolution, FemValidationError> {
    solve_linear_static_with_observer(
        mesh,
        material,
        loads,
        constraints,
        relative_tolerance,
        maximum_dimension,
        |_| Ok(()),
    )
}

pub fn solve_linear_static_with_observer<F>(
    mesh: &FemVolumeMesh,
    material: &FemMaterial,
    loads: &[FemLoad],
    constraints: &[FemConstraint],
    relative_tolerance: f64,
    maximum_dimension: usize,
    observe: F,
) -> Result<FemLinearStaticSolution, FemValidationError>
where
    F: FnMut(FemSolveStage) -> Result<(), FemValidationError>,
{
    solve_linear_static_with_solver_and_observer(
        mesh,
        material,
        loads,
        constraints,
        relative_tolerance,
        maximum_dimension,
        &FaerSparseCholeskySolver,
        observe,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn solve_linear_static_with_solver_and_observer<S, F>(
    mesh: &FemVolumeMesh,
    material: &FemMaterial,
    loads: &[FemLoad],
    constraints: &[FemConstraint],
    relative_tolerance: f64,
    maximum_dimension: usize,
    solver: &S,
    mut observe: F,
) -> Result<FemLinearStaticSolution, FemValidationError>
where
    S: LinearSolver,
    F: FnMut(FemSolveStage) -> Result<(), FemValidationError>,
{
    material.validate()?;
    positive_finite("relativeTolerance", relative_tolerance)?;
    observe(FemSolveStage::Assemble)?;
    let indexed_mesh = FemIndexedTet4Mesh {
        schema_version: FEM_SCHEMA_VERSION,
        nodes: mesh.nodes.clone(),
        cells: mesh.cells.clone(),
    };
    let stiffness = ElementAssembler.assemble_global_stiffness_with_observer(
        &indexed_mesh,
        material,
        |_| observe(FemSolveStage::Assemble),
    )?;
    observe(FemSolveStage::ApplyConstraints)?;
    let boundary_conditions = assemble_boundary_conditions(mesh, loads, constraints)?;
    validate_rigid_body_constraints(mesh, &boundary_conditions.dirichlet)?;
    let reduced = stiffness
        .eliminate_dirichlet(&boundary_conditions.rhs_n, &boundary_conditions.dirichlet)?;
    observe(FemSolveStage::Solve)?;
    let linear_solve = solver.solve(
        &reduced.matrix,
        &reduced.rhs,
        relative_tolerance,
        maximum_dimension,
    )?;
    let displacement_dofs_mm = reduced.recover_full_solution(&linear_solve.solution)?;
    let dof_reactions = reduced.recover_support_reactions(&linear_solve.solution)?;
    let equilibrium = equilibrium_evidence(&boundary_conditions.rhs_n, &dof_reactions)?;
    if equilibrium.relative_imbalance > relative_tolerance {
        return Err(FemValidationError {
            field: "equilibrium.relativeImbalance".to_string(),
            message: format!(
                "applied plus reaction imbalance {} exceeds tolerance {relative_tolerance}; applied={:?}, reactions={:?}",
                equilibrium.relative_imbalance,
                equilibrium.applied_resultant_n,
                equilibrium.reaction_resultant_n
            ),
        });
    }
    let stiffness_entries = stiffness.validated_entries()?;
    let strain_energy_n_mm = 0.5
        * stiffness_entries
            .iter()
            .map(|(&(row, col), value)| {
                displacement_dofs_mm[row] * value * displacement_dofs_mm[col]
            })
            .sum::<f64>();
    let energy_scale = boundary_conditions
        .rhs_n
        .iter()
        .zip(&displacement_dofs_mm)
        .map(|(force, displacement)| (force * displacement).abs())
        .sum::<f64>()
        .max(1.0);
    if !strain_energy_n_mm.is_finite() || strain_energy_n_mm < -64.0 * f64::EPSILON * energy_scale {
        return Err(FemValidationError {
            field: "strainEnergyNmm".to_string(),
            message: format!(
                "strain energy must be finite and non-negative, observed {strain_energy_n_mm}"
            ),
        });
    }
    let reaction_by_dof = dof_reactions
        .iter()
        .map(|reaction| (reaction.dof_index, reaction.reaction_n))
        .collect::<BTreeMap<_, _>>();
    let support_reactions = boundary_conditions
        .support_groups
        .iter()
        .map(|group| {
            let mut resultant_n = [0.0; 3];
            for dof_index in &group.dof_indices {
                resultant_n[*dof_index % 3] +=
                    reaction_by_dof.get(dof_index).copied().unwrap_or(0.0);
            }
            FemSupportReactionResult {
                name: group.name.clone(),
                face_group_indices: group.face_group_indices.clone(),
                resultant_n,
            }
        })
        .collect::<Vec<_>>();
    observe(FemSolveStage::Postprocess)?;
    let postprocess =
        postprocess_linear_static_with_observer(mesh, material, &displacement_dofs_mm, |_| {
            observe(FemSolveStage::Postprocess)
        })?;
    observe(FemSolveStage::Verify)?;
    let mut solution = FemLinearStaticSolution {
        schema_version: FEM_SCHEMA_VERSION,
        mesh_content_digest: mesh.content_digest.clone(),
        displacement_dofs_mm,
        linear_solve,
        dof_reactions,
        support_reactions,
        equilibrium,
        strain_energy_n_mm: strain_energy_n_mm.max(0.0),
        postprocess,
        solution_digest: String::new(),
    };
    solution.solution_digest = stable_digest(&FemLinearStaticSolutionDigestView {
        schema_version: solution.schema_version,
        mesh_content_digest: &solution.mesh_content_digest,
        displacement_dofs_mm: &solution.displacement_dofs_mm,
        linear_solve: &solution.linear_solve,
        dof_reactions: &solution.dof_reactions,
        support_reactions: &solution.support_reactions,
        equilibrium: &solution.equilibrium,
        strain_energy_n_mm: solution.strain_energy_n_mm,
        postprocess: &solution.postprocess,
    });
    Ok(solution)
}

fn validate_rigid_body_constraints(
    mesh: &FemVolumeMesh,
    constraints: &[FemDirichletConstraint],
) -> Result<(), FemValidationError> {
    if constraints.is_empty() {
        return Err(FemValidationError {
            field: "constraints".to_string(),
            message: "underconstrained model has 6 unconstrained DOF rigid-body modes; constrain independent translations and rotations on selected topology"
                .to_string(),
        });
    }
    let centroid = FemPoint3::new(
        mesh.nodes.iter().map(|point| point.x_mm).sum::<f64>() / mesh.nodes.len() as f64,
        mesh.nodes.iter().map(|point| point.y_mm).sum::<f64>() / mesh.nodes.len() as f64,
        mesh.nodes.iter().map(|point| point.z_mm).sum::<f64>() / mesh.nodes.len() as f64,
    );
    let characteristic_length = mesh
        .nodes
        .iter()
        .map(|point| vector_norm(subtract_points(*point, centroid)))
        .fold(0.0_f64, f64::max);
    if !characteristic_length.is_finite() || characteristic_length <= 0.0 {
        return Err(FemValidationError {
            field: "mesh.nodes".to_string(),
            message: "cannot evaluate rigid modes for zero-size mesh".to_string(),
        });
    }
    let mut rows = Vec::<[f64; 6]>::with_capacity(constraints.len());
    for constraint in constraints {
        if constraint.dof_index >= mesh.nodes.len() * 3 {
            return Err(FemValidationError {
                field: "constraints.dofIndex".to_string(),
                message: "is out of range for rigid-mode check".to_string(),
            });
        }
        let point = mesh.nodes[constraint.dof_index / 3];
        let x = (point.x_mm - centroid.x_mm) / characteristic_length;
        let y = (point.y_mm - centroid.y_mm) / characteristic_length;
        let z = (point.z_mm - centroid.z_mm) / characteristic_length;
        rows.push(match constraint.dof_index % 3 {
            0 => [1.0, 0.0, 0.0, 0.0, z, -y],
            1 => [0.0, 1.0, 0.0, -z, 0.0, x],
            _ => [0.0, 0.0, 1.0, y, -x, 0.0],
        });
    }
    let rank = matrix_rank_6(&mut rows);
    if rank < 6 {
        return Err(FemValidationError {
            field: "constraints".to_string(),
            message: format!(
                "underconstrained model has {} unconstrained DOF rigid-body modes (constraint rank {rank}/6); constrain independent translations and rotations on selected topology",
                6 - rank
            ),
        });
    }
    Ok(())
}

fn matrix_rank_6(rows: &mut [[f64; 6]]) -> usize {
    let mut rank = 0;
    for column in 0..6 {
        let pivot = (rank..rows.len()).max_by(|left, right| {
            rows[*left][column]
                .abs()
                .total_cmp(&rows[*right][column].abs())
        });
        let Some(pivot) = pivot else {
            break;
        };
        if rows[pivot][column].abs() <= 1.0e-10 {
            continue;
        }
        rows.swap(rank, pivot);
        let divisor = rows[rank][column];
        for value in &mut rows[rank][column..] {
            *value /= divisor;
        }
        let pivot_row = rows[rank];
        for row in rows.iter_mut().skip(rank + 1) {
            let factor = row[column];
            for inner in column..6 {
                row[inner] -= factor * pivot_row[inner];
            }
        }
        rank += 1;
        if rank == 6 {
            break;
        }
    }
    rank
}

fn equilibrium_evidence(
    rhs_n: &[f64],
    reactions: &[FemDofReaction],
) -> Result<FemEquilibriumEvidence, FemValidationError> {
    if !rhs_n.len().is_multiple_of(3) {
        return Err(FemValidationError {
            field: "rhsN".to_string(),
            message: "length must be divisible by 3".to_string(),
        });
    }
    let mut applied_resultant_n = [0.0; 3];
        for (dof_index, force) in rhs_n.iter().copied().enumerate() {
            finite("rhsN.value", force)?;
            applied_resultant_n[dof_index % 3] += force;
        }
    let mut reaction_resultant_n = [0.0; 3];
    for reaction in reactions {
        finite("reactionN", reaction.reaction_n)?;
        reaction_resultant_n[reaction.dof_index % 3] += reaction.reaction_n;
    }
    let imbalance_n = [
        applied_resultant_n[0] + reaction_resultant_n[0],
        applied_resultant_n[1] + reaction_resultant_n[1],
        applied_resultant_n[2] + reaction_resultant_n[2],
    ];
    let relative_imbalance = vector_norm(imbalance_n) / vector_norm(applied_resultant_n).max(1.0);
    Ok(FemEquilibriumEvidence {
        applied_resultant_n,
        reaction_resultant_n,
        imbalance_n,
        relative_imbalance,
    })
}

fn von_mises_stress(stress: Tet4VoigtVector) -> Result<f64, FemValidationError> {
    for value in stress {
        finite("stressMpa", value)?;
    }
    let [xx, yy, zz, yz, xz, xy] = stress;
    let value = (0.5 * ((xx - yy).powi(2) + (yy - zz).powi(2) + (zz - xx).powi(2))
        + 3.0 * (xy * xy + xz * xz + yz * yz))
        .sqrt();
    finite("vonMisesMpa", value)?;
    Ok(value)
}

fn principal_stresses(stress: Tet4VoigtVector) -> Result<[f64; 3], FemValidationError> {
    for value in stress {
        finite("stressMpa", value)?;
    }
    let [xx, yy, zz, yz, xz, xy] = stress;
    let off_diagonal_energy = xy * xy + xz * xz + yz * yz;
    let mut principal = if off_diagonal_energy == 0.0 {
        [xx, yy, zz]
    } else {
        let mean = (xx + yy + zz) / 3.0;
        let variance = (xx - mean).powi(2)
            + (yy - mean).powi(2)
            + (zz - mean).powi(2)
            + 2.0 * off_diagonal_energy;
        let scale = (variance / 6.0).sqrt();
        if scale == 0.0 {
            [mean; 3]
        } else {
            let a = (xx - mean) / scale;
            let d = (yy - mean) / scale;
            let f = (zz - mean) / scale;
            let b = xy / scale;
            let c = xz / scale;
            let e = yz / scale;
            let determinant = a * (d * f - e * e) - b * (b * f - c * e) + c * (b * e - c * d);
            let angle = (determinant / 2.0).clamp(-1.0, 1.0).acos() / 3.0;
            let first = mean + 2.0 * scale * angle.cos();
            let third = mean + 2.0 * scale * (angle + 2.0 * std::f64::consts::PI / 3.0).cos();
            let second = 3.0 * mean - first - third;
            [first, second, third]
        }
    };
    principal.sort_by(|left, right| right.total_cmp(left));
    Ok(principal)
}

fn yield_safety_factor(
    yield_strength_mpa: f64,
    von_mises_mpa: f64,
) -> Result<FemSafetyFactor, FemValidationError> {
    positive_finite("yieldStrengthMpa", yield_strength_mpa)?;
    finite("vonMisesMpa", von_mises_mpa)?;
    if von_mises_mpa < 0.0 {
        return Err(FemValidationError {
            field: "vonMisesMpa".to_string(),
            message: "must not be negative".to_string(),
        });
    }
    if von_mises_mpa == 0.0 {
        Ok(FemSafetyFactor::Infinite)
    } else {
        Ok(FemSafetyFactor::Finite {
            value: yield_strength_mpa / von_mises_mpa,
        })
    }
}

fn validate_volume_mesh_input_header(input: &FemVolumeMeshInput) -> Result<(), FemValidationError> {
    if input.schema_version != FEM_SCHEMA_VERSION {
        return Err(schema_version_error());
    }
    if input.nodes.len() < 4 {
        return Err(FemValidationError {
            field: "nodes".to_string(),
            message: "must contain at least four nodes".to_string(),
        });
    }
    if input.cells.is_empty() {
        return Err(FemValidationError {
            field: "cells".to_string(),
            message: "must not be empty".to_string(),
        });
    }
    if input.boundary_triangles.len() != input.boundary_face_group_indices.len() {
        return Err(FemValidationError {
            field: "boundaryFaceGroupIndices".to_string(),
            message: "cardinality differs from boundary triangles".to_string(),
        });
    }
    if input.face_group_count == 0 {
        return Err(FemValidationError {
            field: "faceGroupCount".to_string(),
            message: "must be positive".to_string(),
        });
    }
    if input.face_group_targets.len() != input.face_group_count as usize {
        return Err(FemValidationError {
            field: "faceGroupTargets".to_string(),
            message: format!(
                "cardinality {} differs from faceGroupCount {}",
                input.face_group_targets.len(),
                input.face_group_count
            ),
        });
    }
    let mut canonical_ids = BTreeSet::new();
    let mut durable_ids = BTreeSet::new();
    for target in &input.face_group_targets {
        target.validate()?;
        if !canonical_ids.insert(target.canonical_target_id.as_str()) {
            return Err(FemValidationError {
                field: "faceGroupTargets.canonicalTargetId".to_string(),
                message: "contains a duplicate canonical target".to_string(),
            });
        }
        if !durable_ids.insert(target.durable_target_id.as_str()) {
            return Err(FemValidationError {
                field: "faceGroupTargets.durableTargetId".to_string(),
                message: "contains a duplicate durable target".to_string(),
            });
        }
    }
    if input.source_boundary_digest.trim().is_empty() {
        return Err(FemValidationError {
            field: "sourceBoundaryDigest".to_string(),
            message: "must not be empty".to_string(),
        });
    }
    input.mesher_identity.validate()?;
    input.meshing_evidence.validate()?;
    if input.meshing_evidence.tagged_boundary_triangle_count
        != input.boundary_triangles.len() as u64
    {
        return Err(FemValidationError {
            field: "meshingEvidence.taggedBoundaryTriangleCount".to_string(),
            message: format!(
                "observed {}, expected {}",
                input.meshing_evidence.tagged_boundary_triangle_count,
                input.boundary_triangles.len()
            ),
        });
    }
    positive_finite("minimumScaledJacobian", input.minimum_scaled_jacobian)?;
    if input.minimum_scaled_jacobian > 1.0 {
        return Err(FemValidationError {
            field: "minimumScaledJacobian".to_string(),
            message: "must not exceed 1".to_string(),
        });
    }
    Ok(())
}

fn canonicalize_mesh_nodes(
    nodes: Vec<FemPoint3>,
) -> Result<(Vec<FemPoint3>, Vec<u32>), FemValidationError> {
    let mut indexed = nodes
        .into_iter()
        .enumerate()
        .map(|(old_index, node)| {
            validate_point3(&format!("nodes[{old_index}]"), &node)?;
            Ok((
                old_index,
                FemPoint3::new(
                    canonical_zero(node.x_mm),
                    canonical_zero(node.y_mm),
                    canonical_zero(node.z_mm),
                ),
            ))
        })
        .collect::<Result<Vec<_>, FemValidationError>>()?;
    indexed.sort_by(|left, right| {
        left.1
            .x_mm
            .total_cmp(&right.1.x_mm)
            .then(left.1.y_mm.total_cmp(&right.1.y_mm))
            .then(left.1.z_mm.total_cmp(&right.1.z_mm))
    });
    for pair in indexed.windows(2) {
        if pair[0].1 == pair[1].1 {
            return Err(FemValidationError {
                field: "nodes".to_string(),
                message: format!(
                    "contains duplicate coordinates at original nodes {} and {}",
                    pair[0].0, pair[1].0
                ),
            });
        }
    }
    let mut old_to_new = vec![0_u32; indexed.len()];
    let mut canonical = Vec::with_capacity(indexed.len());
    for (new_index, (old_index, node)) in indexed.into_iter().enumerate() {
        let new_index = u32::try_from(new_index).map_err(|_| FemValidationError {
            field: "nodes".to_string(),
            message: "node count exceeds u32 range".to_string(),
        })?;
        old_to_new[old_index] = new_index;
        canonical.push(node);
    }
    Ok((canonical, old_to_new))
}

fn canonicalize_mesh_cells(
    nodes: &[FemPoint3],
    old_to_new: &[u32],
    cells: Vec<[u32; 4]>,
) -> Result<Vec<[u32; 4]>, FemValidationError> {
    let mut result = Vec::with_capacity(cells.len());
    let mut seen = BTreeSet::new();
    for (cell_index, raw_cell) in cells.into_iter().enumerate() {
        let mut cell = [0_u32; 4];
        for (corner, old_index) in raw_cell.into_iter().enumerate() {
            cell[corner] =
                *old_to_new
                    .get(old_index as usize)
                    .ok_or_else(|| FemValidationError {
                        field: "cells".to_string(),
                        message: format!(
                            "cell {cell_index} references out-of-range node {old_index}"
                        ),
                    })?;
        }
        let mut key = cell;
        key.sort_unstable();
        if key.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(FemValidationError {
                field: "cells".to_string(),
                message: format!("cell {cell_index} repeats a node"),
            });
        }
        if !seen.insert(key) {
            return Err(FemValidationError {
                field: "cells".to_string(),
                message: format!("cell {cell_index} duplicates an earlier Tet4"),
            });
        }
        cell = key;
        let volume = indexed_tet_signed_volume_mm3(nodes, cell)?;
        if volume < 0.0 {
            cell.swap(2, 3);
        } else if volume == 0.0 {
            return Err(FemValidationError {
                field: "cells".to_string(),
                message: format!("cell {cell_index} has zero signed volume"),
            });
        }
        result.push(cell);
    }
    Ok(result)
}

type FacetKey = [u32; 3];

fn derive_exterior_facets_and_components(
    cells: &[[u32; 4]],
) -> Result<(BTreeMap<FacetKey, [u32; 3]>, u32), FemValidationError> {
    let mut ownership: BTreeMap<FacetKey, Vec<(usize, [u32; 3])>> = BTreeMap::new();
    for (cell_index, [a, b, c, d]) in cells.iter().copied().enumerate() {
        for triangle in [[b, d, c], [a, c, d], [a, d, b], [a, b, c]] {
            let mut key = triangle;
            key.sort_unstable();
            ownership
                .entry(key)
                .or_default()
                .push((cell_index, triangle));
        }
    }
    let mut neighbours = vec![Vec::new(); cells.len()];
    let mut exterior = BTreeMap::new();
    for (key, owners) in ownership {
        match owners.as_slice() {
            [(cell, triangle)] => {
                let _ = cell;
                exterior.insert(key, *triangle);
            }
            [(left, _), (right, _)] => {
                neighbours[*left].push(*right);
                neighbours[*right].push(*left);
            }
            _ => {
                return Err(FemValidationError {
                    field: "cells".to_string(),
                    message: format!(
                        "facet {key:?} has {} owning Tet4 cells; expected one or two",
                        owners.len()
                    ),
                });
            }
        }
    }
    let mut seen = vec![false; cells.len()];
    let mut component_count = 0_u32;
    for start in 0..cells.len() {
        if seen[start] {
            continue;
        }
        component_count += 1;
        seen[start] = true;
        let mut stack = vec![start];
        while let Some(cell) = stack.pop() {
            for neighbour in &neighbours[cell] {
                if !seen[*neighbour] {
                    seen[*neighbour] = true;
                    stack.push(*neighbour);
                }
            }
        }
    }
    Ok((exterior, component_count))
}

fn canonicalize_input_boundary_groups(
    old_to_new: &[u32],
    triangles: Vec<[u32; 3]>,
    groups: Vec<u32>,
    face_group_count: u32,
) -> Result<BTreeMap<FacetKey, u32>, FemValidationError> {
    let mut result = BTreeMap::new();
    for (triangle_index, (raw_triangle, group)) in triangles.into_iter().zip(groups).enumerate() {
        if group >= face_group_count {
            return Err(FemValidationError {
                field: "boundaryFaceGroupIndices".to_string(),
                message: format!(
                    "boundary triangle {triangle_index} group {group} exceeds face group count {face_group_count}"
                ),
            });
        }
        let mut key = [0_u32; 3];
        for (corner, old_index) in raw_triangle.into_iter().enumerate() {
            key[corner] =
                *old_to_new
                    .get(old_index as usize)
                    .ok_or_else(|| FemValidationError {
                        field: "boundaryTriangles".to_string(),
                        message: format!(
                    "boundary triangle {triangle_index} references out-of-range node {old_index}"
                ),
                    })?;
        }
        key.sort_unstable();
        if key.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(FemValidationError {
                field: "boundaryTriangles".to_string(),
                message: format!("boundary triangle {triangle_index} repeats a node"),
            });
        }
        if result.insert(key, group).is_some() {
            return Err(FemValidationError {
                field: "boundaryTriangles".to_string(),
                message: format!("boundary triangle {triangle_index} is duplicate"),
            });
        }
    }
    Ok(result)
}

fn indexed_tet_signed_volume_mm3(
    nodes: &[FemPoint3],
    cell: [u32; 4],
) -> Result<f64, FemValidationError> {
    let points = cell.map(|index| nodes[index as usize]);
    let ab = subtract_points(points[1], points[0]);
    let ac = subtract_points(points[2], points[0]);
    let ad = subtract_points(points[3], points[0]);
    let determinant = ab[0] * (ac[1] * ad[2] - ac[2] * ad[1])
        - ab[1] * (ac[0] * ad[2] - ac[2] * ad[0])
        + ab[2] * (ac[0] * ad[1] - ac[1] * ad[0]);
    let volume = determinant / 6.0;
    if volume.is_finite() {
        Ok(volume)
    } else {
        Err(FemValidationError {
            field: "cells".to_string(),
            message: "Tet4 signed volume is non-finite".to_string(),
        })
    }
}

fn indexed_tet_scaled_jacobian(
    nodes: &[FemPoint3],
    cell: [u32; 4],
    volume: f64,
) -> Result<f64, FemValidationError> {
    let points = cell.map(|index| nodes[index as usize]);
    let mut maximum_edge_squared = 0.0_f64;
    for left in 0..4 {
        for right in (left + 1)..4 {
            let delta = subtract_points(points[left], points[right]);
            maximum_edge_squared = maximum_edge_squared
                .max(delta[0] * delta[0] + delta[1] * delta[1] + delta[2] * delta[2]);
        }
    }
    let denominator = maximum_edge_squared.sqrt().powi(3);
    let quality = 6.0 * 2.0_f64.sqrt() * volume / denominator;
    if quality.is_finite() && quality > 0.0 {
        Ok(quality.min(1.0))
    } else {
        Err(FemValidationError {
            field: "quality.minimumScaledJacobian".to_string(),
            message: "Tet4 quality is non-finite or non-positive".to_string(),
        })
    }
}

fn indexed_tet_radius_ratio(
    nodes: &[FemPoint3],
    cell: [u32; 4],
    volume: f64,
) -> Result<f64, FemValidationError> {
    let points = cell.map(|index| nodes[index as usize]);
    let surface_area = [
        [cell[1], cell[3], cell[2]],
        [cell[0], cell[2], cell[3]],
        [cell[0], cell[3], cell[1]],
        [cell[0], cell[1], cell[2]],
    ]
    .into_iter()
    .map(|triangle| indexed_triangle_area_mm2(nodes, triangle))
    .sum::<Result<f64, FemValidationError>>()?;
    let inradius = 3.0 * volume / surface_area;
    let edge_rows = [
        subtract_points(points[1], points[0]),
        subtract_points(points[2], points[0]),
        subtract_points(points[3], points[0]),
    ];
    let rhs = edge_rows.map(|edge| 0.5 * dot(edge, edge));
    let determinant = determinant_3x3(edge_rows);
    if !determinant.is_finite() || determinant == 0.0 {
        return Err(FemValidationError {
            field: "quality.minimumRadiusRatio".to_string(),
            message: "Tet4 circumradius solve is singular".to_string(),
        });
    }
    let center = [
        determinant_3x3([
            [rhs[0], edge_rows[0][1], edge_rows[0][2]],
            [rhs[1], edge_rows[1][1], edge_rows[1][2]],
            [rhs[2], edge_rows[2][1], edge_rows[2][2]],
        ]) / determinant,
        determinant_3x3([
            [edge_rows[0][0], rhs[0], edge_rows[0][2]],
            [edge_rows[1][0], rhs[1], edge_rows[1][2]],
            [edge_rows[2][0], rhs[2], edge_rows[2][2]],
        ]) / determinant,
        determinant_3x3([
            [edge_rows[0][0], edge_rows[0][1], rhs[0]],
            [edge_rows[1][0], edge_rows[1][1], rhs[1]],
            [edge_rows[2][0], edge_rows[2][1], rhs[2]],
        ]) / determinant,
    ];
    let circumradius = vector_norm(center);
    let ratio = 3.0 * inradius / circumradius;
    if ratio.is_finite() && ratio > 0.0 {
        Ok(ratio.min(1.0))
    } else {
        Err(FemValidationError {
            field: "quality.minimumRadiusRatio".to_string(),
            message: "Tet4 radius ratio is non-finite or non-positive".to_string(),
        })
    }
}

fn determinant_3x3(matrix: [[f64; 3]; 3]) -> f64 {
    matrix[0][0] * (matrix[1][1] * matrix[2][2] - matrix[1][2] * matrix[2][1])
        - matrix[0][1] * (matrix[1][0] * matrix[2][2] - matrix[1][2] * matrix[2][0])
        + matrix[0][2] * (matrix[1][0] * matrix[2][1] - matrix[1][1] * matrix[2][0])
}

fn indexed_triangle_area_mm2(
    nodes: &[FemPoint3],
    triangle: [u32; 3],
) -> Result<f64, FemValidationError> {
    let a = nodes[triangle[0] as usize];
    let ab = subtract_points(nodes[triangle[1] as usize], a);
    let ac = subtract_points(nodes[triangle[2] as usize], a);
    let cross = [
        ab[1] * ac[2] - ab[2] * ac[1],
        ab[2] * ac[0] - ab[0] * ac[2],
        ab[0] * ac[1] - ab[1] * ac[0],
    ];
    let area = 0.5 * (cross[0] * cross[0] + cross[1] * cross[1] + cross[2] * cross[2]).sqrt();
    if area.is_finite() && area > 0.0 {
        Ok(area)
    } else {
        Err(FemValidationError {
            field: "boundaryTriangles".to_string(),
            message: "contains a non-finite or zero-area triangle".to_string(),
        })
    }
}

fn indexed_tet_centroid(nodes: &[FemPoint3], cell: [u32; 4]) -> FemPoint3 {
    let points = cell.map(|index| nodes[index as usize]);
    FemPoint3::new(
        points.iter().map(|point| point.x_mm).sum::<f64>() / 4.0,
        points.iter().map(|point| point.y_mm).sum::<f64>() / 4.0,
        points.iter().map(|point| point.z_mm).sum::<f64>() / 4.0,
    )
}

fn canonical_zero(value: f64) -> f64 {
    if value == 0.0 {
        0.0
    } else {
        value
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemPoint3 {
    pub x_mm: f64,
    pub y_mm: f64,
    pub z_mm: f64,
}

impl FemPoint3 {
    pub const fn new(x_mm: f64, y_mm: f64, z_mm: f64) -> Self {
        Self { x_mm, y_mm, z_mm }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Tet4Orientation {
    Positive,
    Negative,
    Degenerate,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Tet4Element {
    pub nodes: [FemPoint3; 4],
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemSurfaceTriangle {
    pub schema_version: u32,
    pub nodes: [FemPoint3; 3],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemIndexedTet4Mesh {
    pub schema_version: u32,
    pub nodes: Vec<FemPoint3>,
    pub cells: Vec<[u32; 4]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemSparseEntry {
    pub row: usize,
    pub col: usize,
    pub value: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemSparseMatrix {
    pub dimension: usize,
    pub entries: Vec<FemSparseEntry>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemLinearSolveResult {
    pub solution: Vec<f64>,
    pub residual_l2: f64,
    pub relative_residual: f64,
    pub solver_identity: FemLinearSolverIdentity,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemLinearSolverIdentity {
    pub backend: String,
    pub backend_version: String,
    pub factorization: String,
    pub ordering: String,
    pub scalar_type: String,
    pub parallelism: String,
    pub relative_tolerance: f64,
}

pub trait LinearSolver {
    fn solve(
        &self,
        matrix: &FemSparseMatrix,
        rhs: &[f64],
        relative_tolerance: f64,
        maximum_dimension: usize,
    ) -> Result<FemLinearSolveResult, FemValidationError>;
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct FaerSparseCholeskySolver;

impl LinearSolver for FaerSparseCholeskySolver {
    fn solve(
        &self,
        matrix: &FemSparseMatrix,
        rhs: &[f64],
        relative_tolerance: f64,
        maximum_dimension: usize,
    ) -> Result<FemLinearSolveResult, FemValidationError> {
        use faer::prelude::Solve;
        use faer::sparse::{SparseColMat, Triplet};

        positive_finite("relativeTolerance", relative_tolerance)?;
        if maximum_dimension == 0 || matrix.dimension > maximum_dimension {
            return Err(FemValidationError {
                field: "matrix.dimension".to_string(),
                message: format!(
                    "solve dimension {} exceeds budget {}",
                    matrix.dimension, maximum_dimension
                ),
            });
        }
        let entries = matrix.validated_entries()?;
        if rhs.len() != matrix.dimension {
            return Err(FemValidationError {
                field: "rhs".to_string(),
                message: "length differs from matrix dimension".to_string(),
            });
        }
        for value in rhs {
            finite("rhs.value", *value)?;
        }
        let scale = entries
            .values()
            .map(|value| value.abs())
            .fold(1.0_f64, f64::max);
        let symmetry_tolerance = 64.0 * f64::EPSILON * scale;
        for (&(row, col), value) in &entries {
            let transpose = entries.get(&(col, row)).copied().unwrap_or(0.0);
            if (*value - transpose).abs() > symmetry_tolerance {
                return Err(FemValidationError {
                    field: "matrix".to_string(),
                    message: format!(
                        "must be symmetric; entries ({row},{col}) and ({col},{row}) differ"
                    ),
                });
            }
        }
        let triplets = entries
            .iter()
            .filter(|(&(row, col), _)| row <= col)
            .map(|(&(row, col), &value)| Triplet::new(row, col, value))
            .collect::<Vec<_>>();
        let sparse = SparseColMat::<usize, f64>::try_new_from_triplets(
            matrix.dimension,
            matrix.dimension,
            &triplets,
        )
        .map_err(|error| FemValidationError {
            field: "matrix".to_string(),
            message: format!("Faer sparse matrix creation failed: {error:?}"),
        })?;
        let factor = sparse
            .sp_cholesky(faer::Side::Upper)
            .map_err(|error| FemValidationError {
                field: "matrix".to_string(),
                message: format!(
                    "Faer sparse Cholesky rejected a non-positive-definite or singular matrix: {error:?}"
                ),
            })?;
        let rhs_column = faer::Col::from_fn(matrix.dimension, |row| rhs[row]);
        let solved = factor.solve(&rhs_column);
        let solution = (0..matrix.dimension)
            .map(|row| solved[row])
            .collect::<Vec<f64>>();
        if solution.iter().any(|value| !value.is_finite()) {
            return Err(FemValidationError {
                field: "solution".to_string(),
                message: "Faer returned a non-finite value".to_string(),
            });
        }
        let mut residual = rhs.iter().map(|value| -*value).collect::<Vec<_>>();
        for (&(row, col), coefficient) in &entries {
            residual[row] += coefficient * solution[col];
        }
        let residual_l2 = residual
            .iter()
            .map(|value| value * value)
            .sum::<f64>()
            .sqrt();
        let rhs_l2 = rhs.iter().map(|value| value * value).sum::<f64>().sqrt();
        let relative_residual = residual_l2 / rhs_l2.max(1.0);
        if !relative_residual.is_finite() || relative_residual > relative_tolerance {
            return Err(FemValidationError {
                field: "solution.relativeResidual".to_string(),
                message: format!(
                    "Faer relative residual {relative_residual} exceeds tolerance {relative_tolerance}"
                ),
            });
        }
        Ok(FemLinearSolveResult {
            solution,
            residual_l2,
            relative_residual,
            solver_identity: FemLinearSolverIdentity {
                backend: "faer".to_string(),
                backend_version: "0.24.4".to_string(),
                factorization: "sparse-llt".to_string(),
                ordering: "faer-default-amd".to_string(),
                scalar_type: "f64".to_string(),
                parallelism: "sequential".to_string(),
                relative_tolerance,
            },
        })
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ReferenceCholeskySolver;

impl LinearSolver for ReferenceCholeskySolver {
    fn solve(
        &self,
        matrix: &FemSparseMatrix,
        rhs: &[f64],
        relative_tolerance: f64,
        maximum_dimension: usize,
    ) -> Result<FemLinearSolveResult, FemValidationError> {
        positive_finite("relativeTolerance", relative_tolerance)?;
        if maximum_dimension == 0 || matrix.dimension > maximum_dimension {
            return Err(FemValidationError {
                field: "matrix.dimension".to_string(),
                message: format!(
                    "solve dimension {} exceeds budget {}",
                    matrix.dimension, maximum_dimension
                ),
            });
        }
        matrix.validated_entries()?;
        if rhs.len() != matrix.dimension {
            return Err(FemValidationError {
                field: "rhs".to_string(),
                message: "length differs from matrix dimension".to_string(),
            });
        }
        for value in rhs {
            finite("rhs.value", *value)?;
        }
        let dense = matrix.to_dense();
        let scale = dense
            .iter()
            .flatten()
            .map(|value| value.abs())
            .fold(1.0, f64::max);
        let symmetry_tolerance = 64.0 * f64::EPSILON * scale;
        for (row, row_values) in dense.iter().enumerate() {
            for (col, value) in row_values.iter().take(row).enumerate() {
                if (*value - dense[col][row]).abs() > symmetry_tolerance {
                    return Err(FemValidationError {
                        field: "matrix".to_string(),
                        message: format!(
                            "must be symmetric; entries ({row},{col}) and ({col},{row}) differ"
                        ),
                    });
                }
            }
        }

        let mut lower = vec![vec![0.0; matrix.dimension]; matrix.dimension];
        let pivot_tolerance = 64.0 * f64::EPSILON * scale * matrix.dimension.max(1) as f64;
        for (row, _) in dense.iter().enumerate().take(matrix.dimension) {
            let (previous_rows, current_and_tail) = lower.split_at_mut(row);
            let current_row = &mut current_and_tail[0];
            for col in 0..=row {
                let mut value = dense[row][col];
                for (inner, &lower_value) in current_row.iter().take(col).enumerate() {
                    value -= lower_value * previous_rows[col][inner];
                }
                if row == col {
                    if !value.is_finite() || value <= pivot_tolerance {
                        return Err(FemValidationError {
                            field: "matrix".to_string(),
                            message: format!(
                                "Cholesky pivot {row} is not positive definite: {value}"
                            ),
                        });
                    }
                    current_row[col] = value.sqrt();
                } else {
                    current_row[col] = value / previous_rows[col][col];
                }
            }
        }

        let mut intermediate = vec![0.0; matrix.dimension];
        for row in 0..matrix.dimension {
            let prior = (0..row)
                .map(|col| lower[row][col] * intermediate[col])
                .sum::<f64>();
            intermediate[row] = (rhs[row] - prior) / lower[row][row];
        }
        let mut solution = vec![0.0; matrix.dimension];
        for row in (0..matrix.dimension).rev() {
            let later = (row + 1..matrix.dimension)
                .map(|col| lower[col][row] * solution[col])
                .sum::<f64>();
            solution[row] = (intermediate[row] - later) / lower[row][row];
        }
        if solution.iter().any(|value| !value.is_finite()) {
            return Err(FemValidationError {
                field: "solution".to_string(),
                message: "contains non-finite value".to_string(),
            });
        }
        let residual_l2 = dense
            .iter()
            .zip(rhs)
            .map(|(row, expected)| {
                let residual = row
                    .iter()
                    .zip(&solution)
                    .map(|(coefficient, value)| coefficient * value)
                    .sum::<f64>()
                    - expected;
                residual * residual
            })
            .sum::<f64>()
            .sqrt();
        let rhs_l2 = rhs.iter().map(|value| value * value).sum::<f64>().sqrt();
        let relative_residual = residual_l2 / rhs_l2.max(1.0);
        if !relative_residual.is_finite() || relative_residual > relative_tolerance {
            return Err(FemValidationError {
                field: "solution.relativeResidual".to_string(),
                message: format!(
                    "relative residual {relative_residual} exceeds tolerance {relative_tolerance}"
                ),
            });
        }
        Ok(FemLinearSolveResult {
            solution,
            residual_l2,
            relative_residual,
            solver_identity: FemLinearSolverIdentity {
                backend: "ecky-reference".to_string(),
                backend_version: env!("CARGO_PKG_VERSION").to_string(),
                factorization: "dense-llt".to_string(),
                ordering: "natural".to_string(),
                scalar_type: "f64".to_string(),
                parallelism: "sequential".to_string(),
                relative_tolerance,
            },
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemDirichletConstraint {
    pub dof_index: usize,
    pub value_mm: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemDirichletReduction {
    pub original_dimension: usize,
    pub original_matrix: FemSparseMatrix,
    pub original_rhs: Vec<f64>,
    pub free_dof_indices: Vec<usize>,
    pub constrained_dofs: Vec<FemDirichletConstraint>,
    pub matrix: FemSparseMatrix,
    pub rhs: Vec<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemDofReaction {
    pub dof_index: usize,
    pub reaction_n: f64,
}

impl FemDirichletReduction {
    pub fn recover_full_solution(
        &self,
        reduced_solution: &[f64],
    ) -> Result<Vec<f64>, FemValidationError> {
        if reduced_solution.len() != self.free_dof_indices.len() {
            return Err(FemValidationError {
                field: "reducedSolution".to_string(),
                message: "length differs from free DOF count".to_string(),
            });
        }
        let mut solution = vec![0.0; self.original_dimension];
        for (value, dof_index) in reduced_solution.iter().zip(&self.free_dof_indices) {
            finite("reducedSolution.value", *value)?;
            solution[*dof_index] = *value;
        }
        for constraint in &self.constrained_dofs {
            solution[constraint.dof_index] = constraint.value_mm;
        }
        Ok(solution)
    }

    pub fn recover_support_reactions(
        &self,
        reduced_solution: &[f64],
    ) -> Result<Vec<FemDofReaction>, FemValidationError> {
        let solution = self.recover_full_solution(reduced_solution)?;
        let entries = self.original_matrix.validated_entries()?;
        Ok(self
            .constrained_dofs
            .iter()
            .map(|constraint| {
                let internal = (0..self.original_dimension)
                    .map(|col| {
                        entries
                            .get(&(constraint.dof_index, col))
                            .copied()
                            .unwrap_or(0.0)
                            * solution[col]
                    })
                    .sum::<f64>();
                FemDofReaction {
                    dof_index: constraint.dof_index,
                    reaction_n: internal - self.original_rhs[constraint.dof_index],
                }
            })
            .collect())
    }
}

impl FemSparseMatrix {
    pub fn from_dense(dense: Vec<Vec<f64>>) -> Result<Self, FemValidationError> {
        let dimension = dense.len();
        if dimension == 0 || dense.iter().any(|row| row.len() != dimension) {
            return Err(FemValidationError {
                field: "matrix".to_string(),
                message: "must be non-empty and square".to_string(),
            });
        }
        let mut entries = Vec::new();
        for (row, values) in dense.into_iter().enumerate() {
            for (col, value) in values.into_iter().enumerate() {
                finite("matrix.value", value)?;
                if value != 0.0 {
                    entries.push(FemSparseEntry { row, col, value });
                }
            }
        }
        Ok(Self { dimension, entries })
    }

    pub fn to_dense(&self) -> Vec<Vec<f64>> {
        let mut dense = vec![vec![0.0; self.dimension]; self.dimension];
        for entry in &self.entries {
            if entry.row < self.dimension && entry.col < self.dimension {
                dense[entry.row][entry.col] += entry.value;
            }
        }
        dense
    }

    fn validated_entries(&self) -> Result<BTreeMap<(usize, usize), f64>, FemValidationError> {
        if self.dimension == 0 {
            return Err(FemValidationError {
                field: "matrix.dimension".to_string(),
                message: "must be positive".to_string(),
            });
        }
        let mut entries = BTreeMap::new();
        for entry in &self.entries {
            if entry.row >= self.dimension || entry.col >= self.dimension {
                return Err(FemValidationError {
                    field: "matrix.entries".to_string(),
                    message: "contains out-of-range index".to_string(),
                });
            }
            finite("matrix.entries.value", entry.value)?;
            if entries
                .insert((entry.row, entry.col), entry.value)
                .is_some()
            {
                return Err(FemValidationError {
                    field: "matrix.entries".to_string(),
                    message: "contains duplicate coordinate".to_string(),
                });
            }
        }
        Ok(entries)
    }

    pub fn eliminate_dirichlet(
        &self,
        rhs: &[f64],
        constraints: &[FemDirichletConstraint],
    ) -> Result<FemDirichletReduction, FemValidationError> {
        let entries = self.validated_entries()?;
        if rhs.len() != self.dimension {
            return Err(FemValidationError {
                field: "rhs".to_string(),
                message: "length differs from matrix dimension".to_string(),
            });
        }
        for value in rhs {
            finite("rhs.value", *value)?;
        }
        let mut constrained = BTreeMap::new();
        for constraint in constraints {
            if constraint.dof_index >= self.dimension {
                return Err(FemValidationError {
                    field: "constraints.dofIndex".to_string(),
                    message: "is out of range".to_string(),
                });
            }
            finite("constraints.valueMm", constraint.value_mm)?;
            if constrained
                .insert(constraint.dof_index, constraint.value_mm)
                .is_some()
            {
                return Err(FemValidationError {
                    field: "constraints.dofIndex".to_string(),
                    message: "contains duplicate constrained DOF".to_string(),
                });
            }
        }
        let free_dof_indices = (0..self.dimension)
            .filter(|index| !constrained.contains_key(index))
            .collect::<Vec<_>>();
        if free_dof_indices.is_empty() {
            return Err(FemValidationError {
                field: "constraints".to_string(),
                message: "constrain every DOF; reduced system is empty".to_string(),
            });
        }
        let reduced_index = free_dof_indices
            .iter()
            .enumerate()
            .map(|(reduced, original)| (*original, reduced))
            .collect::<BTreeMap<_, _>>();
        let reduced_rhs = free_dof_indices
            .iter()
            .map(|row| {
                rhs[*row]
                    - constrained
                        .iter()
                        .map(|(col, value)| {
                            entries.get(&(*row, *col)).copied().unwrap_or(0.0) * value
                        })
                        .sum::<f64>()
            })
            .collect::<Vec<_>>();
        let reduced_entries = entries
            .into_iter()
            .filter_map(|((row, col), value)| {
                Some(FemSparseEntry {
                    row: *reduced_index.get(&row)?,
                    col: *reduced_index.get(&col)?,
                    value,
                })
            })
            .collect();
        Ok(FemDirichletReduction {
            original_dimension: self.dimension,
            original_matrix: self.clone(),
            original_rhs: rhs.to_vec(),
            free_dof_indices,
            constrained_dofs: constrained
                .into_iter()
                .map(|(dof_index, value_mm)| FemDirichletConstraint {
                    dof_index,
                    value_mm,
                })
                .collect(),
            matrix: FemSparseMatrix {
                dimension: reduced_index.len(),
                entries: reduced_entries,
            },
            rhs: reduced_rhs,
        })
    }
}

impl Tet4Element {
    pub const fn new(nodes: [FemPoint3; 4]) -> Self {
        Self { nodes }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ElementAssembler;

pub type Tet4ReferenceGradients = [[f64; 3]; 4];
pub type Tet4WorldGradients = [[f64; 3]; 4];
pub type Tet4VoigtVector = [f64; 6];
pub type Tet4BMatrix = [[f64; 12]; 6];
pub type Tet4ElasticityMatrix = [[f64; 6]; 6];
pub type Tet4StiffnessMatrix = [[f64; 12]; 12];

fn validate_point3(field: &str, point: &FemPoint3) -> Result<(), FemValidationError> {
    finite(&format!("{field}.xMm"), point.x_mm)?;
    finite(&format!("{field}.yMm"), point.y_mm)?;
    finite(&format!("{field}.zMm"), point.z_mm)?;
    Ok(())
}

fn validate_tet4_element(element: &Tet4Element) -> Result<(), FemValidationError> {
    for (index, node) in element.nodes.iter().enumerate() {
        validate_point3(&format!("nodes[{index}]"), node)?;
    }
    Ok(())
}

fn subtract_points(left: FemPoint3, right: FemPoint3) -> [f64; 3] {
    [
        left.x_mm - right.x_mm,
        left.y_mm - right.y_mm,
        left.z_mm - right.z_mm,
    ]
}

fn dot(left: [f64; 3], right: [f64; 3]) -> f64 {
    left[0] * right[0] + left[1] * right[1] + left[2] * right[2]
}

fn cross(left: [f64; 3], right: [f64; 3]) -> [f64; 3] {
    [
        left[1] * right[2] - left[2] * right[1],
        left[2] * right[0] - left[0] * right[2],
        left[0] * right[1] - left[1] * right[0],
    ]
}

fn vector_norm(vector: [f64; 3]) -> f64 {
    dot(vector, vector).sqrt()
}

fn surface_triangle_area_normal(
    triangle: &FemSurfaceTriangle,
) -> Result<(f64, [f64; 3]), FemValidationError> {
    if triangle.schema_version != FEM_SCHEMA_VERSION {
        return Err(schema_version_error());
    }
    for (index, node) in triangle.nodes.iter().enumerate() {
        validate_point3(&format!("surfaceTriangle.nodes[{index}]"), node)?;
    }
    let first = subtract_points(triangle.nodes[1], triangle.nodes[0]);
    let second = subtract_points(triangle.nodes[2], triangle.nodes[0]);
    let raw_normal = cross(first, second);
    let twice_area = vector_norm(raw_normal);
    let edge_scale = vector_norm(first).max(vector_norm(second)).max(1.0);
    if !twice_area.is_finite() || twice_area <= 64.0 * f64::EPSILON * edge_scale.powi(2) {
        return Err(FemValidationError {
            field: "surfaceTriangle.nodes".to_string(),
            message: "surface triangle has zero or near-zero area".to_string(),
        });
    }
    Ok((
        0.5 * twice_area,
        raw_normal.map(|component| component / twice_area),
    ))
}

fn equal_triangle_nodal_force(resultant: [f64; 3]) -> [FemForceVector; 3] {
    let nodal = FemForceVector {
        x_n: resultant[0] / 3.0,
        y_n: resultant[1] / 3.0,
        z_n: resultant[2] / 3.0,
    };
    [nodal; 3]
}

type Tet4Geometry = ([f64; 3], [f64; 3], [f64; 3], f64, f64);

fn tet4_geometry(element: &Tet4Element) -> Result<Tet4Geometry, FemValidationError> {
    validate_tet4_element(element)?;

    let p1 = element.nodes[0];
    let p2 = element.nodes[1];
    let p3 = element.nodes[2];
    let p4 = element.nodes[3];

    let edge_12 = subtract_points(p2, p1);
    let edge_13 = subtract_points(p3, p1);
    let edge_14 = subtract_points(p4, p1);

    let signed_six_volume = dot(edge_12, cross(edge_13, edge_14));
    if !signed_six_volume.is_finite() {
        return Err(FemValidationError {
            field: "nodes".to_string(),
            message: "tet4 geometry produced a non-finite signed volume".to_string(),
        });
    }

    let edge_scale = vector_norm(edge_12)
        .max(vector_norm(edge_13))
        .max(vector_norm(edge_14))
        .max(1.0);

    Ok((edge_12, edge_13, edge_14, signed_six_volume, edge_scale))
}

fn tet4_world_gradients_from_edges(
    edge_12: [f64; 3],
    edge_13: [f64; 3],
    edge_14: [f64; 3],
    signed_six_volume: f64,
) -> Tet4WorldGradients {
    let inverse_six_volume = 1.0 / signed_six_volume;

    let gradient_2 = cross(edge_13, edge_14).map(|value| value * inverse_six_volume);
    let gradient_3 = cross(edge_14, edge_12).map(|value| value * inverse_six_volume);
    let gradient_4 = cross(edge_12, edge_13).map(|value| value * inverse_six_volume);
    let gradient_1 = [
        -(gradient_2[0] + gradient_3[0] + gradient_4[0]),
        -(gradient_2[1] + gradient_3[1] + gradient_4[1]),
        -(gradient_2[2] + gradient_3[2] + gradient_4[2]),
    ];

    [gradient_1, gradient_2, gradient_3, gradient_4]
}

fn b_matrix_from_gradients(gradients: &Tet4WorldGradients) -> Tet4BMatrix {
    let mut matrix = [[0.0; 12]; 6];

    for (node_index, gradient) in gradients.iter().enumerate() {
        let base = node_index * 3;
        let gx = gradient[0];
        let gy = gradient[1];
        let gz = gradient[2];

        matrix[0][base] = gx;
        matrix[1][base + 1] = gy;
        matrix[2][base + 2] = gz;

        matrix[3][base + 1] = gz;
        matrix[3][base + 2] = gy;

        matrix[4][base] = gz;
        matrix[4][base + 2] = gx;

        matrix[5][base] = gy;
        matrix[5][base + 1] = gx;
    }

    matrix
}

fn flatten_displacements(displacements: &[FemPoint3; 4]) -> [f64; 12] {
    [
        displacements[0].x_mm,
        displacements[0].y_mm,
        displacements[0].z_mm,
        displacements[1].x_mm,
        displacements[1].y_mm,
        displacements[1].z_mm,
        displacements[2].x_mm,
        displacements[2].y_mm,
        displacements[2].z_mm,
        displacements[3].x_mm,
        displacements[3].y_mm,
        displacements[3].z_mm,
    ]
}

fn multiply_b_matrix(b_matrix: &Tet4BMatrix, displacements: &[f64; 12]) -> Tet4VoigtVector {
    let mut strain = [0.0; 6];

    for row in 0..6 {
        for col in 0..12 {
            strain[row] += b_matrix[row][col] * displacements[col];
        }
    }

    strain
}

fn multiply_elasticity_matrix(
    elasticity: &Tet4ElasticityMatrix,
    strain: &Tet4VoigtVector,
) -> Tet4VoigtVector {
    let mut stress = [0.0; 6];

    for row in 0..6 {
        for col in 0..6 {
            stress[row] += elasticity[row][col] * strain[col];
        }
    }

    stress
}

fn stiffness_from_b_d_v(
    b_matrix: &Tet4BMatrix,
    elasticity: &Tet4ElasticityMatrix,
    volume: f64,
) -> Tet4StiffnessMatrix {
    let mut db = [[0.0; 12]; 6];
    for row in 0..6 {
        for col in 0..12 {
            for inner in 0..6 {
                db[row][col] += elasticity[row][inner] * b_matrix[inner][col];
            }
        }
    }

    let mut stiffness = [[0.0; 12]; 12];
    for row in 0..12 {
        for col in row..12 {
            let mut value = 0.0;
            for voigt in 0..6 {
                value += b_matrix[voigt][row] * db[voigt][col];
            }
            value *= volume;
            stiffness[row][col] = value;
            stiffness[col][row] = value;
        }
    }

    stiffness
}

impl ElementAssembler {
    pub fn assemble_global_stiffness(
        &self,
        mesh: &FemIndexedTet4Mesh,
        material: &FemMaterial,
    ) -> Result<FemSparseMatrix, FemValidationError> {
        self.assemble_global_stiffness_with_observer(mesh, material, |_| Ok(()))
    }

    pub fn assemble_global_stiffness_with_observer<F>(
        &self,
        mesh: &FemIndexedTet4Mesh,
        material: &FemMaterial,
        mut observe_chunk: F,
    ) -> Result<FemSparseMatrix, FemValidationError>
    where
        F: FnMut(usize) -> Result<(), FemValidationError>,
    {
        if mesh.schema_version != FEM_SCHEMA_VERSION {
            return Err(schema_version_error());
        }
        if mesh.nodes.is_empty() || mesh.cells.is_empty() {
            return Err(FemValidationError {
                field: "mesh".to_string(),
                message: "nodes and tet4 cells must not be empty".to_string(),
            });
        }
        for (index, node) in mesh.nodes.iter().enumerate() {
            validate_point3(&format!("mesh.nodes[{index}]"), node)?;
        }
        let dimension = mesh
            .nodes
            .len()
            .checked_mul(3)
            .ok_or_else(|| FemValidationError {
                field: "mesh.nodes".to_string(),
                message: "DOF count overflowed".to_string(),
            })?;
        let mut entries = BTreeMap::<(usize, usize), f64>::new();
        for (cell_index, cell) in mesh.cells.iter().enumerate() {
            if cell_index % 256 == 0 {
                observe_chunk(cell_index)?;
            }
            let mut unique = *cell;
            unique.sort_unstable();
            if unique.windows(2).any(|pair| pair[0] == pair[1]) {
                return Err(FemValidationError {
                    field: format!("mesh.cells[{cell_index}]"),
                    message: "contains repeated node index".to_string(),
                });
            }
            let node_indices = cell.map(|index| usize::try_from(index).expect("u32 fits usize"));
            if node_indices.iter().any(|index| *index >= mesh.nodes.len()) {
                return Err(FemValidationError {
                    field: format!("mesh.cells[{cell_index}]"),
                    message: "contains out-of-range node index".to_string(),
                });
            }
            let element = Tet4Element::new(node_indices.map(|index| mesh.nodes[index]));
            if self.orientation(&element)? != Tet4Orientation::Positive {
                return Err(FemValidationError {
                    field: format!("mesh.cells[{cell_index}]"),
                    message: "must have positive tet4 orientation".to_string(),
                });
            }
            let local = self.stiffness_matrix(&element, material)?;
            for local_row in 0..12 {
                let global_row = node_indices[local_row / 3] * 3 + local_row % 3;
                for local_col in 0..12 {
                    let value = local[local_row][local_col];
                    if value == 0.0 {
                        continue;
                    }
                    let global_col = node_indices[local_col / 3] * 3 + local_col % 3;
                    *entries.entry((global_row, global_col)).or_default() += value;
                }
            }
        }
        let entries = entries
            .into_iter()
            .filter_map(|((row, col), value)| {
                (value != 0.0).then_some(FemSparseEntry { row, col, value })
            })
            .collect();
        Ok(FemSparseMatrix { dimension, entries })
    }

    pub fn integrate_triangle_traction(
        &self,
        triangle: &FemSurfaceTriangle,
        traction_n_per_mm2: [f64; 3],
    ) -> Result<[FemForceVector; 3], FemValidationError> {
        for (index, value) in traction_n_per_mm2.iter().enumerate() {
            finite(&format!("tractionNPerMm2[{index}]"), *value)?;
        }
        let (area_mm2, _) = surface_triangle_area_normal(triangle)?;
        Ok(equal_triangle_nodal_force(
            traction_n_per_mm2.map(|component| component * area_mm2),
        ))
    }

    pub fn integrate_triangle_pressure(
        &self,
        triangle: &FemSurfaceTriangle,
        inward_pressure_mpa: f64,
    ) -> Result<[FemForceVector; 3], FemValidationError> {
        positive_finite("inwardPressureMpa", inward_pressure_mpa)?;
        let (area_mm2, outward_normal) = surface_triangle_area_normal(triangle)?;
        Ok(equal_triangle_nodal_force(outward_normal.map(
            |component| -component * inward_pressure_mpa * area_mm2,
        )))
    }

    pub fn distribute_total_surface_force(
        &self,
        triangles: &[FemSurfaceTriangle],
        total_force: FemForceVector,
    ) -> Result<Vec<[FemForceVector; 3]>, FemValidationError> {
        if triangles.is_empty() {
            return Err(FemValidationError {
                field: "surfaceTriangles".to_string(),
                message: "must not be empty".to_string(),
            });
        }
        total_force.validate()?;
        let areas = triangles
            .iter()
            .map(surface_triangle_area_normal)
            .map(|result| result.map(|(area, _)| area))
            .collect::<Result<Vec<_>, _>>()?;
        let total_area = areas.iter().sum::<f64>();
        if !total_area.is_finite() || total_area <= 0.0 {
            return Err(FemValidationError {
                field: "surfaceTriangles".to_string(),
                message: "total selected surface area must be positive and finite".to_string(),
            });
        }
        Ok(areas
            .into_iter()
            .map(|area| {
                let share = area / total_area;
                equal_triangle_nodal_force([
                    total_force.x_n * share,
                    total_force.y_n * share,
                    total_force.z_n * share,
                ])
            })
            .collect())
    }

    pub fn signed_volume_mm3(&self, element: &Tet4Element) -> Result<f64, FemValidationError> {
        let (_, _, _, signed_six_volume, _) = tet4_geometry(element)?;
        Ok(signed_six_volume / 6.0)
    }

    pub fn orientation(
        &self,
        element: &Tet4Element,
    ) -> Result<Tet4Orientation, FemValidationError> {
        let volume = self.signed_volume_mm3(element)?;
        let tolerance = 64.0 * f64::EPSILON * volume.abs().max(1.0);

        Ok(if volume > tolerance {
            Tet4Orientation::Positive
        } else if volume < -tolerance {
            Tet4Orientation::Negative
        } else {
            Tet4Orientation::Degenerate
        })
    }

    pub fn reference_shape_gradients(&self) -> Tet4ReferenceGradients {
        [
            [-1.0, -1.0, -1.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
        ]
    }

    pub fn reference_shape_functions(
        &self,
        local: [f64; 3],
    ) -> Result<[f64; 4], FemValidationError> {
        finite("localCoordinates.x", local[0])?;
        finite("localCoordinates.y", local[1])?;
        finite("localCoordinates.z", local[2])?;

        Ok([
            1.0 - local[0] - local[1] - local[2],
            local[0],
            local[1],
            local[2],
        ])
    }

    pub fn world_shape_gradients(
        &self,
        element: &Tet4Element,
    ) -> Result<Tet4WorldGradients, FemValidationError> {
        let (edge_12, edge_13, edge_14, signed_six_volume, edge_scale) = tet4_geometry(element)?;
        let tolerance = 64.0 * f64::EPSILON * edge_scale.powi(3);
        if signed_six_volume.abs() <= tolerance {
            return Err(FemValidationError {
                field: "nodes".to_string(),
                message: "tet4 element has zero or near-zero signed volume".to_string(),
            });
        }

        Ok(tet4_world_gradients_from_edges(
            edge_12,
            edge_13,
            edge_14,
            signed_six_volume,
        ))
    }

    pub fn b_matrix(&self, element: &Tet4Element) -> Result<Tet4BMatrix, FemValidationError> {
        let gradients = self.world_shape_gradients(element)?;
        Ok(b_matrix_from_gradients(&gradients))
    }

    /// Isotropic 3D linear elasticity in Voigt order `[xx, yy, zz, yz, xz, xy]`.
    ///
    /// The shear slots use engineering strain (`gamma = 2 * epsilon`) so the
    /// shear diagonal entries are the shear modulus `mu`.
    pub fn constitutive_matrix(
        &self,
        material: &FemMaterial,
    ) -> Result<Tet4ElasticityMatrix, FemValidationError> {
        material.validate()?;

        let young = material.young_modulus_mpa;
        let poisson = material.poisson_ratio;
        let lambda = young * poisson / ((1.0 + poisson) * (1.0 - 2.0 * poisson));
        let mu = young / (2.0 * (1.0 + poisson));

        let mut matrix = [[0.0; 6]; 6];
        for row in matrix.iter_mut().take(3) {
            for entry in row.iter_mut().take(3) {
                *entry = lambda;
            }
        }
        for (index, row) in matrix.iter_mut().enumerate().take(3) {
            row[index] = lambda + 2.0 * mu;
        }
        for (index, row) in matrix.iter_mut().enumerate().skip(3) {
            row[index] = mu;
        }

        Ok(matrix)
    }

    pub fn strain_from_displacements(
        &self,
        element: &Tet4Element,
        nodal_displacements: &[FemPoint3; 4],
    ) -> Result<Tet4VoigtVector, FemValidationError> {
        let b_matrix = self.b_matrix(element)?;
        for (index, displacement) in nodal_displacements.iter().enumerate() {
            validate_point3(&format!("displacements[{index}]"), displacement)?;
        }

        let displacement_vector = flatten_displacements(nodal_displacements);
        Ok(multiply_b_matrix(&b_matrix, &displacement_vector))
    }

    pub fn stress_from_strain(
        &self,
        material: &FemMaterial,
        strain: Tet4VoigtVector,
    ) -> Result<Tet4VoigtVector, FemValidationError> {
        for (index, value) in strain.iter().enumerate() {
            finite(&format!("strain[{index}]"), *value)?;
        }

        let elasticity = self.constitutive_matrix(material)?;
        Ok(multiply_elasticity_matrix(&elasticity, &strain))
    }

    pub fn stress_from_displacements(
        &self,
        element: &Tet4Element,
        nodal_displacements: &[FemPoint3; 4],
        material: &FemMaterial,
    ) -> Result<Tet4VoigtVector, FemValidationError> {
        let strain = self.strain_from_displacements(element, nodal_displacements)?;
        self.stress_from_strain(material, strain)
    }

    pub fn stiffness_matrix(
        &self,
        element: &Tet4Element,
        material: &FemMaterial,
    ) -> Result<Tet4StiffnessMatrix, FemValidationError> {
        let b_matrix = self.b_matrix(element)?;
        let elasticity = self.constitutive_matrix(material)?;
        let volume = self.signed_volume_mm3(element)?.abs();

        Ok(stiffness_from_b_d_v(&b_matrix, &elasticity, volume))
    }
}
