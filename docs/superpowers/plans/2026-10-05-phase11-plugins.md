# Phase 11 (Plugins Engine) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Normalized cross-agent plugin inventory (version/path/source/enabled) with DB persistence and IPC, mirroring the Phase 9/10 engine shape.

**Architecture:** New `plugins` module (model + normalize + persist). Adapters gain additive `plugins_details()` (default `Unsupported`) returning `PluginDetail { name, version, path, source, enabled }`; engine normalizes (id `{agent}:{name}`), persists via replace-per-agent transaction, serves reads from DB.

**Tech Stack:** Rust 2021, std only.

**Spec:** `MASTER_PLAN.md` §21 (model, installed-only-on-evidence).

## Global Constraints

- Installed = directory entry exists (Claude/Codex) or CLI lists it (OpenCode). `enabled=true` exactly then; no disabled inference anywhere.
- Missing version → None (never `""`, never guessed). Missing path/source → None.
- Replace-per-agent persistence (delete + insert in one transaction).
- plugins table already has all columns — no migration.
- No git — no commit steps; ledger is the record.
- Verification per task: `cargo test --lib` in `agenthq/src-tauri` (exit 0). MSVC-prefixed PATH required. (`cargo test --bin agenthq` harness OS-blocked; gate uses `--lib`.)

## Review Focus

- A stray FILE inside a plugin dir scan must not become a plugin — Task 2 pins (fixtures already contain stray files for Claude/Codex).
- A `plugin list` row with missing VERSION column must yield version None, not shift columns — Task 1 pins `test_short_row_version_none` (`"lonely\n"` → name only).
- Double refresh must not duplicate rows — Task 3 pins count stability.
- Same plugin name from marketplace `plugins/` and `external_plugins/` must not collide PK — Task 2 pins: Claude source carries market+kind? Id `{agent}:{name}` collides across kinds. Decision: source = `{market}/{kind}` (e.g. `claude-plugins-official/plugins`), id stays `{agent}:{name}`, and same-name collisions across kinds dedupe first-wins in adapter (deterministic order: `plugins/` before `external_plugins/`). Pin `test_kind_collision_first_wins`.
- Round-trip preserves version/path/source/enabled exactly — Task 3 pins.

---

### Task 1: Trait detail type + OpenCode row parsing

**Files:**
- Modify: `agenthq/src-tauri/src/agents/traits.rs` (add `PluginDetail { name, version: Option<String>, path: Option<String>, source: Option<String>, enabled: bool }` + default `fn plugins_details(&self) -> CapabilityResult<Vec<PluginDetail>> { Err(Unsupported("plugins")) }`), `agenthq/src-tauri/src/agents/opencode/parser.rs` (add `parse_plugin_rows(text) -> Vec<PluginRow{name, version: Option<String>}>`; keep `parse_plugin_list` delegating to names)

**Interfaces:**
- Consumes: existing plugin list parsing
- Produces: `PluginDetail`, default method, `PluginRow`, `parse_plugin_rows` (columns: 1st token name, 2nd token version if present AND line has ≥2 tokens AND second token isn't a separator; header/`---`/blank rules unchanged)

- [ ] **Step 1: Write failing tests**:
```rust
#[test] fn test_plugin_rows_with_versions() { /* "ID VERSION SOURCE\nsuperpowers 8ca22db x\nsecond 1.0 y" → [(superpowers,Some(8ca22db)),(second,Some(1.0))] */ }
#[test] fn test_short_row_version_none() { /* "lonely\n" → [(lonely,None)] */ }
#[test] fn test_plugin_list_still_names_only() { /* existing behavior intact */ }
```
- [ ] **Step 2: Run tests, watch them fail**
Run (MSVC-prefixed PATH): `cargo test --lib`
Expected: FAIL — `PluginDetail`/`parse_plugin_rows` do not exist
- [ ] **Step 3: Implement** per interfaces.
- [ ] **Step 4: Run tests, watch them pass**
Run: `cargo test --lib`
Expected: all pass

### Task 2: Adapter overrides

**Files:**
- Modify: the three adapters (`plugins_details()`)

**Interfaces:**
- Consumes: Task 1 types
- Produces:
  - Claude: walk `plugins/marketplaces/*/plugins/*` then `*/external_plugins/*` dirs (files skipped); {version None, path Some(dir), source Some(`{market}/{kind}`), enabled true}; same-name cross-kind → first wins (plugins/ before external_plugins/)
  - OpenCode: `plugin list` rows → {version, path None, source None, enabled true}; CLI missing/fails → empty
  - Codex: `plugins/*` dir names → {version None, path Some(dir), source None, enabled true}

- [ ] **Step 1: Write failing tests** (existing fixtures):
```rust
#[test] fn test_claude_details_evidence() { /* plugin-a/b present, version None, path ends with name, enabled true, source contains market name */ }
#[test] fn test_kind_collision_first_wins() { /* temp market with plugins/dup + external_plugins/dup → 1 row, source ends with "plugins" */ }
#[test] fn test_opencode_details_versions() { /* hermetic? needs CLI… versions tested at parser level (T1); adapter test asserts empty-without-exe via with_home_and_exe missing bin → Ok(empty). Plus live-CLI test only if exe resolves? Skip live: assert Ok(empty) with bad exe. */ }
#[test] fn test_codex_details_evidence() { /* plugin-a present, stray.txt absent */ }
```
- [ ] **Step 2: Run tests, watch them fail**
Run: `cargo test --lib agents::`
Expected: FAIL — methods return default `Err(Unsupported)`
- [ ] **Step 3: Implement** per interfaces.
- [ ] **Step 4: Run tests, watch them pass**
Run: `cargo test --lib`
Expected: all pass

### Task 3: Engine + persistence + IPC

**Files:**
- Create: `agenthq/src-tauri/src/plugins.rs`, `agenthq/src/types/plugin.ts`
- Modify: `lib.rs` (`mod plugins;` + `refresh_plugins` + `get_plugins` commands), `repository.rs` (`replace_agent_plugins`, `list_plugins`), `manager.rs` (`collect_plugins` + test)

**Interfaces:**
- Consumes: Tasks 1–2
- Produces:
  - `pub struct Plugin { id, name, version: Option<String>, path: Option<String>, source: Option<String>, enabled: bool, agent_id: String }`
  - `pub fn normalize(agent_id: &str, details: Vec<PluginDetail>) -> Vec<Plugin>` — id `{agent}:{name}`
  - `Db::replace_agent_plugins`, `Db::list_plugins(agent_id: Option<&str>)` (enabled stored INTEGER 0/1)
  - `collect_plugins()` (skip Unsupported), `refresh_plugins`/`get_plugins`, TS mirror

- [ ] **Step 1: Write failing tests**:
```rust
#[test] fn test_normalize_ids() { /* a:x ids, enabled passthrough */ }
#[test] fn test_roundtrip_preserves_all_fields() { /* version/path/source/enabled survive replace+list */ }
#[test] fn test_refresh_is_idempotent() { /* replace twice → same count */ }
#[test] fn test_collect_plugins_skips_unsupported() { /* fake all-Err → empty */ }
```
- [ ] **Step 2: Run tests, watch them fail**
Run: `cargo test --lib`
Expected: FAIL — `plugins` module / methods do not exist
- [ ] **Step 3: Implement** per interfaces.
- [ ] **Step 4: Run tests + full verification**
Run (MSVC-prefixed PATH): `cargo fmt`, `cargo check`, `cargo test --lib`; in `agenthq/`: `npx tsc --noEmit`, `npm run build`
Expected: fmt clean; check zero warnings; all pass; tsc/build exit 0
