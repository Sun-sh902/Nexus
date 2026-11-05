//! Local persistence (SQLite). Holds Nexus intent and history only: Providers (non-secret),
//! Models, Profile revisions, Assignments, operation journal, backup index, Health history and
//! the ownership ledger. External configuration remains the observed truth; secrets live only in
//! the OS credential store, referenced here by opaque name.

use crate::core::{Instant, NexusError, NexusResult};
use crate::domain::AgentId;
use crate::models::*;
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use std::path::Path;

pub const SCHEMA_VERSION: i64 = 3;

const MIGRATIONS: &[(i64, &str)] = &[
    (
        1,
        r#"
CREATE TABLE providers (
  id TEXT PRIMARY KEY, display_name TEXT NOT NULL, kind TEXT NOT NULL, base_url TEXT,
  auth_mode TEXT NOT NULL, secret_ref TEXT, env_var TEXT, revision INTEGER NOT NULL,
  created_at TEXT NOT NULL, updated_at TEXT NOT NULL);
CREATE TABLE models (
  id TEXT PRIMARY KEY, provider_id TEXT NOT NULL, identifier TEXT NOT NULL, display_name TEXT,
  source TEXT NOT NULL, source_detail TEXT, capabilities_json TEXT NOT NULL DEFAULT '[]',
  created_at TEXT NOT NULL, UNIQUE(provider_id, identifier));
CREATE TABLE model_profiles (
  id TEXT NOT NULL, revision INTEGER NOT NULL, name TEXT NOT NULL, model_id TEXT NOT NULL,
  reasoning TEXT, created_at TEXT NOT NULL, PRIMARY KEY (id, revision));
CREATE TABLE assignments (
  agent_id TEXT PRIMARY KEY, mode TEXT NOT NULL, model_id TEXT, profile_id TEXT,
  desired_revision INTEGER, applied_identifier TEXT, applied_profile_revision INTEGER,
  applied_provider_revision INTEGER, applied_at TEXT, applied_location TEXT,
  applied_fingerprint TEXT, updated_at TEXT NOT NULL);
CREATE TABLE operations (
  id TEXT PRIMARY KEY, action TEXT NOT NULL, agent_id TEXT, status TEXT NOT NULL,
  plan_digest TEXT NOT NULL, summary_json TEXT NOT NULL, started_at TEXT NOT NULL,
  finished_at TEXT, error_code TEXT);
CREATE TABLE operation_steps (
  operation_id TEXT NOT NULL, seq INTEGER NOT NULL, step TEXT NOT NULL, at TEXT NOT NULL,
  PRIMARY KEY (operation_id, seq));
CREATE TABLE backups (
  id TEXT PRIMARY KEY, operation_id TEXT NOT NULL, agent_id TEXT, target_id TEXT NOT NULL,
  target_path TEXT NOT NULL, target_display TEXT NOT NULL, sha256 TEXT NOT NULL,
  size INTEGER NOT NULL, written_fingerprint TEXT, created_at TEXT NOT NULL);
CREATE TABLE health_history (
  id INTEGER PRIMARY KEY AUTOINCREMENT, agent_id TEXT NOT NULL, readiness TEXT NOT NULL,
  rule_set TEXT NOT NULL, issue_codes TEXT NOT NULL, evaluated_at TEXT NOT NULL);
CREATE TABLE managed_extensions (
  agent_id TEXT NOT NULL, name TEXT NOT NULL, origin TEXT, created_at TEXT NOT NULL,
  operation_id TEXT NOT NULL, PRIMARY KEY (agent_id, name));
CREATE TABLE nexus_installations (
  agent_id TEXT NOT NULL, recipe_id TEXT NOT NULL, version TEXT, installation_id TEXT,
  operation_id TEXT NOT NULL, status TEXT NOT NULL, at TEXT NOT NULL,
  PRIMARY KEY (agent_id, recipe_id));
"#,
    ),
    (
        2,
        r#"
CREATE TABLE workspaces (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  path TEXT NOT NULL UNIQUE,
  primary_agent TEXT NOT NULL,
  fallback_agent TEXT,
  model_id TEXT,
  profile_id TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  last_opened_at TEXT
);
"#,
    ),
    (
        3,
        r#"
CREATE TABLE agent_sessions (
  id TEXT PRIMARY KEY,
  agent_id TEXT NOT NULL,
  workspace_id TEXT,
  external_session_id TEXT NOT NULL,
  title TEXT NOT NULL,
  project_path TEXT NOT NULL,
  started_at TEXT NOT NULL,
  last_active_at TEXT NOT NULL,
  message_count INTEGER NOT NULL DEFAULT 0,
  input_units INTEGER NOT NULL DEFAULT 0,
  output_units INTEGER NOT NULL DEFAULT 0,
  estimated_cost_cents INTEGER NOT NULL DEFAULT 0,
  cost_provenance TEXT NOT NULL,
  model_identifier TEXT,
  created_at TEXT NOT NULL,
  UNIQUE(agent_id, external_session_id)
);
CREATE INDEX idx_sessions_workspace ON agent_sessions(workspace_id);
CREATE INDEX idx_sessions_agent ON agent_sessions(agent_id);
CREATE INDEX idx_sessions_last_active ON agent_sessions(last_active_at);
"#,
    ),
];

pub struct Store {
    conn: Connection,
}

fn db_err(_: rusqlite::Error) -> NexusError {
    NexusError::new("store.io_error")
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationRecord {
    pub id: String,
    pub action: String,
    pub agent_id: Option<String>,
    pub status: String,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub error_code: Option<String>,
    pub summary: serde_json::Value,
    pub steps: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupRecord {
    pub id: String,
    pub operation_id: String,
    pub agent_id: Option<String>,
    pub target_id: String,
    pub target_display: String,
    pub sha256: String,
    pub size: i64,
    pub created_at: String,
    #[serde(skip)]
    pub target_path: String,
    /// Fingerprint Nexus wrote after this backup (lets a later restore detect intervening edits).
    #[serde(skip)]
    #[allow(dead_code)]
    pub written_fingerprint: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthHistoryRow {
    pub agent_id: String,
    pub readiness: String,
    pub issue_codes: Vec<String>,
    pub evaluated_at: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LedgerEntry {
    pub agent_id: String,
    pub recipe_id: String,
    pub version: Option<String>,
    pub installation_id: Option<String>,
    pub operation_id: String,
    pub status: String,
    pub at: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StoreCounts {
    pub operation_count: i64,
    pub unresolved_count: i64,
    pub health_row_count: i64,
    pub backup_count: i64,
    pub workspace_count: i64,
    pub session_count: i64,
}

#[derive(Clone, Debug, Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SessionRecord {
    pub id: String,
    pub agent_id: String,
    pub workspace_id: Option<String>,
    pub external_session_id: String,
    pub title: String,
    pub project_path: String,
    pub started_at: String,
    pub last_active_at: String,
    pub message_count: u32,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub estimated_cost_cents: u64,
    pub cost_provenance: String,
    pub model_identifier: Option<String>,
    pub created_at: String,
}

#[derive(Clone, Debug, Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceRecord {
    pub id: String,
    pub name: String,
    pub path: String,
    pub primary_agent: String,
    pub fallback_agent: Option<String>,
    pub model_id: Option<String>,
    pub profile_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub last_opened_at: Option<String>,
}

#[derive(Clone, Debug)]
pub struct PruneSummary {
    pub pruned_operations: usize,
    pub pruned_health_rows: usize,
    pub pruned_backup_ids: Vec<String>,
}

impl Store {
    pub fn open(path: &Path) -> NexusResult<Store> {
        let conn = Connection::open(path).map_err(|_| NexusError::new("store.open_failed"))?;
        Self::init(conn)
    }
    #[cfg(test)]
    pub fn open_in_memory() -> NexusResult<Store> {
        Self::init(Connection::open_in_memory().map_err(db_err)?)
    }
    fn init(mut conn: Connection) -> NexusResult<Store> {
        conn.pragma_update(None, "journal_mode", "WAL")
            .map_err(db_err)?;
        conn.pragma_update(None, "foreign_keys", "ON")
            .map_err(db_err)?;
        conn.pragma_update(None, "synchronous", "FULL")
            .map_err(db_err)?;
        let current: i64 = conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .map_err(db_err)?;
        if current > SCHEMA_VERSION {
            // Written by a newer Nexus: never downgrade or write into an unknown schema.
            return Err(NexusError::new("store.newer_schema").with("found", current.to_string()));
        }
        for (version, sql) in MIGRATIONS.iter().filter(|(v, _)| *v > current) {
            let tx = conn.transaction().map_err(db_err)?;
            tx.execute_batch(sql).map_err(|_| {
                NexusError::new("store.migration_failed").with("version", version.to_string())
            })?;
            tx.pragma_update(None, "user_version", version)
                .map_err(db_err)?;
            tx.commit().map_err(db_err)?;
        }
        Ok(Store { conn })
    }

    // ---- Providers -------------------------------------------------------------------------

    pub fn providers(&self) -> NexusResult<Vec<ProviderProfile>> {
        let mut stmt = self.conn.prepare("SELECT id, display_name, kind, base_url, auth_mode, secret_ref, env_var, revision FROM providers ORDER BY created_at").map_err(db_err)?;
        let rows = stmt
            .query_map([], |r| {
                Ok(ProviderProfile {
                    id: r.get(0)?,
                    display_name: r.get(1)?,
                    kind: ProviderKind::parse(&r.get::<_, String>(2)?)
                        .unwrap_or(ProviderKind::OpenaiCompatible),
                    base_url: r.get(3)?,
                    auth_mode: match r.get::<_, String>(4)?.as_str() {
                        "nexusSecret" => AuthMode::NexusSecret,
                        "environment" => AuthMode::Environment,
                        _ => AuthMode::AgentLogin,
                    },
                    secret_ref: r.get(5)?,
                    secret_present: None,
                    env_var: r.get(6)?,
                    revision: r.get(7)?,
                    builtin: false,
                    connection: "notTested",
                })
            })
            .map_err(db_err)?;
        rows.collect::<Result<_, _>>().map_err(db_err)
    }

    pub fn insert_provider(&self, p: &ProviderProfile, now: Instant) -> NexusResult<()> {
        let auth = match p.auth_mode {
            AuthMode::AgentLogin => "agentLogin",
            AuthMode::NexusSecret => "nexusSecret",
            AuthMode::Environment => "environment",
        };
        self.conn
            .execute(
                "INSERT INTO providers (id, display_name, kind, base_url, auth_mode, secret_ref, env_var, revision, created_at, updated_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?9)",
                params![p.id, p.display_name, p.kind.as_str(), p.base_url, auth, p.secret_ref, p.env_var, p.revision, now.to_rfc3339()],
            )
            .map_err(db_err)?;
        Ok(())
    }

    /// Deleting a Provider still referenced by a Model is refused (Architecture §9).
    pub fn delete_provider(&self, id: &str) -> NexusResult<Option<String>> {
        let refs: i64 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM models WHERE provider_id = ?1",
                [id],
                |r| r.get(0),
            )
            .map_err(db_err)?;
        if refs > 0 {
            return Err(NexusError::new("provider.in_use").with("models", refs.to_string()));
        }
        let secret: Option<Option<String>> = self
            .conn
            .query_row(
                "SELECT secret_ref FROM providers WHERE id = ?1",
                [id],
                |r| r.get(0),
            )
            .optional()
            .map_err(db_err)?;
        let Some(secret) = secret else {
            return Err(NexusError::new("provider.not_found"));
        };
        self.conn
            .execute("DELETE FROM providers WHERE id = ?1", [id])
            .map_err(db_err)?;
        Ok(secret)
    }

    // ---- Models ----------------------------------------------------------------------------

    pub fn models(&self) -> NexusResult<Vec<ModelDefinition>> {
        let mut stmt = self.conn.prepare("SELECT id, provider_id, identifier, display_name, source, source_detail, capabilities_json FROM models ORDER BY created_at").map_err(db_err)?;
        let rows = stmt
            .query_map([], |r| {
                let caps: String = r.get(6)?;
                let claims: Vec<(String, String, String)> =
                    serde_json::from_str(&caps).unwrap_or_default();
                Ok(ModelDefinition {
                    id: r.get(0)?,
                    provider_id: r.get(1)?,
                    identifier: r.get(2)?,
                    display_name: r.get(3)?,
                    source: r.get(4)?,
                    source_detail: r.get(5)?,
                    capabilities: claims
                        .into_iter()
                        .map(|(key, state, source)| CapabilityClaim { key, state, source })
                        .collect(),
                })
            })
            .map_err(db_err)?;
        rows.collect::<Result<_, _>>().map_err(db_err)
    }

    pub fn insert_model(&self, m: &ModelDefinition, now: Instant) -> NexusResult<()> {
        let caps: Vec<(String, String, String)> = m
            .capabilities
            .iter()
            .map(|c| (c.key.clone(), c.state.clone(), c.source.clone()))
            .collect();
        self.conn
            .execute(
                "INSERT INTO models (id, provider_id, identifier, display_name, source, source_detail, capabilities_json, created_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
                params![m.id, m.provider_id, m.identifier, m.display_name, m.source, m.source_detail, serde_json::to_string(&caps).unwrap_or_default(), now.to_rfc3339()],
            )
            .map_err(|e| match e {
                rusqlite::Error::SqliteFailure(f, _) if f.code == rusqlite::ErrorCode::ConstraintViolation => NexusError::new("model.duplicate"),
                other => db_err(other),
            })?;
        Ok(())
    }

    pub fn delete_model(&self, id: &str) -> NexusResult<()> {
        let refs: i64 = self
            .conn
            .query_row("SELECT (SELECT COUNT(*) FROM model_profiles WHERE model_id = ?1) + (SELECT COUNT(*) FROM assignments WHERE model_id = ?1)", [id], |r| r.get(0))
            .map_err(db_err)?;
        if refs > 0 {
            return Err(NexusError::new("model.in_use"));
        }
        if self
            .conn
            .execute("DELETE FROM models WHERE id = ?1", [id])
            .map_err(db_err)?
            == 0
        {
            return Err(NexusError::new("model.not_found"));
        }
        Ok(())
    }

    // ---- Profiles (revisioned) -------------------------------------------------------------

    pub fn profiles(&self) -> NexusResult<Vec<ModelProfile>> {
        let mut stmt = self
            .conn
            .prepare("SELECT p.id, p.name, p.revision, p.model_id, p.reasoning FROM model_profiles p JOIN (SELECT id, MAX(revision) r FROM model_profiles GROUP BY id) l ON p.id = l.id AND p.revision = l.r ORDER BY p.created_at")
            .map_err(db_err)?;
        let rows = stmt
            .query_map([], |r| {
                Ok(ModelProfile {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    revision: r.get(2)?,
                    model_id: r.get(3)?,
                    reasoning: r.get(4)?,
                })
            })
            .map_err(db_err)?;
        rows.collect::<Result<_, _>>().map_err(db_err)
    }

    #[cfg(test)]
    pub fn profile_revision(&self, id: &str, revision: i64) -> NexusResult<Option<ModelProfile>> {
        self.conn
            .query_row("SELECT id, name, revision, model_id, reasoning FROM model_profiles WHERE id = ?1 AND revision = ?2", params![id, revision], |r| {
                Ok(ModelProfile { id: r.get(0)?, name: r.get(1)?, revision: r.get(2)?, model_id: r.get(3)?, reasoning: r.get(4)? })
            })
            .optional()
            .map_err(db_err)
    }

    /// Editing a Profile creates a new revision; applied revisions stay pinned (Architecture §9).
    pub fn save_profile(
        &self,
        id: &str,
        name: &str,
        model_id: &str,
        reasoning: Option<&str>,
        now: Instant,
    ) -> NexusResult<i64> {
        let next: i64 = self
            .conn
            .query_row(
                "SELECT COALESCE(MAX(revision), 0) + 1 FROM model_profiles WHERE id = ?1",
                [id],
                |r| r.get(0),
            )
            .map_err(db_err)?;
        self.conn
            .execute("INSERT INTO model_profiles (id, revision, name, model_id, reasoning, created_at) VALUES (?1,?2,?3,?4,?5,?6)", params![id, next, name, model_id, reasoning, now.to_rfc3339()])
            .map_err(db_err)?;
        Ok(next)
    }

    /// Deleting a Profile still referenced by an Agent assignment is refused (Architecture §9).
    pub fn delete_profile(&self, id: &str) -> NexusResult<()> {
        let refs: i64 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM assignments WHERE profile_id = ?1",
                [id],
                |r| r.get(0),
            )
            .map_err(db_err)?;
        if refs > 0 {
            return Err(NexusError::new("profile.in_use").with("agents", refs.to_string()));
        }
        let deleted = self
            .conn
            .execute("DELETE FROM model_profiles WHERE id = ?1", [id])
            .map_err(db_err)?;
        if deleted == 0 {
            return Err(NexusError::new("profile.not_found"));
        }
        Ok(())
    }

    // ---- Assignments -----------------------------------------------------------------------

    pub fn assignments(&self) -> NexusResult<Vec<AgentModelAssignment>> {
        let mut stmt = self
            .conn
            .prepare("SELECT agent_id, mode, model_id, profile_id, desired_revision, applied_identifier, applied_profile_revision, applied_provider_revision, applied_at, applied_location, applied_fingerprint, updated_at FROM assignments")
            .map_err(db_err)?;
        let rows = stmt
            .query_map([], |r| {
                let applied_identifier: Option<String> = r.get(5)?;
                Ok((
                    r.get::<_, String>(0)?,
                    AgentModelAssignment {
                        agent_id: AgentId::Codex,
                        mode: match r.get::<_, String>(1)?.as_str() {
                            "direct" => AssignmentMode::Direct,
                            "profile" => AssignmentMode::Profile,
                            _ => AssignmentMode::Native,
                        },
                        model_id: r.get(2)?,
                        profile_id: r.get(3)?,
                        desired_revision: r.get(4)?,
                        applied: match applied_identifier {
                            Some(identifier) => Some(AppliedRecord {
                                identifier,
                                profile_revision: r.get(6)?,
                                provider_revision: r.get(7)?,
                                applied_at: r.get::<_, Option<String>>(8)?.unwrap_or_default(),
                                location: r.get::<_, Option<String>>(9)?.unwrap_or_default(),
                                fingerprint: r.get::<_, Option<String>>(10)?.unwrap_or_default(),
                            }),
                            None => None,
                        },
                        updated_at: r.get(11)?,
                    },
                ))
            })
            .map_err(db_err)?;
        let mut out = Vec::new();
        for row in rows {
            let (agent, mut a) = row.map_err(db_err)?;
            if let Some(id) = AgentId::parse(&agent) {
                a.agent_id = id;
                out.push(a);
            }
        }
        Ok(out)
    }

    pub fn assignment(&self, agent: AgentId) -> NexusResult<Option<AgentModelAssignment>> {
        Ok(self
            .assignments()?
            .into_iter()
            .find(|a| a.agent_id == agent))
    }

    /// Records desired intent only; the applied record is untouched until a verified apply.
    pub fn set_desired(
        &self,
        agent: AgentId,
        mode: AssignmentMode,
        model_id: Option<&str>,
        profile_id: Option<&str>,
        desired_revision: Option<i64>,
        now: Instant,
    ) -> NexusResult<()> {
        let mode_s = match mode {
            AssignmentMode::Native => "native",
            AssignmentMode::Direct => "direct",
            AssignmentMode::Profile => "profile",
        };
        self.conn
            .execute(
                "INSERT INTO assignments (agent_id, mode, model_id, profile_id, desired_revision, updated_at) VALUES (?1,?2,?3,?4,?5,?6)
                 ON CONFLICT(agent_id) DO UPDATE SET mode=excluded.mode, model_id=excluded.model_id, profile_id=excluded.profile_id, desired_revision=excluded.desired_revision, updated_at=excluded.updated_at",
                params![agent.as_str(), mode_s, model_id, profile_id, desired_revision, now.to_rfc3339()],
            )
            .map_err(db_err)?;
        Ok(())
    }

    pub fn set_applied(&self, agent: AgentId, rec: &AppliedRecord) -> NexusResult<()> {
        let n = self
            .conn
            .execute(
                "UPDATE assignments SET applied_identifier=?2, applied_profile_revision=?3, applied_provider_revision=?4, applied_at=?5, applied_location=?6, applied_fingerprint=?7 WHERE agent_id=?1",
                params![agent.as_str(), rec.identifier, rec.profile_revision, rec.provider_revision, rec.applied_at, rec.location, rec.fingerprint],
            )
            .map_err(db_err)?;
        if n == 0 {
            return Err(NexusError::new("assignment.not_found"));
        }
        Ok(())
    }

    pub fn clear_assignment(&self, agent: AgentId) -> NexusResult<()> {
        self.conn
            .execute(
                "DELETE FROM assignments WHERE agent_id = ?1",
                [agent.as_str()],
            )
            .map_err(db_err)?;
        Ok(())
    }

    // ---- Operation journal -----------------------------------------------------------------

    pub fn begin_operation(
        &self,
        id: &str,
        action: &str,
        agent: Option<AgentId>,
        digest: &str,
        summary: &serde_json::Value,
        now: Instant,
    ) -> NexusResult<()> {
        self.conn
            .execute(
                "INSERT INTO operations (id, action, agent_id, status, plan_digest, summary_json, started_at) VALUES (?1,?2,?3,'executing',?4,?5,?6)",
                params![id, action, agent.map(|a| a.as_str()), digest, summary.to_string(), now.to_rfc3339()],
            )
            .map_err(|e| match e {
                rusqlite::Error::SqliteFailure(f, _) if f.code == rusqlite::ErrorCode::ConstraintViolation => NexusError::new("operation.replayed"),
                other => db_err(other),
            })?;
        Ok(())
    }

    /// Journals a step *before* the next side effect so a crash leaves an exact record.
    pub fn step(&self, op: &str, step: &str, now: Instant) -> NexusResult<()> {
        self.conn
            .execute("INSERT INTO operation_steps (operation_id, seq, step, at) VALUES (?1, (SELECT COALESCE(MAX(seq),0)+1 FROM operation_steps WHERE operation_id=?1), ?2, ?3)", params![op, step, now.to_rfc3339()])
            .map_err(db_err)?;
        Ok(())
    }

    pub fn finish_operation(
        &self,
        op: &str,
        status: &str,
        error: Option<&str>,
        now: Instant,
    ) -> NexusResult<()> {
        self.conn
            .execute(
                "UPDATE operations SET status=?2, error_code=?3, finished_at=?4 WHERE id=?1",
                params![op, status, error, now.to_rfc3339()],
            )
            .map_err(db_err)?;
        Ok(())
    }

    pub fn operations(&self, limit: i64) -> NexusResult<Vec<OperationRecord>> {
        let mut stmt = self.conn.prepare("SELECT id, action, agent_id, status, started_at, finished_at, error_code, summary_json FROM operations ORDER BY started_at DESC, rowid DESC LIMIT ?1").map_err(db_err)?;
        let rows = stmt
            .query_map([limit], |r| {
                Ok(OperationRecord {
                    id: r.get(0)?,
                    action: r.get(1)?,
                    agent_id: r.get(2)?,
                    status: r.get(3)?,
                    started_at: r.get(4)?,
                    finished_at: r.get(5)?,
                    error_code: r.get(6)?,
                    summary: serde_json::from_str(&r.get::<_, String>(7)?).unwrap_or_default(),
                    steps: Vec::new(),
                })
            })
            .map_err(db_err)?;
        let mut ops: Vec<OperationRecord> = rows.collect::<Result<_, _>>().map_err(db_err)?;
        for op in ops.iter_mut() {
            let mut s = self
                .conn
                .prepare("SELECT step FROM operation_steps WHERE operation_id = ?1 ORDER BY seq")
                .map_err(db_err)?;
            op.steps = s
                .query_map([&op.id], |r| r.get(0))
                .map_err(db_err)?
                .collect::<Result<_, _>>()
                .map_err(db_err)?;
        }
        Ok(ops)
    }

    /// On startup, any operation still `executing` was interrupted (crash, kill, power loss).
    /// It is marked `interrupted`, never replayed automatically.
    pub fn reconcile_interrupted(&self, now: Instant) -> NexusResult<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id FROM operations WHERE status = 'executing'")
            .map_err(db_err)?;
        let ids: Vec<String> = stmt
            .query_map([], |r| r.get(0))
            .map_err(db_err)?
            .collect::<Result<_, _>>()
            .map_err(db_err)?;
        for id in &ids {
            self.finish_operation(id, "interrupted", Some("operation.interrupted"), now)?;
        }
        Ok(ids)
    }

    // ---- Backups ---------------------------------------------------------------------------

    #[allow(clippy::too_many_arguments)]
    pub fn insert_backup(
        &self,
        id: &str,
        op: &str,
        agent: Option<AgentId>,
        target_id: &str,
        target_path: &str,
        display: &str,
        sha: &str,
        size: usize,
        now: Instant,
    ) -> NexusResult<()> {
        self.conn
            .execute(
                "INSERT INTO backups (id, operation_id, agent_id, target_id, target_path, target_display, sha256, size, created_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
                params![id, op, agent.map(|a| a.as_str()), target_id, target_path, display, sha, size as i64, now.to_rfc3339()],
            )
            .map_err(db_err)?;
        Ok(())
    }

    pub fn set_backup_written(&self, id: &str, fingerprint: &str) -> NexusResult<()> {
        self.conn
            .execute(
                "UPDATE backups SET written_fingerprint = ?2 WHERE id = ?1",
                params![id, fingerprint],
            )
            .map_err(db_err)?;
        Ok(())
    }

    pub fn backups(&self, agent: Option<AgentId>) -> NexusResult<Vec<BackupRecord>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, operation_id, agent_id, target_id, target_display, sha256, size, created_at, target_path, written_fingerprint FROM backups WHERE (?1 IS NULL OR agent_id = ?1) ORDER BY created_at DESC, rowid DESC LIMIT 50")
            .map_err(db_err)?;
        let rows = stmt
            .query_map([agent.map(|a| a.as_str())], |r| {
                Ok(BackupRecord {
                    id: r.get(0)?,
                    operation_id: r.get(1)?,
                    agent_id: r.get(2)?,
                    target_id: r.get(3)?,
                    target_display: r.get(4)?,
                    sha256: r.get(5)?,
                    size: r.get(6)?,
                    created_at: r.get(7)?,
                    target_path: r.get(8)?,
                    written_fingerprint: r.get(9)?,
                })
            })
            .map_err(db_err)?;
        rows.collect::<Result<_, _>>().map_err(db_err)
    }

    pub fn backup(&self, id: &str) -> NexusResult<BackupRecord> {
        self.backups(None)?
            .into_iter()
            .find(|b| b.id == id)
            .ok_or_else(|| NexusError::new("backup.missing"))
    }

    // ---- Health history --------------------------------------------------------------------

    pub fn record_health(
        &self,
        agent: AgentId,
        readiness: &str,
        rule_set: &str,
        codes: &[String],
        at: Instant,
    ) -> NexusResult<()> {
        let tx = self.conn.unchecked_transaction().map_err(db_err)?;
        tx.execute("INSERT INTO health_history (agent_id, readiness, rule_set, issue_codes, evaluated_at) VALUES (?1,?2,?3,?4,?5)", params![agent.as_str(), readiness, rule_set, codes.join(","), at.to_rfc3339()])
            .map_err(db_err)?;
        // Bounded local history (Security §6 retention): keep the latest 200 rows per Agent.
        tx.execute("DELETE FROM health_history WHERE agent_id = ?1 AND id NOT IN (SELECT id FROM health_history WHERE agent_id = ?1 ORDER BY id DESC LIMIT 200)", [agent.as_str()]).map_err(db_err)?;
        tx.commit().map_err(db_err)
    }

    pub fn health_history(&self, agent: AgentId, limit: i64) -> NexusResult<Vec<HealthHistoryRow>> {
        let mut stmt = self.conn.prepare("SELECT agent_id, readiness, issue_codes, evaluated_at FROM health_history WHERE agent_id = ?1 ORDER BY id DESC LIMIT ?2").map_err(db_err)?;
        let rows = stmt
            .query_map(params![agent.as_str(), limit], |r| {
                let codes: String = r.get(2)?;
                Ok(HealthHistoryRow {
                    agent_id: r.get(0)?,
                    readiness: r.get(1)?,
                    issue_codes: codes
                        .split(',')
                        .filter(|s| !s.is_empty())
                        .map(String::from)
                        .collect(),
                    evaluated_at: r.get(3)?,
                })
            })
            .map_err(db_err)?;
        rows.collect::<Result<_, _>>().map_err(db_err)
    }

    // ---- Nexus-managed extensions and installations ---------------------------------------

    pub fn add_managed_extension(
        &self,
        agent: AgentId,
        name: &str,
        origin: Option<&str>,
        op: &str,
        now: Instant,
    ) -> NexusResult<()> {
        self.conn
            .execute("INSERT OR REPLACE INTO managed_extensions (agent_id, name, origin, created_at, operation_id) VALUES (?1,?2,?3,?4,?5)", params![agent.as_str(), name, origin, now.to_rfc3339(), op])
            .map_err(db_err)?;
        Ok(())
    }
    pub fn remove_managed_extension(&self, agent: AgentId, name: &str) -> NexusResult<()> {
        self.conn
            .execute(
                "DELETE FROM managed_extensions WHERE agent_id = ?1 AND name = ?2",
                params![agent.as_str(), name],
            )
            .map_err(db_err)?;
        Ok(())
    }
    pub fn managed_extensions(&self, agent: AgentId) -> NexusResult<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT name FROM managed_extensions WHERE agent_id = ?1")
            .map_err(db_err)?;
        let rows = stmt
            .query_map([agent.as_str()], |r| r.get(0))
            .map_err(db_err)?;
        rows.collect::<Result<_, _>>().map_err(db_err)
    }

    pub fn record_installation(&self, e: &LedgerEntry) -> NexusResult<()> {
        self.conn
            .execute(
                "INSERT OR REPLACE INTO nexus_installations (agent_id, recipe_id, version, installation_id, operation_id, status, at) VALUES (?1,?2,?3,?4,?5,?6,?7)",
                params![e.agent_id, e.recipe_id, e.version, e.installation_id, e.operation_id, e.status, e.at],
            )
            .map_err(db_err)?;
        Ok(())
    }
    pub fn remove_installation(&self, agent: AgentId, recipe: &str) -> NexusResult<()> {
        self.conn
            .execute(
                "DELETE FROM nexus_installations WHERE agent_id = ?1 AND recipe_id = ?2",
                params![agent.as_str(), recipe],
            )
            .map_err(db_err)?;
        Ok(())
    }
    pub fn ledger(&self) -> NexusResult<Vec<LedgerEntry>> {
        let mut stmt = self.conn.prepare("SELECT agent_id, recipe_id, version, installation_id, operation_id, status, at FROM nexus_installations").map_err(db_err)?;
        let rows = stmt
            .query_map([], |r| {
                Ok(LedgerEntry {
                    agent_id: r.get(0)?,
                    recipe_id: r.get(1)?,
                    version: r.get(2)?,
                    installation_id: r.get(3)?,
                    operation_id: r.get(4)?,
                    status: r.get(5)?,
                    at: r.get(6)?,
                })
            })
            .map_err(db_err)?;
        rows.collect::<Result<_, _>>().map_err(db_err)
    }

    // ---- Local storage & retention (Security §6) -------------------------------------------

    pub fn counts(&self) -> NexusResult<StoreCounts> {
        let operation_count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM operations", [], |r| r.get(0))
            .map_err(db_err)?;
        let unresolved_count: i64 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM operations WHERE status IN ('executing', 'interrupted')",
                [],
                |r| r.get(0),
            )
            .map_err(db_err)?;
        let health_row_count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM health_history", [], |r| r.get(0))
            .map_err(db_err)?;
        let backup_count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM backups", [], |r| r.get(0))
            .map_err(db_err)?;
        let workspace_count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM workspaces", [], |r| r.get(0))
            .map_err(db_err)?;
        let session_count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM agent_sessions", [], |r| r.get(0))
            .unwrap_or(0);
        Ok(StoreCounts {
            operation_count,
            unresolved_count,
            health_row_count,
            backup_count,
            workspace_count,
            session_count,
        })
    }

    // ---- Workspaces (Phase 8) ---------------------------------------------------------------

    pub fn insert_workspace(&self, w: &WorkspaceRecord) -> NexusResult<()> {
        self.conn
            .execute(
                "INSERT INTO workspaces (id, name, path, primary_agent, fallback_agent, model_id, profile_id, created_at, updated_at, last_opened_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    w.id,
                    w.name,
                    w.path,
                    w.primary_agent,
                    w.fallback_agent,
                    w.model_id,
                    w.profile_id,
                    w.created_at,
                    w.updated_at,
                    w.last_opened_at,
                ],
            )
            .map_err(|e| {
                if let rusqlite::Error::SqliteFailure(ref f, _) = e {
                    if f.code == rusqlite::ErrorCode::ConstraintViolation {
                        return NexusError::new("workspace.already_exists");
                    }
                }
                db_err(e)
            })?;
        Ok(())
    }

    pub fn delete_workspace(&self, id: &str) -> NexusResult<()> {
        let _ = self.conn.execute(
            "UPDATE agent_sessions SET workspace_id = NULL WHERE workspace_id = ?1",
            [id],
        );
        let deleted = self
            .conn
            .execute("DELETE FROM workspaces WHERE id = ?1", [id])
            .map_err(db_err)?;
        if deleted == 0 {
            return Err(NexusError::new("workspace.not_found"));
        }
        Ok(())
    }

    pub fn update_workspace_opened(&self, id: &str, opened_at_rfc3339: &str) -> NexusResult<()> {
        let updated = self
            .conn
            .execute(
                "UPDATE workspaces SET last_opened_at = ?2, updated_at = ?2 WHERE id = ?1",
                params![id, opened_at_rfc3339],
            )
            .map_err(db_err)?;
        if updated == 0 {
            return Err(NexusError::new("workspace.not_found"));
        }
        Ok(())
    }

    pub fn workspaces(&self) -> NexusResult<Vec<WorkspaceRecord>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, name, path, primary_agent, fallback_agent, model_id, profile_id, created_at, updated_at, last_opened_at
                 FROM workspaces ORDER BY updated_at DESC",
            )
            .map_err(db_err)?;
        let rows = stmt
            .query_map([], |r| {
                Ok(WorkspaceRecord {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    path: r.get(2)?,
                    primary_agent: r.get(3)?,
                    fallback_agent: r.get(4)?,
                    model_id: r.get(5)?,
                    profile_id: r.get(6)?,
                    created_at: r.get(7)?,
                    updated_at: r.get(8)?,
                    last_opened_at: r.get(9)?,
                })
            })
            .map_err(db_err)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row.map_err(db_err)?);
        }
        Ok(out)
    }

    pub fn get_workspace(&self, id: &str) -> NexusResult<Option<WorkspaceRecord>> {
        self.conn
            .query_row(
                "SELECT id, name, path, primary_agent, fallback_agent, model_id, profile_id, created_at, updated_at, last_opened_at
                 FROM workspaces WHERE id = ?1",
                [id],
                |r| {
                    Ok(WorkspaceRecord {
                        id: r.get(0)?,
                        name: r.get(1)?,
                        path: r.get(2)?,
                        primary_agent: r.get(3)?,
                        fallback_agent: r.get(4)?,
                        model_id: r.get(5)?,
                        profile_id: r.get(6)?,
                        created_at: r.get(7)?,
                        updated_at: r.get(8)?,
                        last_opened_at: r.get(9)?,
                    })
                },
            )
            .optional()
            .map_err(db_err)
    }

    // ---- Agent Sessions (Phase 10) ---------------------------------------------------------

    pub fn insert_or_update_session(&self, s: &SessionRecord) -> NexusResult<()> {
        self.conn
            .execute(
                "INSERT INTO agent_sessions (
                    id, agent_id, workspace_id, external_session_id, title, project_path,
                    started_at, last_active_at, message_count, input_units, output_units,
                    estimated_cost_cents, cost_provenance, model_identifier, created_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
                 ON CONFLICT(agent_id, external_session_id) DO UPDATE SET
                    workspace_id = COALESCE(excluded.workspace_id, agent_sessions.workspace_id),
                    title = excluded.title,
                    project_path = excluded.project_path,
                    last_active_at = excluded.last_active_at,
                    message_count = excluded.message_count,
                    input_units = excluded.input_units,
                    output_units = excluded.output_units,
                    estimated_cost_cents = excluded.estimated_cost_cents,
                    cost_provenance = excluded.cost_provenance,
                    model_identifier = COALESCE(excluded.model_identifier, agent_sessions.model_identifier)",
                params![
                    s.id,
                    s.agent_id,
                    s.workspace_id,
                    s.external_session_id,
                    s.title,
                    s.project_path,
                    s.started_at,
                    s.last_active_at,
                    s.message_count,
                    s.input_tokens,
                    s.output_tokens,
                    s.estimated_cost_cents,
                    s.cost_provenance,
                    s.model_identifier,
                    s.created_at,
                ],
            )
            .map_err(db_err)?;
        Ok(())
    }

    pub fn sessions(
        &self,
        workspace_id: Option<&str>,
        agent_id: Option<AgentId>,
        limit: usize,
    ) -> NexusResult<Vec<SessionRecord>> {
        let lim = limit.clamp(1, 200);
        let mut query = "SELECT id, agent_id, workspace_id, external_session_id, title, project_path, started_at, last_active_at, message_count, input_units, output_units, estimated_cost_cents, cost_provenance, model_identifier, created_at FROM agent_sessions WHERE 1=1".to_string();
        let mut param_vals: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(ws_id) = workspace_id {
            query.push_str(" AND workspace_id = ?");
            param_vals.push(Box::new(ws_id.to_string()));
        }
        if let Some(ag_id) = agent_id {
            query.push_str(" AND agent_id = ?");
            param_vals.push(Box::new(ag_id.as_str().to_string()));
        }
        query.push_str(" ORDER BY last_active_at DESC LIMIT ?");
        param_vals.push(Box::new(lim as i64));

        let mut stmt = self.conn.prepare(&query).map_err(db_err)?;
        let param_refs: Vec<&dyn rusqlite::ToSql> = param_vals.iter().map(|p| p.as_ref()).collect();
        let rows = stmt
            .query_map(&param_refs[..], |r| {
                Ok(SessionRecord {
                    id: r.get(0)?,
                    agent_id: r.get(1)?,
                    workspace_id: r.get(2)?,
                    external_session_id: r.get(3)?,
                    title: r.get(4)?,
                    project_path: r.get(5)?,
                    started_at: r.get(6)?,
                    last_active_at: r.get(7)?,
                    message_count: r.get(8)?,
                    input_tokens: r.get(9)?,
                    output_tokens: r.get(10)?,
                    estimated_cost_cents: r.get(11)?,
                    cost_provenance: r.get(12)?,
                    model_identifier: r.get(13)?,
                    created_at: r.get(14)?,
                })
            })
            .map_err(db_err)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row.map_err(db_err)?);
        }
        Ok(out)
    }

    pub fn get_session(&self, id: &str) -> NexusResult<Option<SessionRecord>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, agent_id, workspace_id, external_session_id, title, project_path, started_at, last_active_at, message_count, input_units, output_units, estimated_cost_cents, cost_provenance, model_identifier, created_at
                 FROM agent_sessions WHERE id = ?1",
            )
            .map_err(db_err)?;
        let mut rows = stmt
            .query_map([id], |r| {
                Ok(SessionRecord {
                    id: r.get(0)?,
                    agent_id: r.get(1)?,
                    workspace_id: r.get(2)?,
                    external_session_id: r.get(3)?,
                    title: r.get(4)?,
                    project_path: r.get(5)?,
                    started_at: r.get(6)?,
                    last_active_at: r.get(7)?,
                    message_count: r.get(8)?,
                    input_tokens: r.get(9)?,
                    output_tokens: r.get(10)?,
                    estimated_cost_cents: r.get(11)?,
                    cost_provenance: r.get(12)?,
                    model_identifier: r.get(13)?,
                    created_at: r.get(14)?,
                })
            })
            .map_err(db_err)?;
        match rows.next() {
            Some(res) => Ok(Some(res.map_err(db_err)?)),
            None => Ok(None),
        }
    }

    pub fn delete_session(&self, id: &str) -> NexusResult<()> {
        let deleted = self
            .conn
            .execute("DELETE FROM agent_sessions WHERE id = ?1", [id])
            .map_err(db_err)?;
        if deleted == 0 {
            return Err(NexusError::new("session.not_found"));
        }
        Ok(())
    }

    /// Prunes resolved history older than `cutoff_rfc3339`. Unresolved (`executing` /
    /// `interrupted`) operations, their backups, the newest backup per target, and the newest
    /// health row per Agent are always preserved.
    pub fn prune_history(&self, cutoff_rfc3339: &str) -> NexusResult<PruneSummary> {
        let tx = self.conn.unchecked_transaction().map_err(db_err)?;
        let pruned_backup_ids: Vec<String> = {
            let mut stmt = tx
                .prepare(
                    "SELECT id FROM backups
                     WHERE created_at < ?1
                       AND operation_id NOT IN (
                         SELECT id FROM operations WHERE status IN ('executing', 'interrupted')
                       )
                       AND rowid NOT IN (
                         SELECT b2.rowid FROM backups b2
                         WHERE COALESCE(b2.agent_id, '') = COALESCE(backups.agent_id, '')
                           AND b2.target_id = backups.target_id
                         ORDER BY b2.created_at DESC, b2.rowid DESC LIMIT 1
                       )",
                )
                .map_err(db_err)?;
            let rows = stmt
                .query_map([cutoff_rfc3339], |r| r.get(0))
                .map_err(db_err)?;
            rows.collect::<Result<_, _>>().map_err(db_err)?
        };
        for id in &pruned_backup_ids {
            tx.execute("DELETE FROM backups WHERE id = ?1", [id])
                .map_err(db_err)?;
        }
        tx.execute(
            "DELETE FROM operation_steps WHERE operation_id IN (
               SELECT id FROM operations
               WHERE status NOT IN ('executing', 'interrupted') AND started_at < ?1
             )",
            [cutoff_rfc3339],
        )
        .map_err(db_err)?;
        let pruned_operations = tx
            .execute(
                "DELETE FROM operations
                 WHERE status NOT IN ('executing', 'interrupted') AND started_at < ?1",
                [cutoff_rfc3339],
            )
            .map_err(db_err)?;
        let pruned_health_rows = tx
            .execute(
                "DELETE FROM health_history
                 WHERE evaluated_at < ?1
                   AND id NOT IN (SELECT MAX(id) FROM health_history GROUP BY agent_id)",
                [cutoff_rfc3339],
            )
            .map_err(db_err)?;
        tx.commit().map_err(db_err)?;
        let _ = self.conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);");
        Ok(PruneSummary {
            pruned_operations,
            pruned_health_rows,
            pruned_backup_ids,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_apply_once_and_newer_schema_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nexus.db");
        Store::open(&path).unwrap();
        Store::open(&path).unwrap(); // idempotent
        let conn = Connection::open(&path).unwrap();
        conn.pragma_update(None, "user_version", 99).unwrap();
        drop(conn);
        assert_eq!(Store::open(&path).err().unwrap().code, "store.newer_schema");
    }

    #[test]
    fn profile_edits_create_revisions_and_references_block_deletes() {
        let s = Store::open_in_memory().unwrap();
        let now = Instant(1000);
        let m = ModelDefinition {
            id: "m1".into(),
            provider_id: "builtin.openai".into(),
            identifier: "gpt-x".into(),
            display_name: None,
            source: "manual".into(),
            source_detail: None,
            capabilities: vec![],
        };
        s.insert_model(&m, now).unwrap();
        assert_eq!(s.insert_model(&m, now).unwrap_err().code, "model.duplicate");
        assert_eq!(s.save_profile("p", "Fast", "m1", None, now).unwrap(), 1);
        assert_eq!(
            s.save_profile("p", "Fast", "m1", Some("high"), now)
                .unwrap(),
            2
        );
        assert_eq!(s.profiles().unwrap()[0].revision, 2);
        assert!(s.profile_revision("p", 1).unwrap().is_some());
        assert_eq!(s.delete_model("m1").unwrap_err().code, "model.in_use");
        s.set_desired(
            AgentId::Codex,
            AssignmentMode::Profile,
            None,
            Some("p"),
            Some(2),
            now,
        )
        .unwrap();
        assert_eq!(s.delete_profile("p").unwrap_err().code, "profile.in_use");
        s.clear_assignment(AgentId::Codex).unwrap();
        s.delete_profile("p").unwrap();
        assert!(s.profiles().unwrap().is_empty());
        assert_eq!(s.delete_profile("p").unwrap_err().code, "profile.not_found");
    }

    #[test]
    fn prune_history_preserves_unresolved_ops_and_latest_target_backups() {
        let s = Store::open_in_memory().unwrap();
        s.begin_operation(
            "op_done_old",
            "applyModel",
            Some(AgentId::Codex),
            "d1",
            &serde_json::json!({}),
            Instant(100),
        )
        .unwrap();
        s.finish_operation("op_done_old", "succeeded", None, Instant(101))
            .unwrap();
        s.insert_backup(
            "bak_00000000000000000000000000000001",
            "op_done_old",
            Some(AgentId::Codex),
            "codex.user",
            "/tmp/c",
            "~/.codex/config.toml",
            "sha1",
            10,
            Instant(100),
        )
        .unwrap();
        s.begin_operation(
            "op_done_newer",
            "applyModel",
            Some(AgentId::Codex),
            "d2",
            &serde_json::json!({}),
            Instant(200),
        )
        .unwrap();
        s.finish_operation("op_done_newer", "succeeded", None, Instant(201))
            .unwrap();
        s.insert_backup(
            "bak_00000000000000000000000000000002",
            "op_done_newer",
            Some(AgentId::Codex),
            "codex.user",
            "/tmp/c",
            "~/.codex/config.toml",
            "sha2",
            12,
            Instant(200),
        )
        .unwrap();
        s.begin_operation(
            "op_interrupted",
            "install",
            Some(AgentId::Gemini),
            "d3",
            &serde_json::json!({}),
            Instant(150),
        )
        .unwrap();
        s.finish_operation(
            "op_interrupted",
            "interrupted",
            Some("operation.interrupted"),
            Instant(151),
        )
        .unwrap();
        s.record_health(AgentId::Codex, "ready", "v1", &[], Instant(100))
            .unwrap();
        s.record_health(
            AgentId::Codex,
            "attention",
            "v1",
            &["x".into()],
            Instant(200),
        )
        .unwrap();

        let summary = s.prune_history(&Instant(500).to_rfc3339()).unwrap();
        assert_eq!(summary.pruned_operations, 2);
        assert_eq!(summary.pruned_health_rows, 1);
        assert_eq!(
            summary.pruned_backup_ids,
            vec!["bak_00000000000000000000000000000001".to_string()]
        );
        let counts = s.counts().unwrap();
        assert_eq!(counts.operation_count, 1);
        assert_eq!(counts.unresolved_count, 1);
        assert_eq!(counts.health_row_count, 1);
        assert_eq!(counts.backup_count, 1);
    }

    #[test]
    fn interrupted_operations_are_reconciled_not_replayed() {
        let s = Store::open_in_memory().unwrap();
        s.begin_operation(
            "op_1",
            "lifecycle.install",
            Some(AgentId::Gemini),
            "d",
            &serde_json::json!({}),
            Instant(1),
        )
        .unwrap();
        s.step("op_1", "started", Instant(1)).unwrap();
        assert_eq!(
            s.begin_operation("op_1", "x", None, "d", &serde_json::json!({}), Instant(1))
                .unwrap_err()
                .code,
            "operation.replayed"
        );
        assert_eq!(
            s.reconcile_interrupted(Instant(2)).unwrap(),
            vec!["op_1".to_string()]
        );
        let ops = s.operations(10).unwrap();
        assert_eq!(ops[0].status, "interrupted");
        assert_eq!(ops[0].steps, vec!["started".to_string()]);
    }

    #[test]
    fn applied_record_is_separate_from_desired_intent() {
        let s = Store::open_in_memory().unwrap();
        s.set_desired(
            AgentId::Claude,
            AssignmentMode::Direct,
            Some("m"),
            None,
            None,
            Instant(1),
        )
        .unwrap();
        assert!(s
            .assignment(AgentId::Claude)
            .unwrap()
            .unwrap()
            .applied
            .is_none());
        s.set_applied(
            AgentId::Claude,
            &AppliedRecord {
                identifier: "opus".into(),
                profile_revision: None,
                provider_revision: Some(1),
                applied_at: "t".into(),
                location: "~/x".into(),
                fingerprint: "f".into(),
            },
        )
        .unwrap();
        s.set_desired(
            AgentId::Claude,
            AssignmentMode::Direct,
            Some("m2"),
            None,
            None,
            Instant(2),
        )
        .unwrap();
        let a = s.assignment(AgentId::Claude).unwrap().unwrap();
        assert_eq!(a.model_id.as_deref(), Some("m2"));
        assert_eq!(a.applied.unwrap().identifier, "opus");
    }

    #[test]
    fn schema_has_no_secret_columns() {
        for (_, sql) in MIGRATIONS {
            let lower = sql.to_lowercase();
            assert!(
                !lower.contains("api_key")
                    && !lower.contains("token")
                    && !lower.contains("password")
                    && !lower.contains("secret_value")
            );
        }
    }

    #[test]
    fn workspace_crud_and_uniqueness() {
        let s = Store::open_in_memory().unwrap();
        let ws = WorkspaceRecord {
            id: "ws_1".into(),
            name: "Demo Project".into(),
            path: "/Users/test/demo".into(),
            primary_agent: "claude".into(),
            fallback_agent: Some("codex".into()),
            model_id: None,
            profile_id: None,
            created_at: "2026-11-05T00:00:00Z".into(),
            updated_at: "2026-11-05T00:00:00Z".into(),
            last_opened_at: None,
        };
        s.insert_workspace(&ws).unwrap();
        assert_eq!(s.workspaces().unwrap().len(), 1);
        assert_eq!(
            s.get_workspace("ws_1").unwrap().unwrap().name,
            "Demo Project"
        );
        assert_eq!(s.counts().unwrap().workspace_count, 1);

        // Duplicate path is rejected
        let dup = WorkspaceRecord {
            id: "ws_2".into(),
            name: "Demo Project 2".into(),
            path: "/Users/test/demo".into(),
            primary_agent: "codex".into(),
            fallback_agent: None,
            model_id: None,
            profile_id: None,
            created_at: "2026-11-05T00:00:00Z".into(),
            updated_at: "2026-11-05T00:00:00Z".into(),
            last_opened_at: None,
        };
        assert_eq!(
            s.insert_workspace(&dup).unwrap_err().code,
            "workspace.already_exists"
        );

        // Update opened
        s.update_workspace_opened("ws_1", "2026-11-05T01:00:00Z")
            .unwrap();
        assert_eq!(
            s.get_workspace("ws_1")
                .unwrap()
                .unwrap()
                .last_opened_at
                .as_deref(),
            Some("2026-11-05T01:00:00Z")
        );

        // Delete
        s.delete_workspace("ws_1").unwrap();
        assert_eq!(s.workspaces().unwrap().len(), 0);
        assert_eq!(
            s.delete_workspace("ws_1").unwrap_err().code,
            "workspace.not_found"
        );
    }

    #[test]
    fn session_crud_and_workspace_link() {
        let s = Store::open_in_memory().unwrap();
        let ws = WorkspaceRecord {
            id: "ws_alpha".into(),
            name: "Alpha Workspace".into(),
            path: "/Users/test/alpha".into(),
            primary_agent: "claude".into(),
            fallback_agent: None,
            model_id: None,
            profile_id: None,
            created_at: "2026-11-05T00:00:00Z".into(),
            updated_at: "2026-11-05T00:00:00Z".into(),
            last_opened_at: None,
        };
        s.insert_workspace(&ws).unwrap();

        let session = SessionRecord {
            id: "sess_1".into(),
            agent_id: "claude".into(),
            workspace_id: Some("ws_alpha".into()),
            external_session_id: "ext-uuid-1".into(),
            title: "Implement session continuity".into(),
            project_path: "/Users/test/alpha".into(),
            started_at: "2026-11-05T00:00:00Z".into(),
            last_active_at: "2026-11-05T00:15:00Z".into(),
            message_count: 12,
            input_tokens: 15000,
            output_tokens: 3000,
            estimated_cost_cents: 9,
            cost_provenance: "Official pricing".into(),
            model_identifier: Some("claude-3-5-sonnet".into()),
            created_at: "2026-11-05T00:00:00Z".into(),
        };
        s.insert_or_update_session(&session).unwrap();

        assert_eq!(s.counts().unwrap().session_count, 1);
        let list = s.sessions(Some("ws_alpha"), None, 50).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].title, "Implement session continuity");
        assert_eq!(list[0].message_count, 12);

        // Update on conflict
        let mut updated = session.clone();
        updated.message_count = 14;
        updated.input_tokens = 20000;
        s.insert_or_update_session(&updated).unwrap();
        let fetched = s.get_session("sess_1").unwrap().unwrap();
        assert_eq!(fetched.message_count, 14);
        assert_eq!(fetched.input_tokens, 20000);

        // Workspace deletion nullifies workspace_id, retaining session reference
        s.delete_workspace("ws_alpha").unwrap();
        let list_after_ws_del = s.sessions(None, None, 50).unwrap();
        assert_eq!(list_after_ws_del.len(), 1);
        assert_eq!(list_after_ws_del[0].workspace_id, None);

        // Explicit session deletion
        s.delete_session("sess_1").unwrap();
        assert_eq!(s.sessions(None, None, 50).unwrap().len(), 0);
        assert_eq!(s.counts().unwrap().session_count, 0);
    }
}
