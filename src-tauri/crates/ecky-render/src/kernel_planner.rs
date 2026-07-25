use std::collections::BTreeMap;
use std::fmt;

use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::core_ir::{
    CoreArrayOp, CoreBooleanOp, CoreEdgeAxis, CoreEdgeBound, CoreEdgeSelectorClause,
    CoreFaceAreaRank, CoreFaceSelectorClause, CoreFrameOp, CoreLiteral, CoreMetaOp, CoreNode,
    CoreNodeKind, CoreOperation, CoreParameterValue, CorePathOp, CorePrimitive, CoreProgram,
    CoreReference, CoreSelectorPayload, CoreSurfaceOp, CoreSymbol, CoreTransformOp,
};
use crate::{
    KernelArg, KernelCommand, KernelKeyword, KernelPart, KernelPlan, KernelPlanner,
    RenderParameters, RENDER_SCHEMA_VERSION,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortablePlanError {
    code: &'static str,
    message: String,
}

impl PortablePlanError {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    pub fn code(&self) -> &'static str {
        self.code
    }
}

impl fmt::Display for PortablePlanError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for PortablePlanError {}

#[derive(Debug, Clone, Copy, Default)]
pub struct PortableKernelPlanner;

impl KernelPlanner for PortableKernelPlanner {
    type Error = PortablePlanError;

    fn plan(
        &self,
        program: &CoreProgram,
        parameters: &RenderParameters,
    ) -> Result<KernelPlan, Self::Error> {
        crate::core_ir::verify_core_program(program).map_err(|error| {
            PortablePlanError::new(
                "invalidCoreProgram",
                format!("Portable planner rejected invalid Core IR: {error}"),
            )
        })?;

        let parameter_names = program
            .parameters
            .iter()
            .map(|parameter| (parameter.id.raw(), parameter.key.clone()))
            .collect::<BTreeMap<_, _>>();
        let parameter_values = program
            .parameters
            .iter()
            .map(|parameter| {
                let value = parameters
                    .get(&parameter.key)
                    .cloned()
                    .unwrap_or_else(|| parameter_default_json(&parameter.default_value));
                (parameter.key.clone(), value)
            })
            .collect::<BTreeMap<_, _>>();

        let mut parts = Vec::with_capacity(program.parts.len());
        for part in &program.parts {
            let mut lowerer = PartLowerer::new(&parameter_names, &parameter_values);
            let root = lowerer.lower_shape(&part.root)?;
            parts.push(KernelPart {
                key: part.key.clone(),
                label: part.label.clone(),
                root,
                commands: lowerer.commands,
            });
        }

        Ok(KernelPlan {
            schema_version: RENDER_SCHEMA_VERSION,
            plan_id: plan_digest(&parts)?,
            parts,
        })
    }
}

#[derive(Debug, Clone)]
enum LoweredValue {
    Arg(KernelArg),
    Shape(u64),
}

struct PartLowerer<'a> {
    parameter_names: &'a BTreeMap<u64, String>,
    parameter_values: &'a BTreeMap<String, Value>,
    locals: BTreeMap<String, LoweredValue>,
    nodes: BTreeMap<u64, LoweredValue>,
    commands: Vec<KernelCommand>,
}

impl<'a> PartLowerer<'a> {
    fn new(
        parameter_names: &'a BTreeMap<u64, String>,
        parameter_values: &'a BTreeMap<String, Value>,
    ) -> Self {
        Self {
            parameter_names,
            parameter_values,
            locals: BTreeMap::new(),
            nodes: BTreeMap::new(),
            commands: Vec::new(),
        }
    }

    fn lower_shape(&mut self, node: &CoreNode) -> Result<u64, PortablePlanError> {
        match self.lower_value(node)? {
            LoweredValue::Shape(slot) => Ok(slot),
            LoweredValue::Arg(_) => Err(PortablePlanError::new(
                "expectedShape",
                format!(
                    "Portable planner expected shape at Core node {}.",
                    node.id.raw()
                ),
            )),
        }
    }

    fn lower_value(&mut self, node: &CoreNode) -> Result<LoweredValue, PortablePlanError> {
        if let Some(value) = self.nodes.get(&node.id.raw()) {
            return Ok(value.clone());
        }

        let value = match &node.kind {
            CoreNodeKind::Literal(literal) => LoweredValue::Arg(literal_arg(literal)),
            CoreNodeKind::Reference(reference) => self.lower_reference(reference)?,
            CoreNodeKind::Build { bindings, result } => {
                let snapshot = self.locals.clone();
                for binding in bindings {
                    let value = self.lower_value(&binding.value)?;
                    self.locals.insert(binding.name.clone(), value);
                }
                let result = self.lower_value(result);
                self.locals = snapshot;
                result?
            }
            CoreNodeKind::Let { bindings, body } => {
                let snapshot = self.locals.clone();
                for binding in bindings {
                    let value = self.lower_value(&binding.value)?;
                    self.locals.insert(binding.name.clone(), value);
                }
                let result = self.lower_value(body);
                self.locals = snapshot;
                result?
            }
            CoreNodeKind::Call { op, args, keywords } => {
                self.lower_call(node, op, args, keywords)?
            }
            CoreNodeKind::List(items) => {
                let values = items
                    .iter()
                    .map(|item| self.lower_arg(item))
                    .collect::<Result<Vec<_>, _>>()?;
                LoweredValue::Arg(list_arg(values)?)
            }
            CoreNodeKind::Group(items) => {
                let args = items
                    .iter()
                    .map(|item| self.lower_shape(item).map(|slot| arg("ref", json!(slot))))
                    .collect::<Result<Vec<_>, _>>()?;
                let output = node.id.raw();
                self.commands.push(KernelCommand {
                    output,
                    op: "compound".to_string(),
                    args,
                    keywords: Vec::new(),
                });
                LoweredValue::Shape(output)
            }
            CoreNodeKind::If { .. }
            | CoreNodeKind::Range { .. }
            | CoreNodeKind::Map { .. }
            | CoreNodeKind::Apply { .. } => {
                return Err(PortablePlanError::new(
                    "unresolvedControlFlow",
                    format!(
                        "Portable planner requires normalized Core IR; unresolved control flow remains at node {}.",
                        node.id.raw()
                    ),
                ));
            }
        };

        self.nodes.insert(node.id.raw(), value.clone());
        Ok(value)
    }

    fn lower_call(
        &mut self,
        node: &CoreNode,
        operation: &CoreOperation,
        args: &[CoreNode],
        keywords: &[crate::core_ir::CoreKeywordArg],
    ) -> Result<LoweredValue, PortablePlanError> {
        if matches!(
            operation,
            CoreOperation::Primitive(CorePrimitive::Svg | CorePrimitive::Text | CorePrimitive::Stl)
        ) {
            return Err(PortablePlanError::new(
                "unresolvedAsset",
                format!(
                    "Portable planner requires `{}` to be resolved by a host asset/profile resolver.",
                    operation_token(operation)
                ),
            ));
        }

        let op = operation_token(operation);
        if matches!(
            operation,
            CoreOperation::Array(
                CoreArrayOp::Repeat
                    | CoreArrayOp::RepeatUnion
                    | CoreArrayOp::RepeatCompound
                    | CoreArrayOp::RepeatPick
            ) | CoreOperation::Boolean(CoreBooleanOp::Xor)
        ) {
            return Err(PortablePlanError::new(
                "unresolvedControlFlow",
                format!("Portable planner requires `{op}` to be normalized first."),
            ));
        }
        if matches!(operation, CoreOperation::Custom(name) if name == "hole") {
            return Err(PortablePlanError::new(
                "unresolvedHole",
                "Typed hole must be filled before portable lowering.",
            ));
        }

        let args = args
            .iter()
            .map(|argument| self.lower_arg(argument))
            .collect::<Result<Vec<_>, _>>()?;
        let keywords = keywords
            .iter()
            .map(|keyword| {
                let value = self.lower_arg(keyword.source_node())?;
                Ok(match keyword.selector_payload() {
                    Some(payload) => KernelKeyword {
                        name: keyword.name.clone(),
                        kind: "selector".to_string(),
                        value: Some(value),
                        payload: selector_payload_json(payload),
                    },
                    None => KernelKeyword {
                        name: keyword.name.clone(),
                        kind: "arg".to_string(),
                        value: Some(value),
                        payload: None,
                    },
                })
            })
            .collect::<Result<Vec<_>, PortablePlanError>>()?;
        let output = node.id.raw();
        self.commands.push(KernelCommand {
            output,
            op,
            args,
            keywords,
        });
        Ok(LoweredValue::Shape(output))
    }

    fn lower_arg(&mut self, node: &CoreNode) -> Result<KernelArg, PortablePlanError> {
        match self.lower_value(node)? {
            LoweredValue::Arg(value) => Ok(value),
            LoweredValue::Shape(slot) => Ok(arg("ref", json!(slot))),
        }
    }

    fn lower_reference(
        &self,
        reference: &CoreReference,
    ) -> Result<LoweredValue, PortablePlanError> {
        match reference {
            CoreReference::Parameter(id) => {
                let key = self.parameter_names.get(&id.raw()).ok_or_else(|| {
                    PortablePlanError::new(
                        "unknownParameter",
                        format!("Unknown Core parameter {}.", id.raw()),
                    )
                })?;
                let value = self.parameter_values.get(key).ok_or_else(|| {
                    PortablePlanError::new(
                        "unknownParameter",
                        format!("No value supplied for parameter `{key}`."),
                    )
                })?;
                Ok(LoweredValue::Arg(json_arg(value.clone())?))
            }
            CoreReference::Node(id) => self.nodes.get(&id.raw()).cloned().ok_or_else(|| {
                PortablePlanError::new(
                    "unknownReference",
                    format!("Core node reference {} was not lowered.", id.raw()),
                )
            }),
            CoreReference::Local(name) => self.locals.get(name).cloned().ok_or_else(|| {
                PortablePlanError::new(
                    "unknownReference",
                    format!("Unknown local Core binding `{name}`."),
                )
            }),
            CoreReference::Part(id) => Err(PortablePlanError::new(
                "unresolvedPartReference",
                format!(
                    "Cross-part reference {} requires host normalization.",
                    id.raw()
                ),
            )),
        }
    }
}

fn parameter_default_json(value: &CoreParameterValue) -> Value {
    match value {
        CoreParameterValue::Number(value) => json!(value),
        CoreParameterValue::Boolean(value) => json!(value),
        CoreParameterValue::Text(value)
        | CoreParameterValue::Choice(value)
        | CoreParameterValue::Image(value) => json!(value),
    }
}

fn literal_arg(literal: &CoreLiteral) -> KernelArg {
    match literal {
        CoreLiteral::Number(value) => arg("number", json!(value)),
        CoreLiteral::Boolean(value) => arg("boolean", json!(value)),
        CoreLiteral::Text(value) => arg("text", json!(value)),
        CoreLiteral::Symbol(value) => arg("symbol", json!(symbol_name(value))),
        CoreLiteral::Point2(value) => arg("point2", json!(value)),
        CoreLiteral::Point3(value) => arg("point3", json!(value)),
    }
}

fn json_arg(value: Value) -> Result<KernelArg, PortablePlanError> {
    match value {
        Value::Number(value) => Ok(arg("number", Value::Number(value))),
        Value::Bool(value) => Ok(arg("boolean", json!(value))),
        Value::String(value) => Ok(arg("text", json!(value))),
        Value::Array(values) => {
            let args = values
                .into_iter()
                .map(json_arg)
                .collect::<Result<Vec<_>, _>>()?;
            list_arg(args)
        }
        other => Err(PortablePlanError::new(
            "invalidParameter",
            format!("Kernel parameters must be scalar or list values, got {other}."),
        )),
    }
}

fn list_arg(values: Vec<KernelArg>) -> Result<KernelArg, PortablePlanError> {
    let encoded = values
        .iter()
        .map(serde_json::to_value)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| {
            PortablePlanError::new(
                "planSerialization",
                format!("Could not encode kernel list argument: {error}"),
            )
        })?;
    Ok(arg("list", Value::Array(encoded)))
}

fn arg(kind: &str, value: Value) -> KernelArg {
    KernelArg {
        kind: kind.to_string(),
        value,
    }
}

fn operation_token(operation: &CoreOperation) -> String {
    match operation {
        CoreOperation::Primitive(CorePrimitive::Box) => "box",
        CoreOperation::Primitive(CorePrimitive::Sphere) => "sphere",
        CoreOperation::Primitive(CorePrimitive::Cylinder) => "cylinder",
        CoreOperation::Primitive(CorePrimitive::Cone) => "cone",
        CoreOperation::Primitive(CorePrimitive::Torus) => "torus",
        CoreOperation::Primitive(CorePrimitive::Wedge) => "wedge",
        CoreOperation::Primitive(CorePrimitive::Circle) => "circle",
        CoreOperation::Primitive(CorePrimitive::Ellipse) => "ellipse",
        CoreOperation::Primitive(CorePrimitive::Slot) => "slot-overall",
        CoreOperation::Primitive(CorePrimitive::SlotArc) => "slot-arc",
        CoreOperation::Primitive(CorePrimitive::Rectangle) => "rectangle",
        CoreOperation::Primitive(CorePrimitive::RoundedRectangle) => "rounded-rect",
        CoreOperation::Primitive(CorePrimitive::RoundedPolygon) => "rounded-polygon",
        CoreOperation::Primitive(CorePrimitive::Polygon) => "polygon",
        CoreOperation::Primitive(CorePrimitive::Profile) => "profile",
        CoreOperation::Primitive(CorePrimitive::MakeFace) => "make-face",
        CoreOperation::Primitive(CorePrimitive::Text) => "text",
        CoreOperation::Primitive(CorePrimitive::Svg) => "svg",
        CoreOperation::Primitive(CorePrimitive::Stl) => "import-stl",
        CoreOperation::Boolean(CoreBooleanOp::Union) => "union",
        CoreOperation::Boolean(CoreBooleanOp::Difference) => "difference",
        CoreOperation::Boolean(CoreBooleanOp::Intersection) => "intersection",
        CoreOperation::Boolean(CoreBooleanOp::Xor) => "xor",
        CoreOperation::Transform(CoreTransformOp::Translate) => "translate",
        CoreOperation::Transform(CoreTransformOp::Rotate) => "rotate",
        CoreOperation::Transform(CoreTransformOp::Scale) => "scale",
        CoreOperation::Transform(CoreTransformOp::Mirror) => "mirror",
        CoreOperation::Surface(CoreSurfaceOp::Extrude) => "extrude",
        CoreOperation::Surface(CoreSurfaceOp::Revolve) => "revolve",
        CoreOperation::Surface(CoreSurfaceOp::Loft) => "loft",
        CoreOperation::Surface(CoreSurfaceOp::Sweep) => "sweep",
        CoreOperation::Surface(CoreSurfaceOp::Shell) => "shell",
        CoreOperation::Surface(CoreSurfaceOp::Offset)
        | CoreOperation::Surface(CoreSurfaceOp::OffsetRounded) => "offset",
        CoreOperation::Surface(CoreSurfaceOp::Fillet) => "fillet",
        CoreOperation::Surface(CoreSurfaceOp::Chamfer) => "chamfer",
        CoreOperation::Surface(CoreSurfaceOp::Taper) => "taper",
        CoreOperation::Surface(CoreSurfaceOp::Twist) => "twist",
        CoreOperation::Surface(CoreSurfaceOp::Draft) => "draft",
        CoreOperation::Path(CorePathOp::Polyline) => "path",
        CoreOperation::Path(CorePathOp::BezierPath) => "bezier-path",
        CoreOperation::Path(CorePathOp::Bspline) => "bspline",
        CoreOperation::Array(CoreArrayOp::LinearArray) => "linear-array",
        CoreOperation::Array(CoreArrayOp::RadialArray) => "radial-array",
        CoreOperation::Array(CoreArrayOp::GridArray) => "grid-array",
        CoreOperation::Array(CoreArrayOp::ArcArray) => "arc-array",
        CoreOperation::Array(CoreArrayOp::Repeat) => "repeat",
        CoreOperation::Array(CoreArrayOp::RepeatUnion) => "repeat-union",
        CoreOperation::Array(CoreArrayOp::RepeatCompound) => "repeat-compound",
        CoreOperation::Array(CoreArrayOp::RepeatPick) => "repeat-pick",
        CoreOperation::Frame(CoreFrameOp::Plane) => "plane",
        CoreOperation::Frame(CoreFrameOp::Location) => "location",
        CoreOperation::Frame(CoreFrameOp::PathFrame) => "path-frame",
        CoreOperation::Frame(CoreFrameOp::Place) => "place",
        CoreOperation::Frame(CoreFrameOp::ClipBox) => "clip-box",
        CoreOperation::Meta(CoreMetaOp::Group) => "compound",
        CoreOperation::Meta(CoreMetaOp::Comment) => "comment",
        CoreOperation::Meta(CoreMetaOp::Annotate) => "annotate",
        CoreOperation::Custom(name) => return name.clone(),
    }
    .to_string()
}

fn symbol_name(symbol: &CoreSymbol) -> &'static str {
    match symbol {
        CoreSymbol::Start => "start",
        CoreSymbol::End => "end",
        CoreSymbol::Xy => "xy",
        CoreSymbol::Yz => "yz",
        CoreSymbol::Xz => "xz",
        CoreSymbol::Min => "min",
        CoreSymbol::Center => "center",
        CoreSymbol::Max => "max",
    }
}

fn selector_payload_json(payload: &CoreSelectorPayload) -> Option<Value> {
    match payload {
        CoreSelectorPayload::EdgeAll => None,
        CoreSelectorPayload::EdgeTargetIds(target_ids) => Some(json!({
            "type": "targetIds",
            "kind": "edge",
            "targetIds": target_ids,
        })),
        CoreSelectorPayload::EdgeTag(tag) => Some(json!({
            "type": "targetIds",
            "kind": "edge",
            "targetIds": [format!("tag:{tag}")],
        })),
        CoreSelectorPayload::EdgeClauses(clauses) => Some(json!({
            "type": "clauses",
            "kind": "edge",
            "clauses": clauses.iter().map(edge_clause_json).collect::<Vec<_>>(),
        })),
        CoreSelectorPayload::FaceTargetIds(target_ids) => Some(json!({
            "type": "targetIds",
            "kind": "face",
            "targetIds": target_ids,
        })),
        CoreSelectorPayload::FaceTag(tag) => Some(json!({
            "type": "targetIds",
            "kind": "face",
            "targetIds": [format!("tag:{tag}")],
        })),
        CoreSelectorPayload::FaceClauses(clauses) => Some(json!({
            "type": "clauses",
            "kind": "face",
            "clauses": clauses.iter().map(face_clause_json).collect::<Vec<_>>(),
        })),
    }
}

fn edge_clause_json(clause: &CoreEdgeSelectorClause) -> Value {
    match clause {
        CoreEdgeSelectorClause::Axis(axis) => json!({
            "type": "axis",
            "axis": axis_name(axis),
        }),
        CoreEdgeSelectorClause::Boundary { axis, bound } => json!({
            "type": "boundary",
            "axis": axis_name(axis),
            "bound": bound_name(bound),
        }),
    }
}

fn face_clause_json(clause: &CoreFaceSelectorClause) -> Value {
    match clause {
        CoreFaceSelectorClause::Boundary { axis, bound } => json!({
            "type": "boundary",
            "axis": axis_name(axis),
            "bound": bound_name(bound),
        }),
        CoreFaceSelectorClause::Planar => json!({"type": "planar"}),
        CoreFaceSelectorClause::Normal(axis) => {
            json!({"type": "normal", "axis": axis_name(axis)})
        }
        CoreFaceSelectorClause::Area(rank) => json!({
            "type": "area",
            "rank": match rank {
                CoreFaceAreaRank::Min => "min",
                CoreFaceAreaRank::Max => "max",
            },
        }),
    }
}

fn axis_name(axis: &CoreEdgeAxis) -> &'static str {
    match axis {
        CoreEdgeAxis::X => "x",
        CoreEdgeAxis::Y => "y",
        CoreEdgeAxis::Z => "z",
    }
}

fn bound_name(bound: &CoreEdgeBound) -> &'static str {
    match bound {
        CoreEdgeBound::Min => "min",
        CoreEdgeBound::Max => "max",
    }
}

fn plan_digest(parts: &[KernelPart]) -> Result<String, PortablePlanError> {
    let encoded = serde_json::to_vec(parts).map_err(|error| {
        PortablePlanError::new(
            "planSerialization",
            format!("Could not encode portable kernel plan: {error}"),
        )
    })?;
    let digest = Sha256::digest(encoded);
    Ok(format!("sha256:{digest:x}"))
}
