use std::collections::BTreeMap;

use serde::Serialize;

use crate::contracts::AppResult;
use crate::ecky_core_ir::{
    CoreBooleanOp, CoreKeywordValue, CoreNode, CoreNodeKind, CoreOperation, CoreParameter,
    CoreParameterConstraints, CoreProgram, CoreReference, CoreRelationConstraint,
    CoreRelationOperand,
};

#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct MissionCoreIrEvaluation {
    pub matched: bool,
}

#[tauri::command]
#[specta::specta]
pub fn evaluate_mission_core_ir(
    candidate_source: String,
    reference_source: String,
) -> AppResult<MissionCoreIrEvaluation> {
    let candidate = crate::ecky_scheme::compile_to_core_program(&candidate_source)
        .map_err(crate::ecky_scheme::core_err_to_app)?;
    let reference = crate::ecky_scheme::compile_to_core_program(&reference_source)
        .map_err(crate::ecky_scheme::core_err_to_app)?;
    let candidate_canonical = canonical_program(&candidate);
    let reference_canonical = canonical_program(&reference);
    Ok(MissionCoreIrEvaluation {
        matched: candidate_canonical == reference_canonical,
    })
}

fn canonical_program(program: &CoreProgram) -> String {
    let params = program
        .parameters
        .iter()
        .enumerate()
        .map(|(i, p)| (p.id.raw(), format!("p{i}")))
        .collect();
    let parts = program
        .parts
        .iter()
        .enumerate()
        .map(|(i, p)| (p.id.raw(), format!("part{i}")))
        .collect();
    let parameter_contract = program
        .parameters
        .iter()
        .enumerate()
        .map(|(i, parameter)| canonical_parameter(parameter, &params, i))
        .collect::<Vec<_>>()
        .join(";");
    let relation_contract = program
        .constraints
        .relations
        .iter()
        .map(|relation| canonical_relation(relation, &params))
        .collect::<Vec<_>>()
        .join(";");
    let verify_contract = format!("{:?}", program.constraints.verify_clauses);
    format!(
        "program[params[{parameter_contract}];relations[{relation_contract}];verify[{verify_contract}];parts[{}]]",
        program
            .parts
            .iter()
            .map(|part| canonical_node(
                &part.root,
                &params,
                &parts,
                &BTreeMap::new(),
                &BTreeMap::new(),
            ))
            .collect::<Vec<_>>()
            .join(";")
    )
}

fn canonical_parameter(
    parameter: &CoreParameter,
    params: &BTreeMap<u64, String>,
    index: usize,
) -> String {
    format!(
        "p{index}:{:?}={:?}:frozen={}:{}",
        parameter.kind,
        parameter.default_value,
        parameter.frozen,
        canonical_parameter_constraints(&parameter.constraints, params),
    )
}

fn canonical_parameter_constraints(
    constraints: &CoreParameterConstraints,
    params: &BTreeMap<u64, String>,
) -> String {
    let choices = constraints
        .choices
        .iter()
        .map(|choice| format!("{:?}", choice.value))
        .collect::<Vec<_>>()
        .join(",");
    let relations = constraints
        .relations
        .iter()
        .map(|relation| canonical_relation(relation, params))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "constraints(min={:?},max={:?},step={:?},unit={:?},choices=[{choices}],relations=[{relations}])",
        constraints.min, constraints.max, constraints.step, constraints.unit,
    )
}

fn canonical_relation(relation: &CoreRelationConstraint, params: &BTreeMap<u64, String>) -> String {
    format!(
        "{}{}{}",
        canonical_relation_operand(&relation.left, params),
        relation.operator.as_str(),
        canonical_relation_operand(&relation.right, params),
    )
}

fn canonical_relation_operand(
    operand: &CoreRelationOperand,
    params: &BTreeMap<u64, String>,
) -> String {
    match operand {
        CoreRelationOperand::Parameter(id) => params
            .get(&id.raw())
            .cloned()
            .unwrap_or_else(|| "param?".into()),
        CoreRelationOperand::Number(value) => format!("{value:?}"),
    }
}

fn canonical_node(
    node: &CoreNode,
    params: &BTreeMap<u64, String>,
    parts: &BTreeMap<u64, String>,
    locals: &BTreeMap<String, String>,
    nodes: &BTreeMap<u64, String>,
) -> String {
    match &node.kind {
        CoreNodeKind::Literal(value) => format!("{value:?}"),
        CoreNodeKind::Reference(reference) => match reference {
            CoreReference::Parameter(id) => params
                .get(&id.raw())
                .cloned()
                .unwrap_or_else(|| "param?".into()),
            CoreReference::Part(id) => parts
                .get(&id.raw())
                .cloned()
                .unwrap_or_else(|| "part?".into()),
            CoreReference::Node(id) => nodes
                .get(&id.raw())
                .cloned()
                .unwrap_or_else(|| "node?".into()),
            CoreReference::Local(name) => locals
                .get(name)
                .cloned()
                .unwrap_or_else(|| format!("free:{name}")),
        },
        CoreNodeKind::Build { bindings, result } => canonical_bindings(
            "build",
            bindings.iter().map(|b| (&b.name, &b.value)),
            result,
            params,
            parts,
            locals,
            nodes,
        ),
        CoreNodeKind::Let { bindings, body } => canonical_bindings(
            "let",
            bindings.iter().map(|b| (&b.name, &b.value)),
            body,
            params,
            parts,
            locals,
            nodes,
        ),
        CoreNodeKind::If {
            condition,
            then_branch,
            else_branch,
        } => format!(
            "if({},{},{})",
            canonical_node(condition, params, parts, locals, nodes),
            canonical_node(then_branch, params, parts, locals, nodes),
            canonical_node(else_branch, params, parts, locals, nodes)
        ),
        CoreNodeKind::Call { op, args, keywords } => {
            let mut values = args
                .iter()
                .map(|arg| canonical_node(arg, params, parts, locals, nodes))
                .collect::<Vec<_>>();
            if matches!(
                op,
                CoreOperation::Boolean(
                    CoreBooleanOp::Union | CoreBooleanOp::Intersection | CoreBooleanOp::Xor
                )
            ) {
                values.sort();
            }
            let keywords = keywords
                .iter()
                .map(|keyword| match &keyword.value {
                    CoreKeywordValue::Expr(value) => format!(
                        "{}={}",
                        keyword.name,
                        canonical_node(value, params, parts, locals, nodes)
                    ),
                    CoreKeywordValue::Selector { source, payload } => format!(
                        "{}={}:{}",
                        keyword.name,
                        canonical_node(source, params, parts, locals, nodes),
                        format_args!("{payload:?}")
                    ),
                })
                .collect::<Vec<_>>()
                .join(",");
            format!("{op:?}({};{keywords})", values.join(","))
        }
        CoreNodeKind::Range { start, end } => format!(
            "range({},{})",
            canonical_node(start, params, parts, locals, nodes),
            canonical_node(end, params, parts, locals, nodes)
        ),
        CoreNodeKind::Map {
            params: names,
            sources,
            body,
        } => {
            let mut scoped = locals.clone();
            for (i, name) in names.iter().enumerate() {
                scoped.insert(name.clone(), format!("m{i}"));
            }
            format!(
                "map({};{})",
                sources
                    .iter()
                    .map(|source| canonical_node(source, params, parts, locals, nodes))
                    .collect::<Vec<_>>()
                    .join(","),
                canonical_node(body, params, parts, &scoped, nodes)
            )
        }
        CoreNodeKind::Apply { op, args, list } => format!(
            "apply:{op:?}({};{})",
            args.iter()
                .map(|arg| canonical_node(arg, params, parts, locals, nodes))
                .collect::<Vec<_>>()
                .join(","),
            canonical_node(list, params, parts, locals, nodes)
        ),
        CoreNodeKind::List(items) => format!(
            "list({})",
            items
                .iter()
                .map(|item| canonical_node(item, params, parts, locals, nodes))
                .collect::<Vec<_>>()
                .join(",")
        ),
        CoreNodeKind::Group(items) => format!(
            "group({})",
            items
                .iter()
                .map(|item| canonical_node(item, params, parts, locals, nodes))
                .collect::<Vec<_>>()
                .join(",")
        ),
    }
}

fn canonical_bindings<'a>(
    label: &str,
    bindings: impl Iterator<Item = (&'a String, &'a CoreNode)>,
    body: &CoreNode,
    params: &BTreeMap<u64, String>,
    parts: &BTreeMap<u64, String>,
    locals: &BTreeMap<String, String>,
    nodes: &BTreeMap<u64, String>,
) -> String {
    let mut scoped = locals.clone();
    let mut scoped_nodes = nodes.clone();
    let mut text = format!("{label}[");
    for (i, (original, value)) in bindings.enumerate() {
        let canonical = canonical_node(value, params, parts, &scoped, &scoped_nodes);
        let name = format!("v{i}");
        scoped.insert(original.clone(), name.clone());
        scoped_nodes.insert(value.id.raw(), name.clone());
        text.push_str(&format!("{name}={canonical};"));
    }
    text.push_str(&canonical_node(body, params, parts, &scoped, &scoped_nodes));
    text.push(']');
    text
}

#[cfg(test)]
mod tests {
    use super::evaluate_mission_core_ir;

    #[test]
    fn semantically_equivalent_commutative_union_passes() {
        let result = evaluate_mission_core_ir(
            "(model (part body (union (box 1 2 3) (cylinder 2 4))))".into(),
            "(model (part renamed (union (cylinder 2 4) (box 1 2 3))))".into(),
        )
        .expect("both compile");
        assert!(result.matched);
    }

    #[test]
    fn surface_alias_and_canonical_operation_share_core_ir() {
        let result = evaluate_mission_core_ir(
            "(model (part body (fuse (box 1 2 3) (cylinder 2 4))))".into(),
            "(model (part body (union (box 1 2 3) (cylinder 2 4))))".into(),
        )
        .expect("both compile");
        assert!(result.matched);
    }

    #[test]
    fn difference_operand_order_remains_semantic() {
        let result = evaluate_mission_core_ir(
            "(model (part body (difference (box 8 8 8) (sphere 2))))".into(),
            "(model (part body (difference (sphere 2) (box 8 8 8))))".into(),
        )
        .expect("both compile");
        assert!(!result.matched);
    }

    #[test]
    fn dead_or_wrong_branch_fails_exact_core_ir_match() {
        let result = evaluate_mission_core_ir("(model (params (toggle enabled false)) (part body (if enabled (box 1 1 1) (sphere 2))))".into(), "(model (params (toggle enabled false)) (part body (sphere 2)))".into()).expect("both compile");
        assert!(!result.matched);
    }

    #[test]
    fn parameter_contract_is_part_of_core_ir_match() {
        let result = evaluate_mission_core_ir(
            "(model (params (number size 8 :min 2 :max 20)) (part body (box size 2 2)))".into(),
            "(model (params (number renamed 9 :min 2 :max 20)) (part body (box renamed 2 2)))"
                .into(),
        )
        .expect("both compile");
        assert!(!result.matched, "different defaults must not pass");
    }

    #[test]
    fn renamed_build_bindings_do_not_change_core_ir_match() {
        let result = evaluate_mission_core_ir(
            "(model (part body (build (shape blank (box 8 8 2)) (shape bore (cylinder 1 4)) (result (difference blank bore)))))".into(),
            "(model (part renamed (build (shape stock (box 8 8 2)) (shape cutter (cylinder 1 4)) (result (difference stock cutter)))))".into(),
        )
        .expect("both compile");
        assert!(
            result.matched,
            "generated node ids and binding names must normalize"
        );
    }

    #[test]
    fn compiler_error_surfaces_without_token_fallback() {
        let error = evaluate_mission_core_ir(
            "(model (part body".into(),
            "(model (part body (box 1 1 1)))".into(),
        )
        .expect_err("invalid candidate must report compiler error");
        assert!(error.to_string().contains("parse"), "{error}");
    }
}
