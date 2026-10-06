# Phase 15 (Agent Detail UI) Implementation Plan

**Goal:** Selecting an agent opens a tabbed detail view plus a right inspector, backed only by data the IPC already returns.

**Architecture:** `AgentDetailPage` reads `get_dashboard` for the row and the existing list commands (`get_sessions`, `get_mcp_servers`, `get_skills`, `get_plugins`, `get_process_snapshot`, `get_events`, `get_projects`) filtered by agent id. Models and Configuration have no read command yet → `UnsupportedState`. Lifecycle actions (terminal/restart/stop) have no command → hidden, not fake buttons.

**Spec:** `MASTER_PLAN.md` §25. Actions only when supported.

## Tasks

- [ ] Navigation: dashboard/agents cards set `agent` page + id; sidebar highlights Agents.
- [ ] Detail page: Overview / Sessions / Processes / MCP / Skills / Plugins / Models / Logs / Configuration.
- [ ] Inspector: status, version, CPU, RAM, processes, sessions, subagents (Not available), MCP, skills.
- [ ] Open Project only when a project path exists (`plugin-opener`).
- [ ] Verify: `npx tsc --noEmit`, `npm run build`.
