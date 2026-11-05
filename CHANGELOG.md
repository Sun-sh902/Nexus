# Changelog

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
