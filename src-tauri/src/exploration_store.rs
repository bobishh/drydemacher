//! Durable exploration-controller state and append-only event log.

use crate::contracts::exploration_cycle::{
    CycleEvent, CyclePacket, CyclePhase, CycleStatus, Verification,
};
use rusqlite::{params, Connection, OptionalExtension};

pub fn ensure_schema(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS exploration_cycles (
            cycle_id TEXT PRIMARY KEY,
            thread_id TEXT NOT NULL REFERENCES threads(id) ON DELETE CASCADE,
            packet_json TEXT NOT NULL,
            build_started INTEGER NOT NULL DEFAULT 0,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_exploration_cycles_thread_updated
            ON exploration_cycles(thread_id, updated_at DESC);
        CREATE TABLE IF NOT EXISTS exploration_cycle_events (
            event_id TEXT PRIMARY KEY,
            cycle_id TEXT NOT NULL REFERENCES exploration_cycles(cycle_id) ON DELETE CASCADE,
            sequence INTEGER NOT NULL,
            event_json TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            UNIQUE(cycle_id, sequence)
        );
        CREATE INDEX IF NOT EXISTS idx_exploration_cycle_events_cycle_sequence
            ON exploration_cycle_events(cycle_id, sequence);",
    )
}

pub fn validate_version_ref(
    conn: &Connection,
    thread_id: &str,
    version_id: &str,
) -> rusqlite::Result<bool> {
    conn.query_row(
        "SELECT EXISTS(
            SELECT 1 FROM messages
            WHERE id = ?1 AND thread_id = ?2 AND role = 'assistant'
              AND output IS NOT NULL AND deleted_at IS NULL
        )",
        params![version_id, thread_id],
        |row| row.get(0),
    )
}

pub fn insert_cycle(
    conn: &Connection,
    packet: &CyclePacket,
    event: &CycleEvent,
) -> rusqlite::Result<()> {
    let packet_json = to_json(packet)?;
    let event_json = to_json(event)?;
    conn.execute_batch("SAVEPOINT insert_exploration_cycle")?;
    let result = (|| {
        conn.execute(
            "INSERT INTO exploration_cycles
             (cycle_id, thread_id, packet_json, build_started, created_at, updated_at)
             VALUES (?1, ?2, ?3, 0, ?4, ?4)",
            params![
                packet.state.cycle_id,
                packet.state.thread_id,
                packet_json,
                event.timestamp as i64
            ],
        )?;
        insert_event(conn, event, &event_json)?;
        Ok(())
    })();
    finish_savepoint(conn, "insert_exploration_cycle", result)
}

pub fn load_cycle(
    conn: &Connection,
    cycle_id: &str,
) -> rusqlite::Result<Option<(CyclePacket, bool)>> {
    conn.query_row(
        "SELECT packet_json, build_started FROM exploration_cycles WHERE cycle_id = ?1",
        [cycle_id],
        |row| {
            let raw: String = row.get(0)?;
            let packet = from_json(&raw)?;
            let build_started: i64 = row.get(1)?;
            Ok((packet, build_started != 0))
        },
    )
    .optional()
}

pub fn load_latest_active_cycle_for_thread(
    conn: &Connection,
    thread_id: &str,
) -> rusqlite::Result<Option<(CyclePacket, bool)>> {
    conn.query_row(
        "SELECT packet_json, build_started FROM exploration_cycles
         WHERE thread_id = ?1
           AND json_extract(packet_json, '$.state.status') = 'active'
         ORDER BY updated_at DESC LIMIT 1",
        [thread_id],
        |row| {
            let raw: String = row.get(0)?;
            let packet = from_json(&raw)?;
            let build_started: i64 = row.get(1)?;
            Ok((packet, build_started != 0))
        },
    )
    .optional()
}

pub fn save_transition(
    conn: &Connection,
    packet: &CyclePacket,
    build_started: bool,
    event: &CycleEvent,
) -> rusqlite::Result<()> {
    let packet_json = to_json(packet)?;
    let event_json = to_json(event)?;
    conn.execute_batch("SAVEPOINT save_exploration_transition")?;
    let result = (|| {
        let changed = conn.execute(
            "UPDATE exploration_cycles
             SET packet_json = ?1, build_started = ?2, updated_at = ?3
             WHERE cycle_id = ?4",
            params![
                packet_json,
                i64::from(build_started),
                event.timestamp as i64,
                packet.state.cycle_id
            ],
        )?;
        if changed != 1 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        insert_event(conn, event, &event_json)?;
        Ok(())
    })();
    finish_savepoint(conn, "save_exploration_transition", result)
}

pub fn list_events(
    conn: &Connection,
    cycle_id: &str,
    after_sequence: u64,
    limit: usize,
) -> rusqlite::Result<Vec<CycleEvent>> {
    let bounded = limit.clamp(1, 200) as i64;
    let mut statement = conn.prepare(
        "SELECT event_json FROM exploration_cycle_events
         WHERE cycle_id = ?1 AND sequence > ?2
         ORDER BY sequence ASC LIMIT ?3",
    )?;
    let events = statement
        .query_map(params![cycle_id, after_sequence as i64, bounded], |row| {
            let raw: String = row.get(0)?;
            from_json(&raw)
        })?
        .collect();
    events
}

pub fn mark_in_flight_interrupted(conn: &Connection, now: u64) -> rusqlite::Result<usize> {
    let mut statement =
        conn.prepare("SELECT cycle_id, packet_json, build_started FROM exploration_cycles")?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)? != 0,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    drop(statement);

    let mut changed = 0;
    for (cycle_id, raw, build_started) in rows {
        let mut packet: CyclePacket = from_json(&raw)?;
        let in_flight = build_started
            || matches!(
                packet.state.phase,
                CyclePhase::Building | CyclePhase::Verifying | CyclePhase::Deciding
            );
        if packet.state.status != CycleStatus::Active || !in_flight {
            continue;
        }
        packet.state.status = CycleStatus::Interrupted;
        packet.state.phase = CyclePhase::Idle;
        packet.event_count += 1;
        let event = CycleEvent {
            event_id: uuid::Uuid::new_v4().to_string(),
            cycle_id: cycle_id.clone(),
            sequence: packet.event_count,
            event_type: crate::contracts::exploration_cycle::CycleEventType::Interrupted,
            phase: CyclePhase::Idle,
            source_version_id: Some(packet.state.current_version_id.clone()),
            result_version_id: None,
            evidence_ref: packet.state.last_evidence_ref.clone(),
            raw_error: Some(
                "Cycle interrupted by app restart; expensive work was not resumed.".into(),
            ),
            render_snapshot_id: None,
            artifact_digest: None,
            route: None,
            plan: None,
            question: None,
            blocked_decision: None,
            answer: None,
            timestamp: now,
        };
        save_transition(conn, &packet, false, &event)?;
        changed += 1;
    }
    Ok(changed)
}

fn insert_event(conn: &Connection, event: &CycleEvent, event_json: &str) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO exploration_cycle_events
         (event_id, cycle_id, sequence, event_json, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            event.event_id,
            event.cycle_id,
            event.sequence as i64,
            event_json,
            event.timestamp as i64
        ],
    )?;
    Ok(())
}

fn finish_savepoint(
    conn: &Connection,
    name: &str,
    result: rusqlite::Result<()>,
) -> rusqlite::Result<()> {
    match result {
        Ok(()) => conn.execute_batch(&format!("RELEASE {name}")),
        Err(error) => {
            let _ = conn.execute_batch(&format!("ROLLBACK TO {name}; RELEASE {name}"));
            Err(error)
        }
    }
}

fn to_json<T: serde::Serialize>(value: &T) -> rusqlite::Result<String> {
    serde_json::to_string(value)
        .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))
}

fn from_json<T: serde::de::DeserializeOwned>(raw: &str) -> rusqlite::Result<T> {
    serde_json::from_str(raw).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(error))
    })
}

#[allow(dead_code)]
fn _verification_type_anchor(_: Option<Verification>) {}
