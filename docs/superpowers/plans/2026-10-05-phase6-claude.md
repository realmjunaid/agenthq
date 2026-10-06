# Phase 6 (Claude Adapter) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Claude Code adapter implementing the agent trait from verified local sources (v2.1.289, research 2026-10-05).

**Architecture:** `agents::claude::{adapter, paths, parser}` — all probes take explicit base paths (no ambient HOME reads) so tests run against `fixtures/claude/`. JSON via `serde_json::Value`; transcript scanning capped (50 lines) for perf. No new crates. Registered in `lib.rs` setup.

**Tech Stack:** Rust 2021, serde_json (present), std::fs.

**Spec:** `MASTER_PLAN.md` §16 + §41 research rule. Verified facts (this machine): exe `~/.local/bin/claude.exe`, `--version` → `2.1.289 (Claude Code)`; config `~/.claude/` (settings.json keys: model/env/hooks); MCP per-project in `~/.claude.json/projects/<path>` (`mcpServers`, `enabled/disabledMcpjsonServers`); skills = `~/.claude/skills/*` dirs (+ `<project>/.claude/skills` if present); sessions = `~/.claude/projects/<slug>/<uuid>.jsonl` (typed lines: user/assistant carry timestamp/cwd/sessionId/version, assistant.message.model); plugins = `~/.claude/plugins/marketplaces/*/plugins/*` + `external_plugins/*` dir names; models = settings `model` field; connections = settings `env` key names only.

## Global Constraints

- NEVER read secret values: `env` values, tokens, keys — parse key NAMES and counts only. A test pins this (fixture with fake secret must never appear in output).
- Unverifiable → `Unsupported`/Unknown: subagent counting (sidechain hints exist but counting is guesswork) → `subagents()` Err; MCP liveness → status Configured/Offline only, never Connected; session status → Unknown (transcript can't prove aliveness).
- All parsing must tolerate missing files/dirs (fresh installs) → empty vecs, never Err, never panic.
- No git — no commit steps; ledger is the record.
- Verification per task: `cargo test --lib` in `agenthq/src-tauri` (exit 0). MSVC-prefixed PATH required. (`cargo test --bin agenthq` harness OS-blocked; gate uses `--lib`.)
- Fixtures in `agenthq/src-tauri/fixtures/claude/` are HAND-WRITTEN minimal shapes — never copy real user transcripts/configs.

## Review Focus

- A secret value in a fixture or settings env must never surface in any adapter output — Task 3 pins `test_no_secret_values_leak` (fixture env contains `FAKE-SECRET-xyz`, assert absent from connections output).
- A malformed jsonl line must not kill session enumeration — Task 2 pins `test_bad_line_skipped`.
- Missing `~/.claude` entirely (fresh machine) must yield installed=false + empty lists — Task 1 pins `test_missing_home_is_not_installed`.
- `mcpServers` entries with `env` blocks must expose names + count only — Task 3 pins `test_mcp_env_count_only`.
- Session scan must stay bounded on huge transcripts — Task 2 pins `test_scan_caps_lines` (fixture with 200 lines, model found within first 50 or Unknown; function reads ≤51 lines — assert via line-counter param or fixture design).

---

### Task 1: Paths + installation + version + registration

**Files:**
- Create: `agenthq/src-tauri/src/agents/claude/mod.rs`, `agenthq/src-tauri/src/agents/claude/paths.rs`, `agenthq/src-tauri/src/agents/claude/adapter.rs`
- Modify: `agenthq/src-tauri/src/agents/mod.rs` (add `pub mod claude;`), `agenthq/src-tauri/src/lib.rs` (register `ClaudeAdapter::new()` in setup)

**Interfaces:**
- Consumes: Phase 5 trait + `find_on_path`/`run_version`
- Produces:
  - `paths::home_claude(home: &Path) -> PathBuf` (home.join(".claude")), `paths::claude_json(home) -> PathBuf` (home.join(".claude.json")), `paths::known_exes(home) -> Vec<PathBuf>` ([find_on_path("claude") if Some, home/.local/bin/claude.exe, %PROGRAMFILES%? no — Claude on Win = user install; keep PATH + .local/bin])
  - `pub struct ClaudeAdapter { home: PathBuf }`, `ClaudeAdapter::new() -> Self` (home = dirs home via `std::env::home_dir`? std has no home_dir — use `dirs`? NO new deps. Use env var USERPROFILE/HOME: `std::env::var("USERPROFILE").or(var("HOME"))`. Windows-first, HOME fallback.)
  - `AgentAdapter` impl: `id()` = "claude", `name()` = "Claude Code", `executable_names()` = `&["claude"]`, `processes()` filters snapshot rows by shared stem-match helper, `capabilities()` = {processes, sessions, mcp, skills, plugins, models, connections, lifecycle_control: true; subagents, logs: false}
  - `detect_installation()`: installed = any known exe exists OR config dir exists; executable_path = first existing exe; version = `run_version(exe, ["--version"], 5000)` first token parse (`2.1.289` from `2.1.289 (Claude Code)`), Err → None (installed stays true)

- [ ] **Step 1: Write failing tests**:
```rust
#[test] fn test_missing_home_is_not_installed() { /* adapter with temp empty home → installed false, exe None, version None */ }
#[test] fn test_detect_uses_path_and_version() { /* real HOME? No — hermetic: known_exes() contains an existing file only if present; test parse_version("2.1.289 (Claude Code)") == "2.1.289" via parser fn */ }
#[test] fn test_capabilities_match_verified_set() { /* sessions/mcp/skills/plugins/models/connections true; subagents/logs false */ }
#[test] fn test_processes_filters_by_stem() { /* snapshot rows: claude.exe match, other.exe no, None exe no */ }
```
(`parse_version` lives in parser.rs — Task 1 creates `parser.rs` with just that fn? Plan adjustment: parser.rs created Task 1 with `parse_version`, extended Task 2/3.)
- [ ] **Step 2: Run tests, watch them fail**
Run (MSVC-prefixed PATH): `cargo test --lib agents::claude`
Expected: FAIL — `claude` module does not exist
- [ ] **Step 3: Implement** per interfaces; register in setup.
- [ ] **Step 4: Run tests, watch them pass**
Run: `cargo test --lib`
Expected: all pass

### Task 2: Session discovery

**Files:**
- Create: `agenthq/src-tauri/fixtures/claude/session.jsonl` (hand-written: mode line + user + assistant-with-model lines + one malformed line + 200 filler lines AFTER the model line? No — model must be within first 50: put filler AFTER line 60 to pin the cap)
- Modify: `agenthq/src-tauri/src/agents/claude/parser.rs` (add `parse_session_file(path) -> Option<SessionLite>`, `scan_model(lines) -> Option<String>` capped 50), `adapter.rs` (`sessions()` impl)

**Interfaces:**
- Consumes: Task 1 adapter skeleton
- Produces:
  - `pub struct SessionLite { id: String /* file stem */, project: String /* parent dir name */, started_at: Option<i64> /* first timestamp seen */, last_activity: Option<i64> /* max timestamp seen */, model: Option<String> }`
  - `parse_session_file`: read ≤51 lines; per line serde_json parse (skip Err); track min/max `timestamp` (unix secs; float ok), first assistant `message.model`; id = stem, project = parent dir name
  - `sessions() -> CapabilityResult<Vec<AgentSession>>`: enumerate `home/.claude/projects/*/*.jsonl` (dirs only, ignore `memory/` subdirs and non-jsonl), map to `AgentSession { id, status: "unknown" }`; missing dir → empty vec

- [ ] **Step 1: Write failing tests**:
```rust
#[test] fn test_session_fixture_parses() { /* id/project/started/model from fixture */ }
#[test] fn test_bad_line_skipped() { /* fixture malformed line doesn't fail parse */ }
#[test] fn test_scan_caps_lines() { /* model placed at line 60 → None (cap 50); moved to line 10 → Some */ }
#[test] fn test_missing_projects_dir_is_empty() { /* temp home without projects → sessions() Ok(empty) */ }
```
- [ ] **Step 2: Run tests, watch them fail**
Run: `cargo test --lib agents::claude`
Expected: FAIL — `parse_session_file`/`sessions` missing
- [ ] **Step 3: Implement** per interfaces.
- [ ] **Step 4: Run tests, watch them pass**
Run: `cargo test --lib`
Expected: all pass

### Task 3: MCP + skills + plugins + models + connections

**Files:**
- Create fixtures: `agenthq/src-tauri/fixtures/claude/settings.json` (`{model, env: {FAKE_KEY: "FAKE-SECRET-xyz"}, hooks}`), `agenthq/src-tauri/fixtures/claude/claude.json` (`{projects: {"E:/x": {mcpServers: {gh: {command: "x", env: {T: "SECRET2"}}}, enabledMcpjsonServers: ["gh"], disabledMcpjsonServers: []}}}`), `agenthq/src-tauri/fixtures/claude/skills/` (two fake skill dirs with SKILL.md `description:` lines)
- Modify: `parser.rs` + `adapter.rs` (the five methods)

**Interfaces:**
- Consumes: Tasks 1–2
- Produces:
  - `mcp_servers()`: for each project in claude.json + enabled/disabled lists: entries {name: server key}; status Configured if in enabled OR lists absent, Offline if in disabled; env blocks → counted, never read. Project-level `<project>/.mcp.json`? Out of scope (note in docs).
  - `skills()`: global `skills/*` dirs (+ project `.claude/skills` when a project path is known? Adapter has no project context in Phase 6 — global only, note it): {name: dirname, description: SKILL.md `description:` first-line value or None}
  - `plugins()`: `plugins/marketplaces/*/plugins/*` + `*/external_plugins/*` dir names (skip files like known_marketplaces.json)
  - `models()`: settings `model` string → [{name: value}] or empty if absent
  - `connections()`: settings `env` keys → [{name: key}] (values never read)
  - `subagents()` → `Err(Unsupported("subagents"))` (explicit)

- [ ] **Step 1: Write failing tests**:
```rust
#[test] fn test_mcp_names_and_status() { /* gh → Configured; disabled one → Offline */ }
#[test] fn test_mcp_env_count_only() { /* output contains no "SECRET2"; env count recorded? — McpServerInfo has only name in Phase 5 DTO; assert name present + secret absent. (Rich McpServer DTO arrives Phase 9.) */ }
#[test] fn test_skills_from_dirs() { /* two fixture skills found with descriptions */ }
#[test] fn test_models_and_connections() { /* model auto; connection key present, secret absent */ }
#[test] fn test_no_secret_values_leak() { /* run all five methods on fixtures; assert "FAKE-SECRET-xyz" and "SECRET2" in none of the Debug outputs */ }
```
- [ ] **Step 2: Run tests, watch them fail**
Run: `cargo test --lib agents::claude`
Expected: FAIL — methods return default `Err(Unsupported)` (tests expect Ok)
- [ ] **Step 3: Implement** per interfaces.
- [ ] **Step 4: Run tests, watch them pass**
Run: `cargo test --lib`
Expected: all pass

### Task 4: Research doc update + full verification

**Files:**
- Modify: `docs/agent-research.md` (Claude section: verified paths/behaviors, capabilities verdict table), `docs/development.md` (nothing unless commands change)

**Interfaces:**
- Consumes: Tasks 1–3
- Produces: research doc records exe/config/MCP/skills/sessions/plugins/models/connections sources + subagents-Unknown rationale

- [ ] **Step 1: Update `docs/agent-research.md` Claude rows**
- [ ] **Step 2: Verify everything**
Run (MSVC-prefixed PATH): `cargo check`, `cargo test --lib`; in `agenthq/`: `npx tsc --noEmit`, `npm run build`
Expected: all exit 0; tests all pass
