//! App-data backup and restore commands.
//!
//! These backups protect TARS' own `SQLite` database (`~/.tars/tars.db`). They
//! are separate from project rollback backups, which only cover project files.

use crate::state::AppState;
use aes_gcm::aead::{Aead, OsRng};
use aes_gcm::{AeadCore, Aes256Gcm, Key, KeyInit, Nonce};
use argon2::Argon2;
use chrono::Utc;
use rand::RngCore;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use tars_core::crypto;
use tauri::State;
use zip::write::FileOptions;
use zip::{ZipArchive, ZipWriter};

const LOCAL_BACKUP_EXT: &str = "tars-backup";
const PORTABLE_BACKUP_EXT: &str = "tars-portable-backup";
const PORTABLE_KDF_MEMORY_KIB: u32 = 19 * 1024;
const PORTABLE_KDF_ITERATIONS: u32 = 3;
const PORTABLE_KDF_PARALLELISM: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppDataBackupInfo {
    pub path: String,
    pub file_name: String,
    pub backup_type: String,
    pub created_at: String,
    pub size_bytes: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestoreAppDataBackupResult {
    pub restored: bool,
    pub backup_before_restore_path: String,
    pub restored_from: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppDataBackupDirectory {
    pub path: String,
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BackupManifest {
    format: String,
    backup_type: String,
    created_at: String,
    tars_version: String,
    schema_version: i64,
    db_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PortableEnvelope {
    format: String,
    backup_type: String,
    created_at: String,
    tars_version: String,
    kdf: PortableKdf,
    salt_hex: String,
    nonce_hex: String,
    ciphertext_hex: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PortableKdf {
    algorithm: String,
    memory_kib: u32,
    iterations: u32,
    parallelism: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PortableSecrets {
    api_keys: Vec<PortableSecretRow>,
    project_secrets: Vec<PortableProjectSecretRow>,
    developer_credentials: Vec<PortableSecretRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PortableSecretRow {
    id: i64,
    plaintext: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PortableProjectSecretRow {
    project_id: String,
    name: String,
    plaintext_json: String,
}

#[tauri::command]
pub async fn create_local_app_data_backup(
    output_path: Option<String>,
    state: State<'_, AppState>,
) -> Result<AppDataBackupInfo, String> {
    let backup_path = output_path.map_or_else(
        || {
            configured_backup_dir(&state).map_or_else(
                |_| default_backup_path(state.data_dir(), LOCAL_BACKUP_EXT),
                |dir| default_backup_path_in_dir(&dir, LOCAL_BACKUP_EXT),
            )
        },
        PathBuf::from,
    );
    create_local_backup_at(&state, &backup_path, "local")
}

#[tauri::command]
pub async fn create_portable_app_data_backup(
    passphrase: String,
    output_path: Option<String>,
    state: State<'_, AppState>,
) -> Result<AppDataBackupInfo, String> {
    if passphrase.len() < 8 {
        return Err("Portable backup passphrase must be at least 8 characters.".to_string());
    }

    let backup_path = output_path.map_or_else(
        || {
            configured_backup_dir(&state).map_or_else(
                |_| default_backup_path(state.data_dir(), PORTABLE_BACKUP_EXT),
                |dir| default_backup_path_in_dir(&dir, PORTABLE_BACKUP_EXT),
            )
        },
        PathBuf::from,
    );

    let (snapshot_path, manifest, portable_secrets) = state.with_db(|db| {
        let temp_dir = state.data_dir().join(".backup-tmp");
        fs::create_dir_all(&temp_dir)
            .map_err(|e| format!("Failed to create temporary backup directory: {e}"))?;
        let snapshot_path = temp_dir.join(format!("snapshot-{}.db", Utc::now().timestamp_micros()));
        snapshot_database(db.connection(), &snapshot_path)?;
        let db_bytes =
            fs::read(&snapshot_path).map_err(|e| format!("Failed to read DB snapshot: {e}"))?;
        let manifest = BackupManifest {
            format: "tars-app-data-backup-v1".to_string(),
            backup_type: "portable".to_string(),
            created_at: Utc::now().to_rfc3339(),
            tars_version: env!("CARGO_PKG_VERSION").to_string(),
            schema_version: db
                .connection()
                .pragma_query_value(None, "user_version", |row| row.get(0))
                .map_err(|e| format!("Failed to read schema version: {e}"))?,
            db_sha256: sha256_hex(&db_bytes),
        };
        let portable_secrets = collect_portable_secrets(db.connection())?;
        Ok((snapshot_path, manifest, portable_secrets))
    })?;

    let zip_bytes = build_backup_zip(&snapshot_path, &manifest, Some(&portable_secrets))?;
    let _ = fs::remove_file(&snapshot_path);
    write_portable_envelope(&backup_path, &manifest, &zip_bytes, &passphrase)?;
    backup_info_from_path(&backup_path, "portable", manifest.created_at)
}

#[tauri::command]
pub async fn restore_app_data_backup(
    path: String,
    passphrase: Option<String>,
    state: State<'_, AppState>,
) -> Result<RestoreAppDataBackupResult, String> {
    let restore_path = PathBuf::from(&path);
    if !restore_path.exists() {
        return Err(format!("Backup not found: {}", restore_path.display()));
    }

    let emergency_path = configured_backup_dir(&state).map_or_else(
        |_| default_backup_path_with_prefix(state.data_dir(), "before-restore", LOCAL_BACKUP_EXT),
        |dir| default_backup_path_with_prefix_in_dir(&dir, "before-restore", LOCAL_BACKUP_EXT),
    );
    let emergency = create_local_backup_at(&state, &emergency_path, "before-restore")?;

    let (db_bytes, portable_secrets) = read_backup_payload(&restore_path, passphrase.as_deref())?;
    verify_db_bytes(&db_bytes)?;

    state.close_database();
    let db_path = state.data_dir().join("tars.db");
    fs::create_dir_all(state.data_dir())
        .map_err(|e| format!("Failed to create data directory: {e}"))?;
    fs::write(&db_path, db_bytes).map_err(|e| format!("Failed to restore database: {e}"))?;
    remove_sqlite_sidecars(&db_path);
    state.init_database()?;

    if let Some(secrets) = portable_secrets {
        state.with_db(|db| apply_portable_secrets(db.connection(), &secrets))?;
    }

    Ok(RestoreAppDataBackupResult {
        restored: true,
        backup_before_restore_path: emergency.path,
        restored_from: restore_path.display().to_string(),
    })
}

#[tauri::command]
pub async fn list_app_data_backups(
    state: State<'_, AppState>,
) -> Result<Vec<AppDataBackupInfo>, String> {
    let backup_dir = configured_backup_dir(&state)?;
    if !backup_dir.exists() {
        return Ok(Vec::new());
    }

    let mut backups = Vec::new();
    for entry in fs::read_dir(&backup_dir).map_err(|e| format!("Failed to list backups: {e}"))? {
        let entry = entry.map_err(|e| format!("Failed to read backup entry: {e}"))?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let extension = path.extension().and_then(|ext| ext.to_str());
        let backup_type = match extension {
            Some(LOCAL_BACKUP_EXT) => "local",
            Some(PORTABLE_BACKUP_EXT) => "portable",
            _ => continue,
        };
        let created_at = backup_created_at(&path).unwrap_or_else(|| {
            entry
                .metadata()
                .and_then(|m| m.modified())
                .ok()
                .map_or_else(Utc::now, chrono::DateTime::<Utc>::from)
                .to_rfc3339()
        });
        backups.push(backup_info_from_path(&path, backup_type, created_at)?);
    }

    backups.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(backups)
}

#[tauri::command]
pub async fn get_app_data_backup_dir(
    state: State<'_, AppState>,
) -> Result<AppDataBackupDirectory, String> {
    let backup_dir = configured_backup_dir(&state)?;
    fs::create_dir_all(&backup_dir)
        .map_err(|e| format!("Failed to create app data backup directory: {e}"))?;
    Ok(AppDataBackupDirectory {
        path: backup_dir.display().to_string(),
        is_default: get_backup_dir_setting(&state)?.is_none(),
    })
}

#[tauri::command]
pub async fn set_app_data_backup_dir(
    path: Option<String>,
    state: State<'_, AppState>,
) -> Result<AppDataBackupDirectory, String> {
    let normalized = path
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    if let Some(path) = &normalized {
        let dir = PathBuf::from(path);
        fs::create_dir_all(&dir)
            .map_err(|e| format!("Failed to create app data backup directory: {e}"))?;
        set_backup_dir_setting(&state, Some(&dir))?;
    } else {
        set_backup_dir_setting(&state, None)?;
    }

    get_app_data_backup_dir(state).await
}

#[tauri::command]
pub async fn delete_app_data_backup(
    path: String,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    let backup_path = PathBuf::from(&path);
    if !backup_path.exists() {
        return Ok(false);
    }

    let extension = backup_path.extension().and_then(|ext| ext.to_str());
    if !matches!(extension, Some(LOCAL_BACKUP_EXT | PORTABLE_BACKUP_EXT)) {
        return Err("Refusing to delete unsupported backup file type.".to_string());
    }

    let backup_dir = configured_backup_dir(&state)?;
    let canonical_backup_dir = backup_dir
        .canonicalize()
        .map_err(|e| format!("Failed to inspect backup directory: {e}"))?;
    let parent = backup_path
        .parent()
        .ok_or_else(|| "Invalid backup path".to_string())?;
    let canonical_parent = parent
        .canonicalize()
        .map_err(|e| format!("Failed to inspect backup file path: {e}"))?;
    if canonical_parent != canonical_backup_dir {
        return Err("Refusing to delete backup outside the configured backup folder.".to_string());
    }

    fs::remove_file(&backup_path).map_err(|e| format!("Failed to delete backup: {e}"))?;
    Ok(true)
}

fn create_local_backup_at(
    state: &AppState,
    backup_path: &Path,
    backup_type: &str,
) -> Result<AppDataBackupInfo, String> {
    let (snapshot_path, manifest) = state.with_db(|db| {
        let temp_dir = state.data_dir().join(".backup-tmp");
        fs::create_dir_all(&temp_dir)
            .map_err(|e| format!("Failed to create temporary backup directory: {e}"))?;
        let snapshot_path = temp_dir.join(format!("snapshot-{}.db", Utc::now().timestamp_micros()));
        snapshot_database(db.connection(), &snapshot_path)?;
        let db_bytes =
            fs::read(&snapshot_path).map_err(|e| format!("Failed to read DB snapshot: {e}"))?;
        let manifest = BackupManifest {
            format: "tars-app-data-backup-v1".to_string(),
            backup_type: backup_type.to_string(),
            created_at: Utc::now().to_rfc3339(),
            tars_version: env!("CARGO_PKG_VERSION").to_string(),
            schema_version: db
                .connection()
                .pragma_query_value(None, "user_version", |row| row.get(0))
                .map_err(|e| format!("Failed to read schema version: {e}"))?,
            db_sha256: sha256_hex(&db_bytes),
        };
        Ok((snapshot_path, manifest))
    })?;

    write_backup_zip(backup_path, &snapshot_path, &manifest, None)?;
    let _ = fs::remove_file(&snapshot_path);
    backup_info_from_path(backup_path, backup_type, manifest.created_at)
}

fn snapshot_database(conn: &rusqlite::Connection, output_path: &Path) -> Result<(), String> {
    if output_path.exists() {
        fs::remove_file(output_path)
            .map_err(|e| format!("Failed to remove previous snapshot: {e}"))?;
    }
    let sql = format!("VACUUM main INTO '{}'", sql_quote_path(output_path));
    conn.execute_batch(&sql)
        .map_err(|e| format!("Failed to create SQLite backup snapshot: {e}"))
}

fn sql_quote_path(path: &Path) -> String {
    path.display().to_string().replace('\'', "''")
}

fn write_backup_zip(
    output_path: &Path,
    snapshot_path: &Path,
    manifest: &BackupManifest,
    portable_secrets: Option<&PortableSecrets>,
) -> Result<(), String> {
    let bytes = build_backup_zip(snapshot_path, manifest, portable_secrets)?;
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create backup directory: {e}"))?;
    }
    fs::write(output_path, bytes).map_err(|e| format!("Failed to write backup: {e}"))
}

fn build_backup_zip(
    snapshot_path: &Path,
    manifest: &BackupManifest,
    portable_secrets: Option<&PortableSecrets>,
) -> Result<Vec<u8>, String> {
    let mut cursor = Cursor::new(Vec::new());
    let mut zip = ZipWriter::new(&mut cursor);
    let options = FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    zip.start_file("manifest.json", options)
        .map_err(|e| format!("Failed to write backup manifest: {e}"))?;
    let manifest_json = serde_json::to_vec_pretty(manifest)
        .map_err(|e| format!("Failed to serialize backup manifest: {e}"))?;
    zip.write_all(&manifest_json)
        .map_err(|e| format!("Failed to write backup manifest: {e}"))?;

    zip.start_file("tars.db", options)
        .map_err(|e| format!("Failed to write database snapshot: {e}"))?;
    let db_bytes =
        fs::read(snapshot_path).map_err(|e| format!("Failed to read database snapshot: {e}"))?;
    zip.write_all(&db_bytes)
        .map_err(|e| format!("Failed to write database snapshot: {e}"))?;

    if let Some(secrets) = portable_secrets {
        zip.start_file("portable-secrets.json", options)
            .map_err(|e| format!("Failed to write portable secrets: {e}"))?;
        let secrets_json = serde_json::to_vec(secrets)
            .map_err(|e| format!("Failed to serialize portable secrets: {e}"))?;
        zip.write_all(&secrets_json)
            .map_err(|e| format!("Failed to write portable secrets: {e}"))?;
    }

    zip.finish()
        .map_err(|e| format!("Failed to finish backup archive: {e}"))?;
    drop(zip);
    Ok(cursor.into_inner())
}

fn write_portable_envelope(
    output_path: &Path,
    manifest: &BackupManifest,
    plaintext: &[u8],
    passphrase: &str,
) -> Result<(), String> {
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create backup directory: {e}"))?;
    }

    let mut salt = [0u8; 16];
    OsRng.fill_bytes(&mut salt);
    let key = derive_portable_key(passphrase, &salt)?;
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key));
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let ciphertext = cipher
        .encrypt(&nonce, plaintext)
        .map_err(|e| format!("Failed to encrypt portable backup: {e}"))?;

    let envelope = PortableEnvelope {
        format: "tars-portable-backup-v1".to_string(),
        backup_type: "portable".to_string(),
        created_at: manifest.created_at.clone(),
        tars_version: manifest.tars_version.clone(),
        kdf: PortableKdf {
            algorithm: "argon2id".to_string(),
            memory_kib: PORTABLE_KDF_MEMORY_KIB,
            iterations: PORTABLE_KDF_ITERATIONS,
            parallelism: PORTABLE_KDF_PARALLELISM,
        },
        salt_hex: hex::encode(salt),
        nonce_hex: hex::encode(nonce.as_slice()),
        ciphertext_hex: hex::encode(ciphertext),
    };
    let json = serde_json::to_vec_pretty(&envelope)
        .map_err(|e| format!("Failed to serialize portable backup: {e}"))?;
    fs::write(output_path, json).map_err(|e| format!("Failed to write portable backup: {e}"))
}

fn read_backup_payload(
    path: &Path,
    passphrase: Option<&str>,
) -> Result<(Vec<u8>, Option<PortableSecrets>), String> {
    match path.extension().and_then(|ext| ext.to_str()) {
        Some(LOCAL_BACKUP_EXT) => {
            read_zip_backup(&fs::read(path).map_err(|e| format!("Failed to read backup: {e}"))?)
        }
        Some(PORTABLE_BACKUP_EXT) => {
            let passphrase = passphrase
                .filter(|value| !value.is_empty())
                .ok_or_else(|| "Passphrase is required for portable backup restore.".to_string())?;
            let envelope: PortableEnvelope = serde_json::from_slice(
                &fs::read(path).map_err(|e| format!("Failed to read portable backup: {e}"))?,
            )
            .map_err(|e| format!("Failed to parse portable backup: {e}"))?;
            let salt = hex::decode(&envelope.salt_hex)
                .map_err(|e| format!("Invalid portable backup salt: {e}"))?;
            let nonce = hex::decode(&envelope.nonce_hex)
                .map_err(|e| format!("Invalid portable backup nonce: {e}"))?;
            let ciphertext = hex::decode(&envelope.ciphertext_hex)
                .map_err(|e| format!("Invalid portable backup ciphertext: {e}"))?;
            let key = derive_portable_key(passphrase, &salt)?;
            let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key));
            let plaintext = cipher
                .decrypt(Nonce::from_slice(&nonce), ciphertext.as_ref())
                .map_err(|_| {
                    "Failed to decrypt portable backup. Check the passphrase.".to_string()
                })?;
            read_zip_backup(&plaintext)
        }
        _ => Err("Unsupported backup file type.".to_string()),
    }
}

fn read_zip_backup(bytes: &[u8]) -> Result<(Vec<u8>, Option<PortableSecrets>), String> {
    let mut archive = ZipArchive::new(Cursor::new(bytes))
        .map_err(|e| format!("Failed to open backup archive: {e}"))?;

    let manifest: BackupManifest = {
        let mut manifest_file = archive
            .by_name("manifest.json")
            .map_err(|e| format!("Backup is missing manifest: {e}"))?;
        let mut manifest_json = String::new();
        manifest_file
            .read_to_string(&mut manifest_json)
            .map_err(|e| format!("Failed to read backup manifest: {e}"))?;
        serde_json::from_str(&manifest_json)
            .map_err(|e| format!("Failed to parse backup manifest: {e}"))?
    };

    let mut db_bytes = Vec::new();
    archive
        .by_name("tars.db")
        .map_err(|e| format!("Backup is missing database snapshot: {e}"))?
        .read_to_end(&mut db_bytes)
        .map_err(|e| format!("Failed to read database snapshot: {e}"))?;
    let actual_hash = sha256_hex(&db_bytes);
    if actual_hash != manifest.db_sha256 {
        return Err("Backup database checksum does not match manifest.".to_string());
    }

    let portable_secrets = match archive.by_name("portable-secrets.json") {
        Ok(mut file) => {
            let mut json = String::new();
            file.read_to_string(&mut json)
                .map_err(|e| format!("Failed to read portable secrets: {e}"))?;
            Some(
                serde_json::from_str(&json)
                    .map_err(|e| format!("Failed to parse portable secrets: {e}"))?,
            )
        }
        Err(_) => None,
    };

    Ok((db_bytes, portable_secrets))
}

fn derive_portable_key(passphrase: &str, salt: &[u8]) -> Result<[u8; 32], String> {
    let params = argon2::Params::new(
        PORTABLE_KDF_MEMORY_KIB,
        PORTABLE_KDF_ITERATIONS,
        PORTABLE_KDF_PARALLELISM,
        Some(32),
    )
    .map_err(|e| format!("Failed to configure backup key derivation: {e}"))?;
    let argon2 = Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params);
    let mut key = [0u8; 32];
    argon2
        .hash_password_into(passphrase.as_bytes(), salt, &mut key)
        .map_err(|e| format!("Failed to derive backup key: {e}"))?;
    Ok(key)
}

fn collect_portable_secrets(conn: &rusqlite::Connection) -> Result<PortableSecrets, String> {
    Ok(PortableSecrets {
        api_keys: collect_secret_rows(conn, "api_keys", "id", "encrypted_key", "nonce")?,
        project_secrets: collect_project_secret_rows(conn)?,
        developer_credentials: collect_secret_rows(
            conn,
            "developer_credentials",
            "id",
            "encrypted_secret",
            "nonce",
        )?,
    })
}

fn collect_secret_rows(
    conn: &rusqlite::Connection,
    table: &str,
    id_col: &str,
    encrypted_col: &str,
    nonce_col: &str,
) -> Result<Vec<PortableSecretRow>, String> {
    if !table_exists(conn, table)? {
        return Ok(Vec::new());
    }

    let sql = format!("SELECT {id_col}, {encrypted_col}, {nonce_col} FROM {table}");
    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| format!("Failed to read {table}: {e}"))?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .map_err(|e| format!("Failed to read {table}: {e}"))?;

    let mut secrets = Vec::new();
    for row in rows {
        let (id, encrypted, nonce) = row.map_err(|e| format!("Failed to read {table}: {e}"))?;
        let plaintext = crypto::decrypt(&nonce, &encrypted)
            .map_err(|e| format!("Failed to decrypt {table} row {id}: {e}"))?;
        secrets.push(PortableSecretRow { id, plaintext });
    }
    Ok(secrets)
}

fn collect_project_secret_rows(
    conn: &rusqlite::Connection,
) -> Result<Vec<PortableProjectSecretRow>, String> {
    if !table_exists(conn, "project_secrets")? {
        return Ok(Vec::new());
    }

    let mut stmt = conn
        .prepare("SELECT project_id, name, encrypted_data, nonce FROM project_secrets")
        .map_err(|e| format!("Failed to read project secrets: {e}"))?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })
        .map_err(|e| format!("Failed to read project secrets: {e}"))?;

    let mut secrets = Vec::new();
    for row in rows {
        let (project_id, name, encrypted, nonce) =
            row.map_err(|e| format!("Failed to read project secrets: {e}"))?;
        let plaintext_json = crypto::decrypt(&nonce, &encrypted)
            .map_err(|e| format!("Failed to decrypt project secret {project_id}/{name}: {e}"))?;
        secrets.push(PortableProjectSecretRow {
            project_id,
            name,
            plaintext_json,
        });
    }
    Ok(secrets)
}

fn apply_portable_secrets(
    conn: &rusqlite::Connection,
    secrets: &PortableSecrets,
) -> Result<(), String> {
    if table_exists(conn, "api_keys")? {
        for row in &secrets.api_keys {
            let (nonce, encrypted) = crypto::encrypt(&row.plaintext)
                .map_err(|e| format!("Failed to re-encrypt API key {}: {e}", row.id))?;
            conn.execute(
                "UPDATE api_keys SET encrypted_key = ?1, nonce = ?2 WHERE id = ?3",
                params![encrypted, nonce, row.id],
            )
            .map_err(|e| format!("Failed to update API key {}: {e}", row.id))?;
        }
    }

    if table_exists(conn, "developer_credentials")? {
        for row in &secrets.developer_credentials {
            let (nonce, encrypted) = crypto::encrypt(&row.plaintext).map_err(|e| {
                format!("Failed to re-encrypt developer credential {}: {e}", row.id)
            })?;
            conn.execute(
                "UPDATE developer_credentials SET encrypted_secret = ?1, nonce = ?2 WHERE id = ?3",
                params![encrypted, nonce, row.id],
            )
            .map_err(|e| format!("Failed to update developer credential {}: {e}", row.id))?;
        }
    }

    if table_exists(conn, "project_secrets")? {
        for row in &secrets.project_secrets {
            let (nonce, encrypted) = crypto::encrypt(&row.plaintext_json).map_err(|e| {
                format!(
                    "Failed to re-encrypt project secret {}/{}: {e}",
                    row.project_id, row.name
                )
            })?;
            conn.execute(
                "UPDATE project_secrets SET encrypted_data = ?1, nonce = ?2 WHERE project_id = ?3 AND name = ?4",
                params![encrypted, nonce, row.project_id, row.name],
            )
            .map_err(|e| {
                format!(
                    "Failed to update project secret {}/{}: {e}",
                    row.project_id, row.name
                )
            })?;
        }
    }

    Ok(())
}

fn verify_db_bytes(bytes: &[u8]) -> Result<(), String> {
    let temp_dir = tempfile::TempDir::new()
        .map_err(|e| format!("Failed to create restore verification directory: {e}"))?;
    let path = temp_dir.path().join("verify.db");
    fs::write(&path, bytes).map_err(|e| format!("Failed to stage restore database: {e}"))?;
    let conn = rusqlite::Connection::open(&path)
        .map_err(|e| format!("Restored file is not a valid SQLite database: {e}"))?;
    let result: String = conn
        .query_row("PRAGMA integrity_check", [], |row| row.get(0))
        .map_err(|e| format!("Failed to verify restored database: {e}"))?;
    if result == "ok" {
        Ok(())
    } else {
        Err(format!(
            "Restored database failed integrity check: {result}"
        ))
    }
}

fn table_exists(conn: &rusqlite::Connection, table: &str) -> Result<bool, String> {
    let exists: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
            params![table],
            |row| row.get(0),
        )
        .map_err(|e| format!("Failed to inspect database schema: {e}"))?;
    Ok(exists > 0)
}

fn backup_info_from_path(
    path: &Path,
    backup_type: &str,
    created_at: String,
) -> Result<AppDataBackupInfo, String> {
    let bytes = fs::read(path).map_err(|e| format!("Failed to read backup metadata: {e}"))?;
    let size_bytes = bytes.len() as u64;
    let sha256 = sha256_hex(&bytes);
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("backup")
        .to_string();
    Ok(AppDataBackupInfo {
        path: path.display().to_string(),
        file_name,
        backup_type: backup_type.to_string(),
        created_at,
        size_bytes,
        sha256,
    })
}

fn backup_created_at(path: &Path) -> Option<String> {
    match path.extension().and_then(|ext| ext.to_str()) {
        Some(LOCAL_BACKUP_EXT) => {
            let bytes = fs::read(path).ok()?;
            let mut archive = ZipArchive::new(Cursor::new(bytes)).ok()?;
            let mut manifest_file = archive.by_name("manifest.json").ok()?;
            let mut manifest_json = String::new();
            manifest_file.read_to_string(&mut manifest_json).ok()?;
            let manifest: BackupManifest = serde_json::from_str(&manifest_json).ok()?;
            Some(manifest.created_at)
        }
        Some(PORTABLE_BACKUP_EXT) => {
            let bytes = fs::read(path).ok()?;
            let envelope: PortableEnvelope = serde_json::from_slice(&bytes).ok()?;
            Some(envelope.created_at)
        }
        _ => None,
    }
}

fn default_backup_path(data_dir: &Path, extension: &str) -> PathBuf {
    default_backup_path_in_dir(&default_backup_dir(data_dir), extension)
}

fn default_backup_path_with_prefix(data_dir: &Path, prefix: &str, extension: &str) -> PathBuf {
    default_backup_path_with_prefix_in_dir(&default_backup_dir(data_dir), prefix, extension)
}

fn default_backup_path_in_dir(backup_dir: &Path, extension: &str) -> PathBuf {
    default_backup_path_with_prefix_in_dir(backup_dir, "tars-data", extension)
}

fn default_backup_path_with_prefix_in_dir(
    backup_dir: &Path,
    prefix: &str,
    extension: &str,
) -> PathBuf {
    let timestamp = Utc::now().format("%Y%m%d-%H%M%S");
    backup_dir.join(format!("{prefix}-{timestamp}.{extension}"))
}

fn default_backup_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("app-backups")
}

fn configured_backup_dir(state: &AppState) -> Result<PathBuf, String> {
    get_backup_dir_setting(state)
        .map(|path| path.unwrap_or_else(|| default_backup_dir(state.data_dir())))
}

fn get_backup_dir_setting(state: &AppState) -> Result<Option<PathBuf>, String> {
    state.with_db(|db| {
        ensure_app_settings_table(db.connection())?;
        let result: Option<String> = db
            .connection()
            .query_row(
                "SELECT value FROM app_settings WHERE key = ?1",
                params!["app_data_backup_dir"],
                |row| row.get(0),
            )
            .map(Some)
            .or_else(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(format!("Failed to read backup directory setting: {other}")),
            })?;
        Ok(result.map(PathBuf::from))
    })
}

fn set_backup_dir_setting(state: &AppState, path: Option<&Path>) -> Result<(), String> {
    state.with_db(|db| {
        ensure_app_settings_table(db.connection())?;
        if let Some(path) = path {
            db.connection()
                .execute(
                    r"
                    INSERT INTO app_settings (key, value, updated_at)
                    VALUES (?1, ?2, ?3)
                    ON CONFLICT(key) DO UPDATE SET
                        value = excluded.value,
                        updated_at = excluded.updated_at
                    ",
                    params![
                        "app_data_backup_dir",
                        path.display().to_string(),
                        Utc::now().to_rfc3339()
                    ],
                )
                .map_err(|e| format!("Failed to save backup directory setting: {e}"))?;
        } else {
            db.connection()
                .execute(
                    "DELETE FROM app_settings WHERE key = ?1",
                    params!["app_data_backup_dir"],
                )
                .map_err(|e| format!("Failed to clear backup directory setting: {e}"))?;
        }
        Ok(())
    })
}

fn ensure_app_settings_table(conn: &rusqlite::Connection) -> Result<(), String> {
    conn.execute_batch(
        r"
        CREATE TABLE IF NOT EXISTS app_settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        ",
    )
    .map_err(|e| format!("Failed to initialize app settings: {e}"))
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

fn remove_sqlite_sidecars(db_path: &Path) {
    let _ = fs::remove_file(db_path.with_extension("db-wal"));
    let _ = fs::remove_file(db_path.with_extension("db-shm"));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn portable_key_derivation_is_stable_for_same_salt() {
        let salt = [7u8; 16];
        let one = derive_portable_key("correct horse battery staple", &salt).unwrap();
        let two = derive_portable_key("correct horse battery staple", &salt).unwrap();
        assert_eq!(one, two);
    }

    #[test]
    fn sha256_hex_has_expected_length() {
        assert_eq!(sha256_hex(b"tars").len(), 64);
    }
}
