//! Native STEP live-component lowering.
//!
//! This module is deliberately separate from `component_import_runtime`: the
//! latter owns package/source resolution while this module owns only the
//! post-resolution STEP boundary. The host must call
//! [`validate_step_asset`] **before** it materializes an `import-step` leaf or
//! invokes Direct OCCT. This keeps mutable package bytes outside the native
//! execution boundary.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use steel_core::parser::ast::ExprKind;
use steel_core::parser::parser::Parser;

use crate::contracts::{
    AppError, AppResult, ComponentCoordinate, GeometryProvenance, GeometryRepresentation,
};
use crate::ecky_scheme::compiler::{expr_identifier, expr_list_items};

/// A package STEP payload that has been resolved by the host but not yet
/// admitted to native execution. `path` is ephemeral and must never be put
/// back into authored source, a lock, a bundle, or an origin record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StepAsset {
    pub coordinate: ComponentCoordinate,
    pub alias: String,
    pub path: PathBuf,
    /// Digest of exactly the bytes passed to `STEPControl_Reader`.
    pub payload_digest: String,
    /// Package-carried evidence. The suffix is never used to infer this.
    pub geometry_provenance: GeometryProvenance,
}

impl StepAsset {
    pub fn canonical_identity(&self) -> String {
        self.coordinate.canonical_identity()
    }
}

/// Validate a resolved STEP asset before lowering. This is intentionally a
/// small public seam so package resolution can call it before any compiler or
/// runner process is launched.
pub fn validate_step_asset(asset: &StepAsset) -> AppResult<()> {
    let extension = asset
        .path
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase);
    if !matches!(extension.as_deref(), Some("step") | Some("stp")) {
        return Err(AppError::validation(format!(
            "STEP component '{}' must resolve to a package-local .step or .stp payload, got '{}'.",
            asset.canonical_identity(),
            asset.path.display()
        )));
    }
    if !matches!(
        &asset.geometry_provenance.representation,
        GeometryRepresentation::AnalyticBrep
            | GeometryRepresentation::FacetedPolyBrep
            | GeometryRepresentation::Hybrid
    ) {
        return Err(AppError::validation(format!(
            "STEP component '{}' requires package geometry provenance with representation analyticBrep, facetedPolyBrep, or hybrid; repack the package rather than inferring it from the file extension.",
            asset.canonical_identity()
        )));
    }
    let payload = fs::read(&asset.path).map_err(|error| {
        AppError::not_found(format!(
            "STEP component '{}' payload '{}' cannot be read before native execution: {}",
            asset.canonical_identity(),
            asset.path.display(),
            error
        ))
    })?;
    let actual_digest = sha256_digest(&payload);
    if actual_digest != asset.payload_digest {
        return Err(AppError::validation(format!(
            "STEP component '{}' payload digest mismatch before native execution: expected '{}', got '{}'.",
            asset.canonical_identity(),
            asset.payload_digest,
            actual_digest
        )));
    }
    Ok(())
}

/// Materialize verified STEP assets as internal zero-argument components.
///
/// The returned string is compiler-only source. It has no `import-component`
/// forms for the supplied assets and is the only place an installed absolute
/// path can appear. The persisted input remains untouched.
pub fn lower_step_assets_to_compiler_source(
    authored_source: &str,
    assets: &[StepAsset],
) -> AppResult<String> {
    let mut aliases = BTreeMap::new();
    for asset in assets {
        validate_step_asset(asset)?;
        if aliases.insert(asset.alias.as_str(), asset).is_some() {
            return Err(AppError::validation(format!(
                "STEP live-import alias '{}' resolves more than once.",
                asset.alias
            )));
        }
    }

    let forms = Parser::parse_without_lowering(authored_source)
        .map_err(|error| AppError::parse(format!("Failed to parse authored source: {error}")))?;
    let mut source = String::new();
    for asset in assets {
        source.push_str("(define-component ");
        source.push_str(&asset.alias);
        source.push_str(" () (import-step ");
        source.push_str(&scheme_string_literal(&asset.path));
        source.push_str("))\n");
    }

    for form in &forms {
        if is_matching_step_import(form, &aliases)? {
            continue;
        }
        reject_step_alias_arguments(form, &aliases)?;
        source.push_str(&form.to_string());
        source.push('\n');
    }
    Ok(source)
}

/// Conservatively merge authored analytic geometry with STEP contributor
/// evidence. Callers apply the result unchanged to bundle, manifest, and STEP
/// export provenance so those public contracts cannot drift.
pub fn merge_step_geometry_provenance(
    authored_representation: GeometryRepresentation,
    assets: &[StepAsset],
) -> GeometryProvenance {
    let representation = assets
        .iter()
        .fold(authored_representation, |current, asset| {
            merge_representation(current, asset.geometry_provenance.representation.clone())
        });
    GeometryProvenance {
        representation,
        source_mesh_digests: Vec::new(),
        closed: None,
        boundary_or_non_manifold_edge_count: None,
    }
}

fn merge_representation(
    left: GeometryRepresentation,
    right: GeometryRepresentation,
) -> GeometryRepresentation {
    use GeometryRepresentation::{AnalyticBrep, FacetedPolyBrep, Hybrid, MeshNative};
    match (left, right) {
        (Hybrid, _) | (_, Hybrid) => Hybrid,
        (AnalyticBrep, AnalyticBrep) => AnalyticBrep,
        (FacetedPolyBrep, FacetedPolyBrep) => FacetedPolyBrep,
        // `MeshNative` is not admissible for STEP assets, but preserving it as
        // hybrid here makes this function conservative if a future caller
        // combines it with imported package evidence.
        (MeshNative, MeshNative) => MeshNative,
        _ => Hybrid,
    }
}

fn sha256_digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

fn is_matching_step_import(
    form: &ExprKind,
    aliases: &BTreeMap<&str, &StepAsset>,
) -> AppResult<bool> {
    let Ok(items) = expr_list_items(form, "top-level form") else {
        return Ok(false);
    };
    if items.first().and_then(expr_identifier).as_deref() != Some("import-component") {
        return Ok(false);
    }
    let alias = import_alias(&items)?;
    let Some(asset) = aliases.get(alias.as_str()) else {
        return Ok(false);
    };
    let coordinate = import_coordinate(&items)?;
    if coordinate != asset.coordinate {
        return Err(AppError::validation(format!(
            "STEP alias '{}' declares '{}', but host resolution admitted '{}'.",
            alias,
            coordinate.canonical_identity(),
            asset.canonical_identity()
        )));
    }
    Ok(true)
}

fn reject_step_alias_arguments(
    form: &ExprKind,
    aliases: &BTreeMap<&str, &StepAsset>,
) -> AppResult<()> {
    match form {
        ExprKind::Quote(_) => Ok(()),
        ExprKind::List(_) | ExprKind::Vector(_) => {
            let Ok(items) = expr_list_items(form, "form") else {
                return Ok(());
            };
            if let Some(alias) = items.first().and_then(expr_identifier) {
                if aliases.contains_key(alias.as_str()) && items.len() != 1 {
                    return Err(AppError::validation(format!(
                        "STEP component alias '{}' is a static zero-argument shape; positional and keyword geometry arguments are not supported.",
                        alias
                    )));
                }
            }
            for item in items {
                reject_step_alias_arguments(&item, aliases)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn import_alias(items: &[ExprKind]) -> AppResult<String> {
    let mut index = 2;
    while index + 1 < items.len() {
        if keyword_name(&items[index]).as_deref() == Some("as") {
            return identifier_value(&items[index + 1], "alias");
        }
        index += 2;
    }
    Err(AppError::validation(
        "`import-component` requires a literal `:as` alias symbol.",
    ))
}

fn import_coordinate(items: &[ExprKind]) -> AppResult<ComponentCoordinate> {
    let package_id = string_value(
        items
            .get(1)
            .ok_or_else(|| AppError::validation("`import-component` requires packageId."))?,
        "packageId",
    )?;
    let mut version = None;
    let mut component_id = None;
    let mut index = 2;
    while index + 1 < items.len() {
        match keyword_name(&items[index]).as_deref() {
            Some("version") => version = Some(string_value(&items[index + 1], "version")?),
            Some("component") => component_id = Some(string_value(&items[index + 1], "component")?),
            _ => {}
        }
        index += 2;
    }
    Ok(ComponentCoordinate {
        package_id,
        version: version.ok_or_else(|| {
            AppError::validation("`import-component` requires a literal `:version` string.")
        })?,
        component_id: component_id.ok_or_else(|| {
            AppError::validation("`import-component` requires a literal `:component` string.")
        })?,
    })
}

fn keyword_name(expr: &ExprKind) -> Option<String> {
    use steel_core::parser::tokens::TokenType;
    let ExprKind::Atom(atom) = expr else {
        return None;
    };
    match &atom.syn.ty {
        TokenType::Keyword(value) => Some(
            value
                .to_string()
                .trim_start_matches("#:")
                .trim_start_matches(':')
                .to_string(),
        ),
        TokenType::Identifier(value) => value.to_string().strip_prefix(':').map(str::to_string),
        _ => None,
    }
}

fn string_value(expr: &ExprKind, field: &str) -> AppResult<String> {
    use steel_core::parser::tokens::TokenType;
    let ExprKind::Atom(atom) = expr else {
        return Err(AppError::validation(format!(
            "`import-component` {field} must be a literal string."
        )));
    };
    if let TokenType::StringLiteral(value) = &atom.syn.ty {
        return Ok(value.resolve().to_string());
    }
    Err(AppError::validation(format!(
        "`import-component` {field} must be a literal string."
    )))
}

fn identifier_value(expr: &ExprKind, field: &str) -> AppResult<String> {
    expr_identifier(expr).ok_or_else(|| {
        AppError::validation(format!(
            "`import-component` {field} must be a literal symbol."
        ))
    })
}

fn scheme_string_literal(path: &Path) -> String {
    let value = path.to_string_lossy();
    let mut quoted = String::from("\"");
    for character in value.chars() {
        match character {
            '\\' => quoted.push_str("\\\\"),
            '\"' => quoted.push_str("\\\""),
            '\n' => quoted.push_str("\\n"),
            '\r' => quoted.push_str("\\r"),
            '\t' => quoted.push_str("\\t"),
            control if control.is_control() => {
                quoted.push_str(&format!("\\u{{{:x}}}", control as u32))
            }
            other => quoted.push(other),
        }
    }
    quoted.push('\"');
    quoted
}
