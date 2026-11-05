//! Project Workspaces (Phase 8).
//!
//! Associates a project path with an intended Agent setup, detected instructions (e.g. AGENTS.md,
//! CLAUDE.md), context-aware readiness evaluation, and safe terminal launch configuration.
//!
//! Enforces Security §5 path confinement: paths must reside inside user $HOME, cannot contain
//! traversal (..), cannot cross unsafe symlinks or reparse points, and never silently mutate
//! global Agent configurations.

use crate::core::{new_id, Instant, NexusError, NexusResult};
use crate::domain::AgentId;
use crate::platform::HostContext;
use crate::store::{Store, WorkspaceRecord};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum WorkspaceReadiness {
    Ready,
    Attention,
    Blocked,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceInstruction {
    pub file_name: String,
    pub relative_path: String,
    pub size_bytes: u64,
    pub line_count: usize,
    pub target_agent: String,
    pub is_supported_by_primary: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceView {
    pub id: String,
    pub name: String,
    pub path: String,
    pub display_path: String,
    pub primary_agent: AgentId,
    pub fallback_agent: Option<AgentId>,
    pub active_agent: AgentId,
    pub readiness: WorkspaceReadiness,
    pub readiness_reasons: Vec<String>,
    pub instructions: Vec<WorkspaceInstruction>,
    pub is_git_repo: bool,
    pub git_branch: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub last_opened_at: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateWorkspaceRequest {
    pub name: Option<String>,
    pub path: String,
    pub primary_agent: AgentId,
    pub fallback_agent: Option<AgentId>,
    pub model_id: Option<String>,
    pub profile_id: Option<String>,
}

/// Validates that a workspace path is safe, absolute, exists as a directory,
/// is confined strictly within user `$HOME`, and does not traverse or point outside.
pub fn validate_workspace_path(ctx: &HostContext, raw_path: &str) -> NexusResult<PathBuf> {
    let trimmed = raw_path.trim();
    if trimmed.is_empty() {
        return Err(NexusError::new("workspace.invalid_path"));
    }
    // Reject explicit path traversal
    if trimmed.contains("..") {
        return Err(NexusError::new("workspace.traversal_refused"));
    }
    // Expand ~ if present
    let path = if let Some(stripped) = trimmed.strip_prefix("~/") {
        ctx.home.join(stripped)
    } else if trimmed == "~" {
        ctx.home.clone()
    } else {
        PathBuf::from(trimmed)
    };
    if !path.is_absolute() {
        return Err(NexusError::new("workspace.invalid_path"));
    }
    // Must exist and be a directory
    if !path.exists() || !path.is_dir() {
        return Err(NexusError::new("workspace.directory_not_found"));
    }
    // Must be inside home directory (Security §5 path confinement)
    let home_canon = std::fs::canonicalize(&ctx.home).unwrap_or_else(|_| ctx.home.clone());
    let path_canon = std::fs::canonicalize(&path)
        .map_err(|_| NexusError::new("workspace.directory_not_found"))?;
    if !path_canon.starts_with(&home_canon) {
        return Err(NexusError::new("workspace.outside_home"));
    }
    // Reject if target path is a symlink or reparse point pointing elsewhere
    if let Ok(sym_meta) = std::fs::symlink_metadata(&path) {
        if crate::platform::fs::is_symlink_or_reparse(&sym_meta) {
            return Err(NexusError::new("workspace.symlink_refused"));
        }
    }
    Ok(path_canon)
}

const INSTRUCTION_CANDIDATES: &[(&str, &str)] = &[
    ("AGENTS.md", "universal"),
    ("CLAUDE.md", "claude"),
    ("GEMINI.md", "gemini"),
    (".gemini/rules", "gemini"),
    ("OPENCODE.md", "opencode"),
    (".opencode/rules", "opencode"),
    ("CODEX.md", "codex"),
    (".codex/instructions.md", "codex"),
];

/// Scans for well-known Agent instruction files within the project directory.
pub fn scan_instructions(dir: &Path, primary: AgentId) -> Vec<WorkspaceInstruction> {
    let mut out = Vec::new();
    for (rel, target) in INSTRUCTION_CANDIDATES {
        let p = dir.join(rel);
        if p.is_file() {
            if let Ok(sym_meta) = std::fs::symlink_metadata(&p) {
                if !crate::platform::fs::is_symlink_or_reparse(&sym_meta) {
                    if let Ok(meta) = std::fs::metadata(&p) {
                        // Bounded read (at most 64 KiB)
                        if meta.len() <= 65536 {
                            if let Ok(content) = std::fs::read_to_string(&p) {
                                let lines = content.lines().count();
                                let is_supported =
                                    *target == "universal" || *target == primary.as_str();
                                out.push(WorkspaceInstruction {
                                    file_name: p
                                        .file_name()
                                        .unwrap_or_default()
                                        .to_string_lossy()
                                        .to_string(),
                                    relative_path: (*rel).to_string(),
                                    size_bytes: meta.len(),
                                    line_count: lines,
                                    target_agent: (*target).to_string(),
                                    is_supported_by_primary: is_supported,
                                });
                            }
                        }
                    }
                }
            }
        }
    }
    out
}

/// Safely inspects .git/HEAD for branch identity without executing external child processes.
pub fn detect_git(dir: &Path) -> (bool, Option<String>) {
    let git_dir = dir.join(".git");
    if !git_dir.exists() {
        return (false, None);
    }
    let head_path = if git_dir.is_dir() {
        git_dir.join("HEAD")
    } else if git_dir.is_file() {
        // Linked worktree pointer: gitdir: ...
        if let Ok(content) = std::fs::read_to_string(&git_dir) {
            if let Some(rest) = content.trim().strip_prefix("gitdir:") {
                let p = PathBuf::from(rest.trim());
                let actual = if p.is_relative() { dir.join(p) } else { p };
                actual.join("HEAD")
            } else {
                return (true, None);
            }
        } else {
            return (true, None);
        }
    } else {
        return (false, None);
    };

    if let Ok(head_str) = std::fs::read_to_string(&head_path) {
        let trimmed = head_str.trim();
        if let Some(branch) = trimmed.strip_prefix("ref: refs/heads/") {
            return (true, Some(branch.to_string()));
        } else if trimmed.len() >= 7 {
            return (true, Some(trimmed[..7].to_string()));
        }
    }
    (true, None)
}

/// Evaluates context-aware readiness for the Workspace:
/// Checks if primary agent is installed; if not, checks fallback agent;
/// checks active agent health; checks instruction file compatibility and write access.
pub fn evaluate_workspace_readiness(
    installed_agents: &HashMap<AgentId, bool>,
    agent_health: &HashMap<AgentId, crate::domain::Readiness>,
    primary: AgentId,
    fallback: Option<AgentId>,
    instructions: &[WorkspaceInstruction],
    is_writable: bool,
) -> (AgentId, WorkspaceReadiness, Vec<String>) {
    let mut reasons = Vec::new();
    let primary_installed = installed_agents.get(&primary).copied().unwrap_or(false);
    let fallback_installed = fallback
        .and_then(|f| installed_agents.get(&f).copied())
        .unwrap_or(false);

    let active_agent = if primary_installed {
        primary
    } else if fallback_installed {
        fallback.unwrap()
    } else {
        primary
    };

    if !primary_installed {
        if fallback_installed {
            reasons.push(format!(
                "workspace.using_fallback:{}",
                fallback.unwrap().as_str()
            ));
        } else {
            reasons.push(format!(
                "workspace.primary_not_installed:{}",
                primary.as_str()
            ));
        }
    }

    let active_health = agent_health
        .get(&active_agent)
        .copied()
        .unwrap_or(crate::domain::Readiness::Unsupported);

    match active_health {
        crate::domain::Readiness::Ready => {
            reasons.push(format!("workspace.agent_ready:{}", active_agent.as_str()));
        }
        crate::domain::Readiness::Attention => {
            reasons.push(format!(
                "workspace.agent_attention:{}",
                active_agent.as_str()
            ));
        }
        crate::domain::Readiness::Blocked => {
            reasons.push(format!("workspace.agent_blocked:{}", active_agent.as_str()));
        }
        _ => {
            if primary_installed || fallback_installed {
                reasons.push(format!(
                    "workspace.agent_unverified:{}",
                    active_agent.as_str()
                ));
            }
        }
    }

    let unsupported_instructions: Vec<_> = instructions
        .iter()
        .filter(|i| !i.is_supported_by_primary)
        .collect();
    if !unsupported_instructions.is_empty() {
        reasons.push("workspace.unsupported_instructions".to_string());
    }

    if !is_writable {
        reasons.push("workspace.directory_read_only".to_string());
    }

    let readiness = if (!primary_installed && !fallback_installed)
        || active_health == crate::domain::Readiness::Blocked
    {
        WorkspaceReadiness::Blocked
    } else if (!primary_installed && fallback_installed)
        || active_health == crate::domain::Readiness::Attention
        || !unsupported_instructions.is_empty()
        || !is_writable
    {
        WorkspaceReadiness::Attention
    } else {
        WorkspaceReadiness::Ready
    };

    (active_agent, readiness, reasons)
}

/// Enriches a stored WorkspaceRecord into a full WorkspaceView.
pub fn inspect_workspace(
    ctx: &HostContext,
    record: &WorkspaceRecord,
    installed_agents: &HashMap<AgentId, bool>,
    agent_health: &HashMap<AgentId, crate::domain::Readiness>,
) -> WorkspaceView {
    let path = PathBuf::from(&record.path);
    let primary = AgentId::parse(&record.primary_agent).unwrap_or(AgentId::Claude);
    let fallback = record.fallback_agent.as_deref().and_then(AgentId::parse);
    let display_path = ctx.display_path(&path);

    let (is_git_repo, git_branch) = detect_git(&path);
    let instructions = scan_instructions(&path, primary);

    // Test directory writability safely
    let test_file = path.join(".nexus_write_test");
    let is_writable = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&test_file)
        .map(|_| {
            let _ = std::fs::remove_file(&test_file);
            true
        })
        .unwrap_or(false);

    let (active_agent, readiness, readiness_reasons) = evaluate_workspace_readiness(
        installed_agents,
        agent_health,
        primary,
        fallback,
        &instructions,
        is_writable,
    );

    WorkspaceView {
        id: record.id.clone(),
        name: record.name.clone(),
        path: record.path.clone(),
        display_path,
        primary_agent: primary,
        fallback_agent: fallback,
        active_agent,
        readiness,
        readiness_reasons,
        instructions,
        is_git_repo,
        git_branch,
        created_at: record.created_at.clone(),
        updated_at: record.updated_at.clone(),
        last_opened_at: record.last_opened_at.clone(),
    }
}

/// Creates a new workspace record, enforcing path validation and home confinement.
pub fn create_workspace(
    ctx: &HostContext,
    store: &Store,
    req: CreateWorkspaceRequest,
    now: Instant,
) -> NexusResult<WorkspaceRecord> {
    let valid_path = validate_workspace_path(ctx, &req.path)?;
    let path_str = valid_path.to_string_lossy().to_string();

    let name = req
        .name
        .filter(|n| !n.trim().is_empty())
        .unwrap_or_else(|| {
            valid_path
                .file_name()
                .map(|f| f.to_string_lossy().to_string())
                .unwrap_or_else(|| "Workspace".to_string())
        });

    let id = new_id("ws");
    let record = WorkspaceRecord {
        id,
        name,
        path: path_str,
        primary_agent: req.primary_agent.as_str().to_string(),
        fallback_agent: req.fallback_agent.map(|f| f.as_str().to_string()),
        model_id: req.model_id,
        profile_id: req.profile_id,
        created_at: now.to_rfc3339(),
        updated_at: now.to_rfc3339(),
        last_opened_at: None,
    };

    store.insert_workspace(&record)?;
    Ok(record)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_confinement_and_traversal_guards() {
        let dir = tempfile::tempdir().unwrap();
        let home = dir.path().join("home");
        std::fs::create_dir_all(&home).unwrap();
        let project = home.join("my-project");
        std::fs::create_dir_all(&project).unwrap();

        let outside = dir.path().join("outside");
        std::fs::create_dir_all(&outside).unwrap();

        let ctx = HostContext {
            home: home.clone(),
            data_dir: home.join(".nexus"),
            os: crate::platform::Os::Macos,
            arch: "arm64",
            env: std::collections::BTreeMap::new(),
            extra_bin_dirs: Vec::new(),
        };

        // Valid inside home
        assert!(validate_workspace_path(&ctx, "~/my-project").is_ok());
        assert!(validate_workspace_path(&ctx, &project.to_string_lossy()).is_ok());

        // Traversal is refused
        assert_eq!(
            validate_workspace_path(&ctx, "~/my-project/../outside")
                .unwrap_err()
                .code,
            "workspace.traversal_refused"
        );

        // Outside home is refused
        assert_eq!(
            validate_workspace_path(&ctx, &outside.to_string_lossy())
                .unwrap_err()
                .code,
            "workspace.outside_home"
        );

        // Non-existent directory
        assert_eq!(
            validate_workspace_path(&ctx, "~/does-not-exist")
                .unwrap_err()
                .code,
            "workspace.directory_not_found"
        );
    }

    #[test]
    fn instruction_scanning_and_git_detection() {
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path();

        // Write AGENTS.md and CLAUDE.md
        std::fs::write(
            project.join("AGENTS.md"),
            "# Project Constitution\nLine 2\nLine 3",
        )
        .unwrap();
        std::fs::write(project.join("CLAUDE.md"), "# Claude memory\nDetails").unwrap();

        let instrs = scan_instructions(project, AgentId::Claude);
        assert_eq!(instrs.len(), 2);
        let agents_md = instrs.iter().find(|i| i.file_name == "AGENTS.md").unwrap();
        assert_eq!(agents_md.line_count, 3);
        assert!(agents_md.is_supported_by_primary);

        let claude_md = instrs.iter().find(|i| i.file_name == "CLAUDE.md").unwrap();
        assert_eq!(claude_md.line_count, 2);
        assert!(claude_md.is_supported_by_primary);

        // For Gemini, CLAUDE.md is not supported by primary
        let gemini_instrs = scan_instructions(project, AgentId::Gemini);
        let claude_in_gemini = gemini_instrs
            .iter()
            .find(|i| i.file_name == "CLAUDE.md")
            .unwrap();
        assert!(!claude_in_gemini.is_supported_by_primary);

        // Git branch detection
        let git_dir = project.join(".git");
        std::fs::create_dir_all(&git_dir).unwrap();
        std::fs::write(
            git_dir.join("HEAD"),
            "ref: refs/heads/feature/workspace-p8\n",
        )
        .unwrap();
        let (is_git, branch) = detect_git(project);
        assert!(is_git);
        assert_eq!(branch.as_deref(), Some("feature/workspace-p8"));
    }

    #[test]
    fn context_aware_readiness_with_fallback() {
        let mut installed = HashMap::new();
        let mut health = HashMap::new();

        // Claude is NOT installed, Codex IS installed and Ready
        installed.insert(AgentId::Claude, false);
        installed.insert(AgentId::Codex, true);
        health.insert(AgentId::Codex, crate::domain::Readiness::Ready);

        let (active, readiness, reasons) = evaluate_workspace_readiness(
            &installed,
            &health,
            AgentId::Claude,
            Some(AgentId::Codex),
            &[],
            true,
        );

        assert_eq!(active, AgentId::Codex);
        assert_eq!(readiness, WorkspaceReadiness::Attention);
        assert!(reasons
            .iter()
            .any(|r| r.starts_with("workspace.using_fallback:codex")));

        // Neither installed -> Blocked
        installed.insert(AgentId::Codex, false);
        let (_, readiness_blocked, reasons_blocked) = evaluate_workspace_readiness(
            &installed,
            &health,
            AgentId::Claude,
            Some(AgentId::Codex),
            &[],
            true,
        );
        assert_eq!(readiness_blocked, WorkspaceReadiness::Blocked);
        assert!(reasons_blocked
            .iter()
            .any(|r| r.starts_with("workspace.primary_not_installed")));
    }
}
