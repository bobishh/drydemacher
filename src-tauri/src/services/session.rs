use crate::contracts::{AuthoringTargetRef, LastDesignSnapshot};
use crate::db;
use crate::models::PathResolver;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

const RESTART_POINTER_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct LastDesignPointer {
    schema_version: u32,
    target: AuthoringTargetRef,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    selected_part_id: Option<String>,
}

pub fn last_snapshot_path(app: &dyn PathResolver) -> PathBuf {
    app.app_config_dir().join("last_design.json")
}

pub fn write_last_snapshot(app: &dyn PathResolver, snapshot: Option<&LastDesignSnapshot>) {
    let path = last_snapshot_path(app);
    match snapshot {
        Some(snapshot) => {
            if let Some(serialized) = serialize_restart_pointer(snapshot) {
                let _ = fs::write(path, serialized);
            } else {
                let _ = fs::remove_file(path);
            }
        }
        None => {
            let _ = fs::remove_file(path);
        }
    }
}

pub fn read_last_snapshot(app: &dyn PathResolver, conn: &Connection) -> Option<LastDesignSnapshot> {
    let data = fs::read_to_string(last_snapshot_path(app)).ok()?;
    let pointer = serde_json::from_str::<LastDesignPointer>(&data)
        .ok()
        .or_else(|| legacy_pointer(&data))?;
    resolve_restart_pointer(conn, &pointer)
}

fn serialize_restart_pointer(snapshot: &LastDesignSnapshot) -> Option<String> {
    let target = snapshot.target_ref.clone()?;
    serde_json::to_string_pretty(&LastDesignPointer {
        schema_version: RESTART_POINTER_SCHEMA_VERSION,
        target,
        selected_part_id: snapshot.selected_part_id.clone(),
    })
    .ok()
}

fn legacy_pointer(data: &str) -> Option<LastDesignPointer> {
    let snapshot = serde_json::from_str::<LastDesignSnapshot>(data).ok()?;
    let target = snapshot.target_ref.or_else(|| {
        Some(AuthoringTargetRef::SavedVersion {
            thread_id: snapshot.thread_id?,
            message_id: snapshot.message_id?,
        })
    })?;
    Some(LastDesignPointer {
        schema_version: RESTART_POINTER_SCHEMA_VERSION,
        target,
        selected_part_id: snapshot.selected_part_id,
    })
}

fn resolve_restart_pointer(
    conn: &Connection,
    pointer: &LastDesignPointer,
) -> Option<LastDesignSnapshot> {
    match &pointer.target {
        AuthoringTargetRef::Draft {
            thread_id,
            preview_id,
            session_id,
        } => {
            let draft = db::get_agent_draft_by_preview_id(conn, preview_id).ok()??;
            if &draft.thread_id != thread_id || &draft.session_id != session_id {
                return None;
            }
            Some(LastDesignSnapshot {
                design: Some(draft.design_output),
                thread_id: Some(draft.thread_id),
                message_id: Some(draft.preview_id),
                artifact_bundle: Some(draft.artifact_bundle),
                model_manifest: Some(draft.model_manifest),
                selected_part_id: pointer.selected_part_id.clone(),
                target_ref: Some(pointer.target.clone()),
            })
        }
        AuthoringTargetRef::SavedVersion {
            thread_id,
            message_id,
        } => resolve_saved_pointer(conn, thread_id, message_id, pointer),
        AuthoringTargetRef::LatestSaved { thread_id } => {
            let message_id =
                db::get_latest_successful_message_id_in_thread(conn, thread_id).ok()??;
            resolve_saved_pointer(conn, thread_id, &message_id, pointer)
        }
    }
}

fn resolve_saved_pointer(
    conn: &Connection,
    thread_id: &str,
    message_id: &str,
    pointer: &LastDesignPointer,
) -> Option<LastDesignSnapshot> {
    let (design, output_thread_id) = db::get_message_output_and_thread(conn, message_id).ok()??;
    let (artifact_bundle, model_manifest, runtime_thread_id) =
        db::get_message_runtime_and_thread(conn, message_id).ok()??;
    if output_thread_id != thread_id || runtime_thread_id != thread_id {
        return None;
    }
    Some(LastDesignSnapshot {
        design: Some(design),
        thread_id: Some(thread_id.to_string()),
        message_id: Some(message_id.to_string()),
        artifact_bundle,
        model_manifest,
        selected_part_id: pointer.selected_part_id.clone(),
        target_ref: Some(AuthoringTargetRef::SavedVersion {
            thread_id: thread_id.to_string(),
            message_id: message_id.to_string(),
        }),
    })
}

pub fn build_runtime_snapshot(
    design: Option<crate::contracts::DesignOutput>,
    thread_id: Option<String>,
    message_id: Option<String>,
    artifact_bundle: Option<crate::contracts::ArtifactBundle>,
    model_manifest: Option<crate::contracts::ModelManifest>,
    selected_part_id: Option<String>,
) -> LastDesignSnapshot {
    LastDesignSnapshot {
        design,
        thread_id,
        message_id,
        artifact_bundle,
        model_manifest,
        selected_part_id,
        target_ref: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::{
        AgentDraft, ArtifactBundle, AuthoringTargetRef, DesignOutput, EngineKind, GeometryBackend,
        InteractionMode, MacroDialect, ModelManifest, SourceLanguage, UiSpec,
    };

    fn design() -> DesignOutput {
        DesignOutput {
            title: "Recovered".to_string(),
            version_name: "V1".to_string(),
            response: String::new(),
            interaction_mode: InteractionMode::Design,
            macro_code: "(model (part body (box width 10 10)))".to_string(),
            macro_dialect: MacroDialect::EckyIrV0,
            engine_kind: EngineKind::EckyIrV0,
            source_language: SourceLanguage::EckyIrV0,
            geometry_backend: GeometryBackend::EckyRust,
            ui_spec: UiSpec::default(),
            initial_params: Default::default(),
            post_processing: None,
        }
    }

    fn bundle() -> ArtifactBundle {
        serde_json::from_value(serde_json::json!({
            "modelId": "model-1",
            "sourceKind": "generated",
            "engineKind": "eckyIrV0",
            "sourceLanguage": "eckyIrV0",
            "geometryBackend": "eckyRust",
            "contentHash": "sha256:artifact",
            "fcstdPath": "",
            "manifestPath": "/tmp/model-1.json",
            "previewStlPath": "/tmp/model-1.stl"
        }))
        .unwrap()
    }

    fn manifest() -> ModelManifest {
        serde_json::from_value(serde_json::json!({
            "modelId": "model-1",
            "sourceKind": "generated",
            "engineKind": "eckyIrV0",
            "sourceLanguage": "eckyIrV0",
            "geometryBackend": "eckyRust",
            "document": {
                "documentName": "Recovered",
                "documentLabel": "Recovered",
                "objectCount": 0,
                "warnings": []
            }
        }))
        .unwrap()
    }

    #[test]
    fn restart_cache_serializes_only_tagged_pointer() {
        let snapshot = LastDesignSnapshot {
            design: None,
            thread_id: Some("thread-1".to_string()),
            message_id: Some("preview-1".to_string()),
            artifact_bundle: None,
            model_manifest: None,
            selected_part_id: Some("part-1".to_string()),
            target_ref: Some(AuthoringTargetRef::Draft {
                thread_id: "thread-1".to_string(),
                preview_id: "preview-1".to_string(),
                session_id: "session-1".to_string(),
            }),
        };

        let serialized = serialize_restart_pointer(&snapshot).expect("tagged pointer");
        let value: serde_json::Value = serde_json::from_str(&serialized).unwrap();

        assert_eq!(value["schemaVersion"], 1);
        assert_eq!(value["target"]["kind"], "draft");
        assert_eq!(value["target"]["previewId"], "preview-1");
        assert_eq!(value["selectedPartId"], "part-1");
        assert!(value.get("design").is_none());
        assert!(value.get("artifactBundle").is_none());
        assert!(value.get("modelManifest").is_none());
    }

    #[test]
    fn untagged_snapshot_cannot_become_restart_authority() {
        let snapshot = LastDesignSnapshot {
            design: None,
            thread_id: Some("thread-1".to_string()),
            message_id: Some("message-1".to_string()),
            artifact_bundle: None,
            model_manifest: None,
            selected_part_id: None,
            target_ref: None,
        };

        assert!(serialize_restart_pointer(&snapshot).is_none());
    }

    #[test]
    fn saved_pointer_recovers_from_database_without_payload_cache() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE threads (id TEXT PRIMARY KEY, deleted_at INTEGER);
             CREATE TABLE messages (
               id TEXT PRIMARY KEY,
               thread_id TEXT NOT NULL,
               status TEXT NOT NULL,
               output TEXT,
               artifact_bundle TEXT,
               model_manifest TEXT,
               deleted_at INTEGER
             );",
        )
        .unwrap();
        conn.execute("INSERT INTO threads (id) VALUES ('thread-1')", [])
            .unwrap();
        conn.execute(
            "INSERT INTO messages (
               id, thread_id, status, output, artifact_bundle, model_manifest
             ) VALUES (?1, ?2, 'success', ?3, ?4, ?5)",
            rusqlite::params![
                "message-1",
                "thread-1",
                serde_json::to_string(&design()).unwrap(),
                serde_json::to_string(&bundle()).unwrap(),
                serde_json::to_string(&manifest()).unwrap(),
            ],
        )
        .unwrap();

        let pointer = LastDesignPointer {
            schema_version: RESTART_POINTER_SCHEMA_VERSION,
            target: AuthoringTargetRef::SavedVersion {
                thread_id: "thread-1".to_string(),
                message_id: "message-1".to_string(),
            },
            selected_part_id: None,
        };

        let recovered = resolve_restart_pointer(&conn, &pointer).expect("durable saved version");
        assert_eq!(recovered.message_id.as_deref(), Some("message-1"));
        assert_eq!(recovered.design.unwrap().title, "Recovered");
        assert_eq!(recovered.artifact_bundle.unwrap().model_id, "model-1");
    }

    #[test]
    fn draft_pointer_recovers_from_durable_draft_without_process_cache() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE agent_drafts (
               preview_id TEXT PRIMARY KEY,
               session_id TEXT NOT NULL,
               thread_id TEXT NOT NULL,
               base_message_id TEXT,
               design_output TEXT NOT NULL,
               artifact_bundle TEXT NOT NULL,
               model_manifest TEXT NOT NULL,
               draft_feedback TEXT,
               updated_at INTEGER NOT NULL,
               UNIQUE(session_id, thread_id)
             );",
        )
        .unwrap();
        db::upsert_agent_draft(
            &conn,
            &AgentDraft {
                preview_id: "preview-1".to_string(),
                session_id: "session-1".to_string(),
                thread_id: "thread-1".to_string(),
                base_message_id: Some("message-1".to_string()),
                design_output: design(),
                artifact_bundle: bundle(),
                model_manifest: manifest(),
                draft_feedback: None,
                updated_at: 1,
            },
        )
        .unwrap();

        let pointer = LastDesignPointer {
            schema_version: RESTART_POINTER_SCHEMA_VERSION,
            target: AuthoringTargetRef::Draft {
                thread_id: "thread-1".to_string(),
                preview_id: "preview-1".to_string(),
                session_id: "session-1".to_string(),
            },
            selected_part_id: None,
        };

        let recovered = resolve_restart_pointer(&conn, &pointer).expect("durable draft");
        assert_eq!(recovered.message_id.as_deref(), Some("preview-1"));
        assert_eq!(recovered.design.unwrap().title, "Recovered");
        assert_eq!(recovered.model_manifest.unwrap().model_id, "model-1");
    }
}
