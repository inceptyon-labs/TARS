//! Project metadata storage operations

use super::db::DatabaseError;
use chrono::Utc;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Structured project metadata
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProjectMetadata {
    /// Description
    #[serde(default)]
    pub description: Option<String>,

    /// Custom icon path (relative to project root)
    #[serde(default)]
    pub icon_path: Option<String>,

    /// Platforms & Framework
    #[serde(default)]
    pub platforms: Vec<String>,
    #[serde(default)]
    pub app_framework: Option<String>,

    /// Hosting & Deployment (Web)
    #[serde(default)]
    pub deploy_target: Option<String>,
    #[serde(default)]
    pub web_hosting: Option<String>,
    pub domain: Option<String>,
    pub production_url: Option<String>,
    pub staging_url: Option<String>,
    pub deploy_command: Option<String>,

    /// Data & Storage
    pub database_provider: Option<String>,
    pub database_name: Option<String>,
    #[serde(default)]
    pub database_dashboard_url: Option<String>,
    pub object_storage: Option<String>,
    pub object_storage_bucket: Option<String>,

    /// Local Development
    pub start_command: Option<String>,
    pub requires_tunnel: bool,
    pub tunnel_provider: Option<String>,
    pub tunnel_id: Option<String>,

    /// Source & Distribution
    pub github_url: Option<String>,
    pub app_store_url: Option<String>,
    pub app_store_connect_url: Option<String>,
    pub play_store_url: Option<String>,
    pub package_registry_url: Option<String>,

    /// Infrastructure
    pub ci_cd: Option<String>,
    pub monitoring: Option<String>,

    /// iOS
    #[serde(default)]
    pub ios_deploy_target: Option<String>,
    #[serde(default)]
    pub ios_bundle_id: Option<String>,
    #[serde(default)]
    pub ios_signing_team: Option<String>,
    #[serde(default)]
    pub ios_cloudkit_container: Option<String>,
    #[serde(default)]
    pub ios_cloudkit_dashboard_url: Option<String>,
    #[serde(default)]
    pub ios_uses_push_notifications: bool,
    #[serde(default)]
    pub ios_provisioning: Option<String>,
    /// Legacy single deploy command (migrated to `ios_deploy_commands`)
    #[serde(default)]
    pub ios_deploy_command: Option<String>,
    /// Deploy steps for iOS — copyable commands and informational notes
    #[serde(default, deserialize_with = "deserialize_deploy_steps")]
    pub ios_deploy_commands: Vec<DeployStep>,

    /// Android
    #[serde(default)]
    pub android_package_name: Option<String>,
    #[serde(default)]
    pub android_min_sdk: Option<String>,
    #[serde(default)]
    pub android_target_sdk: Option<String>,
    #[serde(default)]
    pub android_signing_key: Option<String>,
    /// Legacy single deploy command (migrated to `android_deploy_commands`)
    #[serde(default)]
    pub android_deploy_command: Option<String>,
    /// Deploy steps for Android — copyable commands and informational notes
    #[serde(default, deserialize_with = "deserialize_deploy_steps")]
    pub android_deploy_commands: Vec<DeployStep>,
    #[serde(default)]
    pub google_play_console_url: Option<String>,

    /// macOS
    #[serde(default)]
    pub macos_bundle_id: Option<String>,
    #[serde(default)]
    pub macos_signing_team: Option<String>,
    #[serde(default)]
    pub macos_app_category: Option<String>,
    #[serde(default)]
    pub macos_hardened_runtime: bool,
    #[serde(default)]
    pub macos_app_sandbox: bool,
    #[serde(default)]
    pub macos_provisioning: Option<String>,
    #[serde(default)]
    pub macos_deploy_commands: Vec<String>,

    /// Homebrew
    #[serde(default)]
    pub homebrew_formula_name: Option<String>,
    #[serde(default)]
    pub homebrew_tap: Option<String>,
    #[serde(default)]
    pub homebrew_deploy_commands: Vec<String>,

    /// Multiple web deploy commands
    #[serde(default)]
    pub deploy_commands: Vec<String>,

    /// General deploy steps — an ordered list of copyable commands and
    /// non-copyable notes/headers (e.g. a script that deploys to multiple
    /// targets at once, with context between commands)
    #[serde(default)]
    pub deploy_steps: Vec<DeployStep>,

    /// Custom key-value pairs
    #[serde(default)]
    pub custom_fields: Vec<CustomField>,
}

/// A user-defined key-value field
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomField {
    pub key: String,
    pub value: String,
}

/// A single deploy step: either a copyable `command` or an informational
/// `note`/header rendered as plain text.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeployStep {
    /// Either "command" or "note"
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub text: String,
}

/// Deserialize a list of deploy steps, tolerating the legacy format where
/// each entry was a bare command string. Bare strings become `command` steps.
fn deserialize_deploy_steps<'de, D>(deserializer: D) -> Result<Vec<DeployStep>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StepOrString {
        Step(DeployStep),
        Str(String),
    }

    let items = Vec::<StepOrString>::deserialize(deserializer)?;
    Ok(items
        .into_iter()
        .map(|item| match item {
            StepOrString::Step(step) => step,
            StepOrString::Str(text) => DeployStep {
                kind: "command".to_string(),
                text,
            },
        })
        .collect())
}

/// Metadata storage operations
pub struct MetadataStore<'a> {
    conn: &'a Connection,
}

impl<'a> MetadataStore<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// Get metadata for a project
    pub fn get(&self, project_id: Uuid) -> Result<Option<ProjectMetadata>, DatabaseError> {
        let mut stmt = self
            .conn
            .prepare("SELECT data FROM project_metadata WHERE project_id = ?1")?;

        let result = stmt.query_row(params![project_id.to_string()], |row| {
            let json: String = row.get(0)?;
            Ok(json)
        });

        match result {
            Ok(json) => {
                let metadata: ProjectMetadata = serde_json::from_str(&json).map_err(|e| {
                    DatabaseError::Migration(format!("Failed to parse metadata: {e}"))
                })?;
                Ok(Some(metadata))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Save metadata for a project (upsert)
    pub fn save(&self, project_id: Uuid, metadata: &ProjectMetadata) -> Result<(), DatabaseError> {
        let json = serde_json::to_string(metadata)
            .map_err(|e| DatabaseError::Migration(format!("Failed to serialize metadata: {e}")))?;
        let now = Utc::now().to_rfc3339();

        self.conn.execute(
            r"
            INSERT INTO project_metadata (project_id, data, updated_at)
            VALUES (?1, ?2, ?3)
            ON CONFLICT(project_id) DO UPDATE SET
                data = excluded.data,
                updated_at = excluded.updated_at
            ",
            params![project_id.to_string(), json, now],
        )?;

        Ok(())
    }

    /// Delete metadata for a project
    pub fn delete(&self, project_id: Uuid) -> Result<bool, DatabaseError> {
        let deleted = self.conn.execute(
            "DELETE FROM project_metadata WHERE project_id = ?1",
            params![project_id.to_string()],
        )?;
        Ok(deleted > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Serialize a default metadata, override one field, return the JSON —
    /// mirrors how `get()` reads back fully-serialized rows.
    fn metadata_json_with(field: &str, value: serde_json::Value) -> String {
        let mut obj: serde_json::Value = serde_json::to_value(ProjectMetadata::default()).unwrap();
        obj[field] = value;
        obj.to_string()
    }

    #[test]
    fn deploy_steps_accept_legacy_string_array() {
        // Pre-existing data stored deploy commands as bare strings.
        let json = metadata_json_with(
            "ios_deploy_commands",
            serde_json::json!(["fastlane beta", "xcodebuild archive"]),
        );
        let meta: ProjectMetadata = serde_json::from_str(&json).unwrap();
        assert_eq!(meta.ios_deploy_commands.len(), 2);
        assert_eq!(meta.ios_deploy_commands[0].kind, "command");
        assert_eq!(meta.ios_deploy_commands[0].text, "fastlane beta");
    }

    #[test]
    fn deploy_steps_accept_structured_form() {
        let json = metadata_json_with(
            "android_deploy_commands",
            serde_json::json!([
                {"kind": "note", "text": "Deploy to both stores"},
                {"kind": "command", "text": "./gradlew bundleRelease"}
            ]),
        );
        let meta: ProjectMetadata = serde_json::from_str(&json).unwrap();
        assert_eq!(meta.android_deploy_commands.len(), 2);
        assert_eq!(meta.android_deploy_commands[0].kind, "note");
        assert_eq!(
            meta.android_deploy_commands[1].text,
            "./gradlew bundleRelease"
        );
    }
}
