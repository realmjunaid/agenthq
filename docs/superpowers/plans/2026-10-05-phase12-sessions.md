# Phase 12 (Session & Project Engine) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Normalized session + project inventory (confidence-graded, Unknown-preserving) with DB persistence and IPC, mirroring the Phase 9–11 engine shape.

**Architecture:** New `sessions` module (Session/Project models + normalize + persist). Adapters gain additive `sessions_details(project_dir: Option<&Path>)` (default `Unsupported`) returning `SessionDetail`; projects come from a separate `projects_details()` default-`Unsupported` returning `ProjectDetail` (Claude-only in Phase 12: slug→path resolution via `.claude.json` keys). Sessions upsert by scoped id; projects replace-all. Migration 006 rebuilds `projects` without the `UNIQUE(path)` constraint (same path, two agents).

**Tech Stack:** Rust 2021, serde_json (present), std only.

**Spec:** `MASTER_PLAN.md` §22 (models, Unknown rule) + §8 (confidence).

## Global Constraints

- Confidence: transcript/model-derived = Confirmed (Claude), CLI/index-derived = Detected (OpenCode/Codex). Status is always Unknown in Phase 12 (no aliveness proof).
- Unresolvable project linkage → None, never guessed paths. Slug without matching `.claude.json` key yields no project row (path is NOT NULL).
- OpenCode ms epochs → seconds (ms/1000). Sub-second data is not preserved — documented.
- Session ids scoped `{agent}:{native}`; project ids `{agent}:{slug}`.
- Schema changes only via numbered migrations (`006_projects_relax.sql`: rebuild `projects` without UNIQUE, preserving rows).
- Project agents derive from sessions (`DISTINCT agent_id WHERE project_id`), not a column.
- No git — no commit steps; ledger is the record.
- Verification per task: `cargo test --lib` in `agenthq/src-tauri` (exit 0). MSVC-prefixed PATH required. (`cargo test --bin agenthq` harness OS-blocked; gate uses `--lib`.)
- Fixtures extend existing adapter fixture homes (new files only).

## Review Focus

- Two different paths slugifying identically must not merge projects — Task 1 pins `test_slug_collision_first_wins`.
- OpenCode ms→s conversion must divide (not store ms) — Task 2 pins `1791202563262 → 1791202563`.
- Unmatched slug must yield None project linkage, not a fabricated path — Task 1 pins.
- Double refresh must not duplicate sessions or projects — Task 3 pins counts.
- A session whose transcript has no model/timestamps must still appear with Unknowns — Task 1 pins (fixture session without assistant lines).

---

### Task 1: Trait types + Claude sessions/projects

**Files:**
- Modify: `agenthq/src-tauri/src/agents/traits.rs` (add `SessionConfidence { Confirmed, Detected, Estimated, Unknown }` Serialize lowercase, `SessionDetail { id, project: Option<String>, status: String, model: Option<String>, started_at: Option<i64>, last_activity: Option<i64>, confidence: SessionConfidence }`, `ProjectDetail { slug, path: Option<String>, last_seen: Option<i64> }`, defaults `fn sessions_details(&self, _project_dir: Option<&Path>) -> CapabilityResult<Vec<SessionDetail>> { Err(Unsupported("sessions")) }`, `fn projects_details(&self) -> CapabilityResult<Vec<ProjectDetail>> { Err(Unsupported("projects")) }`)
- Modify: `.../claude/parser.rs` (`slugify(path: &str) -> String` — every non-`[A-Za-z0-9]` char → `-`; `resolve_projects(slugs, claude_json_doc) -> HashMap slug→path`; `parse order: sessions first, then projects with last_seen = max session activity per slug`), `.../claude/adapter.rs` (overrides)
- Create fixtures: `agenthq/src-tauri/fixtures/claude/home/.claude/projects/` with `E--demo/{<uuid>.jsonl (full: user+assistant+model lines), <uuid2>.jsonl (user lines only, no model)}` — realistic slugs as dirs

**Interfaces:**
- Consumes: existing `parse_session_file`/`SessionLite`
- Produces:
  - `sessions_details(_)`: enumerate `projects/*/*.jsonl` → SessionDetail {id: stem, project: None (linkage resolved at engine level? No — adapter sets project: Some(slug) always; engine maps slug→project-id via its project rows, None when unmapped), status "unknown", model, started_at, last_activity, confidence Confirmed}
  - Wait — decision needed: adapter sets project=Some(slug) or engine resolves? Engine owns project rows → adapter emits slug, engine maps slug→id (via its own resolved map? engine would need slug→path logic = duplication). Cleaner: adapter emits project: Option<resolved PATH> (it owns slugify+keys), engine matches path→project row id, None when unmapped. SessionDetail.project = Option<path>. SessionLite.project (slug) stays internal.
  - `projects_details()`: enumerate project dirs → slugify-match `.claude.json` projects keys → rows {slug, Some(path), last_seen = max parsed activity}; unmatched slugs SKIPPED (no row); collision (two keys, one slug) → first wins (sorted keys for determinism)
  - `slugify`: `C:/Users/j4u87` → `C--Users-j4u87`; `E:/TempMail Project` → `E--TempMail-Project`

- [ ] **Step 1: Write failing tests**:
```rust
#[test] fn test_slugify_verified_examples() { /* the two real slugs above */ }
#[test] fn test_session_details_full() { /* fixture full session: model Some, started/last set, confidence Confirmed, status unknown */ }
#[test] fn test_session_without_model_is_unknowns() { /* user-only session: model None, started Some?, confidence Confirmed (transcript-derived) */ }
#[test] fn test_projects_resolve_paths() { /* fixture .claude.json projects {"E:/demo"} + slug dir E--demo → row with path; last_seen = max activity */ }
#[test] fn test_unmatched_slug_skipped() { /* slug dir with no key → no row */ }
#[test] fn test_slug_collision_first_wins() { /* keys "a/b" + "a:b" both slugify "a-b"? "a/b"→"a-b", "a:b"→"a-b" → one row, sorted-first key wins */ }
```
Fixture `.claude.json` needs `projects` — existing fixture file? Phase 6 fixture home has .claude.json (mcp fixture). ADD a projects key? That file is read by mcp tests (extra keys harmless). Add `"projects": {"E:/demo": {}}`. Session fixture dirs E--demo with 2 jsonl files (write minimal user/assistant lines with timestamps + model).
- [ ] **Step 2: Run tests, watch them fail**
Run (MSVC-prefixed PATH): `cargo test --lib agents::claude`
Expected: FAIL — `SessionDetail`/`sessions_details` do not exist
- [ ] **Step 3: Implement** per interfaces.
- [ ] **Step 4: Run tests, watch them pass**
Run: `cargo test --lib`
Expected: all pass

### Task 2: OpenCode + Codex details

**Files:**
- Modify: both adapters (`sessions_details` overrides; `projects_details` stays default-Unsupported with explicit comment)

**Interfaces:**
- Consumes: Task 1 trait types; OpenCode `parse_session_list`+`CliSession`; Codex `parse_session_index`
- Produces:
  - OpenCode `sessions_details(project_dir)`: None → Ok(empty); Some(dir) → CLI (10s, Err→empty) → map {id: `{agent}:{native}`? NO — id scoping happens in ENGINE normalize (adapter emits native id; engine prefixes). Adapter emits native ids; engine `normalize` prefixes `{agent}:`. (MCP/skills precedent: engine prefixes. Keep.) model None, started/created_ms/1000, last updated_ms/1000, confidence Detected, status unknown, project None}
  - Codex `sessions_details(_)`: index rows → {native id, model None, started None, last None, confidence Detected, status unknown, project None}

- [ ] **Step 1: Write failing tests**:
```rust
#[test] fn test_opencode_ms_to_seconds() { /* fixture sessions.json entry 1791202563262 → started 1791202563 */ }
#[test] fn test_opencode_no_project_is_empty() { /* None → empty, no spawn */ }
#[test] fn test_codex_index_details_detected() { /* fixture index → 2 rows, confidence Detected, model None */ }
```
- [ ] **Step 2: Run tests, watch them fail**
Run: `cargo test --lib agents::`
Expected: FAIL — methods return default `Err(Unsupported)`
- [ ] **Step 3: Implement** per interfaces (OpenCode reuses CliSession parse; Codex reuses IndexSession).
- [ ] **Step 4: Run tests, watch them pass**
Run: `cargo test --lib`
Expected: all pass

### Task 3: Engine + persistence + IPC + migration 006

**Files:**
- Create: `agenthq/src-tauri/src/sessions.rs`, `agenthq/src-tauri/migrations/006_projects_relax.sql`, `agenthq/src/types/session.ts` (+ project type inside)
- Modify: `lib.rs` (`mod sessions;` + `refresh_sessions(project_dir: Option<String>)` + `get_sessions` + `refresh_projects` + `get_projects`), `repository.rs` (`upsert_sessions`, `list_sessions`, `replace_projects`, `list_projects`, `project_agent_ids`), `manager.rs` (`collect_sessions(project_dir)`, `collect_projects` + tests), `schema.rs` (register 006)

**Interfaces:**
- Consumes: Tasks 1–2
- Produces:
  - `pub struct Session { id, agent_id, project_id: Option<String>, status, model: Option<String>, started_at: Option<i64>, last_activity: Option<i64>, confidence: SessionConfidence }` (reuses traits enum), `pub struct Project { id, name, path, last_seen: Option<i64>, agent_ids: Vec<String> }` (agent_ids filled at read time, not stored)
  - `normalize_sessions(agent_id, project_of: &HashMap<String,String> /* slug-or-path → project id */, details) -> Vec<Session>` — id `{agent}:{native}`; project link: detail.project (a PATH for Claude, None others) matched against known project paths → id, else None
  - Hmm — Claude detail.project = resolved path; engine needs path→id map: refresh_projects runs first (or same refresh: projects then sessions). `refresh_sessions` implementation: collect projects per adapter too? Engine flow in command: for each adapter → details + projects → normalize → persist. Manager gets `collect_projects()`; command builds path→id map from normalized projects, then sessions.
  - `normalize_projects(agent_id, details: Vec<ProjectDetail>) -> Vec<Project>` — id `{agent}:{slug}`, name = basename of path (/ and \ aware), path; skips path-less (already filtered at adapter, double-guard: skip None)
  - `006_projects_relax.sql`: `CREATE TABLE projects_new (...same minus UNIQUE...); INSERT INTO projects_new SELECT * FROM projects; DROP TABLE projects; ALTER TABLE projects_new RENAME TO projects;` (+ preserve nothing else; indexes: none on projects)
  - `Db::upsert_sessions(&[Session])` (INSERT OR REPLACE), `list_sessions(agent)`, `Db::replace_projects(&[Project])` (txn delete+insert), `list_projects()` (+ fill agent_ids via DISTINCT query), `project_agent_ids(project_id)`
  - `refresh_sessions(project_dir: Option<String>)` (collect_sessions(dir) + collect_projects + persist both), `get_sessions(agent_id?)`, `refresh_projects()`? Folded into refresh_sessions (projects refresh alongside). Decision: single `refresh_sessions` handles both (projects enumerated from adapters, not per-cwd). get_projects reads DB.
  - TS mirrors

- [ ] **Step 1: Write failing tests**:
```rust
#[test] fn test_session_ids_scoped() { /* claude:abc → distinct from opencode:abc */ }
#[test] fn test_project_link_or_none() { /* known path → Some(id); unknown → None */ }
#[test] fn test_upsert_no_duplicates() { /* upsert twice → 1 row, updated fields win */ }
#[test] fn test_project_agents_derive_from_sessions() { /* two agents same project → both ids */ }
#[test] fn test_collect_sessions_skips_unsupported() { /* fake → empty */ }
#[test] fn test_006_keeps_rows_drops_unique() { /* temp db at v5 with dup-path attempt? — v5 has UNIQUE so dup insert fails pre-migration; test: migrate v5 db with 1 project row → v6 keeps row; then insert same-path second row → Ok */ }
```
- [ ] **Step 2: Run tests, watch them fail**
Run: `cargo test --lib`
Expected: FAIL — `sessions` module / methods do not exist
- [ ] **Step 3: Implement** per interfaces (incl. updating Phase 3's `test_migrate_from_v1_applies_v2_to_v5` to expect version 6, since 006 is the new latest).
Run (MSVC-prefixed PATH): `cargo fmt`, `cargo check`, `cargo test --lib`; in `agenthq/`: `npx tsc --noEmit`, `npm run build`
Expected: fmt clean; check zero warnings; all pass; tsc/build exit 0
