use super::{
    CompilerError, CompilerErrorKind, CoreEdgeAxis, CoreEdgeBound, CoreEdgeSelectorClause,
    CoreFaceAreaRank, CoreFaceSelectorClause, CoreResult, CoreSelectorPayload,
};

const EDGE_SELECTOR_HELP: &str =
    "`all`, `top`, `bottom`, `left`, `right`, `front`, `back`, `vertical`, `axis-x`, `axis-y`, `axis-z`, `x-min`, `x-max`, `y-min`, `y-max`, `z-min`, `z-max`, `target-id:<id>`, `target-ids:<id>|<id>`, or `+` intersections such as `x-min+axis-z`.";
const FACE_SELECTOR_HELP: &str =
    "`all`, `planar`, `normal-x`, `normal-y`, `normal-z`, `area-min`, `area-max`, `top`, `bottom`, `left`, `right`, `front`, `back`, `x-min`, `x-max`, `y-min`, `y-max`, `z-min`, `z-max`, `target-id:<id>`, `target-ids:<id>|<id>`, or `+` intersections such as `planar+normal-z+z-max`.";

pub fn parse_core_edge_selector_payload(selector: &str) -> CoreResult<CoreSelectorPayload> {
    if let Some(target_ids) = parse_target_ids(selector, ":edge:", "Edge")? {
        return Ok(CoreSelectorPayload::EdgeTargetIds(target_ids));
    }
    let normalized = selector.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return Err(unknown_edge_selector(selector));
    }
    if normalized == "all" {
        return Ok(CoreSelectorPayload::EdgeAll);
    }

    let clauses = normalized
        .split('+')
        .map(|token| parse_edge_clause(token.trim(), selector))
        .collect::<CoreResult<Vec<_>>>()?;
    Ok(CoreSelectorPayload::EdgeClauses(clauses))
}

pub fn parse_core_face_selector_payload(selector: &str) -> CoreResult<CoreSelectorPayload> {
    if let Some(target_ids) = parse_target_ids(selector, ":face:", "Face")? {
        return Ok(CoreSelectorPayload::FaceTargetIds(target_ids));
    }
    let normalized = selector.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return Err(unknown_face_selector(selector));
    }
    if normalized == "all" {
        return Ok(CoreSelectorPayload::FaceClauses(Vec::new()));
    }

    let clauses = normalized
        .split('+')
        .map(|token| parse_face_clause(token.trim(), selector))
        .collect::<CoreResult<Vec<_>>>()?;
    Ok(CoreSelectorPayload::FaceClauses(clauses))
}

fn parse_edge_clause(token: &str, full_selector: &str) -> CoreResult<CoreEdgeSelectorClause> {
    if token.is_empty() {
        return Err(unknown_edge_selector(full_selector));
    }
    if token == "all" {
        return Err(selector_error(format!(
            "Edge selector `{full_selector}` cannot combine `all` with other clauses."
        )));
    }
    match token {
        "top" => return Ok(boundary(CoreEdgeAxis::Z, CoreEdgeBound::Max)),
        "bottom" => return Ok(boundary(CoreEdgeAxis::Z, CoreEdgeBound::Min)),
        "left" => return Ok(boundary(CoreEdgeAxis::X, CoreEdgeBound::Min)),
        "right" => return Ok(boundary(CoreEdgeAxis::X, CoreEdgeBound::Max)),
        "front" => return Ok(boundary(CoreEdgeAxis::Y, CoreEdgeBound::Max)),
        "back" => return Ok(boundary(CoreEdgeAxis::Y, CoreEdgeBound::Min)),
        "vertical" => return Ok(CoreEdgeSelectorClause::Axis(CoreEdgeAxis::Z)),
        _ => {}
    }
    if let Some(axis) = token.strip_prefix("axis-") {
        return Ok(CoreEdgeSelectorClause::Axis(parse_axis(
            axis,
            full_selector,
            EDGE_SELECTOR_HELP,
        )?));
    }
    if let Some((axis, bound)) = token.split_once('-') {
        let axis = parse_axis(axis, full_selector, EDGE_SELECTOR_HELP)?;
        let bound = parse_bound(bound).ok_or_else(|| unknown_edge_selector(full_selector))?;
        return Ok(boundary(axis, bound));
    }
    Err(unknown_edge_selector(full_selector))
}

fn parse_face_clause(token: &str, full_selector: &str) -> CoreResult<CoreFaceSelectorClause> {
    if token.is_empty() {
        return Err(unknown_face_selector(full_selector));
    }
    if token == "all" {
        return Err(selector_error(format!(
            "Face selector `{full_selector}` cannot combine `all` with other clauses."
        )));
    }
    match token {
        "top" => return Ok(face_boundary(CoreEdgeAxis::Z, CoreEdgeBound::Max)),
        "bottom" => return Ok(face_boundary(CoreEdgeAxis::Z, CoreEdgeBound::Min)),
        "left" => return Ok(face_boundary(CoreEdgeAxis::X, CoreEdgeBound::Min)),
        "right" => return Ok(face_boundary(CoreEdgeAxis::X, CoreEdgeBound::Max)),
        "front" => return Ok(face_boundary(CoreEdgeAxis::Y, CoreEdgeBound::Max)),
        "back" => return Ok(face_boundary(CoreEdgeAxis::Y, CoreEdgeBound::Min)),
        "planar" => return Ok(CoreFaceSelectorClause::Planar),
        "area-min" => return Ok(CoreFaceSelectorClause::Area(CoreFaceAreaRank::Min)),
        "area-max" => return Ok(CoreFaceSelectorClause::Area(CoreFaceAreaRank::Max)),
        _ => {}
    }
    if let Some(axis) = token.strip_prefix("normal-") {
        return Ok(CoreFaceSelectorClause::Normal(parse_axis(
            axis,
            full_selector,
            FACE_SELECTOR_HELP,
        )?));
    }
    if let Some((axis, bound)) = token.split_once('-') {
        let axis = parse_axis(axis, full_selector, FACE_SELECTOR_HELP)?;
        let bound = parse_bound(bound).ok_or_else(|| unknown_face_selector(full_selector))?;
        return Ok(face_boundary(axis, bound));
    }
    Err(unknown_face_selector(full_selector))
}

fn parse_target_ids(
    selector: &str,
    required_marker: &str,
    kind: &str,
) -> CoreResult<Option<Vec<String>>> {
    let raw = selector.trim();
    let lower = raw.to_ascii_lowercase();
    let payload = if lower.starts_with("target-id:") {
        raw.get("target-id:".len()..)
    } else if lower.starts_with("target-ids:") {
        raw.get("target-ids:".len()..)
    } else {
        return Ok(None);
    };
    let target_ids = payload
        .unwrap_or_default()
        .split(['|', ','])
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .fold(Vec::<String>::new(), |mut ids, item| {
            if !ids.iter().any(|existing| existing == item) {
                ids.push(item.to_string());
            }
            ids
        });
    if target_ids.is_empty() {
        return Err(selector_error(format!(
            "{kind} selector `{selector}` did not include any target ids."
        )));
    }
    if let Some(invalid) = target_ids
        .iter()
        .find(|target_id| !target_id.contains(required_marker))
    {
        return Err(selector_error(format!(
            "{kind} selector `{selector}` included non-{} target id `{invalid}`.",
            kind.to_ascii_lowercase()
        )));
    }
    Ok(Some(target_ids))
}

fn parse_axis(axis: &str, selector: &str, help: &str) -> CoreResult<CoreEdgeAxis> {
    match axis {
        "x" => Ok(CoreEdgeAxis::X),
        "y" => Ok(CoreEdgeAxis::Y),
        "z" => Ok(CoreEdgeAxis::Z),
        _ => Err(selector_error(format!(
            "Unknown selector `{selector}`. Use {help}"
        ))),
    }
}

fn parse_bound(bound: &str) -> Option<CoreEdgeBound> {
    match bound {
        "min" => Some(CoreEdgeBound::Min),
        "max" => Some(CoreEdgeBound::Max),
        _ => None,
    }
}

fn boundary(axis: CoreEdgeAxis, bound: CoreEdgeBound) -> CoreEdgeSelectorClause {
    CoreEdgeSelectorClause::Boundary { axis, bound }
}

fn face_boundary(axis: CoreEdgeAxis, bound: CoreEdgeBound) -> CoreFaceSelectorClause {
    CoreFaceSelectorClause::Boundary { axis, bound }
}

fn unknown_edge_selector(selector: &str) -> CompilerError {
    selector_error(format!(
        "Unknown edge selector `{selector}`. Use {EDGE_SELECTOR_HELP}"
    ))
}

fn unknown_face_selector(selector: &str) -> CompilerError {
    selector_error(format!(
        "Unknown face selector `{selector}`. Use {FACE_SELECTOR_HELP}"
    ))
}

fn selector_error(message: String) -> CompilerError {
    CompilerError::new(CompilerErrorKind::TypeMismatch, message)
}
