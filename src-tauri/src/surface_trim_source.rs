use crate::contracts::{AppError, AppResult, CaptureSurfaceAnchor};
use crate::ecky_core_ir::{CoreNode, CoreNodeKind, CoreOperation};
use crate::mcp::contracts::EckyAstEditOperation;
use crate::surface_trim_cap::SurfaceTrimCapMode;
use crate::surface_trim_external_shapes::{preview_surface_trim_region, SurfaceTrimPathMode};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Deserialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ApplySurfaceTrimRequest {
    pub schema_version: u32,
    pub thread_id: String,
    #[serde(default)]
    pub target_message_id: Option<String>,
    pub node_id: u64,
    pub expected_source_digest: String,
    pub expected_mesh_content_digest: String,
    pub loop_anchors: Vec<CaptureSurfaceAnchor>,
    pub keep_seed: CaptureSurfaceAnchor,
    pub path_mode: SurfaceTrimPathMode,
    pub cap_mode: SurfaceTrimCapMode,
    #[serde(default)]
    pub replace_trim_node_id: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ApplySurfaceTrimResult {
    pub source: String,
    pub source_digest: String,
    pub trim_node_id: u64,
    pub point_count: u64,
    pub path_mode: SurfaceTrimPathMode,
    pub cap_mode: SurfaceTrimCapMode,
    pub topology: crate::surface_trim_diagnostics::SurfaceTrimMeshDiagnostics,
    pub cap_reports: Vec<crate::surface_trim_cap::SurfaceTrimCapReport>,
}

#[derive(Clone, Debug, Deserialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemoveSurfaceTrimRequest {
    pub thread_id: String,
    #[serde(default)]
    pub target_message_id: Option<String>,
    pub node_id: u64,
    pub trim_node_id: u64,
    pub expected_source_digest: String,
}

#[derive(Clone, Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RemoveSurfaceTrimResult {
    pub source: String,
    pub source_digest: String,
    pub removed_trim_node_id: u64,
}

pub fn resolve_surface_trim_source_path(
    source: &str,
    source_folder: &Path,
    node_id: u64,
    expected_source_digest: &str,
    expected_mesh_content_digest: &str,
) -> AppResult<PathBuf> {
    let actual_source_digest = crate::services::render_snapshot::canonical_source_digest(source);
    if expected_source_digest != actual_source_digest {
        return Err(AppError::conflict(format!(
            "Bound model.ecky changed before surface trim preview: expected '{}', got '{}'.",
            expected_source_digest, actual_source_digest
        )));
    }
    let selected = crate::external_shapes::discover_bound_external_shapes(source, source_folder)?
        .into_iter()
        .find(|item| item.node_id == node_id)
        .ok_or_else(|| {
            AppError::conflict(format!(
                "Imported STL node {} no longer exists in bound model.ecky.",
                node_id
            ))
        })?;
    if !selected.exists {
        return Err(AppError::not_found(format!(
            "Bound external shape is missing '{}'.",
            selected.path
        )));
    }
    if selected.content_digest.as_deref() != Some(expected_mesh_content_digest) {
        return Err(AppError::conflict(format!(
            "Bound external shape content changed before surface trim preview: expected '{}', got '{}'.",
            expected_mesh_content_digest,
            selected.content_digest.as_deref().unwrap_or("missing")
        )));
    }
    Ok(PathBuf::from(selected.path))
}

pub fn apply_surface_trim_to_source(
    source: &str,
    source_folder: &Path,
    request: &ApplySurfaceTrimRequest,
) -> AppResult<ApplySurfaceTrimResult> {
    crate::surface_trim_external_shapes::require_surface_trim_schema_version(
        request.schema_version,
    )?;
    let actual_source_digest = crate::services::render_snapshot::canonical_source_digest(source);
    if request.expected_source_digest != actual_source_digest {
        return Err(AppError::conflict(format!(
            "Bound model.ecky changed before surface trim: expected '{}', got '{}'.",
            request.expected_source_digest, actual_source_digest
        )));
    }

    let sources = crate::external_shapes::discover_bound_external_shapes(source, source_folder)?;
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
            "Bound external shape content changed before surface trim.",
        ));
    }
    if request.replace_trim_node_id.is_none() && !selected.surface_trims.is_empty() {
        return Err(AppError::conflict(format!(
            "Imported STL node {} already has a canonical surface trim. Edit or remove that trim before tracing a replacement.",
            request.node_id
        )));
    }

    let source_path = Path::new(&selected.path);
    preview_surface_trim_region(
        source_path,
        &request.loop_anchors,
        &request.keep_seed,
        request.path_mode,
    )?;
    let canonical_loop = request
        .loop_anchors
        .iter()
        .map(
            |anchor| crate::surface_trim_runtime::CanonicalSurfaceTrimAnchor {
                triangle_index: anchor.triangle_index,
                barycentric: anchor.barycentric,
            },
        )
        .collect::<Vec<_>>();
    let canonical_seed = crate::surface_trim_runtime::CanonicalSurfaceTrimAnchor {
        triangle_index: request.keep_seed.triangle_index,
        barycentric: request.keep_seed.barycentric,
    };
    let runtime_output = crate::surface_trim_runtime::execute_surface_trim(
        source_path,
        &request.expected_mesh_content_digest,
        &canonical_loop,
        &canonical_seed,
        request.path_mode,
        request.cap_mode,
    )?;

    let program = compile_source(source, "apply surface trim")?;
    let (wrap_path, wrap_node, child_path) =
        if let Some(trim_node_id) = request.replace_trim_node_id {
            let (path, node) = find_program_node(&program, trim_node_id).ok_or_else(|| {
                AppError::conflict(format!(
                    "Surface trim node {} no longer exists in bound model.ecky.",
                    trim_node_id
                ))
            })?;
            require_surface_trim_for_import(node, request.node_id)?;
            let child = first_call_arg(node, "Selected surface trim has no source child.")?;
            let _ = child;
            let child_path = format!("{path}/call/args/0");
            (path, node, child_path)
        } else {
            let (path, node) = find_program_node(&program, request.node_id).ok_or_else(|| {
                AppError::conflict(format!(
                    "Imported STL node {} has no editable AST path.",
                    request.node_id
                ))
            })?;
            (path.clone(), node, path)
        };

    let (start, end) = crate::mcp::handlers::source_span_for_ecky_path(source, &child_path)?;
    let child_source = source.get(start..end).ok_or_else(|| {
        AppError::internal("Surface trim child AST span is outside bound model.ecky bytes.")
    })?;
    let replacement = format_surface_trim(child_source, request);
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
    let trim_node_id =
        crate::external_shapes::discover_bound_external_shapes(&next_source, source_folder)?
            .into_iter()
            .find(|source| source.path == selected.path)
            .and_then(|source| {
                source.surface_trims.into_iter().find(|trim| {
                    trim.source_digest == request.expected_mesh_content_digest
                        && trim.loop_anchors.len() == request.loop_anchors.len()
                        && trim.path_mode == request.path_mode
                        && trim.cap_mode == request.cap_mode
                })
            })
            .map(|trim| trim.node_id)
            .ok_or_else(|| {
                AppError::internal(
                    "Applied surface trim could not be resolved from canonical source.",
                )
            })?;

    Ok(ApplySurfaceTrimResult {
        source: next_source,
        source_digest,
        trim_node_id,
        point_count: request.loop_anchors.len() as u64,
        path_mode: request.path_mode,
        cap_mode: request.cap_mode,
        topology: runtime_output.diagnostics,
        cap_reports: runtime_output.cap_reports,
    })
}

pub fn remove_surface_trim_from_source(
    source: &str,
    request: &RemoveSurfaceTrimRequest,
) -> AppResult<RemoveSurfaceTrimResult> {
    let actual_source_digest = crate::services::render_snapshot::canonical_source_digest(source);
    if request.expected_source_digest != actual_source_digest {
        return Err(AppError::conflict(format!(
            "Bound model.ecky changed before surface trim removal: expected '{}', got '{}'.",
            request.expected_source_digest, actual_source_digest
        )));
    }
    let program = compile_source(source, "remove surface trim")?;
    let (trim_path, trim_node) =
        find_program_node(&program, request.trim_node_id).ok_or_else(|| {
            AppError::conflict(format!(
                "Surface trim node {} no longer exists in bound model.ecky.",
                request.trim_node_id
            ))
        })?;
    require_surface_trim_for_import(trim_node, request.node_id)?;
    first_call_arg(trim_node, "Selected surface trim has no source child.")?;
    let child_path = format!("{trim_path}/call/args/0");
    let (start, end) = crate::mcp::handlers::source_span_for_ecky_path(source, &child_path)?;
    let child_source = source.get(start..end).ok_or_else(|| {
        AppError::internal("Surface trim child AST span is outside bound model.ecky bytes.")
    })?;
    let next_source = crate::mcp::handlers::replace_ecky_ast_source(
        source,
        &actual_source_digest,
        &trim_path,
        &crate::mcp::handlers::core_node_digest(trim_node),
        &EckyAstEditOperation::Replace,
        Some(child_source),
        None,
    )?;
    let source_digest = crate::services::render_snapshot::canonical_source_digest(&next_source);
    Ok(RemoveSurfaceTrimResult {
        source: next_source,
        source_digest,
        removed_trim_node_id: request.trim_node_id,
    })
}

fn compile_source(source: &str, action: &str) -> AppResult<crate::ecky_core_ir::CoreProgram> {
    crate::ecky_scheme::compile_to_core_program(source).map_err(|error| {
        AppError::validation(format!(
            "Cannot {action} because bound model.ecky is invalid: {error}"
        ))
    })
}

fn find_program_node(
    program: &crate::ecky_core_ir::CoreProgram,
    node_id: u64,
) -> Option<(String, &CoreNode)> {
    program.parts.iter().find_map(|part| {
        let root_path = format!(
            "/parts/{}/root",
            crate::external_shapes::ast_path_segment(&part.key)
        );
        crate::external_shapes::find_ast_target(&part.root, &root_path, node_id, None, false)
    })
}

fn require_surface_trim_for_import(node: &CoreNode, import_node_id: u64) -> AppResult<()> {
    if !matches!(
        &node.kind,
        CoreNodeKind::Call {
            op: CoreOperation::Custom(name),
            ..
        } if name == "surface-trim"
    ) {
        return Err(AppError::conflict(
            "Selected surface trim node is no longer a surface-trim call.",
        ));
    }
    if !contains_node_id(node, import_node_id) {
        return Err(AppError::conflict(format!(
            "Selected surface trim no longer belongs to imported STL node {}.",
            import_node_id
        )));
    }
    Ok(())
}

fn first_call_arg<'a>(node: &'a CoreNode, message: &str) -> AppResult<&'a CoreNode> {
    let CoreNodeKind::Call { args, .. } = &node.kind else {
        return Err(AppError::conflict(message));
    };
    args.first().ok_or_else(|| AppError::conflict(message))
}

fn format_surface_trim(child_source: &str, request: &ApplySurfaceTrimRequest) -> String {
    let anchors = request
        .loop_anchors
        .iter()
        .map(format_mesh_anchor)
        .collect::<Vec<_>>()
        .join("\n      ");
    let path_mode = match request.path_mode {
        SurfaceTrimPathMode::Shortest => "shortest",
        SurfaceTrimPathMode::Feature => "feature",
    };
    let cap_mode = match request.cap_mode {
        SurfaceTrimCapMode::Open => "open",
        SurfaceTrimCapMode::Flat => "flat",
        SurfaceTrimCapMode::SurfaceFill => "surface-fill",
    };
    format!(
        "(surface-trim\n  {child_source}\n  :schema-version {}\n  :source-digest \"{}\"\n  :loop\n    ({})\n  :keep-seed {}\n  :path-mode \"{path_mode}\"\n  :cap \"{cap_mode}\")",
        request.schema_version,
        escape_text(&request.expected_mesh_content_digest),
        anchors,
        format_mesh_anchor(&request.keep_seed),
    )
}

fn format_mesh_anchor(anchor: &CaptureSurfaceAnchor) -> String {
    format!(
        "(mesh-anchor {} {} {} {})",
        anchor.triangle_index,
        format_scalar(anchor.barycentric[0]),
        format_scalar(anchor.barycentric[1]),
        format_scalar(anchor.barycentric[2]),
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

fn escape_text(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecky_ir::mesh_asset::{IndexedMeshAsset, MeshAssetSource};
    use crate::external_shapes::discover_bound_external_shapes;
    use crate::services::render_snapshot::canonical_source_digest;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    #[derive(Clone)]
    struct SurfaceTrimFixture {
        source_folder: PathBuf,
        source_path: PathBuf,
        source: String,
        source_digest: String,
        node_id: u64,
        mesh_digest: String,
        loop_anchors: Vec<CaptureSurfaceAnchor>,
        keep_seed: CaptureSurfaceAnchor,
    }

    fn cube_vertices() -> [[f64; 3]; 8] {
        [
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
            [1.0, 0.0, 1.0],
            [1.0, 1.0, 1.0],
            [0.0, 1.0, 1.0],
        ]
    }

    fn cube_triangles() -> [([usize; 3], [f64; 3]); 12] {
        [
            ([0, 2, 1], [0.0, 0.0, -1.0]),
            ([0, 3, 2], [0.0, 0.0, -1.0]),
            ([4, 5, 6], [0.0, 0.0, 1.0]),
            ([4, 6, 7], [0.0, 0.0, 1.0]),
            ([0, 1, 5], [0.0, -1.0, 0.0]),
            ([0, 5, 4], [0.0, -1.0, 0.0]),
            ([1, 2, 6], [1.0, 0.0, 0.0]),
            ([1, 6, 5], [1.0, 0.0, 0.0]),
            ([2, 3, 7], [0.0, 1.0, 0.0]),
            ([2, 7, 6], [0.0, 1.0, 0.0]),
            ([3, 0, 4], [-1.0, 0.0, 0.0]),
            ([3, 4, 7], [-1.0, 0.0, 0.0]),
        ]
    }

    fn write_cube_stl_fixture(folder: &Path) -> PathBuf {
        let stl_path = folder.join("cube.stl");
        let stl = r#"solid cube
  facet normal 0 0 -1
    outer loop
      vertex 0 0 0
      vertex 1 1 0
      vertex 1 0 0
    endloop
  endfacet
  facet normal 0 0 -1
    outer loop
      vertex 0 0 0
      vertex 0 1 0
      vertex 1 1 0
    endloop
  endfacet
  facet normal 0 0 1
    outer loop
      vertex 0 0 1
      vertex 1 0 1
      vertex 1 1 1
    endloop
  endfacet
  facet normal 0 0 1
    outer loop
      vertex 0 0 1
      vertex 1 1 1
      vertex 0 1 1
    endloop
  endfacet
  facet normal 0 -1 0
    outer loop
      vertex 0 0 0
      vertex 1 0 0
      vertex 1 0 1
    endloop
  endfacet
  facet normal 0 -1 0
    outer loop
      vertex 0 0 0
      vertex 1 0 1
      vertex 0 0 1
    endloop
  endfacet
  facet normal 1 0 0
    outer loop
      vertex 1 0 0
      vertex 1 1 0
      vertex 1 1 1
    endloop
  endfacet
  facet normal 1 0 0
    outer loop
      vertex 1 0 0
      vertex 1 1 1
      vertex 1 0 1
    endloop
  endfacet
  facet normal 0 1 0
    outer loop
      vertex 1 1 0
      vertex 0 1 0
      vertex 0 1 1
    endloop
  endfacet
  facet normal 0 1 0
    outer loop
      vertex 1 1 0
      vertex 0 1 1
      vertex 1 1 1
    endloop
  endfacet
  facet normal -1 0 0
    outer loop
      vertex 0 1 0
      vertex 0 0 0
      vertex 0 0 1
    endloop
  endfacet
  facet normal -1 0 0
    outer loop
      vertex 0 1 0
      vertex 0 0 1
      vertex 0 1 1
    endloop
  endfacet
endsolid cube
"#;
        fs::write(&stl_path, stl).expect("write cube stl");
        let _asset = IndexedMeshAsset::from_stl(MeshAssetSource::Imported, &stl_path)
            .expect("parse cube stl");
        stl_path
    }

    fn materialize_anchor(
        source_mesh_content_digest: &str,
        triangle_index: usize,
        barycentric: [f64; 3],
    ) -> CaptureSurfaceAnchor {
        let vertices = cube_vertices();
        let (triangle, normal) = cube_triangles()[triangle_index];
        let source_position =
            triangle
                .iter()
                .enumerate()
                .fold([0.0, 0.0, 0.0], |mut acc, (index, vertex_index)| {
                    let weight = barycentric[index];
                    let vertex = vertices[*vertex_index];
                    acc[0] += vertex[0] * weight;
                    acc[1] += vertex[1] * weight;
                    acc[2] += vertex[2] * weight;
                    acc
                });

        CaptureSurfaceAnchor {
            source_mesh_content_digest: source_mesh_content_digest.to_string(),
            triangle_index: triangle_index.try_into().expect("triangle index"),
            barycentric,
            source_position,
            source_normal: normal,
        }
    }

    fn build_fixture() -> SurfaceTrimFixture {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let source_folder = std::env::temp_dir().join(format!(
            "surface-trim-source-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&source_folder).expect("source folder");
        let source_path = write_cube_stl_fixture(&source_folder);
        let source = format!(
            "(model\n  (part scanned\n    (translate 3 4 5\n      (import-stl \"{}\")))\n  (part unrelated\n    (translate 9 8 7 (box 2 2 2))))",
            source_path.display()
        );
        let source_digest = canonical_source_digest(&source);
        let discovered = discover_bound_external_shapes(&source, &source_folder).expect("discover");
        let selected = discovered
            .into_iter()
            .find(|shape| shape.path == source_path.to_string_lossy().as_ref())
            .expect("selected external shape");
        let mesh_digest = selected
            .content_digest
            .clone()
            .expect("selected mesh digest");
        let loop_anchors = vec![
            materialize_anchor(&mesh_digest, 4, [0.25, 0.25, 0.5]),
            materialize_anchor(&mesh_digest, 7, [0.5, 0.25, 0.25]),
            materialize_anchor(&mesh_digest, 6, [0.25, 0.25, 0.5]),
            materialize_anchor(&mesh_digest, 9, [0.5, 0.25, 0.25]),
            materialize_anchor(&mesh_digest, 8, [0.25, 0.25, 0.5]),
            materialize_anchor(&mesh_digest, 11, [0.5, 0.25, 0.25]),
            materialize_anchor(&mesh_digest, 10, [0.25, 0.25, 0.5]),
            materialize_anchor(&mesh_digest, 5, [0.5, 0.25, 0.25]),
        ];
        let keep_seed = materialize_anchor(&mesh_digest, 2, [1.0 / 3.0; 3]);
        SurfaceTrimFixture {
            source_folder,
            source_path,
            source,
            source_digest,
            node_id: selected.node_id,
            mesh_digest,
            loop_anchors,
            keep_seed,
        }
    }

    fn apply_request(
        fixture: &SurfaceTrimFixture,
        replace_trim_node_id: Option<u64>,
        path_mode: SurfaceTrimPathMode,
        cap_mode: SurfaceTrimCapMode,
    ) -> ApplySurfaceTrimRequest {
        ApplySurfaceTrimRequest {
            schema_version: 1,
            thread_id: "thread-1".to_string(),
            target_message_id: None,
            node_id: fixture.node_id,
            expected_source_digest: fixture.source_digest.clone(),
            expected_mesh_content_digest: fixture.mesh_digest.clone(),
            loop_anchors: fixture.loop_anchors.clone(),
            keep_seed: fixture.keep_seed.clone(),
            path_mode,
            cap_mode,
            replace_trim_node_id,
        }
    }

    fn remove_request(
        fixture: &SurfaceTrimFixture,
        trim_node_id: u64,
        source_digest: String,
    ) -> RemoveSurfaceTrimRequest {
        RemoveSurfaceTrimRequest {
            thread_id: "thread-1".to_string(),
            target_message_id: None,
            node_id: fixture.node_id,
            trim_node_id,
            expected_source_digest: source_digest,
        }
    }

    #[test]
    fn apply_wraps_selected_import_and_preserves_surrounding_and_unrelated_source() {
        let fixture = build_fixture();
        let result = apply_surface_trim_to_source(
            &fixture.source,
            &fixture.source_folder,
            &apply_request(
                &fixture,
                None,
                SurfaceTrimPathMode::Feature,
                SurfaceTrimCapMode::Flat,
            ),
        )
        .expect("apply surface trim");

        assert_eq!(result.point_count, 8);
        assert_eq!(result.path_mode, SurfaceTrimPathMode::Feature);
        assert_eq!(result.cap_mode, SurfaceTrimCapMode::Flat);
        assert!(result.source.contains("(translate 3 4 5"));
        assert!(result.source.contains("(part unrelated"));
        assert_eq!(result.source.matches("(surface-trim").count(), 1);

        let discovered = discover_bound_external_shapes(&result.source, &fixture.source_folder)
            .expect("discover applied source");
        let selected = discovered
            .into_iter()
            .find(|shape| shape.path == fixture.source_path.to_string_lossy().as_ref())
            .expect("selected shape");
        assert!(selected
            .surface_trims
            .iter()
            .any(|trim| trim.node_id == result.trim_node_id));
    }

    #[test]
    fn edit_replaces_selected_trim_without_nesting_and_preserves_source_shape() {
        let fixture = build_fixture();
        let applied = apply_surface_trim_to_source(
            &fixture.source,
            &fixture.source_folder,
            &apply_request(
                &fixture,
                None,
                SurfaceTrimPathMode::Shortest,
                SurfaceTrimCapMode::Flat,
            ),
        )
        .expect("initial apply");
        let applied_shape = discover_bound_external_shapes(&applied.source, &fixture.source_folder)
            .expect("discover applied source")
            .into_iter()
            .find(|shape| shape.path == fixture.source_path.to_string_lossy().as_ref())
            .expect("applied shape");

        let edited = apply_surface_trim_to_source(
            &applied.source,
            &fixture.source_folder,
            &ApplySurfaceTrimRequest {
                expected_source_digest: applied.source_digest.clone(),
                node_id: applied_shape.node_id,
                replace_trim_node_id: Some(applied.trim_node_id),
                path_mode: SurfaceTrimPathMode::Feature,
                ..apply_request(
                    &fixture,
                    Some(applied.trim_node_id),
                    SurfaceTrimPathMode::Feature,
                    SurfaceTrimCapMode::Flat,
                )
            },
        )
        .expect("edit trim");

        assert!(edited.source.contains("(translate 3 4 5"));
        assert!(edited.source.contains("(part unrelated"));
        assert_eq!(edited.source.matches("(surface-trim").count(), 1);
        assert!(edited.source.contains(":path-mode \"feature\""));
    }

    #[test]
    fn remove_unwraps_trim_and_preserves_surrounding_and_unrelated_source() {
        let fixture = build_fixture();
        let applied = apply_surface_trim_to_source(
            &fixture.source,
            &fixture.source_folder,
            &apply_request(
                &fixture,
                None,
                SurfaceTrimPathMode::Shortest,
                SurfaceTrimCapMode::Flat,
            ),
        )
        .expect("apply trim");
        let applied_shape = discover_bound_external_shapes(&applied.source, &fixture.source_folder)
            .expect("discover applied source")
            .into_iter()
            .find(|shape| shape.path == fixture.source_path.to_string_lossy().as_ref())
            .expect("applied shape");

        let removed = remove_surface_trim_from_source(
            &applied.source,
            &RemoveSurfaceTrimRequest {
                node_id: applied_shape.node_id,
                ..remove_request(
                    &fixture,
                    applied.trim_node_id,
                    applied.source_digest.clone(),
                )
            },
        )
        .expect("remove trim");

        assert_eq!(removed.source, fixture.source);
        assert_eq!(removed.removed_trim_node_id, applied.trim_node_id);
    }

    #[test]
    fn stale_expected_source_digest_fails_before_output_and_leaves_files_unchanged() {
        let fixture = build_fixture();
        let source_before = fixture.source.clone();
        let original_bytes = fs::read(&fixture.source_path).expect("read source file");
        let error = apply_surface_trim_to_source(
            &fixture.source,
            &fixture.source_folder,
            &ApplySurfaceTrimRequest {
                expected_source_digest: "sha256:stale".to_string(),
                ..apply_request(
                    &fixture,
                    None,
                    SurfaceTrimPathMode::Shortest,
                    SurfaceTrimCapMode::Flat,
                )
            },
        )
        .expect_err("stale digest should fail");

        assert!(error
            .to_string()
            .contains("Bound model.ecky changed before surface trim"));
        assert_eq!(fixture.source, source_before);
        assert_eq!(
            fs::read(&fixture.source_path).expect("read source file again"),
            original_bytes
        );
    }

    #[test]
    fn second_schema_v1_trim_on_same_import_rejects_with_edit_remove_conflict() {
        let fixture = build_fixture();
        let applied = apply_surface_trim_to_source(
            &fixture.source,
            &fixture.source_folder,
            &apply_request(
                &fixture,
                None,
                SurfaceTrimPathMode::Shortest,
                SurfaceTrimCapMode::Flat,
            ),
        )
        .expect("first trim");
        let applied_shape = discover_bound_external_shapes(&applied.source, &fixture.source_folder)
            .expect("discover applied source")
            .into_iter()
            .find(|shape| shape.path == fixture.source_path.to_string_lossy().as_ref())
            .expect("applied shape");

        let error = apply_surface_trim_to_source(
            &applied.source,
            &fixture.source_folder,
            &ApplySurfaceTrimRequest {
                node_id: applied_shape.node_id,
                expected_source_digest: applied.source_digest.clone(),
                ..apply_request(
                    &fixture,
                    None,
                    SurfaceTrimPathMode::Shortest,
                    SurfaceTrimCapMode::Flat,
                )
            },
        )
        .expect_err("second trim must conflict");

        assert_eq!(
            error.to_string(),
            format!(
                "Imported STL node {} already has a canonical surface trim. Edit or remove that trim before tracing a replacement.",
                applied_shape.node_id
            )
        );
    }
}
