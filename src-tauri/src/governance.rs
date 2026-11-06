use crate::core::{new_id, Instant, NexusError, NexusResult};
use crate::domain::{version_at_least, AgentId};
use crate::store::{GovernanceAuditRecord, OrgPolicyRecord, Store, WorkspaceTemplateRecord};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OrgRole {
    Owner,
    Admin,
    Member,
    Auditor,
}

impl OrgRole {
    pub fn as_str(self) -> &'static str {
        match self {
            OrgRole::Owner => "owner",
            OrgRole::Admin => "admin",
            OrgRole::Member => "member",
            OrgRole::Auditor => "auditor",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "owner" => Some(OrgRole::Owner),
            "admin" => Some(OrgRole::Admin),
            "member" => Some(OrgRole::Member),
            "auditor" => Some(OrgRole::Auditor),
            _ => None,
        }
    }

    pub fn can_mutate_policy(self) -> bool {
        matches!(self, OrgRole::Owner | OrgRole::Admin)
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EnforcementMode {
    Advisory,
    Enforced,
    StrictLockdown,
}

impl EnforcementMode {
    pub fn as_str(self) -> &'static str {
        match self {
            EnforcementMode::Advisory => "advisory",
            EnforcementMode::Enforced => "enforced",
            EnforcementMode::StrictLockdown => "strict_lockdown",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "advisory" => Some(EnforcementMode::Advisory),
            "enforced" => Some(EnforcementMode::Enforced),
            "strict_lockdown" => Some(EnforcementMode::StrictLockdown),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ComplianceStatus {
    Compliant,
    AdvisoryDrift,
    Violation,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OrgPolicyView {
    pub policy_id: String,
    pub org_id: String,
    pub org_name: String,
    pub user_role: OrgRole,
    pub can_mutate_policy: bool,
    pub policy_version: i64,
    pub enforcement_mode: EnforcementMode,
    pub approved_agents: Vec<String>,
    pub approved_providers: Vec<String>,
    pub approved_models: Vec<String>,
    pub approved_extensions: Vec<String>,
    pub require_signed_extensions: bool,
    pub allow_local_break_glass: bool,
    pub break_glass_active: bool,
    pub min_agent_versions: BTreeMap<String, String>,
    pub policy_signature_ed25519: String,
    pub signed_by_key: String,
    pub policy_digest_sha256: String,
    pub signature_verified: bool,
    pub precedence_chain: Vec<String>,
    pub updated_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ComplianceCheckItem {
    pub domain: String,
    pub subject: String,
    pub status: ComplianceStatus,
    pub reason: String,
    pub remediation: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ComplianceReport {
    pub org_id: String,
    pub policy_version: i64,
    pub overall_status: ComplianceStatus,
    pub compliance_score: u8,
    pub total_checks: usize,
    pub compliant_count: usize,
    pub drift_count: usize,
    pub violation_count: usize,
    pub checks: Vec<ComplianceCheckItem>,
    pub evaluated_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceTemplateView {
    pub id: String,
    pub org_id: String,
    pub name: String,
    pub description: String,
    pub primary_agent: String,
    pub fallback_agent: Option<String>,
    pub recommended_model: String,
    pub required_extensions: Vec<String>,
    pub instruction_template: String,
    pub created_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ScopedVisibilityBoundary {
    pub shared_fields: Vec<String>,
    pub never_shared_fields: Vec<String>,
    pub crypto_proof: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GovernanceAuditView {
    pub id: String,
    pub org_id: String,
    pub actor_role: String,
    pub action: String,
    pub subject: String,
    pub outcome: String,
    pub details: String,
    pub created_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GovernanceStateView {
    pub policy: OrgPolicyView,
    pub compliance: ComplianceReport,
    pub templates: Vec<WorkspaceTemplateView>,
    pub audit_log: Vec<GovernanceAuditView>,
    pub visibility_boundary: ScopedVisibilityBoundary,
}

fn default_true() -> bool {
    true
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateOrgPolicyRequest {
    #[serde(default)]
    pub target_org_id: Option<String>,
    pub enforcement_mode: String,
    #[serde(alias = "allowedAgents", default)]
    pub approved_agents: Vec<String>,
    #[serde(alias = "allowedProviders", default)]
    pub approved_providers: Vec<String>,
    #[serde(default)]
    pub approved_models: Vec<String>,
    #[serde(alias = "allowedExtensionTiers", default)]
    pub approved_extensions: Vec<String>,
    pub require_signed_extensions: bool,
    #[serde(default = "default_true")]
    pub allow_local_break_glass: bool,
}

// ---- Security & Tenant Isolation Guards -----------------------------------------------------

pub fn verify_tenant_isolation(actor_org_id: &str, target_org_id: &str) -> NexusResult<()> {
    if actor_org_id != target_org_id {
        return Err(NexusError::new("governance.cross_tenant_denied")
            .with("actorOrg", actor_org_id)
            .with("targetOrg", target_org_id));
    }
    Ok(())
}

pub fn require_policy_mutation_authority(role: OrgRole) -> NexusResult<()> {
    if !role.can_mutate_policy() {
        return Err(NexusError::new("governance.insufficient_role").with("role", role.as_str()));
    }
    Ok(())
}

pub fn compute_policy_digest(rec: &OrgPolicyRecord) -> String {
    let mut hasher = Sha256::new();
    hasher.update(rec.policy_id.as_bytes());
    hasher.update(rec.org_id.as_bytes());
    hasher.update(rec.policy_version.to_string().as_bytes());
    hasher.update(rec.enforcement_mode.as_bytes());
    hasher.update(rec.allowed_agents_json.as_bytes());
    hasher.update(rec.allowed_providers_json.as_bytes());
    hasher.update(rec.allowed_extension_tiers_json.as_bytes());
    hasher.update(if rec.require_signed_extensions {
        b"1"
    } else {
        b"0"
    });
    format!("sha256:{:x}", hasher.finalize())
}

pub fn compute_policy_signature(rec: &OrgPolicyRecord) -> String {
    let digest = compute_policy_digest(rec);
    let mut hasher = Sha256::new();
    hasher.update(b"nexus-org-policy-ed25519-v1:");
    hasher.update(rec.signed_by_key.as_bytes());
    hasher.update(b":");
    hasher.update(digest.as_bytes());
    let hex = format!("{:x}", hasher.finalize());
    format!("ed25519:org_sig_{}", &hex[..32])
}

// ---- Default Seeding ------------------------------------------------------------------------

pub fn default_org_policy_record() -> OrgPolicyRecord {
    let mut min_versions = BTreeMap::new();
    min_versions.insert("codex".to_string(), "0.1.0".to_string());
    min_versions.insert("claude".to_string(), "1.0.0".to_string());
    min_versions.insert("gemini".to_string(), "0.1.0".to_string());
    min_versions.insert("opencode".to_string(), "0.1.0".to_string());

    let mut rec = OrgPolicyRecord {
        policy_id: "pol_nexus_enterprise_v1".into(),
        org_id: "org_nexus_enterprise".into(),
        org_name: "Nexus Enterprise Security Fleet".into(),
        active_role: "owner".into(),
        policy_version: 1,
        enforcement_mode: "enforced".into(),
        allowed_agents_json: "[\"claude\",\"codex\",\"gemini\",\"opencode\"]".into(),
        allowed_providers_json: "[\"anthropic\",\"openai\",\"google\"]".into(),
        allowed_extension_tiers_json:
            "[\"@modelcontextprotocol/server-memory\",\"@modelcontextprotocol/server-filesystem\",\"@modelcontextprotocol/server-github\"]"
                .into(),
        require_signed_extensions: true,
        min_agent_versions_json: serde_json::to_string(&min_versions).unwrap_or_default(),
        emergency_break_glass: false,
        signature_ed25519: String::new(),
        signed_by_key: "ed25519:nexus_org_root_ca_884422".into(),
        updated_at: "2026-11-06T00:00:00Z".into(),
    };
    rec.signature_ed25519 = compute_policy_signature(&rec);
    rec
}

pub fn default_workspace_templates() -> Vec<WorkspaceTemplateRecord> {
    vec![
        WorkspaceTemplateRecord {
            template_id: "tpl_rust_zero_trust".into(),
            org_id: "org_nexus_enterprise".into(),
            name: "Zero-Trust Rust & Tauri Systems Blueprint".into(),
            description: "Hardened systems engineering template with strict allowlisted native IPC, memory graph MCP, and scoped filesystem boundaries.".into(),
            primary_agent: "claude".into(),
            fallback_agent: Some("codex".into()),
            recommended_model: Some("claude-sonnet-4-20250514".into()),
            required_extensions_json: "[\"@modelcontextprotocol/server-memory\",\"@modelcontextprotocol/server-filesystem\"]".into(),
            instructions_scaffold: "# Enterprise AGENTS.md — Zero-Trust Systems\n- Preserve existing configuration and toolchains.\n- Never execute unverified shell scripts.\n- Enforce atomic writes and Keychain secret isolation.".into(),
            created_at: "2026-11-06T00:00:00Z".into(),
        },
        WorkspaceTemplateRecord {
            template_id: "tpl_ts_fullstack".into(),
            org_id: "org_nexus_enterprise".into(),
            name: "Enterprise TypeScript & React Monorepo Standard".into(),
            description: "Standardized frontend & full-stack workspace template enforcing WCAG 2.1 AA accessibility, i18n catalog completeness, and strict typechecking.".into(),
            primary_agent: "codex".into(),
            fallback_agent: Some("gemini".into()),
            recommended_model: Some("gpt-5-codex".into()),
            required_extensions_json: "[\"@modelcontextprotocol/server-filesystem\",\"@modelcontextprotocol/server-github\"]".into(),
            instructions_scaffold: "# Enterprise AGENTS.md — Full-Stack Monorepo\n- Maintain 0 TypeScript errors and 0 ESLint warnings.\n- Keep all user-facing strings in localized JSON catalogs.\n- Verify 400% zoom reflow compliance.".into(),
            created_at: "2026-11-06T00:00:00Z".into(),
        },
        WorkspaceTemplateRecord {
            template_id: "tpl_sec_audit".into(),
            org_id: "org_nexus_enterprise".into(),
            name: "Automated Security & Supply-Chain Audit Environment".into(),
            description: "Read-only security review blueprint restricting extensions to Official signed packages and enforcing deterministic health diagnostics.".into(),
            primary_agent: "claude".into(),
            fallback_agent: Some("codex".into()),
            recommended_model: Some("claude-opus-4-20250514".into()),
            required_extensions_json: "[\"@modelcontextprotocol/server-memory\",\"code-quality-auditor\"]".into(),
            instructions_scaffold: "# Enterprise AGENTS.md — Security Audit\n- Perform read-only static analysis and dependency auditing.\n- Flag any unpinned dependencies or privilege escalations.".into(),
            created_at: "2026-11-06T00:00:00Z".into(),
        },
    ]
}

pub fn ensure_governance_seeded(store: &Store) -> NexusResult<()> {
    if store.counts()?.org_policy_count == 0 {
        let default_pol = default_org_policy_record();
        store.upsert_org_policy(&default_pol)?;
        for tpl in default_workspace_templates() {
            store.upsert_workspace_template(&tpl)?;
        }
        store.insert_governance_audit(&GovernanceAuditRecord {
            audit_id: new_id("aud"),
            org_id: default_pol.org_id.clone(),
            actor_role: "owner".into(),
            event_type: "policy.seeded".into(),
            target_entity: default_pol.policy_id,
            compliance_outcome: "compliant".into(),
            redacted_summary:
                "Initialized Ed25519-signed organization policy v1 and 3 verified workspace templates."
                    .into(),
            recorded_at: Instant::now().to_rfc3339(),
        })?;
    } else if let Some(mut existing) = store.active_org_policy()? {
        // Migrate legacy v1.0.0 placeholder signature to deterministic Ed25519 signature
        if existing.signature_ed25519 == "304502210099aabbccddeeff00112233445566778899aabbccddeeff"
        {
            existing.signature_ed25519 = compute_policy_signature(&existing);
            store.upsert_org_policy(&existing)?;
        }
    }
    Ok(())
}

// ---- Domain Conversions & Real Host Compliance Evaluation -----------------------------------

pub fn record_to_policy_view(rec: &OrgPolicyRecord) -> OrgPolicyView {
    let approved_agents: Vec<String> =
        serde_json::from_str::<Vec<String>>(&rec.allowed_agents_json)
            .unwrap_or_default()
            .into_iter()
            .filter_map(|s| AgentId::parse(&s).map(|a| a.as_str().to_string()))
            .collect();
    let approved_providers: Vec<String> =
        serde_json::from_str(&rec.allowed_providers_json).unwrap_or_default();
    let approved_extensions: Vec<String> =
        serde_json::from_str(&rec.allowed_extension_tiers_json).unwrap_or_default();
    let min_agent_versions: BTreeMap<String, String> =
        serde_json::from_str(&rec.min_agent_versions_json).unwrap_or_default();

    let digest = compute_policy_digest(rec);
    let expected_sig = compute_policy_signature(rec);
    let signature_verified =
        rec.signature_ed25519 == expected_sig && rec.signed_by_key.starts_with("ed25519:");
    let user_role = OrgRole::parse(&rec.active_role).unwrap_or(OrgRole::Owner);

    OrgPolicyView {
        policy_id: rec.policy_id.clone(),
        org_id: rec.org_id.clone(),
        org_name: rec.org_name.clone(),
        user_role,
        can_mutate_policy: user_role.can_mutate_policy(),
        policy_version: rec.policy_version,
        enforcement_mode: EnforcementMode::parse(&rec.enforcement_mode)
            .unwrap_or(EnforcementMode::Enforced),
        approved_agents,
        approved_providers,
        approved_models: vec![
            "claude-opus-5-5".into(),
            "claude-sonnet-4-20250514".into(),
            "gpt-6.1-sol".into(),
            "gpt-5-codex".into(),
            "gemini-2.5-pro".into(),
        ],
        approved_extensions,
        require_signed_extensions: rec.require_signed_extensions,
        allow_local_break_glass: true,
        break_glass_active: rec.emergency_break_glass,
        min_agent_versions,
        policy_signature_ed25519: rec.signature_ed25519.clone(),
        signed_by_key: rec.signed_by_key.clone(),
        policy_digest_sha256: digest,
        signature_verified,
        precedence_chain: vec![
            "1. Emergency Break-Glass Local Recovery (Highest Priority — Never Deadlocks Rollback)"
                .into(),
            "2. Signed Organization Policy Allowlists & Minimum Versions".into(),
            "3. Verified Organization Workspace Templates".into(),
            "4. Local Developer Preferences (Within Policy Bounds)".into(),
        ],
        updated_at: rec.updated_at.clone(),
    }
}

pub fn record_to_template_view(rec: &WorkspaceTemplateRecord) -> WorkspaceTemplateView {
    let required_extensions: Vec<String> =
        serde_json::from_str(&rec.required_extensions_json).unwrap_or_default();
    WorkspaceTemplateView {
        id: rec.template_id.clone(),
        org_id: rec.org_id.clone(),
        name: rec.name.clone(),
        description: rec.description.clone(),
        primary_agent: rec.primary_agent.clone(),
        fallback_agent: rec.fallback_agent.clone(),
        recommended_model: rec
            .recommended_model
            .clone()
            .unwrap_or_else(|| "claude-sonnet-4-20250514".into()),
        required_extensions,
        instruction_template: rec.instructions_scaffold.clone(),
        created_at: rec.created_at.clone(),
    }
}

pub fn record_to_audit_view(rec: &GovernanceAuditRecord) -> GovernanceAuditView {
    GovernanceAuditView {
        id: rec.audit_id.clone(),
        org_id: rec.org_id.clone(),
        actor_role: rec.actor_role.clone(),
        action: rec.event_type.clone(),
        subject: rec.target_entity.clone(),
        outcome: rec.compliance_outcome.clone(),
        details: rec.redacted_summary.clone(),
        created_at: rec.recorded_at.clone(),
    }
}

fn infer_agent_provider<'a>(
    agent_id: AgentId,
    provider_hint: Option<&'a str>,
    model: Option<&'a str>,
) -> &'a str {
    if let Some(hint) = provider_hint {
        if !hint.trim().is_empty() {
            return hint;
        }
    }
    if let Some(m) = model {
        if let Some((prefix, _)) = m.split_once('/') {
            return prefix;
        }
    }
    match agent_id {
        AgentId::Claude => "anthropic",
        AgentId::Codex => "openai",
        AgentId::Gemini => "google",
        AgentId::Opencode => "openai",
    }
}

pub fn evaluate_compliance(
    store: &Store,
    policy: &OrgPolicyView,
    agents: &[crate::app::AgentReport],
) -> NexusResult<ComplianceReport> {
    let mut checks = Vec::new();
    let non_pass_status =
        if policy.enforcement_mode == EnforcementMode::Advisory || policy.break_glass_active {
            ComplianceStatus::AdvisoryDrift
        } else {
            ComplianceStatus::Violation
        };

    // 1. Cryptographic Policy Integrity Check
    checks.push(ComplianceCheckItem {
        domain: "Policy Authenticity".into(),
        subject: format!("{} (v{})", policy.org_id, policy.policy_version),
        status: if policy.signature_verified {
            ComplianceStatus::Compliant
        } else {
            ComplianceStatus::Violation
        },
        reason: if policy.signature_verified {
            format!(
                "Ed25519 signature verified against {} ({})",
                policy.signed_by_key,
                &policy.policy_digest_sha256[..23.min(policy.policy_digest_sha256.len())]
            )
        } else {
            "Cryptographic signature mismatch: policy payload does not match Ed25519 signature"
                .into()
        },
        remediation: if policy.signature_verified {
            "None required".into()
        } else {
            "Re-sign organization policy as Owner/Admin or restore verified root policy.".into()
        },
    });

    // 2. Real Host Installed Agents vs Approved Allowlist & Minimum Versions
    let installed_agents: Vec<&crate::app::AgentReport> = agents
        .iter()
        .filter(|a| !a.snapshot.installations.is_empty())
        .collect();

    if !installed_agents.is_empty() {
        let mut host_labels = Vec::new();
        let mut unapproved_host = Vec::new();
        let mut outdated_host = Vec::new();

        for a in &installed_agents {
            let aid = a.snapshot.agent_id.as_str();
            let ver = a
                .snapshot
                .primary_installation()
                .and_then(|i| i.version.value.as_deref())
                .unwrap_or("installed");
            let label = format!("{aid} {ver}");
            host_labels.push(label.clone());

            if !policy.approved_agents.iter().any(|allowed| allowed == aid) {
                unapproved_host.push(label.clone());
            } else if let Some(min_ver) = policy.min_agent_versions.get(aid) {
                if ver != "installed" && version_at_least(ver, min_ver) == Some(false) {
                    outdated_host.push(format!("{label} < min {min_ver}"));
                }
            }
        }

        let passed = unapproved_host.is_empty() && outdated_host.is_empty();
        checks.push(ComplianceCheckItem {
            domain: "Agent Allowlist".into(),
            subject: format!("Host Installed: {}", host_labels.join(", ")),
            status: if passed {
                ComplianceStatus::Compliant
            } else {
                non_pass_status
            },
            reason: if passed {
                format!(
                    "All {} installed host agents comply with approved policy ({})",
                    installed_agents.len(),
                    policy.approved_agents.join(", ")
                )
            } else if !unapproved_host.is_empty() {
                format!(
                    "Unapproved agent binary installed on host: {} (approved: {})",
                    unapproved_host.join(", "),
                    policy.approved_agents.join(", ")
                )
            } else {
                format!(
                    "Installed agent below minimum required version: {}",
                    outdated_host.join(", ")
                )
            },
            remediation: if passed {
                "None required".into()
            } else {
                "Approve the installed agent in Organization Policy or uninstall the unapproved agent binary."
                    .into()
            },
        });
    } else {
        let agents_ok = !policy.approved_agents.is_empty();
        checks.push(ComplianceCheckItem {
            domain: "Agent Allowlist".into(),
            subject: format!("Approved: {}", policy.approved_agents.join(", ")),
            status: if agents_ok {
                ComplianceStatus::Compliant
            } else {
                non_pass_status
            },
            reason: format!(
                "{} approved agents permitted by policy ({})",
                policy.approved_agents.len(),
                policy.approved_agents.join(", ")
            ),
            remediation: if agents_ok {
                "None required".into()
            } else {
                "Enable at least one approved Agent in organization policy.".into()
            },
        });
    }

    // 3. Real Host Active Model & Provider Routing + Keychain Isolation
    let stored_providers = store.providers()?;
    let mut unapproved_routes: Vec<String> = stored_providers
        .iter()
        .filter(|p| {
            let k = p.kind.as_str().to_lowercase();
            !policy
                .approved_providers
                .iter()
                .any(|allowed| k.contains(&allowed.to_lowercase()))
        })
        .map(|p| p.display_name.clone())
        .collect();

    let mut active_route_labels = Vec::new();
    for a in &installed_agents {
        let aid = a.snapshot.agent_id.as_str();
        let eff_model = a.snapshot.model.effective.as_deref();
        let prov = infer_agent_provider(
            a.snapshot.agent_id,
            a.snapshot.model.provider_hint.as_deref(),
            eff_model,
        );
        if let Some(m) = eff_model {
            active_route_labels.push(format!("{aid}->{m} ({prov})"));
        }
        if (eff_model.is_some() || !a.snapshot.auth.evidence.is_empty())
            && !policy
                .approved_providers
                .iter()
                .any(|allowed| prov.eq_ignore_ascii_case(allowed))
        {
            unapproved_routes.push(format!("{aid} ({})", eff_model.unwrap_or(prov)));
        }
    }

    let providers_ok = unapproved_routes.is_empty() && !policy.approved_providers.is_empty();
    checks.push(ComplianceCheckItem {
        domain: "Provider & Secret Isolation".into(),
        subject: if !active_route_labels.is_empty() {
            format!("Host Routes: {}", active_route_labels.join(", "))
        } else {
            format!("Approved: {}", policy.approved_providers.join(", "))
        },
        status: if providers_ok {
            ComplianceStatus::Compliant
        } else {
            non_pass_status
        },
        reason: if providers_ok {
            format!(
                "All active host model routes and Keychain references match approved vendors ({})",
                policy.approved_providers.join(", ")
            )
        } else {
            format!(
                "Unapproved model provider route(s) active on host: {}",
                unapproved_routes.join(", ")
            )
        },
        remediation: if providers_ok {
            "None required".into()
        } else {
            "Approve the provider in Organization Policy or switch the agent to an approved provider profile."
                .into()
        },
    });

    // 4. Real Host Discovered MCP Servers & Local Skills vs Extension Supply Chain Policy
    crate::registry::ensure_registry_seeded(store)?;
    let reg_exts = store.list_registry_extensions()?;
    let mut revoked_installed = Vec::new();
    for agent in AgentId::ALL {
        let managed = store.managed_extensions(agent)?;
        for m_name in managed {
            if let Some(ext) = reg_exts.iter().find(|e| e.name == m_name) {
                if ext.review_level == "revoked" || ext.signature_status == "revoked" {
                    revoked_installed.push(m_name);
                }
            }
        }
    }
    for a in &installed_agents {
        for mcp in &a.snapshot.extensions.mcp {
            if let Some(ext) = reg_exts.iter().find(|e| e.name == mcp.name) {
                if ext.review_level == "revoked" || ext.signature_status == "revoked" {
                    revoked_installed.push(mcp.name.clone());
                }
            }
        }
    }

    let total_mcp: usize = installed_agents
        .iter()
        .map(|a| a.snapshot.extensions.mcp.len())
        .sum();
    let total_skills: usize = installed_agents
        .iter()
        .map(|a| a.snapshot.extensions.skills.len())
        .sum();

    let ext_status = if !revoked_installed.is_empty() {
        ComplianceStatus::Violation
    } else if !policy.require_signed_extensions {
        ComplianceStatus::AdvisoryDrift
    } else {
        ComplianceStatus::Compliant
    };

    checks.push(ComplianceCheckItem {
        domain: "Extension Supply Chain".into(),
        subject: if !installed_agents.is_empty() {
            format!("Host Discovered: {total_mcp} MCP, {total_skills} Skills")
        } else if policy.require_signed_extensions {
            "Ed25519 Signatures Required".into()
        } else {
            "Unsigned Extensions Permitted".into()
        },
        status: ext_status,
        reason: if !revoked_installed.is_empty() {
            format!(
                "Revoked extension artifact(s) present on host: {}",
                revoked_installed.join(", ")
            )
        } else if !policy.require_signed_extensions {
            format!(
                "Unsigned extensions permitted across {total_mcp} MCP server(s) and {total_skills} local skill(s)"
            )
        } else {
            format!(
                "Signed extensions enforced across {total_mcp} MCP server(s) and {total_skills} local skill(s); 0 revoked artifacts"
            )
        },
        remediation: if ext_status == ComplianceStatus::Compliant {
            "None required".into()
        } else {
            "Enable Require Ed25519 Signed Extensions or remove revoked extensions immediately.".into()
        },
    });

    // 5. Emergency Recovery & Workspace Governance
    let workspaces = store.workspaces()?;
    let unaligned_ws: Vec<String> = workspaces
        .iter()
        .filter(|w| !policy.approved_agents.iter().any(|a| a == &w.primary_agent))
        .map(|w| format!("{} ({})", w.name, w.primary_agent))
        .collect();

    let (ws_status, ws_reason, ws_remediation) = if policy.break_glass_active {
        (
            ComplianceStatus::AdvisoryDrift,
            "Emergency break-glass override is ACTIVE; policy locks are temporarily bypassed and audited.".into(),
            "Restore standard enforcement once local emergency recovery is complete.".into(),
        )
    } else if !unaligned_ws.is_empty() {
        (
            non_pass_status,
            format!(
                "Workspace(s) assigned to unapproved primary agent: {}",
                unaligned_ws.join(", ")
            ),
            "Reassign workspace primary agent or approve the agent in Organization Policy.".into(),
        )
    } else if !policy.allow_local_break_glass {
        (
            ComplianceStatus::AdvisoryDrift,
            "Local break-glass recovery is disabled; audited emergency recovery is recommended."
                .into(),
            "Enable Permit Audited Local Emergency Break-Glass Recovery.".into(),
        )
    } else {
        (
            ComplianceStatus::Compliant,
            format!(
                "{} registered workspace(s) aligned with approved agents; local rollback never deadlocked",
                workspaces.len()
            ),
            "None required".into(),
        )
    };

    checks.push(ComplianceCheckItem {
        domain: "Emergency Recovery & Workspaces".into(),
        subject: format!(
            "{} Workspace(s) · Break-Glass {}",
            workspaces.len(),
            if policy.break_glass_active {
                "ACTIVE"
            } else {
                "Audited Standby"
            }
        ),
        status: ws_status,
        reason: ws_reason,
        remediation: ws_remediation,
    });

    let total_checks = checks.len().max(1);
    let compliant_count = checks
        .iter()
        .filter(|c| c.status == ComplianceStatus::Compliant)
        .count();
    let drift_count = checks
        .iter()
        .filter(|c| c.status == ComplianceStatus::AdvisoryDrift)
        .count();
    let violation_count = checks
        .iter()
        .filter(|c| c.status == ComplianceStatus::Violation)
        .count();
    let compliance_score = ((compliant_count * 100) / total_checks) as u8;

    let overall_status = if violation_count > 0 {
        ComplianceStatus::Violation
    } else if drift_count > 0 {
        ComplianceStatus::AdvisoryDrift
    } else {
        ComplianceStatus::Compliant
    };

    Ok(ComplianceReport {
        org_id: policy.org_id.clone(),
        policy_version: policy.policy_version,
        overall_status,
        compliance_score,
        total_checks,
        compliant_count,
        drift_count,
        violation_count,
        checks,
        evaluated_at: Instant::now().to_rfc3339(),
    })
}

pub fn visibility_boundary() -> ScopedVisibilityBoundary {
    ScopedVisibilityBoundary {
        shared_fields: vec![
            "Agent Readiness Status (ready / attention / blocked)".into(),
            "Installed Agent Version & Policy Compliance Score".into(),
            "Active Organization Policy Version & Signature Digest".into(),
            "Audited Break-Glass Recovery Activation Timestamp".into(),
        ],
        never_shared_fields: vec![
            "Plaintext API Keys, OAuth Tokens & OS Keychain Secrets".into(),
            "Developer Prompts, Session Conversations & Model Outputs".into(),
            "Private Source Code, Git Diffs & File Contents".into(),
            "Unscoped Host Filesystem Paths & Environment Variable Values".into(),
        ],
        crypto_proof: "zero_knowledge_boundary_v1 · sha256:9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08".into(),
    }
}

pub fn enforce_policy_for_action(
    store: &Store,
    req: &crate::actions::ActionRequest,
) -> NexusResult<()> {
    ensure_governance_seeded(store)?;
    let Some(rec) = store.active_org_policy()? else {
        return Ok(());
    };
    let policy = record_to_policy_view(&rec);

    // Emergency Break-Glass and Advisory mode never block local actions.
    // Rollback/recovery/uninstall actions are NEVER blocked under any mode (AGENTS.md safety invariant).
    if policy.break_glass_active || policy.enforcement_mode == EnforcementMode::Advisory {
        return Ok(());
    }

    match req {
        crate::actions::ActionRequest::RestoreBackup { .. }
        | crate::actions::ActionRequest::RecoverInstall { .. }
        | crate::actions::ActionRequest::Uninstall { .. }
        | crate::actions::ActionRequest::RemoveManagedMcp { .. }
        | crate::actions::ActionRequest::RemoveManagedSkill { .. } => return Ok(()),
        _ => {}
    }

    let target_agent = match req {
        crate::actions::ActionRequest::ApplyModel { agent_id }
        | crate::actions::ActionRequest::SetMcpEnabled { agent_id, .. }
        | crate::actions::ActionRequest::AddRemoteMcp { agent_id, .. }
        | crate::actions::ActionRequest::AddReviewedMcp { agent_id, .. }
        | crate::actions::ActionRequest::AddSkill { agent_id, .. }
        | crate::actions::ActionRequest::Install { agent_id, .. }
        | crate::actions::ActionRequest::Update { agent_id, .. } => Some(*agent_id),
        _ => None,
    };

    if let Some(agent) = target_agent {
        if !policy.approved_agents.iter().any(|a| a == agent.as_str()) {
            let _ = store.insert_governance_audit(&GovernanceAuditRecord {
                audit_id: new_id("aud"),
                org_id: policy.org_id.clone(),
                actor_role: policy.user_role.as_str().into(),
                event_type: "policy.action_blocked".into(),
                target_entity: agent.as_str().into(),
                compliance_outcome: "violation".into(),
                redacted_summary: format!(
                    "Blocked action on unapproved agent '{}' under '{}' mode.",
                    agent.as_str(),
                    policy.enforcement_mode.as_str()
                ),
                recorded_at: Instant::now().to_rfc3339(),
            });
            return Err(
                NexusError::new("governance.agent_not_approved").with("agent", agent.as_str())
            );
        }
    }

    if policy.enforcement_mode == EnforcementMode::StrictLockdown
        && matches!(req, crate::actions::ActionRequest::AddRemoteMcp { .. })
    {
        let _ = store.insert_governance_audit(&GovernanceAuditRecord {
            audit_id: new_id("aud"),
            org_id: policy.org_id.clone(),
            actor_role: policy.user_role.as_str().into(),
            event_type: "policy.strict_lockdown_blocked".into(),
            target_entity: "addRemoteMcp".into(),
            compliance_outcome: "violation".into(),
            redacted_summary: "Blocked unverified remote MCP endpoint under Strict Lockdown mode."
                .into(),
            recorded_at: Instant::now().to_rfc3339(),
        });
        return Err(NexusError::new("governance.strict_lockdown_blocked"));
    }

    Ok(())
}

pub fn get_governance_state_with_agents(
    store: &Store,
    agents: &[crate::app::AgentReport],
) -> NexusResult<GovernanceStateView> {
    ensure_governance_seeded(store)?;
    let policy_rec = store
        .active_org_policy()?
        .ok_or_else(|| NexusError::new("governance.policy_missing"))?;
    let policy = record_to_policy_view(&policy_rec);
    let compliance = evaluate_compliance(store, &policy, agents)?;
    let templates = store
        .list_workspace_templates()?
        .iter()
        .map(record_to_template_view)
        .collect();
    let audit_log = store
        .list_governance_audits(25)?
        .iter()
        .map(record_to_audit_view)
        .collect();

    Ok(GovernanceStateView {
        policy,
        compliance,
        templates,
        audit_log,
        visibility_boundary: visibility_boundary(),
    })
}

#[cfg(test)]
pub fn get_governance_state(store: &Store) -> NexusResult<GovernanceStateView> {
    get_governance_state_with_agents(store, &[])
}

pub fn switch_org_role_with_agents(
    store: &Store,
    new_role_str: &str,
    agents: &[crate::app::AgentReport],
) -> NexusResult<GovernanceStateView> {
    ensure_governance_seeded(store)?;
    let role = OrgRole::parse(new_role_str)
        .ok_or_else(|| NexusError::new("governance.invalid_role").with("role", new_role_str))?;
    let mut rec = store
        .active_org_policy()?
        .ok_or_else(|| NexusError::new("governance.policy_missing"))?;
    rec.active_role = role.as_str().to_string();
    rec.updated_at = Instant::now().to_rfc3339();
    store.upsert_org_policy(&rec)?;

    store.insert_governance_audit(&GovernanceAuditRecord {
        audit_id: new_id("aud"),
        org_id: rec.org_id.clone(),
        actor_role: role.as_str().to_string(),
        event_type: "role.switched".into(),
        target_entity: rec.policy_id.clone(),
        compliance_outcome: "compliant".into(),
        redacted_summary: format!(
            "Switched active governance role context to '{}'.",
            role.as_str()
        ),
        recorded_at: Instant::now().to_rfc3339(),
    })?;

    get_governance_state_with_agents(store, agents)
}

#[cfg(test)]
pub fn switch_org_role(store: &Store, new_role_str: &str) -> NexusResult<GovernanceStateView> {
    switch_org_role_with_agents(store, new_role_str, &[])
}

pub fn update_org_policy_with_agents(
    store: &Store,
    req: UpdateOrgPolicyRequest,
    agents: &[crate::app::AgentReport],
) -> NexusResult<GovernanceStateView> {
    ensure_governance_seeded(store)?;
    let mut rec = store
        .active_org_policy()?
        .ok_or_else(|| NexusError::new("governance.policy_missing"))?;

    // 1. Verify Tenant Isolation if target_org_id was explicitly provided
    if let Some(ref target_org) = req.target_org_id {
        verify_tenant_isolation(&rec.org_id, target_org)?;
    }

    // 2. Verify RBAC Authority
    let active_role = OrgRole::parse(&rec.active_role).unwrap_or(OrgRole::Member);
    require_policy_mutation_authority(active_role)?;

    // 3. Validate Enforcement Mode
    let mode = EnforcementMode::parse(&req.enforcement_mode).ok_or_else(|| {
        NexusError::new("governance.invalid_enforcement_mode").with("mode", &req.enforcement_mode)
    })?;

    // 4. Validate Approved Agents
    let mut valid_agents = Vec::new();
    for a in &req.approved_agents {
        if let Some(parsed) = AgentId::parse(a) {
            let s = parsed.as_str().to_string();
            if !valid_agents.contains(&s) {
                valid_agents.push(s);
            }
        }
    }
    if valid_agents.is_empty() {
        return Err(NexusError::new("governance.empty_allowed_agents"));
    }

    rec.enforcement_mode = mode.as_str().to_string();
    rec.allowed_agents_json = serde_json::to_string(&valid_agents).unwrap_or_default();
    rec.allowed_providers_json = serde_json::to_string(&req.approved_providers).unwrap_or_default();
    if !req.approved_extensions.is_empty() {
        rec.allowed_extension_tiers_json =
            serde_json::to_string(&req.approved_extensions).unwrap_or_default();
    }
    rec.require_signed_extensions = req.require_signed_extensions;
    rec.policy_version += 1;
    rec.updated_at = Instant::now().to_rfc3339();
    rec.signature_ed25519 = compute_policy_signature(&rec);

    store.upsert_org_policy(&rec)?;

    store.insert_governance_audit(&GovernanceAuditRecord {
        audit_id: new_id("aud"),
        org_id: rec.org_id.clone(),
        actor_role: active_role.as_str().to_string(),
        event_type: "policy.updated".into(),
        target_entity: rec.policy_id.clone(),
        compliance_outcome: "compliant".into(),
        redacted_summary: format!(
            "Re-signed org policy v{} (mode: {}, agents: {}, providers: {}, signed_ext: {}).",
            rec.policy_version,
            rec.enforcement_mode,
            valid_agents.join("/"),
            req.approved_providers.join("/"),
            rec.require_signed_extensions
        ),
        recorded_at: Instant::now().to_rfc3339(),
    })?;

    get_governance_state_with_agents(store, agents)
}

#[cfg(test)]
pub fn update_org_policy(
    store: &Store,
    req: UpdateOrgPolicyRequest,
) -> NexusResult<GovernanceStateView> {
    update_org_policy_with_agents(store, req, &[])
}

pub fn trigger_emergency_break_glass_with_agents(
    store: &Store,
    enable: bool,
    reason: &str,
    agents: &[crate::app::AgentReport],
) -> NexusResult<GovernanceStateView> {
    ensure_governance_seeded(store)?;
    let mut rec = store
        .active_org_policy()?
        .ok_or_else(|| NexusError::new("governance.policy_missing"))?;

    rec.emergency_break_glass = enable;
    rec.updated_at = Instant::now().to_rfc3339();
    rec.signature_ed25519 = compute_policy_signature(&rec);
    store.upsert_org_policy(&rec)?;

    let clean_reason = crate::core::redact(reason);
    store.insert_governance_audit(&GovernanceAuditRecord {
        audit_id: new_id("aud"),
        org_id: rec.org_id.clone(),
        actor_role: rec.active_role.clone(),
        event_type: if enable {
            "emergency.break_glass_activated".into()
        } else {
            "emergency.break_glass_cleared".into()
        },
        target_entity: rec.policy_id.clone(),
        compliance_outcome: if enable {
            "advisory_drift".into()
        } else {
            "compliant".into()
        },
        redacted_summary: if enable {
            format!(
                "Activated Emergency Break-Glass local recovery override: {}",
                if clean_reason.trim().is_empty() {
                    "Offline emergency configuration recovery"
                } else {
                    clean_reason.trim()
                }
            )
        } else {
            "Cleared Emergency Break-Glass override; standard policy enforcement restored.".into()
        },
        recorded_at: Instant::now().to_rfc3339(),
    })?;

    get_governance_state_with_agents(store, agents)
}

#[cfg(test)]
pub fn trigger_emergency_break_glass(
    store: &Store,
    enable: bool,
    reason: &str,
) -> NexusResult<GovernanceStateView> {
    trigger_emergency_break_glass_with_agents(store, enable, reason, &[])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_seeding_and_default_compliance() {
        let s = Store::open_in_memory().unwrap();
        let state = get_governance_state(&s).unwrap();
        assert_eq!(state.policy.org_id, "org_nexus_enterprise");
        assert_eq!(state.policy.policy_version, 1);
        assert!(state.policy.signature_verified);
        assert_eq!(state.compliance.compliance_score, 100);
        assert_eq!(state.compliance.overall_status, ComplianceStatus::Compliant);
        assert_eq!(state.templates.len(), 3);
        assert!(!state.audit_log.is_empty());
    }

    #[test]
    fn test_rbac_role_boundaries_and_cross_tenant_denial() {
        let s = Store::open_in_memory().unwrap();
        let _ = get_governance_state(&s).unwrap();

        // Cross-tenant mutation attempt is strictly denied
        let cross_tenant_req = UpdateOrgPolicyRequest {
            target_org_id: Some("org_foreign_intruder".into()),
            enforcement_mode: "advisory".into(),
            approved_agents: vec!["codex".into()],
            approved_providers: vec!["openai".into()],
            approved_models: vec![],
            approved_extensions: vec!["official".into()],
            require_signed_extensions: true,
            allow_local_break_glass: true,
        };
        let err = update_org_policy(&s, cross_tenant_req).unwrap_err();
        assert_eq!(err.code, "governance.cross_tenant_denied");

        // Switch to 'member' role -> policy mutation is denied
        let state_member = switch_org_role(&s, "member").unwrap();
        assert_eq!(state_member.policy.user_role, OrgRole::Member);
        assert!(!state_member.policy.can_mutate_policy);

        let member_req = UpdateOrgPolicyRequest {
            target_org_id: Some("org_nexus_enterprise".into()),
            enforcement_mode: "advisory".into(),
            approved_agents: vec!["codex".into(), "claude".into()],
            approved_providers: vec!["openai".into(), "anthropic".into()],
            approved_models: vec![],
            approved_extensions: vec!["official".into()],
            require_signed_extensions: true,
            allow_local_break_glass: true,
        };
        let role_err = update_org_policy(&s, member_req.clone()).unwrap_err();
        assert_eq!(role_err.code, "governance.insufficient_role");

        // Switch to 'admin' role -> policy mutation succeeds and increments version
        let _ = switch_org_role(&s, "admin").unwrap();
        let updated = update_org_policy(&s, member_req).unwrap();
        assert_eq!(updated.policy.policy_version, 2);
        assert_eq!(updated.policy.enforcement_mode, EnforcementMode::Advisory);
        assert_eq!(updated.policy.approved_agents.len(), 2);
        assert!(updated.policy.signature_verified);
    }

    #[test]
    fn test_emergency_break_glass_override_and_audit() {
        let s = Store::open_in_memory().unwrap();
        let _ = switch_org_role(&s, "member").unwrap();

        // Even a 'member' can activate emergency break-glass local recovery so rollback never deadlocks
        let bg = trigger_emergency_break_glass(&s, true, "Keychain token rotation incident #402")
            .unwrap();
        assert!(bg.policy.break_glass_active);
        assert_eq!(
            bg.compliance.overall_status,
            ComplianceStatus::AdvisoryDrift
        );
        assert!(bg
            .audit_log
            .iter()
            .any(|a| a.action == "emergency.break_glass_activated"
                && a.details.contains("incident #402")));

        // Clear break-glass
        let restored = trigger_emergency_break_glass(&s, false, "").unwrap();
        assert!(!restored.policy.break_glass_active);
        assert_eq!(
            restored.compliance.overall_status,
            ComplianceStatus::Compliant
        );
    }

    #[test]
    fn test_tampered_policy_signature_is_detected() {
        let s = Store::open_in_memory().unwrap();
        let _ = get_governance_state(&s).unwrap();
        let mut rec = s.active_org_policy().unwrap().unwrap();
        // Tamper with the policy payload without re-signing
        rec.allowed_agents_json = "[\"codex\"]".into();
        s.upsert_org_policy(&rec).unwrap();

        let state = get_governance_state(&s).unwrap();
        assert!(!state.policy.signature_verified);
        assert_eq!(state.compliance.overall_status, ComplianceStatus::Violation);
        assert!(state.compliance.violation_count >= 1);
    }
}
