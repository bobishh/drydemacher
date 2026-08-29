use ecky_cad_lib::thread_source_binding::{
    clear_project_folder_watch_failure, ensure_schema, get_project_folder_watch_failure,
    set_project_folder_watch_failure,
};
use rusqlite::Connection;

#[test]
fn failed_source_digest_survives_watcher_restart_until_source_changes() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(
        "PRAGMA foreign_keys = ON;
         CREATE TABLE threads (id TEXT PRIMARY KEY);
         INSERT INTO threads (id) VALUES ('thread-1');",
    )
    .unwrap();
    ensure_schema(&conn).unwrap();

    set_project_folder_watch_failure(
        &conn,
        "thread-1",
        "sha256:failed",
        "PREVIEW_STL_NON_MANIFOLD",
        100,
    )
    .unwrap();
    let failure = get_project_folder_watch_failure(&conn, "thread-1")
        .unwrap()
        .expect("durable failed digest");
    assert_eq!(failure.source_digest, "sha256:failed");
    assert_eq!(failure.error, "PREVIEW_STL_NON_MANIFOLD");

    clear_project_folder_watch_failure(&conn, "thread-1").unwrap();
    assert!(get_project_folder_watch_failure(&conn, "thread-1")
        .unwrap()
        .is_none());
}
