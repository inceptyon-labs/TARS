//! Beacons management Tauri commands
//!
//! Commands for managing navigation beacons - links to GitHub repos,
//! documentation, and other resources.
//! Stored in ~/.tars/beacons/ (not in Claude config locations).

use crate::state::AppState;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::State;
use uuid::Uuid;

/// Beacon type for categorization
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum BeaconType {
    Github,
    Documentation,
    Api,
    Resource,
    Reddit,
    Twitter,
    #[default]
    Custom,
}

/// A link within a beacon
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeaconLink {
    pub label: Option<String>,
    pub url: String,
}

/// Beacon metadata and content
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(clippy::struct_field_names)]
pub struct Beacon {
    pub id: String,
    pub title: String,
    pub category: Option<String>,
    pub links: Vec<BeaconLink>,
    pub description: Option<String>,
    pub beacon_type: BeaconType,
    pub tags: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Beacon summary for listing
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(clippy::struct_field_names)]
pub struct BeaconSummary {
    pub id: String,
    pub title: String,
    pub category: Option<String>,
    pub links: Vec<BeaconLink>,
    pub beacon_type: BeaconType,
    pub tags: Vec<String>,
    pub updated_at: String,
}

/// Get the beacons directory
fn beacons_dir(state: &AppState) -> PathBuf {
    state.data_dir().join("beacons")
}

/// Validate beacon ID is a valid UUID format (prevents path traversal)
fn validate_beacon_id(id: &str) -> Result<(), String> {
    // UUID format: 8-4-4-4-12 hex characters with dashes
    if id.len() != 36 {
        return Err("Invalid beacon ID format".into());
    }
    if !id.chars().all(|c| c.is_ascii_hexdigit() || c == '-') {
        return Err("Invalid beacon ID format".into());
    }
    Ok(())
}

/// Get the path for a specific beacon
fn beacon_path(state: &AppState, id: &str) -> Result<PathBuf, String> {
    validate_beacon_id(id)?;
    Ok(beacons_dir(state).join(format!("{id}.json")))
}

/// Internal beacon data stored on disk (current version)
#[derive(Debug, Serialize, Deserialize)]
struct BeaconData {
    id: String,
    title: String,
    #[serde(default)]
    category: Option<String>,
    #[serde(default)]
    links: Vec<BeaconLink>,
    // Legacy field for backwards compatibility
    #[serde(skip_serializing)]
    url: Option<String>,
    description: Option<String>,
    beacon_type: BeaconType,
    tags: Vec<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl BeaconData {
    /// Get links, migrating from legacy url field if needed
    fn get_links(&self) -> Vec<BeaconLink> {
        if !self.links.is_empty() {
            self.links.clone()
        } else if let Some(url) = &self.url {
            vec![BeaconLink {
                label: None,
                url: url.clone(),
            }]
        } else {
            Vec::new()
        }
    }
}

/// List all beacons
#[tauri::command]
pub async fn list_beacons(state: State<'_, AppState>) -> Result<Vec<BeaconSummary>, String> {
    let dir = beacons_dir(&state);

    // Ensure directory exists
    if !dir.exists() {
        fs::create_dir_all(&dir).map_err(|e| format!("Failed to create beacons directory: {e}"))?;
        return Ok(Vec::new());
    }

    let mut beacons = Vec::new();

    let entries =
        fs::read_dir(&dir).map_err(|e| format!("Failed to read beacons directory: {e}"))?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "json") {
            let content =
                fs::read_to_string(&path).map_err(|e| format!("Failed to read beacon: {e}"))?;
            let data: BeaconData = serde_json::from_str(&content)
                .map_err(|e| format!("Failed to parse beacon: {e}"))?;

            let links = data.get_links();
            beacons.push(BeaconSummary {
                id: data.id,
                title: data.title,
                category: data.category,
                links,
                beacon_type: data.beacon_type,
                tags: data.tags,
                updated_at: data.updated_at.to_rfc3339(),
            });
        }
    }

    // Sort by updated_at descending (newest first)
    beacons.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

    Ok(beacons)
}

/// Read a single beacon
#[tauri::command]
pub async fn read_beacon(id: String, state: State<'_, AppState>) -> Result<Beacon, String> {
    let path = beacon_path(&state, &id)?;

    if !path.exists() {
        return Err("Beacon not found".into());
    }

    let content = fs::read_to_string(&path).map_err(|e| format!("Failed to read beacon: {e}"))?;
    let data: BeaconData =
        serde_json::from_str(&content).map_err(|e| format!("Failed to parse beacon: {e}"))?;

    let links = data.get_links();
    Ok(Beacon {
        id: data.id,
        title: data.title,
        category: data.category,
        links,
        description: data.description,
        beacon_type: data.beacon_type,
        tags: data.tags,
        created_at: data.created_at.to_rfc3339(),
        updated_at: data.updated_at.to_rfc3339(),
    })
}

/// Create a new beacon
#[tauri::command]
pub async fn create_beacon(
    title: String,
    category: Option<String>,
    links: Vec<BeaconLink>,
    description: Option<String>,
    beacon_type: BeaconType,
    tags: Vec<String>,
    state: State<'_, AppState>,
) -> Result<Beacon, String> {
    // Input validation
    if title.trim().is_empty() {
        return Err("Title cannot be empty".into());
    }
    if title.len() > 500 {
        return Err("Title is too long (max 500 characters)".into());
    }
    if links.len() > 50 {
        return Err("Too many links (max 50)".into());
    }
    if tags.len() > 100 {
        return Err("Too many tags (max 100)".into());
    }

    let dir = beacons_dir(&state);

    // Ensure directory exists
    fs::create_dir_all(&dir).map_err(|e| format!("Failed to create beacons directory: {e}"))?;

    let id = Uuid::new_v4().to_string();
    let now = Utc::now();

    let data = BeaconData {
        id: id.clone(),
        title: title.clone(),
        category: category.clone(),
        links: links.clone(),
        url: None,
        description: description.clone(),
        beacon_type: beacon_type.clone(),
        tags: tags.clone(),
        created_at: now,
        updated_at: now,
    };

    // Write beacon (ID is already validated by Uuid::new_v4())
    let path = beacons_dir(&state).join(format!("{id}.json"));
    let json = serde_json::to_string_pretty(&data)
        .map_err(|e| format!("Failed to serialize beacon: {e}"))?;
    fs::write(&path, json).map_err(|e| format!("Failed to write beacon: {e}"))?;

    Ok(Beacon {
        id,
        title,
        category,
        links,
        description,
        beacon_type,
        tags,
        created_at: now.to_rfc3339(),
        updated_at: now.to_rfc3339(),
    })
}

/// Update an existing beacon
#[tauri::command]
pub async fn update_beacon(
    id: String,
    title: String,
    category: Option<String>,
    links: Vec<BeaconLink>,
    description: Option<String>,
    beacon_type: BeaconType,
    tags: Vec<String>,
    state: State<'_, AppState>,
) -> Result<Beacon, String> {
    // Input validation (same as create_beacon)
    if title.trim().is_empty() {
        return Err("Title cannot be empty".into());
    }
    if title.len() > 500 {
        return Err("Title is too long (max 500 characters)".into());
    }
    if links.len() > 50 {
        return Err("Too many links (max 50)".into());
    }
    if tags.len() > 100 {
        return Err("Too many tags (max 100)".into());
    }

    let path = beacon_path(&state, &id)?;

    if !path.exists() {
        return Err("Beacon not found".into());
    }

    // Read existing data to preserve created_at
    let existing_content =
        fs::read_to_string(&path).map_err(|e| format!("Failed to read beacon: {e}"))?;
    let existing: BeaconData = serde_json::from_str(&existing_content)
        .map_err(|e| format!("Failed to parse beacon: {e}"))?;

    let now = Utc::now();

    let data = BeaconData {
        id: id.clone(),
        title: title.clone(),
        category: category.clone(),
        links: links.clone(),
        url: None,
        description: description.clone(),
        beacon_type: beacon_type.clone(),
        tags: tags.clone(),
        created_at: existing.created_at,
        updated_at: now,
    };

    // Write updated beacon
    let json = serde_json::to_string_pretty(&data)
        .map_err(|e| format!("Failed to serialize beacon: {e}"))?;
    fs::write(&path, json).map_err(|e| format!("Failed to write beacon: {e}"))?;

    Ok(Beacon {
        id,
        title,
        category,
        links,
        description,
        beacon_type,
        tags,
        created_at: existing.created_at.to_rfc3339(),
        updated_at: now.to_rfc3339(),
    })
}

/// Delete a beacon
#[tauri::command]
pub async fn delete_beacon(id: String, state: State<'_, AppState>) -> Result<(), String> {
    let path = beacon_path(&state, &id)?;

    if !path.exists() {
        return Err("Beacon not found".into());
    }

    fs::remove_file(&path).map_err(|e| format!("Failed to delete beacon: {e}"))?;

    Ok(())
}
