# Phase 5 (Agent Detection Engine) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Adapter trait + AgentManager that detects installed/running agents from multiple sources and persists state, proven with fake adapters (real Claude/OpenCode/Codex adapters land in Phases 6–8).

**Architecture:** `agents::traits` declares the `AgentAdapter` contract and shared DTOs; `agents::detect` holds source probes (PATH lookup, version-command with timeout); `agents::manager` registers adapters, runs installation + running detection over a `ProcessMonitor` snapshot, upserts `agents` rows and appends `events` rows. No new crates — PATH walk + timeout use std only.

**Tech Stack:** Rust 2021, std only (no new dependencies).

**Spec:** `MASTER_PLAN.md` §6 (adapter abstraction), §15 (manager responsibilities, multi-source detection).

## Global Constraints

- Core holds no agent-specific names/paths — those live in future adapters; manager tests use inline fake adapters only.
- Never rely on a single detection mechanism (§15) — `detect_installation` combines PATH + known paths + config dir + version command where the adapter provides them.
- Version commands run with a 5s timeout and are killed on expiry; never block startup on a hung CLI.
- Capability-gated data: unsupported adapter methods return `Err(Unsupported)`; manager stores nothing for them.
- No git — no commit steps; ledger is the record.
- Verification per task: `cargo test --lib` in `agenthq/src-tauri` (exit 0). MSVC-prefixed PATH required. (`cargo test --bin agenthq` harness is OS-blocked; gate uses `--lib`.)

## Review Focus

- A hung `--version` CLI must not stall detection — Task 1 pins `test_version_command_times_out` (spawns a sleeper, 300ms budget, asserts Err within 2s wall time).
- PATH lookup must honor `.exe`/PATHEXT on Windows — Task 1 pins `test_find_on_path_finds_cargo` (real PATH binary) and `test_find_on_path_missing_is_none`.
- An adapter whose version command fails must still record installed=true when the executable exists — Task 2 pins fake-adapter case (exe present, version Err → installed, version None).
- A process whose exe path is None must never match an adapter — Task 2 pins `test_running_ignores_none_exe`.
- Detection re-runs must flip running true→false when the process exits — Task 2 pins with two snapshots (present then absent) on the same temp DB.

---

### Task 1: Trait + DTOs + probe helpers

**Files:**
- Create: `agenthq/src-tauri/src/agents/mod.rs`, `agenthq/src-tauri/src/agents/traits.rs`, `agenthq/src-tauri/src/agents/detect.rs`
- Modify: `agenthq/src-tauri/src/lib.rs` (add `mod agents;`)

**Interfaces:**
- Consumes: `monitoring::process::ProcessInfo` (for later tasks), `database::repository::AgentRow` (for later tasks)
- Produces:
  - `pub struct Unsupported(pub &'static str)` + `pub type CapabilityResult<T> = Result<T, Unsupported>`
  - `pub struct AgentCapabilities { processes: bool, sessions: bool, subagents: bool, mcp: bool, skills: bool, plugins: bool, models: bool, connections: bool, logs: bool, lifecycle_control: bool }` (all pub bool)
  - `pub struct DetectionResult { installed: bool, executable_path: Option<String>, version: Option<String> }`
  - `pub struct AgentProcess { pid: u32, name: String }`, `pub struct AgentSession { id: String, status: String }`
  - Minimal `pub struct SubagentInfo { id: String }`, `McpServerInfo { name: String }`, `SkillInfo { name: String }`, `PluginInfo { name: String }`, `ModelInfo { name: String }`, `ConnectionInfo { name: String }` (rich fields arrive with their engines)
  - `pub trait AgentAdapter: Send + Sync { fn id(&self) -> &'static str; fn name(&self) -> &'static str; fn executable_names(&self) -> &[&str] { &[] } fn detect_installation(&self) -> DetectionResult; fn capabilities(&self) -> AgentCapabilities; fn processes(&self, _snapshot: &[ProcessInfo]) -> Vec<AgentProcess> { vec![] } fn sessions(&self) -> CapabilityResult<Vec<AgentSession>> { Err(Unsupported("sessions")) } fn subagents(&self) -> CapabilityResult<Vec<SubagentInfo>> { Err(Unsupported("subagents")) } fn mcp_servers(&self) -> CapabilityResult<Vec<McpServerInfo>> { Err(Unsupported("mcp")) } fn skills(&self) -> CapabilityResult<Vec<SkillInfo>> { Err(Unsupported("skills")) } fn plugins(&self) -> CapabilityResult<Vec<PluginInfo>> { Err(Unsupported("plugins")) } fn models(&self) -> CapabilityResult<Vec<ModelInfo>> { Err(Unsupported("models")) } fn connections(&self) -> CapabilityResult<Vec<ConnectionInfo>> { Err(Unsupported("connections")) } }`
  - `pub fn find_on_path(name: &str) -> Option<String>` — walk PATH entries + PATHEXT (`.exe` etc. when name has no extension)
  - `pub fn run_version(exe: &str, args: &[&str], timeout_ms: u64) -> Result<String, String>` — spawn, poll `try_wait` every 50ms, kill on expiry; Ok(trimmed stdout, first line)

- [ ] **Step 1: Write failing tests** in `detect.rs` + `traits.rs`:
```rust
#[test] fn test_find_on_path_finds_cargo() { /* Some + ends with cargo.exe/cargo */ }
#[test] fn test_find_on_path_missing_is_none() { /* None for agenthq-no-such-bin-xyz */ }
#[test] fn test_version_command_cargo() { /* run_version(cargo, ["--version"], 5000) Ok + contains digit */ }
#[test] fn test_version_command_times_out() { /* spawn "timeout /t 30"? — Windows-safe sleeper: run_version("powershell", ["-NoProfile","-Command","Start-Sleep -Seconds 30"], 300) → Err within 2s */ }
#[test] fn test_unsupported_defaults_err() { /* struct Fake; impl AgentAdapter minimal; sessions().is_err() */ }
```
- [ ] **Step 2: Run tests, watch them fail**
Run (MSVC-prefixed PATH): `cargo test --lib agents::`
Expected: FAIL — `agents` module does not exist
- [ ] **Step 3: Implement** the three files + `mod agents;`.
- [ ] **Step 4: Run tests, watch them pass**
Run: `cargo test --lib`
Expected: all pass

### Task 2: AgentManager

**Files:**
- Create: `agenthq/src-tauri/src/agents/manager.rs`
- Modify: `agenthq/src-tauri/src/agents/mod.rs` (add `pub mod manager;`)

**Interfaces:**
- Consumes: Task 1 trait + helpers, `ProcessInfo`, `Db` + `AgentRow`
- Produces:
  - `pub struct AgentManager { adapters: Vec<Box<dyn AgentAdapter>> }`, `new()`, `register(&mut self, a: Box<dyn AgentAdapter>)`, `adapter_ids(&self) -> Vec<&'static str>`
  - `pub fn detect_all(&self, snapshot: &[ProcessInfo], db: &Db) -> Result<ManagerReport, String>` — per adapter: `detect_installation()` → upsert AgentRow {installed, version, executable_path, running: <matched?>}; running match = snapshot row whose exe file-stem (case-insensitive) is in `executable_names()` AND exe is Some; insert event `agent.started`/`agent.stopped` only on running transitions vs previous DB row (read before upsert); returns `ManagerReport { detected: Vec<String> /* adapter ids with installed=true */, running: Vec<String> }`

- [ ] **Step 1: Write failing tests** (fake adapters inline):
```rust
#[test] fn test_install_detected_exe_present_version_fails() { /* fake: exe=cargo path via find_on_path, version=Err → installed true, version None, row in DB */ }
#[test] fn test_running_matches_exe_stem_case_insensitive() { /* fake executable_names ["fake-agent"]; snapshot row exe "C:\\X\\FAKE-AGENT.EXE" → running true */ }
#[test] fn test_running_ignores_none_exe() { /* snapshot rows with exe None + name matching → running false */ }
#[test] fn test_running_flips_false_when_process_exits() { /* detect with snapshot [row] → running; detect with [] → not running; exactly one stopped event */ }
#[test] fn test_unsupported_adapter_stores_no_extra_state() { /* fake with all-Err capabilities → detect ok, agents row present, no panic */ }
```
Fake adapter registries: `executable_names` returns `&["fake-agent"]` (leaked static or const).
- [ ] **Step 2: Run tests, watch them fail**
Run: `cargo test --lib agents::manager`
Expected: FAIL — `manager` module does not exist
- [ ] **Step 3: Implement** manager per interfaces (exe stem compare: Path file_stem, to_lowercase).
- [ ] **Step 4: Run tests, watch them pass**
Run: `cargo test --lib`
Expected: all pass

### Task 3: Startup + refresh wiring

**Files:**
- Modify: `agenthq/src-tauri/src/lib.rs` (manage `AgentManager` with zero adapters for now — adapters register in Phases 6–8; setup runs `detect_all` with a fresh snapshot; add `refresh_agents` command that re-runs detection)

**Interfaces:**
- Consumes: Tasks 1–2, existing `Db` + `ProcessMonitor` states
- Produces: `#[tauri::command] fn refresh_agents(monitor: State<Mutex<ProcessMonitor>>, manager: State<Mutex<AgentManager>>, db: State<Db>) -> Result<ManagerReport, String>` (lock order: monitor → manager → db, documented); `ManagerReport` derives `Serialize`

- [ ] **Step 1: Write wiring + command**
Setup: after `manage(db)` + monitor manage, `manage(Mutex::new(AgentManager::new()))`, then run one `detect_all` (fresh `ProcessMonitor` snapshot? reuse managed monitor: lock, refresh, snapshot) — errors logged via `insert_event("error", ...)` but never fail startup.
- [ ] **Step 2: Verify Rust**
Run (MSVC-prefixed PATH): `cargo check`, then `cargo test --lib`
Expected: check exit 0 no new warnings; tests all pass
- [ ] **Step 3: Verify frontend untouched**
Run in `agenthq/`: `npx tsc --noEmit` then `npm run build`
Expected: both exit 0
