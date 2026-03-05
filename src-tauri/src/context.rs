use crate::models::{DesignOutput, Message, ThreadReference};

pub const THREAD_SUMMARY_MAX_CHARS: usize = 1600;
pub const SUMMARY_ITEM_MAX_CHARS: usize = 220;
pub const RECENT_DIALOGUE_MAX_MESSAGES: usize = 6;
pub const RECENT_DIALOGUE_ITEM_MAX_CHARS: usize = 260;
pub const PINNED_REFERENCES_MAX_ITEMS: usize = 4;
pub const PINNED_REFERENCE_CONTENT_MAX_CHARS: usize = 2200;
pub const PINNED_REFERENCE_SUMMARY_MAX_CHARS: usize = 200;

pub fn compact_text(text: &str, max_chars: usize) -> String {
    let compact = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if compact.chars().count() <= max_chars {
        compact
    } else {
        let mut out = compact.chars().take(max_chars.saturating_sub(1)).collect::<String>();
        out.push('…');
        out
    }
}

pub fn latest_output(messages: &[Message]) -> Option<DesignOutput> {
    messages
        .iter()
        .rev()
        .find(|m| m.role == "assistant" && m.output.is_some())
        .and_then(|m| m.output.clone())
}

pub fn build_thread_summary(title: &str, messages: &[Message]) -> String {
    let mut sections: Vec<String> = Vec::new();

    if !title.trim().is_empty() {
        sections.push(format!("Thread: {}", compact_text(title, SUMMARY_ITEM_MAX_CHARS)));
    }

    if let Some(output) = latest_output(messages).as_ref() {
        let mut anchor = format!("Current version anchor: {} [{}]", output.title, output.version_name);
        if !output.response.trim().is_empty() {
            anchor.push_str(&format!(" - {}", compact_text(&output.response, SUMMARY_ITEM_MAX_CHARS)));
        }
        sections.push(anchor);
    }

    let recent_user_intents = messages
        .iter()
        .filter(|m| m.role == "user")
        .rev()
        .take(4)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .map(|m| format!("- {}", compact_text(&m.content, SUMMARY_ITEM_MAX_CHARS)))
        .collect::<Vec<_>>();
    if !recent_user_intents.is_empty() {
        sections.push(format!("Recent user intents:\n{}", recent_user_intents.join("\n")));
    }

    let recent_assistant_decisions = messages
        .iter()
        .filter(|m| m.role == "assistant")
        .rev()
        .take(4)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .map(|m| {
            if let Some(output) = &m.output {
                let mut line = format!("{} [{}]", output.title, output.version_name);
                if !output.response.trim().is_empty() {
                    line.push_str(&format!(" - {}", compact_text(&output.response, SUMMARY_ITEM_MAX_CHARS)));
                }
                format!("- {}", line)
            } else {
                format!("- Q/A: {}", compact_text(&m.content, SUMMARY_ITEM_MAX_CHARS))
            }
        })
        .collect::<Vec<_>>();
    if !recent_assistant_decisions.is_empty() {
        sections.push(format!("Recent assistant outcomes:\n{}", recent_assistant_decisions.join("\n")));
    }

    compact_text(&sections.join("\n\n"), THREAD_SUMMARY_MAX_CHARS)
}

pub fn build_recent_dialogue(messages: &[Message]) -> String {
    messages
        .iter()
        .rev()
        .take(RECENT_DIALOGUE_MAX_MESSAGES)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .map(|m| {
            let speaker = if m.role == "user" { "USER" } else { "ASSISTANT" };
            format!("{}: {}", speaker, compact_text(&m.content, RECENT_DIALOGUE_ITEM_MAX_CHARS))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn build_pinned_references_block(references: &[ThreadReference]) -> String {
    references
        .iter()
        .filter(|r| !r.content.trim().is_empty() || !r.summary.trim().is_empty())
        .rev()
        .take(PINNED_REFERENCES_MAX_ITEMS)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .map(|r| {
            let body = if !r.content.trim().is_empty() {
                compact_text(&r.content, PINNED_REFERENCE_CONTENT_MAX_CHARS)
            } else {
                r.summary.clone()
            };
            format!(
                "- {} [{}]\n{}\n",
                r.name,
                r.kind,
                compact_text(&body, PINNED_REFERENCE_CONTENT_MAX_CHARS)
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub struct PromptContext {
    pub thread_id: String,
    pub thread_title: String,
    pub summary: String,
    pub recent_dialogue: String,
    pub pinned_references: String,
    pub last_output: Option<DesignOutput>,
}

pub fn assemble_context(
    db: &rusqlite::Connection,
    thread_id: Option<String>,
    working_design: Option<DesignOutput>,
    parent_macro_code: Option<String>,
) -> PromptContext {
    if let Some(tid) = thread_id {
        let messages = crate::db::get_thread_messages(db, &tid).unwrap_or_default();
        let last_o = latest_output(&messages);
        let summary = crate::db::get_thread_summary(db, &tid)
            .ok()
            .flatten()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| build_thread_summary(
                &crate::db::get_thread_title(db, &tid).ok().flatten().unwrap_or_default(),
                &messages
            ));
        let dialogue = build_recent_dialogue(&messages);
        let title = crate::db::get_thread_title(db, &tid).ok().flatten().unwrap_or_default();
        let refs = crate::db::get_thread_references(db, &tid).unwrap_or_default();
        
        PromptContext {
            thread_id: tid,
            thread_title: title,
            summary,
            recent_dialogue: dialogue,
            pinned_references: build_pinned_references_block(&refs),
            last_output: working_design.or(last_o),
        }
    } else {
        let fallback_output = parent_macro_code.map(|code| DesignOutput {
            title: "Untitled Design".to_string(),
            version_name: "V1".to_string(),
            response: String::new(),
            interaction_mode: "design".to_string(),
            macro_code: code,
            ui_spec: serde_json::json!({ "fields": [] }),
            initial_params: serde_json::json!({}),
        });
        
        PromptContext {
            thread_id: uuid::Uuid::new_v4().to_string(),
            thread_title: String::new(),
            summary: String::new(),
            recent_dialogue: String::new(),
            pinned_references: String::new(),
            last_output: working_design.or(fallback_output),
        }
    }
}

pub fn format_contextual_prompt(ctx: &PromptContext, base_prompt: &str, system_prompt: &str, intent_mode: &str) -> String {
    let full_prompt = format!("{}\n\n{}\n\nUSER_INTENT_MODE: {}", base_prompt, system_prompt, intent_mode);

    if let Some(previous) = &ctx.last_output {
        let ui_spec_json = serde_json::to_string_pretty(&previous.ui_spec).unwrap_or_else(|_| "{}".to_string());
        let params_json = serde_json::to_string_pretty(&previous.initial_params).unwrap_or_else(|_| "{}".to_string());
        
        format!(
            "CURRENT DESIGN CONTEXT\nThread Title: {}\nCurrent Title: {}\nVersion: {}\n\nTHREAD SUMMARY\n{}\n\nRECENT DIALOGUE\n{}\n\nPINNED REFERENCES\n{}\n\nCurrent FreeCAD Macro:\n```python\n{}\n```\n\nCurrent UI Spec:\n```json\n{}\n```\n\nCurrent Initial Params:\n```json\n{}\n```\n\nUSER REQUEST:\n{}",
            ctx.thread_title,
            previous.title,
            previous.version_name,
            if ctx.summary.trim().is_empty() { "[none]" } else { &ctx.summary },
            if ctx.recent_dialogue.trim().is_empty() { "[none]" } else { &ctx.recent_dialogue },
            if ctx.pinned_references.trim().is_empty() { "[none]" } else { &ctx.pinned_references },
            previous.macro_code,
            ui_spec_json,
            params_json,
            full_prompt
        )
    } else {
        full_prompt
    }
}
