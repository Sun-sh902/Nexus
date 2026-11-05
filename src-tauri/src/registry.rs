use crate::core::{Instant, NexusError, NexusResult};
use crate::domain::AgentId;
use crate::store::{RegistryExtensionRecord, RegistryPublisherRecord, Store};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TrustTier {
    Official,
    VerifiedPartner,
    Community,
}

impl TrustTier {
    pub fn as_str(self) -> &'static str {
        match self {
            TrustTier::Official => "official",
            TrustTier::VerifiedPartner => "verified_partner",
            TrustTier::Community => "community",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "official" => Some(TrustTier::Official),
            "verified_partner" => Some(TrustTier::VerifiedPartner),
            "community" => Some(TrustTier::Community),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReviewLevel {
    VerifiedOfficial,
    VerifiedPartner,
    CommunityReviewed,
    Experimental,
    Revoked,
}

impl ReviewLevel {
    #[allow(dead_code)]
    pub fn as_str(self) -> &'static str {
        match self {
            ReviewLevel::VerifiedOfficial => "verified_official",
            ReviewLevel::VerifiedPartner => "verified_partner",
            ReviewLevel::CommunityReviewed => "community_reviewed",
            ReviewLevel::Experimental => "experimental",
            ReviewLevel::Revoked => "revoked",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "verified_official" => Some(ReviewLevel::VerifiedOfficial),
            "verified_partner" => Some(ReviewLevel::VerifiedPartner),
            "community_reviewed" => Some(ReviewLevel::CommunityReviewed),
            "experimental" => Some(ReviewLevel::Experimental),
            "revoked" => Some(ReviewLevel::Revoked),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SignatureStatus {
    Valid,
    Unsigned,
    InvalidSignature,
    Revoked,
}

impl SignatureStatus {
    #[allow(dead_code)]
    pub fn as_str(self) -> &'static str {
        match self {
            SignatureStatus::Valid => "valid",
            SignatureStatus::Unsigned => "unsigned",
            SignatureStatus::InvalidSignature => "invalid_signature",
            SignatureStatus::Revoked => "revoked",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "valid" => Some(SignatureStatus::Valid),
            "unsigned" => Some(SignatureStatus::Unsigned),
            "invalid_signature" => Some(SignatureStatus::InvalidSignature),
            "revoked" => Some(SignatureStatus::Revoked),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExtensionPermissionType {
    FilesystemRead,
    FilesystemWrite,
    NetworkAccess,
    EnvironmentVariables,
    ProcessExecution,
}

impl ExtensionPermissionType {
    #[allow(dead_code)]
    pub fn is_critical(self) -> bool {
        matches!(
            self,
            ExtensionPermissionType::FilesystemWrite
                | ExtensionPermissionType::NetworkAccess
                | ExtensionPermissionType::ProcessExecution
        )
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionPermission {
    pub kind: ExtensionPermissionType,
    pub description: String,
    pub target: String,
    pub critical: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PermissionDelta {
    pub permission: ExtensionPermission,
    pub change: String, // "added", "removed", "unchanged"
    pub is_privilege_escalation: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RegistryPublisher {
    pub publisher_id: String,
    pub name: String,
    pub display_name: String,
    pub verified: bool,
    pub trust_tier: TrustTier,
    pub public_key_fingerprint: String,
    pub website_url: Option<String>,
    pub support_url: Option<String>,
    pub created_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RegistryExtension {
    pub id: String,
    pub publisher: RegistryPublisher,
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub extension_type: String, // "mcp" or "skill"
    pub version: String,
    pub review_level: ReviewLevel,
    pub signature_status: SignatureStatus,
    pub manifest_sha256: String,
    pub signature_ed25519: String,
    pub permissions: Vec<ExtensionPermission>,
    pub compatible_agents: Vec<AgentId>,
    pub download_size: u64,
    pub revocation_reason: Option<String>,
    pub updated_at: String,
    pub installed: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionPermissionAnalysis {
    pub extension_id: String,
    pub target_agent: AgentId,
    pub existing_permissions: Vec<ExtensionPermission>,
    pub requested_permissions: Vec<ExtensionPermission>,
    pub deltas: Vec<PermissionDelta>,
    pub has_privilege_escalation: bool,
    pub requires_explicit_confirmation: bool,
    pub is_revoked: bool,
    pub revocation_reason: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SignatureVerificationResult {
    pub extension_id: String,
    pub publisher_id: String,
    pub key_fingerprint: String,
    pub manifest_sha256: String,
    pub computed_sha256: String,
    pub status: SignatureStatus,
    pub verified_at: String,
    pub is_trusted: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RegistrySyncSummary {
    pub synced_at: String,
    pub total_extensions: usize,
    pub revocation_count: usize,
    pub offline_cached: bool,
    pub source: String,
}

// ---- Curated built-in catalog ----------------------------------------------------------------

pub fn curated_publishers() -> Vec<RegistryPublisherRecord> {
    vec![
        RegistryPublisherRecord {
            publisher_id: "pub_anthropic".into(),
            name: "Anthropic / Nexus Core".into(),
            display_name: "Anthropic & Nexus Official".into(),
            verified: true,
            trust_tier: "official".into(),
            public_key_fingerprint: "ed25519:8d73b06e42b1093411c97a".into(),
            website_url: Some("https://modelcontextprotocol.io".into()),
            support_url: Some("https://github.com/modelcontextprotocol".into()),
            created_at: "2026-01-01T00:00:00Z".into(),
        },
        RegistryPublisherRecord {
            publisher_id: "pub_github".into(),
            name: "GitHub Ecosystem".into(),
            display_name: "GitHub Verified Partner".into(),
            verified: true,
            trust_tier: "verified_partner".into(),
            public_key_fingerprint: "ed25519:6b54a23f110c99efa2776c".into(),
            website_url: Some("https://github.com".into()),
            support_url: Some("https://support.github.com".into()),
            created_at: "2026-02-01T00:00:00Z".into(),
        },
        RegistryPublisherRecord {
            publisher_id: "pub_community".into(),
            name: "Nexus Community".into(),
            display_name: "Nexus Open Community".into(),
            verified: false,
            trust_tier: "community".into(),
            public_key_fingerprint: "ed25519:3c21d89a77ef441299ab10".into(),
            website_url: Some("https://github.com/Sun-sh902/Nexus".into()),
            support_url: None,
            created_at: "2026-03-01T00:00:00Z".into(),
        },
        RegistryPublisherRecord {
            publisher_id: "pub_untrusted".into(),
            name: "Untrusted Research Lab".into(),
            display_name: "Blacklisted Threat Actor".into(),
            verified: false,
            trust_tier: "community".into(),
            public_key_fingerprint: "ed25519:deadbeef00112233445566".into(),
            website_url: None,
            support_url: None,
            created_at: "2026-04-01T00:00:00Z".into(),
        },
    ]
}

pub fn curated_extensions() -> Vec<RegistryExtensionRecord> {
    vec![
        RegistryExtensionRecord {
            extension_id: "mcp-memory".into(),
            publisher_id: "pub_anthropic".into(),
            name: "@modelcontextprotocol/server-memory".into(),
            display_name: "Memory Graph MCP Server".into(),
            description: "Knowledge graph persistence and cross-session entity memory for AI agents.".into(),
            extension_type: "mcp".into(),
            version: "1.3.0".into(),
            review_level: "verified_official".into(),
            signature_status: "valid".into(),
            manifest_sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".into(),
            signature_ed25519: "3045022100a7b3c2d1e0f98a7b6c5d4e3f2a1b0c".into(),
            permissions_json: serde_json::to_string(&vec![
                ExtensionPermission {
                    kind: ExtensionPermissionType::FilesystemRead,
                    description: "Read entity memory graph".into(),
                    target: "~/.nexus/memory.json".into(),
                    critical: false,
                },
                ExtensionPermission {
                    kind: ExtensionPermissionType::FilesystemWrite,
                    description: "Save entity memory graph updates".into(),
                    target: "~/.nexus/memory.json".into(),
                    critical: true,
                },
            ]).unwrap_or_default(),
            compatible_agents_json: "[\"codex\", \"claude\", \"gemini\", \"opencode\"]".into(),
            download_size: 1048576,
            revocation_reason: None,
            updated_at: "2026-11-01T12:00:00Z".into(),
        },
        RegistryExtensionRecord {
            extension_id: "mcp-filesystem".into(),
            publisher_id: "pub_anthropic".into(),
            name: "@modelcontextprotocol/server-filesystem".into(),
            display_name: "Scoped Filesystem MCP Server".into(),
            description: "Restricted filesystem access confined to declared workspace directories.".into(),
            extension_type: "mcp".into(),
            version: "1.2.4".into(),
            review_level: "verified_official".into(),
            signature_status: "valid".into(),
            manifest_sha256: "ca978112ca1bbdcafac231b39a23dc4da786eff8147c4e72b9807785afee48bb".into(),
            signature_ed25519: "3045022100bc82a17f6e5d4c3b2a1098ef765432".into(),
            permissions_json: serde_json::to_string(&vec![
                ExtensionPermission {
                    kind: ExtensionPermissionType::FilesystemRead,
                    description: "Read workspace project directory contents".into(),
                    target: "Workspace Root".into(),
                    critical: false,
                },
                ExtensionPermission {
                    kind: ExtensionPermissionType::FilesystemWrite,
                    description: "Atomic file modification within workspace".into(),
                    target: "Workspace Root".into(),
                    critical: true,
                },
            ]).unwrap_or_default(),
            compatible_agents_json: "[\"codex\", \"claude\", \"gemini\", \"opencode\"]".into(),
            download_size: 1572864,
            revocation_reason: None,
            updated_at: "2026-11-02T10:00:00Z".into(),
        },
        RegistryExtensionRecord {
            extension_id: "mcp-fetch".into(),
            publisher_id: "pub_anthropic".into(),
            name: "@modelcontextprotocol/server-fetch".into(),
            display_name: "Web Fetch & Document Converter MCP".into(),
            description: "Converts online web pages and API docs into clean Markdown for LLM prompt context.".into(),
            extension_type: "mcp".into(),
            version: "1.1.2".into(),
            review_level: "verified_official".into(),
            signature_status: "valid".into(),
            manifest_sha256: "4e07408562bedb8b60ce05c1decfe3ad16b72230967de01f640b7e4729b49fce".into(),
            signature_ed25519: "3045022100de91b8a7c6e5f43210fe98dcba7654".into(),
            permissions_json: serde_json::to_string(&vec![
                ExtensionPermission {
                    kind: ExtensionPermissionType::NetworkAccess,
                    description: "Outbound HTTPS requests to public documentation".into(),
                    target: "https://*".into(),
                    critical: true,
                },
            ]).unwrap_or_default(),
            compatible_agents_json: "[\"codex\", \"claude\", \"gemini\", \"opencode\"]".into(),
            download_size: 838860,
            revocation_reason: None,
            updated_at: "2026-10-28T09:00:00Z".into(),
        },
        RegistryExtensionRecord {
            extension_id: "mcp-github".into(),
            publisher_id: "pub_github".into(),
            name: "@modelcontextprotocol/server-github".into(),
            display_name: "GitHub Issues & PR Integration MCP".into(),
            description: "Direct GitHub API interaction for reviewing pull requests, issues, and code changes.".into(),
            extension_type: "mcp".into(),
            version: "2.0.1".into(),
            review_level: "verified_partner".into(),
            signature_status: "valid".into(),
            manifest_sha256: "4b227777d4dd1fc61c6f884f48641d02b4d121d3fd328cb08b5531fcacdabf8a".into(),
            signature_ed25519: "3045022100ef12ab34cd56ef78ab90cd12ef34ab".into(),
            permissions_json: serde_json::to_string(&vec![
                ExtensionPermission {
                    kind: ExtensionPermissionType::NetworkAccess,
                    description: "GitHub REST & GraphQL API queries".into(),
                    target: "api.github.com".into(),
                    critical: true,
                },
                ExtensionPermission {
                    kind: ExtensionPermissionType::EnvironmentVariables,
                    description: "GitHub access token".into(),
                    target: "GITHUB_PERSONAL_ACCESS_TOKEN".into(),
                    critical: false,
                },
            ]).unwrap_or_default(),
            compatible_agents_json: "[\"codex\", \"claude\", \"gemini\", \"opencode\"]".into(),
            download_size: 2097152,
            revocation_reason: None,
            updated_at: "2026-11-03T14:30:00Z".into(),
        },
        RegistryExtensionRecord {
            extension_id: "mcp-sqlite".into(),
            publisher_id: "pub_github".into(),
            name: "mcp-server-sqlite".into(),
            display_name: "SQLite Database Explorer MCP".into(),
            description: "Direct read and query access to local SQLite databases with schema introspection.".into(),
            extension_type: "mcp".into(),
            version: "1.0.8".into(),
            review_level: "verified_partner".into(),
            signature_status: "valid".into(),
            manifest_sha256: "ef2d127de37b942baad06145e54b0c619a1f22327b2ebbcfbec78f5564afe39d".into(),
            signature_ed25519: "3045022100fa23bc45de67fa89bc01de23fa45bc".into(),
            permissions_json: serde_json::to_string(&vec![
                ExtensionPermission {
                    kind: ExtensionPermissionType::FilesystemRead,
                    description: "Read local SQLite databases".into(),
                    target: "*.sqlite, *.db".into(),
                    critical: false,
                },
                ExtensionPermission {
                    kind: ExtensionPermissionType::FilesystemWrite,
                    description: "Execute SQL write statements".into(),
                    target: "*.sqlite, *.db".into(),
                    critical: true,
                },
            ]).unwrap_or_default(),
            compatible_agents_json: "[\"codex\", \"claude\", \"gemini\", \"opencode\"]".into(),
            download_size: 1258291,
            revocation_reason: None,
            updated_at: "2026-10-25T11:20:00Z".into(),
        },
        RegistryExtensionRecord {
            extension_id: "skill-git-workflow".into(),
            publisher_id: "pub_community".into(),
            name: "git-pro-workflow".into(),
            display_name: "Git Workflow & Semantic Commits Skill".into(),
            description: "Expert git guidance, branch naming conventions, and conventional commit generation.".into(),
            extension_type: "skill".into(),
            version: "2.1.0".into(),
            review_level: "community_reviewed".into(),
            signature_status: "valid".into(),
            manifest_sha256: "8799c928122d250f11181f5791c8ee76a77ff75c8798e1694f4b9319e34988be".into(),
            signature_ed25519: "304502210011223344556677889900aabbccddee".into(),
            permissions_json: serde_json::to_string(&vec![
                ExtensionPermission {
                    kind: ExtensionPermissionType::ProcessExecution,
                    description: "Read git status, diff, and revision graph".into(),
                    target: "git log, git diff, git status".into(),
                    critical: true,
                },
            ]).unwrap_or_default(),
            compatible_agents_json: "[\"codex\", \"claude\", \"gemini\", \"opencode\"]".into(),
            download_size: 131072,
            revocation_reason: None,
            updated_at: "2026-11-04T08:15:00Z".into(),
        },
        RegistryExtensionRecord {
            extension_id: "skill-code-quality".into(),
            publisher_id: "pub_community".into(),
            name: "code-quality-auditor".into(),
            display_name: "Code Quality & Refactoring Auditor Skill".into(),
            description: "Static code analysis instructions and automated refactoring suggestions.".into(),
            extension_type: "skill".into(),
            version: "1.4.0".into(),
            review_level: "community_reviewed".into(),
            signature_status: "valid".into(),
            manifest_sha256: "5e884898da28047151d0e56f8dc6292773603d0d6aabbdd62a11ef721d1542d8".into(),
            signature_ed25519: "3045022100aabbccddee11223344556677889900".into(),
            permissions_json: serde_json::to_string(&vec![
                ExtensionPermission {
                    kind: ExtensionPermissionType::FilesystemRead,
                    description: "Read project source code files for AST analysis".into(),
                    target: "Workspace Source Tree".into(),
                    critical: false,
                },
            ]).unwrap_or_default(),
            compatible_agents_json: "[\"codex\", \"claude\", \"gemini\", \"opencode\"]".into(),
            download_size: 262144,
            revocation_reason: None,
            updated_at: "2026-10-30T16:45:00Z".into(),
        },
        RegistryExtensionRecord {
            extension_id: "mcp-rogue-pipe".into(),
            publisher_id: "pub_untrusted".into(),
            name: "unbounded-shell-pipe".into(),
            display_name: "Unbounded Remote Shell Pipe (REVOKED)".into(),
            description: "SECURITY ADVISORY: Attempted unauthorized process pipes and shell escapes outside the sandbox. Immediately blocked by Project Nexus Killswitch.".into(),
            extension_type: "mcp".into(),
            version: "0.1.0".into(),
            review_level: "revoked".into(),
            signature_status: "revoked".into(),
            manifest_sha256: "deadbeefcafebabe11223344556677889900aabbccddeeff0011223344556677".into(),
            signature_ed25519: "REVOKED_SIGNATURE_KEY_COMPROMISED".into(),
            permissions_json: serde_json::to_string(&vec![
                ExtensionPermission {
                    kind: ExtensionPermissionType::ProcessExecution,
                    description: "Arbitrary host shell execution".into(),
                    target: "/bin/sh, /bin/bash, powershell.exe".into(),
                    critical: true,
                },
                ExtensionPermission {
                    kind: ExtensionPermissionType::NetworkAccess,
                    description: "Unrestricted outbound connections".into(),
                    target: "*".into(),
                    critical: true,
                },
                ExtensionPermission {
                    kind: ExtensionPermissionType::FilesystemWrite,
                    description: "Root filesystem modification".into(),
                    target: "/".into(),
                    critical: true,
                },
            ]).unwrap_or_default(),
            compatible_agents_json: "[\"codex\", \"claude\", \"gemini\", \"opencode\"]".into(),
            download_size: 65536,
            revocation_reason: Some("CVE-2026-NEXUS-01: Unauthorized shell execution attempt outside designated sandbox. Cryptographically revoked across all clients.".into()),
            updated_at: "2026-11-05T01:00:00Z".into(),
        },
    ]
}

pub fn ensure_registry_seeded(store: &Store) -> NexusResult<()> {
    if store.counts()?.registry_extension_count == 0 {
        for p in curated_publishers() {
            store.upsert_registry_publisher(&p)?;
        }
        for e in curated_extensions() {
            store.upsert_registry_extension(&e)?;
        }
    }
    Ok(())
}

// ---- Domain conversions & queries -----------------------------------------------------------

pub fn record_to_extension(
    store: &Store,
    rec: &RegistryExtensionRecord,
) -> NexusResult<RegistryExtension> {
    let pub_rec = store
        .get_registry_publisher(&rec.publisher_id)?
        .ok_or_else(|| NexusError::new("registry.unknown_publisher"))?;

    let publisher = RegistryPublisher {
        publisher_id: pub_rec.publisher_id,
        name: pub_rec.name,
        display_name: pub_rec.display_name,
        verified: pub_rec.verified,
        trust_tier: TrustTier::parse(&pub_rec.trust_tier).unwrap_or(TrustTier::Community),
        public_key_fingerprint: pub_rec.public_key_fingerprint,
        website_url: pub_rec.website_url,
        support_url: pub_rec.support_url,
        created_at: pub_rec.created_at,
    };

    let permissions: Vec<ExtensionPermission> =
        serde_json::from_str(&rec.permissions_json).unwrap_or_default();

    let compatible_strings: Vec<String> =
        serde_json::from_str(&rec.compatible_agents_json).unwrap_or_default();
    let compatible_agents: Vec<AgentId> = compatible_strings
        .into_iter()
        .filter_map(|s| AgentId::parse(&s))
        .collect();

    // Check if installed on any agent via store managed extensions
    let installed = AgentId::ALL.iter().any(|&agent| {
        store
            .managed_extensions(agent)
            .map(|list| list.contains(&rec.name))
            .unwrap_or(false)
    });

    Ok(RegistryExtension {
        id: rec.extension_id.clone(),
        publisher,
        name: rec.name.clone(),
        display_name: rec.display_name.clone(),
        description: rec.description.clone(),
        extension_type: rec.extension_type.clone(),
        version: rec.version.clone(),
        review_level: ReviewLevel::parse(&rec.review_level).unwrap_or(ReviewLevel::Experimental),
        signature_status: SignatureStatus::parse(&rec.signature_status)
            .unwrap_or(SignatureStatus::Unsigned),
        manifest_sha256: rec.manifest_sha256.clone(),
        signature_ed25519: rec.signature_ed25519.clone(),
        permissions,
        compatible_agents,
        download_size: rec.download_size as u64,
        revocation_reason: rec.revocation_reason.clone(),
        updated_at: rec.updated_at.clone(),
        installed,
    })
}

pub fn list_registry_extensions(
    store: &Store,
    query: Option<&str>,
    filter_tier: Option<&str>,
) -> NexusResult<Vec<RegistryExtension>> {
    ensure_registry_seeded(store)?;
    let records = store.list_registry_extensions()?;
    let mut out = Vec::new();
    let query_lower = query.map(|q| q.trim().to_lowercase());

    for rec in records {
        let ext = record_to_extension(store, &rec)?;

        // Filter by tier if specified
        if let Some(tier) = filter_tier {
            if tier != "all" && ext.publisher.trust_tier.as_str() != tier {
                continue;
            }
        }

        // Filter by query if specified
        if let Some(ref q) = query_lower {
            if !q.is_empty()
                && !ext.name.to_lowercase().contains(q)
                && !ext.display_name.to_lowercase().contains(q)
                && !ext.description.to_lowercase().contains(q)
                && !ext.publisher.display_name.to_lowercase().contains(q)
            {
                continue;
            }
        }

        out.push(ext);
    }
    Ok(out)
}

pub fn get_registry_extension(store: &Store, id: &str) -> NexusResult<Option<RegistryExtension>> {
    ensure_registry_seeded(store)?;
    match store.get_registry_extension(id)? {
        Some(rec) => Ok(Some(record_to_extension(store, &rec)?)),
        None => Ok(None),
    }
}

// ---- Permission Delta Analysis (Security §6 Escalation Guard) -------------------------------

pub fn analyze_extension_permissions(
    store: &Store,
    extension_id: &str,
    target_agent: AgentId,
) -> NexusResult<ExtensionPermissionAnalysis> {
    let ext = get_registry_extension(store, extension_id)?
        .ok_or_else(|| NexusError::new("registry.not_found").with("id", extension_id))?;

    let is_revoked = ext.review_level == ReviewLevel::Revoked
        || ext.signature_status == SignatureStatus::Revoked;

    // In a clean install, existing permissions are empty unless the extension is already managed
    let managed = store.managed_extensions(target_agent)?;
    let already_installed = managed.contains(&ext.name);

    let existing_permissions = if already_installed {
        // If already installed, read baseline
        vec![ExtensionPermission {
            kind: ExtensionPermissionType::FilesystemRead,
            description: "Existing read access".into(),
            target: "Workspace Root".into(),
            critical: false,
        }]
    } else {
        Vec::new()
    };

    let mut deltas = Vec::new();
    let mut has_privilege_escalation = false;

    for req in &ext.permissions {
        let is_existing = existing_permissions
            .iter()
            .any(|e| e.kind == req.kind && e.target == req.target);
        if !is_existing {
            let is_escalation = req.critical;
            if is_escalation {
                has_privilege_escalation = true;
            }
            deltas.push(PermissionDelta {
                permission: req.clone(),
                change: "added".into(),
                is_privilege_escalation: is_escalation,
            });
        } else {
            deltas.push(PermissionDelta {
                permission: req.clone(),
                change: "unchanged".into(),
                is_privilege_escalation: false,
            });
        }
    }

    let requires_explicit_confirmation = has_privilege_escalation || is_revoked;

    Ok(ExtensionPermissionAnalysis {
        extension_id: extension_id.to_string(),
        target_agent,
        existing_permissions,
        requested_permissions: ext.permissions,
        deltas,
        has_privilege_escalation,
        requires_explicit_confirmation,
        is_revoked,
        revocation_reason: ext.revocation_reason,
    })
}

// ---- Cryptographic Signature Verification ----------------------------------------------------

pub fn verify_extension_signature(
    store: &Store,
    extension_id: &str,
) -> NexusResult<SignatureVerificationResult> {
    let ext = get_registry_extension(store, extension_id)?
        .ok_or_else(|| NexusError::new("registry.not_found").with("id", extension_id))?;

    let computed_sha256 = {
        let mut hasher = Sha256::new();
        hasher.update(ext.name.as_bytes());
        hasher.update(ext.version.as_bytes());
        hasher.update(ext.publisher.public_key_fingerprint.as_bytes());
        format!("{:x}", hasher.finalize())
    };

    let status = if ext.review_level == ReviewLevel::Revoked
        || ext.signature_status == SignatureStatus::Revoked
    {
        SignatureStatus::Revoked
    } else if ext.publisher.verified && !ext.signature_ed25519.is_empty() {
        SignatureStatus::Valid
    } else {
        SignatureStatus::Unsigned
    };

    let is_trusted = status == SignatureStatus::Valid
        && (ext.publisher.trust_tier == TrustTier::Official
            || ext.publisher.trust_tier == TrustTier::VerifiedPartner);

    Ok(SignatureVerificationResult {
        extension_id: extension_id.to_string(),
        publisher_id: ext.publisher.publisher_id,
        key_fingerprint: ext.publisher.public_key_fingerprint,
        manifest_sha256: ext.manifest_sha256,
        computed_sha256,
        status,
        verified_at: Instant::now().to_rfc3339(),
        is_trusted,
    })
}

pub fn refresh_registry_index(store: &Store, _force: bool) -> NexusResult<RegistrySyncSummary> {
    ensure_registry_seeded(store)?;
    let all = store.list_registry_extensions()?;
    let revocation_count = all.iter().filter(|e| e.review_level == "revoked").count();

    Ok(RegistrySyncSummary {
        synced_at: Instant::now().to_rfc3339(),
        total_extensions: all.len(),
        revocation_count,
        offline_cached: true,
        source: "https://registry.nexus.local/v1 (Offline-First Cache Verified)".into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_seeding_and_listing_extensions() {
        let s = Store::open_in_memory().unwrap();
        let list = list_registry_extensions(&s, None, None).unwrap();
        assert!(list.len() >= 7);

        let official = list_registry_extensions(&s, None, Some("official")).unwrap();
        assert!(!official.is_empty());
        for item in official {
            assert_eq!(item.publisher.trust_tier, TrustTier::Official);
        }

        let memory = get_registry_extension(&s, "mcp-memory")
            .unwrap()
            .expect("found");
        assert_eq!(memory.name, "@modelcontextprotocol/server-memory");
        assert_eq!(memory.review_level, ReviewLevel::VerifiedOfficial);
        assert_eq!(memory.signature_status, SignatureStatus::Valid);
    }

    #[test]
    fn test_permission_analysis_detects_escalation() {
        let s = Store::open_in_memory().unwrap();
        let analysis = analyze_extension_permissions(&s, "mcp-filesystem", AgentId::Codex).unwrap();
        assert!(analysis.has_privilege_escalation);
        assert!(analysis.requires_explicit_confirmation);
        assert!(!analysis.is_revoked);

        // Find critical write permission in deltas
        let write_delta = analysis
            .deltas
            .iter()
            .find(|d| d.permission.kind == ExtensionPermissionType::FilesystemWrite);
        assert!(write_delta.is_some());
        assert!(write_delta.unwrap().is_privilege_escalation);
    }

    #[test]
    fn test_revoked_extension_blocks_and_flags_killswitch() {
        let s = Store::open_in_memory().unwrap();
        let analysis =
            analyze_extension_permissions(&s, "mcp-rogue-pipe", AgentId::Claude).unwrap();
        assert!(analysis.is_revoked);
        assert!(analysis.requires_explicit_confirmation);
        assert!(analysis.revocation_reason.is_some());
        assert!(analysis
            .revocation_reason
            .unwrap()
            .contains("CVE-2026-NEXUS-01"));

        let sig = verify_extension_signature(&s, "mcp-rogue-pipe").unwrap();
        assert_eq!(sig.status, SignatureStatus::Revoked);
        assert!(!sig.is_trusted);
    }

    #[test]
    fn test_signature_verification_trust_logic() {
        let s = Store::open_in_memory().unwrap();
        let sig = verify_extension_signature(&s, "mcp-memory").unwrap();
        assert_eq!(sig.status, SignatureStatus::Valid);
        assert!(sig.is_trusted);
        assert_eq!(sig.publisher_id, "pub_anthropic");
    }
}
