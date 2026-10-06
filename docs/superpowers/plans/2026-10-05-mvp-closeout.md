# MVP Closeout Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close the MVP gaps found 2026-10-05: models/connections persistence + IPC + UI, process tree, configuration view, projects page, docs, packaging, final QA.

**Architecture:** Same engine shape as MCP/skills/plugins (normalize → replace-per-agent txn → refresh/get commands). Adapters already return `ModelInfo{name}` / Claude `ConnectionInfo{name}`; engines persist those. Frontend reuses existing primitives; no new npm deps.

**Tech Stack:** Rust 2021, rusqlite (present), React 19 + TS, Tauri IPC.

**Spec:** `MASTER_PLAN.md` §43 (MVP definition of done), §7 (capability model), §0 rules 8–9 (no secrets, no fabrication).

## Global Constraints

- Model provider: split `provider/name` on first `/` when present, else None. Never invent a provider.
- Connections provider: `env` for Claude settings-env keys (key names only, values never read). OpenCode/Codex stay Unsupported → empty lists, UI shows Not available.
- Replace-per-agent transaction for both tables (idempotent refresh).
- No secrets in DB, events, tray, notifications, or logs.
- No git commits (user deferred git init).
- Verification per Rust task: `cargo test --lib` in `agenthq/src-tauri` (exit 0, MSVC-prefixed PATH). Frontend tasks: `npx tsc --noEmit` + `npm run build` in `agenthq/` (exit 0).

## Review Focus

- A model named without `/` must persist with provider None, never empty-string provider — Task 1 pins.
- Double refresh must not duplicate models/connections — Task 1 pins count stability.
- Unsupported adapters must yield empty lists, never Err — Task 1 pins collect skips.
- Process tree must tolerate orphan rows (parent pid absent) — Task 2 pins pure builder test.
- Configuration tab must show only stored row fields, no guessed paths — Task 2 pins by code read.

---

### Task 1: Models + connections persistence + IPC

**Files:**
- Create: `agenthq/src-tauri/src/models.rs`, `agenthq/src/types/model.ts` (Model + Connection mirrors)
- Modify: `repository.rs` (replace/list for both), `manager.rs` (collect_models/collect_connections + tests), `lib.rs` (4 commands + registration)

**Interfaces:**
- Consumes: trait `ModelInfo{name}`, `ConnectionInfo{name}`; existing `Db`, `AgentManager` patterns
- Produces:
  - `pub struct Model { id, name, provider: Option<String>, agent_id }`, `normalize_models(agent_id, Vec<ModelInfo>) -> Vec<Model>` (id `{agent}:{name}`, provider split on first `/`)
  - `pub struct Connection { id, name, provider, configured: bool, agent_id }`, `normalize_connections(agent_id, Vec<ConnectionInfo>) -> Vec<Connection>` (id `{agent}:{name}`, provider `env`, configured true)
  - `Db::replace_agent_models/list_models`, `Db::replace_agent_connections/list_connections`
  - `collect_models/collect_connections` (skip Unsupported)
  - `refresh_models/get_models`, `refresh_connections/get_connections` commands

- [ ] **Step 1: Write failing tests** in `models.rs` + repository + manager:
```rust
#[test] fn test_provider_split() { /* "opencode/big-pickle" → provider Some("opencode"), name full */ }
#[test] fn test_bare_name_provider_none() { /* "auto" → provider None */ }
#[test] fn test_connections_provider_env() { /* "FOO" → provider "env", configured true */ }
#[test] fn test_replace_idempotent() { /* replace twice → same count (both tables) */ }
#[test] fn test_collect_skips_unsupported() { /* fake all-Err adapter → both empty */ }
```
- [ ] **Step 2: Run tests, watch them fail** — `cargo test --lib models::` Expected: FAIL — module does not exist
- [ ] **Step 3: Implement** per interfaces
- [ ] **Step 4: Run tests, watch them pass** — `cargo test --lib` Expected: all pass

### Task 2: Detail UI — models, connections, tree, configuration + projects page

**Files:**
- Modify: `AgentDetailPage.tsx` (Models tab via get_models; Connections section; Processes tab indented tree via parent_pid; Configuration tab from agent row fields), `api.ts` (getModels/getConnections wrappers), `AppShell.tsx` + `Sidebar.tsx` (Projects page), Create: `ProjectsPage.tsx`

**Interfaces:**
- Consumes: Task 1 commands + existing `get_projects`, agent row
- Produces: Models list (or Not available when empty + capability false → message "Not available for this agent"); Connections list (Claude env-key names only); tree indent by depth with orphan tolerance; Configuration shows id/type/version/exe/installed/running/last_seen only; Projects page lists path/name/agents with Open button

- [ ] **Step 1: Implement UI** (no test infra in frontend; verification is tsc + build)
- [ ] **Step 2: Verify** — `npx tsc --noEmit`, `npm run build` Expected: both exit 0

### Task 3: Docs + packaging + final QA

**Files:**
- Create: `docs/security.md` (pointer + minimum-permissions summary per §37), update `README.md` (what/install/dev/build/agents/privacy), `docs/development.md` (commands + gates)
- Run: `cargo fmt`, `cargo check`, `cargo test --lib`, `npx tsc --noEmit`, `npm run build`, `tauri build` (MSI/NSIS)

- [ ] **Step 1: Write docs**
- [ ] **Step 2: Verify everything** per §40 completion rule (formatter, typecheck, Rust tests, build, warnings check)
