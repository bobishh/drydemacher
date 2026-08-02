//! Typed context envelope (OpenSpec `agent-context-budgeting`, section 2).
//!
//! Internal, intent-aware context assembly with explicit per-section and total
//! Unicode-character budgets. This module owns the *typed* budgeting decision
//! only; it deliberately does not format provider prose (that stays in
//! `context.rs` / `llm.rs`) and never calls a summarization model — projection
//! is pure rules over existing intent signals. Refactoring legacy formatting
//! behind this envelope (section 2.6) is out of scope here.
//!
//! Budgets are enforced as deterministic Unicode scalar-value counts
//! (`str::chars().count()`), not bytes and not provider tokens. Approximate
//! token telemetry is `ceil(chars / 4)` and is a metric only.

use serde::Serialize;

// ── Ceilings ────────────────────────────────────────────────────────────────

/// Default dynamic user-envelope ceiling for generation / design / repair /
/// question stages. Configurable here in one place; tuning waits on p50/p95
/// evidence (section 4.3).
pub const GENERATION_CEILING_CHARS: usize = 64_000;

/// Separate, compact ceiling for intent-classification projections.
pub const CLASSIFIER_CEILING_CHARS: usize = 8_000;

/// Per-section caps for optional/relevant sections. Applied as compaction
/// before the total envelope ceiling is evaluated.
pub const DIALOGUE_MAX_ITEMS: usize = 4;
pub const DIALOGUE_ITEM_CHAR_BUDGET: usize = 200;
pub const REFERENCE_MAX_ITEMS: usize = 2;
pub const REFERENCE_ITEM_CHAR_BUDGET: usize = 1_200;
pub const ASSET_MAX_ROWS: usize = 4;

/// Approximate-token metric divisor. Enforcement stays on deterministic char
/// counts; this is reported only as a telemetry estimate.
pub const CHARS_PER_APPROX_TOKEN: usize = 4;

/// `ceil(chars / CHARS_PER_APPROX_TOKEN)` — metric only, never an enforcement
/// boundary.
pub fn approx_tokens(chars: usize) -> usize {
    (chars + CHARS_PER_APPROX_TOKEN - 1) / CHARS_PER_APPROX_TOKEN
}

/// Deterministic Unicode scalar-value count. This is the enforcement unit.
pub fn measure_chars(content: &str) -> usize {
    content.chars().count()
}

// ── Stable section ids ─────────────────────────────────────────────────────

/// Well-known stable section identifiers. Projections and tests reference these
/// so that telemetry and budget decisions stay keyed by stable strings.
pub mod section_id {
    pub const REQUEST: &str = "request";
    pub const AUTHORING_CONTEXT: &str = "authoring-context";
    pub const CURRENT_SOURCE: &str = "current-source";
    pub const CURRENT_PARAMS: &str = "current-params";
    pub const REPAIR_DIAGNOSTIC: &str = "repair-diagnostic";
    pub const DESIGN_DIGEST: &str = "design-digest";
    pub const ARTIFACT_DIGEST: &str = "artifact-digest";
    pub const RELEVANT_SUMMARY: &str = "relevant-summary";
    pub const FRONTEND_SNAPSHOT: &str = "frontend-snapshot";

    pub fn dialogue(index: usize) -> String {
        format!("dialogue-{index}")
    }
    pub fn reference(index: usize) -> String {
        format!("reference-{index}")
    }
    pub fn asset(index: usize) -> String {
        format!("asset-{index}")
    }
}

// ── Enums ──────────────────────────────────────────────────────────────────

/// Which stage the envelope feeds. Each stage carries its own ceiling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum EnvelopeStage {
    Generation,
    Classifier,
}

impl EnvelopeStage {
    pub fn ceiling_chars(self) -> usize {
        match self {
            EnvelopeStage::Generation => GENERATION_CEILING_CHARS,
            EnvelopeStage::Classifier => CLASSIFIER_CEILING_CHARS,
        }
    }
}

/// Four priority tiers, ordered low → high for eviction.
///
/// Eviction trims the lowest-priority truncatable sections first
/// (`Optional` before `Relevant`). `Authoritative` and `Mandatory` sections are
/// never silently truncated: if their combined measured size exceeds the
/// ceiling, [`assemble_envelope`] returns a [`ContextBudgetError`] instead of a
/// lossy envelope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SectionPriority {
    Optional,
    Relevant,
    Authoritative,
    Mandatory,
}

impl SectionPriority {
    /// Sections that may be compacted or omitted to fit the envelope.
    pub fn is_truncatable(self) -> bool {
        matches!(self, SectionPriority::Optional | SectionPriority::Relevant)
    }

    /// Sections that must remain exact; overflow is an error, never silent loss.
    pub fn is_exact(self) -> bool {
        matches!(
            self,
            SectionPriority::Authoritative | SectionPriority::Mandatory
        )
    }
}

/// Telemetry redaction class. The envelope records shape (ids, counts, sizes)
/// regardless; this class marks whether a section's *content* is safe to emit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Sensitivity {
    /// Shape only — ids, counts, sizes. Safe to emit in telemetry.
    Safe,
    /// Content — source, params, references, prompt. Never logged.
    Sensitive,
}

/// Per-section budgeting outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum InclusionDecision {
    /// Kept in full (observed == returned).
    Included,
    /// Dropped entirely to fit the envelope (truncatable sections only).
    Omitted,
    /// Compacted to fewer characters.
    Truncated,
}

/// Why a section was compacted or dropped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SectionReason {
    /// Hit the section's own per-section cap during compaction.
    PerSectionBudget,
    /// Hit the total envelope ceiling during allocation.
    EnvelopeBudget,
}

// ── Candidate input ────────────────────────────────────────────────────────

/// A candidate context section submitted for budgeting. Internal input only —
/// not serialized. The measured size is derived deterministically from
/// `content` as a Unicode scalar-value count.
#[derive(Debug, Clone)]
pub struct ContextSection {
    pub id: String,
    pub priority: SectionPriority,
    pub sensitivity: Sensitivity,
    /// Per-section cap. For truncatable sections this compacts content before
    /// the envelope ceiling is evaluated. `None` = no per-section cap.
    pub budget_chars: Option<usize>,
    pub content: String,
}

impl ContextSection {
    pub fn new(
        id: impl Into<String>,
        priority: SectionPriority,
        sensitivity: Sensitivity,
        content: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            priority,
            sensitivity,
            budget_chars: None,
            content: content.into(),
        }
    }

    pub fn with_budget(mut self, budget_chars: usize) -> Self {
        self.budget_chars = Some(budget_chars);
        self
    }
}

// ── Budgeted output ───────────────────────────────────────────────────────

/// Shape-only record of one section after budgeting. Content is intentionally
/// absent so that serializing the envelope (for telemetry/boundary snapshots)
/// can never leak source, params, references, or prompt text.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ContextSectionRecord {
    pub id: String,
    pub priority: SectionPriority,
    pub sensitivity: Sensitivity,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub budget_chars: Option<usize>,
    pub observed_chars: usize,
    pub returned_chars: usize,
    pub approx_observed_tokens: usize,
    pub approx_returned_tokens: usize,
    pub decision: InclusionDecision,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<SectionReason>,
}

/// The assembled, budgeted envelope. Shape only — no content.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ContextEnvelope {
    pub stage: EnvelopeStage,
    pub ceiling_chars: usize,
    pub records: Vec<ContextSectionRecord>,
    pub total_observed_chars: usize,
    pub total_returned_chars: usize,
    pub total_approx_returned_tokens: usize,
}

impl ContextEnvelope {
    pub fn record(&self, id: &str) -> Option<&ContextSectionRecord> {
        self.records.iter().find(|r| r.id == id)
    }

    /// `true` when every exact (authoritative/mandatory) section was included
    /// verbatim — i.e. no authoritative current state was silently lost.
    pub fn exact_state_is_intact(&self) -> bool {
        self.records
            .iter()
            .filter(|r| r.priority.is_exact())
            .all(|r| {
                r.decision == InclusionDecision::Included && r.returned_chars == r.observed_chars
            })
    }
}

// ── Safe context telemetry (section 4) ─────────────────────────────────────
//
// OpenSpec `agent-context-budgeting`, decision 5 + spec requirement "Request-
// size telemetry is useful and content-free". These types are derived from
// the shape-only `ContextEnvelope` plus optional numeric provider usage, so
// serializing them can never leak prompt text, source, reference bodies,
// image bytes, API keys, authorization headers, or full paths.

/// Numeric provider usage carried on telemetry. Mirrors the numeric fields of
/// `contracts::UsageSummary` (input/output/total/cached/reasoning tokens) with
/// no string content, so it is safe to serialize to the local profiler path.
/// Built from a `&UsageSummary` at the emission boundary.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TelemetryUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub total_tokens: u64,
    pub cached_input_tokens: u64,
    pub reasoning_tokens: u64,
}

/// Shape-only per-section telemetry. No content: only the stable id, priority,
/// sensitivity class, observed/returned char counts, approximate tokens, and
/// the inclusion decision/reason.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ContextTelemetrySection {
    pub id: String,
    pub priority: SectionPriority,
    pub sensitivity: Sensitivity,
    pub observed_chars: usize,
    pub returned_chars: usize,
    pub approx_observed_tokens: usize,
    pub approx_returned_tokens: usize,
    pub decision: InclusionDecision,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<SectionReason>,
}

/// Shape-only envelope telemetry. Content-free by construction: derived from a
/// `ContextEnvelope` (which carries no content) plus optional numeric provider
/// usage. Safe to serialize to the existing profiler/session-activity path.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ContextTelemetry {
    pub stage: EnvelopeStage,
    pub ceiling_chars: usize,
    pub total_observed_chars: usize,
    pub total_returned_chars: usize,
    pub total_approx_returned_tokens: usize,
    pub sections: Vec<ContextTelemetrySection>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<TelemetryUsage>,
}

/// Derive content-free telemetry from a budgeted `envelope` plus optional
/// numeric provider `usage`. The result carries section ids/counts/decisions,
/// total size, stage, and provider usage — and nothing else. It is the single
/// shape that the local profiler/session-activity path serializes.
pub fn envelope_telemetry(
    envelope: &ContextEnvelope,
    usage: Option<TelemetryUsage>,
) -> ContextTelemetry {
    let sections = envelope
        .records
        .iter()
        .map(|r| ContextTelemetrySection {
            id: r.id.clone(),
            priority: r.priority,
            sensitivity: r.sensitivity,
            observed_chars: r.observed_chars,
            returned_chars: r.returned_chars,
            approx_observed_tokens: r.approx_observed_tokens,
            approx_returned_tokens: r.approx_returned_tokens,
            decision: r.decision,
            reason: r.reason,
        })
        .collect();
    ContextTelemetry {
        stage: envelope.stage,
        ceiling_chars: envelope.ceiling_chars,
        total_observed_chars: envelope.total_observed_chars,
        total_returned_chars: envelope.total_returned_chars,
        total_approx_returned_tokens: envelope.total_approx_returned_tokens,
        sections,
        usage,
    }
}

/// Raw, structured context-budget error returned when exact (mandatory /
/// authoritative) sections cannot fit within the stage ceiling. Naming both
/// observed and allowed sizes keeps the failure actionable; the UI surfaces it
/// verbatim (no generic credential advice).
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ContextBudgetError {
    pub stage: EnvelopeStage,
    pub observed_chars: usize,
    pub allowed_chars: usize,
    pub overflow_sections: Vec<OverflowSection>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OverflowSection {
    pub id: String,
    pub priority: SectionPriority,
    pub observed_chars: usize,
}

impl std::fmt::Display for ContextBudgetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "context budget overflow on {:?} stage: observed {} chars, allowed {} chars",
            self.stage, self.observed_chars, self.allowed_chars
        )
    }
}

impl std::error::Error for ContextBudgetError {}

// ── Budget assembly ────────────────────────────────────────────────────────

/// Assemble `sections` under the stage's ceiling.
///
/// Algorithm:
/// 1. Measure each section by Unicode scalar count and apply per-section caps
///    (compaction) to truncatable sections.
/// 2. If exact (mandatory + authoritative) sections exceed the ceiling, return
///    a [`ContextBudgetError`] — never a lossy envelope.
/// 3. Allocate the remaining budget to truncatable sections in priority-desc /
///    insertion-asc order, so the lowest-priority sections are truncated or
///    omitted first.
pub fn assemble_envelope(
    stage: EnvelopeStage,
    sections: Vec<ContextSection>,
) -> Result<ContextEnvelope, ContextBudgetError> {
    let ceiling = stage.ceiling_chars();

    // Step 1: measure each section by Unicode scalar count and apply per-section
    // caps (compaction) to truncatable sections. Exact sections ignore caps.
    let mut records: Vec<ContextSectionRecord> = Vec::with_capacity(sections.len());
    for s in &sections {
        let observed = measure_chars(&s.content);
        let (returned, decision, reason) =
            if let Some(budget) = s.budget_chars.filter(|_| s.priority.is_truncatable()) {
                if observed > budget {
                    (
                        budget,
                        InclusionDecision::Truncated,
                        Some(SectionReason::PerSectionBudget),
                    )
                } else {
                    (observed, InclusionDecision::Included, None)
                }
            } else {
                (observed, InclusionDecision::Included, None)
            };
        records.push(ContextSectionRecord {
            id: s.id.clone(),
            priority: s.priority,
            sensitivity: s.sensitivity,
            budget_chars: s.budget_chars,
            observed_chars: observed,
            returned_chars: returned,
            approx_observed_tokens: approx_tokens(observed),
            approx_returned_tokens: approx_tokens(returned),
            decision,
            reason,
        });
    }

    // Step 2: exact (Mandatory + Authoritative) sections must all fit verbatim.
    // If they do not, fail before any lossy dispatch with observed/allowed sizes.
    let exact_returned: usize = records
        .iter()
        .filter(|r| r.priority.is_exact())
        .map(|r| r.returned_chars)
        .sum();
    if exact_returned > ceiling {
        let overflow_sections: Vec<OverflowSection> = records
            .iter()
            .filter(|r| r.priority.is_exact())
            .map(|r| OverflowSection {
                id: r.id.clone(),
                priority: r.priority,
                observed_chars: r.observed_chars,
            })
            .collect();
        return Err(ContextBudgetError {
            stage,
            observed_chars: exact_returned,
            allowed_chars: ceiling,
            overflow_sections,
        });
    }

    // Step 3: evict the lowest-priority truncatable sections first until the
    // remaining truncatable content (after per-section compaction) fits in the
    // budget left over after exact sections. Sections that survive eviction
    // keep their Step-1 decision (Included, or Truncated by per-section budget).
    // No partial truncation to fit the envelope: a section is either kept in its
    // compacted form or omitted entirely, which keeps the decision auditable.
    let remaining_budget = ceiling - exact_returned;
    let mut evict_order: Vec<usize> = records
        .iter()
        .enumerate()
        .filter(|(_, r)| r.priority.is_truncatable())
        .map(|(i, _)| i)
        .collect();
    // Lowest priority first; within a priority tier, latest-inserted first.
    evict_order.sort_by(|&a, &b| {
        records[a]
            .priority
            .cmp(&records[b].priority)
            .then_with(|| b.cmp(&a))
    });

    let mut evict_cursor = 0usize;
    loop {
        let trunc_total: usize = evict_order.iter().map(|&i| records[i].returned_chars).sum();
        if trunc_total <= remaining_budget {
            break;
        }
        // Eviction always terminates: evicting every truncatable section leaves
        // only the exact content, which Step 2 proved fits the ceiling.
        let Some(&idx) = evict_order.get(evict_cursor) else {
            break;
        };
        let rec = &mut records[idx];
        rec.returned_chars = 0;
        rec.approx_returned_tokens = 0;
        rec.decision = InclusionDecision::Omitted;
        rec.reason = Some(SectionReason::EnvelopeBudget);
        evict_cursor += 1;
    }

    let total_observed_chars: usize = records.iter().map(|r| r.observed_chars).sum();
    let total_returned_chars: usize = records.iter().map(|r| r.returned_chars).sum();
    Ok(ContextEnvelope {
        stage,
        ceiling_chars: ceiling,
        records,
        total_observed_chars,
        total_returned_chars,
        total_approx_returned_tokens: approx_tokens(total_returned_chars),
    })
}

// ── Intent projection ──────────────────────────────────────────────────────

/// Raw, intent-agnostic candidate inputs. Projection decides which of these
/// become sections and at which priority; the caller fills whichever fields it
/// has and leaves the rest at `None` / empty.
#[derive(Debug, Clone, Default)]
pub struct ContextInputs {
    pub request: String,
    pub authoring_context: Option<String>,
    pub source: Option<String>,
    pub params: Option<String>,
    pub diagnostic: Option<String>,
    pub digest: Option<String>,
    pub summary: Option<String>,
    pub frontend_snapshot: Option<String>,
    pub dialogue: Vec<String>,
    pub references: Vec<String>,
    pub assets: Vec<String>,
}

/// Deterministic projection intent. Carries the only two intent-derived flags
/// that affect section relevance, so projection stays a pure function of
/// `(intent, inputs)` with no model call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectionIntent {
    Design,
    Repair,
    Question {
        /// `true` for deterministic source/code/parameter questions.
        source_required: bool,
    },
    Classifier {
        /// `true` only when attachment/reference intent is present.
        include_references: bool,
    },
}

/// Deterministically project `inputs` into budgeted candidate sections for
/// `intent`, returning the stage (and thus the ceiling). No LLM summarization:
/// relevance is decided by `intent` plus the two flags it carries.
pub fn project_sections(
    intent: ProjectionIntent,
    inputs: &ContextInputs,
) -> (EnvelopeStage, Vec<ContextSection>) {
    let mut out: Vec<ContextSection> = Vec::new();

    match intent {
        ProjectionIntent::Design => {
            push_request(&mut out, inputs);
            push_opt(
                &mut out,
                inputs.authoring_context.as_deref(),
                section_id::AUTHORING_CONTEXT,
                SectionPriority::Mandatory,
                Sensitivity::Sensitive,
                None,
            );
            // Design edits need exact current source and params.
            push_opt(
                &mut out,
                inputs.source.as_deref(),
                section_id::CURRENT_SOURCE,
                SectionPriority::Authoritative,
                Sensitivity::Sensitive,
                None,
            );
            push_opt(
                &mut out,
                inputs.params.as_deref(),
                section_id::CURRENT_PARAMS,
                SectionPriority::Authoritative,
                Sensitivity::Sensitive,
                None,
            );
            push_opt(
                &mut out,
                inputs.digest.as_deref(),
                section_id::DESIGN_DIGEST,
                SectionPriority::Relevant,
                Sensitivity::Safe,
                None,
            );
            push_opt(
                &mut out,
                inputs.summary.as_deref(),
                section_id::RELEVANT_SUMMARY,
                SectionPriority::Relevant,
                Sensitivity::Sensitive,
                None,
            );
            push_capped_items(
                &mut out,
                &inputs.dialogue,
                DIALOGUE_MAX_ITEMS,
                Some(DIALOGUE_ITEM_CHAR_BUDGET),
                section_id::dialogue,
                SectionPriority::Optional,
                Sensitivity::Sensitive,
            );
            push_capped_items(
                &mut out,
                &inputs.references,
                REFERENCE_MAX_ITEMS,
                Some(REFERENCE_ITEM_CHAR_BUDGET),
                section_id::reference,
                SectionPriority::Optional,
                Sensitivity::Sensitive,
            );
            push_capped_items(
                &mut out,
                &inputs.assets,
                ASSET_MAX_ROWS,
                None,
                section_id::asset,
                SectionPriority::Optional,
                Sensitivity::Safe,
            );
        }
        ProjectionIntent::Repair => {
            push_request(&mut out, inputs);
            push_opt(
                &mut out,
                inputs.authoring_context.as_deref(),
                section_id::AUTHORING_CONTEXT,
                SectionPriority::Mandatory,
                Sensitivity::Sensitive,
                None,
            );
            // Repair needs exact source, params, and the latest raw diagnostic.
            push_opt(
                &mut out,
                inputs.source.as_deref(),
                section_id::CURRENT_SOURCE,
                SectionPriority::Authoritative,
                Sensitivity::Sensitive,
                None,
            );
            push_opt(
                &mut out,
                inputs.params.as_deref(),
                section_id::CURRENT_PARAMS,
                SectionPriority::Authoritative,
                Sensitivity::Sensitive,
                None,
            );
            push_opt(
                &mut out,
                inputs.diagnostic.as_deref(),
                section_id::REPAIR_DIAGNOSTIC,
                SectionPriority::Authoritative,
                Sensitivity::Sensitive,
                None,
            );
            push_opt(
                &mut out,
                inputs.digest.as_deref(),
                section_id::DESIGN_DIGEST,
                SectionPriority::Relevant,
                Sensitivity::Safe,
                None,
            );
            push_opt(
                &mut out,
                inputs.summary.as_deref(),
                section_id::RELEVANT_SUMMARY,
                SectionPriority::Relevant,
                Sensitivity::Sensitive,
                None,
            );
            // Unrelated history is deliberately excluded: dialogue, references,
            // and assets do not displace the repair diagnostic + exact source.
        }
        ProjectionIntent::Question { source_required } => {
            push_request(&mut out, inputs);
            push_opt(
                &mut out,
                inputs.authoring_context.as_deref(),
                section_id::AUTHORING_CONTEXT,
                SectionPriority::Mandatory,
                Sensitivity::Sensitive,
                None,
            );
            if source_required {
                push_opt(
                    &mut out,
                    inputs.source.as_deref(),
                    section_id::CURRENT_SOURCE,
                    SectionPriority::Authoritative,
                    Sensitivity::Sensitive,
                    None,
                );
                push_opt(
                    &mut out,
                    inputs.params.as_deref(),
                    section_id::CURRENT_PARAMS,
                    SectionPriority::Authoritative,
                    Sensitivity::Sensitive,
                    None,
                );
            }
            push_opt(
                &mut out,
                inputs.digest.as_deref(),
                section_id::DESIGN_DIGEST,
                SectionPriority::Relevant,
                Sensitivity::Safe,
                None,
            );
            push_opt(
                &mut out,
                inputs.summary.as_deref(),
                section_id::RELEVANT_SUMMARY,
                SectionPriority::Relevant,
                Sensitivity::Sensitive,
                None,
            );
            push_capped_items(
                &mut out,
                &inputs.dialogue,
                DIALOGUE_MAX_ITEMS,
                Some(DIALOGUE_ITEM_CHAR_BUDGET),
                section_id::dialogue,
                SectionPriority::Optional,
                Sensitivity::Sensitive,
            );
            push_capped_items(
                &mut out,
                &inputs.references,
                REFERENCE_MAX_ITEMS,
                Some(REFERENCE_ITEM_CHAR_BUDGET),
                section_id::reference,
                SectionPriority::Optional,
                Sensitivity::Sensitive,
            );
            push_capped_items(
                &mut out,
                &inputs.assets,
                ASSET_MAX_ROWS,
                None,
                section_id::asset,
                SectionPriority::Optional,
                Sensitivity::Safe,
            );
        }
        ProjectionIntent::Classifier { include_references } => {
            push_request(&mut out, inputs);
            push_opt(
                &mut out,
                inputs.digest.as_deref(),
                section_id::DESIGN_DIGEST,
                SectionPriority::Relevant,
                Sensitivity::Safe,
                None,
            );
            push_opt(
                &mut out,
                inputs.frontend_snapshot.as_deref(),
                section_id::FRONTEND_SNAPSHOT,
                SectionPriority::Relevant,
                Sensitivity::Sensitive,
                None,
            );
            // Latest dialogue turn only.
            if let Some(latest) = inputs.dialogue.last() {
                let section = ContextSection::new(
                    section_id::dialogue(0),
                    SectionPriority::Optional,
                    Sensitivity::Sensitive,
                    latest,
                )
                .with_budget(DIALOGUE_ITEM_CHAR_BUDGET);
                out.push(section);
            }
            if include_references {
                push_capped_items(
                    &mut out,
                    &inputs.references,
                    REFERENCE_MAX_ITEMS,
                    Some(REFERENCE_ITEM_CHAR_BUDGET),
                    section_id::reference,
                    SectionPriority::Optional,
                    Sensitivity::Sensitive,
                );
            }
            // Full source, params, and authoring policy never enter the compact
            // classifier projection.
        }
    }

    let stage = match intent {
        ProjectionIntent::Classifier { .. } => EnvelopeStage::Classifier,
        _ => EnvelopeStage::Generation,
    };
    (stage, out)
}

/// The actual user request is always present and mandatory.
fn push_request(out: &mut Vec<ContextSection>, inputs: &ContextInputs) {
    out.push(ContextSection::new(
        section_id::REQUEST,
        SectionPriority::Mandatory,
        Sensitivity::Sensitive,
        &inputs.request,
    ));
}

/// Push a single optional section only when its content is present.
fn push_opt(
    out: &mut Vec<ContextSection>,
    content: Option<&str>,
    id: &str,
    priority: SectionPriority,
    sensitivity: Sensitivity,
    budget: Option<usize>,
) {
    if let Some(content) = content {
        let mut section = ContextSection::new(id, priority, sensitivity, content);
        if let Some(b) = budget {
            section = section.with_budget(b);
        }
        out.push(section);
    }
}

/// Push up to `max_items` capped items from `items`, each with its own stable id
/// and per-section budget. Used for dialogue turns, references, and assets.
fn push_capped_items(
    out: &mut Vec<ContextSection>,
    items: &[String],
    max_items: usize,
    per_item_budget: Option<usize>,
    id: fn(usize) -> String,
    priority: SectionPriority,
    sensitivity: Sensitivity,
) {
    for (index, item) in items.iter().take(max_items).enumerate() {
        let mut section = ContextSection::new(id(index), priority, sensitivity, item);
        if let Some(b) = per_item_budget {
            section = section.with_budget(b);
        }
        out.push(section);
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn section(id: &str, priority: SectionPriority, content: &str) -> ContextSection {
        ContextSection::new(id, priority, Sensitivity::Sensitive, content)
    }

    // ── 2.1 section priorities ─────────────────────────────────────────────

    #[test]
    fn section_priority_orders_optional_before_relevant_before_exact() {
        assert!(SectionPriority::Optional < SectionPriority::Relevant);
        assert!(SectionPriority::Relevant < SectionPriority::Authoritative);
        assert!(SectionPriority::Authoritative < SectionPriority::Mandatory);
    }

    #[test]
    fn truncatable_and_exact_partition_the_priorities() {
        assert!(SectionPriority::Optional.is_truncatable());
        assert!(SectionPriority::Relevant.is_truncatable());
        assert!(!SectionPriority::Authoritative.is_truncatable());
        assert!(!SectionPriority::Mandatory.is_truncatable());

        assert!(!SectionPriority::Optional.is_exact());
        assert!(!SectionPriority::Relevant.is_exact());
        assert!(SectionPriority::Authoritative.is_exact());
        assert!(SectionPriority::Mandatory.is_exact());
    }

    #[test]
    fn default_ceilings_are_64k_generation_and_8k_classifier() {
        assert_eq!(GENERATION_CEILING_CHARS, 64_000);
        assert_eq!(CLASSIFIER_CEILING_CHARS, 8_000);
        assert_eq!(EnvelopeStage::Generation.ceiling_chars(), 64_000);
        assert_eq!(EnvelopeStage::Classifier.ceiling_chars(), 8_000);
    }

    // ── 2.1 Unicode character accounting ───────────────────────────────────

    #[test]
    fn measure_chars_counts_unicode_scalars_not_bytes() {
        // "café🎉中": 4 + 1 emoji + 1 CJK = 6 scalars, but 10 UTF-8 bytes.
        let s = "café🎉中";
        assert_eq!(measure_chars(s), 6);
        assert_ne!(measure_chars(s), s.len()); // not byte length
    }

    #[test]
    fn envelope_accounts_unicode_scalars_not_bytes() {
        // Two multibyte sections; verify observed chars are scalar counts.
        let a = section("a", SectionPriority::Mandatory, "café🎉"); // 5 scalars (4 latin + 1 emoji)
        let b = section("b", SectionPriority::Mandatory, "中文测试"); // 4 scalars
        let env = assemble_envelope(EnvelopeStage::Generation, vec![a, b]).unwrap();
        assert_eq!(env.record("a").unwrap().observed_chars, 5);
        assert_eq!(env.record("b").unwrap().observed_chars, 4);
        assert_eq!(env.total_observed_chars, 9);
    }

    // ── 2.1 optional eviction order ────────────────────────────────────────

    #[test]
    fn optional_eviction_trims_latest_inserted_within_tier_first() {
        // Three Optional dialogue items; budget fits exactly two. Eviction drops
        // the latest-inserted Optional first (within-tier LIFO) and never touches
        // the Mandatory request.
        let request = section(section_id::REQUEST, SectionPriority::Mandatory, "ask"); // 3
        let d0 = section(
            &section_id::dialogue(0),
            SectionPriority::Optional,
            &"x".repeat(3_000),
        );
        let d1 = section(
            &section_id::dialogue(1),
            SectionPriority::Optional,
            &"y".repeat(3_000),
        );
        let d2 = section(
            &section_id::dialogue(2),
            SectionPriority::Optional,
            &"z".repeat(3_000),
        );
        let env = assemble_envelope(EnvelopeStage::Classifier, vec![request, d0, d1, d2]).unwrap();

        assert_eq!(
            env.record(section_id::REQUEST).unwrap().decision,
            InclusionDecision::Included
        );
        // 3 + 3000 + 3000 = 6003 <= 8000; the third (latest-inserted) optional is
        // evicted first, the two earlier optionals survive.
        assert_eq!(
            env.record(&section_id::dialogue(0)).unwrap().decision,
            InclusionDecision::Included
        );
        assert_eq!(
            env.record(&section_id::dialogue(1)).unwrap().decision,
            InclusionDecision::Included
        );
        assert_eq!(
            env.record(&section_id::dialogue(2)).unwrap().decision,
            InclusionDecision::Omitted
        );
        assert!(env.total_returned_chars <= CLASSIFIER_CEILING_CHARS);
    }

    #[test]
    fn optional_eviction_drops_optionals_before_relevant_under_real_pressure() {
        // Build content that genuinely exceeds the 8000-char classifier ceiling.
        let request = section("request", SectionPriority::Mandatory, "ask"); // 3
        let digest = section(
            section_id::DESIGN_DIGEST,
            SectionPriority::Relevant,
            &"d".repeat(4_000),
        );
        // Two optionals that together with the digest overflow the ceiling.
        let dialogue = section(
            &section_id::dialogue(0),
            SectionPriority::Optional,
            &"x".repeat(3_000),
        );
        let asset = section(
            &section_id::asset(0),
            SectionPriority::Optional,
            &"y".repeat(3_000),
        );
        let env = assemble_envelope(
            EnvelopeStage::Classifier,
            vec![request, digest, dialogue, asset],
        )
        .unwrap();

        // Mandatory request verbatim.
        assert_eq!(
            env.record("request").unwrap().decision,
            InclusionDecision::Included
        );
        // Relevant digest is fully retained (3 + 4000 = 4003, leaves 3997).
        let digest_rec = env.record(section_id::DESIGN_DIGEST).unwrap();
        assert_eq!(digest_rec.decision, InclusionDecision::Included);
        assert_eq!(digest_rec.returned_chars, 4_000);
        // Optionals share the remaining 3997: first-allocated dialogue is kept or
        // truncated, last-allocated asset is omitted because it is lowest priority
        // (Optional) and latest in insertion order.
        let asset_rec = env.record(&section_id::asset(0)).unwrap();
        assert_eq!(asset_rec.decision, InclusionDecision::Omitted);
        assert_eq!(asset_rec.returned_chars, 0);
        // Total returned never exceeds the ceiling.
        assert!(env.total_returned_chars <= CLASSIFIER_CEILING_CHARS);
    }

    #[test]
    fn per_section_budget_compacts_truncatable_sections_only() {
        // Optional dialogue capped at 200; authoritative source ignores the cap.
        let dialogue = ContextSection::new(
            section_id::dialogue(0),
            SectionPriority::Optional,
            Sensitivity::Sensitive,
            "z".repeat(500),
        )
        .with_budget(DIALOGUE_ITEM_CHAR_BUDGET);
        let source = ContextSection::new(
            section_id::CURRENT_SOURCE,
            SectionPriority::Authoritative,
            Sensitivity::Sensitive,
            "s".repeat(500),
        )
        .with_budget(10); // must be ignored: authoritative sections are exact
        let env = assemble_envelope(EnvelopeStage::Generation, vec![dialogue, source]).unwrap();
        let d = env.record(&section_id::dialogue(0)).unwrap();
        assert_eq!(d.observed_chars, 500);
        assert_eq!(d.returned_chars, DIALOGUE_ITEM_CHAR_BUDGET);
        assert_eq!(d.decision, InclusionDecision::Truncated);
        assert_eq!(d.reason, Some(SectionReason::PerSectionBudget));

        let s = env.record(section_id::CURRENT_SOURCE).unwrap();
        assert_eq!(s.observed_chars, 500);
        assert_eq!(s.returned_chars, 500); // cap ignored for exact sections
        assert_eq!(s.decision, InclusionDecision::Included);
        assert_eq!(s.reason, None);
    }

    // ── 2.1 mandatory-overflow error ───────────────────────────────────────

    #[test]
    fn mandatory_overflow_returns_error_with_observed_and_allowed() {
        // A single mandatory request larger than the classifier ceiling.
        let huge = section(
            section_id::REQUEST,
            SectionPriority::Mandatory,
            &"q".repeat(CLASSIFIER_CEILING_CHARS + 1),
        );
        let result = assemble_envelope(EnvelopeStage::Classifier, vec![huge]);
        let err = result.expect_err("mandatory overflow must error, not truncate");
        assert_eq!(err.allowed_chars, CLASSIFIER_CEILING_CHARS);
        assert_eq!(err.observed_chars, CLASSIFIER_CEILING_CHARS + 1);
        assert_eq!(err.overflow_sections.len(), 1);
        assert_eq!(err.overflow_sections[0].id, section_id::REQUEST);
        assert_eq!(
            err.overflow_sections[0].priority,
            SectionPriority::Mandatory
        );
        assert_eq!(
            err.overflow_sections[0].observed_chars,
            CLASSIFIER_CEILING_CHARS + 1
        );
    }

    #[test]
    fn mandatory_overflow_combines_authoritative_and_mandatory() {
        let request = section(
            section_id::REQUEST,
            SectionPriority::Mandatory,
            &"q".repeat(4_000),
        );
        let source = section(
            section_id::CURRENT_SOURCE,
            SectionPriority::Authoritative,
            &"s".repeat(5_000),
        );
        let result = assemble_envelope(EnvelopeStage::Classifier, vec![request, source]);
        let err = result.expect_err("combined exact content must overflow 8K");
        assert_eq!(err.allowed_chars, CLASSIFIER_CEILING_CHARS);
        assert_eq!(err.observed_chars, 9_000);
        // Both exact sections are named, regardless of tier.
        let ids: Vec<&str> = err
            .overflow_sections
            .iter()
            .map(|s| s.id.as_str())
            .collect();
        assert!(ids.contains(&section_id::REQUEST));
        assert!(ids.contains(&section_id::CURRENT_SOURCE));
    }

    // ── 2.5 exact state never silently truncated ────────────────────────────

    #[test]
    fn exact_source_survives_optional_pressure() {
        // Authoritative source + a torrent of optional content that must be
        // trimmed away, never the source.
        let request = section(section_id::REQUEST, SectionPriority::Mandatory, "ask");
        let source = section(
            section_id::CURRENT_SOURCE,
            SectionPriority::Authoritative,
            &"s".repeat(50_000),
        );
        let optional = section(
            &section_id::dialogue(0),
            SectionPriority::Optional,
            &"x".repeat(50_000),
        );
        let env =
            assemble_envelope(EnvelopeStage::Generation, vec![request, source, optional]).unwrap();
        let s = env.record(section_id::CURRENT_SOURCE).unwrap();
        assert_eq!(s.decision, InclusionDecision::Included);
        assert_eq!(s.returned_chars, s.observed_chars);
        assert_eq!(s.returned_chars, 50_000);
        assert!(env.exact_state_is_intact());
        // Optional absorbed the trim.
        let d = env.record(&section_id::dialogue(0)).unwrap();
        assert_eq!(d.decision, InclusionDecision::Omitted);
    }

    #[test]
    fn exact_params_and_diagnostic_survive_pressure() {
        let request = section(section_id::REQUEST, SectionPriority::Mandatory, "fix it");
        let params = section(
            section_id::CURRENT_PARAMS,
            SectionPriority::Authoritative,
            &"p".repeat(40_000),
        );
        let diag = section(
            section_id::REPAIR_DIAGNOSTIC,
            SectionPriority::Authoritative,
            &"e".repeat(20_000),
        );
        let dialogue = (0..DIALOGUE_MAX_ITEMS)
            .map(|i| {
                section(
                    &section_id::dialogue(i),
                    SectionPriority::Optional,
                    &"d".repeat(DIALOGUE_ITEM_CHAR_BUDGET),
                )
            })
            .collect::<Vec<_>>();
        let mut all = vec![request, params, diag];
        all.extend(dialogue);
        let env = assemble_envelope(EnvelopeStage::Generation, all).unwrap();
        assert!(env.exact_state_is_intact());
        assert_eq!(
            env.record(section_id::CURRENT_PARAMS)
                .unwrap()
                .returned_chars,
            40_000
        );
        assert_eq!(
            env.record(section_id::REPAIR_DIAGNOSTIC)
                .unwrap()
                .returned_chars,
            20_000
        );
    }

    #[test]
    fn exact_overflow_errors_rather_than_truncates() {
        // Source alone exceeds the generation ceiling: must error, not truncate.
        let source = section(
            section_id::CURRENT_SOURCE,
            SectionPriority::Authoritative,
            &"s".repeat(GENERATION_CEILING_CHARS + 1),
        );
        let err = assemble_envelope(EnvelopeStage::Generation, vec![source])
            .expect_err("exact overflow must error");
        assert_eq!(err.observed_chars, GENERATION_CEILING_CHARS + 1);
        assert_eq!(err.allowed_chars, GENERATION_CEILING_CHARS);
    }

    // ── 2.3 / 2.4 intent projection ────────────────────────────────────────

    fn full_inputs() -> ContextInputs {
        ContextInputs {
            request: "edit the dome".to_string(),
            authoring_context: Some("ctx".to_string()),
            source: Some("(model ...)".to_string()),
            params: Some(r#"{"radius":30}"#.to_string()),
            diagnostic: Some("Error: bad shape".to_string()),
            digest: Some("digest".to_string()),
            summary: Some("summary".to_string()),
            frontend_snapshot: Some("snapshot".to_string()),
            dialogue: vec!["hi".to_string(), "hello".to_string()],
            references: vec!["ref".to_string()],
            assets: vec!["asset".to_string()],
        }
    }

    #[test]
    fn design_projection_includes_exact_source_and_params() {
        let (stage, sections) = project_sections(ProjectionIntent::Design, &full_inputs());
        assert_eq!(stage, EnvelopeStage::Generation);
        let ids: Vec<&str> = sections.iter().map(|s| s.id.as_str()).collect();
        assert!(ids.contains(&section_id::REQUEST));
        assert!(ids.contains(&section_id::CURRENT_SOURCE));
        assert!(ids.contains(&section_id::CURRENT_PARAMS));
        let source = sections
            .iter()
            .find(|s| s.id == section_id::CURRENT_SOURCE)
            .unwrap();
        assert_eq!(source.priority, SectionPriority::Authoritative);
        let params = sections
            .iter()
            .find(|s| s.id == section_id::CURRENT_PARAMS)
            .unwrap();
        assert_eq!(params.priority, SectionPriority::Authoritative);
        let request = sections
            .iter()
            .find(|s| s.id == section_id::REQUEST)
            .unwrap();
        assert_eq!(request.priority, SectionPriority::Mandatory);
    }

    #[test]
    fn repair_projection_keeps_diagnostic_and_drops_unrelated_history() {
        let (_stage, sections) = project_sections(ProjectionIntent::Repair, &full_inputs());
        let ids: Vec<&str> = sections.iter().map(|s| s.id.as_str()).collect();
        assert!(ids.contains(&section_id::REPAIR_DIAGNOSTIC));
        assert!(ids.contains(&section_id::CURRENT_SOURCE));
        assert!(ids.contains(&section_id::CURRENT_PARAMS));
        // Unrelated history is omitted by projection.
        assert!(
            !ids.iter().any(|id| id.starts_with("dialogue-")),
            "repair must not include dialogue"
        );
        assert!(
            !ids.iter().any(|id| id.starts_with("reference-")),
            "repair must not include references"
        );
        let diag = sections
            .iter()
            .find(|s| s.id == section_id::REPAIR_DIAGNOSTIC)
            .unwrap();
        assert_eq!(diag.priority, SectionPriority::Authoritative);
    }

    #[test]
    fn question_default_uses_digest_not_full_source() {
        let (_stage, sections) = project_sections(
            ProjectionIntent::Question {
                source_required: false,
            },
            &full_inputs(),
        );
        let ids: Vec<&str> = sections.iter().map(|s| s.id.as_str()).collect();
        assert!(!ids.contains(&section_id::CURRENT_SOURCE));
        assert!(!ids.contains(&section_id::CURRENT_PARAMS));
        assert!(ids.contains(&section_id::DESIGN_DIGEST));
    }

    #[test]
    fn question_source_required_includes_exact_source() {
        let (_stage, sections) = project_sections(
            ProjectionIntent::Question {
                source_required: true,
            },
            &full_inputs(),
        );
        let ids: Vec<&str> = sections.iter().map(|s| s.id.as_str()).collect();
        assert!(ids.contains(&section_id::CURRENT_SOURCE));
        assert!(ids.contains(&section_id::CURRENT_PARAMS));
        let source = sections
            .iter()
            .find(|s| s.id == section_id::CURRENT_SOURCE)
            .unwrap();
        assert_eq!(source.priority, SectionPriority::Authoritative);
    }

    #[test]
    fn classifier_projection_uses_8k_ceiling_and_excludes_source() {
        let (stage, sections) = project_sections(
            ProjectionIntent::Classifier {
                include_references: false,
            },
            &full_inputs(),
        );
        assert_eq!(stage, EnvelopeStage::Classifier);
        assert_eq!(stage.ceiling_chars(), CLASSIFIER_CEILING_CHARS);
        let ids: Vec<&str> = sections.iter().map(|s| s.id.as_str()).collect();
        // Classifier carries prompt + digest + latest dialogue + snapshot.
        assert!(ids.contains(&section_id::REQUEST));
        assert!(ids.contains(&section_id::DESIGN_DIGEST));
        assert!(ids.contains(&section_id::FRONTEND_SNAPSHOT));
        assert!(ids.iter().any(|id| id.starts_with("dialogue-")));
        // Full source/params never enter the classifier projection.
        assert!(!ids.contains(&section_id::CURRENT_SOURCE));
        assert!(!ids.contains(&section_id::CURRENT_PARAMS));
        // No references without reference intent.
        assert!(!ids.iter().any(|id| id.starts_with("reference-")));
    }

    #[test]
    fn classifier_projection_includes_references_only_with_intent() {
        let (_stage, with_refs) = project_sections(
            ProjectionIntent::Classifier {
                include_references: true,
            },
            &full_inputs(),
        );
        assert!(with_refs.iter().any(|s| s.id.starts_with("reference-")));

        let (_stage, no_refs) = project_sections(
            ProjectionIntent::Classifier {
                include_references: false,
            },
            &full_inputs(),
        );
        assert!(!no_refs.iter().any(|s| s.id.starts_with("reference-")));
    }

    #[test]
    fn projection_caps_dialogue_and_references_to_configured_maxima() {
        let mut inputs = full_inputs();
        inputs.dialogue = (0..(DIALOGUE_MAX_ITEMS + 3))
            .map(|i| format!("d{i}"))
            .collect();
        inputs.references = (0..(REFERENCE_MAX_ITEMS + 3))
            .map(|i| format!("r{i}"))
            .collect();
        let (_stage, sections) = project_sections(ProjectionIntent::Design, &inputs);
        let dialogue_count = sections
            .iter()
            .filter(|s| s.id.starts_with("dialogue-"))
            .count();
        let reference_count = sections
            .iter()
            .filter(|s| s.id.starts_with("reference-"))
            .count();
        assert_eq!(dialogue_count, DIALOGUE_MAX_ITEMS);
        assert_eq!(reference_count, REFERENCE_MAX_ITEMS);
        // Each capped section carries its per-section budget.
        for s in sections.iter().filter(|s| s.id.starts_with("dialogue-")) {
            assert_eq!(s.budget_chars, Some(DIALOGUE_ITEM_CHAR_BUDGET));
        }
        for s in sections.iter().filter(|s| s.id.starts_with("reference-")) {
            assert_eq!(s.budget_chars, Some(REFERENCE_ITEM_CHAR_BUDGET));
        }
    }

    #[test]
    fn projection_is_pure_and_deterministic() {
        // Same inputs + intent always yield byte-identical section lists, and the
        // result depends only on the arguments (no external / model state).
        let a = project_sections(ProjectionIntent::Design, &full_inputs());
        let b = project_sections(ProjectionIntent::Design, &full_inputs());
        assert_eq!(a.0, b.0);
        assert_eq!(a.1.len(), b.1.len());
        for (x, y) in a.1.iter().zip(b.1.iter()) {
            assert_eq!(x.id, y.id);
            assert_eq!(x.priority, y.priority);
            assert_eq!(x.budget_chars, y.budget_chars);
            assert_eq!(x.content, y.content);
        }
    }

    // ── boundary: camelCase serialization, no content leak ─────────────────

    #[test]
    fn serialized_envelope_uses_camelcase_and_leaks_no_content() {
        let request = section(
            section_id::REQUEST,
            SectionPriority::Mandatory,
            "SECRET-SOURCE-CANNOT-LEAK",
        );
        let env = assemble_envelope(EnvelopeStage::Generation, vec![request]).unwrap();
        let json = serde_json::to_string(&env).unwrap();
        assert!(json.contains("observedChars"));
        assert!(json.contains("returnedChars"));
        assert!(json.contains("ceilingChars"));
        assert!(!json.contains("observed_chars"));
        // Content is never carried on the serialized envelope.
        assert!(!json.contains("SECRET-SOURCE-CANNOT-LEAK"));
    }

    #[test]
    fn serialized_budget_error_uses_camelcase() {
        let huge = section(
            section_id::REQUEST,
            SectionPriority::Mandatory,
            &"q".repeat(CLASSIFIER_CEILING_CHARS + 1),
        );
        let err = assemble_envelope(EnvelopeStage::Classifier, vec![huge]).unwrap_err();
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains("observedChars"));
        assert!(json.contains("allowedChars"));
        assert!(json.contains("overflowSections"));
        assert!(!json.contains("observed_chars"));
    }

    // ── 4.1: safe context telemetry (shape only, content-free) ────────────
    //
    // OpenSpec `agent-context-budgeting`, decision 5 + spec requirement
    // "Request-size telemetry is useful and content-free". Telemetry MUST
    // carry section ids, character counts, approximate tokens, inclusion
    // decisions, total size, stage, and provider usage when available, and
    // MUST exclude prompt text, source, reference bodies, image bytes, API
    // keys, authorization headers, and full paths.

    fn sample_usage() -> TelemetryUsage {
        TelemetryUsage {
            input_tokens: 1_234,
            output_tokens: 567,
            total_tokens: 1_801,
            cached_input_tokens: 900,
            reasoning_tokens: 12,
        }
    }

    #[test]
    fn telemetry_carries_shape_only_and_excludes_all_sensitive_content() {
        // Each section carries highly identifiable secret content that must
        // NEVER appear in serialized telemetry: a prompt secret, a fake API
        // key, an authorization header, full filesystem path, source body,
        // and a reference body.
        let request = ContextSection::new(
            section_id::REQUEST,
            SectionPriority::Mandatory,
            Sensitivity::Sensitive,
            "PROMPT-SECRET-DO-NOT-LEAK",
        );
        let source = ContextSection::new(
            section_id::CURRENT_SOURCE,
            SectionPriority::Authoritative,
            Sensitivity::Sensitive,
            "SOURCE-SECRET sk-leak-AAAAAAAAAAAAAAAA Bearer abc123authz",
        );
        let reference = ContextSection::new(
            &section_id::reference(0),
            SectionPriority::Optional,
            Sensitivity::Sensitive,
            "REFERENCE-BODY-SECRET /Users/bogdan/secret/file.ecky",
        )
        .with_budget(REFERENCE_ITEM_CHAR_BUDGET);
        let envelope =
            assemble_envelope(EnvelopeStage::Generation, vec![request, source, reference])
                .expect("small envelope assembles");

        let telemetry = envelope_telemetry(&envelope, Some(sample_usage()));
        let json = serde_json::to_string(&telemetry).expect("telemetry serializes");

        // ── Includes: section ids, counts, approx tokens, decisions, stage,
        //    total size, and provider usage (cache/input/output). ──
        assert!(json.contains("\"id\":\"request\""));
        assert!(json.contains("\"id\":\"current-source\""));
        assert!(json.contains("\"id\":\"reference-0\""));
        assert!(json.contains("\"observedChars\""));
        assert!(json.contains("\"returnedChars\""));
        assert!(json.contains("\"approxObservedTokens\""));
        assert!(json.contains("\"approxReturnedTokens\""));
        assert!(json.contains("\"decision\":\"included\""));
        assert!(json.contains("\"priority\":\"mandatory\""));
        assert!(json.contains("\"stage\":\"generation\""));
        assert!(json.contains("\"ceilingChars\":64000"));
        assert!(json.contains("\"totalReturnedChars\""));
        assert!(json.contains("\"inputTokens\":1234"));
        assert!(json.contains("\"outputTokens\":567"));
        assert!(json.contains("\"cachedInputTokens\":900"));
        assert!(json.contains("\"reasoningTokens\":12"));

        // ── Excludes: prompt, source, reference bodies, API keys, authz
        //    headers, and full paths. Content-free by construction. ──
        for needle in [
            "PROMPT-SECRET-DO-NOT-LEAK",
            "SOURCE-SECRET",
            "sk-leak-AAAAAAAAAAAAAAAA",
            "Bearer abc123authz",
            "REFERENCE-BODY-SECRET",
            "/Users/bogdan/secret/file.ecky",
            "file.ecky",
        ] {
            assert!(
                !json.contains(needle),
                "telemetry leaked sensitive content: {needle}"
            );
        }
    }

    #[test]
    fn telemetry_excludes_image_bytes() {
        // A reference/asset section whose content is a fake base64 image blob.
        // Telemetry must record only the shape, never the bytes.
        let image_section = ContextSection::new(
            &section_id::asset(0),
            SectionPriority::Optional,
            Sensitivity::Safe,
            "IMAGE-BYTES-SECRET-/9j/4AAQSkZJRgABAQAAAQABAAD/2wBDAAgGBgcGBQgHBwcJCQ==",
        );
        let request = ContextSection::new(
            section_id::REQUEST,
            SectionPriority::Mandatory,
            Sensitivity::Sensitive,
            "render it",
        );
        let envelope =
            assemble_envelope(EnvelopeStage::Generation, vec![request, image_section]).unwrap();
        let json = serde_json::to_string(&envelope_telemetry(&envelope, None)).unwrap();

        assert!(json.contains("\"id\":\"asset-0\""));
        assert!(!json.contains("IMAGE-BYTES-SECRET"));
        assert!(!json.contains("/9j/4AAQSkZJRg"));
        assert!(!json.contains("base64"));
    }

    #[test]
    fn telemetry_records_inclusion_decision_and_reason_for_evicted_section() {
        // Force eviction: classifier ceiling (8K) with a Relevant digest and an
        // Optional asset that together overflow after the mandatory request.
        let request = ContextSection::new(
            section_id::REQUEST,
            SectionPriority::Mandatory,
            Sensitivity::Sensitive,
            "ask",
        );
        let digest = ContextSection::new(
            section_id::DESIGN_DIGEST,
            SectionPriority::Relevant,
            Sensitivity::Safe,
            &"d".repeat(4_000),
        );
        let asset = ContextSection::new(
            &section_id::asset(0),
            SectionPriority::Optional,
            Sensitivity::Safe,
            &"y".repeat(4_000),
        );
        let envelope =
            assemble_envelope(EnvelopeStage::Classifier, vec![request, digest, asset]).unwrap();
        let telemetry = envelope_telemetry(&envelope, None);
        let asset_rec = telemetry
            .sections
            .iter()
            .find(|s| s.id == section_id::asset(0))
            .unwrap();
        assert_eq!(asset_rec.decision, InclusionDecision::Omitted);
        assert_eq!(asset_rec.reason, Some(SectionReason::EnvelopeBudget));
        assert_eq!(asset_rec.returned_chars, 0);
    }

    #[test]
    fn telemetry_omits_usage_field_when_no_provider_usage() {
        let request = ContextSection::new(
            section_id::REQUEST,
            SectionPriority::Mandatory,
            Sensitivity::Sensitive,
            "ask",
        );
        let envelope = assemble_envelope(EnvelopeStage::Classifier, vec![request]).unwrap();
        let json = serde_json::to_string(&envelope_telemetry(&envelope, None)).unwrap();
        assert!(!json.contains("\"usage\""));
        assert!(!json.contains("inputTokens"));
    }

    #[test]
    fn telemetry_is_content_free_for_every_intent_projection() {
        // Project full inputs for each intent; none of the carried secrets may
        // surface in telemetry regardless of stage/intent.
        let inputs = ContextInputs {
            request: "REQ-SECRET-ASK".to_string(),
            authoring_context: Some("CTX-SECRET".to_string()),
            source: Some("SRC-SECRET sk-leak-9999999999999999".to_string()),
            params: Some(r#"{"PARAM-SECRET":1}"#.to_string()),
            diagnostic: Some("DIAG-SECRET Bearer zzz".to_string()),
            digest: Some("digest".to_string()),
            summary: Some("SUMMARY-SECRET".to_string()),
            frontend_snapshot: Some("SNAPSHOT-SECRET".to_string()),
            dialogue: vec!["DIALOGUE-SECRET".to_string()],
            references: vec!["REF-SECRET /secret/path.ecky".to_string()],
            assets: vec!["ASSET-SECRET".to_string()],
        };
        let intents = [
            (
                "design",
                project_sections(ProjectionIntent::Design, &inputs),
            ),
            (
                "repair",
                project_sections(ProjectionIntent::Repair, &inputs),
            ),
            (
                "question",
                project_sections(
                    ProjectionIntent::Question {
                        source_required: true,
                    },
                    &inputs,
                ),
            ),
            (
                "classifier",
                project_sections(
                    ProjectionIntent::Classifier {
                        include_references: true,
                    },
                    &inputs,
                ),
            ),
        ];
        let secrets = [
            "REQ-SECRET-ASK",
            "CTX-SECRET",
            "SRC-SECRET",
            "sk-leak-9999999999999999",
            "PARAM-SECRET",
            "DIAG-SECRET",
            "Bearer zzz",
            "SUMMARY-SECRET",
            "SNAPSHOT-SECRET",
            "DIALOGUE-SECRET",
            "REF-SECRET",
            "/secret/path.ecky",
            "ASSET-SECRET",
        ];
        for (label, (stage, sections)) in intents {
            let envelope = assemble_envelope(stage, sections).expect("assembles");
            let json = serde_json::to_string(&envelope_telemetry(&envelope, Some(sample_usage())))
                .unwrap();
            for needle in secrets {
                assert!(
                    !json.contains(needle),
                    "[{label}] telemetry leaked: {needle}"
                );
            }
            // Shape is always present.
            assert!(json.contains("\"stage\":"));
            assert!(json.contains("\"sections\":"));
        }
    }
}
