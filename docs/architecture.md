# AgentHQ — Architecture

Spec: `MASTER_PLAN.md` §5–§9. Status: Phase 0–1 (bootstrap). No code yet.

## Stack

Tauri 2 (Rust backend) + React + TypeScript + Vite (frontend), SQLite via Rust
(mature crate TBD at Phase 3 — `rusqlite`), Tailwind for styling. Windows 10/11 x64.

## Data flow

```text
React UI --Tauri IPC--> Rust Core (AgentManager, ProcessMonitor,
ResourceMonitor, ConfigScanner, FileWatcher, EventBus, SQLite repo)
  --> per-agent Adapters (Claude / OpenCode / Codex) --> local CLIs
```

## Rules (from MASTER_PLAN §0, binding)

- No Electron. No Node/Python background server. OS work in Rust, UI in React.
- Every agent behind an adapter trait; zero agent-specific logic in core.
- Capability-gated UI: unsupported → `Not supported`/`Unknown`, never fake zeros.
- No secrets in state/logs/DB. Local-first, no cloud telemetry.
- Event-driven watching + ~2s resource polling; no full-disk scans.

## Module map (target layout)

```text
agenthq/
  src/                    React: app/ components/ layouts/ pages/ hooks/ stores/ types/ lib/ styles/
  src-tauri/
    src/
      main.rs
      agents/   traits.rs manager.rs claude/ opencode/ codex/
      monitoring/ process.rs resources.rs filesystem.rs network.rs
      database/ mod.rs schema.rs repository.rs
      commands/ events/ config/ utils/
    migrations/
```

## Phase order (MASTER_PLAN §42)

bootstrap → design system → shell → SQLite → process monitor → agent manager →
Claude → OpenCode → Codex adapters → MCP → skills → plugins → sessions/projects →
dashboard → detail page → logs → tray → notifications → lifecycle → audit → perf.
