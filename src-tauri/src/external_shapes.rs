use crate::contracts::{AppError, AppResult, CaptureSurfaceAnchor};
use crate::ecky_core_ir::{
    CoreFrameOp, CoreLiteral, CoreNode, CoreNodeKind, CoreOperation, CorePrimitive,
};
use crate::ecky_ir::mesh_asset::{IndexedMeshAsset, MeshAssetSource};
use crate::mcp::contracts::EckyAstEditOperation;
use serde::{Deserialize, Serialize};
use specta::Type;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ExternalShapeSource {
    pub node_id: u64,
    pub part_key: String,
    pub path: String,
    pub display_name: String,
    pub source_digest: String,
    pub content_digest: Option<String>,
    pub byte_length: Option<u64>,
    pub exists: bool,
    pub plane_crops: Vec<ExternalShapePlaneCrop>,
    pub surface_trims: Vec<ExternalShapeSurfaceTrim>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ExternalShapePlaneCrop {
    pub node_id: u64,
    pub origin: [f64; 3],
    pub normal: [f64; 3],
    pub keep_positive: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ExternalShapeSurfaceTrimAnchor {
    pub triangle_index: u64,
    pub barycentric: [f64; 3],
    pub source_position: Option<[f64; 3]>,
    pub source_normal: Option<[f64; 3]>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ExternalShapeSurfaceTrim {
    pub node_id: u64,
    pub schema_version: u32,
    pub source_digest: String,
    pub loop_anchors: Vec<ExternalShapeSurfaceTrimAnchor>,
    pub keep_seed: ExternalShapeSurfaceTrimAnchor,
    pub path_mode: crate::surface_trim_external_shapes::SurfaceTrimPathMode,
    pub cap_mode: crate::surface_trim_cap::SurfaceTrimCapMode,
}

#[derive(Clone, Debug, Deserialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ApplyExternalShapePlaneCropRequest {
    pub thread_id: String,
    pub node_id: u64,
    pub expected_source_digest: String,
    pub expected_mesh_content_digest: String,
    pub anchors: Vec<CaptureSurfaceAnchor>,
    pub keep_positive: bool,
    #[serde(default)]
    pub replace_crop_node_id: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ApplyExternalShapePlaneCropResult {
    pub source: String,
    pub source_digest: String,
    pub origin: [f64; 3],
    pub normal: [f64; 3],
    pub keep_positive: bool,
}

#[derive(Clone, Debug, Deserialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemoveExternalShapePlaneCropRequest {
    pub thread_id: String,
    pub node_id: u64,
    pub crop_node_id: u64,
    pub expected_source_digest: String,
}

#[derive(Clone, Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RemoveExternalShapePlaneCropResult {
    pub source: String,
    pub source_digest: String,
    pub removed_crop_node_id: u64,
}

pub fn apply_plane_crop_to_source(
    source: &str,
    source_folder: &Path,
    request: &ApplyExternalShapePlaneCropRequest,
) -> AppResult<ApplyExternalShapePlaneCropResult> {
    let actual_source_digest = crate::services::render_snapshot::canonical_source_digest(source);
    if request.expected_source_digest != actual_source_digest {
        return Err(AppError::conflict(format!(
            "Bound model.ecky changed before plane crop: expected '{}', got '{}'.",
            request.expected_source_digest, actual_source_digest
        )));
    }
    if request.anchors.len() != 3 {
        return Err(AppError::validation(
            "Plane crop requires exactly three source mesh points.",
        ));
    }
    let sources = discover_bound_external_shapes(source, source_folder)?;
    let selected = sources
        .iter()
        .find(|item| item.node_id == request.node_id)
        .ok_or_else(|| {
            AppError::conflict(format!(
                "Imported STL node {} no longer exists in bound model.ecky.",
                request.node_id
            ))
        })?;
    if !selected.exists {
        return Err(AppError::not_found(format!(
            "Bound external shape is missing '{}'.",
            selected.path
        )));
    }
    if selected.content_digest.as_deref() != Some(&request.expected_mesh_content_digest) {
        return Err(AppError::conflict(
            "Bound external shape content changed before plane crop.",
        ));
    }
    let mesh_path = Path::new(&selected.path);
    let mut positions = Vec::with_capacity(3);
    for anchor in &request.anchors {
        let validated =
            crate::capture_guidance::validate_surface_anchor_from_stl(mesh_path, anchor, 1.0e-4)?;
        positions.push(validated.source_position);
    }
    let first = positions[0];
    let raw_normal = cross(subtract(positions[1], first), subtract(positions[2], first));
    let magnitude = length(raw_normal);
    if !magnitude.is_finite() || magnitude <= 1.0e-10 {
        return Err(AppError::validation(
            "Plane crop points are duplicate or collinear.",
        ));
    }
    let normal = raw_normal.map(|value| value / magnitude);
    let origin = [
        (positions[0][0] + positions[1][0] + positions[2][0]) / 3.0,
        (positions[0][1] + positions[1][1] + positions[2][1]) / 3.0,
        (positions[0][2] + positions[1][2] + positions[2][2]) / 3.0,
    ];

    let program = crate::ecky_scheme::compile_to_core_program(source).map_err(|error| {
        AppError::validation(format!(
            "Cannot apply plane crop to invalid model.ecky: {error}"
        ))
    })?;
    if let Some(crop_node_id) = request.replace_crop_node_id {
        if !selected
            .plane_crops
            .iter()
            .any(|crop| crop.node_id == crop_node_id)
        {
            return Err(AppError::conflict(format!(
                "Plane crop node {} no longer belongs to imported STL node {}.",
                crop_node_id, request.node_id
            )));
        }
    }
    let wrap_target = program
        .parts
        .iter()
        .find_map(|part| {
            let root_path = format!("/parts/{}/root", ast_path_segment(&part.key));
            let target_node_id = request.replace_crop_node_id.unwrap_or(request.node_id);
            find_ast_target(
                &part.root,
                &root_path,
                target_node_id,
                None,
                request.replace_crop_node_id.is_none(),
            )
        })
        .ok_or_else(|| {
            AppError::conflict(format!(
                "Imported STL node {} has no editable AST path.",
                request.node_id
            ))
        })?;
    let (wrap_path, wrap_node) = wrap_target;
    let inner_path = if request.replace_crop_node_id.is_some() {
        let CoreNodeKind::Call { args, .. } = &wrap_node.kind else {
            return Err(AppError::conflict(
                "Selected plane crop is no longer a call node.",
            ));
        };
        let inner = args
            .first()
            .ok_or_else(|| AppError::conflict("Selected plane crop has no source child."))?;
        let _ = inner;
        format!("{wrap_path}/call/args/0")
    } else {
        wrap_path.clone()
    };
    let (start, end) = crate::mcp::handlers::source_span_for_ecky_path(source, &inner_path)?;
    let wrapped = source.get(start..end).ok_or_else(|| {
        AppError::internal("Plane crop AST span is outside bound model.ecky bytes.")
    })?;
    let keep = if request.keep_positive {
        "positive"
    } else {
        "negative"
    };
    let replacement = format!(
        "(clip-plane\n  {wrapped}\n  :origin {}\n  :normal {}\n  :keep \"{keep}\")",
        format_point3(origin),
        format_point3(normal),
    );
    let next_source = crate::mcp::handlers::replace_ecky_ast_source(
        source,
        &actual_source_digest,
        &wrap_path,
        &crate::mcp::handlers::core_node_digest(wrap_node),
        &EckyAstEditOperation::Replace,
        Some(&replacement),
        None,
    )?;
    let source_digest = crate::services::render_snapshot::canonical_source_digest(&next_source);
    Ok(ApplyExternalShapePlaneCropResult {
        source: next_source,
        source_digest,
        origin,
        normal,
        keep_positive: request.keep_positive,
    })
}

pub fn remove_plane_crop_from_source(
    source: &str,
    source_folder: &Path,
    request: &RemoveExternalShapePlaneCropRequest,
) -> AppResult<RemoveExternalShapePlaneCropResult> {
    let actual_source_digest = crate::services::render_snapshot::canonical_source_digest(source);
    if request.expected_source_digest != actual_source_digest {
        return Err(AppError::conflict(format!(
            "Bound model.ecky changed before plane crop removal: expected '{}', got '{}'.",
            request.expected_source_digest, actual_source_digest
        )));
    }
    let sources = discover_bound_external_shapes(source, source_folder)?;
    let selected = sources
        .iter()
        .find(|item| item.node_id == request.node_id)
        .ok_or_else(|| {
            AppError::conflict(format!(
                "Imported STL node {} no longer exists in bound model.ecky.",
                request.node_id
            ))
        })?;
    if !selected
        .plane_crops
        .iter()
        .any(|crop| crop.node_id == request.crop_node_id)
    {
        return Err(AppError::conflict(format!(
            "Plane crop node {} no longer belongs to imported STL node {}.",
            request.crop_node_id, request.node_id
        )));
    }

    let program = crate::ecky_scheme::compile_to_core_program(source).map_err(|error| {
        AppError::validation(format!(
            "Cannot remove plane crop from invalid model.ecky: {error}"
        ))
    })?;
    let (crop_path, crop_node) = program
        .parts
        .iter()
        .find_map(|part| {
            let root_path = format!("/parts/{}/root", ast_path_segment(&part.key));
            find_ast_target(&part.root, &root_path, request.crop_node_id, None, false)
        })
        .ok_or_else(|| {
            AppError::conflict(format!(
                "Plane crop node {} no longer exists in bound model.ecky.",
                request.crop_node_id
            ))
        })?;
    let CoreNodeKind::Call { args, .. } = &crop_node.kind else {
        return Err(AppError::conflict(
            "Selected plane crop is no longer a call node.",
        ));
    };
    args.first()
        .ok_or_else(|| AppError::conflict("Selected plane crop has no source child."))?;
    let inner_path = format!("{crop_path}/call/args/0");
    let (start, end) = crate::mcp::handlers::source_span_for_ecky_path(source, &inner_path)?;
    let replacement = source.get(start..end).ok_or_else(|| {
        AppError::internal("Plane crop child AST span is outside bound model.ecky bytes.")
    })?;
    let next_source = crate::mcp::handlers::replace_ecky_ast_source(
        source,
        &actual_source_digest,
        &crop_path,
        &crate::mcp::handlers::core_node_digest(crop_node),
        &EckyAstEditOperation::Replace,
        Some(replacement),
        None,
    )?;
    let source_digest = crate::services::render_snapshot::canonical_source_digest(&next_source);
    Ok(RemoveExternalShapePlaneCropResult {
        source: next_source,
        source_digest,
        removed_crop_node_id: request.crop_node_id,
    })
}

pub(crate) fn ast_path_segment(value: &str) -> String {
    value.replace('~', "~0").replace('/', "~1")
}

pub(crate) fn find_ast_target<'a>(
    node: &'a CoreNode,
    path: &str,
    target_node_id: u64,
    wrapper: Option<(String, &'a CoreNode)>,
    wrap_import: bool,
) -> Option<(String, &'a CoreNode)> {
    if node.id.raw() == target_node_id
        && (!wrap_import
            || matches!(
                node.kind,
                CoreNodeKind::Call {
                    op: CoreOperation::Primitive(CorePrimitive::Stl),
                    ..
                }
            ))
    {
        return if wrap_import {
            wrapper.or_else(|| Some((path.to_string(), node)))
        } else {
            Some((path.to_string(), node))
        };
    }
    match &node.kind {
        CoreNodeKind::Call { op, args, keywords } => {
            let is_crop_wrapper = matches!(op, CoreOperation::Frame(CoreFrameOp::ClipPlane))
                || matches!(op, CoreOperation::Custom(name) if name == "surface-trim" || name == "solidify");
            for (index, arg) in args.iter().enumerate() {
                let next_wrapper = if wrap_import && is_crop_wrapper && index == 0 {
                    wrapper.clone().or_else(|| Some((path.to_string(), node)))
                } else {
                    None
                };
                let child_path = format!("{path}/call/args/{index}");
                if let Some(target) =
                    find_ast_target(arg, &child_path, target_node_id, next_wrapper, wrap_import)
                {
                    return Some(target);
                }
            }
            for keyword in keywords {
                let child_path =
                    format!("{path}/call/keywords/{}", ast_path_segment(&keyword.name));
                if let Some(target) = find_ast_target(
                    keyword.source_node(),
                    &child_path,
                    target_node_id,
                    None,
                    wrap_import,
                ) {
                    return Some(target);
                }
            }
            None
        }
        CoreNodeKind::Build { bindings, result } => bindings
            .iter()
            .find_map(|binding| {
                let child_path =
                    format!("{path}/build/bindings/{}", ast_path_segment(&binding.name));
                find_ast_target(
                    &binding.value,
                    &child_path,
                    target_node_id,
                    None,
                    wrap_import,
                )
            })
            .or_else(|| {
                find_ast_target(
                    result,
                    &format!("{path}/build/result"),
                    target_node_id,
                    None,
                    wrap_import,
                )
            }),
        CoreNodeKind::Let { bindings, body } => bindings
            .iter()
            .find_map(|binding| {
                let child_path = format!("{path}/let/bindings/{}", ast_path_segment(&binding.name));
                find_ast_target(
                    &binding.value,
                    &child_path,
                    target_node_id,
                    None,
                    wrap_import,
                )
            })
            .or_else(|| {
                find_ast_target(
                    body,
                    &format!("{path}/let/body"),
                    target_node_id,
                    None,
                    wrap_import,
                )
            }),
        CoreNodeKind::If {
            condition,
            then_branch,
            else_branch,
        } => [
            condition.as_ref(),
            then_branch.as_ref(),
            else_branch.as_ref(),
        ]
        .into_iter()
        .enumerate()
        .find_map(|(index, child)| {
            let branch = ["condition", "then", "else"][index];
            find_ast_target(
                child,
                &format!("{path}/if/{branch}"),
                target_node_id,
                None,
                wrap_import,
            )
        }),
        CoreNodeKind::Range { start, end } => find_ast_target(
            start,
            &format!("{path}/range/start"),
            target_node_id,
            None,
            wrap_import,
        )
        .or_else(|| {
            find_ast_target(
                end,
                &format!("{path}/range/end"),
                target_node_id,
                None,
                wrap_import,
            )
        }),
        CoreNodeKind::Map { sources, body, .. } => sources
            .iter()
            .enumerate()
            .find_map(|(index, item)| {
                find_ast_target(
                    item,
                    &format!("{path}/map/sources/{index}"),
                    target_node_id,
                    None,
                    wrap_import,
                )
            })
            .or_else(|| {
                find_ast_target(
                    body,
                    &format!("{path}/map/body"),
                    target_node_id,
                    None,
                    wrap_import,
                )
            }),
        CoreNodeKind::Apply { args, list, .. } => args
            .iter()
            .enumerate()
            .find_map(|(index, item)| {
                find_ast_target(
                    item,
                    &format!("{path}/apply/args/{index}"),
                    target_node_id,
                    None,
                    wrap_import,
                )
            })
            .or_else(|| {
                find_ast_target(
                    list,
                    &format!("{path}/apply/list"),
                    target_node_id,
                    None,
                    wrap_import,
                )
            }),
        CoreNodeKind::List(items) => items.iter().enumerate().find_map(|(index, item)| {
            find_ast_target(
                item,
                &format!("{path}/list/{index}"),
                target_node_id,
                None,
                wrap_import,
            )
        }),
        CoreNodeKind::Group(items) => items.iter().enumerate().find_map(|(index, item)| {
            find_ast_target(
                item,
                &format!("{path}/group/{index}"),
                target_node_id,
                None,
                wrap_import,
            )
        }),
        CoreNodeKind::Literal(_) | CoreNodeKind::Reference(_) => None,
    }
}

fn subtract(left: [f64; 3], right: [f64; 3]) -> [f64; 3] {
    [left[0] - right[0], left[1] - right[1], left[2] - right[2]]
}

fn cross(left: [f64; 3], right: [f64; 3]) -> [f64; 3] {
    [
        left[1] * right[2] - left[2] * right[1],
        left[2] * right[0] - left[0] * right[2],
        left[0] * right[1] - left[1] * right[0],
    ]
}

fn length(value: [f64; 3]) -> f64 {
    (value[0] * value[0] + value[1] * value[1] + value[2] * value[2]).sqrt()
}

fn format_point3(value: [f64; 3]) -> String {
    format!(
        "({} {} {})",
        format_scalar(value[0]),
        format_scalar(value[1]),
        format_scalar(value[2])
    )
}

fn format_scalar(value: f64) -> String {
    let formatted = format!("{value:.9}");
    let trimmed = formatted.trim_end_matches('0').trim_end_matches('.');
    if trimmed == "-0" {
        "0".to_string()
    } else {
        trimmed.to_string()
    }
}

pub fn discover_bound_external_shapes(
    source: &str,
    source_folder: &Path,
) -> AppResult<Vec<ExternalShapeSource>> {
    if source.trim().is_empty() {
        return Ok(Vec::new());
    }
    let program = crate::ecky_scheme::compile_to_core_program(source).map_err(|error| {
        AppError::validation(format!(
            "Cannot inspect bound external shapes because model.ecky is invalid: {error}"
        ))
    })?;
    let source_digest = crate::services::render_snapshot::canonical_source_digest(source);
    let mut imports = Vec::new();
    for part in &program.parts {
        visit_imports(
            &part.root,
            &part.key,
            source_folder,
            &source_digest,
            &[],
            &mut imports,
        );
    }
    for import in &mut imports {
        for part in &program.parts {
            collect_surface_trims_for_import(&part.root, import.node_id, &mut import.surface_trims);
        }
        import.surface_trims.sort_by_key(|trim| trim.node_id);
        import.surface_trims.dedup_by_key(|trim| trim.node_id);
        materialize_surface_trim_anchors(import);
    }
    imports.sort_by_key(|item| item.node_id);
    Ok(imports)
}

fn visit_imports(
    node: &CoreNode,
    part_key: &str,
    source_folder: &Path,
    source_digest: &str,
    plane_crops: &[ExternalShapePlaneCrop],
    imports: &mut Vec<ExternalShapeSource>,
) {
    match &node.kind {
        CoreNodeKind::Call {
            op: CoreOperation::Primitive(CorePrimitive::Stl),
            args,
            ..
        } => {
            if let Some(CoreNode {
                kind: CoreNodeKind::Literal(CoreLiteral::Text(authored_path)),
                ..
            }) = args.first()
            {
                imports.push(external_shape_source(
                    node.id.raw(),
                    part_key,
                    authored_path,
                    source_folder,
                    source_digest,
                    plane_crops,
                ));
            }
            for arg in args {
                visit_imports(
                    arg,
                    part_key,
                    source_folder,
                    source_digest,
                    plane_crops,
                    imports,
                );
            }
        }
        CoreNodeKind::Call { op, args, keywords } => {
            for (index, arg) in args.iter().enumerate() {
                let next_plane_crops =
                    if index == 0 && matches!(op, CoreOperation::Frame(CoreFrameOp::ClipPlane)) {
                        read_plane_crop(node.id.raw(), keywords).map(|crop| {
                            let mut crops = plane_crops.to_vec();
                            crops.push(crop);
                            crops
                        })
                    } else {
                        None
                    };
                visit_imports(
                    arg,
                    part_key,
                    source_folder,
                    source_digest,
                    next_plane_crops.as_deref().unwrap_or(plane_crops),
                    imports,
                );
            }
            for keyword in keywords {
                visit_imports(
                    keyword.source_node(),
                    part_key,
                    source_folder,
                    source_digest,
                    plane_crops,
                    imports,
                );
            }
        }
        CoreNodeKind::Build { bindings, result } => {
            for binding in bindings {
                visit_imports(
                    &binding.value,
                    part_key,
                    source_folder,
                    source_digest,
                    plane_crops,
                    imports,
                );
            }
            visit_imports(
                result,
                part_key,
                source_folder,
                source_digest,
                plane_crops,
                imports,
            );
        }
        CoreNodeKind::Let { bindings, body } => {
            for binding in bindings {
                visit_imports(
                    &binding.value,
                    part_key,
                    source_folder,
                    source_digest,
                    plane_crops,
                    imports,
                );
            }
            visit_imports(
                body,
                part_key,
                source_folder,
                source_digest,
                plane_crops,
                imports,
            );
        }
        CoreNodeKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            for child in [
                condition.as_ref(),
                then_branch.as_ref(),
                else_branch.as_ref(),
            ] {
                visit_imports(
                    child,
                    part_key,
                    source_folder,
                    source_digest,
                    plane_crops,
                    imports,
                );
            }
        }
        CoreNodeKind::Range { start, end } => {
            visit_imports(
                start,
                part_key,
                source_folder,
                source_digest,
                plane_crops,
                imports,
            );
            visit_imports(
                end,
                part_key,
                source_folder,
                source_digest,
                plane_crops,
                imports,
            );
        }
        CoreNodeKind::Map { sources, body, .. } => {
            for source in sources {
                visit_imports(
                    source,
                    part_key,
                    source_folder,
                    source_digest,
                    plane_crops,
                    imports,
                );
            }
            visit_imports(
                body,
                part_key,
                source_folder,
                source_digest,
                plane_crops,
                imports,
            );
        }
        CoreNodeKind::Apply { args, list, .. } => {
            for arg in args {
                visit_imports(
                    arg,
                    part_key,
                    source_folder,
                    source_digest,
                    plane_crops,
                    imports,
                );
            }
            visit_imports(
                list,
                part_key,
                source_folder,
                source_digest,
                plane_crops,
                imports,
            );
        }
        CoreNodeKind::List(items) | CoreNodeKind::Group(items) => {
            for item in items {
                visit_imports(
                    item,
                    part_key,
                    source_folder,
                    source_digest,
                    plane_crops,
                    imports,
                );
            }
        }
        CoreNodeKind::Literal(_) | CoreNodeKind::Reference(_) => {}
    }
}

fn read_plane_crop(
    node_id: u64,
    keywords: &[crate::ecky_core_ir::CoreKeywordArg],
) -> Option<ExternalShapePlaneCrop> {
    let keyword = |name: &str| {
        keywords
            .iter()
            .find(|keyword| keyword.name == name)
            .map(|keyword| keyword.source_node())
    };
    let keep_positive = match &keyword("keep")?.kind {
        CoreNodeKind::Literal(CoreLiteral::Text(value)) if value == "positive" => true,
        CoreNodeKind::Literal(CoreLiteral::Text(value)) if value == "negative" => false,
        _ => return None,
    };
    Some(ExternalShapePlaneCrop {
        node_id,
        origin: literal_point3(keyword("origin")?)?,
        normal: literal_point3(keyword("normal")?)?,
        keep_positive,
    })
}

fn literal_point3(node: &CoreNode) -> Option<[f64; 3]> {
    if let CoreNodeKind::Literal(CoreLiteral::Point3(point)) = &node.kind {
        return Some(*point);
    }
    let CoreNodeKind::List(items) = &node.kind else {
        return None;
    };
    if items.len() != 3 {
        return None;
    }
    let mut point = [0.0; 3];
    for (index, item) in items.iter().enumerate() {
        let CoreNodeKind::Literal(CoreLiteral::Number(value)) = &item.kind else {
            return None;
        };
        point[index] = *value;
    }
    Some(point)
}

fn collect_surface_trims_for_import(
    node: &CoreNode,
    import_node_id: u64,
    trims: &mut Vec<ExternalShapeSurfaceTrim>,
) {
    if contains_node_id(node, import_node_id) {
        if let Some(trim) = read_surface_trim(node) {
            trims.push(trim);
        }
    }
    for child in child_nodes(node) {
        collect_surface_trims_for_import(child, import_node_id, trims);
    }
}

fn read_surface_trim(node: &CoreNode) -> Option<ExternalShapeSurfaceTrim> {
    let CoreNodeKind::Call {
        op: CoreOperation::Custom(name),
        keywords,
        ..
    } = &node.kind
    else {
        return None;
    };
    if name != "surface-trim" {
        return None;
    }
    let keyword = |name: &str| {
        keywords
            .iter()
            .find(|keyword| keyword.name == name)
            .map(|keyword| keyword.source_node())
    };
    let schema_version = match &keyword("schema-version")?.kind {
        CoreNodeKind::Literal(CoreLiteral::Number(value))
            if value.is_finite()
                && value.fract().abs() <= f64::EPSILON
                && *value >= 0.0
                && *value <= u32::MAX as f64 =>
        {
            *value as u32
        }
        _ => return None,
    };
    let source_digest = match &keyword("source-digest")?.kind {
        CoreNodeKind::Literal(CoreLiteral::Text(value)) => value.clone(),
        _ => return None,
    };
    let CoreNodeKind::List(items) = &keyword("loop")?.kind else {
        return None;
    };
    let loop_anchors = items
        .iter()
        .map(read_surface_trim_anchor)
        .collect::<Option<Vec<_>>>()?;
    let keep_seed = read_surface_trim_anchor(keyword("keep-seed")?)?;
    let path_mode = match &keyword("path-mode")?.kind {
        CoreNodeKind::Literal(CoreLiteral::Text(value)) if value == "shortest" => {
            crate::surface_trim_external_shapes::SurfaceTrimPathMode::Shortest
        }
        CoreNodeKind::Literal(CoreLiteral::Text(value)) if value == "feature" => {
            crate::surface_trim_external_shapes::SurfaceTrimPathMode::Feature
        }
        _ => return None,
    };
    let cap_mode = match &keyword("cap")?.kind {
        CoreNodeKind::Literal(CoreLiteral::Text(value)) if value == "open" => {
            crate::surface_trim_cap::SurfaceTrimCapMode::Open
        }
        CoreNodeKind::Literal(CoreLiteral::Text(value)) if value == "flat" => {
            crate::surface_trim_cap::SurfaceTrimCapMode::Flat
        }
        CoreNodeKind::Literal(CoreLiteral::Text(value)) if value == "surface-fill" => {
            crate::surface_trim_cap::SurfaceTrimCapMode::SurfaceFill
        }
        _ => return None,
    };
    Some(ExternalShapeSurfaceTrim {
        node_id: node.id.raw(),
        schema_version,
        source_digest,
        loop_anchors,
        keep_seed,
        path_mode,
        cap_mode,
    })
}

fn read_surface_trim_anchor(node: &CoreNode) -> Option<ExternalShapeSurfaceTrimAnchor> {
    let CoreNodeKind::Call {
        op: CoreOperation::Custom(name),
        args,
        ..
    } = &node.kind
    else {
        return None;
    };
    if name != "mesh-anchor" || args.len() != 4 {
        return None;
    }
    let number = |index: usize| match &args.get(index)?.kind {
        CoreNodeKind::Literal(CoreLiteral::Number(value)) if value.is_finite() => Some(*value),
        _ => None,
    };
    let triangle = number(0)?;
    if triangle < 0.0 || triangle.fract().abs() > f64::EPSILON || triangle > u64::MAX as f64 {
        return None;
    }
    Some(ExternalShapeSurfaceTrimAnchor {
        triangle_index: triangle as u64,
        barycentric: [number(1)?, number(2)?, number(3)?],
        source_position: None,
        source_normal: None,
    })
}

fn materialize_surface_trim_anchors(source: &mut ExternalShapeSource) {
    let Some(content_digest) = source.content_digest.as_deref() else {
        return;
    };
    let Ok(mesh) = IndexedMeshAsset::from_stl(MeshAssetSource::Imported, Path::new(&source.path))
    else {
        return;
    };
    for trim in &mut source.surface_trims {
        if trim.source_digest != content_digest {
            continue;
        }
        for anchor in trim
            .loop_anchors
            .iter_mut()
            .chain(std::iter::once(&mut trim.keep_seed))
        {
            let Some(triangle) = mesh.triangles().get(anchor.triangle_index as usize) else {
                continue;
            };
            let points = triangle.map(|index| mesh.vertices()[index as usize]);
            anchor.source_position = Some([
                points[0][0] * anchor.barycentric[0]
                    + points[1][0] * anchor.barycentric[1]
                    + points[2][0] * anchor.barycentric[2],
                points[0][1] * anchor.barycentric[0]
                    + points[1][1] * anchor.barycentric[1]
                    + points[2][1] * anchor.barycentric[2],
                points[0][2] * anchor.barycentric[0]
                    + points[1][2] * anchor.barycentric[1]
                    + points[2][2] * anchor.barycentric[2],
            ]);
            let raw_normal = cross(
                subtract(points[1], points[0]),
                subtract(points[2], points[0]),
            );
            let magnitude = length(raw_normal);
            if magnitude.is_finite() && magnitude > 1.0e-12 {
                anchor.source_normal = Some(raw_normal.map(|value| value / magnitude));
            }
        }
    }
}

fn contains_node_id(node: &CoreNode, node_id: u64) -> bool {
    node.id.raw() == node_id
        || child_nodes(node)
            .into_iter()
            .any(|child| contains_node_id(child, node_id))
}

fn child_nodes(node: &CoreNode) -> Vec<&CoreNode> {
    match &node.kind {
        CoreNodeKind::Call { args, keywords, .. } => args
            .iter()
            .chain(keywords.iter().map(|keyword| keyword.source_node()))
            .collect(),
        CoreNodeKind::Build { bindings, result } => bindings
            .iter()
            .map(|binding| &binding.value)
            .chain(std::iter::once(result.as_ref()))
            .collect(),
        CoreNodeKind::Let { bindings, body } => bindings
            .iter()
            .map(|binding| &binding.value)
            .chain(std::iter::once(body.as_ref()))
            .collect(),
        CoreNodeKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            vec![
                condition.as_ref(),
                then_branch.as_ref(),
                else_branch.as_ref(),
            ]
        }
        CoreNodeKind::Range { start, end } => vec![start.as_ref(), end.as_ref()],
        CoreNodeKind::Map { sources, body, .. } => sources
            .iter()
            .chain(std::iter::once(body.as_ref()))
            .collect(),
        CoreNodeKind::Apply { args, list, .. } => {
            args.iter().chain(std::iter::once(list.as_ref())).collect()
        }
        CoreNodeKind::List(items) | CoreNodeKind::Group(items) => items.iter().collect(),
        CoreNodeKind::Literal(_) | CoreNodeKind::Reference(_) => Vec::new(),
    }
}

fn external_shape_source(
    node_id: u64,
    part_key: &str,
    authored_path: &str,
    source_folder: &Path,
    source_digest: &str,
    plane_crops: &[ExternalShapePlaneCrop],
) -> ExternalShapeSource {
    let authored = PathBuf::from(authored_path);
    let resolved = if authored.is_absolute() {
        authored
    } else {
        source_folder.join(authored)
    };
    let bytes = std::fs::read(&resolved).ok();
    let mesh_content_digest = bytes.as_ref().and_then(|_| {
        IndexedMeshAsset::from_stl(MeshAssetSource::Imported, &resolved)
            .ok()
            .map(|mesh| mesh.content_digest().to_string())
    });
    ExternalShapeSource {
        node_id,
        part_key: part_key.to_string(),
        display_name: resolved
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(authored_path)
            .to_string(),
        path: resolved.to_string_lossy().to_string(),
        source_digest: source_digest.to_string(),
        content_digest: mesh_content_digest,
        byte_length: bytes.as_ref().map(|value| value.len() as u64),
        exists: bytes.is_some(),
        plane_crops: plane_crops.iter().rev().cloned().collect(),
        surface_trims: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "ecky-external-shapes-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ))
    }

    #[test]
    fn discovers_nested_bound_stl_and_resolves_relative_path() {
        let root = temp_root("bound-stl");
        std::fs::create_dir_all(&root).expect("mkdir");
        let stl = root.join("rocksteady-1.stl");
        let fixture =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/vertex-genie-ecky.stl");
        std::fs::copy(&fixture, &stl).expect("copy stl");
        let source = r#"(model (part head_only (solidify (import-stl "rocksteady-1.stl"))))"#;

        let sources = discover_bound_external_shapes(source, &root).expect("discover");

        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].part_key, "head_only");
        assert_eq!(sources[0].path, stl.to_string_lossy());
        assert_eq!(sources[0].display_name, "rocksteady-1.stl");
        assert!(sources[0].exists);
        assert_eq!(
            sources[0].byte_length,
            Some(std::fs::metadata(&stl).expect("metadata").len())
        );
        assert!(sources[0]
            .content_digest
            .as_deref()
            .is_some_and(|digest| digest.starts_with("sha256:")));
    }

    #[test]
    fn preserves_missing_import_without_substituting_another_mesh() {
        let root = temp_root("missing-stl");
        let source = r#"(model (part head_only (import-stl "missing.stl")))"#;

        let sources = discover_bound_external_shapes(source, &root).expect("discover");

        assert_eq!(sources.len(), 1);
        assert!(!sources[0].exists);
        assert_eq!(sources[0].content_digest, None);
        assert_eq!(sources[0].path, root.join("missing.stl").to_string_lossy());
    }

    #[test]
    fn applies_two_plane_crops_as_nested_canonical_source() {
        let fixture =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/vertex-genie-ecky.stl");
        let folder = fixture.parent().expect("fixture folder");
        let source = format!(
            "(model (part head_only (solidify (import-stl {:?}))))",
            fixture.to_string_lossy()
        );
        let discovered = discover_bound_external_shapes(&source, folder).expect("discover");
        let mesh = IndexedMeshAsset::from_stl(MeshAssetSource::Imported, &fixture).expect("mesh");
        let triangle = mesh.triangles()[0];
        let positions = [
            mesh.vertices()[triangle[0] as usize],
            mesh.vertices()[triangle[1] as usize],
            mesh.vertices()[triangle[2] as usize],
        ];
        let raw_normal = cross(
            subtract(positions[1], positions[0]),
            subtract(positions[2], positions[0]),
        );
        let magnitude = length(raw_normal);
        let normal = raw_normal.map(|value| value / magnitude);
        let barycentrics = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        let anchors = positions
            .into_iter()
            .zip(barycentrics)
            .map(|(source_position, barycentric)| CaptureSurfaceAnchor {
                source_mesh_content_digest: mesh.content_digest().to_string(),
                triangle_index: 0,
                barycentric,
                source_position,
                source_normal: normal,
            })
            .collect::<Vec<_>>();
        let first_request = ApplyExternalShapePlaneCropRequest {
            thread_id: "rocksteady-thread".to_string(),
            node_id: discovered[0].node_id,
            expected_source_digest: discovered[0].source_digest.clone(),
            expected_mesh_content_digest: mesh.content_digest().to_string(),
            anchors: anchors.clone(),
            keep_positive: true,
            replace_crop_node_id: None,
        };

        let first = apply_plane_crop_to_source(&source, folder, &first_request).expect("first");
        assert_eq!(first.source.matches("(clip-plane").count(), 1);
        let rediscovered =
            discover_bound_external_shapes(&first.source, folder).expect("rediscover");
        assert_eq!(rediscovered[0].plane_crops.len(), 1);
        assert!(rediscovered[0].plane_crops[0].keep_positive);
        let second_request = ApplyExternalShapePlaneCropRequest {
            node_id: rediscovered[0].node_id,
            expected_source_digest: rediscovered[0].source_digest.clone(),
            keep_positive: false,
            ..first_request
        };

        let second =
            apply_plane_crop_to_source(&first.source, folder, &second_request).expect("second");
        assert_eq!(second.source.matches("(clip-plane").count(), 2);
        assert!(second.source.contains(":keep \"positive\""));
        assert!(second.source.contains(":keep \"negative\""));
        let rediscovered =
            discover_bound_external_shapes(&second.source, folder).expect("rediscover twice");
        assert_eq!(rediscovered[0].plane_crops.len(), 2);
        assert!(rediscovered[0].plane_crops[0].keep_positive);
        assert!(!rediscovered[0].plane_crops[1].keep_positive);

        let edited_request = ApplyExternalShapePlaneCropRequest {
            node_id: rediscovered[0].node_id,
            expected_source_digest: rediscovered[0].source_digest.clone(),
            keep_positive: false,
            replace_crop_node_id: Some(rediscovered[0].plane_crops[0].node_id),
            ..second_request
        };
        let edited =
            apply_plane_crop_to_source(&second.source, folder, &edited_request).expect("edit");
        assert_eq!(edited.source.matches("(clip-plane").count(), 2);
        let rediscovered =
            discover_bound_external_shapes(&edited.source, folder).expect("rediscover edit");
        assert!(rediscovered[0]
            .plane_crops
            .iter()
            .all(|crop| !crop.keep_positive));

        let remove_request = RemoveExternalShapePlaneCropRequest {
            thread_id: "rocksteady-thread".to_string(),
            node_id: rediscovered[0].node_id,
            crop_node_id: rediscovered[0].plane_crops[0].node_id,
            expected_source_digest: rediscovered[0].source_digest.clone(),
        };
        let removed =
            remove_plane_crop_from_source(&edited.source, folder, &remove_request).expect("remove");
        assert_eq!(removed.source.matches("(clip-plane").count(), 1);
        assert_eq!(
            discover_bound_external_shapes(&removed.source, folder).expect("rediscover removal")[0]
                .plane_crops
                .len(),
            1
        );
    }

    #[test]
    fn rejects_collinear_plane_points_without_changing_source() {
        let fixture =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/vertex-genie-ecky.stl");
        let folder = fixture.parent().expect("fixture folder");
        let source = format!(
            "(model (part head_only (solidify (import-stl {:?}))))",
            fixture.to_string_lossy()
        );
        let discovered = discover_bound_external_shapes(&source, folder).expect("discover");
        let mesh = IndexedMeshAsset::from_stl(MeshAssetSource::Imported, &fixture).expect("mesh");
        let triangle = mesh.triangles()[0];
        let position = mesh.vertices()[triangle[0] as usize];
        let b = mesh.vertices()[triangle[1] as usize];
        let c = mesh.vertices()[triangle[2] as usize];
        let raw_normal = cross(subtract(b, position), subtract(c, position));
        let magnitude = length(raw_normal);
        let anchor = CaptureSurfaceAnchor {
            source_mesh_content_digest: mesh.content_digest().to_string(),
            triangle_index: 0,
            barycentric: [1.0, 0.0, 0.0],
            source_position: position,
            source_normal: raw_normal.map(|value| value / magnitude),
        };
        let request = ApplyExternalShapePlaneCropRequest {
            thread_id: "rocksteady-thread".to_string(),
            node_id: discovered[0].node_id,
            expected_source_digest: discovered[0].source_digest.clone(),
            expected_mesh_content_digest: mesh.content_digest().to_string(),
            anchors: vec![anchor.clone(), anchor.clone(), anchor],
            keep_positive: true,
            replace_crop_node_id: None,
        };

        let error =
            apply_plane_crop_to_source(&source, folder, &request).expect_err("collinear points");
        assert_eq!(
            error.message,
            "Plane crop points are duplicate or collinear."
        );
        assert_eq!(source.matches("(clip-plane").count(), 0);
    }

    #[test]
    fn new_plane_crop_wraps_existing_surface_trim_instead_of_changing_its_raw_source_child() {
        let fixture =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/vertex-genie-ecky.stl");
        let folder = fixture.parent().expect("fixture folder");
        let mesh = IndexedMeshAsset::from_stl(MeshAssetSource::Imported, &fixture).expect("mesh");
        let source = format!(
            "(model (part head_only (surface-trim (import-stl {:?}) :schema-version 1 :source-digest {:?} :loop ((mesh-anchor 0 1 0 0) (mesh-anchor 1 1 0 0) (mesh-anchor 2 1 0 0)) :keep-seed (mesh-anchor 3 1 0 0) :path-mode \"shortest\" :cap \"flat\")))",
            fixture.to_string_lossy(),
            mesh.content_digest(),
        );
        let discovered = discover_bound_external_shapes(&source, folder).expect("discover");
        let triangle = mesh.triangles()[0];
        let positions = triangle.map(|index| mesh.vertices()[index as usize]);
        let raw_normal = cross(
            subtract(positions[1], positions[0]),
            subtract(positions[2], positions[0]),
        );
        let magnitude = length(raw_normal);
        let normal = raw_normal.map(|value| value / magnitude);
        let anchors = positions
            .into_iter()
            .zip([[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]])
            .map(|(source_position, barycentric)| CaptureSurfaceAnchor {
                source_mesh_content_digest: mesh.content_digest().to_string(),
                triangle_index: 0,
                barycentric,
                source_position,
                source_normal: normal,
            })
            .collect::<Vec<_>>();
        let request = ApplyExternalShapePlaneCropRequest {
            thread_id: "surface-trim-thread".to_string(),
            node_id: discovered[0].node_id,
            expected_source_digest: discovered[0].source_digest.clone(),
            expected_mesh_content_digest: mesh.content_digest().to_string(),
            anchors,
            keep_positive: true,
            replace_crop_node_id: None,
        };

        let applied =
            apply_plane_crop_to_source(&source, folder, &request).expect("plane outside trim");

        assert!(applied.source.contains("(clip-plane\n  (surface-trim"));
        assert!(applied.source.contains(&format!(
            "(surface-trim (import-stl {:?})",
            fixture.to_string_lossy()
        )));
        let rediscovered =
            discover_bound_external_shapes(&applied.source, folder).expect("rediscover");
        assert_eq!(rediscovered[0].surface_trims.len(), 1);
        assert_eq!(rediscovered[0].plane_crops.len(), 1);
    }
}
