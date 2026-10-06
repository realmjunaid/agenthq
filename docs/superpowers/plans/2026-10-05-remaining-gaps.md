# Remaining Gaps Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Close the 5 documented deferrals: frontend tests, file watcher, measured performance, tight capability, subagents verdict.

**Architecture:** `notify` crate watches the 3 known config roots with a 2s debounce; changes re-run the quiet engine sync and emit one `config.changed` event. Vitest covers pure frontend helpers. Capability drops to listen/unlisten + opener. Subagents stay Not available (no source exists — fabricating would break §0 rule 9).

**Tech Stack:** Rust 2021 + notify 8, vitest, Tauri 2 ACL.

**Spec:** `MASTER_PLAN.md` §33 (watch files), §34 (UI tests), §32 (measure first), §31 (minimum permissions), §0 rules 8–9.

## Global Constraints

- Watcher covers known agent config dirs only — never full-disk scans. Missing dirs are skipped, never errors.
- Debounce 2s; coalesce bursts into one sync + one event.
- `config.changed` is a new BusEvent name — add to the TS union and the Rust emitter in the same task.
- Capability change gets a runtime check: every page + tray open + refresh, zero "not allowed" console errors, or revert.
- Perf numbers are measured on the release exe (startup ms, idle WorkingSet/CPU) and written into `docs/performance.md` with the method. No tuning without a profile showing the bottleneck.
- Subagents: NO implementation. Verdict documented: no adapter exposes a countable source; process-tree children are shown in the Processes tab already.
- No git commits.
- Verification per Rust task: `cargo test --lib` (MSVC-prefixed PATH). Frontend: `npx tsc --noEmit` + `npm run build` + `npx vitest run`.

## Review Focus

- A deleted watched dir must not crash the watcher — Task 2 pins by code read (remove-watch on error, log only).
- Rapid successive writes must produce one sync, not N — Task 2 pins debounce test with fake clock/counter.
- `config.changed` payload must carry names/counts only, never file contents — Task 2 pins message assertion.
- Tight capability must not break listen/openPath at runtime — Task 4 pins by runtime check (all pages, tray open, refresh).
- Perf numbers must name method + machine state, not bare claims — Task 3 pins by doc read.

---

### Task 1: Vitest + pure-helper tests

**Files:**
- Create: `agenthq/vitest.config.ts`, `agenthq/src/lib/format.test.ts`, `agenthq/src/pages/AgentDetailPage.test.ts` (treeDepths only)
- Modify: `agenthq/package.json` (devDeps vitest + `test` script)

**Interfaces:**
- Consumes: `formatBytes/formatCpu/formatCount/formatAgo/errMessage`, exported `treeDepths`
- Produces: `npx vitest run` green (format edge cases: 0 B, NaN, negative; treeDepths: orphan→0, cycle→bounded, chain depth)

- [ ] **Step 1: Write failing tests** — create test files first, run `npx vitest run`. Expected: FAIL — vitest not installed / cannot resolve.
- [ ] **Step 2: Install + wire** — `npm install -D vitest`, config, test script.
- [ ] **Step 3: Run, watch pass** — `npx vitest run`. Expected: all pass.

### Task 2: File watcher

**Files:**
- Modify: `Cargo.toml` (+ `notify = "8"`), new `src-tauri/src/watch.rs`, `lib.rs` (spawn in setup), `events.rs` (`config.changed` emitter reuse), `types/events.ts` (union +1)

**Interfaces:**
- Consumes: `startup_sync(&AgentManager, &Db)`, adapter home-dir knowledge (watch `~/.claude`, `~/.config/opencode`, `~/.codex` if present)
- Produces: `watch::spawn(app_handle)` — RecursiveMode, 2s debounce via mpsc + last-fire timestamp, on fire: `startup_sync` + `config.changed` ("config changed: N dirs") — errors logged only

- [ ] **Step 1: Write failing tests** — `watch::should_fire(last: Instant, now: Instant) -> bool` pure debounce test + coalesce test. Run `cargo test --lib watch::`. Expected: FAIL — module missing.
- [ ] **Step 2: Implement** per interfaces.
- [ ] **Step 3: Run, watch pass** — `cargo test --lib`. Expected: all pass.

### Task 3: Measured performance

**Files:**
- Modify: `docs/performance.md` (measured table + method)

**Interfaces:**
- Consumes: release exe from the final build
- Produces: startup ms (process start → first window shown, measured via stopwatch + window title poll), idle WorkingSet MB + CPU% (5s sample via Get-Process/Get-Counter after 60s idle), dashboard render note

- [ ] **Step 1: Measure** on release exe, idle 60s, record raw numbers.
- [ ] **Step 2: Write doc** — numbers + method + machine state. No tuning claims without bottleneck evidence.

### Task 4: Tight capability + runtime check

**Files:**
- Modify: `capabilities/default.json` → `["core:event:allow-listen", "core:event:allow-unlisten", "opener:default"]`

**Interfaces:**
- Consumes: frontend API inventory (listen/unlisten, invoke ungated, openPath)
- Produces: minimal capability file

- [ ] **Step 1: Edit capability.**
- [ ] **Step 2: Runtime check** — release run: open every page, trigger refresh, tray open/settings, watch console for "not allowed". Any failure → revert to `core:default` + ledger ruling.
