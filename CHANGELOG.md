# Changelog

## 0.6.0 — 2026-11-05 (Breakthrough: Phase 10 Session Continuity & Contextual Usage Insights)

Major breakthrough milestone delivering **Phase 10: Session Continuity & Contextual Usage/Cost Insights**, empowering developers to resume in-flight project conversations directly and understand their LLM resource investment with transparent mathematical accounting and 100% offline privacy guarantees.

### Added
- **Session Continuity Engine (`sessions.rs`)**: Version-qualified discovery, indexing, and one-click resumption for local Agent sessions across Claude Code, OpenAI Codex, Gemini CLI, and OpenCode.
- **Safe Terminal Session Resume (`nexus_resume_session`)**: Verified binary executable checks, strictly pinned working directory to project path, and native OS terminal launcher with audit logging in SQLite operations.
- **Contextual Usage & Cost Insights (`usage_insights.rs`)**: Offline, deterministic token usage aggregation and reference cost computation across Workspaces, Agents, and Models with full provenance disclosure.
- **Official Model Pricing Matrix**: Reference pricing schedules (input/output cents per 1M tokens) for Claude 3.5 Sonnet, Claude 3.5 Haiku, Claude 3 Opus, GPT-4o, GPT-4o mini, o1-preview, o1-mini, Gemini 1.5 Pro, and Gemini 1.5 Flash. Unrecognized or local models are explicitly labeled as unknown/unpriced.
- **SQLite Database Migration #3 (`agent_sessions`)**: Created indexed `agent_sessions` table with unique constraint on `(agent_id, external_session_id)`, tracking message count, input units, output units, estimated costs, and last active timestamps.
- **Workspace-Detached Session Retention**: Deleting a workspace nullifies references without destroying historical session records; users retain full privacy control to remove individual session references at any time (`nexus_delete_session`).
- **Interactive Session Continuity UI (`SessionContinuity.tsx`)**: High-fidelity session cards embedded in Workspaces with token count, message count, cost badge, and 1-click terminal resume.
- **Interactive Contextual Usage Panel (`UsageInsightsPanel.tsx`)**: Statistical summary cards, Agent token distribution bars, Model breakdown tables, and privacy guarantee disclosures.
- **Typed IPC Allowlist**: Registered `nexus_list_sessions`, `nexus_resume_session`, `nexus_delete_session`, and `nexus_usage_insights` across Tauri capabilities, build manifests, and contract test suites.

## 0.5.0 — 2026-11-05 (Breakthrough: Phase 9 AI Doctor & Visual Health Radar)

Major breakthrough release delivering Phase 9 AI Doctor and the 7-Stage Visual Health Radar, transforming Project Nexus into an intelligent, privacy-preserving self-healing lifecycle copilot.

### Added
- **AI Doctor Expert Reasoning Engine (Phase 9)**: Local, deterministic expert reasoning engine that analyzes live Agent health telemetry, configuration integrity, model drift, and operation journals without remote script execution, AI hallucinations, or cloud dependencies.
- **100% In-Memory Privacy Redaction**: All filesystem paths, usernames, environment variables, and authentication tokens are scrubbed via `crate::core::redact` before synthesis; zero plaintext secrets are ever exposed.
- **Ranked Probable Causes & Grounded Confidence**: Diagnostic findings are ranked by impact and certainty (`High`, `Medium`, `Low`) with concrete redacted evidence snippets and mathematical confidence scoring (e.g., 96% Grounded Confidence).
- **Verified Safe Repairs with 1-Click Native Confirmation**: Recommendations map strictly to allowlisted, typed `ActionRequest` variants (`restoreBackup`, `applyModel`, `recoverInstall`, `setMcpEnabled`). Triggering repairs feeds directly into the native OS confirmation dialog via `useActionFlow` with zero arbitrary execution.
- **7-Stage Visual Health Radar (`HealthRadar.tsx`)**: High-fidelity signal bus component visualizing the end-to-end readiness pipeline: `Runtime` ──▶ `Toolchain` ──▶ `Config` ──▶ `Auth` ──▶ `Model` ──▶ `Extensions` ──▶ `Network`, featuring status indicators and an interactive telemetry inspector.
- **Interactive AI Doctor Panel (`AiDoctorPanel.tsx`)**: Embedded on the Agent Detail Diagnostics tab with live scanning animation, health state badges, root-cause breakdowns, and 1-click repair actions.
- **Typed IPC Command & Allowlist**: Registered and allowlisted `nexus_consult_ai_doctor` across Tauri capabilities, build manifest, and contract tests.

## 0.4.0 — 2026-11-05 (Breakthrough: Phase 8 Project Workspaces)

Major milestone delivering Phase 8 Project Workspaces, bringing context-aware Agent environments, instruction discovery, and safe terminal launch to Project Nexus.

### Added
- **Project Workspaces (Phase 8)**: register local software repositories and directories as managed Workspaces, binding primary and fallback Agents directly to project paths without mutating global configs.
- **Deep Project Instruction Discovery**: automatically detects, parses, and line-counts project instruction manifests (`AGENTS.md`, `CLAUDE.md`, `GEMINI.md`, `.gemini/rules`, `OPENCODE.md`, `.opencode/rules`, `CODEX.md`) with bounded reads and Agent compatibility verification.
- **Context-Aware Readiness Evaluation**: computes real-time readiness (`ready`, `attention`, `blocked`) for each workspace based on active Agent installation, agent health, instruction compatibility, directory writability, and automatic fallback resolution.
- **Safe Terminal Launcher (`nexus_launch_workspace`)**: one-click launch of the active Agent in the user's terminal with working directory strictly pinned to the project path and journaled in the SQLite operations log.
- **Strict Path Confinement & Traversal Guards**: workspace paths must reside within `$HOME`, strictly rejecting path traversal (`..`), symlinks, and reparse points pointing outside.
- **Persistent Storage Migration**: added SQLite migration #2 for `workspaces` table with unique path constraints and last-opened timestamps.
- **Home Integration**: live Project Workspaces section on the Home page with card grid, Git branch indicators, active Agent badges, instruction tags, and inline registration form.

## 0.3.0-beta.1 — 2026-11-05 (pre-release, Core Backend Beta Candidate / Phase 7 Public Beta Gate)

Iteration 3 release completing local Skill lifecycle management, directory traversal and executable script disclosures, safe managed removal, and Phase 7 public Beta readiness.

### Added
- **Local Skill lifecycle management (`addSkill` and `removeManagedSkill`)**: users can install local Skill packages into any supported Agent (`Codex`, `Claude Code`, `Gemini CLI`, `OpenCode`) directly from existing folders with preview, native confirmation, and independent postcondition verification.
- **Strict Skill safety & directory traversal guards**: source directories must reside within user `$HOME`, traversal (`..`) is strictly refused, recursive scans enforce file count (max 200) and byte size (max 50 MB) limits, and any symlinks or Windows reparse points cause immediate closed failure.
- **Deep script & executable disclosure**: skill directories are recursively inspected for executable binaries and scripts (`.sh`, `.py`, `.js`, `.ps1`, `.bat`, `.cmd`, `.exe`), disclosing `"warning.skillScripts"` or `"warning.skillInstructionsOnly"` in the native OS confirmation dialog before copying.
- **Safe managed removal with automatic compensation**: installed skills are recorded in the `managed_extensions` ledger (`nexus_managed = true`); user pre-existing skills can never be deleted through Nexus; and installation failure automatically compensates by removing temporary and newly copied folders.
- **Skills UI on Extensions & Agent Detail**: live Skills tab now features an "Add local Skill" card with Agent selection, folder path input, preview action flow, and "Remove…" actions for Nexus-managed skills.

## 0.2.2-beta.1 — 2026-11-05 (pre-release, Core Backend Beta Candidate)

Iteration 2 release achieving Windows parity, Gate I review closure, and user-initiated Provider connectivity verification.

### Added
- **Opt-in Provider reachability & credential verification (`nexus_check_provider`)**: user-initiated read-only check obeying Security §7 network controls. HTTPS is enforced by default; plaintext HTTP is strictly confined to loopback addresses (`127.0.0.1`, `localhost`, `[::1]`). Redirects are disabled (0 redirects) to eliminate credential leakage across origins, cloud metadata endpoints (`169.254.169.254`, `metadata.google.internal`) are blocked, and requests time out at 5 seconds.
- **Provider connection state & model discovery**: parses OpenAI/Ollama model lists up to 64 items into sanitized model definitions, measures live latency in milliseconds, and exposes live status ("connected", "authFailed", "unreachable") on the Models page with an on-demand "Test connection" button.
- **Fallible transaction journaling**: `config::transaction::apply` now makes `on_step` fallible, ensuring any failure in SQLite backup insertion or step logging aborts before atomic file replacement on disk.

### Fixed & Hardened
- **Windows process & executable parity**: `is_executable_file` now recognizes `.cmd` and `.bat` shims on Windows, unblocking Windows `npm.cmd` execution.
- **Windows filesystem safety**: `check_mutable_target` refuses mutation across Windows reparse points (`FILE_ATTRIBUTE_REPARSE_POINT`) in addition to symlinks; `check_owner` guards against mutating read-only files on Windows.
- **Windows global npm layout**: `npm_prefix_of` supports Windows `%AppData%/npm/node_modules/` layout in addition to Unix `/lib/node_modules/`.
- **Platform support level updated**: Windows discovery, health, and verified operations are promoted to `Supported` alongside macOS.

## 0.2.1-beta.1 — 2026-11-03 (pre-release, Core Backend Beta Candidate)

Iteration 1 hardening release toward the Phase 7 public Beta gate. Closes security, freshness, comment-preservation, MCP management, Model Profile lifecycle, local retention, and redacted diagnostic export gaps while keeping Phases 1–7 boundaries intact.

### Added
- **Reviewed local stdio MCP recipes (`addReviewedMcp`)** for OpenAI Codex, Gemini CLI and OpenCode: compiled `@modelcontextprotocol/server-memory` and `@modelcontextprotocol/server-filesystem` recipes via `npx`, with strict `$HOME` non-hidden directory confinement, native confirmation disclosure, postcondition verification, and managed removal.
- **Model Profile deletion and revision impact UI**: `nexus_delete_profile` refuses deletion while assigned (`profile.in_use`); the Models page shows per-Profile revision alignment across assigned Agents and warns before saving a new revision.
- **Settings — Local data & retention**: live SQLite database and encrypted backup vault usage (`nexus_storage_status`) plus bounded history pruning (`nexus_prune_history`) that preserves unresolved operations, the latest backup per configuration file, and the latest readiness scan per Agent.
- **Settings — Redacted diagnostic export**: preview-required JSON bundle (`nexus_diagnostic_preview`) with `~`-minimized paths and automatic token redaction, plus explicit Copy JSON and Save JSON actions.
- **Agent operation and readiness history** in Agent Detail → Configuration (`Recent Agent operations` and `Recent readiness checks`).
- **GitHub Actions CI (`.github/workflows/ci.yml`)** running TypeScript, ESLint, Vitest, Vite production build, `rustfmt`, `clippy -D warnings`, and Rust tests across macOS and Windows, plus Playwright UI and `axe` accessibility tests on macOS.

### Fixed & Hardened
- **Span-preserving JSON/JSONC member removal**: `config::jsonc::Document::remove` now uses comment-aware comma scanning so standalone and inline comments survive first, middle, last, sole, and CRLF member removals.
- **Structural `restoreBackup` execution-sensitive detection**: replaced substring matching with structural `serde_json::Value` comparison across hooks, credential helpers, shell policies, and MCP entries.
- **Managed MCP hydration in `App::report`**: `server.nexus_managed` is now hydrated from `managed_extensions`, enabling safe removal of Nexus-added remote and reviewed local MCP servers in Agent Detail and Extensions.
- **Plan expiry re-check after native confirmation**: `App::confirm` re-verifies `plan.expires_at` immediately after the OS confirmation dialog returns so a stalled dialog cannot execute a stale plan.
- **No raw configuration diffs over IPC**: `ConfigEdit` omits raw file diff lines and `ProviderProfile` skips `secret_ref` serialization over IPC; frontend freshness timers invalidate stale snapshots cleanly.
- **English-only (`en-US`) interface and documentation**: removed Simplified Chinese catalogs/fonts and added a comprehensive README with screenshots.

## 0.2.0-beta.1 — 2026-10-27 (pre-release, Core Backend Beta Candidate)

First build with a functional backend underneath the approved Phase 1 interface. Validated on macOS arm64 only. **Not a public Beta**: Windows is unvalidated and the build is not notarized.

### Added
- Read-only Agent and System discovery for OpenAI Codex, Claude Code, Gemini CLI and OpenCode: installations, versions with evidence source, install source, duplicates, configuration files, model precedence (including environment and shell-profile overrides), login evidence (presence only), MCP servers, Skills, Node/npm/Git/Python/Homebrew.
- Deterministic Health engine (`nexus-readiness-1`) with explicit coverage and staleness; structured Diagnostics with cause certainty and evidence.
- Action Registry with immutable, digest-bound, single-use plans; Rust-owned native confirmation; journaled execution; independent verification; automatic compensation only when no external edit intervened.
- Safe Repair: restore an encrypted known-good backup, reapply a displaced model assignment, recover an interrupted Nexus installation, disable an MCP server whose command is missing.
- Lifecycle: npm and Homebrew install/update/uninstall recipes with pinned versions; Claude Code `claude update`; launch in Terminal. Script installers are never executed.
- Models: Providers, Models (built-in aliases, found-in-config, manual), revisioned Model Profiles, per-Agent assignments with desired/applied/effective states; Keychain-only secrets.
- Extensions: MCP enable/disable, add remote HTTPS server, remove Nexus-added servers (Codex, Gemini CLI, OpenCode); Skills read-only.
- Local SQLite persistence with versioned migrations; no secrets stored.
- Live data across Home, Agents, Agent Detail, Models, Extensions, System and Settings.

### Known limitations
- Windows: implemented paths are unverified; Windows npm recipes are inactive.
- No online Provider/credential validation or model discovery; custom endpoints cannot be assigned to Agents.
- No Skills installation, no stdio MCP creation, no Claude Code MCP editing.
- Removing a JSONC entry can drop an adjacent comment line.
- See `docs/BACKEND_IMPLEMENTATION_PROGRESS.md` for the full capability matrix and open review items.
