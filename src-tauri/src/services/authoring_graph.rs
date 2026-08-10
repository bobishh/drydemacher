use crate::contracts::{
    AppError, AppResult, ArtifactBundle, AuthoringGraph, AuthoringGraphAstNode,
    AuthoringGraphConstraint, AuthoringGraphDependency, AuthoringGraphFeature,
    AuthoringGraphTarget, FeatureNode, ModelManifest, SelectionTarget,
};
use crate::ecky_core_ir::{
    CoreNode, CoreNodeKind, CoreProgram, CoreReference, CoreRelationConstraint, CoreRelationOperand,
};
use std::collections::{BTreeSet, HashMap};

pub(crate) fn path_segment(value: &str) -> String {
    value.replace('~', "~0").replace('/', "~1")
}

fn path_segment_decode(value: &str) -> String {
    value.replace("~1", "/").replace("~0", "~")
}

pub(crate) fn core_node_child_paths<'a>(
    node: &'a CoreNode,
    path: &str,
) -> Vec<(String, &'a CoreNode)> {
    match &node.kind {
        CoreNodeKind::Literal(_) | CoreNodeKind::Reference(_) => Vec::new(),
        CoreNodeKind::Build { bindings, result } => bindings
            .iter()
            .map(|binding| {
                (
                    format!("{}/build/bindings/{}", path, path_segment(&binding.name)),
                    &binding.value,
                )
            })
            .chain(std::iter::once((
                format!("{path}/build/result"),
                result.as_ref(),
            )))
            .collect(),
        CoreNodeKind::Let { bindings, body } => bindings
            .iter()
            .map(|binding| {
                (
                    format!("{}/let/bindings/{}", path, path_segment(&binding.name)),
                    &binding.value,
                )
            })
            .chain(std::iter::once((format!("{path}/let/body"), body.as_ref())))
            .collect(),
        CoreNodeKind::If {
            condition,
            then_branch,
            else_branch,
        } => vec![
            (format!("{path}/if/condition"), condition.as_ref()),
            (format!("{path}/if/then"), then_branch.as_ref()),
            (format!("{path}/if/else"), else_branch.as_ref()),
        ],
        CoreNodeKind::Call { args, keywords, .. } => args
            .iter()
            .enumerate()
            .map(|(index, arg)| (format!("{path}/call/args/{index}"), arg))
            .chain(keywords.iter().map(|keyword| {
                (
                    format!("{}/call/keywords/{}", path, path_segment(&keyword.name)),
                    keyword.source_node(),
                )
            }))
            .collect(),
        CoreNodeKind::Range { start, end } => vec![
            (format!("{path}/range/start"), start.as_ref()),
            (format!("{path}/range/end"), end.as_ref()),
        ],
        CoreNodeKind::Map { sources, body, .. } => sources
            .iter()
            .enumerate()
            .map(|(index, source)| (format!("{path}/map/sources/{index}"), source))
            .chain(std::iter::once((format!("{path}/map/body"), body.as_ref())))
            .collect(),
        CoreNodeKind::Apply { args, list, .. } => args
            .iter()
            .enumerate()
            .map(|(index, arg)| (format!("{path}/apply/args/{index}"), arg))
            .chain(std::iter::once((
                format!("{path}/apply/list"),
                list.as_ref(),
            )))
            .collect(),
        CoreNodeKind::List(items) => items
            .iter()
            .enumerate()
            .map(|(index, item)| (format!("{path}/list/{index}"), item))
            .collect(),
        CoreNodeKind::Group(items) => items
            .iter()
            .enumerate()
            .map(|(index, item)| (format!("{path}/group/{index}"), item))
            .collect(),
    }
}

fn node_kind(node: &CoreNode) -> &'static str {
    match &node.kind {
        CoreNodeKind::Literal(_) => "Literal",
        CoreNodeKind::Reference(_) => "Reference",
        CoreNodeKind::Build { .. } => "Build",
        CoreNodeKind::Let { .. } => "Let",
        CoreNodeKind::If { .. } => "If",
        CoreNodeKind::Call { .. } => "Call",
        CoreNodeKind::Range { .. } => "Range",
        CoreNodeKind::Map { .. } => "Map",
        CoreNodeKind::Apply { .. } => "Apply",
        CoreNodeKind::List(_) => "List",
        CoreNodeKind::Group(_) => "Group",
    }
}

fn node_operation(node: &CoreNode) -> Option<String> {
    match &node.kind {
        CoreNodeKind::Call { op, .. } | CoreNodeKind::Apply { op, .. } => Some(format!("{op:?}")),
        _ => None,
    }
}

fn find_node<'a>(node: &'a CoreNode, path: &str, requested: &str) -> Option<&'a CoreNode> {
    if path == requested {
        return Some(node);
    }
    core_node_child_paths(node, path)
        .into_iter()
        .find_map(|(child_path, child)| {
            requested
                .starts_with(&child_path)
                .then(|| find_node(child, &child_path, requested))
                .flatten()
        })
}

fn find_program_node<'a>(program: &'a CoreProgram, requested: &str) -> Option<&'a CoreNode> {
    program.parts.iter().find_map(|part| {
        let root_path = format!("/parts/{}/root", path_segment(&part.key));
        requested
            .starts_with(&root_path)
            .then(|| find_node(&part.root, &root_path, requested))
            .flatten()
    })
}

fn stable_key(path: &str, kind: &str, value_kind: &str, operation: Option<&str>) -> String {
    let mut identity = vec![
        format!("path={path}"),
        format!("kind={kind}"),
        format!("valueKind={value_kind}"),
    ];
    if let Some(operation) = operation {
        identity.push(format!("op={operation}"));
    }
    let segments = path
        .split('/')
        .filter(|segment| !segment.is_empty())
        .map(path_segment_decode)
        .collect::<Vec<_>>();
    if segments.len() == 2 && matches!(segments[0].as_str(), "params" | "parts" | "analyses") {
        identity.push(format!("binding={}", segments[1]));
    } else if let Some(binding) = segments.windows(2).find_map(|pair| {
        matches!(pair[0].as_str(), "bindings" | "keywords").then(|| pair[1].clone())
    }) {
        identity.push(format!("binding={binding}"));
    }
    crate::services::render_snapshot::canonical_source_digest(&identity.join("|"))
}

pub(crate) fn stable_node_key_from_parts(
    _source: &str,
    path: &str,
    kind: &str,
    value_kind: &str,
    operation: Option<&str>,
) -> String {
    stable_key(path, kind, value_kind, operation)
}

pub(crate) fn stable_node_key_for_program_path(
    source: &str,
    program: &CoreProgram,
    path: &str,
) -> Option<String> {
    let segments = path
        .split('/')
        .filter(|segment| !segment.is_empty())
        .map(path_segment_decode)
        .collect::<Vec<_>>();
    if segments.len() == 2 && segments[0] == "params" {
        let param = program
            .parameters
            .iter()
            .find(|param| param.key == segments[1])?;
        return Some(stable_key(
            path,
            "Param",
            &format!("{:?}", param.kind),
            None,
        ));
    }
    if segments.len() == 2 && segments[0] == "parts" {
        program.parts.iter().find(|part| part.key == segments[1])?;
        return Some(stable_key(path, "Part", "Part", None));
    }
    if segments.len() == 2 && segments[0] == "analyses" {
        program
            .analyses
            .iter()
            .find(|analysis| analysis.name == segments[1])?;
        return Some(stable_key(path, "Analysis", "Analysis", None));
    }
    let node = find_program_node(program, path)?;
    Some(stable_node_key_from_parts(
        source,
        path,
        node_kind(node),
        &format!("{:?}", node.value_kind),
        node_operation(node).as_deref(),
    ))
}

fn collect_ast_node(
    source: &str,
    node: &CoreNode,
    path: &str,
    part_id: &str,
    output: &mut Vec<AuthoringGraphAstNode>,
) {
    output.push(AuthoringGraphAstNode {
        path: path.to_string(),
        stable_node_key: stable_node_key_from_parts(
            source,
            path,
            node_kind(node),
            &format!("{:?}", node.value_kind),
            node_operation(node).as_deref(),
        ),
        kind: node_kind(node).to_string(),
        value_kind: format!("{:?}", node.value_kind),
        operation: node_operation(node),
        part_id: Some(part_id.to_string()),
    });
    for (child_path, child) in core_node_child_paths(node, path) {
        collect_ast_node(source, child, &child_path, part_id, output);
    }
}

fn collect_reference_paths(
    node: &CoreNode,
    path: &str,
    parameter_id: crate::ecky_core_ir::ParamId,
    output: &mut Vec<String>,
) {
    if matches!(&node.kind, CoreNodeKind::Reference(CoreReference::Parameter(id)) if *id == parameter_id)
    {
        output.push(path.to_string());
    }
    for (child_path, child) in core_node_child_paths(node, path) {
        collect_reference_paths(child, &child_path, parameter_id, output);
    }
}

pub(crate) fn dependent_source_paths_for_param(
    program: &CoreProgram,
    parameter_id: crate::ecky_core_ir::ParamId,
) -> Vec<String> {
    let mut paths = Vec::new();
    for part in &program.parts {
        let root_path = format!("/parts/{}/root", path_segment(&part.key));
        collect_reference_paths(&part.root, &root_path, parameter_id, &mut paths);
    }
    paths
}

pub(crate) fn impacted_part_ids_for_dependency_paths(paths: &[String]) -> Vec<String> {
    let mut ids = BTreeSet::new();
    for path in paths {
        let segments = path
            .split('/')
            .filter(|segment| !segment.is_empty())
            .collect::<Vec<_>>();
        if segments.len() >= 2 && segments[0] == "parts" {
            ids.insert(path_segment_decode(segments[1]));
        }
    }
    ids.into_iter().collect()
}

pub(crate) fn selection_target_ids(target: &SelectionTarget) -> Vec<String> {
    target
        .target_id
        .iter()
        .chain(target.durable_target_id.iter())
        .chain(target.canonical_target_id.iter())
        .chain(target.alias_ids.iter())
        .filter(|id| !id.trim().is_empty())
        .cloned()
        .collect()
}

pub(crate) fn feature_bindings_for_target_ids(
    manifest: &ModelManifest,
    target_ids: &[String],
) -> (Vec<String>, Vec<String>) {
    let Some(graph) = manifest.feature_graph.as_ref() else {
        return (Vec::new(), Vec::new());
    };
    let requested = target_ids
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let mut feature_ids = BTreeSet::new();
    let mut source_paths = BTreeSet::new();
    for node in &graph.nodes {
        let output_match = node
            .output_refs
            .iter()
            .flat_map(|output| &output.target_ids)
            .chain(node.ports.iter().flat_map(|port| &port.target_ids))
            .any(|id| requested.contains(id.as_str()));
        if !output_match {
            continue;
        }
        feature_ids.insert(node.feature_id.clone());
        if let Some(path) = node
            .source_ref
            .as_ref()
            .and_then(|source_ref| source_ref.path.as_ref())
        {
            source_paths.insert(path.clone());
        }
        for port in &node.ports {
            if port
                .target_ids
                .iter()
                .any(|id| requested.contains(id.as_str()))
            {
                if let Some(path) = port
                    .source_ref
                    .as_ref()
                    .and_then(|source_ref| source_ref.path.as_ref())
                {
                    source_paths.insert(path.clone());
                }
            }
        }
    }
    (
        feature_ids.into_iter().collect(),
        source_paths.into_iter().collect(),
    )
}

fn relation_parameter_keys(
    program: &CoreProgram,
    relation: &CoreRelationConstraint,
) -> Vec<String> {
    [&relation.left, &relation.right]
        .into_iter()
        .filter_map(|operand| match operand {
            CoreRelationOperand::Parameter(id) => program
                .parameters
                .iter()
                .find(|param| param.id == *id)
                .map(|param| param.key.clone()),
            CoreRelationOperand::Number(_) => None,
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn relation_operand(program: &CoreProgram, operand: &CoreRelationOperand) -> String {
    match operand {
        CoreRelationOperand::Number(value) => value.to_string(),
        CoreRelationOperand::Parameter(id) => program
            .parameters
            .iter()
            .find(|param| param.id == *id)
            .map(|param| param.key.clone())
            .unwrap_or_else(|| format!("param#{}", id.raw())),
    }
}

fn feature_target_ids(feature: &FeatureNode) -> Vec<String> {
    feature
        .output_refs
        .iter()
        .flat_map(|output| output.target_ids.iter())
        .chain(feature.ports.iter().flat_map(|port| port.target_ids.iter()))
        .cloned()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

pub fn build_authoring_graph(
    source: &str,
    manifest: Option<&ModelManifest>,
    artifact_bundle: Option<&ArtifactBundle>,
) -> AppResult<AuthoringGraph> {
    let program = crate::ecky_scheme::compile_to_core_program(source).map_err(|error| {
        AppError::validation(format!(
            "Failed to compile Ecky source for authoring graph: {error}"
        ))
    })?;
    let mut ast_nodes = Vec::new();
    for parameter in &program.parameters {
        let path = format!("/params/{}", path_segment(&parameter.key));
        ast_nodes.push(AuthoringGraphAstNode {
            stable_node_key: stable_node_key_for_program_path(source, &program, &path)
                .expect("parameter path must resolve"),
            path,
            kind: "Param".to_string(),
            value_kind: format!("{:?}", parameter.kind),
            operation: None,
            part_id: None,
        });
    }
    for part in &program.parts {
        let part_path = format!("/parts/{}", path_segment(&part.key));
        ast_nodes.push(AuthoringGraphAstNode {
            stable_node_key: stable_node_key_for_program_path(source, &program, &part_path)
                .expect("part path must resolve"),
            path: part_path.clone(),
            kind: "Part".to_string(),
            value_kind: "Part".to_string(),
            operation: None,
            part_id: Some(part.key.clone()),
        });
        collect_ast_node(
            source,
            &part.root,
            &format!("{part_path}/root"),
            &part.key,
            &mut ast_nodes,
        );
    }
    let stable_keys = ast_nodes
        .iter()
        .map(|node| (node.path.clone(), node.stable_node_key.clone()))
        .collect::<HashMap<_, _>>();

    let features = manifest
        .and_then(|manifest| manifest.feature_graph.as_ref())
        .map(|graph| {
            graph
                .nodes
                .iter()
                .map(|feature| {
                    let source_path = feature
                        .source_ref
                        .as_ref()
                        .and_then(|source_ref| source_ref.path.clone());
                    AuthoringGraphFeature {
                        feature_id: feature.feature_id.clone(),
                        kind: feature.kind.clone(),
                        label: feature.label.clone(),
                        source_stable_node_key: source_path
                            .as_ref()
                            .and_then(|path| stable_keys.get(path).cloned()),
                        source_path,
                        dependency_ids: feature.dependency_ids.clone(),
                        output_ids: feature
                            .output_refs
                            .iter()
                            .map(|output| output.output_id.clone())
                            .collect(),
                        target_ids: feature_target_ids(feature),
                    }
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let targets = manifest
        .map(|manifest| {
            manifest
                .selection_targets
                .iter()
                .filter_map(|target| {
                    let ids = selection_target_ids(target);
                    let target_id = target
                        .durable_target_id
                        .clone()
                        .or_else(|| target.canonical_target_id.clone())
                        .or_else(|| target.target_id.clone())?;
                    let (feature_ids, source_paths) =
                        feature_bindings_for_target_ids(manifest, &ids);
                    let source_stable_node_keys = source_paths
                        .iter()
                        .filter_map(|path| stable_keys.get(path).cloned())
                        .collect::<Vec<_>>();
                    let non_editable_reason = if !target.editable {
                        Some(format!(
                            "Manifest target '{}' is marked non-editable.",
                            target_id
                        ))
                    } else if feature_ids.is_empty() {
                        Some(format!(
                            "No feature output or port references target IDs: {}.",
                            ids.join(", ")
                        ))
                    } else if source_paths.is_empty() {
                        Some(format!(
                            "Feature provenance has no source path for target IDs: {}.",
                            ids.join(", ")
                        ))
                    } else if source_stable_node_keys.is_empty() {
                        Some(format!(
                            "Feature source paths do not resolve to stable AST nodes for target IDs: {}. Paths: {}.",
                            ids.join(", "),
                            source_paths.join(", ")
                        ))
                    } else {
                        None
                    };
                    Some(AuthoringGraphTarget {
                        target_id,
                        durable_target_id: target.durable_target_id.clone(),
                        canonical_target_id: target.canonical_target_id.clone(),
                        alias_ids: target.alias_ids.clone(),
                        part_id: target.part_id.clone(),
                        viewer_node_id: target.viewer_node_id.clone(),
                        label: target.label.clone(),
                        kind: target.kind.clone(),
                        parameter_keys: target.parameter_keys.clone(),
                        primitive_ids: target.primitive_ids.clone(),
                        feature_ids,
                        source_stable_node_keys,
                        editable: non_editable_reason.is_none(),
                        non_editable_reason,
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let dependencies: Vec<AuthoringGraphDependency> = program
        .parameters
        .iter()
        .map(|parameter| {
            let parameter_path = format!("/params/{}", path_segment(&parameter.key));
            let dependent_source_paths = dependent_source_paths_for_param(&program, parameter.id);
            let impacted_part_ids = impacted_part_ids_for_dependency_paths(&dependent_source_paths);
            let affected_stable_node_keys = dependent_source_paths
                .iter()
                .filter_map(|path| stable_keys.get(path).cloned())
                .collect::<Vec<_>>();
            let target_ids = targets
                .iter()
                .filter(|target| {
                    target
                        .parameter_keys
                        .iter()
                        .any(|key| key == &parameter.key)
                })
                .map(|target| target.target_id.clone())
                .collect::<BTreeSet<_>>();
            let feature_ids = targets
                .iter()
                .filter(|target| target_ids.contains(&target.target_id))
                .flat_map(|target| target.feature_ids.iter().cloned())
                .collect::<BTreeSet<_>>();
            AuthoringGraphDependency {
                parameter_key: parameter.key.clone(),
                parameter_stable_node_key: stable_keys
                    .get(&parameter_path)
                    .cloned()
                    .expect("parameter stable key"),
                dependent_source_paths,
                affected_stable_node_keys,
                impacted_part_ids,
                feature_ids: feature_ids.into_iter().collect(),
                target_ids: target_ids.into_iter().collect(),
            }
        })
        .collect();

    let constraints: Vec<AuthoringGraphConstraint> = program
        .constraints
        .relations
        .iter()
        .enumerate()
        .map(|(index, relation)| {
            let parameter_keys = relation_parameter_keys(&program, relation);
            let affected_stable_node_keys = dependencies
                .iter()
                .filter(|dependency| parameter_keys.contains(&dependency.parameter_key))
                .flat_map(|dependency| dependency.affected_stable_node_keys.iter().cloned())
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect();
            AuthoringGraphConstraint {
                constraint_id: format!("relation:{index}"),
                label: format!(
                    "{} {} {}",
                    relation_operand(&program, &relation.left),
                    relation.operator.as_str(),
                    relation_operand(&program, &relation.right)
                ),
                kind: "relation".to_string(),
                parameter_keys,
                affected_stable_node_keys,
            }
        })
        .collect();

    let core_identity = ast_nodes
        .iter()
        .map(|node| node.stable_node_key.as_str())
        .chain(
            constraints
                .iter()
                .map(|constraint: &AuthoringGraphConstraint| constraint.constraint_id.as_str()),
        )
        .collect::<Vec<_>>()
        .join("|");
    Ok(AuthoringGraph {
        source_digest: crate::services::render_snapshot::canonical_source_digest(source),
        core_digest: crate::services::render_snapshot::canonical_source_digest(&core_identity),
        artifact_digest: artifact_bundle
            .map(crate::services::render_snapshot::artifact_bundle_digest)
            .transpose()?,
        ast_nodes,
        features,
        dependencies,
        constraints,
        targets,
        handles: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use crate::contracts::ModelManifest;

    #[test]
    fn joins_ast_dependencies_features_and_stable_viewer_targets() {
        let source = "(model (params (number width 20) (number height 10) :relations ((< height width))) (part body (box width height 5)))";
        let manifest: ModelManifest = serde_json::from_value(serde_json::json!({
            "modelId": "model-1",
            "sourceKind": "generated",
            "document": {
                "documentName": "JoinFixture",
                "documentLabel": "Join fixture"
            },
            "parts": [],
            "selectionTargets": [{
                "targetId": "face:body:top",
                "durableTargetId": "durable:body:top",
                "partId": "body",
                "viewerNodeId": "body",
                "label": "Top face",
                "kind": "face",
                "editable": true,
                "parameterKeys": ["width"]
            }],
            "featureGraph": {
                "nodes": [{
                    "featureId": "feature:body:box",
                    "kind": "box",
                    "label": "Body box",
                    "sourceRef": { "path": "/parts/body/root" },
                    "outputRefs": [{
                        "featureId": "feature:body:box",
                        "outputId": "solid",
                        "targetIds": ["durable:body:top"]
                    }]
                }]
            }
        }))
        .expect("manifest fixture");

        let graph =
            super::build_authoring_graph(source, Some(&manifest), None).expect("authoring graph");
        let parameter = graph
            .ast_nodes
            .iter()
            .find(|node| node.path == "/params/width")
            .expect("parameter node");
        let dependency = graph
            .dependencies
            .iter()
            .find(|dependency| dependency.parameter_key == "width")
            .expect("parameter dependency");

        assert!(!parameter.stable_node_key.is_empty());
        assert!(dependency
            .affected_stable_node_keys
            .iter()
            .any(|key| key != &parameter.stable_node_key));
        assert_eq!(dependency.feature_ids, ["feature:body:box"]);
        assert_eq!(dependency.target_ids, ["durable:body:top"]);
        assert_eq!(graph.targets[0].feature_ids, ["feature:body:box"]);
        assert_eq!(graph.targets[0].source_stable_node_keys.len(), 1);
        assert_eq!(graph.constraints[0].parameter_keys, ["height", "width"]);
        assert!(!graph.constraints[0].affected_stable_node_keys.is_empty());
    }

    #[test]
    fn keeps_target_selectable_but_non_editable_without_exact_source_binding() {
        let source = "(model (part body (box 20 10 5)))";
        let manifest: ModelManifest = serde_json::from_value(serde_json::json!({
            "modelId": "model-derived",
            "sourceKind": "generated",
            "document": {
                "documentName": "DerivedFixture",
                "documentLabel": "Derived fixture"
            },
            "selectionTargets": [{
                "targetId": "face:derived:17",
                "partId": "body",
                "viewerNodeId": "body",
                "label": "Derived face",
                "kind": "face",
                "editable": true
            }]
        }))
        .expect("manifest fixture");

        let graph =
            super::build_authoring_graph(source, Some(&manifest), None).expect("authoring graph");
        let target = graph.targets.first().expect("selectable target");

        assert!(!target.editable);
        assert_eq!(
            target.non_editable_reason.as_deref(),
            Some("No feature output or port references target IDs: face:derived:17.")
        );
        let boundary = serde_json::to_value(target).expect("serialize target");
        assert_eq!(boundary["editable"], false);
        assert_eq!(
            boundary["nonEditableReason"],
            "No feature output or port references target IDs: face:derived:17."
        );
        assert!(boundary.get("non_editable_reason").is_none());
    }
}
