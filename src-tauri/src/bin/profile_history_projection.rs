use ecky_cad_lib::{db, services::history};
use std::path::PathBuf;
use std::time::Instant;

fn main() {
    let db_path = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            eprintln!("usage: profile_history_projection <isolated-history.sqlite>");
            std::process::exit(2);
        });
    let conn = db::init_db(&db_path).unwrap_or_else(|error| {
        eprintln!("failed to open isolated projection fixture: {error}");
        std::process::exit(1);
    });
    if std::env::args().any(|argument| argument == "--open-only") {
        println!("projection_open_only");
        return;
    }
    let mode = std::env::args().nth(2);
    let started = Instant::now();
    let summaries = (mode.as_deref() != Some("--page")
        && mode.as_deref() != Some("--detail")
        && mode.as_deref() != Some("--topology"))
    .then(|| history::get_history(&conn).expect("bounded summaries"));
    let page = (mode.as_deref() != Some("--summary")
        && mode.as_deref() != Some("--detail")
        && mode.as_deref() != Some("--topology"))
    .then(|| {
        history::get_thread_messages_page(&conn, "large-history-thread", None, Some(50), false)
            .expect("bounded timeline page")
    });
    let detail = (mode.as_deref() != Some("--summary")
        && mode.as_deref() != Some("--page")
        && mode.as_deref() != Some("--topology"))
    .then(|| {
        history::get_version_detail(&conn, "large-history-thread", "version-149")
            .expect("bounded version detail")
    });
    let topology = (mode.as_deref() == Some("--topology")).then(|| {
        history::get_dense_topology_page(
            &conn,
            "large-history-thread",
            "version-149",
            ecky_cad_lib::contracts::DenseTopologyKind::Selection,
            None,
            Some(500),
        )
        .expect("bounded topology page")
    });
    println!(
        "projection_read elapsed_ms={} summary_bytes={} timeline_bytes={} detail_bytes={} topology_bytes={} topology_count={}",
        started.elapsed().as_millis(),
        summaries.as_ref().map_or(0, |value| serde_json::to_vec(value).expect("summary serialization").len()),
        page.as_ref().map_or(0, |value| serde_json::to_vec(value).expect("timeline serialization").len()),
        detail.as_ref().map_or(0, |value| serde_json::to_vec(value).expect("detail serialization").len()),
        topology.as_ref().map_or(0, |value| serde_json::to_vec(value).expect("topology serialization").len()),
        topology.as_ref().map_or_else(
            || detail.as_ref().map_or(0, |value| value.selection_target_count),
            |value| value.total_count,
        ),
    );
}
