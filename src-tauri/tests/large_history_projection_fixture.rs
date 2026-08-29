use ecky_cad_lib::{db, services::history, transport_budget};
use rusqlite::params;
use std::path::PathBuf;
use std::time::Instant;

fn isolated_fixture(
    dense_target_count: usize,
    source_bytes: usize,
) -> (rusqlite::Connection, String, PathBuf) {
    let db_path = std::env::var_os("ECKY_PROJECTION_FIXTURE_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::temp_dir().join(format!(
                "ecky-bounded-history-fixture-{}.sqlite",
                uuid::Uuid::new_v4()
            ))
        });
    let _ = std::fs::remove_file(&db_path);
    let _ = std::fs::remove_file(format!("{}-wal", db_path.display()));
    let _ = std::fs::remove_file(format!("{}-shm", db_path.display()));
    let conn = db::init_db(&db_path).expect("isolated fixture db");
    let thread_id = "large-history-thread".to_string();
    db::create_or_update_thread(&conn, &thread_id, "Large fixture", 1, None).unwrap();
    let source = "x".repeat(source_bytes);
    for index in 0..150 {
        let output = serde_json::json!({
            "title": format!("Version {index}"),
            "versionName": format!("V{index}"),
            "response": "fixture",
            "macroCode": source,
        });
        conn.execute(
            "INSERT INTO messages(id, thread_id, role, content, status, output, timestamp)
             VALUES (?1, ?2, 'assistant', ?3, 'success', ?4, 100)",
            params![
                format!("version-{index:03}"),
                thread_id,
                source,
                output.to_string()
            ],
        )
        .unwrap();
    }
    let target = serde_json::json!({
        "targetId": "dense",
        "partId": "body",
        "viewerNodeId": "Body",
        "label": "Dense",
        "kind": "face",
        "editable": false
    });
    let dense_targets = vec![target; dense_target_count];
    let manifest = serde_json::json!({
        "modelId": "fixture-model",
        "sourceKind": "generated",
        "document": { "documentName": "Fixture", "documentLabel": "Fixture" },
        "selectionTargets": dense_targets,
        "enrichmentState": { "status": "none", "proposals": [] }
    });
    conn.execute(
        "UPDATE messages SET model_manifest = ?1 WHERE id = 'version-149'",
        [manifest.to_string()],
    )
    .unwrap();
    conn.execute(
        "DELETE FROM schema_migrations WHERE key = 'binary-cad-payload-v1'",
        [],
    )
    .unwrap();
    drop(conn);
    let conn = db::init_db(&db_path).expect("one-time binary payload migration");
    (conn, thread_id, db_path)
}

fn remove_fixture(conn: rusqlite::Connection, db_path: PathBuf) {
    drop(conn);
    for path in [
        db_path.clone(),
        PathBuf::from(format!("{}-wal", db_path.display())),
        PathBuf::from(format!("{}-shm", db_path.display())),
    ] {
        let _ = std::fs::remove_file(path);
    }
}

#[test]
fn bounded_projection_fixture_proves_equal_timestamp_paging_and_lazy_dense_targets() {
    let (conn, thread_id, db_path) = isolated_fixture(2_000, 16 * 1024);
    let first =
        history::get_thread_messages_page(&conn, &thread_id, None, Some(50), false).unwrap();
    let second = history::get_thread_messages_page(
        &conn,
        &thread_id,
        first.next_before.clone(),
        Some(50),
        false,
    )
    .unwrap();
    assert_eq!(first.messages.len(), 50);
    assert_eq!(second.messages.len(), 50);
    let first_ids = first
        .messages
        .iter()
        .map(|row| &row.id)
        .collect::<std::collections::HashSet<_>>();
    assert!(second
        .messages
        .iter()
        .all(|row| !first_ids.contains(&row.id)));
    assert!(first.observed_bytes <= transport_budget::TIMELINE_PAGE_MAX_BYTES);

    let detail = history::get_version_detail(&conn, &thread_id, "version-149").unwrap();
    assert_eq!(detail.selection_target_count, 2_000);
    assert!(detail
        .message
        .model_manifest
        .as_ref()
        .unwrap()
        .selection_targets
        .is_empty());
    let page = history::get_dense_topology_page(
        &conn,
        &thread_id,
        "version-149",
        ecky_cad_lib::contracts::DenseTopologyKind::Selection,
        None,
        Some(500),
    )
    .unwrap();
    assert_eq!(page.items.len(), 500);
    assert_eq!(page.total_count, 2_000);
    assert!(page.observed_bytes <= transport_budget::TOPOLOGY_PAGE_MAX_BYTES);
    remove_fixture(conn, db_path);
}

#[test]
#[ignore = "explicit projection profiler: builds two million dense targets in an isolated temp database"]
fn profile_two_million_dense_targets_without_touching_live_history() {
    let fixture_started = Instant::now();
    let (conn, thread_id, db_path) = isolated_fixture(2_000_000, 256 * 1024);
    let query_started = Instant::now();
    let summaries = history::get_history(&conn).unwrap();
    let page = history::get_thread_messages_page(&conn, &thread_id, None, Some(50), false).unwrap();
    let detail = history::get_version_detail(&conn, &thread_id, "version-149").unwrap();
    assert_eq!(summaries.len(), 1);
    assert_eq!(page.messages.len(), 50);
    assert_eq!(detail.selection_target_count, 2_000_000);
    println!(
        "projection_profile fixture_ms={} query_ms={} summary_bytes={} timeline_bytes={} detail_bytes={} topology_count={}",
        fixture_started.elapsed().as_millis(),
        query_started.elapsed().as_millis(),
        serde_json::to_vec(&summaries).unwrap().len(),
        serde_json::to_vec(&page).unwrap().len(),
        serde_json::to_vec(&detail).unwrap().len(),
        detail.selection_target_count,
    );
    if std::env::var_os("ECKY_PROJECTION_FIXTURE_PATH").is_some() {
        drop(conn);
        println!("projection_fixture_path={}", db_path.display());
    } else {
        remove_fixture(conn, db_path);
    }
}
