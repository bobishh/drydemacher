use std::path::Path;

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use specta::Type;

use crate::campaign_definition::{self, CampaignStepPayload};
use crate::campaign_projects;
use crate::contracts::{AppError, AppResult, CampaignRun};

#[derive(Debug, Clone, Deserialize, Serialize, Type, PartialEq)]
#[serde(
    tag = "action",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum CampaignRunTransitionAction {
    #[specta(rename_all = "camelCase")]
    SaveDraft { draft: String },
    #[specta(rename_all = "camelCase")]
    Continue {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        draft: Option<String>,
    },
    #[specta(rename_all = "camelCase")]
    Back {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        draft: Option<String>,
    },
    #[specta(rename_all = "camelCase")]
    CheckSolution { candidate_source: String },
}

#[derive(Debug, Clone, Deserialize, Serialize, Type, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TransitionCampaignRunInput {
    pub run_id: String,
    pub action: CampaignRunTransitionAction,
}

#[derive(Debug, Clone, Serialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CampaignTransitionCheck {
    pub matched: bool,
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TransitionCampaignRunResult {
    pub run: CampaignRun,
    pub step: CampaignStepPayload,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub check: Option<CampaignTransitionCheck>,
}

pub fn transition_campaign_run(
    db: &mut Connection,
    campaign_root: &Path,
    input: TransitionCampaignRunInput,
) -> AppResult<TransitionCampaignRunResult> {
    let mut run = campaign_projects::get(db, input.run_id.trim())?;
    let current =
        campaign_definition::step(campaign_root, &run.definition_id, &run.current_step_id)?;
    if current.definition_version != run.definition_version {
        return Err(AppError::conflict(format!(
            "Campaign definition changed from '{}' to '{}'; reopen the campaign before editing progress.",
            run.definition_version, current.definition_version
        )));
    }
    let active = current
        .current_step
        .as_ref()
        .ok_or_else(|| AppError::validation("Campaign current step is unavailable."))?;
    let mut check = None;

    let projected_step = match input.action {
        CampaignRunTransitionAction::SaveDraft { draft } => {
            save_current_draft(&mut run, &current, draft)?;
            current.clone()
        }
        CampaignRunTransitionAction::Continue { draft } => {
            if active.kind == "challenge" {
                return Err(AppError::validation(
                    "Challenge steps must pass CHECK SOLUTION before continuing.",
                ));
            }
            if let Some(draft) = draft {
                save_current_draft(&mut run, &current, draft)?;
            }
            let next_step_id = active
                .next_step_id
                .as_deref()
                .ok_or_else(|| AppError::validation("Campaign current step has no next step."))?;
            let next = campaign_definition::step(campaign_root, &run.definition_id, next_step_id)?;
            push_unique(&mut run.completed_step_ids, &active.id);
            run.current_step_id = next_step_id.to_string();
            next
        }
        CampaignRunTransitionAction::Back { draft } => {
            if let Some(draft) = draft {
                save_current_draft(&mut run, &current, draft)?;
            }
            let previous_step_id = active
                .previous_step
                .as_ref()
                .map(|previous| previous.id.as_str())
                .ok_or_else(|| {
                    AppError::validation("Campaign current step has no previous step.")
                })?;
            if !run
                .completed_step_ids
                .iter()
                .any(|id| id == previous_step_id)
            {
                return Err(AppError::validation(format!(
                    "Campaign previous step '{}' is not completed.",
                    previous_step_id
                )));
            }
            let previous =
                campaign_definition::step(campaign_root, &run.definition_id, previous_step_id)?;
            run.current_step_id = previous_step_id.to_string();
            previous
        }
        CampaignRunTransitionAction::CheckSolution { candidate_source } => {
            if active.kind != "challenge" || active.acceptance.is_none() {
                return Err(AppError::validation(
                    "Campaign current step is not a checkable challenge.",
                ));
            }
            save_current_draft(&mut run, &current, candidate_source.clone())?;
            let evaluation = campaign_definition::check_step(
                campaign_root,
                &run.definition_id,
                &active.id,
                candidate_source,
            )?;
            check = Some(CampaignTransitionCheck {
                matched: evaluation.matched,
            });
            if evaluation.matched {
                push_unique(&mut run.completed_step_ids, &active.id);
                push_unique(&mut run.passed_challenge_ids, &active.id);
                if let Some(next_step_id) = active.next_step_id.as_deref() {
                    let next =
                        campaign_definition::step(campaign_root, &run.definition_id, next_step_id)?;
                    run.current_step_id = next_step_id.to_string();
                    next
                } else {
                    current.clone()
                }
            } else {
                current.clone()
            }
        }
    };

    let run = campaign_projects::save(db, run)?;
    Ok(TransitionCampaignRunResult {
        run,
        step: projected_step,
        check,
    })
}

fn save_current_draft(
    run: &mut CampaignRun,
    payload: &CampaignStepPayload,
    draft: String,
) -> AppResult<()> {
    if draft.contains('\0') {
        return Err(AppError::validation("Campaign draft contains a null byte."));
    }
    let active = payload
        .current_step
        .as_ref()
        .ok_or_else(|| AppError::validation("Campaign current step is unavailable."))?;
    if active.source.is_none() {
        return Err(AppError::validation(
            "Campaign current step has no editable source draft.",
        ));
    }
    let digest = active.canonical_source_digest.as_deref().ok_or_else(|| {
        AppError::validation("Campaign source step has no canonical source digest.")
    })?;
    let key = format!("{}/{}@{}", payload.definition_id, active.id, digest);
    run.draft_overrides_by_step_id.insert(key, draft);
    Ok(())
}

fn push_unique(ids: &mut Vec<String>, id: &str) {
    if !ids.iter().any(|candidate| candidate == id) {
        ids.push(id.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::CreateCampaignRunInput;

    fn database() -> Connection {
        let db = Connection::open_in_memory().expect("db");
        db.execute_batch(
            "PRAGMA foreign_keys = ON;
             CREATE TABLE campaign_runs (
                id TEXT PRIMARY KEY, definition_id TEXT NOT NULL, definition_version TEXT NOT NULL,
                title TEXT NOT NULL, current_step_id TEXT NOT NULL, created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
             );
             CREATE TABLE campaign_run_steps (
                run_id TEXT NOT NULL REFERENCES campaign_runs(id) ON DELETE CASCADE,
                step_id TEXT NOT NULL, status TEXT NOT NULL, draft_override TEXT,
                updated_at INTEGER NOT NULL, PRIMARY KEY(run_id, step_id, status)
             );",
        )
        .expect("schema");
        db
    }

    struct CampaignFixture {
        base: std::path::PathBuf,
        root: std::path::PathBuf,
    }

    impl CampaignFixture {
        fn new() -> Self {
            let base = std::env::temp_dir()
                .join(format!("ecky-campaign-transition-{}", uuid::Uuid::new_v4()));
            let root = base.join("docs/books/ecky-ir/missions");
            std::fs::create_dir_all(&root).expect("campaign root");
            std::fs::write(
                root.join("manifest.edn"),
                r#"{
                  :missions [{
                    :id "mission-test"
                    :section-slug "test"
                    :title "Test mission"
                    :content "docs/books/ecky-ir/missions/test.md"
                    :steps [
                      {:id "intro" :kind "explain" :title "Intro"}
                      {:id "worked" :kind "worked" :title "Worked" :source "docs/books/ecky-ir/missions/worked.ecky"}
                      {:id "challenge" :kind "challenge" :title "Challenge" :source "docs/books/ecky-ir/missions/challenge.ecky"
                       :acceptance {:mode "equivalentCoreIr" :reference-step-id "solution"}}
                      {:id "solution" :kind "solution" :title "Solution" :source "docs/books/ecky-ir/missions/solution.ecky"}
                    ]
                  }]
                }"#,
            )
            .expect("manifest");
            std::fs::write(
                root.join("test.md"),
                "# Test\n\n## Intro {#intro}\n\nIntro prose.\n\n## Worked {#worked}\n\nWorked prose.\n\n## Challenge {#challenge}\n\nChallenge prose.\n\n## Solution {#solution}\n\nSolution prose.\n",
            )
            .expect("markdown");
            std::fs::write(
                root.join("worked.ecky"),
                "(model (part worked (box 1 2 3)))",
            )
            .expect("source");
            std::fs::write(
                root.join("challenge.ecky"),
                "(model (part result (box 1 1 1)))",
            )
            .expect("challenge");
            std::fs::write(
                root.join("solution.ecky"),
                "(model (part result (box 2 2 2)))",
            )
            .expect("solution");
            Self { base, root }
        }
    }

    impl Drop for CampaignFixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.base);
        }
    }

    fn create_run(db: &mut Connection, root: &Path, step_id: &str) -> CampaignRun {
        let payload =
            campaign_definition::step(root, "ecky-ir-build-missions", step_id).expect("step");
        campaign_projects::create(
            db,
            CreateCampaignRunInput {
                title: "Campaign".to_string(),
                definition_id: payload.definition_id,
                definition_version: payload.definition_version,
                current_step_id: step_id.to_string(),
            },
        )
        .expect("run")
    }

    #[test]
    fn continue_owns_completion_current_step_and_atomic_persistence() {
        let mut db = database();
        let fixture = CampaignFixture::new();
        let run = create_run(&mut db, &fixture.root, "mission-test/intro");

        let result = transition_campaign_run(
            &mut db,
            &fixture.root,
            TransitionCampaignRunInput {
                run_id: run.id.clone(),
                action: CampaignRunTransitionAction::Continue { draft: None },
            },
        )
        .expect("continue");

        assert_eq!(result.run.current_step_id, "mission-test/worked");
        assert_eq!(result.run.completed_step_ids, vec!["mission-test/intro"]);
        assert_eq!(campaign_projects::get(&db, &run.id).unwrap(), result.run);
    }

    #[test]
    fn illegal_back_does_not_mutate_persisted_run() {
        let mut db = database();
        let fixture = CampaignFixture::new();
        let run = create_run(&mut db, &fixture.root, "mission-test/worked");

        let error = transition_campaign_run(
            &mut db,
            &fixture.root,
            TransitionCampaignRunInput {
                run_id: run.id.clone(),
                action: CampaignRunTransitionAction::Back { draft: None },
            },
        )
        .expect_err("back must be locked");

        assert!(error.message.contains("not completed"));
        assert_eq!(campaign_projects::get(&db, &run.id).unwrap(), run);
    }

    #[test]
    fn save_draft_derives_digest_bound_key_in_rust() {
        let mut db = database();
        let fixture = CampaignFixture::new();
        let run = create_run(&mut db, &fixture.root, "mission-test/worked");

        let result = transition_campaign_run(
            &mut db,
            &fixture.root,
            TransitionCampaignRunInput {
                run_id: run.id,
                action: CampaignRunTransitionAction::SaveDraft {
                    draft: "(model (part learner (box 1 2 3)))".to_string(),
                },
            },
        )
        .expect("draft");

        let key = result
            .run
            .draft_overrides_by_step_id
            .keys()
            .next()
            .expect("draft key");
        assert!(key.starts_with("ecky-ir-build-missions/mission-test/worked@sha256:"));
    }

    #[test]
    fn matched_challenge_check_persists_pass_completion_draft_and_next_step_once() {
        let mut db = database();
        let fixture = CampaignFixture::new();
        let run = create_run(&mut db, &fixture.root, "mission-test/challenge");
        let candidate = std::fs::read_to_string(fixture.root.join("solution.ecky")).unwrap();

        let result = transition_campaign_run(
            &mut db,
            &fixture.root,
            TransitionCampaignRunInput {
                run_id: run.id.clone(),
                action: CampaignRunTransitionAction::CheckSolution {
                    candidate_source: candidate.clone(),
                },
            },
        )
        .expect("check");

        assert_eq!(
            result.check,
            Some(CampaignTransitionCheck { matched: true })
        );
        assert_eq!(result.run.current_step_id, "mission-test/solution");
        assert_eq!(
            result.run.completed_step_ids,
            vec!["mission-test/challenge"]
        );
        assert_eq!(
            result.run.passed_challenge_ids,
            vec!["mission-test/challenge"]
        );
        assert_eq!(
            result.run.draft_overrides_by_step_id.values().next(),
            Some(&candidate)
        );
        assert_eq!(campaign_projects::get(&db, &run.id).unwrap(), result.run);
    }

    #[test]
    fn transition_boundary_is_camel_case_and_tagged() {
        let value = serde_json::to_value(TransitionCampaignRunInput {
            run_id: "run-1".to_string(),
            action: CampaignRunTransitionAction::CheckSolution {
                candidate_source: "(model)".to_string(),
            },
        })
        .expect("serialize");
        assert_eq!(value["runId"], "run-1");
        assert_eq!(value["action"]["action"], "checkSolution");
        assert_eq!(value["action"]["candidateSource"], "(model)");
        assert!(value.get("run_id").is_none());
    }
}
