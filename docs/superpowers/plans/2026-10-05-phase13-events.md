# Phase 13 (Event Bus) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Transition-based local event system: refresh pipelines emit DB events + live Tauri frontend events only on real changes, with an installed-event for first-seen agents.

**Architecture:** New `events` module (pure set-diff + emit helpers). `AgentManager::detect_all` additionally emits `agent.installed` on first-seen-installed rows. Each `refresh_*` command diffs new-vs-stored id sets, inserts transition events, and emits them live over Tauri (`agenthq://event`) via `AppHandle` — emit failures are logged, never fatal. Frontend subscribes in Phase 14; this phase ships the payload contract + a TS listener type.

**Tech Stack:** Rust 2021, Tauri 2 events, std only.

**Spec:** `MASTER_PLAN.md` §23 (event list, incremental UI, no full reloads).

## Global Constraints

- No change → no events: identical refresh output emits nothing (event-storm rule).
- Emit failure never fails the refresh command (log-and-continue).
- Event messages carry names/ids/counts only — no paths, no env, no secrets.
- First-seen logic: `agent.installed` fires when no previous agents row exists AND installed=true (reinstall after row exists = no event).
- No git — no commit steps; ledger is the record.
- Verification per task: `cargo test --lib` in `agenthq/src-tauri` (exit 0). MSVC-prefixed PATH required. (`cargo test --bin agenthq` harness OS-blocked; gate uses `--lib`.)

## Review Focus

- A refresh producing identical data must insert zero events — Task 2 pins `test_no_change_no_events` per pipeline (sessions + mcp representatives; skills/plugins share the helper).
- `agent.installed` must fire exactly once (first-seen), not on every detect — Task 1 pins with two detects (installed persists, one event).
- Emit failure (no listener / closed channel) must not propagate — Task 2 pins by code read: emit wrapped in `let _ =` with no `?`; reviewer verifies.
- Diff must be by id set, not order — Task 1 pins `test_diff_ignores_order`.
- Transition messages must not contain filesystem paths — Task 2 pins `test_messages_have_no_paths` (refresh against fixture-shaped data, assert no `\` or `/` in messages... careful: names could contain slashes legitimately? Names are dir basenames/CLI ids — assert messages match `^[A-Za-z0-9 _.:-]+$`? Too strict for future. Pin: messages contain no `:\` and no `/.claude`-style segments? Simplest honest pin: assert `!message.contains('\\')` for the temp-dir-based test where paths WOULD leak if interpolated. Keep that.)

---

### Task 1: Diff helper + installed event

**Files:**
- Create: `agenthq/src-tauri/src/events.rs`
- Modify: `agenthq/src-tauri/src/lib.rs` (`mod events;`), `agenthq/src-tauri/src/agents/manager.rs` (installed emission)

**Interfaces:**
- Consumes: existing `insert_event`, `AgentRow`
- Produces:
  - `pub struct ChangeSet { pub added: Vec<String>, pub removed: Vec<String> }`, `pub fn diff_ids(old: &[String], new: &[String]) -> ChangeSet` (order-insensitive, sorted output, deduped)
  - `pub const FRONTEND_EVENT: &str = "agenthq://event"`
  - Manager `detect_all`: after upsert, if previous row was None AND installed → `insert_event("info", Some(id), "agent.installed", "{name} detected")`

- [ ] **Step 1: Write failing tests**:
```rust
#[test] fn test_diff_added_removed() { /* old [a,b] new [b,c] → added [c], removed [a] */ }
#[test] fn test_diff_ignores_order() { /* same set shuffled → empty */ }
#[test] fn test_diff_dedupes() { /* new [a,a] → added [a] once */ }
#[test] fn test_installed_fires_once() { /* fake installed adapter, temp DB: detect → 1 installed event; detect again → still 1 */ }
```
- [ ] **Step 2: Run tests, watch them fail**
Run (MSVC-prefixed PATH): `cargo test --lib events::`
Expected: FAIL — `events` module does not exist (installed test fails on missing hook — write it against detect_all after impl? No: write all tests now; compile fails = RED)
- [ ] **Step 3: Implement** per interfaces.
- [ ] **Step 4: Run tests, watch them pass**
Run: `cargo test --lib`
Expected: all pass

### Task 2: Refresh diffs + live emit

**Files:**
- Modify: `lib.rs` (all four `refresh_*` commands take `app: tauri::AppHandle`, diff + insert + emit)

**Interfaces:**
- Consumes: Task 1 `diff_ids`, `FRONTEND_EVENT`
- Produces: per refresh command: read OLD ids from DB first (`list_*` filtered by agent), compute new ids, `diff_ids`, insert `session.started/stopped`, `mcp.connected/disconnected` (added→connected? No — Connected means liveness; use `mcp.configured`/`mcp.removed`? §23 names are connected/disconnected. Honest mapping: added → `mcp.connected` is WRONG (no liveness). Decision: emit `mcp.configured` for added, `mcp.removed` for removed — deviates from §23 names deliberately (ledger it; §23 predates the no-Connected rule). skills/plugins/config → `{kind}.changed` single event per agent with counts when sets differ (not per-item). sessions → per-id started/stopped. Then `let _ = app.emit(FRONTEND_EVENT, &payload)` where payload mirrors the DB event row (id/ts/level/agent/event/message) — emit AFTER insert so listener can fetch by id.
- Event names used: agent.installed/started/stopped/error(detect_failed exists), session.started/stopped, mcp.configured/removed, skill.changed, plugin.changed. subagent.*: no producer yet (all adapters Unsupported) → unemitted, noted. config.changed: no config watcher yet → unemitted, noted (Phase 13 defers file watching to... watcher was deferred to Phase 13 in Phase 10 plan! Decision: file watching (notify crate = new dep) is OUT — refresh commands are the change detectors; config.changed fires when? No config tracking exists → unemitted, noted.)

- [ ] **Step 1: Write failing tests** (pure parts; emit path is read-verified):
```rust
#[test] fn test_no_change_no_events() { /* same sessions twice via upsert+diff helper → ChangeSet empty (call diff_ids on id lists) */ }
#[test] fn test_messages_have_no_paths() { /* build messages via the same format strings? — extract `fn started_msg(name) / stopped_msg / changed_msg(kind, added, removed)` pure fns in events.rs; assert no backslash */ }
```
So Task 2 also adds pure message builders to events.rs: `session_started_msg(id)`, `session_stopped_msg(id)`, `collection_changed_msg(kind, added: usize, removed: usize)`.
- [ ] **Step 2: Run tests, watch them fail**
Run: `cargo test --lib events::`
Expected: FAIL — builders do not exist
- [ ] **Step 3: Implement** builders + command wiring (diff old/new per agent, insert, `let _ = app.emit(...)`).
- [ ] **Step 4: Run tests, watch them pass**
Run: `cargo test --lib`
Expected: all pass

### Task 3: TS contract + full verification

**Files:**
- Create: `agenthq/src/types/events.ts` (`BusEvent` mirror + `FRONTEND_EVENT = "agenthq://event"` + event-name union)
- Modify: nothing else (no UI subscription until Phase 14)

**Interfaces:**
- Consumes: Tasks 1–2 payload shape (id/ts/level/agent_id/event/message)

- [ ] **Step 1: Write the contract file**
`export const FRONTEND_EVENT = "agenthq://event"` + `export type BusEvent = { id: number; ts: number; level: string; agent_id: string | null; event: AgentEventName; message: string }` + union of emitted names: agent.installed/started/stopped/error/detect_failed, session.started/stopped, mcp.configured/removed, skill.changed, plugin.changed.
- [ ] **Step 2: Verify everything**
Run (MSVC-prefixed PATH): `cargo fmt`, `cargo check`, `cargo test --lib`; in `agenthq/`: `npx tsc --noEmit`, `npm run build`
Expected: fmt clean; check zero warnings; all pass; tsc/build exit 0
