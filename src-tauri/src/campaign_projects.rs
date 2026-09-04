use crate::contracts::{
    ActiveProjectNavigation, AppError, AppResult, CampaignRun, CreateCampaignRunInput,
    ThreadWindowLayout,
};
use rusqlite::{params, Connection, OptionalExtension};
use std::collections::{BTreeMap, BTreeSet};
use std::time::{SystemTime, UNIX_EPOCH};

fn now() -> AppResult<i64> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_secs() as i64)
        .map_err(|_| AppError::persistence("campaign clock failed"))
}

fn fail(error: rusqlite::Error) -> AppError {
    AppError::persistence(error.to_string())
}

fn required(value: &str, field: &str) -> AppResult<()> {
    if value.trim().is_empty() {
        Err(AppError::validation(format!(
            "Campaign {field} is required."
        )))
    } else {
        Ok(())
    }
}

fn validate_create(input: &CreateCampaignRunInput) -> AppResult<()> {
    required(&input.title, "title")?;
    required(&input.definition_id, "definition")?;
    required(&input.definition_version, "definition version")?;
    required(&input.current_step_id, "current step")
}

fn validate_run(run: &CampaignRun) -> AppResult<()> {
    if run.kind != "campaignRun" {
        return Err(AppError::validation("Invalid campaign project kind."));
    }
    required(&run.id, "ID")?;
    required(&run.title, "title")?;
    required(&run.definition_id, "definition")?;
    required(&run.definition_version, "definition version")?;
    required(&run.current_step_id, "current step")?;
    for id in run
        .completed_step_ids
        .iter()
        .chain(&run.passed_challenge_ids)
    {
        required(id, "step ID")?;
    }
    for (id, draft) in &run.draft_overrides_by_step_id {
        required(id, "draft step ID")?;
        if draft.contains('\0') {
            return Err(AppError::validation("Campaign draft contains a null byte."));
        }
    }
    Ok(())
}

fn assemble(db: &Connection, id: &str) -> AppResult<CampaignRun> {
    let (definition_id, definition_version, title, current_step_id, created_at, updated_at):
        (String, String, String, String, i64, i64) = db
        .query_row(
            "SELECT definition_id, definition_version, title, current_step_id, created_at, updated_at
             FROM campaign_runs WHERE id = ?1",
            [id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?)),
        )
        .optional()
        .map_err(fail)?
        .ok_or_else(|| AppError::not_found("Campaign run not found."))?;

    let mut completed_step_ids = Vec::new();
    let mut passed_challenge_ids = Vec::new();
    let mut draft_overrides_by_step_id = BTreeMap::new();
    let mut statement = db
        .prepare(
            "SELECT step_id, status, draft_override
             FROM campaign_run_steps WHERE run_id = ?1 ORDER BY step_id, status",
        )
        .map_err(fail)?;
    let rows = statement
        .query_map([id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
            ))
        })
        .map_err(fail)?;
    for row in rows {
        let (step_id, status, draft_override) = row.map_err(fail)?;
        match status.as_str() {
            "completed" => completed_step_ids.push(step_id),
            "passed" => passed_challenge_ids.push(step_id),
            "draft" => {
                let draft = draft_override.ok_or_else(|| {
                    AppError::persistence("Campaign draft row is missing its source override.")
                })?;
                draft_overrides_by_step_id.insert(step_id, draft);
            }
            _ => {
                return Err(AppError::persistence(
                    "Campaign step row has an unknown status.",
                ))
            }
        }
    }

    Ok(CampaignRun {
        id: id.to_owned(),
        kind: "campaignRun".to_owned(),
        title,
        definition_id,
        definition_version,
        current_step_id,
        completed_step_ids,
        passed_challenge_ids,
        draft_overrides_by_step_id,
        created_at: created_at as u64,
        updated_at: updated_at as u64,
    })
}

pub fn create(db: &Connection, input: CreateCampaignRunInput) -> AppResult<CampaignRun> {
    validate_create(&input)?;
    let id = uuid::Uuid::new_v4().to_string();
    let timestamp = now()?;
    db.execute(
        "INSERT INTO campaign_runs
         (id, definition_id, definition_version, title, current_step_id, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
        params![
            id,
            input.definition_id.trim(),
            input.definition_version.trim(),
            input.title.trim(),
            input.current_step_id.trim(),
            timestamp,
        ],
    )
    .map_err(fail)?;
    assemble(db, &id)
}

pub fn get(db: &Connection, id: &str) -> AppResult<CampaignRun> {
    assemble(db, id)
}

pub fn list(db: &Connection) -> AppResult<Vec<CampaignRun>> {
    let mut statement = db
        .prepare("SELECT id FROM campaign_runs ORDER BY updated_at DESC, id ASC")
        .map_err(fail)?;
    let ids = statement
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(fail)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(fail)?;
    drop(statement);
    ids.iter().map(|id| assemble(db, id)).collect()
}

pub fn save(db: &mut Connection, run: CampaignRun) -> AppResult<CampaignRun> {
    validate_run(&run)?;
    let timestamp = now()?;
    let tx = db.transaction().map_err(fail)?;
    let existing: (String, String, i64) = tx
        .query_row(
            "SELECT definition_id, definition_version, created_at FROM campaign_runs WHERE id = ?1",
            [&run.id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()
        .map_err(fail)?
        .ok_or_else(|| AppError::not_found("Campaign run not found."))?;
    if existing
        != (
            run.definition_id.clone(),
            run.definition_version.clone(),
            run.created_at as i64,
        )
    {
        return Err(AppError::conflict(
            "Campaign definition identity is immutable.",
        ));
    }

    tx.execute(
        "UPDATE campaign_runs SET title = ?2, current_step_id = ?3, updated_at = ?4 WHERE id = ?1",
        params![
            run.id,
            run.title.trim(),
            run.current_step_id.trim(),
            timestamp
        ],
    )
    .map_err(fail)?;
    tx.execute(
        "DELETE FROM campaign_run_steps WHERE run_id = ?1",
        [&run.id],
    )
    .map_err(fail)?;

    for step_id in BTreeSet::<_>::from_iter(run.completed_step_ids.iter().map(String::as_str)) {
        tx.execute(
            "INSERT INTO campaign_run_steps (run_id, step_id, status, draft_override, updated_at)
             VALUES (?1, ?2, 'completed', NULL, ?3)",
            params![run.id, step_id, timestamp],
        )
        .map_err(fail)?;
    }
    for step_id in BTreeSet::<_>::from_iter(run.passed_challenge_ids.iter().map(String::as_str)) {
        tx.execute(
            "INSERT INTO campaign_run_steps (run_id, step_id, status, draft_override, updated_at)
             VALUES (?1, ?2, 'passed', NULL, ?3)",
            params![run.id, step_id, timestamp],
        )
        .map_err(fail)?;
    }
    for (step_id, draft) in &run.draft_overrides_by_step_id {
        tx.execute(
            "INSERT INTO campaign_run_steps (run_id, step_id, status, draft_override, updated_at)
             VALUES (?1, ?2, 'draft', ?3, ?4)",
            params![run.id, step_id, draft, timestamp],
        )
        .map_err(fail)?;
    }
    tx.commit().map_err(fail)?;
    assemble(db, &run.id)
}

pub fn delete(db: &Connection, id: &str) -> AppResult<()> {
    let deleted = db
        .execute("DELETE FROM campaign_runs WHERE id = ?1", [id])
        .map_err(fail)?;
    if deleted == 0 {
        Err(AppError::not_found("Campaign run not found."))
    } else {
        Ok(())
    }
}

pub fn delete_with_navigation(db: &mut Connection, id: &str) -> AppResult<()> {
    let tx = db.transaction().map_err(fail)?;
    delete(&tx, id)?;
    if get_active_project_navigation(&tx)?
        .is_some_and(|navigation| navigation.kind == "campaign" && navigation.id == id)
    {
        clear_active_project_navigation(&tx)?;
    }
    tx.commit().map_err(fail)
}

fn validate_active_project(navigation: &ActiveProjectNavigation) -> AppResult<()> {
    required(&navigation.id, "project ID")?;
    match (navigation.kind.as_str(), navigation.view.as_str()) {
        ("design", "workbench") | ("campaign", "campaign") => Ok(()),
        _ => Err(AppError::validation(
            "Active Project kind and view are incompatible.",
        )),
    }
}

pub fn get_active_project_navigation(
    db: &Connection,
) -> AppResult<Option<ActiveProjectNavigation>> {
    db.query_row(
        "SELECT kind, project_id, view FROM active_project_navigation WHERE slot = 1",
        [],
        |row| {
            Ok(ActiveProjectNavigation {
                kind: row.get(0)?,
                id: row.get(1)?,
                view: row.get(2)?,
            })
        },
    )
    .optional()
    .map_err(fail)
}

pub fn save_active_project_navigation(
    db: &Connection,
    navigation: ActiveProjectNavigation,
) -> AppResult<ActiveProjectNavigation> {
    validate_active_project(&navigation)?;
    let timestamp = now()?;
    db.execute(
        "INSERT INTO active_project_navigation (slot, kind, project_id, view, updated_at)
         VALUES (1, ?1, ?2, ?3, ?4)
         ON CONFLICT(slot) DO UPDATE SET
           kind = excluded.kind, project_id = excluded.project_id,
           view = excluded.view, updated_at = excluded.updated_at",
        params![navigation.kind, navigation.id, navigation.view, timestamp],
    )
    .map_err(fail)?;
    get_active_project_navigation(db)?
        .ok_or_else(|| AppError::persistence("Active Project save failed."))
}

pub fn clear_active_project_navigation(db: &Connection) -> AppResult<()> {
    db.execute("DELETE FROM active_project_navigation WHERE slot = 1", [])
        .map_err(fail)?;
    Ok(())
}

pub fn get_app_window_layout(db: &Connection) -> AppResult<Option<ThreadWindowLayout>> {
    db.query_row(
        "SELECT layout_json FROM app_window_layouts WHERE slot = 1",
        [],
        |row| {
            let raw: String = row.get(0)?;
            serde_json::from_str(&raw).map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(
                    0,
                    rusqlite::types::Type::Text,
                    Box::new(error),
                )
            })
        },
    )
    .optional()
    .map_err(fail)
}

pub fn save_app_window_layout(db: &Connection, layout: ThreadWindowLayout) -> AppResult<()> {
    let raw =
        serde_json::to_string(&layout).map_err(|error| AppError::persistence(error.to_string()))?;
    db.execute(
        "INSERT INTO app_window_layouts (slot, layout_json, updated_at) VALUES (1, ?1, ?2)
         ON CONFLICT(slot) DO UPDATE SET layout_json = excluded.layout_json, updated_at = excluded.updated_at",
        params![raw, now()?],
    )
    .map_err(fail)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn database() -> Connection {
        let db = Connection::open_in_memory().unwrap();
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
             );
             CREATE TABLE app_window_layouts (
                slot INTEGER PRIMARY KEY CHECK(slot = 1), layout_json TEXT NOT NULL,
                updated_at INTEGER NOT NULL
             );",
        )
        .unwrap();
        db
    }

    fn create_input(title: &str) -> CreateCampaignRunInput {
        CreateCampaignRunInput {
            title: title.to_owned(),
            definition_id: "ecky-field-guide".to_owned(),
            definition_version: "2026-08-02".to_owned(),
            current_step_id: "m01-bracket".to_owned(),
        }
    }

    #[test]
    fn campaign_runs_persist_resume_list_and_delete_without_thread_rows() {
        let mut db = database();
        db.execute("INSERT INTO campaign_runs VALUES ('sentinel', 'd', 'v', 'design API untouched', 'x', 1, 1)", [])
            .unwrap();
        db.execute("DELETE FROM campaign_runs WHERE id = 'sentinel'", [])
            .unwrap();
        let first = create(&db, create_input("First")).unwrap();
        let second = create(&db, create_input("Second")).unwrap();
        assert_ne!(first.id, second.id);
        assert!(uuid::Uuid::parse_str(&first.id).is_ok());

        let mut changed = first.clone();
        changed.current_step_id = "m02-enclosure".to_owned();
        changed.completed_step_ids = vec!["m01-bracket".to_owned()];
        changed.passed_challenge_ids = vec!["m01-bracket".to_owned(), "m02-enclosure".to_owned()];
        changed
            .draft_overrides_by_step_id
            .insert("m02-enclosure".to_owned(), "(model ...)".to_owned());
        let saved = save(&mut db, changed).unwrap();
        assert_eq!(saved.current_step_id, "m02-enclosure");
        assert_eq!(saved.completed_step_ids, vec!["m01-bracket"]);
        assert_eq!(
            saved.passed_challenge_ids,
            vec!["m01-bracket", "m02-enclosure"]
        );
        assert_eq!(
            saved
                .draft_overrides_by_step_id
                .get("m02-enclosure")
                .unwrap(),
            "(model ...)"
        );
        assert_eq!(get(&db, &first.id).unwrap(), saved);
        assert_eq!(list(&db).unwrap().len(), 2);

        delete(&db, &first.id).unwrap();
        assert!(
            matches!(get(&db, &first.id), Err(error) if error.code == crate::contracts::AppErrorCode::NotFound)
        );
        let orphan_count: i64 = db
            .query_row(
                "SELECT COUNT(*) FROM campaign_run_steps WHERE run_id = ?1",
                [&first.id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(orphan_count, 0);
    }

    #[test]
    fn deleting_active_campaign_clears_navigation_atomically() {
        let mut db = database();
        let run = create(&db, create_input("Active")).unwrap();
        save_active_project_navigation(
            &db,
            ActiveProjectNavigation {
                kind: "campaign".to_string(),
                id: run.id.clone(),
                view: "campaign".to_string(),
            },
        )
        .unwrap();

        delete_with_navigation(&mut db, &run.id).unwrap();

        assert!(matches!(get(&db, &run.id), Err(AppError { .. })));
        assert_eq!(get_active_project_navigation(&db).unwrap(), None);
    }

    #[test]
    fn active_campaign_navigation_and_app_layout_roundtrip_without_thread_rows() {
        let db = database();
        let navigation = save_active_project_navigation(
            &db,
            ActiveProjectNavigation {
                kind: "campaign".to_owned(),
                id: "campaign-run".to_owned(),
                view: "campaign".to_owned(),
            },
        )
        .unwrap();
        assert_eq!(
            get_active_project_navigation(&db).unwrap(),
            Some(navigation)
        );

        let layout = ThreadWindowLayout {
            schema_version: 1,
            remember_layout: true,
            windows: std::collections::HashMap::new(),
        };
        save_app_window_layout(&db, layout.clone()).unwrap();
        assert_eq!(get_app_window_layout(&db).unwrap(), Some(layout));

        clear_active_project_navigation(&db).unwrap();
        assert_eq!(get_active_project_navigation(&db).unwrap(), None);
    }
}
