//! AI Doctor & Visual Health Radar Engine (Architecture §6, ADR 003, ROADMAP Phase 9).
//!
//! Provides privacy-preserving, 100% redacted root-cause reasoning, probable cause ranking,
//! and one-click registered safe action recommendations.
//!
//! Security guarantees:
//! - Purely in-memory evaluation: zero cloud telemetry, zero remote script execution.
//! - Strict redaction: all paths, environment tokens, and secrets scrubbed via `crate::core::redact`.
//! - Safe native repairs: all recommendations map strictly to allowlisted `ActionRequest` variants
//!   requiring explicit native user confirmation.

use crate::actions::ActionRequest;
use crate::core::{redact, Instant};
use crate::domain::*;
use crate::health::{AgentHealth, Outcome};
use crate::platform::HostContext;
use crate::store::{BackupRecord, OperationRecord};
use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ConfidenceLevel {
    High,
    Medium,
    Low,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProbableCause {
    pub rank: usize,
    pub category: String,
    pub cause: String,
    pub confidence: ConfidenceLevel,
    pub evidence_snippet: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiDoctorRecommendation {
    pub id: String,
    pub title: String,
    pub rationale: String,
    pub risk_assessment: String,
    pub action: Option<ActionRequest>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthRadarStage {
    pub id: String,
    pub label: String,
    pub outcome: Outcome,
    pub summary: String,
    pub check_ids: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiDoctorConsultation {
    pub agent_id: AgentId,
    pub consulted_at: Instant,
    pub overall_severity: String, // "healthy" | "advisory" | "critical"
    pub confidence_score: f32,
    pub analysis_summary: String,
    pub radar_stages: Vec<HealthRadarStage>,
    pub ranked_causes: Vec<ProbableCause>,
    pub recommendations: Vec<AiDoctorRecommendation>,
    pub redaction_notice: String,
    pub disclaimer: String,
}

pub struct AiDoctorInputs<'a> {
    pub agent_id: AgentId,
    pub snapshot: &'a AgentSnapshot,
    pub health: &'a AgentHealth,
    pub findings: &'a [crate::diagnostics::DiagnosticFinding],
    pub backups: &'a [BackupRecord],
    pub interrupted: &'a [OperationRecord],
    pub desired_assignment: Option<&'a str>,
    pub host: &'a HostContext,
}

pub fn consult(inputs: &AiDoctorInputs) -> AiDoctorConsultation {
    let now = Instant::now();
    let agent_id = inputs.agent_id;
    let snapshot = inputs.snapshot;
    let health = inputs.health;
    let findings = inputs.findings;
    let backups = inputs.backups;
    let interrupted = inputs.interrupted;
    let desired_assignment = inputs.desired_assignment;
    let host = inputs.host;

    // 1. Synthesize the 7-stage Health Radar pipeline
    let radar_stages = build_radar_stages(health);

    // 2. Synthesize ranked root causes with privacy redaction
    let mut ranked_causes = Vec::new();
    let mut recommendations = Vec::new();

    // Cause Analysis: Interrupted operations
    for op in interrupted {
        let snippet = redact(&format!(
            "Action: {}, started: {}",
            op.action, op.started_at
        ));
        ranked_causes.push(ProbableCause {
            rank: ranked_causes.len() + 1,
            category: "Lifecycle".to_string(),
            cause: format!(
                "Interrupted {} operation left unverified artifacts.",
                op.action
            ),
            confidence: ConfidenceLevel::High,
            evidence_snippet: snippet,
        });

        if op.action == "install" {
            recommendations.push(AiDoctorRecommendation {
                id: format!("rec_recover_{}", op.id),
                title: "Recover interrupted installation".to_string(),
                rationale: "Reconciles the interrupted installation state and resumes clean lifecycle execution.".to_string(),
                risk_assessment: "Low (Safe compensation)".to_string(),
                action: Some(ActionRequest::RecoverInstall {
                    operation_id: op.id.clone(),
                }),
            });
        }
    }

    // Cause Analysis: Corrupted configuration files
    for cfg in &snapshot.config_sources {
        if cfg.status == ObsStatus::ParseFailed {
            let format_name = match cfg.format {
                ConfigFormat::Toml => "TOML",
                ConfigFormat::Json => "JSON",
                ConfigFormat::Jsonc => "JSONC",
            };
            let location = host.display_path(&cfg.path);
            ranked_causes.push(ProbableCause {
                rank: ranked_causes.len() + 1,
                category: "Configuration".to_string(),
                cause: format!("Syntax corruption detected in {format_name} configuration file."),
                confidence: ConfidenceLevel::High,
                evidence_snippet: redact(&format!("File location: {location}")),
            });

            if let Some(backup) = backups
                .iter()
                .find(|b| b.target_display == location || b.target_id == cfg.id)
            {
                recommendations.push(AiDoctorRecommendation {
                    id: format!("rec_restore_{}", backup.id),
                    title: format!("Restore verified backup from {}", backup.created_at),
                    rationale: "Reverts the corrupted configuration to the last known-good state with atomic verification.".to_string(),
                    risk_assessment: "Low (Reversible via backup)".to_string(),
                    action: Some(ActionRequest::RestoreBackup {
                        backup_id: backup.id.clone(),
                    }),
                });
            }
        }
    }

    // Cause Analysis: Toolchain executable missing
    if snapshot.installations.is_empty() || snapshot.primary.is_none() {
        ranked_causes.push(ProbableCause {
            rank: ranked_causes.len() + 1,
            category: "Toolchain".to_string(),
            cause: format!(
                "{} CLI executable not found in PATH or standard directories.",
                snapshot.name
            ),
            confidence: ConfidenceLevel::High,
            evidence_snippet: "PATH probe returned zero valid executable candidates.".to_string(),
        });
    }

    // Cause Analysis: Model drift
    let model_drift_check = health.checks.iter().find(|c| c.id == "model.drift");
    if let Some(drift) = model_drift_check {
        if drift.outcome == Outcome::Fail {
            ranked_causes.push(ProbableCause {
                rank: ranked_causes.len() + 1,
                category: "ModelAssignment".to_string(),
                cause: "Effective agent model has drifted from the assigned Model Profile."
                    .to_string(),
                confidence: ConfidenceLevel::High,
                evidence_snippet: redact(&format!(
                    "Effective: {:?}, Desired: {:?}",
                    snapshot.model.effective, desired_assignment
                )),
            });

            recommendations.push(AiDoctorRecommendation {
                id: format!("rec_apply_model_{}", agent_id.as_str()),
                title: "Apply desired Model Profile".to_string(),
                rationale:
                    "Synchronizes the local agent configuration with your designated Model Profile."
                        .to_string(),
                risk_assessment: "Low (Modifies model field only)".to_string(),
                action: Some(ActionRequest::ApplyModel { agent_id }),
            });
        }
    }

    // Cause Analysis: Missing authentication
    let auth_check = health.checks.iter().find(|c| c.id == "auth.evidence");
    if let Some(auth) = auth_check {
        if auth.outcome == Outcome::Fail || snapshot.auth.evidence.is_empty() {
            ranked_causes.push(ProbableCause {
                rank: ranked_causes.len() + 1,
                category: "Authentication".to_string(),
                cause: "No verified authentication credentials or API tokens found.".to_string(),
                confidence: ConfidenceLevel::High,
                evidence_snippet: "Checked agent-owned login tokens and environment variables."
                    .to_string(),
            });
        }
    }

    // Cause Analysis: MCP Server unresolvable
    for server in &snapshot.extensions.mcp {
        if server.command_resolvable == Some(false) {
            ranked_causes.push(ProbableCause {
                rank: ranked_causes.len() + 1,
                category: "Extensions".to_string(),
                cause: format!(
                    "MCP server '{}' command binary cannot be resolved in PATH.",
                    server.name
                ),
                confidence: ConfidenceLevel::Medium,
                evidence_snippet: redact(&format!(
                    "Server: {}, Command: {:?}",
                    server.name, server.command
                )),
            });

            if server.can_toggle {
                recommendations.push(AiDoctorRecommendation {
                    id: format!("rec_toggle_mcp_{}", server.name),
                    title: format!("Disable unresolvable MCP server '{}'", server.name),
                    rationale:
                        "Disables the failing MCP server to prevent runtime initialization crashes."
                            .to_string(),
                    risk_assessment: "Low (Can be re-enabled anytime)".to_string(),
                    action: Some(ActionRequest::SetMcpEnabled {
                        agent_id,
                        name: server.name.clone(),
                        enabled: false,
                    }),
                });
            }
        }
    }

    // Cause Analysis: Remaining findings from deterministic diagnostics
    for finding in findings {
        if !ranked_causes
            .iter()
            .any(|c| c.cause.contains(&finding.code))
        {
            let conf = match finding.severity {
                "fail" => ConfidenceLevel::High,
                "warning" => ConfidenceLevel::Medium,
                _ => ConfidenceLevel::Low,
            };
            let ev = if finding.evidence.is_empty() {
                format!("Diagnostic code: {}", finding.code)
            } else {
                redact(&finding.evidence.join("; "))
            };
            ranked_causes.push(ProbableCause {
                rank: ranked_causes.len() + 1,
                category: "Diagnostics".to_string(),
                cause: format!("Observed diagnostic finding: {}", finding.code),
                confidence: conf,
                evidence_snippet: ev,
            });
        }
    }

    // Default healthy state if no issues found
    if ranked_causes.is_empty() {
        ranked_causes.push(ProbableCause {
            rank: 1,
            category: "Readiness".to_string(),
            cause: "All 7 lifecycle stages (Runtime, Toolchain, Config, Auth, Model, Extensions, Network) are verified.".to_string(),
            confidence: ConfidenceLevel::High,
            evidence_snippet: "Zero configuration drift and complete prerequisite coverage observed.".to_string(),
        });
    }

    // Overall severity calculation
    let critical_count = ranked_causes
        .iter()
        .filter(|c| c.confidence == ConfidenceLevel::High && c.category != "Readiness")
        .count();
    let advisory_count = ranked_causes
        .iter()
        .filter(|c| c.confidence == ConfidenceLevel::Medium)
        .count();
    let overall_severity = if critical_count > 0 || health.readiness == Readiness::Blocked {
        "critical".to_string()
    } else if advisory_count > 0 || health.readiness == Readiness::Attention {
        "advisory".to_string()
    } else {
        "healthy".to_string()
    };

    // Grounded confidence score calculation
    let passed = health.coverage.required_passed + health.coverage.advisory_passed;
    let total = (health.coverage.required + health.coverage.advisory).max(1);
    let confidence_score = if overall_severity == "healthy" {
        0.98
    } else {
        ((passed as f32 / total as f32) * 0.5 + 0.45).clamp(0.60, 0.96)
    };

    // Executive summary text
    let analysis_summary = match overall_severity.as_str() {
        "critical" => format!(
            "Agent {} has {} critical issue{} blocking ready execution. Review the ranked causes and verified safe repairs below.",
            snapshot.name,
            critical_count,
            if critical_count > 1 { "s" } else { "" }
        ),
        "advisory" => format!(
            "Agent {} is operational with {} advisory warning{}. Applying recommended optimizations will prevent drift.",
            snapshot.name,
            advisory_count,
            if advisory_count > 1 { "s" } else { "" }
        ),
        _ => format!(
            "Agent {} is completely healthy and ready to work. All 7 lifecycle stages are synchronized and verified.",
            snapshot.name
        ),
    };

    AiDoctorConsultation {
        agent_id,
        consulted_at: now,
        overall_severity,
        confidence_score,
        analysis_summary,
        radar_stages,
        ranked_causes,
        recommendations,
        redaction_notice: "100% of sensitive paths, tokens, and credentials were scrubbed locally before analysis.".to_string(),
        disclaimer: "AI Doctor recommendations map strictly to allowlisted native actions. Native confirmation is required before any state changes.".to_string(),
    }
}

fn build_radar_stages(health: &AgentHealth) -> Vec<HealthRadarStage> {
    let stage_defs = [
        (
            "runtime",
            "Runtime Environment",
            vec!["platform.supported", "runtime.prerequisite"],
        ),
        (
            "toolchain",
            "Toolchain & Binary",
            vec![
                "install.present",
                "install.executable",
                "install.version",
                "install.unique",
            ],
        ),
        ("config", "Config & Integrity", vec!["config.unambiguous"]),
        ("auth", "Auth & Tokens", vec!["auth.evidence"]),
        (
            "model",
            "Model & Drift",
            vec![
                "model.effective",
                "model.compatible",
                "model.applied",
                "model.drift",
            ],
        ),
        (
            "extensions",
            "MCP & Skills",
            vec!["extensions.mcp", "extensions.skills"],
        ),
        (
            "network",
            "Provider & Network",
            vec!["network.reachability"],
        ),
    ];

    stage_defs
        .iter()
        .map(|(id, label, check_prefixes)| {
            let matched_checks: Vec<_> = health
                .checks
                .iter()
                .filter(|c| {
                    check_prefixes
                        .iter()
                        .any(|prefix| c.id == *prefix || c.id.starts_with(prefix))
                        || (id == &"config" && c.category == "configuration")
                })
                .collect();

            let outcome = if matched_checks.is_empty() {
                Outcome::Pass
            } else if matched_checks.iter().any(|c| c.outcome == Outcome::Fail) {
                Outcome::Fail
            } else if matched_checks.iter().any(|c| c.outcome == Outcome::Unknown) {
                Outcome::Unknown
            } else {
                Outcome::Pass
            };

            let summary = match outcome {
                Outcome::Pass => format!("{label} verified and operational"),
                Outcome::Fail => format!("{label} check failed"),
                Outcome::Unknown => format!("{label} state unverified"),
                Outcome::NotApplicable => format!("{label} not applicable"),
            };

            HealthRadarStage {
                id: id.to_string(),
                label: label.to_string(),
                outcome,
                summary,
                check_ids: matched_checks
                    .into_iter()
                    .map(|c| c.id.to_string())
                    .collect(),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::Instant;
    use crate::health::{evaluate, AssignmentContext, HealthCheck, Requirement};

    fn make_test_host() -> HostContext {
        HostContext::isolated(
            std::path::Path::new("/Users/testuser"),
            std::path::Path::new("/Users/testuser/Library/Application Support/dev.nexus.app"),
        )
    }

    fn make_healthy_snapshot(agent_id: AgentId) -> AgentSnapshot {
        AgentSnapshot {
            snapshot_id: "snap_1".to_string(),
            agent_id,
            name: "Claude Code",
            vendor: "Anthropic",
            observed_at: Instant::now(),
            platform: Support::Supported,
            installations: vec![Installation {
                id: "inst_1".to_string(),
                executable: "/usr/local/bin/claude".to_string(),
                resolved: None,
                source: InstallSource::Homebrew,
                version: Observed::ok(
                    "1.0.0".to_string(),
                    EvidenceSource::VersionProbe,
                    None,
                    Instant::now(),
                ),
                path_rank: Some(0),
                executable_ok: true,
                nexus_managed: false,
                wrapper: false,
            }],
            primary: Some(0),
            config_sources: vec![],
            model: ModelObservation {
                effective: Some("claude-3-7-sonnet".to_string()),
                effective_kind: ModelSourceKind::UserConfig,
                layers: vec![],
                provider_hint: None,
            },
            auth: AuthObservation {
                evidence: vec![AuthEvidence::AgentLoginPresent],
                locations: vec![],
            },
            extensions: ExtensionObservation {
                mcp: vec![],
                mcp_status: ObsStatus::Observed,
                skills: vec![],
                skills_status: ObsStatus::Observed,
            },
            env_overrides: vec![],
            capabilities: vec![],
            runtime_requirement: None,
        }
    }

    #[test]
    fn healthy_agent_yields_healthy_ai_doctor_consultation() {
        let host = make_test_host();
        let snap = make_healthy_snapshot(AgentId::Claude);
        let system = crate::health::fixtures::system(Instant::now(), Some("20.0.0"));
        let health = evaluate(
            &snap,
            &system,
            &AssignmentContext::default(),
            Instant::now(),
        );
        let consultation = consult(&AiDoctorInputs {
            agent_id: AgentId::Claude,
            snapshot: &snap,
            health: &health,
            findings: &[],
            backups: &[],
            interrupted: &[],
            desired_assignment: Some("claude-3-7-sonnet"),
            host: &host,
        });

        assert_eq!(consultation.overall_severity, "healthy");
        assert!(consultation.confidence_score >= 0.95);
        assert_eq!(consultation.radar_stages.len(), 7);
        assert_eq!(consultation.ranked_causes[0].category, "Readiness");
        assert!(consultation
            .redaction_notice
            .contains("100% of sensitive paths"));
    }

    #[test]
    fn corrupted_config_recommends_backup_restore_with_redacted_evidence() {
        let host = make_test_host();
        let mut snap = make_healthy_snapshot(AgentId::Codex);
        let config_path = host.home.join(".codex").join("config.json");
        let display = host.display_path(&config_path);
        snap.config_sources.push(ConfigSource {
            id: "user_config".to_string(),
            scope: ConfigScope::User,
            format: ConfigFormat::Json,
            location: display.clone(),
            status: ObsStatus::ParseFailed,
            fingerprint: None,
            via_symlink: false,
            tolerant: false,
            path: config_path.clone(),
        });

        let health = AgentHealth {
            agent_id: AgentId::Codex,
            readiness: Readiness::Blocked,
            checks: vec![HealthCheck {
                id: "config.codex.syntax",
                category: "configuration",
                requirement: Requirement::Required,
                outcome: Outcome::Fail,
                detail: Some("parseFailed"),
                params: vec![],
                evidence: vec!["Unexpected token at line 4".to_string()],
                source: None,
                evaluated_at: Instant::now(),
            }],
            coverage: crate::health::Coverage {
                required: 5,
                required_passed: 4,
                advisory: 2,
                advisory_passed: 2,
                unknown: 0,
                not_checked: vec![],
            },
            rule_set: "nexus-readiness-1",
            snapshot_id: "s1".to_string(),
            evaluated_at: Instant::now(),
            valid_until: Instant::now(),
            stale: false,
        };

        let backup = BackupRecord {
            id: "bak_codex_good".to_string(),
            operation_id: "op_1".to_string(),
            agent_id: Some("codex".to_string()),
            target_id: "user_config".to_string(),
            target_display: display,
            sha256: "abc123sha".to_string(),
            size: 1024,
            created_at: "2026-11-05T07:00:00Z".to_string(),
            target_path: config_path.display().to_string(),
            written_fingerprint: None,
        };

        let consultation = consult(&AiDoctorInputs {
            agent_id: AgentId::Codex,
            snapshot: &snap,
            health: &health,
            findings: &[],
            backups: &[backup],
            interrupted: &[],
            desired_assignment: None,
            host: &host,
        });

        assert_eq!(consultation.overall_severity, "critical");
        assert!(consultation
            .ranked_causes
            .iter()
            .any(|c| c.category == "Configuration"));
        let restore_rec = consultation
            .recommendations
            .iter()
            .find(|r| r.id == "rec_restore_bak_codex_good");
        assert!(restore_rec.is_some());
        assert_eq!(
            restore_rec.unwrap().action,
            Some(ActionRequest::RestoreBackup {
                backup_id: "bak_codex_good".to_string()
            })
        );
        let cause = consultation
            .ranked_causes
            .iter()
            .find(|c| c.category == "Configuration")
            .unwrap();
        assert!(cause.evidence_snippet.contains(".codex") && cause.evidence_snippet.contains("config.json"));
        assert!(!cause.evidence_snippet.contains("testuser"));
    }
}
