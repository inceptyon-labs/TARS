//! Cross-agent standalone skills library + deployment storage.
//!
//! `skill_sources` are directories TARS scans for standalone `SKILL.md`
//! bundles (the "library"). `skill_deployments` record where each catalog
//! skill has been materialized — per agent (Claude / Codex) and scope
//! (user / project) — so TARS can reconcile the on/off state of every
//! (skill × agent × scope) target. A deployment row's presence is the
//! "on" state; its absence is "off".

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use super::db::DatabaseError;

/// A registered library source directory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillSource {
    pub id: i64,
    pub path: String,
    pub label: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A materialized deployment of a catalog skill to one agent+scope target.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDeployment {
    pub id: i64,
    pub skill_name: String,
    pub source_path: String,
    /// Target agent: `"claude"` or `"codex"`.
    pub agent: String,
    /// Target scope: `"user"` or `"project"`.
    pub scope: String,
    /// `None` for user-scope targets; a `projects.id` UUID for project scope.
    pub project_id: Option<String>,
    /// Absolute path of the symlink (or copy) TARS created for this target.
    pub link_path: String,
    /// How the skill was materialized: `"symlink"` or `"copy"`.
    pub link_kind: String,
    /// Content hash captured at deploy time (used to detect drift for copies).
    pub sha256: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Input for recording a new deployment.
#[derive(Debug, Clone)]
pub struct SkillDeploymentInput {
    pub skill_name: String,
    pub source_path: String,
    pub agent: String,
    pub scope: String,
    pub project_id: Option<String>,
    pub link_path: String,
    pub link_kind: String,
    pub sha256: Option<String>,
}

fn parse_datetime(value: &str) -> Result<DateTime<Utc>, DatabaseError> {
    DateTime::parse_from_rfc3339(value)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| DatabaseError::Migration(format!("Bad skill library timestamp: {e}")))
}

fn row_to_source(row: &rusqlite::Row<'_>) -> Result<SkillSource, rusqlite::Error> {
    let created_at: String = row.get(3)?;
    let updated_at: String = row.get(4)?;
    Ok(SkillSource {
        id: row.get(0)?,
        path: row.get(1)?,
        label: row.get(2)?,
        created_at: parse_datetime(&created_at)
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?,
        updated_at: parse_datetime(&updated_at)
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?,
    })
}

fn row_to_deployment(row: &rusqlite::Row<'_>) -> Result<SkillDeployment, rusqlite::Error> {
    let created_at: String = row.get(9)?;
    let updated_at: String = row.get(10)?;
    Ok(SkillDeployment {
        id: row.get(0)?,
        skill_name: row.get(1)?,
        source_path: row.get(2)?,
        agent: row.get(3)?,
        scope: row.get(4)?,
        project_id: row.get(5)?,
        link_path: row.get(6)?,
        link_kind: row.get(7)?,
        sha256: row.get(8)?,
        created_at: parse_datetime(&created_at)
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?,
        updated_at: parse_datetime(&updated_at)
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?,
    })
}

/// Store for registered library source directories.
pub struct SkillSourceStore<'a> {
    conn: &'a Connection,
}

impl<'a> SkillSourceStore<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    pub fn create(&self, path: &str, label: Option<&str>) -> Result<SkillSource, DatabaseError> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            r"
            INSERT INTO skill_sources (path, label, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?3)
            ",
            params![path, label, now],
        )?;
        let id = self.conn.last_insert_rowid();
        self.get(id)?
            .ok_or_else(|| DatabaseError::Migration("skill source vanished after insert".into()))
    }

    pub fn list(&self) -> Result<Vec<SkillSource>, DatabaseError> {
        let mut stmt = self.conn.prepare(
            r"
            SELECT id, path, label, created_at, updated_at
            FROM skill_sources
            ORDER BY created_at ASC, id ASC
            ",
        )?;
        let rows = stmt.query_map([], row_to_source)?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(DatabaseError::from)
    }

    pub fn get(&self, id: i64) -> Result<Option<SkillSource>, DatabaseError> {
        let mut stmt = self.conn.prepare(
            r"
            SELECT id, path, label, created_at, updated_at
            FROM skill_sources WHERE id = ?1
            ",
        )?;
        stmt.query_row(params![id], row_to_source)
            .optional()
            .map_err(DatabaseError::from)
    }

    pub fn get_by_path(&self, path: &str) -> Result<Option<SkillSource>, DatabaseError> {
        let mut stmt = self.conn.prepare(
            r"
            SELECT id, path, label, created_at, updated_at
            FROM skill_sources WHERE path = ?1
            ",
        )?;
        stmt.query_row(params![path], row_to_source)
            .optional()
            .map_err(DatabaseError::from)
    }

    pub fn delete(&self, id: i64) -> Result<bool, DatabaseError> {
        let count = self
            .conn
            .execute("DELETE FROM skill_sources WHERE id = ?1", params![id])?;
        Ok(count > 0)
    }
}

const DEPLOYMENT_COLUMNS: &str = "id, skill_name, source_path, agent, scope, project_id, \
     link_path, link_kind, sha256, created_at, updated_at";

/// Store for cross-agent skill deployments.
pub struct SkillDeploymentStore<'a> {
    conn: &'a Connection,
}

impl<'a> SkillDeploymentStore<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    pub fn create(&self, input: &SkillDeploymentInput) -> Result<SkillDeployment, DatabaseError> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            r"
            INSERT INTO skill_deployments (
                skill_name, source_path, agent, scope, project_id,
                link_path, link_kind, sha256, created_at, updated_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)
            ",
            params![
                input.skill_name,
                input.source_path,
                input.agent,
                input.scope,
                input.project_id,
                input.link_path,
                input.link_kind,
                input.sha256,
                now,
            ],
        )?;
        let id = self.conn.last_insert_rowid();
        self.get(id)?.ok_or_else(|| {
            DatabaseError::Migration("skill deployment vanished after insert".into())
        })
    }

    pub fn list(&self) -> Result<Vec<SkillDeployment>, DatabaseError> {
        let sql = format!(
            "SELECT {DEPLOYMENT_COLUMNS} FROM skill_deployments \
             ORDER BY updated_at DESC, id DESC"
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map([], row_to_deployment)?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(DatabaseError::from)
    }

    /// All deployments for a given project (project-scope targets only).
    pub fn list_for_project(
        &self,
        project_id: &str,
    ) -> Result<Vec<SkillDeployment>, DatabaseError> {
        let sql = format!(
            "SELECT {DEPLOYMENT_COLUMNS} FROM skill_deployments \
             WHERE project_id = ?1 ORDER BY skill_name ASC, agent ASC"
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params![project_id], row_to_deployment)?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(DatabaseError::from)
    }

    /// All user-scope deployments (`project_id` IS NULL).
    pub fn list_user_scope(&self) -> Result<Vec<SkillDeployment>, DatabaseError> {
        let sql = format!(
            "SELECT {DEPLOYMENT_COLUMNS} FROM skill_deployments \
             WHERE project_id IS NULL ORDER BY skill_name ASC, agent ASC"
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map([], row_to_deployment)?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(DatabaseError::from)
    }

    pub fn get(&self, id: i64) -> Result<Option<SkillDeployment>, DatabaseError> {
        let sql = format!("SELECT {DEPLOYMENT_COLUMNS} FROM skill_deployments WHERE id = ?1");
        let mut stmt = self.conn.prepare(&sql)?;
        stmt.query_row(params![id], row_to_deployment)
            .optional()
            .map_err(DatabaseError::from)
    }

    /// Look up a single deployment by its natural target key. Folds NULL
    /// `project_id` to `''` so user-scope targets match the unique index.
    pub fn get_target(
        &self,
        agent: &str,
        scope: &str,
        project_id: Option<&str>,
        skill_name: &str,
    ) -> Result<Option<SkillDeployment>, DatabaseError> {
        let sql = format!(
            "SELECT {DEPLOYMENT_COLUMNS} FROM skill_deployments \
             WHERE agent = ?1 AND scope = ?2 AND IFNULL(project_id, '') = ?3 AND skill_name = ?4"
        );
        let mut stmt = self.conn.prepare(&sql)?;
        stmt.query_row(
            params![agent, scope, project_id.unwrap_or(""), skill_name],
            row_to_deployment,
        )
        .optional()
        .map_err(DatabaseError::from)
    }

    pub fn delete(&self, id: i64) -> Result<bool, DatabaseError> {
        let count = self
            .conn
            .execute("DELETE FROM skill_deployments WHERE id = ?1", params![id])?;
        Ok(count > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::migrations::run_migrations;

    fn conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        run_migrations(&conn).unwrap();
        conn
    }

    fn sample(agent: &str) -> SkillDeploymentInput {
        SkillDeploymentInput {
            skill_name: "deep-research".into(),
            source_path: "/lib/deep-research".into(),
            agent: agent.into(),
            scope: "user".into(),
            project_id: None,
            link_path: format!("/u/.{agent}/skills/deep-research"),
            link_kind: "symlink".into(),
            sha256: None,
        }
    }

    #[test]
    fn source_create_and_lookup() {
        let conn = conn();
        let store = SkillSourceStore::new(&conn);
        let src = store.create("/Users/j/skills-lib", Some("pasiv")).unwrap();
        assert_eq!(src.path, "/Users/j/skills-lib");
        assert_eq!(
            store
                .get_by_path("/Users/j/skills-lib")
                .unwrap()
                .unwrap()
                .id,
            src.id
        );
        assert_eq!(store.list().unwrap().len(), 1);
        assert!(store.delete(src.id).unwrap());
        assert!(store.list().unwrap().is_empty());
    }

    #[test]
    fn deployment_toggle_by_target() {
        let conn = conn();
        let store = SkillDeploymentStore::new(&conn);

        // Deploy to Claude user scope.
        let claude = store.create(&sample("claude")).unwrap();
        // The same skill on Codex is a distinct, independent target.
        store.create(&sample("codex")).unwrap();

        assert!(store
            .get_target("claude", "user", None, "deep-research")
            .unwrap()
            .is_some());
        assert!(store
            .get_target("codex", "user", None, "deep-research")
            .unwrap()
            .is_some());
        assert_eq!(store.list_user_scope().unwrap().len(), 2);

        // "Off" = delete the row.
        assert!(store.delete(claude.id).unwrap());
        assert!(store
            .get_target("claude", "user", None, "deep-research")
            .unwrap()
            .is_none());
        assert!(store
            .get_target("codex", "user", None, "deep-research")
            .unwrap()
            .is_some());
    }
}
