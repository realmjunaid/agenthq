# Phase 4 (Process Monitor) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rust process monitor that snapshots local processes (PID/PPID/name/exe/CPU/RAM/start/cmdline) with cached metadata, plus system totals and an IPC surface.

**Architecture:** `sysinfo` crate behind `monitoring::process` (one cached `System`, metadata snapshotted once per PID, CPU/RAM refreshed per tick) and `monitoring::resources` (system totals). Pure process-tree grouping (`children_of`) is dependency-free and unit-tested; live collection has smoke tests only. Frontend gets two read commands; polling cadence (2s default) belongs to Phase 14+ UI, not this phase.

**Tech Stack:** Rust 2021, sysinfo 0.32, serde (present).

**Spec:** `MASTER_PLAN.md` §14 (fields, caching, 2s interval, child grouping), §0 rules 8/12/13 (no secrets, no full-disk scans, reasonable polling).

## Global Constraints

- Command lines are truncated to 512 chars at collection (may carry secrets; full audit in Phase 21).
- Refresh only process CPU/memory specifics — never disk/network enumeration here.
- PIDs as `u32` throughout; no `usize`/`i32` mixing at boundaries.
- CPU% from sysinfo needs two refreshes to be nonzero — first tick may report 0.0; documented, never faked.
- No git — no commit steps; ledger is the record.
- Verification per task: `cargo test --lib` in `agenthq/src-tauri` (exit 0). MSVC `link.exe` NOT on inherited PATH — prepend `C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64` for every Rust command. (`cargo test --bin agenthq` harness is OS-blocked in this env; gate uses `--lib`.)

## Review Focus

- First refresh reports CPU 0.0 — Task 2 pins a test that refreshes twice and asserts the call succeeds (not a nonzero value).
- A PID in the snapshot may exit before use — Task 1 pins `children_of` tolerating unknown PIDs (empty vec, no panic).
- Cmdline must never exceed 512 chars — Task 2 pins `test_cmdline_truncated`.
- Unknown exe/cmd (permission denied) must be `None`, not `""` or panic — Task 2 pins `test_empty_exe_and_cmd_become_none` on the `ProcessInfo::new` constructor.
- Full snapshot on a busy machine must stay bounded — Task 3 pins the IPC command capping rows at 500 sorted by CPU desc.

---

### Task 1: Types + pure tree grouping

**Files:**
- Create: `agenthq/src-tauri/src/monitoring/mod.rs`, `agenthq/src-tauri/src/monitoring/process.rs`
- Modify: `agenthq/src-tauri/src/lib.rs` (add `mod monitoring;`), `agenthq/src-tauri/Cargo.toml` (add `sysinfo = "0.32"`)

**Interfaces:**
- Consumes: none
- Produces:
  - `pub struct ProcessInfo { pid: u32, parent_pid: Option<u32>, name: String, exe: Option<String>, cpu: f32, ram_bytes: u64, started_at: i64, cmdline: Option<String> }` (derives `Clone, Serialize`)
  - `pub fn children_of(pid: u32, parent_of: &HashMap<u32, u32>) -> Vec<u32>` — direct children; unknown pid → empty
  - `pub fn parent_map(procs: &[ProcessInfo]) -> HashMap<u32, u32>` — pid → parent for rows with a parent

- [ ] **Step 1: Write failing tests** in `process.rs`:
```rust
#[test] fn test_children_of_returns_direct_children_only() { /* map 1->{2,3}, 2->{4}: children_of(1)=={2,3}, children_of(2)=={4} */ }
#[test] fn test_children_of_unknown_pid_is_empty() { /* children_of(99999) == [] */ }
#[test] fn test_parent_map_skips_root_processes() { /* row with parent None absent from map */ }
```
- [ ] **Step 2: Run tests, watch them fail**
Run (MSVC-prefixed PATH): `cargo test --lib monitoring::`
Expected: FAIL — `monitoring` module / functions do not exist
- [ ] **Step 3: Implement** dep + `mod monitoring;` + types + two pure fns (no sysinfo calls yet).
- [ ] **Step 4: Run tests, watch them pass**
Run: `cargo test --lib`
Expected: all pass

### Task 2: Live snapshot + system totals

**Files:**
- Modify: `agenthq/src-tauri/src/monitoring/process.rs` (add `ProcessMonitor`), `agenthq/src-tauri/src/monitoring/mod.rs`
- Create: `agenthq/src-tauri/src/monitoring/resources.rs`

**Interfaces:**
- Consumes: Task 1 `ProcessInfo`
- Produces:
  - `pub struct ProcessMonitor` — `new() -> Self` (empty cache), `refresh(&mut self)` (one pass: new PIDs snapshotted with metadata, all PIDs get fresh cpu/ram), `snapshot(&self) -> Vec<ProcessInfo>`, `remove_exited(&mut self)` folded into `refresh` (drop PIDs gone from System)
  - `pub fn truncate_cmdline(cmd: &str) -> String` — first 512 chars (char-boundary safe)
  - `pub const DEFAULT_TICK_SECS: u64 = 2` — resource update interval (§14 default; UI phases schedule ticks)
  - `pub struct SystemStats { total_cpu: f32, used_mem_bytes: u64, total_mem_bytes: u64 }`, `pub fn system_stats() -> SystemStats`
  - `impl ProcessInfo { pub fn new(pid: u32, parent_pid: Option<u32>, name: String, exe: Option<String>, cpu: f32, ram_bytes: u64, started_at: i64, cmdline: Option<String>) -> Self }` — normalizes empty exe/cmdline to `None`, truncates cmdline via `truncate_cmdline`

- [ ] **Step 1: Write failing/smoke tests**:
```rust
#[test] fn test_truncate_cmdline_caps_at_512() { /* 2000-char input → len ≤ 512 chars, char-boundary */ }
#[test] fn test_empty_exe_and_cmd_become_none() { /* ProcessInfo::new with Some("") exe/cmd → None,None */ }
#[test] fn test_tick_interval_default_is_two_seconds() { /* DEFAULT_TICK_SECS == 2 */ }
#[test] fn test_snapshot_contains_current_process() { /* refresh twice; find own pid; assert exe.is_some() and name non-empty */ }
#[test] fn test_double_refresh_succeeds() { /* two refresh() calls, snapshot non-empty — pins the two-tick CPU contract */ }
#[test] fn test_system_stats_sane() { /* total_mem_bytes > 0, used <= total */ }
```
- [ ] **Step 2: Run tests, watch them fail**
Run: `cargo test --lib monitoring::`
Expected: FAIL — `ProcessMonitor`/`truncate_cmdline`/`system_stats` do not exist
- [ ] **Step 3: Implement** with sysinfo (`System::new`, `refresh_processes_specifics(ProcessRefreshKind::nothing().with_cpu().with_memory().with_cmd().with_exe()...)`, `RunTime`→started_at epoch). Missing exe/cmd → None.
- [ ] **Step 4: Run tests, watch them pass**
Run: `cargo test --lib`
Expected: all pass

### Task 3: IPC commands + frontend types

**Files:**
- Modify: `agenthq/src-tauri/src/lib.rs` (manage `Mutex<ProcessMonitor>`? No — construct per call? Decision: hold `std::sync::Mutex<ProcessMonitor>` in managed state so metadata cache persists across ticks; commands lock, refresh, snapshot)
- Create: `agenthq/src/types/process.ts` (`ProcessInfo` + `SystemStats` mirrors)

**Interfaces:**
- Consumes: Tasks 1–2
- Produces:
  - `#[tauri::command] fn get_process_snapshot(monitor: State<Mutex<ProcessMonitor>>) -> Result<Vec<ProcessInfo>, String>` — refresh, sort by cpu desc, truncate to 500 rows
  - `#[tauri::command] fn get_system_stats() -> SystemStats`
  - TS mirrors with same field names (`ram_bytes`, `started_at`, `parent_pid: number | null`, etc.)

- [ ] **Step 1: Write commands + state + TS mirrors**
- [ ] **Step 2: Verify Rust**
Run (MSVC-prefixed PATH): `cargo check`
Expected: exit 0, no warnings on new code
- [ ] **Step 3: Verify frontend**
Run in `agenthq/`: `npx tsc --noEmit` then `npm run build`
Expected: both exit 0
