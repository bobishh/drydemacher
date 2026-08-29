//! Backend-independent local component frame and rigid mate math.
//!
//! Host/package and inline-source placement share this module. Geometry
//! backends receive only the solved right-handed frame plus an optional local
//! mirror operation.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

const FRAME_EPSILON: f64 = 1.0e-6;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlacementFrame {
    pub origin: [f64; 3],
    pub x_axis: [f64; 3],
    pub y_axis: [f64; 3],
    pub z_axis: [f64; 3],
}

impl PlacementFrame {
    pub fn identity() -> Self {
        Self {
            origin: [0.0, 0.0, 0.0],
            x_axis: [1.0, 0.0, 0.0],
            y_axis: [0.0, 1.0, 0.0],
            z_axis: [0.0, 0.0, 1.0],
        }
    }

    pub fn from_origin_x_z(
        origin: [f64; 3],
        x_axis: [f64; 3],
        z_axis: [f64; 3],
        label: &str,
    ) -> Result<Self, PlacementError> {
        validate_origin(origin, label)?;
        let z_axis = normalize(z_axis, label, "zAxis")?;
        let x_axis = normalize(x_axis, label, "xAxis")?;
        if dot(x_axis, z_axis).abs() > FRAME_EPSILON {
            return Err(PlacementError::new(format!(
                "{label} frame xAxis and zAxis must be orthogonal."
            )));
        }
        let y_axis = normalize(cross(z_axis, x_axis), label, "derived yAxis")?;
        let frame = Self {
            origin,
            x_axis,
            y_axis,
            z_axis,
        };
        frame.validate(label)?;
        Ok(frame)
    }

    pub fn from_axes(
        origin: [f64; 3],
        x_axis: [f64; 3],
        y_axis: [f64; 3],
        z_axis: [f64; 3],
        label: &str,
    ) -> Result<Self, PlacementError> {
        validate_origin(origin, label)?;
        let frame = Self {
            origin,
            x_axis: normalize(x_axis, label, "xAxis")?,
            y_axis: normalize(y_axis, label, "yAxis")?,
            z_axis: normalize(z_axis, label, "zAxis")?,
        };
        frame.validate(label)?;
        Ok(frame)
    }

    pub fn validate(&self, label: &str) -> Result<(), PlacementError> {
        validate_origin(self.origin, label)?;
        let x = normalize(self.x_axis, label, "xAxis")?;
        let y = normalize(self.y_axis, label, "yAxis")?;
        let z = normalize(self.z_axis, label, "zAxis")?;
        if dot(x, y).abs() > FRAME_EPSILON
            || dot(x, z).abs() > FRAME_EPSILON
            || dot(y, z).abs() > FRAME_EPSILON
        {
            return Err(PlacementError::new(format!(
                "{label} frame axes must be orthogonal."
            )));
        }
        if dot(cross(x, y), z) <= FRAME_EPSILON {
            return Err(PlacementError::new(format!(
                "{label} frame axes must form a right-handed basis."
            )));
        }
        Ok(())
    }

    pub fn approx_eq(&self, other: &Self) -> bool {
        norm(sub(self.origin, other.origin)) <= FRAME_EPSILON
            && norm(sub(self.x_axis, other.x_axis)) <= FRAME_EPSILON
            && norm(sub(self.y_axis, other.y_axis)) <= FRAME_EPSILON
            && norm(sub(self.z_axis, other.z_axis)) <= FRAME_EPSILON
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MateNormalMode {
    Aligned,
    Opposed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MirrorAxis {
    X,
    Y,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MateModifiers {
    pub normal_mode: MateNormalMode,
    pub roll_degrees: f64,
    pub offset: [f64; 3],
    pub mirror_axis: Option<MirrorAxis>,
}

impl MateModifiers {
    pub fn opposed() -> Self {
        Self {
            normal_mode: MateNormalMode::Opposed,
            roll_degrees: 0.0,
            offset: [0.0, 0.0, 0.0],
            mirror_axis: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SolvedPlacement {
    pub placement_frame: PlacementFrame,
    pub mirror_axis: Option<MirrorAxis>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthoredPlacementEvidence {
    pub instance_id: String,
    pub component_id: String,
    pub source_instance_id: String,
    pub source_port_id: String,
    pub target_instance_id: String,
    pub target_port_id: String,
    pub placement_frame: PlacementFrame,
    pub normal_mode: MateNormalMode,
    pub roll_degrees: f64,
    pub offset: [f64; 3],
    pub mirror_axis: Option<MirrorAxis>,
    pub resolved_fit_values: BTreeMap<String, f64>,
    pub source_start: Option<u32>,
    pub source_end: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlacementError {
    pub message: String,
}

impl PlacementError {
    fn new(message: String) -> Self {
        Self { message }
    }
}

impl std::fmt::Display for PlacementError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for PlacementError {}

pub fn solve_mate(
    source: &PlacementFrame,
    target: &PlacementFrame,
    modifiers: MateModifiers,
    label: &str,
) -> Result<SolvedPlacement, PlacementError> {
    source.validate(&format!("{label} source port"))?;
    target.validate(&format!("{label} target port"))?;
    if !modifiers.roll_degrees.is_finite() || !all_finite(modifiers.offset) {
        return Err(PlacementError::new(format!(
            "{label} mate modifiers must be finite."
        )));
    }

    let source = match modifiers.mirror_axis {
        Some(axis) => mirror_port_frame(*source, axis, label)?,
        None => *source,
    };
    let source = RigidFrame::try_from(source, &format!("{label} source port"))?;
    let target = RigidFrame::try_from(*target, &format!("{label} target port"))?;
    let modifier = modifier_frame(modifiers);
    let solved = target.compose(&modifier).compose(&source.inverse());
    let placement_frame = solved.into_placement_frame();
    placement_frame.validate(&format!("{label} solved placement"))?;
    Ok(SolvedPlacement {
        placement_frame,
        mirror_axis: modifiers.mirror_axis,
    })
}

/// Solves the unknown placement of `other_port` from a known instance
/// placement and one connected pair of local port frames.
pub fn solve_connected_placement(
    current_placement: &PlacementFrame,
    current_port: &PlacementFrame,
    other_port: &PlacementFrame,
    label: &str,
) -> Result<PlacementFrame, PlacementError> {
    let current_placement = RigidFrame::try_from(*current_placement, "current placement")?;
    let current_port = RigidFrame::try_from(*current_port, &format!("{label} current port"))?;
    let other_port = RigidFrame::try_from(*other_port, &format!("{label} other port"))?;
    let solved = current_placement
        .compose(&current_port)
        .compose(&other_port.inverse())
        .into_placement_frame();
    solved.validate(&format!("{label} solved placement"))?;
    Ok(solved)
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlacementGraphMate {
    pub mate_id: String,
    pub a_instance_id: String,
    pub a_port_id: String,
    pub a_port_frame: PlacementFrame,
    pub b_instance_id: String,
    pub b_port_id: String,
    pub b_port_frame: PlacementFrame,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlacementGraphSolution {
    pub placements: BTreeMap<String, PlacementFrame>,
    pub solved_mate_ids: Vec<String>,
}

/// Solves a rigid mate graph from explicit roots. Declaration order cannot
/// select between conflicting constraints: adjacency is sorted by stable mate
/// id and every redundant path must reproduce the existing frame.
pub fn solve_placement_graph(
    instance_ids: impl IntoIterator<Item = String>,
    roots: BTreeMap<String, PlacementFrame>,
    mates: &[PlacementGraphMate],
    label: &str,
) -> Result<PlacementGraphSolution, PlacementError> {
    let mut instances = instance_ids.into_iter().collect::<BTreeSet<_>>();
    let mut adjacency = BTreeMap::<String, Vec<usize>>::new();
    let mut mate_ids = BTreeSet::new();
    for (index, mate) in mates.iter().enumerate() {
        if !mate_ids.insert(mate.mate_id.clone()) {
            return Err(PlacementError::new(format!(
                "{label} defines mate id `{}` more than once.",
                mate.mate_id
            )));
        }
        instances.insert(mate.a_instance_id.clone());
        instances.insert(mate.b_instance_id.clone());
        adjacency
            .entry(mate.a_instance_id.clone())
            .or_default()
            .push(index);
        adjacency
            .entry(mate.b_instance_id.clone())
            .or_default()
            .push(index);
    }
    for indices in adjacency.values_mut() {
        indices.sort_by(|left, right| mates[*left].mate_id.cmp(&mates[*right].mate_id));
    }

    let mut placements = BTreeMap::new();
    let mut placement_paths = BTreeMap::<String, Vec<String>>::new();
    let mut queue = VecDeque::new();
    for (instance_id, frame) in roots {
        if !instances.contains(&instance_id) {
            return Err(PlacementError::new(format!(
                "{label} root `{instance_id}` does not name an instance."
            )));
        }
        frame.validate(&format!("{label} root `{instance_id}`"))?;
        placements.insert(instance_id.clone(), frame);
        placement_paths.insert(instance_id.clone(), Vec::new());
        queue.push_back(instance_id);
    }

    let mut solved_mates = BTreeSet::new();
    while let Some(current_id) = queue.pop_front() {
        let current_placement = placements[&current_id];
        for index in adjacency.get(&current_id).cloned().unwrap_or_default() {
            let mate = &mates[index];
            let (current_port_id, current_port, other_id, other_port_id, other_port) =
                if mate.a_instance_id == current_id {
                    (
                        mate.a_port_id.as_str(),
                        mate.a_port_frame,
                        mate.b_instance_id.as_str(),
                        mate.b_port_id.as_str(),
                        mate.b_port_frame,
                    )
                } else {
                    (
                        mate.b_port_id.as_str(),
                        mate.b_port_frame,
                        mate.a_instance_id.as_str(),
                        mate.a_port_id.as_str(),
                        mate.a_port_frame,
                    )
                };
            let derived = solve_connected_placement(
                &current_placement,
                &current_port,
                &other_port,
                &format!(
                    "{label} mate `{}` (`{}.{current_port_id}` -> `{other_id}.{other_port_id}`)",
                    mate.mate_id, current_id
                ),
            )?;
            if let Some(existing) = placements.get(other_id) {
                if !existing.approx_eq(&derived) {
                    let existing_path = placement_paths.get(other_id).cloned().unwrap_or_default();
                    let mut resolved_path = placement_paths
                        .get(&current_id)
                        .cloned()
                        .unwrap_or_default();
                    resolved_path.push(mate.mate_id.clone());
                    return Err(PlacementError::new(format!(
                        "{label} mate `{}` conflicts for instance `{other_id}`: existing mate path [{}] produced frame {}; resolved mate path [{}] produced frame {}.",
                        mate.mate_id,
                        existing_path.join(", "),
                        render_frame(existing),
                        resolved_path.join(", "),
                        render_frame(&derived)
                    )));
                }
            } else {
                placements.insert(other_id.to_string(), derived);
                let mut path = placement_paths
                    .get(&current_id)
                    .cloned()
                    .unwrap_or_default();
                path.push(mate.mate_id.clone());
                placement_paths.insert(other_id.to_string(), path);
                queue.push_back(other_id.to_string());
            }
            solved_mates.insert(mate.mate_id.clone());
        }
    }

    let placed_instances = placements.keys().cloned().collect::<BTreeSet<_>>();
    let unrooted = instances
        .difference(&placed_instances)
        .cloned()
        .collect::<Vec<_>>();
    if !unrooted.is_empty() {
        return Err(PlacementError::new(format!(
            "{label} has underconstrained unrooted instance(s): {}.",
            unrooted.join(", ")
        )));
    }

    Ok(PlacementGraphSolution {
        placements,
        solved_mate_ids: solved_mates.into_iter().collect(),
    })
}

pub fn validate_port_compatibility(
    source_type: &str,
    source_compatible_with: &[String],
    target_type: &str,
    target_compatible_with: &[String],
    label: &str,
) -> Result<(), PlacementError> {
    if source_type == target_type
        || source_compatible_with
            .iter()
            .any(|value| value == target_type)
        || target_compatible_with
            .iter()
            .any(|value| value == source_type)
    {
        return Ok(());
    }
    Err(PlacementError::new(format!(
        "{label} has incompatible port types `{source_type}` and `{target_type}`."
    )))
}

pub fn validate_clearance(
    required: Option<f64>,
    available: Option<f64>,
    label: &str,
) -> Result<(), PlacementError> {
    if required.is_some_and(|value| !value.is_finite())
        || available.is_some_and(|value| !value.is_finite())
    {
        return Err(PlacementError::new(format!(
            "{label} clearance values must be finite."
        )));
    }
    let Some(required) = required else {
        return Ok(());
    };
    let Some(available) = available else {
        return Err(PlacementError::new(format!(
            "{label} requires clearance {required}, but target clearance is missing."
        )));
    };
    if available + FRAME_EPSILON < required {
        return Err(PlacementError::new(format!(
            "{label} available clearance {available} is below required clearance {required}."
        )));
    }
    Ok(())
}

fn render_frame(frame: &PlacementFrame) -> String {
    format!(
        "origin={:?}, xAxis={:?}, yAxis={:?}, zAxis={:?}",
        frame.origin, frame.x_axis, frame.y_axis, frame.z_axis
    )
}

fn mirror_port_frame(
    frame: PlacementFrame,
    axis: MirrorAxis,
    label: &str,
) -> Result<PlacementFrame, PlacementError> {
    let reflect = |value: [f64; 3]| match axis {
        MirrorAxis::X => [-value[0], value[1], value[2]],
        MirrorAxis::Y => [value[0], -value[1], value[2]],
    };
    PlacementFrame::from_origin_x_z(
        reflect(frame.origin),
        reflect(frame.x_axis),
        reflect(frame.z_axis),
        &format!("{label} mirrored source port"),
    )
}

#[derive(Debug, Clone, Copy)]
struct RigidFrame {
    origin: [f64; 3],
    // Row-major matrix. Frame axes are columns.
    basis: [[f64; 3]; 3],
}

impl RigidFrame {
    fn try_from(frame: PlacementFrame, label: &str) -> Result<Self, PlacementError> {
        frame.validate(label)?;
        Ok(Self {
            origin: frame.origin,
            basis: matrix_from_columns(frame.x_axis, frame.y_axis, frame.z_axis),
        })
    }

    fn compose(&self, other: &Self) -> Self {
        Self {
            origin: add(self.origin, matrix_vector(self.basis, other.origin)),
            basis: matrix_multiply(self.basis, other.basis),
        }
    }

    fn inverse(&self) -> Self {
        let basis = transpose(self.basis);
        Self {
            origin: scale(matrix_vector(basis, self.origin), -1.0),
            basis,
        }
    }

    fn into_placement_frame(self) -> PlacementFrame {
        PlacementFrame {
            origin: self.origin,
            x_axis: matrix_column(self.basis, 0),
            y_axis: matrix_column(self.basis, 1),
            z_axis: matrix_column(self.basis, 2),
        }
    }
}

fn modifier_frame(modifiers: MateModifiers) -> RigidFrame {
    let radians = modifiers.roll_degrees.to_radians();
    let (sin, cos) = radians.sin_cos();
    let roll = [[cos, -sin, 0.0], [sin, cos, 0.0], [0.0, 0.0, 1.0]];
    let normal = match modifiers.normal_mode {
        MateNormalMode::Aligned => [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        // Keep X aligned while reversing Z; Y reverses to keep a rigid,
        // right-handed basis (180-degree rotation around local X).
        MateNormalMode::Opposed => [[1.0, 0.0, 0.0], [0.0, -1.0, 0.0], [0.0, 0.0, -1.0]],
    };
    RigidFrame {
        origin: modifiers.offset,
        basis: matrix_multiply(roll, normal),
    }
}

fn validate_origin(origin: [f64; 3], label: &str) -> Result<(), PlacementError> {
    if all_finite(origin) {
        Ok(())
    } else {
        Err(PlacementError::new(format!(
            "{label} frame origin must be finite."
        )))
    }
}

fn all_finite(value: [f64; 3]) -> bool {
    value.into_iter().all(f64::is_finite)
}

fn normalize(value: [f64; 3], label: &str, axis: &str) -> Result<[f64; 3], PlacementError> {
    if !all_finite(value) {
        return Err(PlacementError::new(format!(
            "{label} frame {axis} must be finite."
        )));
    }
    let length = norm(value);
    if length <= FRAME_EPSILON {
        return Err(PlacementError::new(format!(
            "{label} frame {axis} must be non-zero."
        )));
    }
    Ok(scale(value, 1.0 / length))
}

fn add(left: [f64; 3], right: [f64; 3]) -> [f64; 3] {
    [left[0] + right[0], left[1] + right[1], left[2] + right[2]]
}

fn sub(left: [f64; 3], right: [f64; 3]) -> [f64; 3] {
    [left[0] - right[0], left[1] - right[1], left[2] - right[2]]
}

fn scale(value: [f64; 3], factor: f64) -> [f64; 3] {
    [value[0] * factor, value[1] * factor, value[2] * factor]
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

fn norm(value: [f64; 3]) -> f64 {
    dot(value, value).sqrt()
}

fn matrix_from_columns(x: [f64; 3], y: [f64; 3], z: [f64; 3]) -> [[f64; 3]; 3] {
    [[x[0], y[0], z[0]], [x[1], y[1], z[1]], [x[2], y[2], z[2]]]
}

fn matrix_column(matrix: [[f64; 3]; 3], column: usize) -> [f64; 3] {
    [matrix[0][column], matrix[1][column], matrix[2][column]]
}

fn matrix_vector(matrix: [[f64; 3]; 3], vector: [f64; 3]) -> [f64; 3] {
    [
        dot(matrix[0], vector),
        dot(matrix[1], vector),
        dot(matrix[2], vector),
    ]
}

fn matrix_multiply(left: [[f64; 3]; 3], right: [[f64; 3]; 3]) -> [[f64; 3]; 3] {
    let right_t = transpose(right);
    [
        [
            dot(left[0], right_t[0]),
            dot(left[0], right_t[1]),
            dot(left[0], right_t[2]),
        ],
        [
            dot(left[1], right_t[0]),
            dot(left[1], right_t[1]),
            dot(left[1], right_t[2]),
        ],
        [
            dot(left[2], right_t[0]),
            dot(left[2], right_t[1]),
            dot(left[2], right_t[2]),
        ],
    ]
}

fn transpose(matrix: [[f64; 3]; 3]) -> [[f64; 3]; 3] {
    [
        [matrix[0][0], matrix[1][0], matrix[2][0]],
        [matrix[0][1], matrix[1][1], matrix[2][1]],
        [matrix[0][2], matrix[1][2], matrix[2][2]],
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(origin: [f64; 3], x: [f64; 3], z: [f64; 3]) -> PlacementFrame {
        PlacementFrame::from_origin_x_z(origin, x, z, "test port").expect("valid frame")
    }

    #[test]
    fn derives_right_handed_y_axis_and_rejects_parallel_axes() {
        let identity = frame([0.0; 3], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]);
        assert_eq!(identity.y_axis, [0.0, 1.0, 0.0]);

        let error = PlacementFrame::from_origin_x_z(
            [0.0; 3],
            [0.0, 0.0, 2.0],
            [0.0, 0.0, 1.0],
            "component `latch` port `mount`",
        )
        .expect_err("parallel axes fail");
        assert!(error.message.contains("latch"), "{}", error.message);
        assert!(error.message.contains("mount"), "{}", error.message);
        assert!(error.message.contains("xAxis"), "{}", error.message);
    }

    #[test]
    fn opposed_mate_maps_source_port_to_target_without_euler_input() {
        let source = frame([0.0; 3], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]);
        let target = frame([50.0, 0.0, 15.0], [0.0, 1.0, 0.0], [1.0, 0.0, 0.0]);
        let solved =
            solve_mate(&source, &target, MateModifiers::opposed(), "side latch").expect("solve");
        assert_eq!(solved.placement_frame.origin, [50.0, 0.0, 15.0]);
        assert!(norm(sub(solved.placement_frame.x_axis, [0.0, 1.0, 0.0])) < FRAME_EPSILON);
        assert!(norm(sub(solved.placement_frame.z_axis, [-1.0, 0.0, 0.0])) < FRAME_EPSILON);
    }

    #[test]
    fn roll_and_target_local_offset_compose_before_source_inverse() {
        let source = frame([2.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]);
        let target = frame([10.0, 20.0, 30.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]);
        let solved = solve_mate(
            &source,
            &target,
            MateModifiers {
                normal_mode: MateNormalMode::Aligned,
                roll_degrees: 90.0,
                offset: [1.0, 2.0, 3.0],
                mirror_axis: None,
            },
            "rolled latch",
        )
        .expect("solve");
        assert!(norm(sub(solved.placement_frame.origin, [11.0, 20.0, 33.0])) < FRAME_EPSILON);
        assert!(norm(sub(solved.placement_frame.x_axis, [0.0, 1.0, 0.0])) < FRAME_EPSILON);
    }

    #[test]
    fn mirror_is_recorded_separately_from_right_handed_placement() {
        let source = frame([0.0; 3], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]);
        let target = frame([0.0; 3], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]);
        let solved = solve_mate(
            &source,
            &target,
            MateModifiers {
                normal_mode: MateNormalMode::Aligned,
                roll_degrees: 0.0,
                offset: [0.0; 3],
                mirror_axis: Some(MirrorAxis::X),
            },
            "mirrored latch",
        )
        .expect("solve");
        assert_eq!(solved.mirror_axis, Some(MirrorAxis::X));
        solved
            .placement_frame
            .validate("mirrored placement")
            .expect("right handed");
    }

    #[test]
    fn placement_graph_accepts_consistent_cycle_and_rejects_conflict() {
        let identity = PlacementFrame::identity();
        let shifted = frame([10.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]);
        let mate = |mate_id: &str, a: &str, a_frame, b: &str, b_frame| PlacementGraphMate {
            mate_id: mate_id.to_string(),
            a_instance_id: a.to_string(),
            a_port_id: format!("{a}-port"),
            a_port_frame: a_frame,
            b_instance_id: b.to_string(),
            b_port_id: format!("{b}-port"),
            b_port_frame: b_frame,
        };
        let consistent = vec![
            mate("a-b", "a", identity, "b", identity),
            mate("b-c", "b", identity, "c", identity),
            mate("c-a", "c", identity, "a", identity),
        ];
        let solved = solve_placement_graph(
            ["a", "b", "c"].into_iter().map(str::to_string),
            BTreeMap::from([("a".to_string(), identity)]),
            &consistent,
            "fixture",
        )
        .expect("consistent cycle");
        assert_eq!(solved.placements.len(), 3);

        let conflicting = vec![
            mate("a-b", "a", identity, "b", identity),
            mate("b-c", "b", identity, "c", identity),
            mate("c-a", "c", shifted, "a", identity),
        ];
        let error = solve_placement_graph(
            ["a", "b", "c"].into_iter().map(str::to_string),
            BTreeMap::from([("a".to_string(), identity)]),
            &conflicting,
            "fixture",
        )
        .expect_err("conflicting cycle");
        assert!(error.message.contains("b-c"), "{}", error.message);
        assert!(error.message.contains("c-a"), "{}", error.message);
        assert!(
            error.message.contains("existing mate path"),
            "{}",
            error.message
        );
    }

    #[test]
    fn placement_graph_rejects_unrooted_instances() {
        let error = solve_placement_graph(
            ["loose-a", "loose-b"].into_iter().map(str::to_string),
            BTreeMap::new(),
            &[PlacementGraphMate {
                mate_id: "loose-mate".to_string(),
                a_instance_id: "loose-a".to_string(),
                a_port_id: "mount".to_string(),
                a_port_frame: PlacementFrame::identity(),
                b_instance_id: "loose-b".to_string(),
                b_port_id: "mount".to_string(),
                b_port_frame: PlacementFrame::identity(),
            }],
            "fixture",
        )
        .expect_err("unrooted graph");
        assert!(
            error.message.contains("underconstrained"),
            "{}",
            error.message
        );
        assert!(error.message.contains("loose-a"), "{}", error.message);
        assert!(error.message.contains("loose-b"), "{}", error.message);
    }

    #[test]
    fn compatibility_and_clearance_have_shared_deterministic_rules() {
        validate_port_compatibility(
            "rail.v1",
            &[],
            "slot.v1",
            &["rail.v1".to_string()],
            "mate `rail-slot`",
        )
        .expect("declared compatibility");
        let incompatible =
            validate_port_compatibility("rail.v1", &[], "bolt.v1", &[], "mate `bad`")
                .expect_err("incompatible");
        assert!(incompatible.message.contains("rail.v1"));
        assert!(incompatible.message.contains("bolt.v1"));

        validate_clearance(Some(0.3), Some(0.4), "mate `fit`").expect("clearance");
        let too_tight =
            validate_clearance(Some(0.5), Some(0.4), "mate `fit`").expect_err("too tight");
        assert!(too_tight.message.contains("0.4"));
        assert!(too_tight.message.contains("0.5"));
    }
}
