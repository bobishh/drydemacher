//! OpenSpec `agent-context-budgeting`, task 4.3.
//!
//! Representative p50/p95 context-budget audit across four turn shapes —
//! empty, normal, repair, and large-source — plus a pathological mandatory
//! overflow. This is pure-rule evidence over the typed envelope (no model call,
//! no provider dispatch): it sizes representative inputs, runs them through the
//! deterministic projection + budget assembly, and reports p50/p95 of the
//! returned envelope size and approximate tokens.
//!
//! The audit CONFIRMS (it does not speculatively revise) the default ceilings:
//! every representative turn stays within its stage ceiling with exact state
//! intact, and an oversized authoritative source fails pre-dispatch instead of
//! producing a lossy envelope. Real large-model p50/p95 tuning remains an open
//! question (design §"Open Questions"); no ceiling is changed here without that
//! evidence.
//!
//! Run the report with stdout visible: `cargo test --test context_budget_audit
//! -- --nocapture`.

use ecky_cad_lib::context_envelope::{
    approx_tokens, assemble_envelope, measure_chars, project_sections, ContextInputs,
    EnvelopeStage, ProjectionIntent, GENERATION_CEILING_CHARS,
};
use std::cmp::min;

// ── Percentile helper ──────────────────────────────────────────────────────

/// Deterministic nearest-rank percentile (1..=100) over a non-empty sample.
/// Sorts ascending and picks index `ceil(p/100 * n) - 1`.
fn percentile(sorted: &[usize], p: usize) -> usize {
    assert!(!sorted.is_empty(), "percentile needs a sample");
    assert!((1..=100).contains(&p), "percentile in 1..=100");
    let n = sorted.len();
    let rank = ((p as f64) / 100.0 * n as f64).ceil() as usize;
    sorted[min(rank, n) - 1]
}

// ── Representative fixture builders ─────────────────────────────────────────
//
// Sizes are chosen to be representative of real authoring turns, not
// pathological. Dialogue/reference/asset counts stay at or below the projection
// caps so the budget decisions reflect typical included/compacted outcomes.

const SAMPLES: usize = 21;

fn lorem(chars: usize) -> String {
    // Deterministic, vaguely-code-like filler so sizes are stable and char
    // counts are exact. Repeats a 40-char unit and trims to `chars`.
    let unit = "shape = Part.makeBox(1, 1, 1) # wall \n";
    let needed = chars / unit.chars().count() + 1;
    let mut s = unit.repeat(needed);
    s.truncate(
        s.char_indices()
            .nth(chars)
            .map(|(b, _)| b)
            .unwrap_or(s.len()),
    );
    s
}

fn empty_inputs(seed: usize) -> ContextInputs {
    ContextInputs {
        request: format!("what can you do? ({seed})"),
        ..Default::default()
    }
}

fn normal_inputs(seed: usize) -> ContextInputs {
    let scale = 1 + seed % 5; // 1..=5 — gentle spread
    ContextInputs {
        request: format!("increase the fillet radius to {}mm", 5 + seed),
        authoring_context: Some(
            "current: legacyPython/freecad → target: legacyPython/freecad".into(),
        ),
        source: Some(lorem(2_000 * scale)),
        params: Some(format!(
            r#"{{"radius":{},"height":{}}}"#,
            10 + seed,
            20 + seed
        )),
        digest: Some("Bracket [V3] (legacyPython) — 2 parts, 1 fillet".into()),
        summary: Some(lorem(300)),
        dialogue: vec![lorem(180), lorem(190), format!("USER: tweak {seed}")],
        references: vec![lorem(900)],
        assets: vec!["/assets/bracket.step".to_string()],
        ..Default::default()
    }
}

fn repair_inputs(seed: usize) -> ContextInputs {
    let scale = 1 + seed % 5;
    ContextInputs {
        request: format!("repair the model — it fails to render (case {seed})"),
        authoring_context: Some("current/target: legacyPython/freecad".into()),
        source: Some(lorem(3_000 * scale)),
        params: Some(format!(r#"{{"radius":{}}}"#, 5 + seed)),
        // Repair carries the latest raw diagnostic verbatim (authoritative).
        diagnostic: Some(format!(
            "Traceback (most recent call last):\n  File \"<string>\", line {}\n{}\nPart OCCError: {}",
            10 + seed,
            lorem(400 * scale),
            "cannot compute fillet on degenerate edge"
        )),
        digest: Some("Bracket [V3] (legacyPython)".into()),
        ..Default::default()
    }
}

fn large_source_inputs(seed: usize) -> ContextInputs {
    // A genuinely large but sub-ceiling source (20K..~32K) plus full optional
    // state, representative of a big macro being edited.
    let scale = 1 + seed % 7; // 1..=7
    ContextInputs {
        request: format!("refactor the wall pattern, case {seed}"),
        authoring_context: Some("current/target: legacyPython/freecad".into()),
        source: Some(lorem(20_000 + 1_500 * scale)),
        params: Some(lorem(400)),
        digest: Some("WallPattern [V7] (legacyPython) — 142 parts".into()),
        summary: Some(lorem(500)),
        dialogue: vec![lorem(190), format!("USER: keep the lip {seed}")],
        references: vec![lorem(1_100), lorem(1_150)],
        assets: vec![
            "/assets/wall.step".to_string(),
            "/assets/lip.step".to_string(),
        ],
        ..Default::default()
    }
}

struct CategoryResult {
    name: &'static str,
    ceiling: usize,
    returned_p50: usize,
    returned_p95: usize,
    tokens_p50: usize,
    tokens_p95: usize,
    all_within_ceiling: bool,
    exact_intact: usize,
    samples: usize,
}

fn run_category(
    name: &'static str,
    intent: ProjectionIntent,
    builder: fn(usize) -> ContextInputs,
) -> CategoryResult {
    let mut returned: Vec<usize> = Vec::with_capacity(SAMPLES);
    let mut tokens: Vec<usize> = Vec::with_capacity(SAMPLES);
    let mut all_within = true;
    let mut exact_intact = 0usize;

    for seed in 0..SAMPLES {
        let inputs = builder(seed);
        let (stage, sections) = project_sections(intent, &inputs);
        let envelope = assemble_envelope(stage, sections).expect(
            "representative mandatory content must fit the ceiling; \
             if this errors, the fixture is no longer representative",
        );
        if envelope.total_returned_chars > stage.ceiling_chars() {
            all_within = false;
        }
        if envelope.exact_state_is_intact() {
            exact_intact += 1;
        }
        returned.push(envelope.total_returned_chars);
        tokens.push(envelope.total_approx_returned_tokens);
    }

    returned.sort_unstable();
    tokens.sort_unstable();

    CategoryResult {
        name,
        ceiling: intent_ceiling(intent),
        returned_p50: percentile(&returned, 50),
        returned_p95: percentile(&returned, 95),
        tokens_p50: percentile(&tokens, 50),
        tokens_p95: percentile(&tokens, 95),
        all_within_ceiling: all_within,
        exact_intact,
        samples: SAMPLES,
    }
}

fn intent_ceiling(intent: ProjectionIntent) -> usize {
    let stage = match intent {
        ProjectionIntent::Classifier { .. } => EnvelopeStage::Classifier,
        _ => EnvelopeStage::Generation,
    };
    stage.ceiling_chars()
}

fn print_report(results: &[CategoryResult]) {
    eprintln!();
    eprintln!("==== agent-context-budgeting §4.3 audit (representative fixtures) ====");
    eprintln!(
        "{:<14} {:>8} {:>10} {:>10} {:>10} {:>10} {:>10}",
        "category", "ceiling", "ret_p50", "ret_p95", "tok_p50", "tok_p95", "exact_ok"
    );
    for r in results {
        eprintln!(
            "{:<14} {:>8} {:>10} {:>10} {:>10} {:>10} {:>10}/{:<3}",
            r.name,
            r.ceiling,
            r.returned_p50,
            r.returned_p95,
            r.tokens_p50,
            r.tokens_p95,
            r.exact_intact,
            r.samples,
        );
    }
    eprintln!("legend: ret_* = returned chars (deterministic Unicode count); tok_* = ceil(chars/4) metric");
    eprintln!(
        "ceilings: generation = {} chars, classifier (unused here) = 8_000 chars",
        GENERATION_CEILING_CHARS
    );
    eprintln!("=====================================================================");
    eprintln!();
}

#[test]
fn representative_turns_stay_within_ceiling_with_exact_state_intact() {
    let results = vec![
        run_category("empty", ProjectionIntent::Design, empty_inputs),
        run_category("normal", ProjectionIntent::Design, normal_inputs),
        run_category("repair", ProjectionIntent::Repair, repair_inputs),
        run_category(
            "large-source",
            ProjectionIntent::Design,
            large_source_inputs,
        ),
    ];
    print_report(&results);

    // ── Evidence to CONFIRM the default ceilings (no speculative change). ──
    for r in &results {
        assert!(
            r.all_within_ceiling,
            "[{}] every representative envelope must stay within its {}-char ceiling \
             (p95 returned = {})",
            r.name, r.ceiling, r.returned_p95,
        );
        assert_eq!(
            r.exact_intact, r.samples,
            "[{}] exact (mandatory/authoritative) state must survive on every turn",
            r.name,
        );
        // Headroom: representative p95 should sit comfortably under the ceiling.
        assert!(
            r.returned_p95 < r.ceiling,
            "[{}] p95 returned ({}) must be below the ceiling ({})",
            r.name,
            r.returned_p95,
            r.ceiling,
        );
    }

    // Large-source is the stress category: confirm the 64K ceiling is adequate
    // for representative (sub-pathological) large macro edits and leave a
    // documented margin. This is the evidence the open question asks for; it
    // does not justify changing the ceiling.
    let large = results.iter().find(|r| r.name == "large-source").unwrap();
    let margin = GENERATION_CEILING_CHARS.saturating_sub(large.returned_p95);
    eprintln!(
        "large-source p95 = {} chars; remaining headroom under {} ceiling = {} chars (~{} approx tokens)",
        large.returned_p95,
        GENERATION_CEILING_CHARS,
        margin,
        approx_tokens(margin),
    );
    assert!(
        large.returned_p95 <= 48_000,
        "representative large-source p95 ({}) unexpectedly close to the 64K ceiling; \
         revisit fixture realism before drawing ceiling conclusions",
        large.returned_p95,
    );
}

#[test]
fn pathological_oversized_authoritative_source_errors_pre_dispatch() {
    // An authoritative current source larger than the 64K generation ceiling
    // must fail pre-dispatch with a raw budget error (observed/allowed), never a
    // lossy envelope. This is the safety backstop behind the audit's "fits"
    // cases above.
    let oversized = lorem(GENERATION_CEILING_CHARS + 8_000);
    let inputs = ContextInputs {
        request: "edit the oversized model".to_string(),
        source: Some(oversized),
        ..Default::default()
    };
    let (stage, sections) = project_sections(ProjectionIntent::Design, &inputs);
    let result = assemble_envelope(stage, sections);
    let err = result.expect_err("oversized authoritative source must overflow, not truncate");
    assert_eq!(err.allowed_chars, GENERATION_CEILING_CHARS);
    assert!(err.observed_chars > GENERATION_CEILING_CHARS);
    assert!(
        err.overflow_sections
            .iter()
            .any(|s| s.id == "current-source"),
        "overflow error must name the overflowing section",
    );

    // Sanity: the same source measured directly is above the ceiling, so the
    // error's observed size is consistent with the enforcement unit.
    let measured = measure_chars(inputs.source.as_deref().unwrap());
    assert!(measured > GENERATION_CEILING_CHARS);
    assert_eq!(
        measured,
        err.observed_chars - (measure_chars(&inputs.request))
    );
}
