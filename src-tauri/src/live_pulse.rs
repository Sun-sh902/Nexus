//! Real-Time Host Process, Memory, Configuration SHA-256 Ledger & Cross-Agent Skill Matrix.
//!
//! Performs a fast, bounded, read-only inspection of:
//! 1. Active local Agent and MCP server processes (`/bin/ps` on Unix, strictly redacted to
//!    executable basenames only — never exposing command-line arguments or environment variables).
//! 2. Real configuration file SHA-256 digests, byte sizes, line counts, and parse status on disk.
//! 3. Cross-Agent Skill Coverage Matrix across Claude Code, OpenAI Codex, Gemini CLI, and OpenCode,
//!    enabling 1-click skill teleportation across agents via the `AddSkill` action plan.

use crate::app::AgentReport;
use crate::core::{sha256_hex, Instant};
use crate::domain::{AgentId, ObsStatus};
use crate::platform::{HostContext, Os};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;
use std::process::{Command, Stdio};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HostProcessItem {
    pub pid: u32,
    pub agent_id: Option<AgentId>,
    /// Redacted executable basename only (never arguments or environment).
    pub command: String,
    pub cpu_percent: f32,
    pub memory_mb: f32,
    pub elapsed: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ConfigFilePulse {
    pub agent_id: AgentId,
    pub label: String,
    pub location: String,
    pub exists: bool,
    pub size_bytes: u64,
    pub line_count: usize,
    pub sha256: Option<String>,
    pub status: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CrossAgentSkillItem {
    pub name: String,
    pub description: Option<String>,
    pub source_agent: AgentId,
    pub source_location: String,
    pub has_scripts: bool,
    pub installed_on: Vec<AgentId>,
    pub missing_on: Vec<AgentId>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LivePulseReport {
    pub scanned_at: String,
    pub active_process_count: usize,
    pub total_memory_mb: f32,
    pub processes: Vec<HostProcessItem>,
    pub config_ledger: Vec<ConfigFilePulse>,
    pub skill_matrix: Vec<CrossAgentSkillItem>,
}

/// Classifies a process executable basename into an optional `AgentId` or MCP server helper.
/// Returns `Some((agent_id_opt, canonical_basename))` if the process belongs to the local AI agent
/// ecosystem, or `None` for unrelated host processes.
pub fn classify_process_basename(raw_comm: &str) -> Option<(Option<AgentId>, String)> {
    let trimmed = raw_comm.trim();
    if trimmed.is_empty() {
        return None;
    }
    let base = Path::new(trimmed)
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_else(|| trimmed.to_string());
    let lower = base.to_ascii_lowercase();

    match lower.as_str() {
        "claude" | "claude-code" => Some((Some(AgentId::Claude), base)),
        "codex" | "codex-cli" => Some((Some(AgentId::Codex), base)),
        "gemini" | "gemini-cli" => Some((Some(AgentId::Gemini), base)),
        "opencode" => Some((Some(AgentId::Opencode), base)),
        _ if lower == "mcp"
            || lower.starts_with("mcp-")
            || lower.starts_with("mcp_")
            || lower.ends_with("-mcp") =>
        {
            Some((None, base))
        }
        _ => None,
    }
}

/// Parses output from `/bin/ps -axo pid=,pcpu=,rss=,etime=,comm=`.
pub fn parse_ps_output(raw: &str) -> Vec<HostProcessItem> {
    let mut items = Vec::new();
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let mut parts = trimmed.split_whitespace();
        let Some(pid_str) = parts.next() else {
            continue;
        };
        let Some(cpu_str) = parts.next() else {
            continue;
        };
        let Some(rss_str) = parts.next() else {
            continue;
        };
        let Some(etime_str) = parts.next() else {
            continue;
        };
        let comm_rest: Vec<&str> = parts.collect();
        if comm_rest.is_empty() {
            continue;
        }
        let comm_joined = comm_rest.join(" ");
        let Ok(pid) = pid_str.parse::<u32>() else {
            continue;
        };
        let cpu_percent = cpu_str.parse::<f32>().unwrap_or(0.0);
        let rss_kb = rss_str.parse::<u64>().unwrap_or(0);
        let memory_mb = (((rss_kb as f32) / 1024.0) * 10.0).round() / 10.0;

        if let Some((agent_id, command)) = classify_process_basename(&comm_joined) {
            items.push(HostProcessItem {
                pid,
                agent_id,
                command,
                cpu_percent,
                memory_mb,
                elapsed: etime_str.to_string(),
            });
        }
    }
    items.sort_by(|a, b| {
        b.memory_mb
            .partial_cmp(&a.memory_mb)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.pid.cmp(&b.pid))
    });
    items
}

fn scan_host_processes(ctx: &HostContext) -> Vec<HostProcessItem> {
    if ctx.os == Os::Macos || ctx.os == Os::Linux {
        let ps_path = Path::new("/bin/ps");
        if ps_path.exists() {
            if let Ok(out) = Command::new(ps_path)
                .args(["-axo", "pid=,pcpu=,rss=,etime=,comm="])
                .env_clear()
                .env("LC_ALL", "C")
                .current_dir(&ctx.home)
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .output()
            {
                if out.status.success() {
                    let raw = String::from_utf8_lossy(&out.stdout);
                    return parse_ps_output(&raw);
                }
            }
        }
    }
    Vec::new()
}

fn status_label(status: ObsStatus) -> &'static str {
    match status {
        ObsStatus::Observed => "observed",
        ObsStatus::NotFound => "notFound",
        ObsStatus::AccessDenied => "accessDenied",
        ObsStatus::Timeout => "timeout",
        ObsStatus::ParseFailed => "parseFailed",
        ObsStatus::NotChecked => "notChecked",
        ObsStatus::Unsafe => "unsafe",
    }
}

pub fn collect_live_pulse(
    ctx: &HostContext,
    agents: &[AgentReport],
    now: Instant,
) -> LivePulseReport {
    let processes = scan_host_processes(ctx);
    let active_process_count = processes.len();
    let total_memory_mb = if processes.is_empty() {
        0.0
    } else {
        ((processes.iter().map(|p| p.memory_mb).sum::<f32>() * 10.0).round() / 10.0).max(0.0)
    };

    let mut config_ledger = Vec::new();
    for report in agents {
        let agent_id = report.snapshot.agent_id;
        for cs in &report.snapshot.config_sources {
            let expanded = crate::actions::expand(ctx, &cs.location);
            let label = expanded
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| cs.id.clone());

            let (exists, size_bytes, line_count, sha256) =
                if let Ok(meta) = std::fs::metadata(&expanded) {
                    if meta.is_file() {
                        if let Ok(read) = crate::platform::fs::read_bounded(&expanded, 256 * 1024) {
                            let lines = String::from_utf8_lossy(&read.bytes).lines().count();
                            let digest = sha256_hex(&read.bytes);
                            (true, meta.len(), lines, Some(digest))
                        } else {
                            (true, meta.len(), 0, cs.fingerprint.clone())
                        }
                    } else {
                        (false, 0, 0, None)
                    }
                } else {
                    (false, 0, 0, None)
                };

            config_ledger.push(ConfigFilePulse {
                agent_id,
                label,
                location: cs.location.clone(),
                exists,
                size_bytes,
                line_count,
                sha256,
                status: status_label(cs.status).to_string(),
            });
        }
    }

    let mut skill_map: BTreeMap<String, CrossAgentSkillItem> = BTreeMap::new();
    for report in agents {
        let agent_id = report.snapshot.agent_id;
        for skill in &report.snapshot.extensions.skills {
            let entry =
                skill_map
                    .entry(skill.name.clone())
                    .or_insert_with(|| CrossAgentSkillItem {
                        name: skill.name.clone(),
                        description: skill.description.clone(),
                        source_agent: agent_id,
                        source_location: skill.location.clone(),
                        has_scripts: skill.has_scripts,
                        installed_on: Vec::new(),
                        missing_on: Vec::new(),
                    });
            if entry.description.is_none() && skill.description.is_some() {
                entry.description = skill.description.clone();
            }
            if skill.has_scripts {
                entry.has_scripts = true;
            }
            if !entry.installed_on.contains(&agent_id) {
                entry.installed_on.push(agent_id);
            }
        }
    }

    let mut skill_matrix: Vec<CrossAgentSkillItem> = skill_map
        .into_values()
        .map(|mut item| {
            item.missing_on = AgentId::ALL
                .iter()
                .copied()
                .filter(|a| !item.installed_on.contains(a))
                .collect();
            item
        })
        .collect();
    skill_matrix.sort_by(|a, b| a.name.cmp(&b.name));

    LivePulseReport {
        scanned_at: now.to_rfc3339(),
        active_process_count,
        total_memory_mb,
        processes,
        config_ledger,
        skill_matrix,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ps_output_parsing_and_basename_redaction() {
        let sample = "\
  1024   1.2  92160    01:12:45 /Users/example/.local/share/claude/versions/2.1.311/claude
  2048   0.5  65536       14:22 /Applications/ChatGPT.app/Contents/Resources/codex-cli/CodexCLI.app/Contents/MacOS/codex
  3099   0.0  16384       05:10 /usr/local/bin/mcp-server-filesystem
  4001   8.4 512000    10:00:00 /Applications/Safari.app/Contents/MacOS/Safari
";
        let items = parse_ps_output(sample);
        assert_eq!(items.len(), 3);
        assert_eq!(items[0].pid, 1024);
        assert_eq!(items[0].agent_id, Some(AgentId::Claude));
        assert_eq!(items[0].command, "claude");
        assert_eq!(items[0].memory_mb, 90.0);

        assert_eq!(items[1].pid, 2048);
        assert_eq!(items[1].agent_id, Some(AgentId::Codex));
        assert_eq!(items[1].command, "codex");
        assert_eq!(items[1].memory_mb, 64.0);

        assert_eq!(items[2].pid, 3099);
        assert_eq!(items[2].agent_id, None);
        assert_eq!(items[2].command, "mcp-server-filesystem");
    }
}
