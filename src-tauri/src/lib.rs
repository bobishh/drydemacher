pub mod models;
pub mod db;
pub mod llm;
pub mod freecad;
pub mod context;

use tauri::{State, AppHandle, Manager};
use serde_json::json;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;
use std::fs;
use std::sync::Mutex;
use base64::{Engine as _, engine::general_purpose};

use crate::models::{AppState, DesignOutput, Message, ThreadReference, Attachment, GenerateOutput, CommitOutput, IntentDecision, QuestionReply};
use crate::context::*;

fn extract_code_blocks(text: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut cursor = text;
    while let Some(start) = cursor.find("```") {
        let after_ticks = &cursor[start + 3..];
        let Some(end) = after_ticks.find("```") else {
            break;
        };
        let block = &after_ticks[..end];
        let normalized = if let Some(newline) = block.find('\n') {
            let first_line = block[..newline].trim().to_lowercase();
            let rest = block[newline + 1..].trim();
            if first_line.is_empty() || first_line.contains("python") || first_line.contains("py") {
                rest.to_string()
            } else {
                block.trim().to_string()
            }
        } else {
            block.trim().to_string()
        };
        if !normalized.is_empty() {
            blocks.push(normalized);
        }
        cursor = &after_ticks[end + 3..];
    }
    blocks
}

fn looks_like_python_macro(text: &str) -> bool {
    let lowered = text.to_lowercase();
    let signal_count = [
        "import freecad",
        "import part",
        "app.activedocument",
        "app.newdocument",
        "params.get(",
        "doc.recompute(",
        "part::feature",
        "part.make",
        "vector(",
        "placemen",
    ]
    .iter()
    .filter(|needle| lowered.contains(**needle))
    .count();
    signal_count >= 2 || (lowered.contains("import ") && lowered.contains("if doc is none"))
}

const PINNED_REFERENCE_SUMMARY_MAX_CHARS: usize = 200;
const PINNED_REFERENCE_CONTENT_MAX_CHARS: usize = 2200;

fn summarize_reference(kind: &str, name: &str, content: &str) -> String {
    let intro = match kind {
        "python_macro" => "Python macro reference",
        "attachment" => "Attachment reference",
        _ => "Reference",
    };
    let first_line = content.lines().find(|line| !line.trim().is_empty()).unwrap_or("");
    if first_line.is_empty() {
        compact_text(&format!("{}: {}", intro, name), PINNED_REFERENCE_SUMMARY_MAX_CHARS)
    } else {
        compact_text(
            &format!("{} [{}]: {}", intro, name, first_line.trim()),
            PINNED_REFERENCE_SUMMARY_MAX_CHARS,
        )
    }
}

fn extract_prompt_references(
    thread_id: &str,
    message_id: &str,
    prompt: &str,
    created_at: u64,
) -> Vec<ThreadReference> {
    let mut refs = Vec::new();
    let code_blocks = extract_code_blocks(prompt);
    if !code_blocks.is_empty() {
        for (idx, block) in code_blocks.into_iter().enumerate() {
            if looks_like_python_macro(&block) {
                refs.push(ThreadReference {
                    id: Uuid::new_v4().to_string(),
                    thread_id: thread_id.to_string(),
                    source_message_id: Some(message_id.to_string()),
                    ordinal: idx as i64,
                    kind: "python_macro".to_string(),
                    name: format!("prompt_macro_{}", idx + 1),
                    content: compact_text(&block, PINNED_REFERENCE_CONTENT_MAX_CHARS),
                    summary: summarize_reference("python_macro", &format!("prompt_macro_{}", idx + 1), &block),
                    pinned: true,
                    created_at,
                });
            }
        }
    } else if looks_like_python_macro(prompt) {
        refs.push(ThreadReference {
            id: Uuid::new_v4().to_string(),
            thread_id: thread_id.to_string(),
            source_message_id: Some(message_id.to_string()),
            ordinal: 0,
            kind: "python_macro".to_string(),
            name: "prompt_macro_1".to_string(),
            content: compact_text(prompt, PINNED_REFERENCE_CONTENT_MAX_CHARS),
            summary: summarize_reference("python_macro", "prompt_macro_1", prompt),
            pinned: true,
            created_at,
        });
    }
    refs
}

fn persist_user_prompt_references(
    conn: &rusqlite::Connection,
    thread_id: &str,
    message_id: &str,
    prompt: &str,
    attachments: Option<&Vec<Attachment>>,
    created_at: u64,
) -> Result<(), String> {
    for reference in extract_prompt_references(thread_id, message_id, prompt, created_at) {
        db::add_thread_reference(conn, &reference).map_err(|e| e.to_string())?;
    }

    if let Some(attachments) = attachments {
        let mut ordinal_offset = 100;
        for attachment in attachments {
            let ext = attachment
                .path
                .split('.')
                .last()
                .unwrap_or("")
                .to_lowercase();
            let is_python = matches!(ext.as_str(), "py" | "fcmacro");
            let content = if is_python {
                fs::read_to_string(&attachment.path).unwrap_or_default()
            } else {
                String::new()
            };
            let summary = compact_text(
                &format!(
                    "{} attachment [{}]: {}",
                    if is_python { "Python macro" } else { "External" },
                    attachment.name,
                    attachment.explanation
                ),
                PINNED_REFERENCE_SUMMARY_MAX_CHARS,
            );
            let reference = ThreadReference {
                id: Uuid::new_v4().to_string(),
                thread_id: thread_id.to_string(),
                source_message_id: Some(message_id.to_string()),
                ordinal: ordinal_offset,
                kind: if is_python { "python_macro".to_string() } else { "attachment".to_string() },
                name: attachment.name.clone(),
                content: compact_text(&content, PINNED_REFERENCE_CONTENT_MAX_CHARS),
                summary,
                pinned: true,
                created_at,
            };
            db::add_thread_reference(conn, &reference).map_err(|e| e.to_string())?;
            ordinal_offset += 1;
        }
    }

    Ok(())
}

fn migrate_legacy_references(conn: &rusqlite::Connection) -> Result<(), String> {
    let threads = db::get_all_threads(conn).map_err(|e| e.to_string())?;
    for thread in threads {
        for message in thread.messages.iter().filter(|m| m.role == "user") {
            persist_user_prompt_references(conn, &thread.id, &message.id, &message.content, None, message.timestamp)?;
        }
        if !thread.summary.trim().is_empty() {
            continue;
        }
        let summary = build_thread_summary(&thread.title, &thread.messages);
        db::update_thread_summary(conn, &thread.id, &summary).map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn persist_thread_summary(
    conn: &rusqlite::Connection,
    thread_id: &str,
    title: &str,
) -> Result<String, String> {
    let messages = db::get_thread_messages(conn, thread_id).map_err(|e| e.to_string())?;
    let summary = build_thread_summary(title, &messages);
    db::update_thread_summary(conn, thread_id, &summary).map_err(|e| e.to_string())?;
    Ok(summary)
}

#[tauri::command]
async fn get_config(state: State<'_, AppState>) -> Result<crate::models::Config, String> {
    let config = state.config.lock().unwrap();
    Ok(config.clone())
}

#[tauri::command]
async fn save_config(config: crate::models::Config, state: State<'_, AppState>, app: AppHandle) -> Result<(), String> {
    let config_dir = app.path().app_config_dir().unwrap();
    let config_path = config_dir.join("config.json");
    
    let data = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;
    fs::write(config_path, data).map_err(|e| e.to_string())?;
    
    let mut state_config = state.config.lock().unwrap();
    *state_config = config;
    Ok(())
}

#[tauri::command]
async fn get_history(state: State<'_, AppState>) -> Result<Vec<crate::models::Thread>, String> {
    let db = state.db.lock().unwrap();
    db::get_all_threads(&db).map_err(|e: rusqlite::Error| e.to_string())
}

#[tauri::command]
async fn clear_history(state: State<'_, AppState>) -> Result<(), String> {
    let db = state.db.lock().unwrap();
    db::clear_history(&db).map_err(|e: rusqlite::Error| e.to_string())
}

#[tauri::command]
async fn delete_thread(id: String, state: State<'_, AppState>) -> Result<(), String> {
    let db = state.db.lock().unwrap();
    db::delete_thread(&db, &id).map_err(|e: rusqlite::Error| e.to_string())
}

#[tauri::command]
async fn delete_version(message_id: String, state: State<'_, AppState>) -> Result<(), String> {
    let db = state.db.lock().unwrap();
    db::delete_message(&db, &message_id).map_err(|e: rusqlite::Error| e.to_string())
}

#[tauri::command]
async fn generate_design(
    prompt: String, 
    thread_id: Option<String>,
    parent_macro_code: Option<String>,
    working_design: Option<DesignOutput>,
    is_retry: bool,
    image_data: Option<String>,
    attachments: Option<Vec<Attachment>>,
    question_mode: Option<bool>,
    state: State<'_, AppState>, 
    app: AppHandle
) -> Result<GenerateOutput, String> {
    let engine = {
        let config = state.config.lock().unwrap();
        config.engines.iter().find(|e| e.id == config.selected_engine_id).cloned()
    }.ok_or("No active engine selected")?;

    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let question_mode = question_mode.unwrap_or(false);

    let ctx = {
        let db = state.db.lock().unwrap();
        context::assemble_context(&db, thread_id, working_design, parent_macro_code)
    };

    let intent_mode = if question_mode { "QUESTION_ONLY" } else { "DESIGN_EDIT" };
    let contextual_prompt = format_contextual_prompt(&ctx, &prompt, TECHNICAL_SYSTEM_PROMPT, intent_mode);

    let mut images = Vec::new();
    if let Some(ref main_img) = image_data {
        images.push(main_img.clone());
    }

    if let Some(atts) = &attachments {
        for att in atts {
            if att.r#type == "image" {
                if let Ok(bytes) = fs::read(&att.path) {
                    let b64 = general_purpose::STANDARD.encode(bytes);
                    let ext = att.path.split('.').last().unwrap_or("png").to_lowercase();
                    let mime = if ext == "jpg" || ext == "jpeg" { "image/jpeg" } else { "image/png" };
                    images.push(format!("data:{};base64,{}", mime, b64));
                }
            }
        }
    }

    let result: Result<DesignOutput, String> = llm::generate_design(&engine, &contextual_prompt, images).await;

    let (status, content, output): (String, String, Option<DesignOutput>) = match result {
        Ok(mut out) => {
            if question_mode {
                out.interaction_mode = "question".to_string();
                if let Some(previous) = &ctx.last_output {
                    out.title = previous.title.clone();
                    out.version_name = previous.version_name.clone();
                    out.macro_code = previous.macro_code.clone();
                    out.ui_spec = previous.ui_spec.clone();
                    out.initial_params = previous.initial_params.clone();
                }
                if out.version_name.trim().is_empty() {
                    out.version_name = "Q&A".to_string();
                }
                if out.response.trim().is_empty() {
                    out.response = "Question answered. Geometry unchanged.".to_string();
                }
            } else if out.interaction_mode.trim().is_empty() {
                out.interaction_mode = "design".to_string();
            }

            let assistant_text = if out.response.trim().is_empty() {
                "Synthesized design output.".to_string()
            } else {
                out.response.clone()
            };

            ("success".to_string(), assistant_text, Some(out))
        },
        Err(raw_body) => ("error".to_string(), format!("LLM Response (Unparsed): {}", raw_body), None)
    };

    let assistant_msg_id = Uuid::new_v4().to_string();
    let thread_id_actual = ctx.thread_id.clone();

    // Persist immediately on the backend
    {
        let db = state.db.lock().unwrap();
        let thread_title = output.as_ref().map(|o| o.title.clone()).unwrap_or_else(|| "Failed Design Attempt".to_string());
        db::create_or_update_thread(&db, &thread_id_actual, &thread_title, now).map_err(|e: rusqlite::Error| e.to_string())?;

        if !is_retry {
            let user_msg = Message {
                id: Uuid::new_v4().to_string(),
                role: "user".to_string(),
                content: prompt.clone(),
                status: "success".to_string(),
                output: None,
                image_data: image_data.clone(),
                timestamp: now,
            };
            db::add_message(&db, &thread_id_actual, &user_msg).map_err(|e: rusqlite::Error| e.to_string())?;
            persist_user_prompt_references(&db, &thread_id_actual, &user_msg.id, &prompt, attachments.as_ref(), now)?;
        }

        let assistant_msg = Message {
            id: assistant_msg_id.clone(),
            role: "assistant".to_string(),
            content: content.clone(),
            status: status.clone(),
            output: output.clone(),
            image_data: None,
            timestamp: now + 1,
        };
        db::add_message(&db, &thread_id_actual, &assistant_msg).map_err(|e: rusqlite::Error| e.to_string())?;
        let _ = persist_thread_summary(&db, &thread_id_actual, &thread_title);
    }

    if let Some(out) = output {
        let mut last = state.last_design.lock().unwrap();
        *last = Some(out.clone());
        let mut last_tid = state.last_thread_id.lock().unwrap();
        *last_tid = Some(thread_id_actual.clone());

        let cache_path = app.path().app_config_dir().unwrap().join("last_design.json");
        let session_data = json!({
            "design": out,
            "thread_id": Some(thread_id_actual.clone())
        });
        if let Ok(json) = serde_json::to_string_pretty(&session_data) {
            let _ = fs::write(cache_path, json);
        }
        Ok(GenerateOutput { design: out, thread_id: thread_id_actual, message_id: assistant_msg_id })
    } else {
        Err(format!("ERR_ID:{}|{}", thread_id_actual, content))
    }
}

fn fallback_intent(prompt: &str) -> IntentDecision {
    let p = prompt.to_lowercase();
    let has_question_signal = p.contains('?')
        || p.contains("explain")
        || p.contains("why")
        || p.contains("how")
        || p.contains("what");
    let has_design_signal = p.contains("generate")
        || p.contains("create")
        || p.contains("make")
        || p.contains("add")
        || p.contains("remove")
        || p.contains("change")
        || p.contains("update")
        || p.contains("set")
        || p.contains("resize")
        || p.contains("connector")
        || p.contains("diameter");

    if has_question_signal && !has_design_signal {
        IntentDecision {
            intent_mode: "question".to_string(),
            confidence: 0.55,
            response: "Thinking not deep enough. This looks like a question.".to_string(),
        }
    } else {
        IntentDecision {
            intent_mode: "design".to_string(),
            confidence: 0.55,
            response: "This looks like a geometry change request.".to_string(),
        }
    }
}

#[tauri::command]
async fn commit_generated_version(
    thread_id: String,
    prompt: String,
    design: DesignOutput,
    image_data: Option<String>,
    attachments: Option<Vec<Attachment>>,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<CommitOutput, String> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let thread_title = design.title.clone();
    let assistant_text = if design.response.trim().is_empty() {
        "Synthesized design output.".to_string()
    } else {
        design.response.clone()
    };

    let user_msg = Message {
        id: Uuid::new_v4().to_string(),
        role: "user".to_string(),
        content: prompt.clone(),
        status: "success".to_string(),
        output: None,
        image_data: image_data.clone(),
        timestamp: now,
    };
    let assistant_msg = Message {
        id: Uuid::new_v4().to_string(),
        role: "assistant".to_string(),
        content: assistant_text,
        status: "success".to_string(),
        output: Some(design.clone()),
        image_data: None,
        timestamp: now + 1,
    };

    {
        let db = state.db.lock().unwrap();
        db::create_or_update_thread(&db, &thread_id, &thread_title, now).map_err(|e: rusqlite::Error| e.to_string())?;
        db::add_message(&db, &thread_id, &user_msg).map_err(|e: rusqlite::Error| e.to_string())?;
        persist_user_prompt_references(&db, &thread_id, &user_msg.id, &prompt, attachments.as_ref(), now)?;
        db::add_message(&db, &thread_id, &assistant_msg).map_err(|e: rusqlite::Error| e.to_string())?;
        let _ = persist_thread_summary(&db, &thread_id, &thread_title);
    }

    {
        let mut last = state.last_design.lock().unwrap();
        *last = Some(design.clone());
        let mut last_tid = state.last_thread_id.lock().unwrap();
        *last_tid = Some(thread_id.clone());
    }

    let cache_path = app.path().app_config_dir().unwrap().join("last_design.json");
    let session_data = json!({
        "design": design,
        "thread_id": Some(thread_id.clone())
    });
    if let Ok(json) = serde_json::to_string_pretty(&session_data) {
        let _ = fs::write(cache_path, json);
    }

    Ok(CommitOutput {
        thread_id,
        message_id: assistant_msg.id,
    })
}

#[tauri::command]
async fn classify_intent(
    prompt: String,
    thread_id: Option<String>,
    context: Option<String>,
    state: State<'_, AppState>
) -> Result<IntentDecision, String> {
    let engine = {
        let config = state.config.lock().unwrap();
        config
            .engines
            .iter()
            .find(|e| e.id == config.selected_engine_id)
            .cloned()
    }
    .ok_or("No active engine selected")?;

    let backend_context = if thread_id.is_some() {
        let ctx = {
            let db = state.db.lock().unwrap();
            context::assemble_context(&db, thread_id, None, None)
        };

        let mut blocks = Vec::new();
        if !ctx.summary.trim().is_empty() {
            blocks.push(format!("THREAD SUMMARY\n{}", ctx.summary));
        }
        if !ctx.recent_dialogue.trim().is_empty() {
            blocks.push(format!("RECENT DIALOGUE\n{}", ctx.recent_dialogue));
        }
        if !ctx.pinned_references.trim().is_empty() {
            blocks.push(format!("PINNED REFERENCES\n{}", ctx.pinned_references));
        }
        if let Some(c) = context.as_ref().filter(|c| !c.trim().is_empty()) {
            blocks.push(format!("CURRENT LIVE SNAPSHOT\n{}", c));
        }
        Some(blocks.join("\n\n"))
    } else {
        context
    };

    match llm::classify_intent(&engine, &prompt, backend_context.as_deref()).await {
        Ok(classification) => Ok(IntentDecision {
            intent_mode: classification.intent,
            confidence: classification.confidence,
            response: classification.response,
        }),
        Err(_) => Ok(fallback_intent(&prompt)),
    }
}

#[tauri::command]
async fn answer_question_light(
    prompt: String,
    response: String,
    thread_id: Option<String>,
    title_hint: Option<String>,
    image_data: Option<String>,
    attachments: Option<Vec<Attachment>>,
    state: State<'_, AppState>
) -> Result<QuestionReply, String> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let thread_id_actual = thread_id.unwrap_or_else(|| Uuid::new_v4().to_string());

    {
        let db = state.db.lock().unwrap();
        let existing_title = db::get_thread_title(&db, &thread_id_actual).map_err(|e| e.to_string())?;
        let thread_title = existing_title
            .or(title_hint)
            .unwrap_or_else(|| "Question Session".to_string());

        db::create_or_update_thread(&db, &thread_id_actual, &thread_title, now).map_err(|e| e.to_string())?;

        let user_msg = Message {
            id: Uuid::new_v4().to_string(),
            role: "user".to_string(),
            content: prompt.clone(),
            status: "success".to_string(),
            output: None,
            image_data: image_data.clone(),
            timestamp: now,
        };
        db::add_message(&db, &thread_id_actual, &user_msg).map_err(|e| e.to_string())?;
        persist_user_prompt_references(&db, &thread_id_actual, &user_msg.id, &prompt, attachments.as_ref(), now)?;

        let assistant_msg = Message {
            id: Uuid::new_v4().to_string(),
            role: "assistant".to_string(),
            content: response.clone(),
            status: "success".to_string(),
            output: None,
            image_data: None,
            timestamp: now + 1,
        };
        db::add_message(&db, &thread_id_actual, &assistant_msg).map_err(|e| e.to_string())?;
        let _ = persist_thread_summary(&db, &thread_id_actual, &thread_title);
    }

    {
        let mut last_tid = state.last_thread_id.lock().unwrap();
        *last_tid = Some(thread_id_actual.clone());
    }

    Ok(QuestionReply {
        thread_id: thread_id_actual,
        response,
    })
}

#[tauri::command]
async fn render_stl(macro_code: String, parameters: serde_json::Value, app: AppHandle) -> Result<String, String> {
    freecad::render(&macro_code, &parameters, &app)
}

#[tauri::command]
async fn get_default_macro(app: AppHandle) -> Result<String, String> {
    freecad::get_default_macro(&app)
}

#[tauri::command]
async fn get_last_design(state: State<'_, AppState>) -> Result<Option<(DesignOutput, Option<String>)>, String> {
    let last = state.last_design.lock().unwrap();
    let thread_id = state.last_thread_id.lock().unwrap();
    Ok(last.as_ref().map(|d| (d.clone(), thread_id.clone())))
}

#[tauri::command]
async fn get_system_prompt() -> Result<String, String> {
    Ok(DEFAULT_PROMPT.to_string())
}

#[tauri::command]
async fn list_models(provider: String, api_key: String, base_url: String) -> Result<Vec<String>, String> {
    llm::list_models(&provider, &api_key, &base_url).await
}

#[tauri::command]
async fn update_ui_spec(
    message_id: String,
    ui_spec: serde_json::Value,
    state: State<'_, AppState>,
    app: AppHandle
) -> Result<(), String> {
    let (updated_output, updated_thread_id) = {
        let db = state.db.lock().unwrap();
        db::update_message_ui_spec(&db, &message_id, &ui_spec).map_err(|e| e.to_string())?;
        db::get_message_output_and_thread(&db, &message_id).map_err(|e| e.to_string())?
    }
    .ok_or("Message output not found for ui_spec update")?;

    {
        let mut last = state.last_design.lock().unwrap();
        *last = Some(updated_output.clone());
        let mut last_tid = state.last_thread_id.lock().unwrap();
        *last_tid = Some(updated_thread_id.clone());
    }

    let cache_path = app.path().app_config_dir().unwrap().join("last_design.json");
    let session_data = json!({
        "design": updated_output,
        "thread_id": Some(updated_thread_id)
    });
    if let Ok(json) = serde_json::to_string_pretty(&session_data) {
        let _ = fs::write(cache_path, json);
    }

    Ok(())
}

#[tauri::command]
async fn update_parameters(
    message_id: String,
    parameters: serde_json::Value,
    state: State<'_, AppState>,
    app: AppHandle
) -> Result<(), String> {
    let (updated_output, updated_thread_id) = {
        let db = state.db.lock().unwrap();
        db::update_message_parameters(&db, &message_id, &parameters).map_err(|e| e.to_string())?;
        db::get_message_output_and_thread(&db, &message_id).map_err(|e| e.to_string())?
    }
    .ok_or("Message output not found for parameter update")?;

    {
        let mut last = state.last_design.lock().unwrap();
        *last = Some(updated_output.clone());
        let mut last_tid = state.last_thread_id.lock().unwrap();
        *last_tid = Some(updated_thread_id.clone());
    }

    let cache_path = app.path().app_config_dir().unwrap().join("last_design.json");
    let session_data = json!({
        "design": updated_output,
        "thread_id": Some(updated_thread_id)
    });
    if let Ok(json) = serde_json::to_string_pretty(&session_data) {
        let _ = fs::write(cache_path, json);
    }

    Ok(())
}

#[tauri::command]
async fn export_file(source_path: String, target_path: String) -> Result<(), String> {
    fs::copy(&source_path, &target_path).map_err(|e| format!("Failed to export file: {}", e))?;
    Ok(())
}

#[tauri::command]
async fn add_manual_version(
    thread_id: String,
    title: String,
    version_name: String,
    macro_code: String,
    parameters: serde_json::Value,
    ui_spec: serde_json::Value,
    state: State<'_, AppState>
) -> Result<(), String> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let db = state.db.lock().unwrap();

    let output = DesignOutput {
        title: title.clone(),
        version_name,
        response: "Manual edit committed as new version.".to_string(),
        interaction_mode: "design".to_string(),
        macro_code,
        ui_spec,
        initial_params: parameters,
    };

    let msg = Message {
        id: Uuid::new_v4().to_string(),
        role: "assistant".to_string(),
        content: "Manual edit committed as new version.".to_string(),
        status: "success".to_string(),
        output: Some(output),
        image_data: None,
        timestamp: now,
    };

    db::add_message(&db, &thread_id, &msg).map_err(|e: rusqlite::Error| e.to_string())?;
    db::create_or_update_thread(&db, &thread_id, &title, now).map_err(|e: rusqlite::Error| e.to_string())?;
    let _ = persist_thread_summary(&db, &thread_id, &title);

    Ok(())
}

const DEFAULT_PROMPT: &str = r#"You are a CAD Design Agent.
You generate FreeCAD Python macros and a UI specification for their parameters based on the following user intent:

$USER_PROMPT

Macro Requirements:
- Write a FreeCAD Python macro using Part/OCCT BRep (no hand-built meshes).
- Units are in millimeters.
- Create at least one visible solid.
- Do NOT use string formatting braces like `{param_name}` in the generated code to reference parameters.
- UI Parameters are injected globally into the macro execution context. Access them directly by name (e.g., `width = frame_width`) or via the injected `params` dictionary (e.g., `width = params.get("frame_width", 90.0)`).

Return a JSON object with:
1. "title": A short (2-5 words) descriptive title.
2. "version_name": Short descriptive name for this iteration.
3. "response": short end-user text for Ecky's speech bubble (1-3 concise sentences).
4. "interaction_mode": "design" or "question".
5. "macro_code": The Python macro code.
6. "ui_spec": { 
     "fields": [
       { 
         "key": string, 
         "label": string, 
         "type": "range" | "number" | "select" | "checkbox", 
         "min"?: number, 
         "max"?: number, 
         "step"?: number,
         "options"?: [{ "label": string, "value": string | number }] 
       }
     ] 
   }
7. "initial_params": { ... }

UI Guidelines:
- Use "range" for continuous dimensions.
- Use "select" (enums) for discrete choices. Ensure "options" are provided.
- Use "checkbox" for boolean flags (e.g., "Show Holes"). Value will be true or false.
"#;

const TECHNICAL_SYSTEM_PROMPT: &str = r#"Return a JSON object with:
1. "title": 2-5 words project title.
2. "version_name": Short descriptive name for this iteration.
3. "response": short end-user text for the advisor speech bubble (1-3 concise sentences).
4. "interaction_mode": "design" or "question".
5. "macro_code": FreeCAD Python code.
6. "ui_spec": { "fields": [ { "key": string, "label": string, "type": "range"|"number"|"select"|"checkbox" } ] }
7. "initial_params": { "key": value }

CRITICAL RULES:
- UNITS: ALL dimensions are in MILLIMETERS (mm).
- UI: Focus on 'key', 'label' and 'type'. Don't worry about 'min'/'max' for ranges; the system will calculate bounds based on your 'initial_params'.
- PARAMETERS: Access parameters directly by name (e.g. `L = connector_length`) or via `params.get("key", default)`.
- NO BRACES: NEVER use `{var}` style interpolation inside the macro_code string.
- If USER_INTENT_MODE is "QUESTION_ONLY":
  - Set "interaction_mode" to "question".
  - Use "response" to explain the current design/code.
  - Keep "macro_code", "ui_spec", and "initial_params" aligned with the existing design context unless the user explicitly asks to modify geometry.
- If USER_INTENT_MODE is "DESIGN_EDIT":
  - Set "interaction_mode" to "design".
  - Use "response" as a short summary of what changed.
"#;

#[tauri::command]
async fn upload_asset(
    source_path: String,
    name: String,
    format: String,
    app: AppHandle
) -> Result<crate::models::Asset, String> {
    let app_data_dir = app.path().app_data_dir().unwrap();
    let assets_dir = app_data_dir.join("assets");
    if !assets_dir.exists() {
        fs::create_dir_all(&assets_dir).map_err(|e| e.to_string())?;
    }

    let id = Uuid::new_v4().to_string();
    let file_name = format!("{}.{}", id, format.to_lowercase());
    let target_path = assets_dir.join(&file_name);

    fs::copy(&source_path, &target_path).map_err(|e| e.to_string())?;

    Ok(crate::models::Asset {
        id,
        name,
        path: target_path.to_string_lossy().to_string(),
        format,
    })
}

#[tauri::command]
async fn save_recorded_audio(
    base64_data: String,
    name: String,
    app: AppHandle
) -> Result<crate::models::Asset, String> {
    let app_data_dir = app.path().app_data_dir().unwrap();
    let assets_dir = app_data_dir.join("assets");
    if !assets_dir.exists() {
        fs::create_dir_all(&assets_dir).map_err(|e| e.to_string())?;
    }

    let id = Uuid::new_v4().to_string();
    let file_name = format!("{}.webm", id); // MediaRecorder typically outputs webm/opus
    let target_path = assets_dir.join(&file_name);

    let bytes = general_purpose::STANDARD.decode(base64_data).map_err(|e| e.to_string())?;
    fs::write(&target_path, bytes).map_err(|e| e.to_string())?;

    Ok(crate::models::Asset {
        id,
        name,
        path: target_path.to_string_lossy().to_string(),
        format: "WEBM".to_string(),
    })
}

pub fn run() {
    let context = tauri::generate_context!();
    
    let default_config = crate::models::Config {
        engines: vec![
            crate::models::Engine {
                id: "default-gemini".to_string(),
                name: "Google Gemini".to_string(),
                provider: "gemini".to_string(),
                api_key: "".to_string(),
                model: "gemini-2.0-flash".to_string(),
                light_model: "gemini-2.0-flash-lite".to_string(),
                base_url: "".to_string(),
                system_prompt: "You are a CAD expert.".to_string(),
            }
        ],
        selected_engine_id: "default-gemini".to_string(),
        assets: vec![],
        microwave: None,
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(move |app| {
            let config_dir = app.path().app_config_dir()?;
            let app_data_dir = app.path().app_data_dir()?;
            if !config_dir.exists() {
                fs::create_dir_all(&config_dir)?;
            }
            if !app_data_dir.exists() {
                fs::create_dir_all(&app_data_dir)?;
            }

            let mut config = default_config;
            let config_path = config_dir.join("config.json");
            if config_path.exists() {
                if let Ok(data) = fs::read_to_string(&config_path) {
                    if let Ok(c) = serde_json::from_str::<crate::models::Config>(&data) {
                        config = c;
                    }
                }
            }

            let mut last_design = None;
            let mut last_thread_id = None;
            let last_path = config_dir.join("last_design.json");
            if last_path.exists() {
                if let Ok(data) = fs::read_to_string(&last_path) {
                    #[derive(serde::Deserialize)]
                    struct LastSession {
                        design: DesignOutput,
                        thread_id: Option<String>,
                    }
                    if let Ok(session) = serde_json::from_str::<LastSession>(&data) {
                        last_design = Some(session.design);
                        last_thread_id = session.thread_id;
                    } else if let Ok(design) = serde_json::from_str::<DesignOutput>(&data) {
                        // fallback for old format
                        last_design = Some(design);
                    }
                }
            }

            let db_path = config_dir.join("history.sqlite");
            let conn = db::init_db(&db_path).expect("Failed to initialize SQLite database");
            let _ = migrate_legacy_references(&conn);

            app.manage(AppState {
                config: Mutex::new(config),
                last_design: Mutex::new(last_design),
                last_thread_id: Mutex::new(last_thread_id),
                db: Mutex::new(conn),
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_config,
            save_config,
            get_history,
            clear_history,
            delete_thread,
            delete_version,
            generate_design,
            commit_generated_version,
            render_stl,
            list_models,
            classify_intent,
            answer_question_light,
            get_default_macro,
            get_last_design,
            get_system_prompt,
            export_file,
            add_manual_version,
            update_ui_spec,
            update_parameters,
            upload_asset,
            save_recorded_audio
        ])
        .run(context)
        .expect("error while running tauri application");
}
