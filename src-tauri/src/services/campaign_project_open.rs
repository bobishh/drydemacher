use std::path::Path;

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use specta::Type;

use crate::campaign_definition::{self, CampaignStepPayload};
use crate::campaign_projects;
use crate::contracts::{
    ActiveProjectNavigation, AppError, AppResult, CampaignRun, CreateCampaignRunInput,
};

#[derive(Debug, Clone, Deserialize, Serialize, Type, PartialEq, Eq)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum OpenCampaignProjectIntent {
    #[specta(rename_all = "camelCase")]
    Start { definition_id: String },
    #[specta(rename_all = "camelCase")]
    Resume { run_id: String },
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct OpenCampaignProjectResult {
    pub run: CampaignRun,
    pub step: CampaignStepPayload,
}

pub fn open_campaign_project(
    db: &mut Connection,
    campaign_root: &Path,
    intent: OpenCampaignProjectIntent,
) -> AppResult<OpenCampaignProjectResult> {
    match intent {
        OpenCampaignProjectIntent::Start { definition_id } => {
            let definition_id = definition_id.trim();
            let summary = campaign_definition::summaries(campaign_root)?
                .into_iter()
                .find(|summary| summary.definition_id == definition_id)
                .ok_or_else(|| {
                    AppError::not_found(format!("Campaign definition not found: {definition_id}"))
                })?;
            let step = campaign_definition::step(
                campaign_root,
                &summary.definition_id,
                &summary.first_step_id,
            )?;

            let tx = db
                .transaction()
                .map_err(|error| AppError::persistence(error.to_string()))?;
            let run = campaign_projects::create(
                &tx,
                CreateCampaignRunInput {
                    title: summary.title,
                    definition_id: summary.definition_id,
                    definition_version: step.definition_version.clone(),
                    current_step_id: summary.first_step_id,
                },
            )?;
            save_campaign_navigation(&tx, &run.id)?;
            tx.commit()
                .map_err(|error| AppError::persistence(error.to_string()))?;

            Ok(OpenCampaignProjectResult { run, step })
        }
        OpenCampaignProjectIntent::Resume { run_id } => {
            let tx = db
                .transaction()
                .map_err(|error| AppError::persistence(error.to_string()))?;
            let run = campaign_projects::get(&tx, run_id.trim())?;
            let step =
                campaign_definition::step(campaign_root, &run.definition_id, &run.current_step_id)?;
            if step.definition_version != run.definition_version {
                return Err(AppError::conflict(format!(
                    "Campaign definition changed from '{}' to '{}'.",
                    run.definition_version, step.definition_version
                )));
            }
            save_campaign_navigation(&tx, &run.id)?;
            tx.commit()
                .map_err(|error| AppError::persistence(error.to_string()))?;

            Ok(OpenCampaignProjectResult { run, step })
        }
    }
}

fn save_campaign_navigation(db: &Connection, run_id: &str) -> AppResult<()> {
    campaign_projects::save_active_project_navigation(
        db,
        ActiveProjectNavigation {
            kind: "campaign".to_string(),
            id: run_id.to_string(),
            view: "campaign".to_string(),
        },
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::campaign_projects;

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
             );
             CREATE TABLE active_project_navigation (
                slot INTEGER PRIMARY KEY CHECK(slot = 1), kind TEXT NOT NULL,
                project_id TEXT NOT NULL, view TEXT NOT NULL, updated_at INTEGER NOT NULL
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
            let base = std::env::temp_dir().join(format!(
                "ecky-campaign-project-open-{}",
                uuid::Uuid::new_v4()
            ));
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
                    :steps [{:id "intro" :kind "explain" :title "Intro"}]
                  }]
                }"#,
            )
            .expect("manifest");
            std::fs::write(
                root.join("test.md"),
                "# Test\n\n## Intro {#intro}\n\nIntro prose.\n",
            )
            .expect("markdown");
            Self { base, root }
        }
    }

    impl Drop for CampaignFixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.base);
        }
    }

    #[test]
    fn start_derives_first_step_version_title_and_navigation_atomically() {
        let mut db = database();
        let fixture = CampaignFixture::new();

        let result = open_campaign_project(
            &mut db,
            &fixture.root,
            OpenCampaignProjectIntent::Start {
                definition_id: "ecky-ir-build-missions".to_string(),
            },
        )
        .expect("start");

        assert_eq!(result.run.title, "Ecky: six build missions");
        assert_eq!(result.run.current_step_id, "mission-test/intro");
        assert_eq!(
            result.run.definition_version,
            result.step.definition_version
        );
        assert_eq!(
            campaign_projects::get_active_project_navigation(&db)
                .expect("navigation")
                .expect("active project")
                .id,
            result.run.id
        );
    }

    #[test]
    fn unknown_definition_rolls_back_run_and_navigation() {
        let mut db = database();
        let fixture = CampaignFixture::new();

        let error = open_campaign_project(
            &mut db,
            &fixture.root,
            OpenCampaignProjectIntent::Start {
                definition_id: "missing".to_string(),
            },
        )
        .expect_err("missing definition");

        assert!(error.message.contains("Campaign definition not found"));
        assert!(campaign_projects::list(&db).expect("runs").is_empty());
        assert_eq!(
            campaign_projects::get_active_project_navigation(&db).expect("navigation"),
            None
        );
    }

    #[test]
    fn intent_boundary_is_tagged_and_camel_case() {
        let value = serde_json::to_value(OpenCampaignProjectIntent::Start {
            definition_id: "definition-1".to_string(),
        })
        .expect("serialize");

        assert_eq!(value["kind"], "start");
        assert_eq!(value["definitionId"], "definition-1");
        assert!(value.get("definition_id").is_none());
    }
}
