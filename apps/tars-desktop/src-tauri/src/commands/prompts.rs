//! Prompts management Tauri commands
//!
//! Commands for managing personal prompts and notes.
//! Stored in ~/.tars/prompts/ (not in Claude config locations).

use crate::state::AppState;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::State;
use uuid::Uuid;

/// Prompt metadata and content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prompt {
    pub id: String,
    pub title: String,
    pub content: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Prompt summary for listing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptSummary {
    pub id: String,
    pub title: String,
    pub preview: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Get the prompts directory
fn prompts_dir(state: &AppState) -> PathBuf {
    state.data_dir().join("prompts")
}

/// Get the path for a specific prompt
fn prompt_path(state: &AppState, id: &str) -> PathBuf {
    prompts_dir(state).join(format!("{}.md", id))
}

/// Get the metadata path for a prompt
fn prompt_meta_path(state: &AppState, id: &str) -> PathBuf {
    prompts_dir(state).join(format!("{}.json", id))
}

/// Prompt metadata stored alongside content
#[derive(Debug, Serialize, Deserialize)]
struct PromptMeta {
    id: String,
    title: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

/// List all prompts
#[tauri::command]
pub async fn list_prompts(state: State<'_, AppState>) -> Result<Vec<PromptSummary>, String> {
    let dir = prompts_dir(&state);

    // Ensure directory exists
    if !dir.exists() {
        fs::create_dir_all(&dir)
            .map_err(|e| format!("Failed to create prompts directory: {e}"))?;
        return Ok(Vec::new());
    }

    let mut prompts = Vec::new();

    let entries = fs::read_dir(&dir)
        .map_err(|e| format!("Failed to read prompts directory: {e}"))?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "json") {
            let id = path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or_default()
                .to_string();

            // Read metadata
            let meta_content = fs::read_to_string(&path)
                .map_err(|e| format!("Failed to read prompt metadata: {e}"))?;
            let meta: PromptMeta = serde_json::from_str(&meta_content)
                .map_err(|e| format!("Failed to parse prompt metadata: {e}"))?;

            // Read content for preview
            let content_path = prompt_path(&state, &id);
            let content = fs::read_to_string(&content_path).unwrap_or_default();
            let preview = content.lines().take(2).collect::<Vec<_>>().join(" ");
            let preview = if preview.len() > 100 {
                format!("{}...", &preview[..100])
            } else {
                preview
            };

            prompts.push(PromptSummary {
                id: meta.id,
                title: meta.title,
                preview,
                created_at: meta.created_at.to_rfc3339(),
                updated_at: meta.updated_at.to_rfc3339(),
            });
        }
    }

    // Sort by updated_at descending (newest first)
    prompts.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

    Ok(prompts)
}

/// Read a single prompt
#[tauri::command]
pub async fn read_prompt(id: String, state: State<'_, AppState>) -> Result<Prompt, String> {
    let meta_path = prompt_meta_path(&state, &id);
    let content_path = prompt_path(&state, &id);

    if !meta_path.exists() {
        return Err(format!("Prompt not found: {id}"));
    }

    // Read metadata
    let meta_content = fs::read_to_string(&meta_path)
        .map_err(|e| format!("Failed to read prompt metadata: {e}"))?;
    let meta: PromptMeta = serde_json::from_str(&meta_content)
        .map_err(|e| format!("Failed to parse prompt metadata: {e}"))?;

    // Read content
    let content = fs::read_to_string(&content_path)
        .map_err(|e| format!("Failed to read prompt content: {e}"))?;

    Ok(Prompt {
        id: meta.id,
        title: meta.title,
        content,
        created_at: meta.created_at.to_rfc3339(),
        updated_at: meta.updated_at.to_rfc3339(),
    })
}

/// Create a new prompt
#[tauri::command]
pub async fn create_prompt(
    title: String,
    content: String,
    state: State<'_, AppState>,
) -> Result<Prompt, String> {
    let dir = prompts_dir(&state);

    // Ensure directory exists
    fs::create_dir_all(&dir)
        .map_err(|e| format!("Failed to create prompts directory: {e}"))?;

    let id = Uuid::new_v4().to_string();
    let now = Utc::now();

    let meta = PromptMeta {
        id: id.clone(),
        title: title.clone(),
        created_at: now,
        updated_at: now,
    };

    // Write metadata
    let meta_path = prompt_meta_path(&state, &id);
    let meta_json = serde_json::to_string_pretty(&meta)
        .map_err(|e| format!("Failed to serialize prompt metadata: {e}"))?;
    fs::write(&meta_path, meta_json)
        .map_err(|e| format!("Failed to write prompt metadata: {e}"))?;

    // Write content
    let content_path = prompt_path(&state, &id);
    fs::write(&content_path, &content)
        .map_err(|e| format!("Failed to write prompt content: {e}"))?;

    Ok(Prompt {
        id,
        title,
        content,
        created_at: now.to_rfc3339(),
        updated_at: now.to_rfc3339(),
    })
}

/// Update an existing prompt
#[tauri::command]
pub async fn update_prompt(
    id: String,
    title: String,
    content: String,
    state: State<'_, AppState>,
) -> Result<Prompt, String> {
    let meta_path = prompt_meta_path(&state, &id);
    let content_path = prompt_path(&state, &id);

    if !meta_path.exists() {
        return Err(format!("Prompt not found: {id}"));
    }

    // Read existing metadata to preserve created_at
    let existing_meta_content = fs::read_to_string(&meta_path)
        .map_err(|e| format!("Failed to read prompt metadata: {e}"))?;
    let existing_meta: PromptMeta = serde_json::from_str(&existing_meta_content)
        .map_err(|e| format!("Failed to parse prompt metadata: {e}"))?;

    let now = Utc::now();

    let meta = PromptMeta {
        id: id.clone(),
        title: title.clone(),
        created_at: existing_meta.created_at,
        updated_at: now,
    };

    // Write metadata
    let meta_json = serde_json::to_string_pretty(&meta)
        .map_err(|e| format!("Failed to serialize prompt metadata: {e}"))?;
    fs::write(&meta_path, meta_json)
        .map_err(|e| format!("Failed to write prompt metadata: {e}"))?;

    // Write content
    fs::write(&content_path, &content)
        .map_err(|e| format!("Failed to write prompt content: {e}"))?;

    Ok(Prompt {
        id,
        title,
        content,
        created_at: existing_meta.created_at.to_rfc3339(),
        updated_at: now.to_rfc3339(),
    })
}

/// Delete a prompt
#[tauri::command]
pub async fn delete_prompt(id: String, state: State<'_, AppState>) -> Result<(), String> {
    let meta_path = prompt_meta_path(&state, &id);
    let content_path = prompt_path(&state, &id);

    if !meta_path.exists() {
        return Err(format!("Prompt not found: {id}"));
    }

    // Delete both files
    fs::remove_file(&meta_path)
        .map_err(|e| format!("Failed to delete prompt metadata: {e}"))?;

    if content_path.exists() {
        fs::remove_file(&content_path)
            .map_err(|e| format!("Failed to delete prompt content: {e}"))?;
    }

    Ok(())
}
