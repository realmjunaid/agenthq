# Phase 3 (SQLite Database) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Persistent local SQLite store with versioned migrations and a tested repository, opened at app startup and exposed to the UI via one command.

**Architecture:** `rusqlite` (bundled SQLite, single binary) behind a small `Db` wrapper (`Mutex<Connection>`) held in Tauri managed state. Migrations are plain `.sql` files under `src-tauri/migrations/`, embedded with `include_str!`, applied in order via `PRAGMA user_version`. Errors surface as `String` (IPC-safe, no path/secret leakage).

**Tech Stack:** Rust 2021, rusqlite 0.32 (features `bundled`), serde (already present).

**Spec:** `MASTER_PLAN.md` §13 (table list + agents DDL verbatim), §40 (phase gate).

## Global Constraints

- Never make schema changes without a migration (numbered `.sql` file + `user_version` bump in the same task).
- `PRAGMA foreign_keys = ON` on every connection the repository opens.
- Timestamps are INTEGER epoch seconds.
- No secrets columns; events/messages never carry env values or tokens.
- No git in this repo yet — no commit steps; ledger is the record.
- Verification per task: `cargo test` in `agenthq/src-tauri` (exit 0, all tests). MSVC `link.exe` is NOT on inherited PATH — every Rust command must prepend `C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64`.

## Review Focus

- `migrate()` run twice on the same file must change nothing the second time — Task 1 pins `test_migrate_is_idempotent`.
- A child row referencing a missing parent must fail (FK on) — Task 2 pins `test_foreign_keys_enforced`.
- `open()` must create missing parent directories — Task 1 pins `test_open_creates_parent_dirs` with a nested temp path.
- A corrupt/zero-byte file at the DB path must produce `Err`, not panic — Task 1 pins `test_open_corrupt_file_errors`.
- IPC errors must be plain messages — Task 4 pins command returning `Result<_, String>`.

---

### Task 1: Dependency + open/migrate + agents table

**Files:**
- Modify: `agenthq/src-tauri/Cargo.toml` (add `rusqlite = { version = "0.32", features = ["bundled"] }`)
- Create: `agenthq/src-tauri/src/database/mod.rs`, `agenthq/src-tauri/src/database/schema.rs`, `agenthq/src-tauri/src/database/repository.rs`
- Create: `agenthq/src-tauri/migrations/001_agents.sql` (DDL verbatim from §13)
- Modify: `agenthq/src-tauri/src/lib.rs` (add `mod database;`)

**Interfaces:**
- Consumes: none
- Produces:
  - `database::repository::Db` — `pub struct Db(Mutex<Connection>)`
  - `Db::open(path: &Path) -> Result<Self, String>` — creates parent dirs, opens, sets `foreign_keys ON`, runs `migrate`
  - `migrate(conn: &Connection) -> Result<(), String>` — applies pending `MIGRATIONS` in order, bumps `user_version`
  - `schema::MIGRATIONS: &[(&str, u32)]` — `(include_str! sql, target_version)` pairs

- [ ] **Step 1: Write failing tests** in `repository.rs` `#[cfg(test)]`:
```rust
#[test] fn test_open_creates_parent_dirs() { /* open nested temp path, assert file exists */ }
#[test] fn test_migrate_is_idempotent() { /* open, migrate again, assert user_version unchanged */ }
#[test] fn test_open_corrupt_file_errors() { /* write garbage, assert open() is Err */ }
#[test] fn test_agents_table_exists() { /* assert sqlite_master contains agents */ }
```
- [ ] **Step 2: Run tests, watch them fail**
Run in `agenthq/src-tauri/` (MSVC-prefixed PATH): `cargo test database::`
Expected: FAIL — `database` module / `Db` does not exist
- [ ] **Step 3: Implement** Cargo dep + `mod database;` + schema.rs + repository.rs + `001_agents.sql` (spec DDL verbatim).
- [ ] **Step 4: Run tests, watch them pass**
Run: `cargo test`
Expected: all pass, including the 4 new tests

### Task 2: Remaining tables + FK enforcement

**Files:**
- Create: `agenthq/src-tauri/migrations/002_activity.sql` (`agent_processes`, `sessions`, `subagents`, `resource_snapshots`)
- Create: `agenthq/src-tauri/migrations/003_catalog.sql` (`mcp_servers`, `skills`, `plugins`, `models`, `connections`, `projects`)
- Create: `agenthq/src-tauri/migrations/004_events.sql` (`events`, `settings`)
- Modify: `agenthq/src-tauri/src/database/schema.rs` (register 002–004)

**Interfaces:**
- Consumes: Task 1 `MIGRATIONS`, `Db::open`
- Produces: all 12 §13 tables present after `open()`; child tables reference parents (`agent_processes.agent_id → agents(id)`, `sessions.agent_id → agents(id)`, `events.agent_id → agents(id)`, others use plain TEXT ids where cross-agent linkage is optional)

Foreign keys (minimal, non-nullable where the row is meaningless without a parent):
- `agent_processes(agent_id)`, `sessions(agent_id)`, `subagents(session_id → sessions(id))`, `resource_snapshots(agent_id)`, `events(agent_id NULL — global events allowed)`

- [ ] **Step 1: Write failing tests**:
```rust
#[test] fn test_all_twelve_tables_exist() { /* open fresh, assert sqlite_master has all 12 names */ }
#[test] fn test_foreign_keys_enforced() { /* insert agent_processes row with agent_id='ghost' → Err */ }
#[test] fn test_migrate_from_v1_applies_v2_to_v4() { /* open with only 001 applied (set user_version=1 + 001 schema), reopen, assert 12 tables */ }
```
- [ ] **Step 2: Run tests, watch them fail**
Run: `cargo test database::`
Expected: FAIL — tables missing
- [ ] **Step 3: Implement** the three `.sql` files + register in `MIGRATIONS`.
- [ ] **Step 4: Run tests, watch them pass**
Run: `cargo test`
Expected: all pass

### Task 3: Agent + event repository methods

**Files:**
- Modify: `agenthq/src-tauri/src/database/repository.rs`

**Interfaces:**
- Consumes: Tasks 1–2 schema
- Produces:
  - `pub struct AgentRow { id: String, name: String, agent_type: String, version: Option<String>, executable_path: Option<String>, installed: bool, running: bool, last_seen: Option<i64> }` (derives `Serialize`; Rust field `agent_type` maps the DB column `type` and serializes as `agent_type`)
  - `Db::upsert_agent(&self, row: &AgentRow) -> Result<(), String>` (INSERT … ON CONFLICT(id) DO UPDATE)
  - `Db::get_agent(&self, id: &str) -> Result<Option<AgentRow>, String>`
  - `Db::list_agents(&self) -> Result<Vec<AgentRow>, String>` (ORDER BY name)
  - `pub struct EventRow { id: i64, ts: i64, level: String, agent_id: Option<String>, event: String, message: String }`
  - `Db::insert_event(&self, level: &str, agent_id: Option<&str>, event: &str, message: &str) -> Result<i64, String>`
  - `Db::list_recent_events(&self, limit: i64) -> Result<Vec<EventRow>, String>` (ORDER BY ts DESC, id DESC)

- [ ] **Step 1: Write failing tests**:
```rust
#[test] fn test_upsert_then_get_roundtrip() { /* upsert claude row, get → equal, installed/running flags */ }
#[test] fn test_upsert_updates_existing() { /* upsert same id twice, version changes, still 1 row */ }
#[test] fn test_list_agents_ordered() { /* insert b,a → returns a,b */ }
#[test] fn test_events_insert_and_recent_order() { /* insert 3, list_recent(2) → newest 2 first */ }
```
- [ ] **Step 2: Run tests, watch them fail**
Run: `cargo test database::`
Expected: FAIL — methods do not exist
- [ ] **Step 3: Implement** the structs + methods.
- [ ] **Step 4: Run tests, watch them pass**
Run: `cargo test`
Expected: all pass

### Task 4: Startup wiring + IPC command + frontend types

**Files:**
- Modify: `agenthq/src-tauri/src/lib.rs` (`.setup()` opens `%APPDATA%/com.agenthq.app/agenthq.db` via `app.path().app_data_dir()`, `manage()`s `Db`; register `list_agents` command; keep `greet`)
- Create: `agenthq/src/types/agent.ts` (`AgentRow` mirror: same field names as serialized Rust — `agent_type: string`, `version: string | null` etc.)
- Modify: `agenthq/src/pages/AgentsPage.tsx` (placeholder keeps EmptyState; add comment pointing at `list_agents` for Phase 14 — no fetch yet)

**Interfaces:**
- Consumes: Task 3 `Db`, `AgentRow`
- Produces: `#[tauri::command] fn list_agents(db: State<Db>) -> Result<Vec<AgentRow>, String>`; frontend `AgentRow` type

- [ ] **Step 1: Write the command + wiring + types file**
`setup` creates data dir (`create_dir_all`), opens DB, manages state; on open failure, log + continue without DB? No — fail fast with message (startup without DB is a broken install). Decide: `expect()` with clear message naming the path.
- [ ] **Step 2: Verify Rust**
Run in `agenthq/src-tauri/` (MSVC-prefixed PATH): `cargo check`
Expected: exit 0, no warnings on new code
- [ ] **Step 3: Verify frontend**
Run in `agenthq/`: `npx tsc --noEmit` then `npm run build`
Expected: both exit 0
