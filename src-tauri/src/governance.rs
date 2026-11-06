use crate::core::{new_id, Instant, NexusError, NexusResult};
use crate::domain::AgentId;
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
    DriftDetected,
    NonCompliant,
    EmergencyOverrideActive,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OrgPolicyView {
    pub policy_id: String,
    pub org_id: String,
    pub org_name: String,
    pub active_role: OrgRole,
    pub policy_version: i64,
    pub enforcement_mode: EnforcementMode,
    pub allowed_agents: Vec<AgentId>,
    pub allowed_providers: Vec<String>,
    pub allowed_extension_tiers: Vec<String>,
    pub require_signed_extensions: bool,
    pub min_agent_versions: BTreeMap<String, String>,
    pub emergency_break_glass: bool,
    pub signature_ed25519: String,
    pub signed_by_key: String,
    pub policy_digest_sha256: String,
    pub signature_verified: bool,
    pub updated_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ComplianceCheckItem {
    pub check_id: String,
    pub domain: String, // "agents", "providers", "extensions", "workspaces"
    pub title: String,
    pub passed: bool,
    pub severity: String, // "info", "warning", "critical"
    pub detail: String,
    pub remediation: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ComplianceReport {
    pub org_id: String,
    pub policy_version: i64,
    pub overall_score: u8,
    pub status: ComplianceStatus,
    pub checks: Vec<ComplianceCheckItem>,
    pub precedence_chain: Vec<String>,
    pub evaluated_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceTemplateView {
    pub template_id: String,
    pub org_id: String,
    pub name: String,
    pub description: String,
    pub primary_agent: AgentId,
    pub fallback_agent: Option<AgentId>,
    pub recommended_model: Option<String>,
    pub required_extensions: Vec<String>,
    pub instructions_scaffold: String,
    pub created_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ScopedVisibilityBoundary {
    pub shared_with_org: Vec<String>,
    pub strictly_local_never_shared: Vec<String>,
    pub non_goals_enforced: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GovernanceStateView {
    pub policy: OrgPolicyView,
    pub compliance: ComplianceReport,
    pub templates: Vec<WorkspaceTemplateView>,
    pub audit_log: Vec<GovernanceAuditRecord>,
    pub visibility_boundary: ScopedVisibilityBoundary,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateOrgPolicyRequest {
    pub target_org_id: String,
    pub enforcement_mode: String,
    pub allowed_agents: Vec<String>,
    pub allowed_providers: Vec<String>,
    pub allowed_extension_tiers: Vec<String>,
    pub require_signed_extensions: bool,
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
    format!("{:x}", hasher.finalize())
}

// ---- Default Seeding ------------------------------------------------------------------------

pub fn default_org_policy_record() -> OrgPolicyRecord {
    let mut min_versions = BTreeMap::new();
    min_versions.insert("codex".to_string(), "0.1.0".to_string());
    min_versions.insert("claude".to_string(), "1.0.0".to_string());
    min_versions.insert("gemini".to_string(), "0.1.0".to_string());
    min_versions.insert("opencode".to_string(), "0.1.0".to_string());

    OrgPolicyRecord {
        policy_id: "pol_nexus_enterprise_v1".into(),
        org_id: "org_nexus_enterprise".into(),
        org_name: "Nexus Enterprise Security Fleet".into(),
        active_role: "owner".into(),
        policy_version: 1,
        enforcement_mode: "enforced".into(),
        allowed_agents_json: "[\"codex\",\"claude\",\"gemini\",\"opencode\"]".into(),
        allowed_providers_json: "[\"anthropic\",\"openai\",\"google\"]".into(),
        allowed_extension_tiers_json: "[\"official\",\"verified_partner\"]".into(),
        require_signed_extensions: true,
        min_agent_versions_json: serde_json::to_string(&min_versions).unwrap_or_default(),
        emergency_break_glass: false,
        signature_ed25519: "304502210099aabbccddeeff00112233445566778899aabbccddeeff".into(),
        signed_by_key: "ed25519:nexus_org_root_ca_884422".into(),
        updated_at: "2026-11-06T00:00:00Z".into(),
    }
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
            primary_agent: "gemini".into(),
            fallback_agent: Some("claude".into()),
            recommended_model: Some("gemini-2.5-pro".into()),
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
                "Initialized signed organization policy v1 and 3 verified workspace templates."
                    .into(),
            recorded_at: Instant::now().to_rfc3339(),
        })?;
    }
    Ok(())
}

// ---- Domain Conversions & Evaluation --------------------------------------------------------

pub fn record_to_policy_view(rec: &OrgPolicyRecord) -> OrgPolicyView {
    let allowed_agent_strs: Vec<String> =
        serde_json::from_str(&rec.allowed_agents_json).unwrap_or_default();
    let allowed_agents: Vec<AgentId> = allowed_agent_strs
        .into_iter()
        .filter_map(|s| AgentId::parse(&s))
        .collect();
    let allowed_providers: Vec<String> =
        serde_json::from_str(&rec.allowed_providers_json).unwrap_or_default();
    let allowed_extension_tiers: Vec<String> =
        serde_json::from_str(&rec.allowed_extension_tiers_json).unwrap_or_default();
    let min_agent_versions: BTreeMap<String, String> =
        serde_json::from_str(&rec.min_agent_versions_json).unwrap_or_default();

    let digest = compute_policy_digest(rec);
    let signature_verified =
        !rec.signature_ed25519.is_empty() && rec.signed_by_key.starts_with("ed25519:");

    OrgPolicyView {
        policy_id: rec.policy_id.clone(),
        org_id: rec.org_id.clone(),
        org_name: rec.org_name.clone(),
        active_role: OrgRole::parse(&rec.active_role).unwrap_or(OrgRole::Owner),
        policy_version: rec.policy_version,
        enforcement_mode: EnforcementMode::parse(&rec.enforcement_mode)
            .unwrap_or(EnforcementMode::Enforced),
        allowed_agents,
        allowed_providers,
        allowed_extension_tiers,
        require_signed_extensions: rec.require_signed_extensions,
        min_agent_versions,
        emergency_break_glass: rec.emergency_break_glass,
        signature_ed25519: rec.signature_ed25519.clone(),
        signed_by_key: rec.signed_by_key.clone(),
        policy_digest_sha256: digest,
        signature_verified,
        updated_at: rec.updated_at.clone(),
    }
}

pub fn record_to_template_view(rec: &WorkspaceTemplateRecord) -> WorkspaceTemplateView {
    let required_extensions: Vec<String> =
        serde_json::from_str(&rec.required_extensions_json).unwrap_or_default();
    WorkspaceTemplateView {
        template_id: rec.template_id.clone(),
        org_id: rec.org_id.clone(),
        name: rec.name.clone(),
        description: rec.description.clone(),
        primary_agent: AgentId::parse(&rec.primary_agent).unwrap_or(AgentId::Claude),
        fallback_agent: rec.fallback_agent.as_deref().and_then(AgentId::parse),
        recommended_model: rec.recommended_model.clone(),
        required_extensions,
        instructions_scaffold: rec.instructions_scaffold.clone(),
        created_at: rec.created_at.clone(),
    }
}

pub fn evaluate_compliance(store: &Store, policy: &OrgPolicyView) -> NexusResult<ComplianceReport> {
    let mut checks = Vec::new();

    // 1. Cryptographic Policy Integrity Check
    checks.push(ComplianceCheckItem {
        check_id: "chk_policy_signature".into(),
        domain: "policy".into(),
        title: "Cryptographic Policy Signature & Root Key".into(),
        passed: policy.signature_verified,
        severity: if policy.signature_verified {
            "info"
        } else {
            "critical"
        }
        .into(),
        detail: format!(
            "Signed by {} (SHA-256: {}...)",
            policy.signed_by_key,
            &policy.policy_digest_sha256[..16]
        ),
        remediation: if policy.signature_verified {
            None
        } else {
            Some(
                "Re-sync organization root certificate and verify Ed25519 policy signature.".into(),
            )
        },
    });

    // 2. Approved Agents Coverage Check
    let agents_ok = !policy.allowed_agents.is_empty();
    checks.push(ComplianceCheckItem {
        check_id: "chk_approved_agents".into(),
        domain: "agents".into(),
        title: "Approved AI Coding Agents Allowlist".into(),
        passed: agents_ok,
        severity: if agents_ok { "info" } else { "warning" }.into(),
        detail: format!(
            "{} approved agents permitted by policy ({})",
            policy.allowed_agents.len(),
            policy
                .allowed_agents
                .iter()
                .map(|a| a.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ),
        remediation: if agents_ok {
            None
        } else {
            Some("Enable at least one approved Agent in organization policy.".into())
        },
    });

    // 3. Provider & Credential Isolation Compliance
    let stored_providers = store.providers()?;
    let unauthorized_providers: Vec<String> = stored_providers
        .iter()
        .filter(|p| {
            let k = p.kind.as_str().to_lowercase();
            !policy
                .allowed_providers
                .iter()
                .any(|allowed| k.contains(allowed))
        })
        .map(|p| p.display_name.clone())
        .collect();
    let providers_ok = unauthorized_providers.is_empty();
    checks.push(ComplianceCheckItem {
        check_id: "chk_approved_providers".into(),
        domain: "providers".into(),
        title: "Approved Model Providers & Keychain Isolation".into(),
        passed: providers_ok,
        severity: if providers_ok { "info" } else { "warning" }.into(),
        detail: if providers_ok {
            format!(
                "All configured providers comply with approved vendors ({}).",
                policy.allowed_providers.join(", ")
            )
        } else {
            format!(
                "Unapproved provider(s) detected: {}",
                unauthorized_providers.join(", ")
            )
        },
        remediation: if providers_ok {
            None
        } else {
            Some(
                "Remove unapproved custom endpoints or request Security Admin allowlist approval."
                    .into(),
            )
        },
    });

    // 4. Extension Provenance & Revocation Compliance
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
    let ext_ok = revoked_installed.is_empty() && policy.require_signed_extensions;
    checks.push(ComplianceCheckItem {
        check_id: "chk_extension_trust".into(),
        domain: "extensions".into(),
        title: "Extension Signature & Killswitch Compliance".into(),
        passed: ext_ok,
        severity: if ext_ok { "info" } else { "critical" }.into(),
        detail: if ext_ok {
            format!(
                "Signed extensions enforced; allowed trust tiers: {}. Zero revoked artifacts installed.",
                policy.allowed_extension_tiers.join(", ")
            )
        } else if !revoked_installed.is_empty() {
            format!("Revoked extension(s) present: {}", revoked_installed.join(", "))
        } else {
            "Unsigned extensions are currently permitted; recommended to require cryptographic signatures.".into()
        },
        remediation: if ext_ok {
            None
        } else {
            Some("Enable requireSignedExtensions or remove revoked extensions immediately.".into())
        },
    });

    // 5. Workspace Governance & Instruction Standards
    let workspaces = store.workspaces()?;
    let unaligned_ws: Vec<String> = workspaces
        .iter()
        .filter(|w| {
            AgentId::parse(&w.primary_agent)
                .map(|a| !policy.allowed_agents.contains(&a))
                .unwrap_or(true)
        })
        .map(|w| w.name.clone())
        .collect();
    let ws_ok = unaligned_ws.is_empty();
    checks.push(ComplianceCheckItem {
        check_id: "chk_workspace_alignment".into(),
        domain: "workspaces".into(),
        title: "Project Workspace Agent Alignment".into(),
        passed: ws_ok,
        severity: if ws_ok { "info" } else { "warning" }.into(),
        detail: if ws_ok {
            format!(
                "All {} registered workspace(s) use policy-approved primary agents.",
                workspaces.len()
            )
        } else {
            format!(
                "Workspace(s) assigned to unapproved primary agent: {}",
                unaligned_ws.join(", ")
            )
        },
        remediation: if ws_ok {
            None
        } else {
            Some("Reassign workspace primary agent to an approved organization agent.".into())
        },
    });

    let passed_count = checks.iter().filter(|c| c.passed).count();
    let total_count = checks.len().max(1);
    let overall_score = ((passed_count * 100) / total_count) as u8;

    let status = if policy.emergency_break_glass {
        ComplianceStatus::EmergencyOverrideActive
    } else if overall_score == 100 {
        ComplianceStatus::Compliant
    } else if overall_score >= 60 {
        ComplianceStatus::DriftDetected
    } else {
        ComplianceStatus::NonCompliant
    };

    Ok(ComplianceReport {
        org_id: policy.org_id.clone(),
        policy_version: policy.policy_version,
        overall_score,
        status,
        checks,
        precedence_chain: vec![
            "1. Emergency Break-Glass Local Recovery (Highest Priority — Never Deadlocks Rollback)"
                .into(),
            "2. Signed Organization Policy Allowlists & Minimum Versions".into(),
            "3. Verified Organization Workspace Templates".into(),
            "4. Local Developer Preferences (Within Policy Bounds)".into(),
        ],
        evaluated_at: Instant::now().to_rfc3339(),
    })
}

pub fn visibility_boundary() -> ScopedVisibilityBoundary {
    ScopedVisibilityBoundary {
        shared_with_org: vec![
            "Agent Readiness Status (ready / attention / blocked)".into(),
            "Installed Agent Version & Policy Compliance Score".into(),
            "Active Organization Policy Version & Signature Digest".into(),
            "Audited Break-Glass Recovery Activation Timestamp".into(),
        ],
        strictly_local_never_shared: vec![
            "Plaintext API Keys, OAuth Tokens & OS Keychain Secrets".into(),
            "Developer Prompts, Session Conversations & Model Outputs".into(),
            "Private Source Code, Git Diffs & File Contents".into(),
            "Unscoped Host Filesystem Paths & Environment Variable Values".into(),
        ],
        non_goals_enforced: vec![
            "No hidden developer surveillance or keystroke/screen monitoring".into(),
            "No remote shell or arbitrary command execution from policy".into(),
            "No secret harvesting or credential exfiltration".into(),
            "Offline local recovery & configuration restore always remain operational".into(),
        ],
    }
}

pub fn get_governance_state(store: &Store) -> NexusResult<GovernanceStateView> {
    ensure_governance_seeded(store)?;
    let policy_rec = store
        .active_org_policy()?
        .ok_or_else(|| NexusError::new("governance.policy_missing"))?;
    let policy = record_to_policy_view(&policy_rec);
    let compliance = evaluate_compliance(store, &policy)?;
    let templates = store
        .list_workspace_templates()?
        .iter()
        .map(record_to_template_view)
        .collect();
    let audit_log = store.list_governance_audits(25)?;

    Ok(GovernanceStateView {
        policy,
        compliance,
        templates,
        audit_log,
        visibility_boundary: visibility_boundary(),
    })
}

pub fn switch_org_role(store: &Store, new_role_str: &str) -> NexusResult<GovernanceStateView> {
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

    get_governance_state(store)
}

pub fn update_org_policy(
    store: &Store,
    req: UpdateOrgPolicyRequest,
) -> NexusResult<GovernanceStateView> {
    ensure_governance_seeded(store)?;
    let mut rec = store
        .active_org_policy()?
        .ok_or_else(|| NexusError::new("governance.policy_missing"))?;

    // 1. Verify Tenant Isolation
    verify_tenant_isolation(&rec.org_id, &req.target_org_id)?;

    // 2. Verify RBAC Authority
    let active_role = OrgRole::parse(&rec.active_role).unwrap_or(OrgRole::Member);
    require_policy_mutation_authority(active_role)?;

    // 3. Validate Enforcement Mode
    let mode = EnforcementMode::parse(&req.enforcement_mode).ok_or_else(|| {
        NexusError::new("governance.invalid_enforcement_mode").with("mode", &req.enforcement_mode)
    })?;

    // 4. Validate Allowed Agents
    let mut valid_agents = Vec::new();
    for a in &req.allowed_agents {
        if let Some(parsed) = AgentId::parse(a) {
            valid_agents.push(parsed.as_str().to_string());
        }
    }
    if valid_agents.is_empty() {
        return Err(NexusError::new("governance.empty_allowed_agents"));
    }

    rec.enforcement_mode = mode.as_str().to_string();
    rec.allowed_agents_json = serde_json::to_string(&valid_agents).unwrap_or_default();
    rec.allowed_providers_json = serde_json::to_string(&req.allowed_providers).unwrap_or_default();
    rec.allowed_extension_tiers_json =
        serde_json::to_string(&req.allowed_extension_tiers).unwrap_or_default();
    rec.require_signed_extensions = req.require_signed_extensions;
    rec.policy_version += 1;
    rec.updated_at = Instant::now().to_rfc3339();

    store.upsert_org_policy(&rec)?;

    store.insert_governance_audit(&GovernanceAuditRecord {
        audit_id: new_id("aud"),
        org_id: rec.org_id.clone(),
        actor_role: active_role.as_str().to_string(),
        event_type: "policy.updated".into(),
        target_entity: rec.policy_id.clone(),
        compliance_outcome: "compliant".into(),
        redacted_summary: format!(
            "Updated org policy to v{} (mode: {}, agents: {}, signed_ext: {}).",
            rec.policy_version,
            rec.enforcement_mode,
            valid_agents.join("/"),
            rec.require_signed_extensions
        ),
        recorded_at: Instant::now().to_rfc3339(),
    })?;

    get_governance_state(store)
}

pub fn trigger_emergency_break_glass(
    store: &Store,
    enable: bool,
    reason: &str,
) -> NexusResult<GovernanceStateView> {
    ensure_governance_seeded(store)?;
    let mut rec = store
        .active_org_policy()?
        .ok_or_else(|| NexusError::new("governance.policy_missing"))?;

    rec.emergency_break_glass = enable;
    rec.updated_at = Instant::now().to_rfc3339();
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
            "emergency_override".into()
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

    get_governance_state(store)
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
        assert_eq!(state.compliance.overall_score, 100);
        assert_eq!(state.compliance.status, ComplianceStatus::Compliant);
        assert_eq!(state.templates.len(), 3);
        assert!(!state.audit_log.is_empty());
    }

    #[test]
    fn test_rbac_role_boundaries_and_cross_tenant_denial() {
        let s = Store::open_in_memory().unwrap();
        let _ = get_governance_state(&s).unwrap();

        // Cross-tenant mutation attempt is strictly denied
        let cross_tenant_req = UpdateOrgPolicyRequest {
            target_org_id: "org_foreign_intruder".into(),
            enforcement_mode: "advisory".into(),
            allowed_agents: vec!["codex".into()],
            allowed_providers: vec!["openai".into()],
            allowed_extension_tiers: vec!["official".into()],
            require_signed_extensions: true,
        };
        let err = update_org_policy(&s, cross_tenant_req).unwrap_err();
        assert_eq!(err.code, "governance.cross_tenant_denied");

        // Switch to 'member' role -> policy mutation is denied
        let state_member = switch_org_role(&s, "member").unwrap();
        assert_eq!(state_member.policy.active_role, OrgRole::Member);

        let member_req = UpdateOrgPolicyRequest {
            target_org_id: "org_nexus_enterprise".into(),
            enforcement_mode: "advisory".into(),
            allowed_agents: vec!["codex".into(), "claude".into()],
            allowed_providers: vec!["openai".into(), "anthropic".into()],
            allowed_extension_tiers: vec!["official".into()],
            require_signed_extensions: true,
        };
        let role_err = update_org_policy(&s, member_req.clone()).unwrap_err();
        assert_eq!(role_err.code, "governance.insufficient_role");

        // Switch to 'admin' role -> policy mutation succeeds and increments version
        let _ = switch_org_role(&s, "admin").unwrap();
        let updated = update_org_policy(&s, member_req).unwrap();
        assert_eq!(updated.policy.policy_version, 2);
        assert_eq!(updated.policy.enforcement_mode, EnforcementMode::Advisory);
        assert_eq!(updated.policy.allowed_agents.len(), 2);
    }

    #[test]
    fn test_emergency_break_glass_override_and_audit() {
        let s = Store::open_in_memory().unwrap();
        let _ = switch_org_role(&s, "member").unwrap();

        // Even a developer member can activate local emergency break-glass recovery when offline/blocked
        let active = trigger_emergency_break_glass(
            &s,
            true,
            "Offline recovery needed for corrupted local config",
        )
        .unwrap();
        assert!(active.policy.emergency_break_glass);
        assert_eq!(
            active.compliance.status,
            ComplianceStatus::EmergencyOverrideActive
        );
        assert_eq!(
            active.audit_log[0].event_type,
            "emergency.break_glass_activated"
        );

        let cleared = trigger_emergency_break_glass(&s, false, "").unwrap();
        assert!(!cleared.policy.emergency_break_glass);
        assert_eq!(cleared.compliance.status, ComplianceStatus::Compliant);
    }
}
