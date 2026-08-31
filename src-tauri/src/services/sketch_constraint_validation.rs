use serde::{Deserialize, Serialize};
use specta::Type;

use crate::contracts::{SketchConstraintKind, SketchDocument, SketchPrimitive};

const DIMENSION_TOLERANCE: f64 = 1e-6;

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SketchConstraintValidationResult {
    pub passed: bool,
    #[serde(default)]
    pub evidence: Vec<String>,
    #[serde(default)]
    pub issues: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SketchConstraintValueRepairResult {
    pub document: SketchDocument,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SketchConstraintGeometryRepairEvidence {
    pub primitive_id: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SketchConstraintGeometryRepairResult {
    pub document: SketchDocument,
    pub evidence: Vec<SketchConstraintGeometryRepairEvidence>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum SketchConstraintEvaluationMode {
    Validate,
    RepairConstraintValues,
    AutoRepairGeometry,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SketchConstraintEvaluationRequest {
    pub document: SketchDocument,
    pub mode: SketchConstraintEvaluationMode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_delta: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum SketchConstraintEvaluationResponse {
    Validation {
        passed: bool,
        evidence: Vec<String>,
        issues: Vec<String>,
    },
    ConstraintValuesRepaired {
        document: SketchDocument,
        evidence: Vec<String>,
    },
    GeometryRepaired {
        document: SketchDocument,
        evidence: Vec<SketchConstraintGeometryRepairEvidence>,
    },
    Error {
        error: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Dimension {
    Width,
    Height,
}

impl Dimension {
    fn label(self) -> &'static str {
        match self {
            Self::Width => "width",
            Self::Height => "height",
        }
    }

    fn axis(self) -> usize {
        match self {
            Self::Width => 0,
            Self::Height => 1,
        }
    }
}

pub fn evaluate_constraints(
    request: SketchConstraintEvaluationRequest,
) -> SketchConstraintEvaluationResponse {
    match request.mode {
        SketchConstraintEvaluationMode::Validate => {
            let result = validate_constraints(&request.document);
            SketchConstraintEvaluationResponse::Validation {
                passed: result.passed,
                evidence: result.evidence,
                issues: result.issues,
            }
        }
        SketchConstraintEvaluationMode::RepairConstraintValues => {
            match repair_dimension_constraint_values(&request.document) {
                Ok(result) => SketchConstraintEvaluationResponse::ConstraintValuesRepaired {
                    document: result.document,
                    evidence: result.evidence,
                },
                Err(error) => SketchConstraintEvaluationResponse::Error { error },
            }
        }
        SketchConstraintEvaluationMode::AutoRepairGeometry => {
            let result =
                auto_repair_dimension_geometry(&request.document, request.max_delta.unwrap_or(1.0));
            SketchConstraintEvaluationResponse::GeometryRepaired {
                document: result.document,
                evidence: result.evidence,
            }
        }
    }
}

pub fn validate_constraints(document: &SketchDocument) -> SketchConstraintValidationResult {
    let mut issues = Vec::new();
    let mut evidence = Vec::new();

    for sketch in &document.sketches {
        for constraint in &sketch.constraints {
            if constraint.kind != SketchConstraintKind::Dimension {
                continue;
            }

            let Some(expected_value) = constraint.value.filter(|value| value.is_finite()) else {
                issues.push(format!(
                    "sketch '{}' dimension constraint '{}' has missing or non-finite value.",
                    sketch.sketch_id, constraint.constraint_id
                ));
                continue;
            };
            let Some(dimension) = constraint_dimension(&constraint.constraint_id) else {
                issues.push(format!(
                    "sketch '{}' dimension constraint '{}' is neither width nor height.",
                    sketch.sketch_id, constraint.constraint_id
                ));
                continue;
            };

            for target_id in &constraint.target_ids {
                let Some(primitive) = sketch
                    .primitives
                    .iter()
                    .find(|primitive| primitive.primitive_id == *target_id)
                else {
                    issues.push(format!(
                        "sketch '{}' dimension constraint '{}' targets missing primitive '{}'.",
                        sketch.sketch_id, constraint.constraint_id, target_id
                    ));
                    continue;
                };
                let Some(measured) = measure_primitive_dimension(primitive, dimension) else {
                    issues.push(format!(
                        "sketch '{}' primitive '{}' has invalid or no points.",
                        sketch.sketch_id, primitive.primitive_id
                    ));
                    continue;
                };

                if (expected_value - measured).abs() > DIMENSION_TOLERANCE {
                    issues.push(format!(
                        "sketch '{}' primitive '{}' {} dimension expected {} but measured {}.",
                        sketch.sketch_id,
                        primitive.primitive_id,
                        dimension.label(),
                        format_mm(expected_value),
                        format_mm(measured)
                    ));
                    continue;
                }

                evidence.push(format!(
                    "sketch '{}' primitive '{}' {} dimension matched {}.",
                    sketch.sketch_id,
                    primitive.primitive_id,
                    dimension.label(),
                    format_mm(measured)
                ));
            }
        }
    }

    if !issues.is_empty() {
        return SketchConstraintValidationResult {
            passed: false,
            evidence: Vec::new(),
            issues,
        };
    }
    if evidence.is_empty() {
        evidence.push("No dimension constraints.".to_string());
    }
    SketchConstraintValidationResult {
        passed: true,
        evidence,
        issues: Vec::new(),
    }
}

pub fn repair_dimension_constraint_values(
    document: &SketchDocument,
) -> Result<SketchConstraintValueRepairResult, String> {
    let mut repaired_document = document.clone();
    let mut issues = Vec::new();
    let mut evidence = Vec::new();

    for sketch in &mut repaired_document.sketches {
        let primitives = &sketch.primitives;
        for constraint in &mut sketch.constraints {
            if constraint.kind != SketchConstraintKind::Dimension {
                continue;
            }

            let Some(current_value) = constraint.value.filter(|value| value.is_finite()) else {
                issues.push(format!(
                    "sketch '{}' dimension constraint '{}' has missing or non-finite value.",
                    sketch.sketch_id, constraint.constraint_id
                ));
                continue;
            };
            let Some(dimension) = constraint_dimension(&constraint.constraint_id) else {
                issues.push(format!(
                    "sketch '{}' dimension constraint '{}' is neither width nor height.",
                    sketch.sketch_id, constraint.constraint_id
                ));
                continue;
            };
            if constraint.target_ids.is_empty() {
                issues.push(format!(
                    "sketch '{}' dimension constraint '{}' has no targets.",
                    sketch.sketch_id, constraint.constraint_id
                ));
                continue;
            }

            for target_id in &constraint.target_ids {
                let Some(primitive) = primitives
                    .iter()
                    .find(|primitive| primitive.primitive_id == *target_id)
                else {
                    issues.push(format!(
                        "sketch '{}' dimension constraint '{}' targets missing primitive '{}'.",
                        sketch.sketch_id, constraint.constraint_id, target_id
                    ));
                    continue;
                };
                let Some(measured) = measure_primitive_dimension(primitive, dimension) else {
                    issues.push(format!(
                        "sketch '{}' primitive '{}' has invalid or no points.",
                        sketch.sketch_id, primitive.primitive_id
                    ));
                    continue;
                };
                if (current_value - measured).abs() <= DIMENSION_TOLERANCE {
                    continue;
                }

                let repaired_value = round_to(measured, 4);
                constraint.value = Some(repaired_value);
                evidence.push(format!(
                    "sketch '{}' primitive '{}' {} dimension repaired {} -> {}.",
                    sketch.sketch_id,
                    primitive.primitive_id,
                    dimension.label(),
                    format_mm(current_value),
                    format_mm(repaired_value)
                ));
            }
        }
    }

    if !issues.is_empty() {
        return Err(issues.join(" "));
    }
    if evidence.is_empty() {
        return Err("No repairable dimension constraint mismatch.".to_string());
    }
    Ok(SketchConstraintValueRepairResult {
        document: repaired_document,
        evidence,
    })
}

pub fn auto_repair_dimension_geometry(
    document: &SketchDocument,
    max_delta: f64,
) -> SketchConstraintGeometryRepairResult {
    let mut repaired_document = document.clone();
    let mut evidence = Vec::new();

    for sketch in &mut repaired_document.sketches {
        for constraint_index in 0..sketch.constraints.len() {
            let constraint = &sketch.constraints[constraint_index];
            if constraint.kind != SketchConstraintKind::Dimension {
                continue;
            }
            let Some(expected_value) = constraint.value.filter(|value| value.is_finite()) else {
                continue;
            };
            let Some(dimension) = constraint_dimension(&constraint.constraint_id) else {
                continue;
            };
            let target_ids = constraint.target_ids.clone();

            for target_id in target_ids {
                let Some(primitive) = sketch
                    .primitives
                    .iter_mut()
                    .find(|primitive| primitive.primitive_id == target_id)
                else {
                    continue;
                };
                let Some(measured) = measure_primitive_dimension(primitive, dimension) else {
                    continue;
                };
                let delta = (expected_value - measured).abs();
                if delta <= DIMENSION_TOLERANCE || delta > max_delta {
                    continue;
                }
                if !resize_primitive_dimension(primitive, dimension, expected_value) {
                    continue;
                }

                evidence.push(SketchConstraintGeometryRepairEvidence {
                    primitive_id: primitive.primitive_id.clone(),
                    detail: format!(
                        "{} dimension {} -> {}",
                        dimension.label(),
                        format_mm(measured),
                        format_mm(expected_value)
                    ),
                });
            }
        }
    }

    SketchConstraintGeometryRepairResult {
        document: repaired_document,
        evidence,
    }
}

fn constraint_dimension(constraint_id: &str) -> Option<Dimension> {
    if constraint_id.contains("width") {
        Some(Dimension::Width)
    } else if constraint_id.contains("height") {
        Some(Dimension::Height)
    } else {
        None
    }
}

fn measure_primitive_dimension(primitive: &SketchPrimitive, dimension: Dimension) -> Option<f64> {
    let points = primitive_points(primitive)?;
    let axis = dimension.axis();
    let mut min = f64::INFINITY;
    let mut max = f64::NEG_INFINITY;
    for point in points {
        min = min.min(point[axis]);
        max = max.max(point[axis]);
    }
    Some(max - min)
}

fn primitive_points(primitive: &SketchPrimitive) -> Option<&[[f64; 2]]> {
    let points = primitive.points.as_slice();
    let logical_points = if has_closed_duplicate(points) {
        &points[..points.len() - 1]
    } else {
        points
    };
    if logical_points.is_empty()
        || logical_points
            .iter()
            .flatten()
            .any(|coordinate| !coordinate.is_finite())
    {
        return None;
    }
    Some(logical_points)
}

fn resize_primitive_dimension(
    primitive: &mut SketchPrimitive,
    dimension: Dimension,
    target_value: f64,
) -> bool {
    let Some(points) = primitive_points(primitive) else {
        return false;
    };
    let axis = dimension.axis();
    let min = points
        .iter()
        .map(|point| point[axis])
        .fold(f64::INFINITY, f64::min);
    let max = points
        .iter()
        .map(|point| point[axis])
        .fold(f64::NEG_INFINITY, f64::max);
    let current_value = max - min;
    if current_value <= DIMENSION_TOLERANCE {
        return false;
    }

    let center = (min + max) / 2.0;
    let scale = target_value / current_value;
    for point in &mut primitive.points {
        point[axis] = round_to(center + (point[axis] - center) * scale, 4);
    }
    true
}

fn has_closed_duplicate(points: &[[f64; 2]]) -> bool {
    points.len() >= 2 && points.first() == points.last()
}

fn format_mm(value: f64) -> String {
    let rounded = round_to(value, 6);
    if rounded == 0.0 {
        return "0mm".to_string();
    }
    let mut formatted = format!("{rounded:.6}");
    while formatted.contains('.') && formatted.ends_with('0') {
        formatted.pop();
    }
    if formatted.ends_with('.') {
        formatted.pop();
    }
    format!("{formatted}mm")
}

fn round_to(value: f64, digits: i32) -> f64 {
    let factor = 10_f64.powi(digits);
    (value * factor).round() / factor
}
