//! Application use cases: assembles discovery, Health, Diagnostics and Models into the typed
//! results IPC returns. Owns process-wide state (store, plans, locks, cached overview).

use crate::actions::{Plan, PlanView};
use crate::adapters;
use crate::backup::BackupVault;
use crate::core::{Instant, NexusError, NexusResult};
use crate::diagnostics::{diagnose, DiagnosticFinding, DiagnosticInputs};
use crate::discovery::discover_agent;
use crate::discovery::system::{discover_system, SystemSnapshot};
use crate::domain::*;
use crate::health::{evaluate, AgentHealth, AssignmentContext};
use crate::lifecycle;
use crate::models::*;
use crate::platform::HostContext;
use crate::secrets::SecretStore;
use crate::store::{BackupRecord, OperationRecord, Store};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, MutexGuard};

pub trait Confirmer: Send + Sync {
    fn confirm(&self, plan: &PlanView, locale: &str) -> bool;
}

pub struct App {
    pub ctx: HostContext,
    store: Mutex<Store>,
    pub secrets: Arc<dyn SecretStore>,
    pub vault: BackupVault,
    pub plans: Mutex<HashMap<String, Plan>>,
    pub busy: Mutex<HashSet<String>>,
    pub confirmer: Box<dyn Confirmer>,
    cache: Mutex<Option<Overview>>,
    locale: Mutex<String>,
    provider_statuses: Mutex<HashMap<String, crate::network::ProviderReachabilityResult>>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LifecycleOption {
    pub recipe_id: &'static str,
    pub op: lifecycle::Op,
    pub package: String,
    pub available: bool,
    pub installation_id: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentReport {
    pub snapshot: AgentSnapshot,
    pub health: AgentHealth,
    pub findings: Vec<DiagnosticFinding>,
    pub assignment: Option<ResolvedAssignment>,
    pub lifecycle: Vec<LifecycleOption>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Overview {
    pub contract_version: u8,
    pub generated_at: Instant,
    pub system: SystemSnapshot,
    pub agents: Vec<AgentReport>,
    pub recent_operations: Vec<OperationRecord>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedAssignment {
    pub assignment: AgentModelAssignment,
    pub provider: ProviderProfile,
    pub model: ModelDefinition,
    pub profile: Option<ModelProfile>,
    pub profile_revision: Option<i64>,
    pub compatibility: Compatibility,
    pub pending: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelsView {
    pub providers: Vec<ProviderProfile>,
    pub models: Vec<ModelDefinition>,
    pub profiles: Vec<ModelProfile>,
    pub assignments: Vec<ResolvedAssignment>,
    pub matrix: Vec<(String, Vec<Compatibility>)>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentHistory {
    pub backups: Vec<BackupRecord>,
    pub operations: Vec<OperationRecord>,
    pub health: Vec<crate::store::HealthHistoryRow>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageStatus {
    pub data_dir: String,
    pub db_bytes: u64,
    pub backup_count: usize,
    pub backup_bytes: u64,
    pub operation_count: i64,
    pub unresolved_count: i64,
    pub health_row_count: i64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PruneResult {
    pub pruned_operations: usize,
    pub pruned_health_rows: usize,
    pub pruned_backups: usize,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticTool {
    pub key: String,
    pub present: bool,
    pub version: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticSystem {
    pub os: crate::platform::Os,
    pub os_version: Option<String>,
    pub arch: String,
    pub data_dir: String,
    pub tools: Vec<DiagnosticTool>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticConfigSource {
    pub id: String,
    pub scope: ConfigScope,
    pub location: String,
    pub status: ObsStatus,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticAgent {
    pub agent_id: AgentId,
    pub readiness: Readiness,
    pub installed: bool,
    pub primary_source: Option<InstallSource>,
    pub version: Option<String>,
    pub config_sources: Vec<DiagnosticConfigSource>,
    pub auth_evidence: Vec<AuthEvidence>,
    pub mcp_count: usize,
    pub skill_count: usize,
    pub finding_codes: Vec<String>,
    pub non_passing_checks: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticOperation {
    pub action: String,
    pub agent_id: Option<String>,
    pub status: String,
    pub error_code: Option<String>,
    pub started_at: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticBundle {
    pub schema_version: u8,
    pub generated_at: Instant,
    pub app_version: &'static str,
    pub system: DiagnosticSystem,
    pub agents: Vec<DiagnosticAgent>,
    pub recent_operations: Vec<DiagnosticOperation>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticPreview {
    pub filename: String,
    pub json: String,
}

impl App {
    pub fn new(
        ctx: HostContext,
        store: Store,
        secrets: Arc<dyn SecretStore>,
        confirmer: Box<dyn Confirmer>,
    ) -> NexusResult<App> {
        crate::platform::fs::ensure_private_dir(&ctx.data_dir)?;
        let vault = BackupVault::new(ctx.data_dir.join("backups"), secrets.clone());
        // Anything still "executing" was interrupted by a crash or kill: mark, never replay.
        store.reconcile_interrupted(Instant::now())?;
        crate::registry::ensure_registry_seeded(&store)?;
        crate::governance::ensure_governance_seeded(&store)?;
        Ok(App {
            ctx,
            store: Mutex::new(store),
            secrets,
            vault,
            plans: Mutex::new(HashMap::new()),
            busy: Mutex::new(HashSet::new()),
            confirmer,
            cache: Mutex::new(None),
            locale: Mutex::new("en-US".into()),
            provider_statuses: Mutex::new(HashMap::new()),
        })
    }

    pub fn store(&self) -> NexusResult<MutexGuard<'_, Store>> {
        self.store
            .lock()
            .map_err(|_| NexusError::new("app.state_poisoned"))
    }
    pub fn locale(&self) -> String {
        self.locale
            .lock()
            .map(|l| l.clone())
            .unwrap_or_else(|_| "en-US".into())
    }
    pub fn invalidate(&self) {
        if let Ok(mut c) = self.cache.lock() {
            *c = None;
        }
    }

    pub fn all_providers(&self) -> NexusResult<Vec<ProviderProfile>> {
        let mut out = builtin_providers();
        for mut p in self.store()?.providers()? {
            if let Some(r) = &p.secret_ref {
                // Presence only. A locked store is reported as unknown, never as absent.
                p.secret_present = self.secrets.get(r).ok().map(|v| v.is_some());
            }
            out.push(p);
        }
        if let Ok(lock) = self.provider_statuses.lock() {
            for p in &mut out {
                if let Some(status) = lock.get(&p.id) {
                    p.connection = match status.status.as_str() {
                        "connected" => "connected",
                        "authFailed" => "authFailed",
                        "unreachable" => "unreachable",
                        "invalidEndpoint" => "invalidEndpoint",
                        _ => "notTested",
                    };
                }
            }
        }
        Ok(out)
    }

    pub fn check_provider(
        &self,
        provider_id: &str,
    ) -> NexusResult<crate::network::ProviderReachabilityResult> {
        let providers = self.all_providers()?;
        let p = providers
            .iter()
            .find(|p| p.id == provider_id)
            .ok_or_else(|| NexusError::new("provider.not_found"))?;
        let secret = if let Some(r) = &p.secret_ref {
            self.secrets.get(r).ok().flatten()
        } else {
            None
        };
        let res = crate::network::check_provider_reachability(
            &p.id,
            p.base_url.as_deref(),
            secret.as_deref(),
        );
        if let Ok(mut lock) = self.provider_statuses.lock() {
            lock.insert(provider_id.to_string(), res.clone());
        }
        Ok(res)
    }

    pub fn all_models(&self, observed: &[ModelDefinition]) -> NexusResult<Vec<ModelDefinition>> {
        let mut out = builtin_models();
        out.extend(self.store()?.models()?);
        for m in observed {
            if !out
                .iter()
                .any(|x| x.provider_id == m.provider_id && x.identifier == m.identifier)
            {
                out.push(m.clone());
            }
        }
        Ok(out)
    }

    /// Model identifiers currently configured in Agent configs become `observed` catalog
    /// entries (with provenance), so users can adopt what they already use.
    pub fn observed_models(&self, snapshots: &[AgentSnapshot]) -> Vec<ModelDefinition> {
        let mut out = Vec::new();
        for snap in snapshots {
            for layer in snap
                .model
                .layers
                .iter()
                .filter(|l| l.kind == ModelSourceKind::UserConfig)
            {
                let Some(value) = &layer.value else { continue };
                if validate_identifier(value).is_err() {
                    continue;
                }
                let (provider, identifier) = match snap.agent_id {
                    AgentId::Claude if snap.model.provider_hint.as_deref() == Some("anthropic") => {
                        ("builtin.anthropic", value.clone())
                    }
                    AgentId::Codex
                        if matches!(snap.model.provider_hint.as_deref(), None | Some("openai")) =>
                    {
                        ("builtin.openai", value.clone())
                    }
                    AgentId::Gemini => ("builtin.google", value.clone()),
                    AgentId::Opencode => match value.split_once('/') {
                        Some(("anthropic", m)) => ("builtin.anthropic", m.to_string()),
                        Some(("openai", m)) => ("builtin.openai", m.to_string()),
                        Some(("google", m)) => ("builtin.google", m.to_string()),
                        _ => continue,
                    },
                    _ => continue,
                };
                if out.iter().any(|m: &ModelDefinition| {
                    m.provider_id == provider && m.identifier == identifier
                }) {
                    continue;
                }
                out.push(ModelDefinition {
                    id: format!("observed.{provider}.{identifier}"),
                    provider_id: provider.into(),
                    identifier,
                    display_name: None,
                    source: "observed".into(),
                    source_detail: Some(layer.location.clone()),
                    capabilities: Vec::new(),
                });
            }
        }
        out
    }

    pub fn resolve_assignment(&self, agent: AgentId) -> NexusResult<Option<ResolvedAssignment>> {
        let Some(assignment) = self.store()?.assignment(agent)? else {
            return Ok(None);
        };
        let (model_id, profile, profile_revision) = match assignment.mode {
            AssignmentMode::Native => return Ok(None),
            AssignmentMode::Direct => (assignment.model_id.clone(), None, None),
            AssignmentMode::Profile => {
                let pid = assignment
                    .profile_id
                    .clone()
                    .ok_or_else(|| NexusError::new("assignment.invalid"))?;
                let profile = self
                    .store()?
                    .profiles()?
                    .into_iter()
                    .find(|p| p.id == pid)
                    .ok_or_else(|| NexusError::new("profile.not_found"))?;
                let rev = profile.revision;
                (Some(profile.model_id.clone()), Some(profile), Some(rev))
            }
        };
        let model_id = model_id.ok_or_else(|| NexusError::new("assignment.invalid"))?;
        let models = self.all_models(&[])?;
        let model = models
            .into_iter()
            .find(|m| m.id == model_id)
            .ok_or_else(|| NexusError::new("model.not_found"))?;
        let provider = self
            .all_providers()?
            .into_iter()
            .find(|p| p.id == model.provider_id)
            .ok_or_else(|| NexusError::new("provider.not_found"))?;
        let compatibility = compatibility(agent, &provider, &model);
        let pending = match &assignment.applied {
            None => true,
            Some(a) => {
                Some(&a.identifier) != compatibility.native_identifier.as_ref()
                    || (assignment.mode == AssignmentMode::Profile
                        && a.profile_revision != profile_revision)
            }
        };
        Ok(Some(ResolvedAssignment {
            assignment,
            provider,
            model,
            profile,
            profile_revision,
            compatibility,
            pending,
        }))
    }

    fn report(
        &self,
        mut snap: AgentSnapshot,
        system: &SystemSnapshot,
        now: Instant,
        ops: &[OperationRecord],
    ) -> NexusResult<AgentReport> {
        let managed_exts = self.store()?.managed_extensions(snap.agent_id)?;
        for server in &mut snap.extensions.mcp {
            server.nexus_managed = managed_exts.contains(&server.name);
        }
        for skill in &mut snap.extensions.skills {
            skill.nexus_managed = managed_exts.contains(&skill.name);
        }
        let resolved = self.resolve_assignment(snap.agent_id).unwrap_or(None);
        let ctx = match &resolved {
            Some(r) => AssignmentContext {
                desired_identifier: r
                    .compatibility
                    .native_identifier
                    .clone()
                    .or_else(|| Some(r.model.identifier.clone())),
                compatible: Some(r.compatibility.support),
                applied_identifier: r.assignment.applied.as_ref().map(|a| a.identifier.clone()),
                desired_matches_applied: !r.pending,
            },
            None => AssignmentContext::default(),
        };
        let health = evaluate(&snap, system, &ctx, now);
        let backups = self.store()?.backups(Some(snap.agent_id))?;
        let interrupted: Vec<OperationRecord> = ops
            .iter()
            .filter(|o| {
                o.status == "interrupted" && o.agent_id.as_deref() == Some(snap.agent_id.as_str())
            })
            .cloned()
            .collect();
        let tools = lifecycle::toolchain(&self.ctx);
        let recipes = adapters::adapter(snap.agent_id).recipes();
        let mut options = Vec::new();
        if snap.installations.is_empty() {
            for r in recipes.iter().filter(|r| {
                r.ops.contains(&lifecycle::Op::Install)
                    && r.mechanism.platforms().contains(&self.ctx.os)
            }) {
                options.push(LifecycleOption {
                    recipe_id: r.id,
                    op: lifecycle::Op::Install,
                    package: r.mechanism.package_label(),
                    available: lifecycle::available(r, &self.ctx, &tools),
                    installation_id: None,
                });
            }
        }
        for inst in &snap.installations {
            for op in [lifecycle::Op::Update, lifecycle::Op::Uninstall] {
                if let Some(r) = recipes
                    .iter()
                    .find(|r| r.mechanism.source() == inst.source && r.ops.contains(&op))
                {
                    options.push(LifecycleOption {
                        recipe_id: r.id,
                        op,
                        package: r.mechanism.package_label(),
                        available: lifecycle::available(r, &self.ctx, &tools),
                        installation_id: Some(inst.id.clone()),
                    });
                }
            }
        }
        let install_available = options
            .iter()
            .any(|o| o.op == lifecycle::Op::Install && o.available);
        let findings = diagnose(&DiagnosticInputs {
            snapshot: &snap,
            health: &health,
            backups: &backups,
            interrupted: &interrupted,
            install_recipe_available: install_available,
        });
        Ok(AgentReport {
            snapshot: snap,
            health,
            findings,
            assignment: resolved,
            lifecycle: options,
        })
    }

    /// Full read-only scan. Agents are discovered in parallel with bounded per-probe timeouts.
    pub fn overview(&self, refresh: bool) -> NexusResult<Overview> {
        if !refresh {
            if let Some(cached) = self
                .cache
                .lock()
                .map_err(|_| NexusError::new("app.state_poisoned"))?
                .clone()
            {
                if cached
                    .agents
                    .iter()
                    .all(|a| a.health.valid_until > Instant::now())
                {
                    return Ok(cached);
                }
            }
        }
        let now = Instant::now();
        let ledger: Vec<String> = self
            .store()?
            .ledger()?
            .into_iter()
            .filter_map(|e| e.installation_id)
            .collect();
        let ctx = &self.ctx;
        let (system, snapshots) = std::thread::scope(|scope| {
            let sys = scope.spawn(|| discover_system(ctx, now));
            let handles: Vec<_> = adapters::registry()
                .into_iter()
                .map(|a| {
                    let ledger = &ledger;
                    scope.spawn(move || discover_agent(ctx, a.as_ref(), ledger, now))
                })
                .collect();
            let snaps: Vec<AgentSnapshot> =
                handles.into_iter().filter_map(|h| h.join().ok()).collect();
            (sys.join().expect("system discovery thread"), snaps)
        });
        let ops = self.store()?.operations(50)?;
        let mut agents = Vec::new();
        for snap in snapshots {
            let report = self.report(snap, &system, now, &ops)?;
            let codes: Vec<String> = report.findings.iter().map(|f| f.code.clone()).collect();
            let readiness = serde_json::to_value(report.health.readiness)
                .ok()
                .and_then(|v| v.as_str().map(String::from))
                .unwrap_or_default();
            self.store()?.record_health(
                report.snapshot.agent_id,
                &readiness,
                crate::health::RULE_SET,
                &codes,
                now,
            )?;
            agents.push(report);
        }
        let overview = Overview {
            contract_version: 1,
            generated_at: now,
            system,
            agents,
            recent_operations: ops.into_iter().take(10).collect(),
        };
        *self
            .cache
            .lock()
            .map_err(|_| NexusError::new("app.state_poisoned"))? = Some(overview.clone());
        Ok(overview)
    }

    pub fn agent_history(&self, agent: AgentId) -> NexusResult<AgentHistory> {
        let store = self.store()?;
        Ok(AgentHistory {
            backups: store.backups(Some(agent))?,
            operations: store
                .operations(100)?
                .into_iter()
                .filter(|o| o.agent_id.as_deref() == Some(agent.as_str()))
                .take(20)
                .collect(),
            health: store.health_history(agent, 20)?,
        })
    }

    pub fn models_view(&self) -> NexusResult<ModelsView> {
        let overview = self.overview(false)?;
        let snapshots: Vec<AgentSnapshot> =
            overview.agents.iter().map(|a| a.snapshot.clone()).collect();
        let observed = self.observed_models(&snapshots);
        let providers = self.all_providers()?;
        let models = self.all_models(&observed)?;
        let profiles = self.store()?.profiles()?;
        let mut assignments = Vec::new();
        for agent in AgentId::ALL {
            if let Ok(Some(r)) = self.resolve_assignment(agent) {
                assignments.push(r);
            }
        }
        let matrix = models
            .iter()
            .filter_map(|m| {
                providers.iter().find(|p| p.id == m.provider_id).map(|p| {
                    (
                        m.id.clone(),
                        AgentId::ALL
                            .iter()
                            .map(|a| compatibility(*a, p, m))
                            .collect(),
                    )
                })
            })
            .collect();
        Ok(ModelsView {
            providers,
            models,
            profiles,
            assignments,
            matrix,
        })
    }

    // ---- Model domain mutations (Nexus records only; Agent configs change via actions) ----

    pub fn create_provider(
        &self,
        name: &str,
        kind: ProviderKind,
        base_url: Option<&str>,
        auth: AuthMode,
        env_var: Option<&str>,
    ) -> NexusResult<ProviderProfile> {
        let display_name = validate_display_name(name)?;
        let base_url = match (kind, base_url) {
            (ProviderKind::OpenaiCompatible, Some(u)) => Some(validate_base_url(u)?),
            (ProviderKind::OpenaiCompatible, None) => {
                return Err(NexusError::new("provider.invalid_base_url"))
            }
            (_, Some(_)) => return Err(NexusError::new("provider.fixed_origin")),
            (k, None) => k.default_origin().map(String::from),
        };
        if auth == AuthMode::Environment {
            validate_env_var(env_var.ok_or_else(|| NexusError::new("provider.invalid_env_var"))?)?;
        }
        let id = crate::core::new_id("prov");
        let p = ProviderProfile {
            secret_ref: (auth == AuthMode::NexusSecret).then(|| format!("provider:{id}")),
            id,
            display_name,
            kind,
            base_url,
            auth_mode: auth,
            secret_present: Some(false),
            env_var: if auth == AuthMode::Environment {
                env_var.map(String::from)
            } else {
                None
            },
            revision: 1,
            builtin: false,
            connection: "notTested",
        };
        self.store()?.insert_provider(&p, Instant::now())?;
        self.invalidate();
        Ok(p)
    }

    /// Stores a secret in the OS credential store. The value is never returned or persisted
    /// elsewhere; the caller's buffer is dropped when this returns.
    pub fn set_provider_secret(&self, provider_id: &str, secret: String) -> NexusResult<()> {
        let p = self
            .store()?
            .providers()?
            .into_iter()
            .find(|p| p.id == provider_id)
            .ok_or_else(|| NexusError::new("provider.not_found"))?;
        let reference = p
            .secret_ref
            .ok_or_else(|| NexusError::new("provider.no_secret_reference"))?;
        let trimmed = secret.trim();
        if trimmed.is_empty() || trimmed.len() > 4096 || trimmed.chars().any(|c| c.is_control()) {
            return Err(NexusError::new("credential.invalid_value"));
        }
        self.secrets.set(&reference, trimmed.as_bytes())?;
        self.invalidate();
        Ok(())
    }

    pub fn delete_provider(&self, provider_id: &str) -> NexusResult<()> {
        if provider_id.starts_with("builtin.") {
            return Err(NexusError::new("provider.builtin_read_only"));
        }
        let secret = self.store()?.delete_provider(provider_id)?;
        if let Some(reference) = secret {
            self.secrets.delete(&reference)?;
        }
        self.invalidate();
        Ok(())
    }

    pub fn create_model(
        &self,
        provider_id: &str,
        identifier: &str,
        display_name: Option<&str>,
    ) -> NexusResult<ModelDefinition> {
        validate_identifier(identifier)?;
        if !self.all_providers()?.iter().any(|p| p.id == provider_id) {
            return Err(NexusError::new("provider.not_found"));
        }
        let m = ModelDefinition {
            id: crate::core::new_id("model"),
            provider_id: provider_id.into(),
            identifier: identifier.into(),
            display_name: display_name.map(validate_display_name).transpose()?,
            source: "manual".into(),
            source_detail: None,
            capabilities: Vec::new(),
        };
        self.store()?.insert_model(&m, Instant::now())?;
        self.invalidate();
        Ok(m)
    }

    pub fn delete_model(&self, id: &str) -> NexusResult<()> {
        self.store()?.delete_model(id)?;
        self.invalidate();
        Ok(())
    }

    /// Ensures a model id refers to a stored or builtin definition, importing an observed one.
    fn materialize_model(&self, model_id: &str) -> NexusResult<()> {
        if self.all_models(&[])?.iter().any(|m| m.id == model_id) {
            return Ok(());
        }
        let overview = self.overview(false)?;
        let snaps: Vec<AgentSnapshot> =
            overview.agents.iter().map(|a| a.snapshot.clone()).collect();
        let observed = self
            .observed_models(&snaps)
            .into_iter()
            .find(|m| m.id == model_id)
            .ok_or_else(|| NexusError::new("model.not_found"))?;
        self.store()?.insert_model(&observed, Instant::now())
    }

    pub fn save_profile(
        &self,
        id: Option<&str>,
        name: &str,
        model_id: &str,
        reasoning: Option<&str>,
    ) -> NexusResult<ModelProfile> {
        let name = validate_display_name(name)?;
        if let Some(r) = reasoning {
            if !matches!(r, "low" | "medium" | "high") {
                return Err(NexusError::new("profile.invalid_reasoning"));
            }
        }
        self.materialize_model(model_id)?;
        let id = match id {
            Some(existing) => {
                if !self.store()?.profiles()?.iter().any(|p| p.id == existing) {
                    return Err(NexusError::new("profile.not_found"));
                }
                existing.to_string()
            }
            None => crate::core::new_id("profile"),
        };
        let revision =
            self.store()?
                .save_profile(&id, &name, model_id, reasoning, Instant::now())?;
        self.invalidate();
        Ok(ModelProfile {
            id,
            name,
            revision,
            model_id: model_id.into(),
            reasoning: reasoning.map(String::from),
        })
    }

    /// Records desired intent. The Agent configuration is unchanged until an applyModel plan is
    /// previewed, confirmed and verified.
    pub fn set_assignment(
        &self,
        agent: AgentId,
        mode: AssignmentMode,
        model_id: Option<&str>,
        profile_id: Option<&str>,
    ) -> NexusResult<Option<ResolvedAssignment>> {
        let now = Instant::now();
        match mode {
            AssignmentMode::Native => {
                self.store()?.clear_assignment(agent)?;
            }
            AssignmentMode::Direct => {
                let m = model_id.ok_or_else(|| NexusError::new("assignment.invalid"))?;
                self.materialize_model(m)?;
                self.store()?
                    .set_desired(agent, mode, Some(m), None, None, now)?;
            }
            AssignmentMode::Profile => {
                let pid = profile_id.ok_or_else(|| NexusError::new("assignment.invalid"))?;
                let p = self
                    .store()?
                    .profiles()?
                    .into_iter()
                    .find(|p| p.id == pid)
                    .ok_or_else(|| NexusError::new("profile.not_found"))?;
                self.store()?
                    .set_desired(agent, mode, None, Some(pid), Some(p.revision), now)?;
            }
        }
        self.invalidate();
        self.resolve_assignment(agent)
    }

    pub fn delete_profile(&self, id: &str) -> NexusResult<()> {
        self.store()?.delete_profile(id)?;
        self.invalidate();
        Ok(())
    }

    pub fn storage_status(&self) -> NexusResult<StorageStatus> {
        let counts = self.store()?.counts()?;
        let mut db_bytes = 0u64;
        for name in ["nexus.db", "nexus.db-wal"] {
            if let Ok(meta) = std::fs::metadata(self.ctx.data_dir.join(name)) {
                db_bytes = db_bytes.saturating_add(meta.len());
            }
        }
        let (vault_files, backup_bytes) = self.vault.disk_usage();
        Ok(StorageStatus {
            data_dir: self.ctx.display_path(&self.ctx.data_dir),
            db_bytes,
            backup_count: (counts.backup_count.max(0) as usize).max(vault_files),
            backup_bytes,
            operation_count: counts.operation_count,
            unresolved_count: counts.unresolved_count,
            health_row_count: counts.health_row_count,
        })
    }

    pub fn prune_history(&self, retain_days: u32) -> NexusResult<PruneResult> {
        if retain_days > 3650 {
            return Err(NexusError::new("ipc.invalid_input"));
        }
        let cutoff = if retain_days == 0 {
            Instant::now().plus_secs(1)
        } else {
            Instant(
                Instant::now()
                    .0
                    .saturating_sub((retain_days as i64) * 86_400),
            )
        };
        let summary = self.store()?.prune_history(&cutoff.to_rfc3339())?;
        for id in &summary.pruned_backup_ids {
            let _ = self.vault.remove(id);
        }
        self.invalidate();
        Ok(PruneResult {
            pruned_operations: summary.pruned_operations,
            pruned_health_rows: summary.pruned_health_rows,
            pruned_backups: summary.pruned_backup_ids.len(),
        })
    }

    /// Generates a redacted, user-previewable diagnostic bundle. Contains readiness, check IDs,
    /// finding codes, and `~`-relative paths only — never secrets, raw configuration, or process
    /// output (Security §6/§7, AGENTS.md).
    pub fn diagnostic_preview(&self) -> NexusResult<DiagnosticPreview> {
        let overview = self.overview(false)?;
        let bundle = DiagnosticBundle {
            schema_version: 1,
            generated_at: Instant::now(),
            app_version: env!("CARGO_PKG_VERSION"),
            system: DiagnosticSystem {
                os: overview.system.os,
                os_version: overview.system.os_version.value,
                arch: overview.system.arch.to_string(),
                data_dir: overview.system.data_dir,
                tools: overview
                    .system
                    .tools
                    .into_iter()
                    .map(|t| DiagnosticTool {
                        key: t.key.to_string(),
                        present: t.present,
                        version: t.version.value,
                    })
                    .collect(),
            },
            agents: overview
                .agents
                .into_iter()
                .map(|r| {
                    let primary = r.snapshot.primary_installation().cloned();
                    DiagnosticAgent {
                        agent_id: r.snapshot.agent_id,
                        readiness: r.health.readiness,
                        installed: !r.snapshot.installations.is_empty(),
                        primary_source: primary.as_ref().map(|i| i.source),
                        version: primary.and_then(|i| i.version.value),
                        config_sources: r
                            .snapshot
                            .config_sources
                            .into_iter()
                            .map(|c| DiagnosticConfigSource {
                                id: c.id,
                                scope: c.scope,
                                location: c.location,
                                status: c.status,
                            })
                            .collect(),
                        auth_evidence: r.snapshot.auth.evidence,
                        mcp_count: r.snapshot.extensions.mcp.len(),
                        skill_count: r.snapshot.extensions.skills.len(),
                        finding_codes: r.findings.into_iter().map(|f| f.code).collect(),
                        non_passing_checks: r
                            .health
                            .checks
                            .into_iter()
                            .filter(|c| {
                                matches!(
                                    c.outcome,
                                    crate::health::Outcome::Fail | crate::health::Outcome::Unknown
                                )
                            })
                            .map(|c| c.id.to_string())
                            .collect(),
                    }
                })
                .collect(),
            recent_operations: overview
                .recent_operations
                .into_iter()
                .map(|op| DiagnosticOperation {
                    action: op.action,
                    agent_id: op.agent_id,
                    status: op.status,
                    error_code: op.error_code,
                    started_at: op.started_at,
                })
                .collect(),
        };
        let raw = serde_json::to_string_pretty(&bundle)
            .map_err(|_| NexusError::new("app.task_failed"))?;
        let home_str = self.ctx.home.to_string_lossy();
        let minimized = if !home_str.is_empty() && home_str != "/" {
            raw.replace(home_str.as_ref(), "~")
        } else {
            raw
        };
        Ok(DiagnosticPreview {
            filename: "nexus-diagnostics.json".into(),
            json: crate::core::redact(&minimized),
        })
    }

    // ---- Workspaces (Phase 8) ---------------------------------------------------------------

    pub fn list_workspaces(&self) -> NexusResult<Vec<crate::workspaces::WorkspaceView>> {
        let records = {
            let store = self.store()?;
            store.workspaces()?
        };
        let overview = self.overview(false)?;
        let mut installed = HashMap::new();
        let mut health = HashMap::new();
        for a in &overview.agents {
            installed.insert(a.snapshot.agent_id, !a.snapshot.installations.is_empty());
            health.insert(a.snapshot.agent_id, a.health.readiness);
        }
        let views = records
            .iter()
            .map(|r| crate::workspaces::inspect_workspace(&self.ctx, r, &installed, &health))
            .collect();
        Ok(views)
    }

    pub fn create_workspace(
        &self,
        req: crate::workspaces::CreateWorkspaceRequest,
    ) -> NexusResult<crate::workspaces::WorkspaceView> {
        let now = Instant::now();
        let record = {
            let store = self.store()?;
            crate::workspaces::create_workspace(&self.ctx, &store, req, now)?
        };
        let overview = self.overview(false)?;
        let mut installed = HashMap::new();
        let mut health = HashMap::new();
        for a in &overview.agents {
            installed.insert(a.snapshot.agent_id, !a.snapshot.installations.is_empty());
            health.insert(a.snapshot.agent_id, a.health.readiness);
        }
        Ok(crate::workspaces::inspect_workspace(
            &self.ctx, &record, &installed, &health,
        ))
    }

    pub fn delete_workspace(&self, id: &str) -> NexusResult<()> {
        let store = self.store()?;
        store.delete_workspace(id)
    }

    pub fn discover_local_workspaces(
        &self,
    ) -> NexusResult<Vec<crate::workspaces::DiscoveredWorkspaceCandidate>> {
        let store = self.store()?;
        crate::workspaces::discover_local_workspaces(&self.ctx, &store, Instant::now())
    }

    pub fn live_pulse(&self) -> NexusResult<crate::live_pulse::LivePulseReport> {
        let overview = self.overview(false)?;
        Ok(crate::live_pulse::collect_live_pulse(
            &self.ctx,
            &overview.agents,
            Instant::now(),
        ))
    }

    pub fn consult_ai_doctor(
        &self,
        agent_id: AgentId,
    ) -> NexusResult<crate::ai_doctor::AiDoctorConsultation> {
        let overview = self.overview(false)?;
        let agent_view = overview
            .agents
            .iter()
            .find(|a| a.snapshot.agent_id == agent_id)
            .ok_or_else(|| NexusError::new("ai_doctor.agent_not_found"))?;

        let (backups, operations, desired_assignment) = {
            let store = self.store()?;
            let backups = store.backups(Some(agent_id))?;
            let operations = store.operations(50)?;
            let desired = store.assignment(agent_id)?.and_then(|a| a.model_id);
            (backups, operations, desired)
        };

        let interrupted: Vec<OperationRecord> = operations
            .into_iter()
            .filter(|o| {
                o.status == "interrupted" && o.agent_id.as_deref() == Some(agent_id.as_str())
            })
            .collect();

        Ok(crate::ai_doctor::consult(
            &crate::ai_doctor::AiDoctorInputs {
                agent_id,
                snapshot: &agent_view.snapshot,
                health: &agent_view.health,
                findings: &agent_view.findings,
                backups: &backups,
                interrupted: &interrupted,
                desired_assignment: desired_assignment.as_deref(),
                host: &self.ctx,
            },
        ))
    }

    // ---- Session Continuity & Usage Insights (Phase 10) -------------------------------------

    pub fn list_sessions(
        &self,
        workspace_id: Option<String>,
        agent_id: Option<AgentId>,
        limit: Option<usize>,
    ) -> NexusResult<Vec<crate::sessions::SessionSummary>> {
        let now = Instant::now();
        let store = self.store()?;
        let workspaces = store.workspaces().unwrap_or_default();
        for discovered in crate::sessions::discover_host_agent_sessions(&self.ctx, &workspaces, now)
        {
            let _ = store.insert_or_update_session(&discovered);
        }
        if let Some(ref wid) = workspace_id {
            if let Ok(Some(ws)) = store.get_workspace(wid) {
                let p = std::path::PathBuf::from(&ws.path);
                let primary = AgentId::parse(&ws.primary_agent).unwrap_or(AgentId::Claude);
                let markers =
                    crate::sessions::scan_project_session_markers(&self.ctx, &p, primary, wid, now);
                for m in markers {
                    let _ = store.insert_or_update_session(&m);
                }
            }
        }
        let records = store.sessions(workspace_id.as_deref(), agent_id, limit.unwrap_or(50))?;
        let summaries = records
            .iter()
            .map(|r| crate::sessions::to_session_summary(&self.ctx, r))
            .collect();
        Ok(summaries)
    }

    pub fn resume_session(&self, session_id: &str) -> NexusResult<String> {
        let store = self.store()?;
        let record = store
            .get_session(session_id)?
            .ok_or_else(|| NexusError::new("session.not_found"))?;

        let proj_path = std::path::PathBuf::from(&record.project_path);
        let aid = AgentId::parse(&record.agent_id).unwrap_or(AgentId::Claude);
        let spec = crate::sessions::build_resume_spec(
            &self.ctx,
            &proj_path,
            aid,
            &record.external_session_id,
        )?;

        // Update last_active_at
        let mut updated = record.clone();
        updated.last_active_at = Instant::now().to_rfc3339();
        let _ = store.insert_or_update_session(&updated);

        // Record operation in SQLite
        let now = Instant::now();
        let op_id = crate::core::new_id("op");
        store.begin_operation(
            &op_id,
            "resume_session",
            Some(aid),
            "resume_session",
            &serde_json::json!({
                "sessionId": session_id,
                "title": record.title,
                "projectPath": record.project_path,
            }),
            now,
        )?;

        let out = crate::platform::process::run(&spec)
            .map_err(|_| NexusError::new("lifecycle.launch_failed"))?;
        let status = if out.success() { "succeeded" } else { "failed" };
        let _ = store.finish_operation(
            &op_id,
            status,
            if out.success() {
                None
            } else {
                Some("exit_nonzero")
            },
            Instant::now(),
        );

        Ok(format!(
            "Resumed session {} for {}",
            session_id, record.agent_id
        ))
    }

    pub fn delete_session(&self, session_id: &str) -> NexusResult<()> {
        let store = self.store()?;
        store.delete_session(session_id)
    }

    pub fn usage_insights(
        &self,
        workspace_id: Option<String>,
        agent_id: Option<AgentId>,
    ) -> NexusResult<crate::usage_insights::UsageInsights> {
        let now = Instant::now();
        let store = self.store()?;
        let workspaces = store.workspaces()?;
        for discovered in crate::sessions::discover_host_agent_sessions(&self.ctx, &workspaces, now)
        {
            let _ = store.insert_or_update_session(&discovered);
        }
        let sessions = store.sessions(workspace_id.as_deref(), agent_id, 200)?;
        Ok(crate::usage_insights::compute_usage_insights(
            &sessions,
            &workspaces,
        ))
    }

    // ---- Accounts & Cloud Sync (Phase 11) ----------------------------------------------------

    pub fn get_account(&self) -> NexusResult<crate::accounts::AccountProfile> {
        let store = self.store()?;
        match store.active_account()? {
            Some(rec) => Ok(crate::accounts::record_to_profile(&rec)),
            None => Ok(crate::accounts::default_offline_account()),
        }
    }

    pub fn switch_account_tier(
        &self,
        tier_str: &str,
    ) -> NexusResult<crate::accounts::AccountProfile> {
        let store = self.store()?;
        let current = self.get_account()?;
        let new_tier = crate::accounts::AccountTier::parse(tier_str);
        let mut updated = current;
        updated.tier = new_tier;
        updated.backup_retention_days = new_tier.backup_retention_days();
        updated.cloud_sync_enabled = matches!(
            new_tier,
            crate::accounts::AccountTier::Pro | crate::accounts::AccountTier::Team
        );
        updated.is_offline_only = false;
        if updated.id == "acc_offline_local" {
            updated.id = "acc_nexus_dev".into();
            updated.email = "developer@nexus.dev".into();
            updated.display_name = "Cloud Developer".into();
            updated.created_at = crate::core::Instant::now().to_rfc3339();
        }
        let rec = crate::accounts::profile_to_record(&updated);
        store.upsert_account(&rec)?;
        Ok(updated)
    }

    pub fn list_sync_devices(&self) -> NexusResult<Vec<crate::sync::SyncDeviceSummary>> {
        let store = self.store()?;
        crate::sync::ensure_current_device_registered(&store)?;
        let records = store.list_sync_devices()?;
        Ok(crate::sync::to_device_summaries(&records))
    }

    pub fn trigger_sync(&self) -> NexusResult<crate::sync::SyncCycleResult> {
        let store = self.store()?;
        crate::sync::execute_sync_cycle(&store)
    }

    pub fn resolve_sync_conflict(&self, conflict_id: &str, strategy_str: &str) -> NexusResult<()> {
        let store = self.store()?;
        let strategy = crate::sync::ConflictResolutionStrategy::parse(strategy_str);
        crate::sync::resolve_sync_conflict(&store, conflict_id, strategy)
    }

    pub fn export_account_bundle(&self) -> NexusResult<String> {
        let store = self.store()?;
        let account = self.get_account()?;
        crate::accounts::export_account_data_bundle(&store, &account)
    }

    pub fn purge_cloud_sync(&self) -> NexusResult<usize> {
        let store = self.store()?;
        let purged_envelopes = store.purge_all_sync_envelopes()?;
        let _ = store.purge_all_paired_devices()?;
        Ok(purged_envelopes)
    }

    // ---- Extension Registry & Marketplace (Phase 12) ---------------------------------------

    pub fn list_registry_extensions(
        &self,
        query: Option<String>,
        filter_tier: Option<String>,
    ) -> NexusResult<Vec<crate::registry::RegistryExtension>> {
        let store = self.store()?;
        crate::registry::list_registry_extensions(&store, query.as_deref(), filter_tier.as_deref())
    }

    pub fn get_registry_extension(
        &self,
        extension_id: &str,
    ) -> NexusResult<Option<crate::registry::RegistryExtension>> {
        let store = self.store()?;
        crate::registry::get_registry_extension(&store, extension_id)
    }

    pub fn analyze_extension_permissions(
        &self,
        extension_id: &str,
        target_agent: AgentId,
    ) -> NexusResult<crate::registry::ExtensionPermissionAnalysis> {
        let store = self.store()?;
        crate::registry::analyze_extension_permissions(&store, extension_id, target_agent)
    }

    pub fn verify_extension_signature(
        &self,
        extension_id: &str,
    ) -> NexusResult<crate::registry::SignatureVerificationResult> {
        let store = self.store()?;
        crate::registry::verify_extension_signature(&store, extension_id)
    }

    pub fn refresh_registry_index(
        &self,
        force: bool,
    ) -> NexusResult<crate::registry::RegistrySyncSummary> {
        let store = self.store()?;
        crate::registry::refresh_registry_index(&store, force)
    }

    // ---- Team & Organization Control Plane (Phase 13) --------------------------------------

    pub fn get_governance_state(&self) -> NexusResult<crate::governance::GovernanceStateView> {
        let agents = self.overview(false).map(|o| o.agents).unwrap_or_default();
        let store = self.store()?;
        crate::governance::get_governance_state_with_agents(&store, &agents)
    }

    pub fn switch_org_role(
        &self,
        new_role: &str,
    ) -> NexusResult<crate::governance::GovernanceStateView> {
        let agents = self.overview(false).map(|o| o.agents).unwrap_or_default();
        let store = self.store()?;
        crate::governance::switch_org_role_with_agents(&store, new_role, &agents)
    }

    pub fn update_org_policy(
        &self,
        request: crate::governance::UpdateOrgPolicyRequest,
    ) -> NexusResult<crate::governance::GovernanceStateView> {
        let agents = self.overview(false).map(|o| o.agents).unwrap_or_default();
        let store = self.store()?;
        crate::governance::update_org_policy_with_agents(&store, request, &agents)
    }

    pub fn trigger_emergency_break_glass(
        &self,
        enabled: bool,
        reason: Option<String>,
    ) -> NexusResult<crate::governance::GovernanceStateView> {
        let agents = self.overview(false).map(|o| o.agents).unwrap_or_default();
        let store = self.store()?;
        crate::governance::trigger_emergency_break_glass_with_agents(
            &store,
            enabled,
            reason.as_deref().unwrap_or(""),
            &agents,
        )
    }
}

#[cfg(all(test, unix))]
pub mod test_support {
    use super::*;
    use crate::secrets::MemorySecretStore;

    pub struct Fixed(pub bool);
    impl Confirmer for Fixed {
        fn confirm(&self, _plan: &PlanView, _locale: &str) -> bool {
            self.0
        }
    }

    pub fn app(home: &std::path::Path, confirm: bool) -> App {
        app_with_confirmer(home, Box::new(Fixed(confirm)), false)
    }

    pub fn app_with_confirmer(
        home: &std::path::Path,
        confirmer: Box<dyn Confirmer>,
        disk_store: bool,
    ) -> App {
        let data_dir = home.join("nexus-data");
        let ctx = HostContext::isolated(home, &data_dir);
        let store = if disk_store {
            crate::platform::fs::ensure_private_dir(&data_dir).unwrap();
            Store::open(&data_dir.join("nexus.db")).unwrap()
        } else {
            Store::open_in_memory().unwrap()
        };
        App::new(
            ctx,
            store,
            Arc::new(MemorySecretStore::default()),
            confirmer,
        )
        .unwrap()
    }
}
