# Phase 10 (Skills Engine) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Normalized cross-agent skill inventory (scope Global/Project, path, description) with DB persistence and IPC, mirroring the Phase 9 MCP engine shape.

**Architecture:** New `skills` module (model + normalize + persist). Adapters gain additive `skills_details()` (default `Unsupported`) returning `SkillDetail { name, description, path, scope, source }`; engine normalizes (id `{agent}:{scope}:{name}`), persists via replace-per-agent transaction, serves reads from DB. No filesystem watching in this phase (explicit refresh commands; watcher arrives with the event bus, Phase 13).

**Tech Stack:** Rust 2021, std only.

**Spec:** `MASTER_PLAN.md` §20 (model, Global/Project scopes).

## Global Constraints

- Descriptions come from SKILL.md `description:` frontmatter only (existing `skill_description` helper); never read skill bodies into the DB.
- Scope is Global for all Phase 10 rows (no adapter has project context yet); project rows arrive with Phase 12. No fake Project rows.
- Replace-per-agent persistence (delete + insert in one transaction).
- skills table already has all columns (id/name/description/path/scope/source/agent_id) — no migration.
- No git — no commit steps; ledger is the record.
- Verification per task: `cargo test --lib` in `agenthq/src-tauri` (exit 0). MSVC-prefixed PATH required. (`cargo test --bin agenthq` harness OS-blocked; gate uses `--lib`.)

## Review Focus

- Duplicate skill dir names across scopes must not collide — Task 3 pins scoped ids (`{agent}:{scope}:{name}` distinct for global/project rows).
- Double refresh must not duplicate rows — Task 3 pins count stability.
- A skill dir that vanishes between list and read must not fail the refresh — Task 1 pins `skills_details` tolerating disappearing dirs (read_dir collected names first? No — read entries live; a removed SKILL.md yields description None, not Err; pin `test_missing_skill_md_is_none_description`).
- Non-UTF8 SKILL.md must not kill enumeration — Task 1 pins `read_to_string` Err → None (binary SKILL.md fixture? Use invalid-UTF8 bytes file in temp dir — construct in-test via fs::write, no fixture needed).
- Scope strings in DB must round-trip exactly (`global`/`project`) — Task 3 pins enum↔string mapping both directions.

---

### Task 1: Trait detail type + shared enumeration

**Files:**
- Modify: `agenthq/src-tauri/src/agents/traits.rs` (add `SkillScope { Global, Project }` + `SkillDetail { name, description: Option<String>, path: Option<String>, scope: SkillScope, source: Option<String> }` + default `fn skills_details(&self) -> CapabilityResult<Vec<SkillDetail>> { Err(Unsupported("skills")) }`)

**Interfaces:**
- Consumes: existing `skill_description` helper, `SkillInfo`
- Produces: `SkillScope` (Serialize lowercase), `SkillDetail`, default method. Shared helper `pub fn enumerate_skill_dirs(dir: &Path, scope: SkillScope, source: &str) -> Vec<SkillDetail>` in claude::parser (reused by all three adapters; skips dotfiles + non-dirs; description via skill_description; path = dir path string).

Where should enumerate live? Claude parser already has skill_description; add `enumerate_skill_dirs` there (pub). OpenCode/Codex adapters call it with their skills dir + scope Global + their source strings ("opencode-global", "codex-global", Claude "claude-global").

- [ ] **Step 1: Write failing tests** (in claude parser tests + traits compile):
```rust
#[test] fn test_enumerate_global_skills() { /* temp dir with a/ (SKILL.md w/ description) + b/ (no SKILL.md) + .hidden/ + file.txt → 2 rows, a has description+path, b description None */ }
#[test] fn test_missing_skill_md_is_none_description() { /* covered above via b/ */ }
#[test] fn test_non_utf8_skill_md_is_none() { /* write invalid UTF-8 SKILL.md bytes → row present, description None */ }
#[test] fn test_scope_serializes_lowercase() { /* serde_json::to_string(Global) == "\"global\"" */ }
```
- [ ] **Step 2: Run tests, watch them fail**
Run (MSVC-prefixed PATH): `cargo test --lib`
Expected: FAIL — `SkillDetail`/`enumerate_skill_dirs` do not exist
- [ ] **Step 3: Implement** per interfaces.
- [ ] **Step 4: Run tests, watch them pass**
Run: `cargo test --lib`
Expected: all pass

### Task 2: Adapter overrides

**Files:**
- Modify: the three adapters (`skills_details()` returning `enumerate_skill_dirs` over their global skills dir with their source string)

**Interfaces:**
- Consumes: Task 1 helper
- Produces: Claude/OpenCode/Codex `skills_details()` → Global rows; existing name-only `skills()` untouched

- [ ] **Step 1: Write failing tests** (fixture homes — existing fixtures have skills dirs):
```rust
#[test] fn test_claude_details_global() { /* demo-skill Global + path ends with demo-skill, plain description None */ }
#[test] fn test_opencode_details_global() { /* same shape */ }
#[test] fn test_codex_details_global() { /* same shape */ }
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
- Create: `agenthq/src-tauri/src/skills.rs`, `agenthq/src/types/skill.ts`
- Modify: `lib.rs` (`mod skills;` + `refresh_skills` + `get_skills` commands), `repository.rs` (`replace_agent_skills`, `list_skills`), `manager.rs` (`collect_skills` + test)

**Interfaces:**
- Consumes: Tasks 1–2
- Produces:
  - `pub struct Skill { id, name, description: Option<String>, path: Option<String>, scope: SkillScope, source: Option<String>, agent_id: String }` — reuses traits::SkillScope (Serialize)
  - `pub fn normalize(agent_id: &str, details: Vec<SkillDetail>) -> Vec<Skill>` — id `{agent}:{scope-lower}:{name}`
  - `Db::replace_agent_skills`, `Db::list_skills(agent_id: Option<&str>)`
  - `collect_skills()` on manager (skip Unsupported)
  - `refresh_skills` (source per adapter = `{agent}-global`), `get_skills`
  - TS mirror

- [ ] **Step 1: Write failing tests**:
```rust
#[test] fn test_normalize_scoped_ids() { /* Global vs Project same name → distinct ids */ }
#[test] fn test_scope_roundtrip() { /* replace + list preserves scope/description/path/source */ }
#[test] fn test_refresh_is_idempotent() { /* replace twice → same count */ }
#[test] fn test_collect_skills_skips_unsupported() { /* fake all-Err → empty */ }
```
- [ ] **Step 2: Run tests, watch them fail**
Run: `cargo test --lib`
Expected: FAIL — `skills` module / methods do not exist
- [ ] **Step 3: Implement** per interfaces.
- [ ] **Step 4: Run tests + full verification**
Run (MSVC-prefixed PATH): `cargo fmt`, `cargo check`, `cargo test --lib`; in `agenthq/`: `npx tsc --noEmit`, `npm run build`
Expected: fmt clean; check zero warnings; all pass; tsc/build exit 0
