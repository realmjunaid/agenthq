# AgentHQ

Local-first Windows desktop app: a control/observability center for locally
installed AI coding agents (Claude Code, OpenCode, Codex CLI).

No cloud, no telemetry, no background server. OS work lives in Rust, UI in
React, persistence in local SQLite.

## What it does

- Detects installed/running agents (PATH, known locations, config dirs, version commands)
- Shows per-agent CPU/RAM, sessions, projects, MCP servers, skills, plugins, models, connections
- Process tree per agent, system monitoring page, local event log
- System tray with pause, Windows notifications (per-type settings)
- Start / stop / restart / open terminal with a confirm step; only
  exe-stem-matched processes are ever touched

## Screenshots

No screenshots checked in yet. Run `npm run tauri dev` in `agenthq/` to see it live.

## Installation

Download the MSI or NSIS setup from the release bundle
(`agenthq/src-tauri/target/release/bundle/` after `npm run tauri build`).

## Development setup

Windows 10/11 x64. Details in `docs/development.md`.

```bash
cd agenthq
npm install
npm run dev            # Vite frontend only
npm run tauri dev      # full desktop app
npm run tauri build    # release bundle
```

Rust builds need the MSVC linker on PATH (VS Build Tools 2022):

```powershell
$env:Path = "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64;" + $env:Path
```

## Build commands

```bash
npx tsc --noEmit                 # frontend typecheck (in agenthq/)
npm run build                    # frontend build (in agenthq/)
cargo test --lib                 # Rust tests (in agenthq/src-tauri/)
cargo check                      # Rust check (in agenthq/src-tauri/)
```

## Supported agents

| Agent      | Install detect | Version | Processes | Sessions | MCP | Skills | Plugins | Models | Connections |
|------------|----------------|---------|-----------|----------|-----|--------|---------|--------|-------------|
| Claude Code| yes            | yes     | yes       | yes      | yes | yes    | yes     | yes    | env-key names only |
| OpenCode   | yes            | yes     | yes       | yes*     | yes | yes    | yes     | yes    | Not available |
| Codex CLI  | yes            | yes     | yes       | yes*     | yes | yes    | yes     | yes    | Not available |

`*` OpenCode sessions need a project directory; Codex sessions come from its session index. Status is `Unknown` unless the agent proves otherwise. Subagents are `Not available` everywhere — no adapter can count them reliably yet.

## Privacy statement

Everything stays on this machine: SQLite at the app-data directory, no
accounts, no analytics, no crash uploader. MCP `env` blocks and settings
values are never read — only key names and counts. Secrets never enter the
database, logs, tray text, or notifications.
