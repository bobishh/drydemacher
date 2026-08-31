use crate::contracts::{
    Advisory, AdvisoryCondition, AppError, AppResult, ControlPrimitive, ControlRelation,
    ControlView, ControlViewScope, ControlViewSection, ControlViewSource, EnrichmentStatus,
    ModelManifest, ModelSourceKind, ParameterGroup, ProposalStatusEdit, SemanticManifestEditIntent,
    SemanticManifestEditResult, SourceLanguage,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemanticEditActor {
    Manual,
    Llm,
}

impl SemanticEditActor {
    fn source(self) -> ControlViewSource {
        match self {
            Self::Manual => ControlViewSource::Manual,
            Self::Llm => ControlViewSource::Llm,
        }
    }

    fn id_segment(self) -> &'static str {
        match self {
            Self::Manual => "manual",
            Self::Llm => "llm",
        }
    }
}

pub fn apply_semantic_manifest_edit(
    manifest: &ModelManifest,
    edit: SemanticManifestEditIntent,
    actor: SemanticEditActor,
) -> AppResult<SemanticManifestEditResult> {
    ensure_persisted_semantic_edits_supported(manifest)?;

    let result = match edit {
        SemanticManifestEditIntent::SaveView {
            view_id,
            label,
            scope,
            part_ids,
            primitive_ids,
            sections,
            is_default,
        } => {
            let view_id = match view_id {
                Some(view_id) => require_trimmed(&view_id, "View id")?,
                None => generated_id("view", actor, uuid::Uuid::new_v4()),
            };
            let next = upsert_control_view(
                manifest,
                ControlView {
                    view_id: view_id.clone(),
                    label,
                    scope,
                    part_ids,
                    primitive_ids,
                    sections,
                    is_default,
                    source: actor.source(),
                    status: EnrichmentStatus::None,
                    order: 0,
                },
                actor,
            )?;
            SemanticManifestEditResult {
                manifest: next,
                edited_id: view_id.clone(),
                selected_view_id: Some(view_id),
            }
        }
        SemanticManifestEditIntent::DeleteView { view_id } => {
            let view_id = require_trimmed(&view_id, "View id")?;
            let next = delete_control_view(manifest, &view_id, actor)?;
            SemanticManifestEditResult {
                manifest: next,
                edited_id: view_id,
                selected_view_id: None,
            }
        }
        SemanticManifestEditIntent::SavePrimitive {
            primitive_id,
            label,
            primitive_kind,
            scope,
            part_id,
            bindings,
            attach_to_view,
            base_view_id,
        } => {
            let label = require_trimmed(&label, "Primitive label")?;
            if bindings.is_empty() {
                return Err(AppError::validation(
                    "Manual primitive must include at least one binding.",
                ));
            }
            for binding in &bindings {
                if binding.parameter_key.trim().is_empty() {
                    return Err(AppError::validation(
                        "Manual primitive binding parameterKey cannot be empty.",
                    ));
                }
                if !binding.scale.is_finite()
                    || !binding.offset.is_finite()
                    || binding.min.is_some_and(|value| !value.is_finite())
                    || binding.max.is_some_and(|value| !value.is_finite())
                {
                    return Err(AppError::validation(
                        "Manual primitive binding values must be finite.",
                    ));
                }
                if matches!((binding.min, binding.max), (Some(min), Some(max)) if min > max) {
                    return Err(AppError::validation(format!(
                        "Manual primitive binding '{}' has min greater than max.",
                        binding.parameter_key
                    )));
                }
            }

            let part_ids = match scope {
                ControlViewScope::Global => Vec::new(),
                ControlViewScope::Part => vec![require_trimmed(
                    part_id.as_deref().unwrap_or_default(),
                    "Primitive part id",
                )?],
            };
            let requested_id = primitive_id
                .as_deref()
                .map(|value| require_trimmed(value, "Primitive id"))
                .transpose()?;
            let existing = requested_id.as_deref().and_then(|primitive_id| {
                manifest
                    .control_primitives
                    .iter()
                    .find(|primitive| primitive.primitive_id == primitive_id)
            });
            if requested_id.is_some() && existing.is_none() {
                return Err(AppError::validation(format!(
                    "Control primitive '{}' was not found.",
                    requested_id.as_deref().unwrap_or_default()
                )));
            }
            if existing.is_some_and(|primitive| primitive.source != actor.source()) {
                return Err(AppError::validation(format!(
                    "{} primitive edit cannot replace non-{} primitive '{}'.",
                    actor.id_segment(),
                    actor.id_segment(),
                    requested_id.as_deref().unwrap_or_default()
                )));
            }

            let primitive_id = requested_id
                .unwrap_or_else(|| generated_id("primitive", actor, uuid::Uuid::new_v4()));
            let order = existing
                .map(|primitive| primitive.order)
                .unwrap_or_else(|| {
                    manifest
                        .control_primitives
                        .iter()
                        .map(|primitive| primitive.order)
                        .max()
                        .unwrap_or(0)
                        + 1
                });
            let primitive = ControlPrimitive {
                primitive_id: primitive_id.clone(),
                label,
                kind: primitive_kind,
                source: actor.source(),
                part_ids,
                bindings,
                editable: true,
                order,
            };
            let mut next = manifest.clone();
            next.control_primitives
                .retain(|entry| entry.primitive_id != primitive_id);
            next.control_primitives.push(primitive);
            sort_primitives(&mut next.control_primitives);

            let selected_view_id = if attach_to_view {
                Some(attach_primitive_to_view(
                    &mut next,
                    &primitive_id,
                    base_view_id.as_deref(),
                    scope,
                    part_id.as_deref(),
                    actor,
                )?)
            } else {
                None
            };
            next.validate()?;
            SemanticManifestEditResult {
                manifest: next,
                edited_id: primitive_id,
                selected_view_id,
            }
        }
        SemanticManifestEditIntent::DeletePrimitive { primitive_id } => {
            let primitive_id = require_trimmed(&primitive_id, "Primitive id")?;
            let existing = manifest
                .control_primitives
                .iter()
                .find(|primitive| primitive.primitive_id == primitive_id)
                .ok_or_else(|| {
                    AppError::validation(format!(
                        "Control primitive '{}' was not found.",
                        primitive_id
                    ))
                })?;
            if existing.source != actor.source() {
                return Err(AppError::validation(format!(
                    "{} primitive edit cannot delete non-{} primitive '{}'.",
                    actor.id_segment(),
                    actor.id_segment(),
                    primitive_id
                )));
            }

            let mut next = manifest.clone();
            next.control_primitives
                .retain(|primitive| primitive.primitive_id != primitive_id);
            next.control_relations.retain(|relation| {
                relation.source_primitive_id != primitive_id
                    && relation.target_primitive_id != primitive_id
            });
            for advisory in &mut next.advisories {
                advisory
                    .primitive_ids
                    .retain(|entry| entry != &primitive_id);
            }
            for target in &mut next.selection_targets {
                target.primitive_ids.retain(|entry| entry != &primitive_id);
            }
            for annotation in &mut next.measurement_annotations {
                annotation
                    .primitive_ids
                    .retain(|entry| entry != &primitive_id);
            }
            let mut removed_view_ids = Vec::new();
            for view in &mut next.control_views {
                view.primitive_ids.retain(|entry| entry != &primitive_id);
                for section in &mut view.sections {
                    section.primitive_ids.retain(|entry| entry != &primitive_id);
                }
                view.sections
                    .retain(|section| !section.primitive_ids.is_empty());
                if view.source == actor.source() && view.primitive_ids.is_empty() {
                    removed_view_ids.push(view.view_id.clone());
                }
            }
            next.control_views
                .retain(|view| !removed_view_ids.contains(&view.view_id));
            for advisory in &mut next.advisories {
                advisory
                    .view_ids
                    .retain(|view_id| !removed_view_ids.contains(view_id));
            }
            for target in &mut next.selection_targets {
                target
                    .view_ids
                    .retain(|view_id| !removed_view_ids.contains(view_id));
            }
            next.validate()?;
            SemanticManifestEditResult {
                manifest: next,
                edited_id: primitive_id,
                selected_view_id: None,
            }
        }
        SemanticManifestEditIntent::SaveAdvisory {
            label,
            severity,
            primitive_ids,
            view_id,
            message,
            condition,
            threshold,
        } => {
            let label = require_trimmed(&label, "Advisory label")?;
            let message = require_trimmed(&message, "Advisory message")?;
            if primitive_ids.is_empty() {
                return Err(AppError::validation(
                    "Manual advisory must reference at least one primitive.",
                ));
            }
            let threshold = match condition {
                AdvisoryCondition::Always => None,
                AdvisoryCondition::Below | AdvisoryCondition::Above => {
                    let threshold = threshold.ok_or_else(|| {
                        AppError::validation(
                            "Conditional advisory must include a finite threshold.",
                        )
                    })?;
                    if !threshold.is_finite() {
                        return Err(AppError::validation(
                            "Conditional advisory threshold must be finite.",
                        ));
                    }
                    Some(threshold)
                }
            };
            let advisory_id = generated_id("advisory", actor, uuid::Uuid::new_v4());
            let advisory = Advisory {
                advisory_id: advisory_id.clone(),
                label,
                severity,
                primitive_ids: dedupe(primitive_ids),
                view_ids: view_id.into_iter().collect(),
                message,
                condition,
                threshold,
            };
            let mut next = manifest.clone();
            next.advisories.push(advisory);
            next.validate()?;
            SemanticManifestEditResult {
                manifest: next,
                edited_id: advisory_id,
                selected_view_id: None,
            }
        }
        SemanticManifestEditIntent::DeleteAdvisory { advisory_id } => {
            let advisory_id = require_trimmed(&advisory_id, "Advisory id")?;
            ensure_prefixed_owner("advisory", &advisory_id, actor)?;
            if !manifest
                .advisories
                .iter()
                .any(|advisory| advisory.advisory_id == advisory_id)
            {
                return Err(AppError::validation(format!(
                    "Advisory '{}' was not found.",
                    advisory_id
                )));
            }
            let mut next = manifest.clone();
            next.advisories
                .retain(|advisory| advisory.advisory_id != advisory_id);
            next.validate()?;
            SemanticManifestEditResult {
                manifest: next,
                edited_id: advisory_id,
                selected_view_id: None,
            }
        }
        SemanticManifestEditIntent::SaveRelation {
            source_primitive_id,
            target_primitive_id,
            mode,
            scale,
            offset,
        } => {
            let source_primitive_id =
                require_trimmed(&source_primitive_id, "Relation source primitive id")?;
            let target_primitive_id =
                require_trimmed(&target_primitive_id, "Relation target primitive id")?;
            if !scale.is_finite() || !offset.is_finite() {
                return Err(AppError::validation(
                    "Control relation scale and offset must be finite.",
                ));
            }
            let relation_id = generated_id("relation", actor, uuid::Uuid::new_v4());
            let relation = ControlRelation {
                relation_id: relation_id.clone(),
                source_primitive_id,
                target_primitive_id,
                mode,
                scale,
                offset,
                enabled: true,
            };
            let mut next = manifest.clone();
            next.control_relations.push(relation);
            next.validate()?;
            SemanticManifestEditResult {
                manifest: next,
                edited_id: relation_id,
                selected_view_id: None,
            }
        }
        SemanticManifestEditIntent::DeleteRelation { relation_id } => {
            let relation_id = require_trimmed(&relation_id, "Relation id")?;
            ensure_prefixed_owner("relation", &relation_id, actor)?;
            if !manifest
                .control_relations
                .iter()
                .any(|relation| relation.relation_id == relation_id)
            {
                return Err(AppError::validation(format!(
                    "Control relation '{}' was not found.",
                    relation_id
                )));
            }
            let mut next = manifest.clone();
            next.control_relations
                .retain(|relation| relation.relation_id != relation_id);
            next.validate()?;
            SemanticManifestEditResult {
                manifest: next,
                edited_id: relation_id,
                selected_view_id: None,
            }
        }
        SemanticManifestEditIntent::SetProposalStatus {
            proposal_id,
            status,
        } => apply_proposal_statuses(
            manifest,
            vec![ProposalStatusEdit {
                proposal_id,
                status,
            }],
        )?,
        SemanticManifestEditIntent::SetProposalStatuses { entries } => {
            apply_proposal_statuses(manifest, entries)?
        }
    };

    Ok(result)
}

fn attach_primitive_to_view(
    manifest: &mut ModelManifest,
    primitive_id: &str,
    base_view_id: Option<&str>,
    fallback_scope: ControlViewScope,
    fallback_part_id: Option<&str>,
    actor: SemanticEditActor,
) -> AppResult<String> {
    let base_view = base_view_id
        .map(|view_id| {
            manifest
                .control_views
                .iter()
                .find(|view| view.view_id == view_id)
                .cloned()
                .ok_or_else(|| {
                    AppError::validation(format!("Control view '{}' was not found.", view_id))
                })
        })
        .transpose()?;
    let existing_views = manifest.control_views.clone();
    let max_order = existing_views
        .iter()
        .map(|view| view.order)
        .max()
        .unwrap_or(0);

    let (view_id, label, scope, part_ids, primitive_ids, is_default, order) = if base_view
        .as_ref()
        .is_some_and(|view| view.source == actor.source())
    {
        let base = base_view.as_ref().expect("checked base view");
        (
            base.view_id.clone(),
            base.label.clone(),
            base.scope.clone(),
            base.part_ids.clone(),
            dedupe(
                base.primitive_ids
                    .iter()
                    .cloned()
                    .chain(std::iter::once(primitive_id.to_string()))
                    .collect(),
            ),
            base.is_default,
            base.order,
        )
    } else {
        let label = base_view
            .as_ref()
            .map(|view| format!("{} Custom", view.label))
            .unwrap_or_else(|| match fallback_scope {
                ControlViewScope::Part => "Part Custom".to_string(),
                ControlViewScope::Global => "Custom".to_string(),
            });
        let scope = base_view
            .as_ref()
            .map(|view| view.scope.clone())
            .unwrap_or_else(|| fallback_scope.clone());
        let part_ids = base_view
            .as_ref()
            .map(|view| view.part_ids.clone())
            .unwrap_or_else(|| match fallback_scope {
                ControlViewScope::Part => {
                    fallback_part_id.map(str::to_string).into_iter().collect()
                }
                ControlViewScope::Global => Vec::new(),
            });
        let primitive_ids = dedupe(
            base_view
                .as_ref()
                .map(|view| view.primitive_ids.clone())
                .unwrap_or_default()
                .into_iter()
                .chain(std::iter::once(primitive_id.to_string()))
                .collect(),
        );
        (
            generated_id("view", actor, uuid::Uuid::new_v4()),
            label,
            scope,
            part_ids,
            primitive_ids,
            false,
            max_order + 1,
        )
    };

    let sections = infer_sections(&existing_views, base_view_id, &primitive_ids);
    let view = ControlView {
        view_id: view_id.clone(),
        label,
        scope,
        part_ids,
        primitive_ids,
        sections,
        is_default,
        source: actor.source(),
        status: if actor == SemanticEditActor::Manual {
            EnrichmentStatus::Accepted
        } else {
            EnrichmentStatus::None
        },
        order,
    };
    *manifest = upsert_control_view(manifest, view, actor)?;
    Ok(view_id)
}

fn infer_sections(
    views: &[ControlView],
    preferred_view_id: Option<&str>,
    primitive_ids: &[String],
) -> Vec<ControlViewSection> {
    #[derive(Clone)]
    struct Bucket {
        section_id: String,
        label: String,
        collapsed: bool,
        order: usize,
        primitive_ids: Vec<String>,
    }

    let mut ordered_views = Vec::new();
    if let Some(preferred_view_id) = preferred_view_id {
        if let Some(view) = views.iter().find(|view| view.view_id == preferred_view_id) {
            ordered_views.push(view);
        }
    }
    ordered_views.extend(
        views
            .iter()
            .filter(|view| Some(view.view_id.as_str()) != preferred_view_id),
    );

    let mut buckets: Vec<Bucket> = Vec::new();
    for primitive_id in primitive_ids {
        let matched = ordered_views.iter().find_map(|view| {
            view.sections
                .iter()
                .enumerate()
                .find(|(_, section)| section.primitive_ids.contains(primitive_id))
                .map(|(order, section)| (section, order))
        });
        let (section_id, label, collapsed, order) = matched
            .map(|(section, order)| {
                (
                    section.section_id.clone(),
                    section.label.clone(),
                    section.collapsed,
                    order,
                )
            })
            .unwrap_or_else(|| ("main".to_string(), "Main".to_string(), false, 0));
        if let Some(bucket) = buckets
            .iter_mut()
            .find(|bucket| bucket.section_id == section_id)
        {
            bucket.primitive_ids.push(primitive_id.clone());
        } else {
            buckets.push(Bucket {
                section_id,
                label,
                collapsed,
                order,
                primitive_ids: vec![primitive_id.clone()],
            });
        }
    }
    buckets.sort_by(|left, right| {
        left.order
            .cmp(&right.order)
            .then_with(|| left.label.cmp(&right.label))
    });
    buckets
        .into_iter()
        .map(|bucket| ControlViewSection {
            section_id: bucket.section_id,
            label: bucket.label,
            primitive_ids: bucket.primitive_ids,
            collapsed: bucket.collapsed,
        })
        .collect()
}

fn apply_proposal_statuses(
    manifest: &ModelManifest,
    entries: Vec<ProposalStatusEdit>,
) -> AppResult<SemanticManifestEditResult> {
    if manifest.source_kind != ModelSourceKind::ImportedFcstd {
        return Err(AppError::validation(
            "Enrichment proposal status edits require an imported FCStd manifest.",
        ));
    }
    if entries.is_empty() {
        return Err(AppError::validation(
            "Enrichment proposal status batch must include at least one entry.",
        ));
    }
    let known_proposal_ids = manifest
        .enrichment_state
        .proposals
        .iter()
        .map(|proposal| proposal.proposal_id.as_str())
        .collect::<std::collections::HashSet<_>>();
    let mut seen = std::collections::HashSet::new();
    let mut normalized = Vec::with_capacity(entries.len());
    for entry in entries {
        let proposal_id = require_trimmed(&entry.proposal_id, "Proposal id")?;
        if entry.status == EnrichmentStatus::None {
            return Err(AppError::validation(format!(
                "Enrichment proposal '{}' status must be pending, accepted, or rejected.",
                proposal_id
            )));
        }
        if !seen.insert(proposal_id.clone()) {
            return Err(AppError::validation(format!(
                "Enrichment proposal status batch contains duplicate proposalId '{}'.",
                proposal_id
            )));
        }
        if !known_proposal_ids.contains(proposal_id.as_str()) {
            return Err(AppError::validation(format!(
                "Enrichment proposal '{}' was not found.",
                proposal_id
            )));
        }
        normalized.push(ProposalStatusEdit {
            proposal_id,
            status: entry.status,
        });
    }

    let mut next = manifest.clone();
    for proposal in &mut next.enrichment_state.proposals {
        if let Some(entry) = normalized
            .iter()
            .find(|entry| entry.proposal_id == proposal.proposal_id)
        {
            proposal.status = entry.status.clone();
        }
    }
    next.enrichment_state.status = derive_enrichment_status(&next.enrichment_state.proposals);

    let auto_group_ids = next
        .parameter_groups
        .iter()
        .filter(|group| group.group_id.starts_with("proposal-bind-"))
        .map(|group| group.group_id.clone())
        .collect::<std::collections::HashSet<_>>();
    let mut auto_keys_by_part = std::collections::HashMap::<String, Vec<String>>::new();
    for group in &next.parameter_groups {
        if !auto_group_ids.contains(&group.group_id) {
            continue;
        }
        for part_id in &group.part_ids {
            let bucket = auto_keys_by_part.entry(part_id.clone()).or_default();
            for key in &group.parameter_keys {
                if !bucket.contains(key) {
                    bucket.push(key.clone());
                }
            }
        }
    }
    let accepted = next
        .enrichment_state
        .proposals
        .iter()
        .filter(|proposal| proposal.status == EnrichmentStatus::Accepted)
        .cloned()
        .collect::<Vec<_>>();
    let mut accepted_keys_by_part = std::collections::HashMap::<String, Vec<String>>::new();
    for proposal in &accepted {
        for part_id in &proposal.part_ids {
            let bucket = accepted_keys_by_part.entry(part_id.clone()).or_default();
            for key in &proposal.parameter_keys {
                if !bucket.contains(key) {
                    bucket.push(key.clone());
                }
            }
        }
    }
    for part in &mut next.parts {
        let auto_keys = auto_keys_by_part.get(&part.part_id);
        let mut parameter_keys = part
            .parameter_keys
            .iter()
            .filter(|key| !auto_keys.is_some_and(|keys| keys.contains(key)))
            .cloned()
            .collect::<Vec<_>>();
        for key in accepted_keys_by_part
            .get(&part.part_id)
            .into_iter()
            .flatten()
        {
            if !parameter_keys.contains(key) {
                parameter_keys.push(key.clone());
            }
        }
        part.parameter_keys = parameter_keys;
        part.editable = !part.parameter_keys.is_empty();
    }
    next.parameter_groups
        .retain(|group| !group.group_id.starts_with("proposal-bind-"));
    next.parameter_groups
        .extend(accepted.iter().map(|proposal| ParameterGroup {
            group_id: format!("proposal-bind-{}", proposal.proposal_id),
            label: proposal.label.clone(),
            parameter_keys: dedupe(proposal.parameter_keys.clone()),
            part_ids: dedupe(proposal.part_ids.clone()),
            editable: true,
            presentation: None,
            order: None,
        }));
    let editable_part_ids = next
        .parts
        .iter()
        .filter(|part| part.editable)
        .map(|part| part.part_id.as_str())
        .collect::<std::collections::HashSet<_>>();
    for target in &mut next.selection_targets {
        target.editable = editable_part_ids.contains(target.part_id.as_str());
    }
    next.warnings.retain(|warning| {
        warning != "Imported FCStd models are inspect-only until bindings are confirmed."
            && warning != "Imported FCStd bindings were accepted from heuristic proposals."
    });
    if accepted.is_empty() {
        next.warnings.push(
            "Imported FCStd models are inspect-only until bindings are confirmed.".to_string(),
        );
    } else {
        next.warnings
            .push("Imported FCStd bindings were accepted from heuristic proposals.".to_string());
    }
    next.validate()?;
    Ok(SemanticManifestEditResult {
        manifest: next,
        edited_id: normalized
            .iter()
            .map(|entry| entry.proposal_id.as_str())
            .collect::<Vec<_>>()
            .join(","),
        selected_view_id: None,
    })
}

fn derive_enrichment_status(
    proposals: &[crate::contracts::EnrichmentProposal],
) -> EnrichmentStatus {
    if proposals
        .iter()
        .any(|proposal| proposal.status == EnrichmentStatus::Pending)
    {
        EnrichmentStatus::Pending
    } else if proposals
        .iter()
        .any(|proposal| proposal.status == EnrichmentStatus::Accepted)
    {
        EnrichmentStatus::Accepted
    } else if proposals
        .iter()
        .any(|proposal| proposal.status == EnrichmentStatus::Rejected)
    {
        EnrichmentStatus::Rejected
    } else {
        EnrichmentStatus::None
    }
}

fn require_trimmed(value: &str, label: &str) -> AppResult<String> {
    let value = value.trim();
    if value.is_empty() {
        return Err(AppError::validation(format!("{} cannot be empty.", label)));
    }
    Ok(value.to_string())
}

fn generated_id(kind: &str, actor: SemanticEditActor, id: uuid::Uuid) -> String {
    format!("{}-{}-{}", kind, actor.id_segment(), id.simple())
}

fn ensure_prefixed_owner(kind: &str, id: &str, actor: SemanticEditActor) -> AppResult<()> {
    let expected = format!("{}-{}-", kind, actor.id_segment());
    if !id.starts_with(&expected) {
        return Err(AppError::validation(format!(
            "{} edit cannot delete non-{} {} '{}'.",
            actor.id_segment(),
            actor.id_segment(),
            kind,
            id
        )));
    }
    Ok(())
}

fn dedupe(values: Vec<String>) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    values
        .into_iter()
        .filter(|value| seen.insert(value.clone()))
        .collect()
}

fn sort_primitives(primitives: &mut [ControlPrimitive]) {
    primitives.sort_by(|left, right| {
        left.order
            .cmp(&right.order)
            .then_with(|| left.label.cmp(&right.label))
    });
}

fn ensure_persisted_semantic_edits_supported(manifest: &ModelManifest) -> AppResult<()> {
    if manifest.source_language == SourceLanguage::EckyIrV0 {
        return Err(AppError::validation(
            "Ecky-native models derive semantic controls from AST provenance; persisted semantic edits are unsupported.",
        ));
    }
    Ok(())
}

pub fn upsert_control_view(
    manifest: &ModelManifest,
    view: ControlView,
    actor: SemanticEditActor,
) -> AppResult<ModelManifest> {
    ensure_persisted_views_supported(manifest)?;

    let view_id = view.view_id.trim();
    if view_id.is_empty() {
        return Err(AppError::validation("View id cannot be empty."));
    }
    let label = view.label.trim();
    if label.is_empty() {
        return Err(AppError::validation("View label cannot be empty."));
    }

    let existing = manifest
        .control_views
        .iter()
        .find(|entry| entry.view_id == view_id);
    if actor == SemanticEditActor::Manual
        && existing.is_some_and(|entry| entry.source != ControlViewSource::Manual)
    {
        return Err(AppError::validation(format!(
            "Manual view edit cannot replace non-manual view '{}'.",
            view_id
        )));
    }

    let order = if view.order == 0 {
        existing.map(|entry| entry.order).unwrap_or_else(|| {
            manifest
                .control_views
                .iter()
                .map(|entry| entry.order)
                .max()
                .unwrap_or(0)
                + 1
        })
    } else {
        view.order
    };
    let status = if actor == SemanticEditActor::Manual {
        EnrichmentStatus::Accepted
    } else {
        view.status
    };
    let next_view = ControlView {
        view_id: view_id.to_string(),
        label: label.to_string(),
        scope: view.scope,
        part_ids: view.part_ids,
        primitive_ids: view.primitive_ids,
        sections: view.sections,
        is_default: view.is_default,
        source: actor.source(),
        status,
        order,
    };

    let mut next = manifest.clone();
    next.control_views.retain(|entry| entry.view_id != view_id);
    next.control_views.push(next_view);
    next.control_views.sort_by(|left, right| {
        left.order
            .cmp(&right.order)
            .then_with(|| left.label.cmp(&right.label))
    });
    next.validate()?;
    Ok(next)
}

pub fn delete_control_view(
    manifest: &ModelManifest,
    view_id: &str,
    actor: SemanticEditActor,
) -> AppResult<ModelManifest> {
    ensure_persisted_views_supported(manifest)?;
    let view_id = view_id.trim();
    if view_id.is_empty() {
        return Err(AppError::validation("View id cannot be empty."));
    }
    let existing = manifest
        .control_views
        .iter()
        .find(|entry| entry.view_id == view_id)
        .ok_or_else(|| {
            AppError::validation(format!("Control view '{}' was not found.", view_id))
        })?;
    if actor == SemanticEditActor::Manual && existing.source != ControlViewSource::Manual {
        return Err(AppError::validation(format!(
            "Manual view edit cannot delete non-manual view '{}'.",
            view_id
        )));
    }

    let mut next = manifest.clone();
    next.control_views.retain(|entry| entry.view_id != view_id);
    for advisory in &mut next.advisories {
        advisory.view_ids.retain(|entry| entry != view_id);
    }
    next.validate()?;
    Ok(next)
}

fn ensure_persisted_views_supported(manifest: &ModelManifest) -> AppResult<()> {
    if manifest.source_language == SourceLanguage::EckyIrV0 {
        return Err(AppError::validation(
            "Ecky-native models derive controls from AST provenance; persisted control views are unsupported.",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::{ControlViewScope, EnrichmentStatus};

    fn manifest(source_language: &str) -> ModelManifest {
        serde_json::from_value(serde_json::json!({
            "modelId": "model-1",
            "sourceKind": "importedFcstd",
            "sourceLanguage": source_language,
            "document": {
                "documentName": "Model",
                "documentLabel": "Model",
                "objectCount": 1,
                "warnings": []
            },
            "parts": [{
                "partId": "body",
                "freecadObjectName": "Body",
                "label": "Body",
                "kind": "solid",
                "viewerNodeIds": ["body-node"],
                "parameterKeys": ["width"],
                "editable": true
            }],
            "parameterGroups": [{
                "groupId": "body-params",
                "label": "Body",
                "parameterKeys": ["width"],
                "partIds": ["body"],
                "editable": true
            }],
            "controlPrimitives": [{
                "primitiveId": "width-knob",
                "label": "Width",
                "kind": "number",
                "source": "generated",
                "partIds": ["body"],
                "bindings": [{ "parameterKey": "width" }],
                "editable": true,
                "order": 1
            }],
            "controlViews": [{
                "viewId": "generated-view",
                "label": "Generated",
                "scope": "part",
                "partIds": ["body"],
                "primitiveIds": ["width-knob"],
                "sections": [],
                "default": true,
                "source": "generated",
                "status": "accepted",
                "order": 1
            }],
            "advisories": [{
                "advisoryId": "view-note",
                "label": "Note",
                "severity": "info",
                "primitiveIds": [],
                "viewIds": ["manual-view"],
                "message": "Keep fit.",
                "condition": "always"
            }]
        }))
        .expect("manifest")
    }

    fn manual_view() -> ControlView {
        ControlView {
            view_id: " manual-view ".to_string(),
            label: " Outer fit ".to_string(),
            scope: ControlViewScope::Part,
            part_ids: vec!["body".to_string()],
            primitive_ids: vec!["width-knob".to_string()],
            sections: Vec::new(),
            is_default: false,
            source: ControlViewSource::Generated,
            status: EnrichmentStatus::None,
            order: 0,
        }
    }

    fn valid_manifest(source_language: &str) -> ModelManifest {
        let mut manifest = manifest(source_language);
        manifest.advisories[0].view_ids.clear();
        manifest
    }

    #[test]
    fn manual_upsert_owns_source_order_and_validation() {
        let next = upsert_control_view(
            &manifest("legacyPython"),
            manual_view(),
            SemanticEditActor::Manual,
        )
        .expect("manual view");

        let saved = next
            .control_views
            .iter()
            .find(|view| view.view_id == "manual-view")
            .expect("saved view");
        assert_eq!(saved.label, "Outer fit");
        assert_eq!(saved.source, ControlViewSource::Manual);
        assert_eq!(saved.status, EnrichmentStatus::Accepted);
        assert_eq!(saved.order, 2);
    }

    #[test]
    fn llm_upsert_uses_same_rules_with_llm_source() {
        let next = upsert_control_view(
            &manifest("legacyPython"),
            manual_view(),
            SemanticEditActor::Llm,
        )
        .expect("LLM view");

        let saved = next
            .control_views
            .iter()
            .find(|view| view.view_id == "manual-view")
            .expect("saved view");
        assert_eq!(saved.source, ControlViewSource::Llm);
        assert_eq!(saved.status, EnrichmentStatus::None);
        assert_eq!(saved.order, 2);
    }

    #[test]
    fn invalid_primitive_reference_is_rejected() {
        let mut view = manual_view();
        view.primitive_ids = vec!["missing-knob".to_string()];

        let error = upsert_control_view(&manifest("legacyPython"), view, SemanticEditActor::Manual)
            .expect_err("invalid reference");
        assert!(error.message.contains("missing-knob"));
    }

    #[test]
    fn delete_removes_advisory_reference() {
        let with_manual = upsert_control_view(
            &manifest("legacyPython"),
            manual_view(),
            SemanticEditActor::Manual,
        )
        .expect("manual view");
        let next = delete_control_view(&with_manual, "manual-view", SemanticEditActor::Manual)
            .expect("delete");

        assert!(next
            .control_views
            .iter()
            .all(|view| view.view_id != "manual-view"));
        assert!(next.advisories[0].view_ids.is_empty());
    }

    #[test]
    fn ecky_native_view_persistence_is_rejected() {
        let error =
            upsert_control_view(&manifest("ecky"), manual_view(), SemanticEditActor::Manual)
                .expect_err("Ecky views forbidden");
        assert!(error.message.contains("Ecky-native"));
    }

    #[test]
    fn manual_primitive_save_owns_identity_source_order_and_attached_view() {
        let next = apply_semantic_manifest_edit(
            &valid_manifest("legacyPython"),
            SemanticManifestEditIntent::SavePrimitive {
                primitive_id: None,
                label: " Depth knob ".to_string(),
                primitive_kind: crate::contracts::ControlPrimitiveKind::Number,
                scope: ControlViewScope::Part,
                part_id: Some("body".to_string()),
                bindings: vec![crate::contracts::PrimitiveBinding {
                    parameter_key: "depth".to_string(),
                    scale: 1.0,
                    offset: 0.0,
                    min: Some(1.0),
                    max: Some(20.0),
                }],
                attach_to_view: true,
                base_view_id: Some("generated-view".to_string()),
            },
            SemanticEditActor::Manual,
        )
        .expect("save primitive");

        let saved = next
            .manifest
            .control_primitives
            .iter()
            .find(|primitive| primitive.primitive_id == next.edited_id)
            .expect("saved primitive");
        assert!(saved.primitive_id.starts_with("primitive-manual-"));
        assert_eq!(saved.label, "Depth knob");
        assert_eq!(saved.source, ControlViewSource::Manual);
        assert_eq!(saved.order, 2);
        let selected_view_id = next.selected_view_id.expect("selected manual view");
        let attached = next
            .manifest
            .control_views
            .iter()
            .find(|view| view.view_id == selected_view_id)
            .expect("attached manual view");
        assert_eq!(attached.source, ControlViewSource::Manual);
        assert!(attached.primitive_ids.contains(&saved.primitive_id));
    }

    #[test]
    fn tagged_view_save_owns_identity_and_delete_cleans_references() {
        let saved = apply_semantic_manifest_edit(
            &valid_manifest("legacyPython"),
            SemanticManifestEditIntent::SaveView {
                view_id: None,
                label: " Outer Fit ".into(),
                scope: ControlViewScope::Part,
                part_ids: vec!["body".into()],
                primitive_ids: vec!["width-knob".into()],
                sections: Vec::new(),
                is_default: false,
            },
            SemanticEditActor::Manual,
        )
        .unwrap();
        assert!(saved.edited_id.starts_with("view-manual-"));
        let view = saved
            .manifest
            .control_views
            .iter()
            .find(|view| view.view_id == saved.edited_id)
            .unwrap();
        assert_eq!(view.label, "Outer Fit");
        assert_eq!(view.source, ControlViewSource::Manual);

        let deleted = apply_semantic_manifest_edit(
            &saved.manifest,
            SemanticManifestEditIntent::DeleteView {
                view_id: saved.edited_id.clone(),
            },
            SemanticEditActor::Manual,
        )
        .unwrap();
        assert!(deleted
            .manifest
            .control_views
            .iter()
            .all(|view| view.view_id != saved.edited_id));
    }

    #[test]
    fn manual_primitive_delete_cleans_references_and_rejects_generated_owner() {
        let created = apply_semantic_manifest_edit(
            &valid_manifest("legacyPython"),
            SemanticManifestEditIntent::SavePrimitive {
                primitive_id: None,
                label: "Depth".to_string(),
                primitive_kind: crate::contracts::ControlPrimitiveKind::Number,
                scope: ControlViewScope::Part,
                part_id: Some("body".to_string()),
                bindings: vec![crate::contracts::PrimitiveBinding {
                    parameter_key: "depth".to_string(),
                    scale: 1.0,
                    offset: 0.0,
                    min: None,
                    max: None,
                }],
                attach_to_view: true,
                base_view_id: Some("generated-view".to_string()),
            },
            SemanticEditActor::Manual,
        )
        .expect("create primitive");
        let primitive_id = created.edited_id.clone();
        let mut with_refs = created.manifest;
        with_refs
            .control_relations
            .push(crate::contracts::ControlRelation {
                relation_id: "relation-manual-test".to_string(),
                source_primitive_id: "width-knob".to_string(),
                target_primitive_id: primitive_id.clone(),
                mode: crate::contracts::ControlRelationMode::Mirror,
                scale: 1.0,
                offset: 0.0,
                enabled: true,
            });
        with_refs.advisories.push(crate::contracts::Advisory {
            advisory_id: "advisory-manual-test".to_string(),
            label: "Depth warning".to_string(),
            severity: crate::contracts::AdvisorySeverity::Warning,
            primitive_ids: vec![primitive_id.clone()],
            view_ids: Vec::new(),
            message: "Depth matters".to_string(),
            condition: crate::contracts::AdvisoryCondition::Always,
            threshold: None,
        });

        let deleted = apply_semantic_manifest_edit(
            &with_refs,
            SemanticManifestEditIntent::DeletePrimitive {
                primitive_id: primitive_id.clone(),
            },
            SemanticEditActor::Manual,
        )
        .expect("delete primitive");
        assert!(deleted
            .manifest
            .control_primitives
            .iter()
            .all(|primitive| primitive.primitive_id != primitive_id));
        assert!(deleted.manifest.control_relations.is_empty());
        assert!(deleted
            .manifest
            .advisories
            .iter()
            .all(|advisory| { !advisory.primitive_ids.contains(&primitive_id) }));

        let error = apply_semantic_manifest_edit(
            &valid_manifest("legacyPython"),
            SemanticManifestEditIntent::DeletePrimitive {
                primitive_id: "width-knob".to_string(),
            },
            SemanticEditActor::Manual,
        )
        .expect_err("generated primitive protected");
        assert!(error.message.contains("non-manual"));
    }

    #[test]
    fn manual_advisory_and_relation_edits_own_ids_and_delete_scope() {
        let advisory = apply_semantic_manifest_edit(
            &valid_manifest("legacyPython"),
            SemanticManifestEditIntent::SaveAdvisory {
                label: " Width floor ".to_string(),
                severity: crate::contracts::AdvisorySeverity::Warning,
                primitive_ids: vec!["width-knob".to_string()],
                view_id: Some("generated-view".to_string()),
                message: " Keep width printable. ".to_string(),
                condition: crate::contracts::AdvisoryCondition::Below,
                threshold: Some(2.0),
            },
            SemanticEditActor::Manual,
        )
        .expect("save advisory");
        assert!(advisory.edited_id.starts_with("advisory-manual-"));

        let relation_manifest = apply_semantic_manifest_edit(
            &advisory.manifest,
            SemanticManifestEditIntent::SavePrimitive {
                primitive_id: None,
                label: "Depth".to_string(),
                primitive_kind: crate::contracts::ControlPrimitiveKind::Number,
                scope: ControlViewScope::Part,
                part_id: Some("body".to_string()),
                bindings: vec![crate::contracts::PrimitiveBinding {
                    parameter_key: "depth".to_string(),
                    scale: 1.0,
                    offset: 0.0,
                    min: None,
                    max: None,
                }],
                attach_to_view: false,
                base_view_id: None,
            },
            SemanticEditActor::Manual,
        )
        .expect("save relation target");
        let target_id = relation_manifest.edited_id.clone();
        let relation = apply_semantic_manifest_edit(
            &relation_manifest.manifest,
            SemanticManifestEditIntent::SaveRelation {
                source_primitive_id: "width-knob".to_string(),
                target_primitive_id: target_id,
                mode: crate::contracts::ControlRelationMode::Scale,
                scale: 2.0,
                offset: 0.0,
            },
            SemanticEditActor::Manual,
        )
        .expect("save relation");
        assert!(relation.edited_id.starts_with("relation-manual-"));

        let error = apply_semantic_manifest_edit(
            &relation.manifest,
            SemanticManifestEditIntent::DeleteAdvisory {
                advisory_id: "view-note".to_string(),
            },
            SemanticEditActor::Manual,
        )
        .expect_err("generated advisory protected");
        assert!(error.message.contains("non-manual"));
    }

    #[test]
    fn proposal_status_rebuilds_imported_bindings_in_rust() {
        let mut imported = valid_manifest("legacyPython");
        imported.enrichment_state.proposals = vec![crate::contracts::EnrichmentProposal {
            proposal_id: "proposal-depth".to_string(),
            label: "Depth controls".to_string(),
            part_ids: vec!["body".to_string()],
            parameter_keys: vec!["depth".to_string()],
            confidence: Some(0.9),
            status: EnrichmentStatus::Pending,
            provenance: "heuristic".to_string(),
        }];

        let next = apply_semantic_manifest_edit(
            &imported,
            SemanticManifestEditIntent::SetProposalStatus {
                proposal_id: "proposal-depth".to_string(),
                status: EnrichmentStatus::Accepted,
            },
            SemanticEditActor::Manual,
        )
        .expect("accept proposal");

        assert_eq!(
            next.manifest.enrichment_state.status,
            EnrichmentStatus::Accepted
        );
        assert!(next.manifest.parameter_groups.iter().any(|group| {
            group.group_id == "proposal-bind-proposal-depth"
                && group.parameter_keys == vec!["depth".to_string()]
        }));
        assert!(next.manifest.parts[0]
            .parameter_keys
            .contains(&"depth".to_string()));
        assert!(next.manifest.warnings.iter().any(|warning| {
            warning == "Imported FCStd bindings were accepted from heuristic proposals."
        }));
    }

    #[test]
    fn ecky_native_manual_semantic_edit_is_rejected() {
        let error = apply_semantic_manifest_edit(
            &valid_manifest("ecky"),
            SemanticManifestEditIntent::DeleteRelation {
                relation_id: "relation-manual-missing".to_string(),
            },
            SemanticEditActor::Manual,
        )
        .expect_err("Ecky semantic edits forbidden");
        assert!(error.message.contains("Ecky-native"));
    }

    #[test]
    fn semantic_edit_boundary_serializes_camel_case_tagged_intent() {
        let value = serde_json::to_value(SemanticManifestEditIntent::SavePrimitive {
            primitive_id: Some("primitive-manual-existing".to_string()),
            label: "Depth".to_string(),
            primitive_kind: crate::contracts::ControlPrimitiveKind::Number,
            scope: ControlViewScope::Part,
            part_id: Some("body".to_string()),
            bindings: vec![crate::contracts::PrimitiveBinding {
                parameter_key: "depth".to_string(),
                scale: 1.0,
                offset: 0.0,
                min: None,
                max: None,
            }],
            attach_to_view: true,
            base_view_id: Some("generated-view".to_string()),
        })
        .expect("serialize intent");

        assert_eq!(value["action"], "savePrimitive");
        assert_eq!(value["primitiveId"], "primitive-manual-existing");
        assert_eq!(value["primitiveKind"], "number");
        assert_eq!(value["attachToView"], true);
        assert!(value.get("primitive_id").is_none());
        assert!(value.get("attach_to_view").is_none());
    }

    #[test]
    fn proposal_status_batch_applies_all_entries_before_one_binding_rebuild() {
        let mut imported = valid_manifest("legacyPython");
        imported.enrichment_state.proposals = vec![
            crate::contracts::EnrichmentProposal {
                proposal_id: "proposal-width".to_string(),
                label: "Width controls".to_string(),
                part_ids: vec!["body".to_string()],
                parameter_keys: vec!["width".to_string()],
                confidence: Some(0.9),
                status: EnrichmentStatus::Pending,
                provenance: "heuristic".to_string(),
            },
            crate::contracts::EnrichmentProposal {
                proposal_id: "proposal-depth".to_string(),
                label: "Depth controls".to_string(),
                part_ids: vec!["body".to_string()],
                parameter_keys: vec!["depth".to_string()],
                confidence: Some(0.8),
                status: EnrichmentStatus::Pending,
                provenance: "heuristic".to_string(),
            },
        ];

        let next = apply_semantic_manifest_edit(
            &imported,
            SemanticManifestEditIntent::SetProposalStatuses {
                entries: vec![
                    crate::contracts::ProposalStatusEdit {
                        proposal_id: "proposal-width".to_string(),
                        status: EnrichmentStatus::Rejected,
                    },
                    crate::contracts::ProposalStatusEdit {
                        proposal_id: "proposal-depth".to_string(),
                        status: EnrichmentStatus::Accepted,
                    },
                ],
            },
            SemanticEditActor::Manual,
        )
        .expect("batch proposal statuses");

        assert_eq!(
            next.manifest.enrichment_state.status,
            EnrichmentStatus::Accepted
        );
        assert_eq!(next.edited_id, "proposal-width,proposal-depth");
        assert_eq!(
            next.manifest
                .enrichment_state
                .proposals
                .iter()
                .map(|proposal| proposal.status.clone())
                .collect::<Vec<_>>(),
            vec![EnrichmentStatus::Rejected, EnrichmentStatus::Accepted]
        );
        assert!(next.manifest.parameter_groups.iter().any(|group| {
            group.group_id == "proposal-bind-proposal-depth"
                && group.parameter_keys == vec!["depth".to_string()]
        }));
        assert!(next
            .manifest
            .parameter_groups
            .iter()
            .all(|group| group.group_id != "proposal-bind-proposal-width"));
    }

    #[test]
    fn proposal_status_batch_rejects_unknown_id_before_any_mutation() {
        let mut imported = valid_manifest("legacyPython");
        imported.enrichment_state.proposals = vec![crate::contracts::EnrichmentProposal {
            proposal_id: "proposal-width".to_string(),
            label: "Width controls".to_string(),
            part_ids: vec!["body".to_string()],
            parameter_keys: vec!["width".to_string()],
            confidence: Some(0.9),
            status: EnrichmentStatus::Pending,
            provenance: "heuristic".to_string(),
        }];
        let original = imported.clone();

        let error = apply_semantic_manifest_edit(
            &imported,
            SemanticManifestEditIntent::SetProposalStatuses {
                entries: vec![
                    crate::contracts::ProposalStatusEdit {
                        proposal_id: "proposal-width".to_string(),
                        status: EnrichmentStatus::Accepted,
                    },
                    crate::contracts::ProposalStatusEdit {
                        proposal_id: "proposal-missing".to_string(),
                        status: EnrichmentStatus::Rejected,
                    },
                ],
            },
            SemanticEditActor::Manual,
        )
        .expect_err("unknown batch entry");

        assert!(error.message.contains("proposal-missing"));
        assert_eq!(imported, original);
    }

    #[test]
    fn proposal_status_batch_boundary_is_camel_case() {
        let value = serde_json::to_value(SemanticManifestEditIntent::SetProposalStatuses {
            entries: vec![crate::contracts::ProposalStatusEdit {
                proposal_id: "proposal-width".to_string(),
                status: EnrichmentStatus::Accepted,
            }],
        })
        .expect("serialize batch");

        assert_eq!(value["action"], "setProposalStatuses");
        assert_eq!(value["entries"][0]["proposalId"], "proposal-width");
        assert_eq!(value["entries"][0]["status"], "accepted");
        assert!(value["entries"][0].get("proposal_id").is_none());
    }
}
