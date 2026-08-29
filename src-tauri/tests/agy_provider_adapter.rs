use ecky_cad_lib::services::agy_provider::{
    parse_agy_version, project_stream_event, AgyProjectedEvent, AgyProviderSupervisor,
    MINIMUM_AGY_VERSION,
};
use serde_json::json;

static AGY_ENV_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

#[test]
fn version_gate_requires_bidirectional_stream_json_release() {
    assert_eq!(MINIMUM_AGY_VERSION, (1, 1, 15));
    assert_eq!(
        parse_agy_version("Antigravity CLI 1.1.15\n").unwrap(),
        (1, 1, 15)
    );
    assert_eq!(parse_agy_version("agy 2.0.3").unwrap(), (2, 0, 3));
    assert!(parse_agy_version("Antigravity CLI unknown").is_err());
}

#[test]
fn stream_projection_exposes_public_progress_but_not_tool_stdout() {
    let delta = project_stream_event(&json!({
        "event": "step_update",
        "step_update": {
            "conversation_id": "agy-7",
            "step_index": 4,
            "state": "DONE",
            "step_type": "agent_response",
            "text_delta": "Inspecting constraints."
        }
    }))
    .unwrap()
    .unwrap();
    assert_eq!(
        delta,
        AgyProjectedEvent::AssistantDelta {
            conversation_id: "agy-7".to_string(),
            step_index: 4,
            text: "Inspecting constraints.".to_string(),
        }
    );

    let tool = project_stream_event(&json!({
        "event": "step_update",
        "step_update": {
            "conversation_id": "agy-7",
            "step_index": 5,
            "state": "DONE",
            "step_type": "tool",
            "tool_name": "call_mcp_tool",
            "tool_info": {
                "input": {
                    "ServerName": "ecky_mcp",
                    "ToolName": "target_meta_get",
                    "Arguments": { "threadId": "thread-7" }
                },
                "output": "secret terminal dump"
            }
        }
    }))
    .unwrap()
    .unwrap();
    assert_eq!(
        tool,
        AgyProjectedEvent::Working {
            conversation_id: "agy-7".to_string(),
            step_index: 5,
            text: "USING TOOL · ecky_mcp/target_meta_get".to_string(),
        }
    );
    assert!(!format!("{tool:?}").contains("secret terminal dump"));
}

#[test]
fn stream_projection_prefers_public_actions_and_suppresses_protocol_noise() {
    let tool = project_stream_event(&json!({
        "event": "step_update",
        "step_update": {
            "conversation_id": "agy-7",
            "step_index": 6,
            "state": "ACTIVE",
            "step_type": "tool",
            "tool_name": "call_mcp_tool",
            "tool_info": {
                "input": {
                    "ServerName": "ecky_mcp",
                    "ToolName": "session_log_in",
                    "Arguments": { "threadId": "thread-7" },
                    "toolAction": "Logging into Ecky session",
                    "toolSummary": "Ecky login"
                },
                "output": "private tool output"
            }
        }
    }))
    .unwrap()
    .unwrap();
    assert_eq!(
        tool,
        AgyProjectedEvent::Working {
            conversation_id: "agy-7".to_string(),
            step_index: 6,
            text: "WORKING · Logging into Ecky session".to_string(),
        }
    );
    assert!(!format!("{tool:?}").contains("private tool output"));

    let encoded_tool = project_stream_event(&json!({
        "event": "step_update",
        "step_update": {
            "conversation_id": "agy-7",
            "step_index": 7,
            "state": "ACTIVE",
            "step_type": "tool",
            "tool_name": "run_command",
            "tool_info": {
                "input": "{\"toolAction\":\"Inspecting fit constraints\"}",
                "output": { "toolAction": "private output must not leak" }
            }
        }
    }))
    .unwrap()
    .unwrap();
    assert_eq!(
        encoded_tool,
        AgyProjectedEvent::Working {
            conversation_id: "agy-7".to_string(),
            step_index: 7,
            text: "WORKING · Inspecting fit constraints".to_string(),
        }
    );
    assert!(!format!("{encoded_tool:?}").contains("private output must not leak"));

    for step_type in ["system_message", "unknown", "progress"] {
        let event = project_stream_event(&json!({
            "event": "step_update",
            "step_update": {
                "conversation_id": "agy-7",
                "step_index": 7,
                "state": "DONE",
                "step_type": step_type,
                "text_delta": "internal protocol payload"
            }
        }))
        .unwrap();
        assert_eq!(event, None, "{step_type} must not become a fake activity");
    }
}

#[test]
fn terminal_result_carries_exact_status_response_and_error() {
    let result = project_stream_event(&json!({
        "event": "result",
        "result": {
            "conversation_id": "agy-7",
            "status": "ERROR",
            "response": "partial",
            "error": "MCP transport returned 503 raw body"
        }
    }))
    .unwrap()
    .unwrap();
    assert_eq!(
        result,
        AgyProjectedEvent::Result {
            conversation_id: "agy-7".to_string(),
            status: "ERROR".to_string(),
            response: "partial".to_string(),
            error: Some("MCP transport returned 503 raw body".to_string()),
        }
    );
}

#[cfg(unix)]
#[tokio::test]
async fn bidirectional_session_finishes_each_turn_without_waiting_for_process_exit() {
    use std::os::unix::fs::PermissionsExt;
    let _environment = AGY_ENV_LOCK.lock().await;

    let directory = std::env::temp_dir().join(format!("ecky-agy-test-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&directory).unwrap();
    let executable = directory.join("agy-fake.py");
    std::fs::write(
        &executable,
        r#"#!/usr/bin/env python3
import json
import sys

if "--version" in sys.argv:
    print("Antigravity CLI 1.1.15")
    raise SystemExit(0)
if "--model" not in sys.argv or sys.argv[sys.argv.index("--model") + 1] != "claude-sonnet-4-6":
    print("missing selected model", file=sys.stderr)
    raise SystemExit(2)

conversation_id = "agy-test-7"
if "--conversation" in sys.argv:
    conversation_id = sys.argv[sys.argv.index("--conversation") + 1]
print(json.dumps({"event": "init", "conversation_id": conversation_id, "init": {}}), flush=True)
turns = 0
for line in sys.stdin:
    message = json.loads(line)
    if message.get("event") != "user":
        continue
    turns += 1
    print(json.dumps({"event": "step_update", "step_update": {
        "conversation_id": conversation_id, "step_index": turns,
        "state": "DONE", "step_type": "agent_response", "text_delta": "working"
    }}), flush=True)
    print(json.dumps({"event": "result", "result": {
        "conversation_id": conversation_id, "status": "SUCCESS",
        "response": "answer-%d" % turns
    }}), flush=True)
"#,
    )
    .unwrap();
    let mut permissions = std::fs::metadata(&executable).unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&executable, permissions).unwrap();

    std::env::set_var("ECKY_AGY_BIN", &executable);
    let supervisor = AgyProviderSupervisor::new();
    let first = supervisor
        .start_new_turn(
            directory.to_str().unwrap(),
            "first",
            Some("claude-sonnet-4-6"),
            Some("http://127.0.0.1:39249/mcp?providerThreadId=thread-1"),
        )
        .await
        .unwrap();
    assert_eq!(first.conversation_id, "agy-test-7");
    let first_result = tokio::time::timeout(std::time::Duration::from_secs(2), first.result)
        .await
        .expect("result arrives while process remains open")
        .unwrap()
        .unwrap();
    assert_eq!(first_result.response, "answer-1");
    let first_traces = supervisor.turn_traces("agy-test-7").await;
    assert_eq!(first_traces.len(), 1);
    assert_eq!(first_traces[0].status, "success");
    assert_eq!(first_traces[0].messages[0].content, "working");
    assert_eq!(
        first_traces[0].messages[0].provider_event_kind,
        Some(ecky_cad_lib::contracts::ProviderEventKind::Assistant)
    );

    let second = supervisor
        .start_turn(
            "agy-test-7",
            directory.to_str().unwrap(),
            "second",
            Some("claude-sonnet-4-6"),
            Some("http://127.0.0.1:39249/mcp?providerThreadId=thread-1"),
        )
        .await
        .unwrap();
    let second_result = tokio::time::timeout(std::time::Duration::from_secs(2), second.result)
        .await
        .expect("second result reuses warm process")
        .unwrap()
        .unwrap();
    assert_eq!(second_result.response, "answer-2");
    assert_eq!(supervisor.turn_traces("agy-test-7").await.len(), 2);
    assert_eq!(supervisor.runtime("agy-test-7").await.phase, "idle");

    let after_endpoint_change = supervisor
        .start_turn(
            "agy-test-7",
            directory.to_str().unwrap(),
            "third",
            Some("claude-sonnet-4-6"),
            Some("http://127.0.0.1:39250/mcp?providerThreadId=thread-1"),
        )
        .await
        .unwrap();
    let third_result = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        after_endpoint_change.result,
    )
    .await
    .expect("endpoint change respawns and resumes the provider")
    .unwrap()
    .unwrap();
    assert_eq!(third_result.response, "answer-1");

    std::env::remove_var("ECKY_AGY_BIN");
    let _ = std::fs::remove_dir_all(directory);
}

#[cfg(unix)]
#[tokio::test]
async fn bound_conversation_activates_without_sending_a_fake_turn() {
    use std::os::unix::fs::PermissionsExt;
    let _environment = AGY_ENV_LOCK.lock().await;

    let directory =
        std::env::temp_dir().join(format!("ecky-agy-owner-test-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&directory).unwrap();
    let executable = directory.join("agy-owner-fake.py");
    let prompts = directory.join("prompts");
    std::fs::write(
        &executable,
        r#"#!/usr/bin/env python3
import json
import os
import sys

if "--version" in sys.argv:
    print("Antigravity CLI 1.1.15")
    raise SystemExit(0)

conversation_id = sys.argv[sys.argv.index("--conversation") + 1]
print(json.dumps({"event": "init", "conversation_id": conversation_id, "init": {}}), flush=True)
for line in sys.stdin:
    with open(os.environ["ECKY_AGY_TEST_PROMPTS"], "a", encoding="utf-8") as output:
        output.write(line)
"#,
    )
    .unwrap();
    let mut permissions = std::fs::metadata(&executable).unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&executable, permissions).unwrap();

    std::env::set_var("ECKY_AGY_BIN", &executable);
    std::env::set_var("ECKY_AGY_TEST_PROMPTS", &prompts);
    let supervisor = AgyProviderSupervisor::new();
    supervisor
        .activate_conversation("agy-owned-8", directory.to_str().unwrap(), None)
        .await
        .unwrap();
    assert_eq!(supervisor.runtime("agy-owned-8").await.phase, "idle");
    assert!(
        !prompts.exists(),
        "activation must not synthesize a user turn"
    );

    std::env::remove_var("ECKY_AGY_BIN");
    std::env::remove_var("ECKY_AGY_TEST_PROMPTS");
    let _ = std::fs::remove_dir_all(directory);
}

#[cfg(unix)]
#[tokio::test]
async fn stop_normalizes_provider_timeout_and_resumes_next_turn_in_fresh_process() {
    use std::os::unix::fs::PermissionsExt;
    let _environment = AGY_ENV_LOCK.lock().await;

    let directory =
        std::env::temp_dir().join(format!("ecky-agy-stop-test-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&directory).unwrap();
    let executable = directory.join("agy-stop-fake.py");
    std::fs::write(
        &executable,
        r#"#!/usr/bin/env python3
import json
import signal
import sys
import time

if "--version" in sys.argv:
    print("Antigravity CLI 1.1.15")
    raise SystemExit(0)

conversation_id = "agy-stop-7"
print(json.dumps({"event": "init", "conversation_id": conversation_id, "init": {}}), flush=True)

if "--conversation" in sys.argv:
    for line in sys.stdin:
        json.loads(line)
        print(json.dumps({"event": "result", "result": {
            "conversation_id": conversation_id,
            "status": "SUCCESS",
            "response": "queued turn delivered"
        }}), flush=True)
    raise SystemExit(0)

def interrupted(_signal, _frame):
    print(json.dumps({"event": "result", "result": {
        "conversation_id": conversation_id,
        "status": "ERROR",
        "response": "",
        "error": "timeout waiting for response"
    }}), flush=True)
    raise SystemExit(130)

signal.signal(signal.SIGINT, interrupted)
for line in sys.stdin:
    json.loads(line)
    print(json.dumps({"event": "step_update", "step_update": {
        "conversation_id": conversation_id, "step_index": 1,
        "state": "ACTIVE", "step_type": "tool", "tool_name": "ecky_ast_inspect"
    }}), flush=True)
    time.sleep(30)
"#,
    )
    .unwrap();
    let mut permissions = std::fs::metadata(&executable).unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&executable, permissions).unwrap();

    std::env::set_var("ECKY_AGY_BIN", &executable);
    let supervisor = AgyProviderSupervisor::new();
    let started = supervisor
        .start_new_turn(directory.to_str().unwrap(), "work", None, None)
        .await
        .unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(1), async {
        while supervisor
            .live_messages(&started.conversation_id)
            .await
            .is_empty()
        {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("public event arrives before interrupt");
    supervisor
        .stop_turn(&started.conversation_id, &started.turn_id)
        .await
        .unwrap();
    let result = tokio::time::timeout(std::time::Duration::from_secs(2), started.result)
        .await
        .expect("SIGINT produces terminal result")
        .unwrap()
        .unwrap();
    assert_eq!(result.status, "INTERRUPTED");
    assert_eq!(result.error, None);
    let traces = supervisor.turn_traces("agy-stop-7").await;
    assert_eq!(traces.len(), 1);
    assert_eq!(traces[0].status, "interrupted");
    assert_eq!(
        traces[0].messages[0].content,
        "USING TOOL · ecky_ast_inspect"
    );

    let resumed = supervisor
        .start_turn(
            "agy-stop-7",
            directory.to_str().unwrap(),
            "queued work",
            None,
            None,
        )
        .await
        .unwrap();
    let resumed_result = tokio::time::timeout(std::time::Duration::from_secs(2), resumed.result)
        .await
        .expect("queued turn starts after STOP")
        .unwrap()
        .unwrap();
    assert_eq!(resumed_result.status, "SUCCESS");
    assert_eq!(resumed_result.response, "queued turn delivered");

    std::env::remove_var("ECKY_AGY_BIN");
    let _ = std::fs::remove_dir_all(directory);
}

#[cfg(unix)]
#[tokio::test]
async fn app_shutdown_stops_the_owned_agy_process_group_including_descendants() {
    use std::os::unix::fs::PermissionsExt;
    let _environment = AGY_ENV_LOCK.lock().await;

    let directory =
        std::env::temp_dir().join(format!("ecky-agy-shutdown-test-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&directory).unwrap();
    let executable = directory.join("agy-shutdown-fake.py");
    let process_ids = directory.join("process-ids");
    std::fs::write(
        &executable,
        r#"#!/usr/bin/env python3
import json
import os
import subprocess
import sys
import time

if "--version" in sys.argv:
    print("Antigravity CLI 1.1.15")
    raise SystemExit(0)

conversation_id = "agy-shutdown-7"
descendant = subprocess.Popen(["sleep", "30"])
with open(os.environ["ECKY_AGY_TEST_PROCESS_IDS"], "w", encoding="utf-8") as output:
    output.write("%d %d" % (os.getpid(), descendant.pid))
print(json.dumps({"event": "init", "conversation_id": conversation_id, "init": {}}), flush=True)
for line in sys.stdin:
    json.loads(line)
    time.sleep(30)
"#,
    )
    .unwrap();
    let mut permissions = std::fs::metadata(&executable).unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&executable, permissions).unwrap();

    std::env::set_var("ECKY_AGY_BIN", &executable);
    std::env::set_var("ECKY_AGY_TEST_PROCESS_IDS", &process_ids);
    let supervisor = AgyProviderSupervisor::new();
    let started = supervisor
        .start_new_turn(directory.to_str().unwrap(), "work", None, None)
        .await
        .unwrap();
    let ids = tokio::time::timeout(std::time::Duration::from_secs(1), async {
        loop {
            if let Ok(raw) = std::fs::read_to_string(&process_ids) {
                break raw;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("fake provider records its process tree");
    let ids = ids
        .split_whitespace()
        .map(|value| value.parse::<i32>().unwrap())
        .collect::<Vec<_>>();

    supervisor.shutdown_all().await;
    let result = tokio::time::timeout(std::time::Duration::from_secs(1), started.result)
        .await
        .expect("shutdown resolves the active turn")
        .unwrap()
        .expect_err("shutdown cannot report success");
    assert!(result.message.contains("Ecky shutdown"));
    for pid in ids {
        tokio::time::timeout(std::time::Duration::from_secs(1), async {
            while unsafe { libc::kill(pid, 0) } == 0 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap_or_else(|_| panic!("process {pid} survived"));
    }

    std::env::remove_var("ECKY_AGY_BIN");
    std::env::remove_var("ECKY_AGY_TEST_PROCESS_IDS");
    let _ = std::fs::remove_dir_all(directory);
}
