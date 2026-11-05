//! Developer Account & Entitlement Engine (Phase 11).
//!
//! Project Nexus adheres strictly to the offline-first, local-first philosophy (PRD §8, Architecture §6).
//! Developer identity and cloud synchronization are strictly OPTIONAL convenience capabilities.
//! Discovery, Agent Health evaluation, safe repair actions, rollbacks, and security updates remain
//! 100% operational offline without an account, and are NEVER paywalled.
//!
//! Account tiers:
//! - `Free`: Local Core, 7-day backup retention, offline diagnostics & workspaces.
//! - `Pro`: Cross-device sync mesh, 90-day retention, priority diagnostic insights.
//! - `Team`: Organization sync, 365-day retention, template governance.

use crate::core::{Instant, NexusError, NexusResult};
use crate::store::{AccountRecord, Store};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AccountTier {
    Free,
    Pro,
    Team,
}

impl AccountTier {
    pub fn as_str(&self) -> &'static str {
        match self {
            AccountTier::Free => "free",
            AccountTier::Pro => "pro",
            AccountTier::Team => "team",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "pro" => AccountTier::Pro,
            "team" => AccountTier::Team,
            _ => AccountTier::Free,
        }
    }

    pub fn backup_retention_days(&self) -> u32 {
        match self {
            AccountTier::Free => 7,
            AccountTier::Pro => 90,
            AccountTier::Team => 365,
        }
    }

    pub fn max_retained_operations(&self) -> usize {
        match self {
            AccountTier::Free => 10,
            AccountTier::Pro => 1000,
            AccountTier::Team => 10000,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AccountProfile {
    pub id: String,
    pub email: String,
    pub display_name: String,
    pub tier: AccountTier,
    pub avatar_url: Option<String>,
    pub created_at: String,
    pub expires_at: Option<String>,
    pub backup_retention_days: u32,
    pub cloud_sync_enabled: bool,
    pub is_offline_only: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EntitlementStatus {
    pub tier: AccountTier,
    pub is_valid: bool,
    pub features: Vec<String>,
    pub retention_days: u32,
    pub max_operations: usize,
    pub safety_guarantee: String,
}

pub fn default_offline_account() -> AccountProfile {
    AccountProfile {
        id: "acc_offline_local".into(),
        email: "developer@local.mesh".into(),
        display_name: "Local Developer".into(),
        tier: AccountTier::Free,
        avatar_url: None,
        created_at: "2026-10-27T00:00:00Z".into(),
        expires_at: None,
        backup_retention_days: AccountTier::Free.backup_retention_days(),
        cloud_sync_enabled: false,
        is_offline_only: true,
    }
}

pub fn record_to_profile(rec: &AccountRecord) -> AccountProfile {
    let tier = AccountTier::parse(&rec.tier);
    AccountProfile {
        id: rec.id.clone(),
        email: rec.email.clone(),
        display_name: rec.display_name.clone(),
        tier,
        avatar_url: rec.avatar_url.clone(),
        created_at: rec.created_at.clone(),
        expires_at: rec.expires_at.clone(),
        backup_retention_days: tier.backup_retention_days(),
        cloud_sync_enabled: matches!(tier, AccountTier::Pro | AccountTier::Team),
        is_offline_only: false,
    }
}

pub fn profile_to_record(profile: &AccountProfile) -> AccountRecord {
    AccountRecord {
        id: profile.id.clone(),
        email: profile.email.clone(),
        display_name: profile.display_name.clone(),
        tier: profile.tier.as_str().to_string(),
        avatar_url: profile.avatar_url.clone(),
        created_at: profile.created_at.clone(),
        expires_at: profile.expires_at.clone(),
        is_active: true,
    }
}

pub fn evaluate_entitlements(profile: &AccountProfile) -> EntitlementStatus {
    let mut features = vec![
        "deterministic_health".to_string(),
        "safe_repair_catalog".to_string(),
        "unrestricted_rollback".to_string(),
        "offline_doctor_insights".to_string(),
        "local_workspaces".to_string(),
    ];

    match profile.tier {
        AccountTier::Free => {}
        AccountTier::Pro => {
            features.push("cloud_encrypted_sync".to_string());
            features.push("extended_backup_retention_90d".to_string());
            features.push("cross_device_workspace_mesh".to_string());
            features.push("priority_issue_diagnostics".to_string());
        }
        AccountTier::Team => {
            features.push("cloud_encrypted_sync".to_string());
            features.push("extended_backup_retention_365d".to_string());
            features.push("cross_device_workspace_mesh".to_string());
            features.push("priority_issue_diagnostics".to_string());
            features.push("team_workspace_templates".to_string());
            features.push("central_governance_policy".to_string());
        }
    }

    EntitlementStatus {
        tier: profile.tier,
        is_valid: true,
        features,
        retention_days: profile.backup_retention_days,
        max_operations: profile.tier.max_retained_operations(),
        safety_guarantee: "Local discovery, Agent health, safe repairs, and rollbacks are 100% offline and never paywalled.".into(),
    }
}

pub fn export_account_data_bundle(store: &Store, profile: &AccountProfile) -> NexusResult<String> {
    let devices = store.list_sync_devices()?;
    let envelopes = store.list_sync_envelopes()?;
    let workspaces = store.workspaces()?;

    let bundle = serde_json::json!({
        "nexusExportVersion": "1.0.0",
        "exportedAt": Instant::now().to_rfc3339(),
        "account": profile,
        "entitlements": evaluate_entitlements(profile),
        "syncMesh": {
            "deviceCount": devices.len(),
            "devices": devices,
            "envelopeCount": envelopes.len(),
            "envelopes": envelopes.iter().map(|e| {
                serde_json::json!({
                    "envelopeId": e.envelope_id,
                    "entityType": e.entity_type,
                    "entityId": e.entity_id,
                    "version": e.entity_version,
                    "checksumSha256": e.checksum_sha256,
                    "syncedAt": e.synced_at,
                })
            }).collect::<Vec<_>>(),
        },
        "workspaces": workspaces,
        "privacyNotice": "No plaintext authentication tokens, credentials, or keys are included in this export.",
    });

    serde_json::to_string_pretty(&bundle).map_err(|_| NexusError::new("account.export_failed"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_account_is_offline_and_free() {
        let acc = default_offline_account();
        assert_eq!(acc.tier, AccountTier::Free);
        assert!(acc.is_offline_only);
        assert!(!acc.cloud_sync_enabled);
        assert_eq!(acc.backup_retention_days, 7);

        let ent = evaluate_entitlements(&acc);
        assert!(ent.is_valid);
        assert_eq!(ent.retention_days, 7);
        assert!(ent.safety_guarantee.contains("100% offline"));
        assert!(!ent.features.contains(&"cloud_encrypted_sync".to_string()));
    }

    #[test]
    fn pro_tier_grants_extended_retention_and_sync() {
        let rec = AccountRecord {
            id: "acc_pro".into(),
            email: "pro@dev.io".into(),
            display_name: "Pro Developer".into(),
            tier: "pro".into(),
            avatar_url: None,
            created_at: "2026-10-27T00:00:00Z".into(),
            expires_at: Some("2027-10-27T00:00:00Z".into()),
            is_active: true,
        };
        let profile = record_to_profile(&rec);
        assert_eq!(profile.tier, AccountTier::Pro);
        assert!(profile.cloud_sync_enabled);
        assert_eq!(profile.backup_retention_days, 90);

        let ent = evaluate_entitlements(&profile);
        assert_eq!(ent.max_operations, 1000);
        assert!(ent.features.contains(&"cloud_encrypted_sync".to_string()));
        assert!(ent
            .features
            .contains(&"extended_backup_retention_90d".to_string()));
    }

    #[test]
    fn export_bundle_has_provenance_and_no_credentials() {
        let s = Store::open_in_memory().unwrap();
        let acc = default_offline_account();
        let json_str = export_account_data_bundle(&s, &acc).unwrap();
        assert!(json_str.contains("nexusExportVersion"));
        assert!(json_str.contains("privacyNotice"));
        let lower = json_str.to_lowercase();
        assert!(!lower.contains("secret_value"));
        assert!(!lower.contains("bearer"));
    }
}
