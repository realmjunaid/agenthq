# Grok Adapter Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Grok CLI adapter implementing the agent trait from verified local sources (grok 1.0.46, research 2026-10-05).

**Architecture:** `agents::grok::{adapter, paths, parser}` mirroring sibling shape — explicit base paths, fixture-driven tests, no new crates (hand-rolled RFC3339 + percent-decode). CLI-as-API where files are insufficient (`mcp list --json`, `plugin list --json`); files for the rest. Registered in `lib.rs` setup; `~/.grok` added to watcher roots.

**Tech Stack:** Rust 2021, serde_json (present), std only.

**Spec:** `MASTER_PLAN.md` §41 + §6/§7. Verified facts: exe `~/.grok/bin/grok.exe` (+PATH); `--version` → `grok 1.0.46 (2765805b9442) [stable]`; config `~/.grok/` (config.toml safe sections cli/marketplace/ui/privacy; `auth.json` EXISTS — never opened; `settings_cache.json` keys payload/signature — opaque, never parsed); sessions `sessions/<url-encoded-project>/<hash>/` with `usage.json` {sessionId, updatedAt, session.primaryModelId} + project-level `prompt_history.jsonl` {timestamp, session_id} + `chat_history.jsonl` {type, content}; skills `skills/*` dirs (SKILL.md `description:`); installed plugins = `installed-plugins/` dirs minus `*.lock` (currently only registry.lock → empty); models `models_cache.json` `models` string array; `mcp list --json` → `[]` exit 0; `plugin list --json` → `[]` exit 0; `sessions list` cwd-scoped ("No sessions found" here) → file discovery instead.

## Global Constraints

- NEVER open `auth.json`; NEVER parse `settings_cache.json` (opaque signed blob). Pin by source-grep test (dynamic needle, like Codex).
- `sessions list` CLI is cwd-scoped and empty here → sessions come from files only.
- MCP/plugin JSON schemas for non-empty states are unverified → accept array of objects with string `name` (fallback `id`), else plain strings; anything else skipped. Names only, env counted never read.
- RFC3339 + percent-decode hand-rolled (no chrono/percent-encoding deps); unparseable → None, never guessed.
- Missing CLI/config → installed=false / empty lists, never panic. CLI timeout 10s, Err → empty.
- `connections()`/`subagents()` stay trait-default Unsupported. No git commits.
- Verification per task: `cargo test --lib` (MSVC-prefixed PATH). Frontend untouched (generic pages pick up registration).

## Review Focus

- `auth.json` must have no reader — Task 1 pins by source grep.
- Bare `grok` token vs version token — Task 1 pins `test_parse_version_second_token`.
- Malformed jsonl/usage lines must not kill enumeration — Task 2 pins skip.
- URL-encoded project dirs must decode to real paths — Task 2 pins `E%3A%5CTempMail%20Project`.
- `*.lock` files must not become plugins — Task 3 pins.
- models_cache without `models` array → empty — Task 3 pins.

---

### Task 1: Paths + install + version + registration

**Files:** Create `agents/grok/{mod.rs,paths.rs,parser.rs,adapter.rs}`; modify `agents/mod.rs`, `lib.rs` (register), `watch.rs` (add `~/.grok` root).

**Interfaces:**
- Produces: `paths::{grok_dir, config_file, session_index→n/a, skills_dir, plugins_dir, models_cache, known_exes}` (PATH + `~/.grok/bin/grok.exe`), `parser::parse_version` (first version-shaped token), `GrokAdapter::{new,with_home,with_home_and_exe,evaluate}`, id `"grok"`, name `"Grok CLI"`, `executable_names()=&["grok"]`, capabilities {processes,sessions,mcp,skills,plugins,models,lifecycle_control:true; subagents,connections,logs:false}, `processes()` stem-filter.

- [ ] Step 1: failing tests — parse_version, evaluate-false, releases n/a (no releases glob; known_exes prefers PATH then .grok/bin), caps, processes stem, no-auth-reader grep.
- [ ] Step 2: run `cargo test --lib agents::grok` → FAIL, module missing.
- [ ] Step 3: implement + register + watcher root.
- [ ] Step 4: `cargo test --lib` → all pass.

### Task 2: Sessions + projects (files)

**Files:** fixtures `fixtures/grok/home/.grok/sessions/E%3A%5Cdemo/<hash>/usage.json` + `prompt_history.jsonl` (2 sessions, 1 bad line); modify parser (`parse_rfc3339`, `percent_decode`, `parse_usage`, session enumeration) + adapter (`sessions`, `sessions_details`, `projects_details`).

**Interfaces:**
- Produces: `SessionDetail{id(native hash), project:Some(decoded path)|None, status unknown, model: usage primaryModelId, started: min prompt ts, last: usage updatedAt, confidence Confirmed}`; `ProjectDetail{slug: encoded dirname, path: Some(decoded), last_seen: max session last}`; unmatched n/a (every dir decodes to a path — path always Some).

- [ ] Step 1: failing tests — rfc3339 Z + offset + garbage→None; percent_decode fixture string; session full/bare; projects resolve + last_seen max.
- [ ] Step 2: run → FAIL (fns missing).
- [ ] Step 3: implement.
- [ ] Step 4: `cargo test --lib` → all pass.

### Task 3: MCP + skills + plugins + models

**Files:** fixtures (config.toml safe subset, models_cache.json, skills dirs, installed-plugins/plugin-a + stray.lock); modify parser + adapter.

**Interfaces:**
- Produces: `mcp_servers()` = `mcp list --json` names (banner/empty→empty, Err→empty); `mcp_details()` same rows as Unknown transport (command/url/env taken only if string/object-typed in the JSON object, else defaults); `skills()`/`skills_details()` via shared `enumerate_skill_dirs` (source grok-global); `plugins()`/`plugins_details()` installed-plugins dirs minus `*.lock` (version None, enabled true); `models()` models_cache strings; connections default Unsupported.

- [ ] Step 1: failing tests — banner→empty, lock skipped, models array + missing-key→empty, skills dirs, no-secret pins (fixture canary in a skipped field? MCP env: fixture JSON object with env containing canary → assert absent).
- [ ] Step 2: run → FAIL (default Err).
- [ ] Step 3: implement.
- [ ] Step 4: `cargo test --lib` → all pass.

### Task 4: Research doc + gate + rebuild + relaunch

**Files:** modify `docs/agent-research.md` (Grok section), `PROGRESS.md`.

- [ ] Step 1: research doc.
- [ ] Step 2: `cargo fmt`, `cargo check` (zero warnings), `cargo test --lib`, `npx tsc --noEmit`, `npm run build`, `npm test`.
- [ ] Step 3: kill app, `npm run tauri build`, relaunch, confirm window + grok card via dashboard check.
