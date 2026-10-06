# AgentHQ — Project Progress

**App:** AgentHQ (local-first Windows desktop: AI coding-agent control center)
**Stack:** Tauri 2 + Rust + React + TS + SQLite | **Spec:** `MASTER_PLAN.md`
**Plan:** `docs/superpowers/plans/2026-10-05-phase0-phase1-bootstrap.md`
**Code dir:** `agenthq/` (scaffold pending) | **Git:** not yet (user deferred)

## Environment (verified 2026-10-05)

| Item | Status |
|---|---|
| Windows 11 Pro 26200 x64 | OK |
| Node v24.19.0 + npm v11.17.0 | OK |
| Rust 1.99.0 + Cargo (MSVC target) | OK (`~/.cargo/bin`, fresh shells) |
| VS Build Tools 2022 (C++ linker) | OK (MSVC 14.44, explicit PATH in build shells) |
| WebView2 (via Edge 154) | OK |
| Claude Code 2.1.289 / OpenCode 2.0.23 | OK |
| Codex CLI | MISSING (research docs-only) |
| pnpm | MISSING (using npm) |

## Phase 0–1 Tasks

- [x] Task 1 — Rust toolchain (rustc/cargo 1.99.0, msvc target, PATH)
- [x] Task 2 — VS Build Tools (MSVC 14.44 present; linker via explicit PATH)
- [x] Task 3 — Docs (`docs/architecture.md`, `docs/agent-research.md`, `docs/development.md`)
- [x] Task 4 — Scaffold Tauri 2 + React-TS into `agenthq/` (`cargo check` PASS)
- [x] Task 5 — AppShell UI (Sidebar/Topbar/theme toggle/placeholders/tokens + layout CSS)
- [x] Task 6 — `tauri dev` + Windows build verify (MSI 2.03 MiB + setup exe 1.38 MiB)
- [x] Final review + fix pass (shell layout CSS, Tailwind v4 wired, App.css deleted)

## Phase 2 (Design System)

- [x] Atoms: Button, Badge, StatusDot, Input/Search, Metric
- [x] Cards: Card, StatCard, AgentCard
- [x] Controls: Tabs, SegmentedControl, Dropdown, Tooltip, Dialog
- [x] States + panels: Empty/Error/Unsupported, PageHeader, ContentPanel, InspectorPanel
- [x] Shell refactor onto layouts/ + token audit clean
- [x] Final review + fix pass (Search handler, card ellipsis) — 0 Critical

## Phase 3 (SQLite)

- [x] rusqlite + open/migrate + 001 agents (verbatim DDL)
- [x] Migrations 002–004 (all 13 tables, FKs, atomic apply)
- [x] Agent/event repository (upsert/get/list/ordered events) — 12 lib tests green
- [x] Startup wiring (app_data/agenthq.db) + `list_agents` IPC + TS mirror
- [x] Final review + fix pass (FK pin test, atomic migrations) — 0 Critical

## Phase 4 (Process Monitor)

- [x] Types + pure tree grouping (children_of/parent_map)
- [x] Live snapshot (sysinfo system-only) + totals + 2s tick const
- [x] IPC (`get_process_snapshot` capped 500, stateful `get_system_stats`) + TS mirrors — 21 lib tests green
- [x] Final review + fix pass (stateful system CPU) — 0 Critical

## Phase 5 (Agent Detection Engine)

- [x] Adapter trait + DTOs + PATH/version probes (PATHEXT, 5s-timeout kill)
- [x] AgentManager (register/detect/running-match/persist/transitions) — 31 lib tests green
- [x] Startup wiring + `refresh_agents` IPC
- [x] Final review — 0 Critical, 0 Important

## Phase 6 (Claude Adapter)

- [x] Paths/install/version/capabilities/processes + registration
- [x] Session discovery (capped scan, status Unknown) + fixtures
- [x] MCP/skills/plugins/models/connections (names only, no secrets) — 48 lib tests green
- [x] Research doc + full verification + final review — 0 Critical, 0 Important

## Phase 7 (OpenCode Adapter)

- [x] Paths/version/CLI runner (PATHEXT, timeouts, 500-line cap) + registration
- [x] Sessions via CLI (tolerant parse, project-scoped, empty-safe) + fixtures
- [x] MCP/skills/plugins/models (names only, no secrets) — 63 lib tests green
- [x] Research doc + full verification + final review — 0 Critical, 0 Important

## Phase 8 (Codex Adapter)

- [x] Releases-glob paths/version/caps/processes + registration
- [x] Sessions via session_index.jsonl + fixtures
- [x] MCP header-scanner/skills/plugins/models_cache slugs (no secrets) — 76 lib tests green
- [x] Research doc + full verification + final review — 0 Critical, 0 Important

## Phase 9 (MCP Engine)

- [x] McpServerDetail trait + Claude full-entry parsing (transport/env-count/caps)
- [x] OpenCode shared mapping + Codex Unknown rows
- [x] Normalize/status/persist (replace txn) + refresh/get IPC + TS mirror — 90 lib tests green
- [x] Final review + fix pass (project-scoped ids) — 0 Critical

## Phase 10 (Skills Engine)

- [x] SkillScope/SkillDetail trait + shared dir enumeration
- [x] Claude/OpenCode/Codex Global details via shared helper
- [x] Normalize/scoped-ids/persist/refresh/get IPC + TS mirror — 99 lib tests green
- [x] Final review — 0 Critical, 0 Important

## Phase 11 (Plugins Engine)

- [x] PluginDetail trait + CLI row parsing (versions, short-row None)
- [x] Claude market/kind walk, OpenCode CLI versions, Codex dir evidence
- [x] Normalize/persist/refresh/get IPC + TS mirror — 109 lib tests green
- [x] Final review — 0 Critical, 0 Important

## Phase 12 (Session & Project Engine)

- [x] SessionConfidence/SessionDetail/ProjectDetail trait + Claude slugify/resolve
- [x] OpenCode ms→s + Codex index details
- [x] Normalize/link/persist (upsert+replace, derived agents) + refresh/get IPC + TS mirrors — 124 lib tests green
- [x] Final review + fix pass (single project replace) — 0 Critical

## Grok Adapter (2026-10-05)

- [x] Paths/version/caps/processes + registration + watcher root
- [x] Sessions/projects from files (RFC3339 + percent-decode, hand-rolled)
- [x] MCP --json/skills/plugins minus *.lock/models_cache — 174 lib tests green
- [x] Research doc + full gate + final review (0 Critical, 2 Minor fixed with RED→GREEN) — MSI 3.27 MiB + setup 2.34 MiB

## Remaining Gaps (2026-10-05)

- [x] Vitest + `npm test` (format edges, treeDepths orphan/cycle) — 14/14 green
- [x] File watcher (notify 8, known config dirs, 2s debounce, config.changed) — 152 lib tests green
- [x] Measured perf (startup 802ms, idle CPU 0%, WS 31.3MB — all within targets, no tuning)
- [x] Tight capability (listen/unlisten + opener; runtime-checked on release build)
- [x] Subagents verdict: stays Not available (no source; no fabrication)
- [x] Final rebuild → MSI 3.25 MiB + setup 2.32 MiB

## MVP Closeout (2026-10-05)

- [x] Models + connections persistence (provider-split/env, replace txn) + refresh/get IPC + TS mirrors — 147 lib tests green
- [x] Detail UI: Models tab, Connections (Claude-only), indented process tree, Configuration from stored fields, Projects page + nav
- [x] Docs: root README, docs/security.md, development.md gates note
- [x] Packaging: tauri build → MSI 3.22 MiB + NSIS setup 2.31 MiB
- [x] Final review — 0 Critical, 0 Important

## Phases 16–22

- [x] Monitoring page (CPU/RAM/disk/network bars, agent table, 2s tick while open)
- [x] Logs page (level filters, search, Ctrl+K)
- [x] System tray (status, open, pause, settings, exit, close-to-tray, start-with-Windows)
- [x] Notifications (per-type settings, rising-edge high CPU/RAM)
- [x] Lifecycle (start/stop/restart/terminal; confirm; exe-stem only)
- [x] Security audit note (`docs/security-audit.md`) + CSP
- [x] Performance note (`docs/performance.md`) — targets not claimed without a profile
- [x] 141 lib tests green; tsc 0; vite build pass

## Phase 15 (Agent Detail UI)

- [x] Card → detail navigation
- [x] Tabs + inspector from existing IPC (models/config unsupported; lifecycle actions hidden — no command)
- [x] Open Project via plugin-opener when a path exists

## Phase 14 (Dashboard UI)

- [x] get_dashboard aggregate + get_events (stem sums, system totals from ProcessMonitor)
- [x] Live DashboardPage (stats, cards, activity, event refresh) + AgentsPage list
- [x] Final review + fix pass (hide uninstalled, stale-load guard, stopped sessions) — 0 Critical

## Phase 13 (Event Bus)

- [x] diff_ids/ChangeSet/FRONTEND_EVENT + agent.installed once
- [x] Message builders + 5 refresh commands diff/insert/emit (rowids, no dupes)
- [x] TS event contract + full verification — 130 lib tests green
- [x] Final review + fix pass (exactly-once session.stopped) — 0 Critical

## Implemented so far

- Toolchain: Rust stable via rustup; MSVC 14.44; Defender exclusions for project + .cargo.
- Scaffold: `agenthq/` (Tauri 2.12.1 + React 19 + TS + Tailwind v4), `npm install` clean.
- UI shell: AppShell + 3 placeholder pages, light/dark tokens + minimal layout.
- Builds: `cargo check` PASS, `tsc` PASS, `vite build` PASS, `tauri build` → MSI + NSIS.
- Docs: architecture/agent-research/development + plan + ledger. Gate §47 PASS.

## Decisions

- Scaffold in subfolder `agenthq/`, product name `AgentHQ`.
- npm (no pnpm on machine). Git live at https://github.com/realmjunaid/agenthq (main, public, MIT).
- Native (inline) execution, quality-first; final review before done.

## Skills & Plugins (global, all harnesses)

Plugin: superpowers (OpenCode). Skills (31): frontend-design, build-awwwards-quality-sites,
accessibility, shadcn, tailwind-design-system, code-review, improve-codebase-architecture,
tauri-v2, vercel-react-best-practices, vitest, rust-patterns, agent-browser, find-skills,
caveman×14 (caveman, megacave, cavecrew, +commit/review/compress/explore/learn/setup/manage/
optimize/discover/evidence-review/help/stats), investigate-first, lean-build, migration.
