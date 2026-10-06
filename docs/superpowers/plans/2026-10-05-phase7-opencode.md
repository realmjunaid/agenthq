# Phase 7 (OpenCode Adapter) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** OpenCode adapter implementing the agent trait from verified local sources (v2.0.23, research 2026-10-05).

**Architecture:** `agents::opencode::{adapter, paths, parser}` mirroring the Claude adapter shape — explicit base paths, fixture-driven tests, no new crates. CLI-as-API where config files are insufficient (`session list --format json`, `plugin list`, `models`); config files for the rest. Registered in `lib.rs` setup.

**Tech Stack:** Rust 2021, serde_json (present), std only.

**Spec:** `MASTER_PLAN.md` §17 + §41. Verified facts: exe PATH `opencode` (+ `%APPDATA%/npm/opencode.cmd`), `--version` → `opencode v2.0.23`; global config `~/.config/opencode/opencode.json` (keys incl. `plugins`; `mcp` object when configured — currently absent); global skills `~/.config/opencode/skills/*` (SKILL.md212 with `description:`); sessions via `opencode session list --format json -n N` → `[{id,title,updated,created,projectId,directory}]` (ms epochs, per-project of cwd); plugins via `opencode plugin list` (ID/VERSION/SOURCE columns); models via `opencode models` (`provider/model` lines); `opencode mcp list` → none configured here; auth/credentials NEVER touched → connections Unsupported.

## Global Constraints

- NEVER touch credentials: no `auth export`, no reading auth files, no secret values. Tests pin absence.
- `session list`/`plugin list`/`models` run with 10s timeout via `run_version`-style helper with arbitrary args (generalize: reuse `run_version(exe, args, timeout)` — it returns first stdout line only; add `run_command_lines(exe, args, cwd, timeout) -> Result<Vec<String>, String>` in detect.rs capturing full stdout, line-capped at 500).
- Missing CLI/config → installed=false or empty lists, never panic. Timeouts → empty lists (sessions) with no error propagation past the adapter.
- Project-level config (`<project>/opencode.json` MCP, project skills) has no project context in Phase 7 → global only; separation arrives with the projects engine (Phase 12). Note it, don't fake it.
- `sessions()` without project context returns Ok(empty) — mechanism verified by fixtures + project-dir param; data flows in Phase 12.
- No git — no commit steps; ledger is the record.
- Verification per task: `cargo test --lib` in `agenthq/src-tauri` (exit 0). MSVC-prefixed PATH required. (`cargo test --bin agenthq` harness OS-blocked; gate uses `--lib`.)
- Fixtures hand-written in `agenthq/src-tauri/fixtures/opencode/`.

## Review Focus

- A `session list` timeout/hang must degrade to empty, never Err — Task 2 pins `test_sessions_timeout_is_empty` (bad exe or 200ms budget vs sleeper).
- `plugin list` header/separator lines must not become plugins — Task 3 pins header + `---` + empty lines skipped.
- `models` blank lines must not become models — Task 3 pins.
- MCP `env` blocks in opencode.json must expose names only — Task 3 pins secret absence.
- `session list` output over 500 lines must be capped — Task 2 pins `run_command_lines` cap (feed 600-line fixture via echo? Use a temp script? Simpler: unit-test the capping inside the line-collector with a synthetic large stdout — implement collector as pure `take_lines(output: &str) -> Vec<String>` capped 500, tested directly).

---

### Task 1: Paths + installation + version + registration

**Files:**
- Create: `agenthq/src-tauri/src/agents/opencode/mod.rs`, `.../paths.rs`, `.../adapter.rs`, `.../parser.rs`
- Modify: `agenthq/src-tauri/src/agents/mod.rs` (add `pub mod opencode;`), `agenthq/src-tauri/src/lib.rs` (register `OpenCodeAdapter::new()`), `agenthq/src-tauri/src/agents/detect.rs` (add `run_command_lines`)

**Interfaces:**
- Consumes: Phase 5/6 trait + helpers
- Produces:
  - `paths::config_dir(home) -> PathBuf` (`~/.config/opencode`), `paths::global_config(home)`, `paths::global_skills(home)`, `paths::known_exes(home)` (PATH `opencode` + `%APPDATA%/npm/opencode.cmd` + `.cmd`/`.exe` variants)
  - `parser::parse_version("opencode v2.0.23") == Some("2.0.23")` (last token, strip leading `v`, digits/dots check)
  - `detect::run_command_lines(exe, args, cwd: Option<&Path>, timeout_ms) -> Result<Vec<String>, String>` + pure `take_lines(&str) -> Vec<String>` (first 500 non-`\r` lines)
  - `OpenCodeAdapter { home: PathBuf }`, `new()` (USERPROFILE/HOME), `with_home()`, `with_home_and_exe(home, exe)` (hermetic tests), `id()` = "opencode", `name()` = "OpenCode", `executable_names()` = `&["opencode"]`, capabilities {processes, sessions, mcp, skills, plugins, models, lifecycle_control: true; subagents, connections, logs: false}, `processes()` stem-filter like Claude

- [ ] **Step 1: Write failing tests**:
```rust
#[test] fn test_parse_version_strips_v() { /* "opencode v2.0.23" → "2.0.23"; "" → None */ }
#[test] fn test_missing_home_is_not_installed() { /* hermetic evaluate → false */ }
#[test] fn test_take_lines_caps_500() { /* 600-line input → 500 */ }
#[test] fn test_capabilities_match_verified_set() { /* per produces above */ }
#[test] fn test_processes_filters_by_stem() { /* opencode.exe match */ }
```
- [ ] **Step 2: Run tests, watch them fail**
Run (MSVC-prefixed PATH): `cargo test --lib agents::opencode`
Expected: FAIL — `opencode` module does not exist
- [ ] **Step 3: Implement** per interfaces (mirror Claude adapter structure incl. pure `evaluate()`).
- [ ] **Step 4: Run tests, watch them pass**
Run: `cargo test --lib`
Expected: all pass

### Task 2: Sessions via CLI

**Files:**
- Create fixtures: `agenthq/src-tauri/fixtures/opencode/sessions.json` (hand-written array of 2 entries with all 6 keys)
- Modify: `parser.rs` (add `parse_session_list(json: &str) -> Vec<SessionLite2>` reusing SessionLite shape? SessionLite lives in claude::parser — move a shared struct? Decision: define `pub struct CliSession { id: String, title: Option<String>, created_ms: Option<i64>, updated_ms: Option<i64>, project: Option<String> }` in opencode parser; adapter maps to AgentSession{id, status unknown}), `adapter.rs` (`sessions()` + `project_dir: Option<PathBuf>` field + `with_project_dir`)

**Interfaces:**
- Consumes: Task 1 adapter + `run_command_lines`
- Produces:
  - `parse_session_list` — serde parse array, per-item fields optional-tolerant (missing keys → None, never fail whole list); ms→s for AgentRow-less use (sessions only need id here; keep ms in struct)
  - `sessions()`: `project_dir` None → Ok(empty); Some(dir) → `run_command_lines(exe, ["session","list","--format","json","-n","100"], Some(dir), 10000)` → join stdout → parse → map (limit 100 rows)

- [ ] **Step 1: Write failing tests**:
```rust
#[test] fn test_session_fixture_parses() { /* 2 entries, ids/titles/directory */ }
#[test] fn test_session_missing_keys_tolerated() { /* [{id}] → title None, still 1 row */ }
#[test] fn test_sessions_without_project_is_empty() { /* with_home temp → Ok(empty), no subprocess spawned */ }
#[test] fn test_sessions_timeout_is_empty() { /* with_home_and_exe(powershell) + project_dir tmp + tiny timeout? — timeout param fixed 10000 in impl… make timeout a const fn param? Decision: sessions() uses const 10000; timeout test instead calls run_command_lines directly with 300ms vs sleeper → Err. And adapter maps Err → empty: pin via with_home_and_exe pointing at real `opencode`? No — hermetic: test parse path only + run_command_lines timeout test in detect.rs. */ }
```
Concretely: add to detect.rs tests `test_run_command_lines_times_out` (powershell sleep 30, 300ms → Err). Adapter test: `test_sessions_cli_error_is_empty` using exe = "agenthq-no-such-bin-xyz" → Ok(empty).
- [ ] **Step 2: Run tests, watch them fail**
Run: `cargo test --lib agents::opencode`
Expected: FAIL — `parse_session_list`/sessions missing or default Err
- [ ] **Step 3: Implement** per interfaces.
- [ ] **Step 4: Run tests, watch them pass**
Run: `cargo test --lib`
Expected: all pass

### Task 3: MCP + skills + plugins + models

**Files:**
- Create fixtures: `agenthq/src-tauri/fixtures/opencode/home/.config/opencode/opencode.json` (`{plugins: [...], mcp: {gh: {command: "x", env: {T: "SECRET3"}}}}`), `.../skills/demo-skill/SKILL.md` (description frontmatter), `.../skills/plain/` (no SKILL.md)
- Modify: `parser.rs` + `adapter.rs`

**Interfaces:**
- Consumes: Tasks 1–2
- Produces:
  - `mcp_servers()`: read global config `mcp` object keys → names (env never read); missing → empty
  - `skills()`: global skills dirs → {name, description via shared logic? claude::parser::skill_description is pub(crate)? It's `pub fn` in claude::parser — reuse via `crate::agents::claude::parser::skill_description` (no duplication)}
  - `plugins()`: `run_command_lines(exe, ["plugin","list"], None, 10000)` → skip header/separator/empty → first whitespace token per line → names; CLI missing/fails → empty
  - `models()`: `run_command_lines(exe, ["models"], None, 10000)` → non-empty trimmed lines → names; fail → empty
  - `subagents()`/`connections()` → default `Err(Unsupported)` (explicit, no override)

- [ ] **Step 1: Write failing tests**:
```rust
#[test] fn test_mcp_names_only() { /* gh present; SECRET3 absent from Debug */ }
#[test] fn test_skills_from_dirs() { /* demo-skill with description; plain without; stray file ignored */ }
#[test] fn test_plugin_lines_parsed() { /* feed parser fn parse_plugin_list(text) with header + --- + rows → ids */ }
#[test] fn test_models_skip_blanks() { /* parser fn parse_model_list with blanks → clean list */ }
#[test] fn test_no_secret_values_leak() { /* all five Debug outputs lack SECRET3 */ }
```
(`parse_plugin_list(&str)` + `parse_model_list(&str)` pure fns in parser.rs — CLI output parsing unit-testable without spawning.)
- [ ] **Step 2: Run tests, watch them fail**
Run: `cargo test --lib agents::opencode`
Expected: FAIL — methods default Err / fns missing
- [ ] **Step 3: Implement** per interfaces.
- [ ] **Step 4: Run tests, watch them pass**
Run: `cargo test --lib`
Expected: all pass

### Task 4: Research doc + full verification

**Files:**
- Modify: `docs/agent-research.md` (OpenCode section: verified sources, capabilities verdicts, project-level deferrals, connections-Unsupported rationale)

- [ ] **Step 1: Update research doc**
- [ ] **Step 2: Verify everything**
Run (MSVC-prefixed PATH): `cargo fmt`, `cargo check`, `cargo test --lib`; in `agenthq/`: `npx tsc --noEmit`, `npm run build`
Expected: fmt clean; check zero warnings; all tests pass; tsc/build exit 0
