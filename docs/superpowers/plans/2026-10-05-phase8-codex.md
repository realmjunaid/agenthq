# Phase 8 (Codex Adapter) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Codex adapter implementing the agent trait from verified local sources (codex-cli 0.160.0, research 2026-10-05).

**Architecture:** `agents::codex::{adapter, paths, parser}` mirroring Claude/OpenCode shape — explicit base paths, fixture-driven tests, no new crates. TOML is parsed with a minimal section-header scanner (no `toml` dep for one use). Registered in `lib.rs` setup.

**Tech Stack:** Rust 2021, serde_json (present), std only.

**Spec:** `MASTER_PLAN.md` §18 + §41. Verified facts: exe NOT on PATH; live at `~/.codex/packages/app-server-daemon/releases/<ver>-x86_64-pc-windows-msvc/bin/codex.exe` (self-updating; pick max version dir), currently 0.160.0, processes running; `--version` → `codex-cli 0.160.0`; config `~/.codex/` (config.toml safe keys tui/windows/features; `auth.json` EXISTS — never read); sessions = `session_index.jsonl` lines `{id, thread_name, updated_at}` (+ rollout transcripts `sessions/YYYY/MM/DD/rollout-*.jsonl` with `session_meta` payload: session_id/timestamp/cwd/cli_version/model_provider — no model name); skills = `~/.codex/skills/*` dirs; plugins = `~/.codex/plugins/*` entries; models = `models_cache.json` `models[].slug` (public slugs).

## Global Constraints

- NEVER read `auth.json` or any credential/token/secret value. Tests pin absence (fixture contains canary secret; no code path opens auth.json — pin by asserting outputs lack the canary AND by code read in review).
- Do not invent sessions/subagents: sessions from `session_index.jsonl` only (status Unknown); subagents → `Err(Unsupported)`; session model → Unknown (payload has provider, DTO has no slot).
- MCP names from `config.toml` `[mcp_servers.<name>]` section headers only — values/`env` never read. No `toml` crate; scanner reads headers, ignores all else.
- Missing CLI/config → installed=false / empty lists, never panic.
- No git — no commit steps; ledger is the record.
- Verification per task: `cargo test --lib` in `agenthq/src-tauri` (exit 0). MSVC-prefixed PATH required. (`cargo test --bin agenthq` harness OS-blocked; gate uses `--lib`.)
- Fixtures hand-written in `agenthq/src-tauri/fixtures/codex/`.

## Review Focus

- `auth.json` must have no reader — Task 1 pins by grep: `auth` string absent from `agents/codex/*.rs` except in a comment naming the avoidance.
- A releases dir with multiple versions must pick the max — Task 1 pins `test_releases_picks_max_version` (dirs 0.159.0 + 0.160.0 → 0.160.0 exe).
- A malformed `session_index.jsonl` line must not kill enumeration — Task 2 pins skip.
- `[mcp_servers.x]` headers with tricky spacing/quotes must parse name-only — Task 3 pins `mcp_servers."quoted name"` + extra spaces variants.
- `models_cache.json` without `models` array must yield empty, not Err — Task 3 pins.

---

### Task 1: Paths + installation + version + registration

**Files:**
- Create: `agenthq/src-tauri/src/agents/codex/mod.rs`, `.../paths.rs`, `.../adapter.rs`, `.../parser.rs`
- Modify: `agenthq/src-tauri/src/agents/mod.rs` (add `pub mod codex;`), `agenthq/src-tauri/src/lib.rs` (register `CodexAdapter::new()`)

**Interfaces:**
- Consumes: Phase 5/7 trait + helpers + `manager::exe_stem_matches`
- Produces:
  - `paths::codex_dir(home) -> PathBuf` (`~/.codex`), `paths::config_file`, `paths::session_index`, `paths::releases_exe(home) -> Option<PathBuf>` (glob `packages/*/releases/*/bin/codex.exe`, pick max version dirname lexically-by-semver-parts), `paths::known_exes(home)` (PATH `codex` + releases_exe)
  - `parser::parse_version("codex-cli 0.160.0") == Some("0.160.0")` (last token)
  - `CodexAdapter { home }`, `new()`/`with_home()`/`evaluate()` (same pure shape as siblings), `id()` = "codex", `name()` = "Codex CLI", `executable_names()` = `&["codex"]`, `processes()` stem-filter, capabilities {processes, sessions, mcp, skills, plugins, models, lifecycle_control: true; subagents, connections, logs: false}
  - `detect_installation()`: installed = exe found OR codex dir exists; version via `run_version(exe, ["--version"], 5000)` + parse; exe missing → version None, installed per dir

- [ ] **Step 1: Write failing tests**:
```rust
#[test] fn test_parse_version_last_token() { /* "codex-cli 0.160.0" → "0.160.0"; "" → None */ }
#[test] fn test_missing_home_is_not_installed() { /* evaluate(None,false,None) → false */ }
#[test] fn test_releases_picks_max_version() { /* temp packages/a/releases/{0.159.0,0.160.0}-x86_64-pc-windows-msvc/bin/codex.exe (empty files) → picks 0.160.0 */ }
#[test] fn test_capabilities_match_verified_set() { /* per produces */ }
#[test] fn test_processes_filters_by_stem() { /* codex.exe match */ }
#[test] fn test_no_auth_reader_exists() { /* read agents/codex/*.rs sources? — source-grep test is brittle; instead: list public fns? Decision: implement as `assert!(!include_str!("adapter.rs").contains("auth.json") && !include_str!("parser.rs").contains("auth.json") && !include_str!("paths.rs").contains("auth.json"))` — include_str of own sources, hermetic and honest. */ }
```
- [ ] **Step 2: Run tests, watch them fail**
Run (MSVC-prefixed PATH): `cargo test --lib agents::codex`
Expected: FAIL — `codex` module does not exist
- [ ] **Step 3: Implement** per interfaces; register in setup.
- [ ] **Step 4: Run tests, watch them pass**
Run: `cargo test --lib`
Expected: all pass

### Task 2: Sessions via index

**Files:**
- Create fixture: `agenthq/src-tauri/fixtures/codex/home/.codex/session_index.jsonl` (2 valid lines + 1 malformed)
- Modify: `parser.rs` (`parse_session_index(text) -> Vec<IndexSession{id}>` skipping bad lines), `adapter.rs` (`sessions()`)

**Interfaces:**
- Consumes: Task 1 adapter
- Produces:
  - `pub struct IndexSession { id: String }`, `parse_session_index` — per-line serde parse, skip Err, require string `id`
  - `sessions()`: read `session_index` file → parse → map to `AgentSession{id, status unknown}`; missing/unreadable → empty

- [ ] **Step 1: Write failing tests**:
```rust
#[test] fn test_index_fixture_parses_two() { /* ids match */ }
#[test] fn test_bad_line_skipped() { /* 2 valid despite malformed line */ }
#[test] fn test_missing_index_is_empty() { /* temp home → Ok(empty) */ }
```
- [ ] **Step 2: Run tests, watch them fail**
Run: `cargo test --lib agents::codex`
Expected: FAIL — `parse_session_index`/sessions missing or default Err
- [ ] **Step 3: Implement** per interfaces.
- [ ] **Step 4: Run tests, watch them pass**
Run: `cargo test --lib`
Expected: all pass

### Task 3: MCP + skills + plugins + models

**Files:**
- Create fixtures under `agenthq/src-tauri/fixtures/codex/home/.codex/`: `config.toml` (`[mcp_servers.gh]` + `[mcp_servers."quoted name"]` + `[other]` + `key = "CANARY-SECRET-1"`), `skills/demo-skill/SKILL.md` (description), `skills/plain/` (empty dir), `plugins/plugin-a/` (empty dir), `models_cache.json` (`{models: [{slug: "demo-model"}]}`) + a second fixture `models_cache_empty.json`? Use inline strings for the empty case instead of a file.
- Modify: `parser.rs` + `adapter.rs`

**Interfaces:**
- Consumes: Tasks 1–2
- Produces:
  - `parser::mcp_server_names(toml_text: &str) -> Vec<String>` — lines matching `^\s*\[mcp_servers\.(?:"([^"]+)"|([^\]\s]+))` → names; everything else ignored (values never parsed)
  - `mcp_servers()`: read config.toml → names; missing → empty
  - `skills()`: `skills/*` dirs (skip dotfiles) → {name, description via `crate::agents::claude::parser::skill_description`}; sort
  - `plugins()`: `plugins/*` dir names (files skipped); sort
  - `models()`: `models_cache.json` → `models[].slug` strings; missing/invalid/no-array → empty
  - `subagents()`/`connections()` → default `Err(Unsupported)` (no override)

- [ ] **Step 1: Write failing tests**:
```rust
#[test] fn test_mcp_names_headers_only() { /* gh + quoted name; "other" section excluded */ }
#[test] fn test_config_canary_absent() { /* Debug of mcp_servers() lacks CANARY-SECRET-1 */ }
#[test] fn test_skills_from_dirs() { /* demo-skill with description; plain without; dotfiles skipped */ }
#[test] fn test_plugins_and_models() { /* plugin-a; demo-model; empty-cache inline → empty */ }
```
- [ ] **Step 2: Run tests, watch them fail**
Run: `cargo test --lib agents::codex`
Expected: FAIL — methods default Err / fns missing
- [ ] **Step 3: Implement** per interfaces.
- [ ] **Step 4: Run tests, watch them pass**
Run: `cargo test --lib`
Expected: all pass

### Task 4: Research doc + full verification

**Files:**
- Modify: `docs/agent-research.md` (Codex section replaces UNVERIFIED block: exe location pattern, version, config files, sessions/skills/plugins/models sources, auth.json avoidance, subagents/connections rationale)

- [ ] **Step 1: Update research doc**
- [ ] **Step 2: Verify everything**
Run (MSVC-prefixed PATH): `cargo fmt`, `cargo check`, `cargo test --lib`; in `agenthq/`: `npx tsc --noEmit`, `npm run build`
Expected: fmt clean; check zero warnings; all pass; tsc/build exit 0
