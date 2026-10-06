# Phase 14 (Dashboard UI) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Live dashboard composing real backend data: overview stats, per-agent cards, recent activity, and event-driven refresh.

**Architecture:** One aggregated `get_dashboard` IPC call (agent rows + counts + resource sums + system totals) keeps the frontend to a single fetch per refresh; a `get_events` reader exposes recent activity; the frontend subscribes to `agenthq://event` and re-fetches (incremental scope arrives with later pages; this phase re-fetches dashboard state, never full reloads). New code in `dashboard.rs` (aggregate) + `lib.rs` (commands) + `pages/DashboardPage.tsx` + `lib/api.ts`.

**Tech Stack:** Rust 2021, React 19 + TS, Tauri IPC (`invoke`/`listen`), existing primitives.

## Global Constraints

- Unknown stays Unknown: subagents render `Not available` (no producer exists); missing version shows `Unknown`; zero counts render as `0` only when the capability is supported (else `Not available`).
- Bytes→human formatting lives in ONE helper (`formatBytes`) shared by cards and stats.
- Event listener is registered once with cleanup on unmount (no duplicate subscriptions).
- No polling loops in this phase (event-driven only; manual Refresh button provided).
- No git — no commit steps; ledger is the record.
- Verification: Rust tasks `cargo test --lib` (exit 0, MSVC-prefixed PATH); frontend tasks `npx tsc --noEmit` + `npm run build` (exit 0). (`cargo test --bin agenthq` harness OS-blocked; gate uses `--lib`.)

## Review Focus

- Empty DB (fresh install) must render empty states, never crash on null — Task 2 pins by code read (reviewer checks every `.` access on invoke results has a fallback) + `get_dashboard` on empty DB covered by a Rust test returning empty vecs.
- `get_dashboard` on a DB with agents but no snapshots must not divide by zero or NaN — Task 1 pins `test_empty_snapshot_sums_zero` (sums 0.0/0, counts from DB).
- Unmount must remove the listener (StrictMode double-mount safe) — Task 2 pins by code read (unlisten in effect cleanup, reviewer verifies).
- A failing `invoke` must render ErrorState with retry, never blank — Task 3 pins by code read (error branch present on every fetch path).
- Stale closure over `fetchState` in the event callback must not freeze updates — Task 2 pins by code read (callback calls stable `load` ref, no captured state).

---

### Task 1: Backend aggregate (`get_dashboard` + `get_events`)

**Files:**
- Create: `agenthq/src-tauri/src/dashboard.rs`
- Modify: `lib.rs` (`mod dashboard;` + 2 commands), `agenthq/src/types/dashboard.ts`

**Interfaces:**
- Consumes: `Db` readers, `ProcessMonitor` snapshot, `AgentManager` adapter executable_names, `system_stats`
- Produces:
  - `pub struct DashboardAgent { agent: AgentRow, ram_bytes: u64, cpu: f32, sessions: u64, subagents: Option<u64> /* always None in Phase 14 */, mcp: u64, skills: u64 }`
  - `pub struct DashboardData { agents: Vec<DashboardAgent>, total_ram_bytes: u64, total_cpu: f32, running: u64, system: SystemStats }`
  - `pub fn build_dashboard(db: &Db, monitor: &Mutex<ProcessMonitor>, manager: &Mutex<AgentManager>) -> Result<DashboardData, String>` — refresh monitor, snapshot; per agent row: match snapshot exe stems via adapter names (needs adapter lookup by id → add `AgentManager::executable_names_for(id) -> Vec<String>` (owned Strings, empty when unknown); sums; counts via list_sessions/list_mcp_servers/list_skills filtered by agent_id; subagents None
  - `#[tauri::command] fn get_dashboard(monitor, manager, db) -> Result<DashboardData, String>`
  - `#[tauri::command] fn get_events(db: State<Db>, limit: Option<i64>) -> Result<Vec<EventRow>, String>` — clamp limit 1..=200, default 50. (EventRow needs Serialize + ts field! Add `#[derive Serialize]` + `pub ts` — wait EventRow HAS ts field already? EventRow { id, ts, level, agent_id, event, message } ✓ just needs Serialize derive.)
  - TS `dashboard.ts` mirrors

- [ ] **Step 1: Write failing tests**:
```rust
#[test] fn test_empty_db_empty_dashboard() { /* temp DB, empty monitor/manager → agents empty, sums zero */ }
#[test] fn test_empty_snapshot_sums_zero() { /* agent row present, empty snapshot → ram 0, cpu 0.0, counts from DB */ }
#[test] fn test_resource_sums_match_stems() { /* fake snapshot rows (2 claude procs) via real monitor? Monitor wraps live System — instead test pure `sum_for(names, snapshot)` helper with fabricated ProcessInfo vecs */ }
```
So dashboard.rs exposes pure `pub fn sum_for(names: &[String], snapshot: &[ProcessInfo]) -> (u64, f32)` — tested directly. Manager gets `executable_names_for`.
- [ ] **Step 2: Run tests, watch them fail**
Run (MSVC-prefixed PATH): `cargo test --lib dashboard::`
Expected: FAIL — `dashboard` module does not exist
- [ ] **Step 3: Implement** per interfaces (+ EventRow Serialize, manager method).
- [ ] **Step 4: Run tests, watch them pass**
Run: `cargo test --lib`
Expected: all pass

### Task 2: DashboardPage live UI

**Files:**
- Create: `agenthq/src/lib/api.ts` (typed `invoke` wrappers: getDashboard, getEvents, refreshAgents, refreshAll? — refresh chain helper `refreshAll()` invoking refresh_agents/sessions/mcp/skills/plugins sequentially), `agenthq/src/hooks/useAgentEvents.ts` (`listen(FRONTEND_EVENT)` → onEvent callback, cleanup)
- Modify: `agenthq/src/pages/DashboardPage.tsx` (real layout), `agenthq/src/components/ui/AgentCard.tsx`? No — AgentCard takes AgentStats strings; reuse with formatted values + subagents "Not available"? AgentCard renders Metric per stat — for subagents pass value "—" with hint? Decision: pass `"N/A"` + hint "Not available for this agent". Keep component untouched.
- Create: `agenthq/src/lib/format.ts` (`formatBytes`, `formatCpu`)

**Interfaces:**
- Consumes: Task 1 commands + Phase 2 primitives + Phase 13 contract
- Produces: DashboardPage states — loading (first fetch) → error (ErrorState+retry) → empty (EmptyState when agents empty) → loaded (StatCards: Agents/Running/MCP/RAM + AgentCard grid + Recent activity list + Refresh button + InspectorPanel? No — inspector belongs to detail page Phase 15; dashboard keeps single column + activity)

- [ ] **Step 1: Write api.ts + format.ts + useAgentEvents.ts + DashboardPage**
Fetch on mount; `listen` → reload; unlisten cleanup; Refresh button runs refresh chain then reload (disable while pending).
- [ ] **Step 2: Verify**
Run in `agenthq/`: `npx tsc --noEmit` then `npm run build`
Expected: both exit 0
- [ ] **Step 3: Verify by read** (reviewer-grade self-check before final review): every invoke result access has fallback; unlisten in cleanup; error branch on all fetches; event callback references stable loader only.

### Task 3: Agents page wiring + full verification

**Files:**
- Modify: `agenthq/src/pages/AgentsPage.tsx` (list agents via getDashboard agents or list_agents + per-agent StatusDot; EmptyState when none; ErrorState on failure)

**Interfaces:**
- Consumes: Tasks 1–2

- [ ] **Step 1: Write AgentsPage list** (Card per agent: name, version ?? Unknown, status dot, sessions count via get_sessions? Keep light: reuse getDashboard payload (sessions counts included? DashboardAgent has sessions count ✓ — reuse getDashboard for both pages).
- [ ] **Step 2: Verify**
Run: `npx tsc --noEmit`, `npm run build`
Expected: both exit 0
- [ ] **Step 3: Full backend verification**
Run (MSVC-prefixed PATH): `cargo fmt`, `cargo check`, `cargo test --lib`
Expected: fmt clean; check zero warnings; all pass
