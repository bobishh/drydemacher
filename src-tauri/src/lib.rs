pub mod models;
pub mod db;
pub mod llm;
pub mod freecad;

use tauri::{State, AppHandle, Manager};
use serde_json::{json, Value};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;
use std::fs;
use std::sync::Mutex;

use crate::models::{AppState, Config, Engine, DesignOutput, Message};

#[tauri::command]
async fn get_config(state: State<'_, AppState>) -> Result<Config, String> {
    let config = state.config.lock().unwrap();
    Ok(config.clone())
}

#[tauri::command]
async fn save_config(config: Config, state: State<'_, AppState>, app: AppHandle) -> Result<(), String> {
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

#[derive(serde::Serialize)]
struct GenerateOutput {
    design: DesignOutput,
    thread_id: String,
}

#[tauri::command]
async fn generate_design(
    prompt: String, 
    thread_id: Option<String>,
    parent_macro_code: Option<String>,
    is_retry: bool,
    image_data: Option<String>,
    state: State<'_, AppState>, 
    app: AppHandle
) -> Result<GenerateOutput, String> {
    let engine = {
        let config = state.config.lock().unwrap();
        config.engines.iter().find(|e| e.id == config.selected_engine_id).cloned()
    }.ok_or("No active engine selected")?;

    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();

    // Find the thread and its context
    let (thread_id_actual, last_macro) = {
        let db = state.db.lock().unwrap();
        if let Some(tid) = thread_id.clone() {
            let messages = db::get_thread_messages(&db, &tid).unwrap_or_default();
            let last_m = messages.iter()
                .rev()
                .find(|m| m.role == "assistant" && m.output.is_some())
                .and_then(|m| m.output.as_ref().map(|o| o.macro_code.clone()));
            (tid, last_m)
        } else {
            (Uuid::new_v4().to_string(), parent_macro_code)
        }
    };

    // Construct technical context
    let full_prompt = format!("{}\n\n{}", prompt, TECHNICAL_SYSTEM_PROMPT);

    let contextual_prompt = if let Some(code) = last_macro {
        format!(
            "a question regarding following FreeCAD Macro:\n\n```python\n{}\n```\n\nUser Question: {}", 
            code, full_prompt
        )
    } else {
        full_prompt
    };

    let result: Result<DesignOutput, String> = llm::generate_design(&engine, &contextual_prompt, image_data.clone()).await;

    let (status, content, output): (String, String, Option<DesignOutput>) = match result {
        Ok(out) => ("success".to_string(), "Synthesized design output:".to_string(), Some(out)),
        Err(raw_body) => ("error".to_string(), format!("LLM Response (Unparsed): {}", raw_body), None)
    };

    // DB update
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
        }

        let assistant_msg = Message {
            id: Uuid::new_v4().to_string(),
            role: "assistant".to_string(),
            content: content.clone(),
            status: status.clone(),
            output: output.clone(),
            image_data: None,
            timestamp: now + 1,
        };
        db::add_message(&db, &thread_id_actual, &assistant_msg).map_err(|e: rusqlite::Error| e.to_string())?;
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
        Ok(GenerateOutput { design: out, thread_id: thread_id_actual })
    } else {
        // Return thread_id even on error so frontend can stay in context
        Err(format!("ERR_ID:{}|{}", thread_id_actual, content))
    }
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
async fn update_ui_spec(message_id: String, ui_spec: serde_json::Value, state: State<'_, AppState>) -> Result<(), String> {
    let db = state.db.lock().unwrap();
    db::update_message_ui_spec(&db, &message_id, &ui_spec).map_err(|e| e.to_string())
}

#[tauri::command]
async fn update_parameters(message_id: String, parameters: serde_json::Value, state: State<'_, AppState>) -> Result<(), String> {
    let db = state.db.lock().unwrap();
    db::update_message_parameters(&db, &message_id, &parameters).map_err(|e| e.to_string())
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
2. "macro_code": The Python macro code.
3. "ui_spec": { 
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
4. "initial_params": { ... }

UI Guidelines:
- Use "range" for continuous dimensions.
- Use "select" (enums) for discrete choices. Ensure "options" are provided.
- Use "checkbox" for boolean flags (e.g., "Show Holes"). Value will be true or false.
"#;

const TECHNICAL_SYSTEM_PROMPT: &str = r#"Return a JSON object with:
1. "title": 2-5 words project title.
2. "version_name": Short descriptive name for this iteration.
3. "macro_code": FreeCAD Python code.
4. "ui_spec": { "fields": [ { "key": string, "label": string, "type": "range"|"number"|"select"|"checkbox" } ] }
5. "initial_params": { "key": value }

CRITICAL RULES:
- UNITS: ALL dimensions are in MILLIMETERS (mm).
- UI: Focus on 'key', 'label' and 'type'. Don't worry about 'min'/'max' for ranges; the system will calculate bounds based on your 'initial_params'.
- PARAMETERS: Access parameters directly by name (e.g. `L = connector_length`) or via `params.get("key", default)`.
- NO BRACES: NEVER use `{var}` style interpolation inside the macro_code string.
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

pub fn run() {
    let context = tauri::generate_context!();
    
    let default_config = Config {
        engines: vec![
            Engine {
                id: "default-gemini".to_string(),
                name: "Google Gemini".to_string(),
                provider: "gemini".to_string(),
                api_key: "".to_string(),
                model: "gemini-2.0-flash".to_string(),
                base_url: "".to_string(),
                system_prompt: DEFAULT_PROMPT.to_string(),
            }
        ],
        selected_engine_id: "default-gemini".to_string(),
        assets: vec![],
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
                    if let Ok(c) = serde_json::from_str::<Config>(&data) {
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
            generate_design,
            render_stl,
            list_models,
            get_default_macro,
            get_last_design,
            get_system_prompt,
            export_file,
            add_manual_version,
            update_ui_spec,
            update_parameters,
            upload_asset
        ])
        .run(context)
        .expect("error while running tauri application");
}
