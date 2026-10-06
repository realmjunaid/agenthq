# AI Agent Center — Master Development Plan

**Document:** `MASTER_PLAN.md`  
**Version:** 1.0  
**Target:** Windows 10/11 x64  
**Product type:** Lightweight local-first desktop application  
**Primary stack:** Tauri 2 + Rust + React + TypeScript + SQLite

---

# 0. READ THIS FIRST — INSTRUCTIONS FOR THE AI CODING AGENT

You are building **AI Agent Center**, a lightweight Windows desktop application that monitors and inspects locally installed AI coding agents such as Claude Code, OpenCode, and OpenAI Codex CLI.

The application is **NOT** a generic process manager. Its purpose is to become a unified control/observability center for local AI coding agents.

## Non-negotiable rules

1. Use **Tauri 2 + Rust + React + TypeScript**.
2. Do **NOT** use Electron.
3. Do **NOT** create a Node.js/Python background server.
4. Keep OS-level work in Rust.
5. Keep UI/state management in React.
6. Use SQLite for local persistence.
7. Keep the application local-first and privacy-first.
8. Never expose or store API keys, OAuth tokens, passwords, private keys, or secret environment variable values.
9. Never fabricate information. If an agent does not expose something, show `Unknown`, `Unavailable`, or `Not supported`.
10. Use an **adapter architecture** for every AI agent.
11. Do not hard-code Claude/OpenCode/Codex logic into the generic core.
12. Avoid full-disk scans.
13. Prefer event-driven filesystem monitoring and reasonable polling.
14. Optimize for low RAM and CPU usage.
15. Do not add unnecessary dependencies.
16. Do not implement future features before the current phase is stable.
17. Before changing architecture, inspect the existing code and update this plan if necessary.
18. After each phase, run tests/build checks and fix regressions before moving on.
19. Keep the code production-quality, typed, modular, and documented.
20. Never claim an agent capability is supported until it has been verified from its actual local configuration/process behavior.

---

# 1. PRODUCT VISION

Build a modern Windows application that feels like:

> **VS Code + Raycast + Docker Desktop + Process Explorer, specifically for AI coding agents.**

The application should let a developer answer:

- Which AI agents are installed?
- Which ones are running?
- Which projects are they working on?
- How much CPU/RAM are they consuming?
- How many sessions are active?
- How many subagents are running?
- Which MCP servers are configured/connected?
- Which skills are installed?
- Which plugins are installed?
- Which models are configured?
- Which connections/providers are configured?
- What processes belong to each agent?
- What happened recently?
- Can I open/restart/stop the agent?

The product should be **fast, clean, minimal, private, and extensible**.

---

# 2. DESIGN REFERENCE

The provided visual reference image shows the desired design language.

Use the reference as inspiration for:

- Spacious layout
- Clean white/light-gray surfaces
- Thin subtle borders
- Rounded cards
- Compact left sidebar
- Large central workspace
- Optional contextual right panel
- Minimal black/gray controls
- Small status colors
- Clean typography
- Premium developer-tool feeling
- Very little visual noise

Do NOT copy the reference branding or content.

The UI should feel like a purpose-built professional desktop application.

---

# 3. DESIGN SYSTEM

## Light mode

```text
Background: #FAFAFA
Surface: #FFFFFF
Border: #EAEAEA
Primary: #111111
Text: #171717
Muted: #8A8A8A
Success: #16A34A
Warning: #D97706
Error: #DC2626
Info: #2563EB
```

## Dark mode

```text
Background: #0F0F10
Surface: #171719
Border: #29292C
Primary: #F5F5F5
Text: #F5F5F5
Muted: #8B8B8F
Success: #22C55E
Warning: #F59E0B
Error: #EF4444
Info: #60A5FA
```

Do not hard-code these values throughout the application. Use design tokens/CSS variables.

---

# 4. TECHNOLOGY STACK

## Desktop

- Tauri 2
- Rust

## Frontend

- React
- TypeScript
- Vite
- Tailwind CSS
- shadcn/ui or lightweight equivalent
- Lucide icons

## State

Prefer a lightweight store such as Zustand only if needed.

Avoid adding a state library if React state/context is sufficient.

## Database

- SQLite
- Prefer a mature Rust SQLite library.

## Windows

Use native Windows APIs/crates where needed for:

- Process information
- CPU/RAM monitoring
- Windows notifications
- System tray
- Process control
- Credential Manager if secure credential storage is ever required

---

# 5. ARCHITECTURE

```text
                    ┌────────────────────────┐
                    │       React UI         │
                    │                        │
                    │ Dashboard              │
                    │ Agents                 │
                    │ Sessions               │
                    │ Projects               │
                    │ MCP                    │
                    │ Skills                 │
                    │ Plugins                │
                    │ Logs                   │
                    │ Settings               │
                    └───────────┬────────────┘
                                │
                           Tauri IPC
                                │
                    ┌───────────▼────────────┐
                    │       Rust Core        │
                    │                        │
                    │ Agent Manager          │
                    │ Process Monitor        │
                    │ Resource Monitor       │
                    │ Config Scanner         │
                    │ File Watcher           │
                    │ Session Manager        │
                    │ Event Bus              │
                    │ SQLite Repository      │
                    └───────────┬────────────┘
                                │
             ┌──────────────────┼──────────────────┐
             │                  │                  │
             ▼                  ▼                  ▼
      Claude Adapter      OpenCode Adapter    Codex Adapter
             │                  │                  │
             ▼                  ▼                  ▼
        Claude CLI          OpenCode CLI        Codex CLI
```

---

# 6. CORE ARCHITECTURAL RULE — ADAPTERS

The core must not contain agent-specific parsing logic.

Create an adapter abstraction.

Conceptual interface:

```rust
trait AgentAdapter {
    fn id(&self) -> &'static str;
    fn name(&self) -> &'static str;

    fn detect_installation(&self) -> DetectionResult;
    fn get_version(&self) -> Option<String>;

    fn get_processes(&self) -> Vec<AgentProcess>;
    fn get_sessions(&self) -> Vec<AgentSession>;

    fn get_subagents(&self) -> CapabilityResult<Vec<Subagent>>;
    fn get_mcp_servers(&self) -> CapabilityResult<Vec<McpServer>>;
    fn get_skills(&self) -> CapabilityResult<Vec<Skill>>;
    fn get_plugins(&self) -> CapabilityResult<Vec<Plugin>>;
    fn get_models(&self) -> CapabilityResult<Vec<Model>>;
    fn get_connections(&self) -> CapabilityResult<Vec<Connection>>;

    fn capabilities(&self) -> AgentCapabilities;
}
```

The exact Rust API may be adjusted during implementation.

The important requirement is separation of concerns.

---

# 7. CAPABILITY MODEL

Every adapter must explicitly declare capabilities.

Example:

```json
{
  "processes": true,
  "sessions": true,
  "subagents": false,
  "mcp": true,
  "skills": true,
  "plugins": true,
  "models": true,
  "connections": true,
  "logs": false,
  "lifecycle_control": true
}
```

The UI must respond to capabilities.

If a capability is unsupported:

- Do not display fake zeros.
- Show `Not supported` or `Unavailable`.
- Explain when useful.

---

# 8. INFORMATION CONFIDENCE

Every detected piece of information should have a confidence/source concept where appropriate.

Possible values:

```text
Confirmed
Detected
Estimated
Unknown
```

Example:

```text
Subagents: 3
Source: agent-reported
Confidence: Confirmed
```

or:

```text
Subagents: 3
Source: process tree
Confidence: Detected
```

This is important because different agents expose different amounts of runtime information.

---

# 9. PROJECT STRUCTURE

Use a structure similar to:

```text
ai-agent-center/
│
├── src/
│   ├── app/
│   ├── components/
│   ├── layouts/
│   ├── pages/
│   ├── hooks/
│   ├── stores/
│   ├── types/
│   ├── lib/
│   └── styles/
│
├── src-tauri/
│   ├── src/
│   │   ├── main.rs
│   │   ├── agents/
│   │   │   ├── mod.rs
│   │   │   ├── traits.rs
│   │   │   ├── manager.rs
│   │   │   ├── claude/
│   │   │   ├── opencode/
│   │   │   └── codex/
│   │   │
│   │   ├── monitoring/
│   │   │   ├── process.rs
│   │   │   ├── resources.rs
│   │   │   ├── filesystem.rs
│   │   │   └── network.rs
│   │   │
│   │   ├── database/
│   │   │   ├── mod.rs
│   │   │   ├── schema.rs
│   │   │   └── repository.rs
│   │   │
│   │   ├── commands/
│   │   ├── events/
│   │   ├── config/
│   │   └── utils/
│   │
│   └── migrations/
│
├── tests/
├── fixtures/
├── README.md
├── MASTER_PLAN.md
├── package.json
└── Cargo.toml
```

Adjust only when there is a clear technical reason.

---

# 10. PHASE 0 — DISCOVERY BEFORE CODING

Before writing implementation code:

1. Inspect the environment.
2. Inspect installed runtimes/tools.
3. Verify Tauri prerequisites.
4. Verify Rust toolchain.
5. Verify Node/pnpm/npm.
6. Inspect Windows version.
7. Confirm target architecture.
8. Research current local configuration behavior of:
   - Claude Code
   - OpenCode
   - Codex CLI
9. Identify:
   - Executable/process names
   - Config locations
   - MCP configuration locations
   - Skills locations
   - Plugin locations
   - Session information sources
   - Version commands
10. Do not guess paths if they can be detected dynamically.

Create:

```text
docs/agent-research.md
```

Document every verified path/behavior.

Important:

Agent configuration formats can change. Keep agent-specific paths/parsers isolated inside their adapters.

---

# 11. PHASE 1 — BOOTSTRAP

Create the initial Tauri application.

Requirements:

- Tauri 2
- React
- TypeScript
- Tailwind
- Rust
- Windows build

Build a minimal shell with:

```text
Sidebar
Top bar
Main content
```

Do NOT implement agents yet.

### Acceptance criteria

- App starts.
- App builds successfully.
- Windows executable can be generated.
- No unnecessary background service exists.
- UI is responsive.
- Light/dark theme foundation exists.

---

# 12. PHASE 2 — DESIGN SYSTEM

Implement reusable UI primitives:

```text
Button
Card
Badge
StatusDot
Tabs
SegmentedControl
Input
Search
Tooltip
Dropdown
Dialog
EmptyState
ErrorState
StatCard
AgentCard
Metric
```

Do not duplicate styling.

Create layout components:

```text
AppShell
Sidebar
Topbar
PageHeader
ContentPanel
InspectorPanel
```

---

# 13. PHASE 3 — DATABASE

Create SQLite database.

Tables:

```text
agents
agent_processes
sessions
subagents
mcp_servers
skills
plugins
models
connections
projects
events
resource_snapshots
settings
```

Basic agent table:

```sql
CREATE TABLE agents (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    type TEXT NOT NULL,
    version TEXT,
    executable_path TEXT,
    installed INTEGER NOT NULL DEFAULT 0,
    running INTEGER NOT NULL DEFAULT 0,
    last_seen INTEGER,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);
```

Use migrations.

Never make schema changes without a migration.

---

# 14. PHASE 4 — PROCESS MONITOR

Implement Rust process monitoring.

For each relevant process collect:

```text
PID
Parent PID
Process name
Executable path
CPU usage
RAM usage
Start time
Command line where safely available
```

Requirements:

- Do not scan every process repeatedly if unnecessary.
- Cache process metadata.
- Update resource metrics at a reasonable interval.
- Group child processes under the appropriate agent.

Default resource update interval:

```text
2 seconds
```

Allow future configuration.

---

# 15. PHASE 5 — AGENT DETECTION ENGINE

Create:

```text
AgentManager
```

Responsibilities:

1. Register adapters.
2. Run installation detection.
3. Detect running agents.
4. Update agent state.
5. Trigger events.
6. Persist important state.

Example:

```text
AgentManager
 ├── ClaudeAdapter
 ├── OpenCodeAdapter
 └── CodexAdapter
```

Detection sources:

1. Running process
2. PATH
3. Known executable
4. Known config directory
5. CLI version command

Do not rely on only one detection mechanism.

---

# 16. PHASE 6 — CLAUDE ADAPTER

Implement the Claude adapter.

Capabilities should be verified before being marked supported.

Implement:

```text
Installation detection
Version
Process detection
Process tree association
Configuration discovery
MCP discovery
Skills discovery
Plugins discovery where locally detectable
Session discovery where reliably available
Model information where reliably available
Connection/provider information where safely detectable
```

Create fixtures:

```text
fixtures/claude/
```

Tests must use fixtures where possible.

Do not require a live API key for tests.

---

# 17. PHASE 7 — OPENCODE ADAPTER

Implement OpenCode adapter.

Investigate and support verified:

```text
Installation
Version
Processes
Sessions
Projects
MCP
Skills
Plugins
Models
Configuration
```

Support both user/global and project-level configuration where applicable.

Separate:

```text
Global
Project
```

in the data model/UI.

---

# 18. PHASE 8 — CODEX ADAPTER

Implement Codex adapter.

Implement verified capabilities such as:

```text
Installation
Version
Processes
MCP
Configuration
Sessions where reliably detectable
Models where reliably detectable
Connections where safely detectable
```

Do not invent session/subagent information.

---

# 19. PHASE 9 — MCP ENGINE

Create a normalized MCP model:

```text
McpServer
 ├── id
 ├── name
 ├── type
 ├── command
 ├── args
 ├── url
 ├── source
 ├── status
 ├── agent_id
 └── project_id
```

Status:

```text
Connected
Configured
Offline
Error
Unknown
```

Never expose secret environment variable values.

Instead:

```text
Environment variables: 3 configured
```

---

# 20. PHASE 10 — SKILLS ENGINE

Normalized model:

```text
Skill
 ├── id
 ├── name
 ├── description
 ├── path
 ├── scope
 ├── source
 └── agent_id
```

Scopes:

```text
Global
Project
```

Watch known directories instead of repeatedly scanning the entire disk.

---

# 21. PHASE 11 — PLUGINS ENGINE

Normalized model:

```text
Plugin
 ├── id
 ├── name
 ├── version
 ├── path
 ├── source
 ├── enabled
 └── agent_id
```

Only mark a plugin as installed when evidence exists.

---

# 22. PHASE 12 — SESSION & PROJECT ENGINE

Normalized project:

```text
Project
 ├── id
 ├── name
 ├── path
 ├── last_seen
 └── agent_ids
```

Session:

```text
Session
 ├── id
 ├── agent_id
 ├── project_id
 ├── status
 ├── model
 ├── started_at
 ├── last_activity
 └── confidence
```

Use:

```text
Unknown
```

when exact data cannot be obtained.

---

# 23. PHASE 13 — EVENT BUS

Create a local event system.

Events:

```text
agent.installed
agent.started
agent.stopped
agent.error

session.started
session.stopped

subagent.started
subagent.stopped

mcp.connected
mcp.disconnected

skill.changed
plugin.changed
config.changed
```

Events should update the UI incrementally.

Do not reload the entire application state for every event.

---

# 24. PHASE 14 — DASHBOARD UI

Build the main dashboard based on the provided design reference.

Layout:

```text
Sidebar
    ↓
Dashboard workspace
    ↓
Overview statistics
    ↓
Agent cards
    ↓
Recent activity
    ↓
Optional inspector panel
```

Top stats:

```text
Agents
Running
MCP
RAM
```

Agent cards should show:

```text
Agent icon
Agent name
Vendor
Status
RAM
CPU
Sessions
Subagents
MCP
Skills
```

Example:

```text
Claude Code
● Running

412 MB
4.2% CPU
2 Sessions
3 Subagents
4 MCP
7 Skills
```

---

# 25. PHASE 15 — AGENT DETAIL UI

When an agent is selected:

Tabs:

```text
Overview
Sessions
Processes
MCP
Skills
Plugins
Models
Logs
Configuration
```

Right-side inspector should show:

```text
Status
Version
CPU
RAM
Processes
Sessions
Subagents
MCP
Skills
```

Actions:

```text
Open Terminal
Open Project
Restart
Stop
```

Only show actions supported by the adapter.

---

# 26. PHASE 16 — MONITORING PAGE

Create a system monitoring page.

Show:

```text
CPU
RAM
Disk
Network
```

Agent resource table:

```text
Agent       CPU       RAM       Processes
Claude      4.2%      412 MB    7
OpenCode    1.8%      287 MB    5
Codex       0.7%      190 MB    3
```

Keep charts lightweight.

Do not render expensive animated charts continuously.

---

# 27. PHASE 17 — LOGS

Create local event/log viewer.

Columns:

```text
Time
Level
Agent
Event
Message
```

Filters:

```text
All
Info
Warning
Error
```

Search:

```text
Ctrl + K
```

Never store secrets in logs.

---

# 28. PHASE 18 — SYSTEM TRAY

Implement Windows system tray.

Menu:

```text
AI Agent Center

Claude Code       ● Running
OpenCode          ● Running
Codex             ○ Offline

---------------------

Open Dashboard
Pause Monitoring
Settings
Exit
```

Allow:

```text
Start with Windows
Launch minimized
Close to tray
```

---

# 29. PHASE 19 — NOTIFICATIONS

Local Windows notifications.

Supported events:

```text
Agent started
Agent stopped
Agent error
MCP disconnected
High CPU
High RAM
```

All notification types must be configurable.

---

# 30. PHASE 20 — LIFECYCLE CONTROL

Only after monitoring is stable.

Implement:

```text
Start
Stop
Restart
Open terminal
Open project directory
```

Safety:

- Confirm destructive operations.
- Never kill unrelated processes.
- Use process ownership/association carefully.
- Never assume a child process belongs to an agent only because its name matches.

---

# 31. PHASE 21 — SECURITY AUDIT

Before release, audit:

- API key exposure
- Environment variable exposure
- Log leakage
- SQLite contents
- Crash reports
- Debug logs
- IPC permissions
- Tauri capabilities
- File-system permissions

Principle:

> The application should require the minimum permissions necessary.

---

# 32. PHASE 22 — PERFORMANCE OPTIMIZATION

Measure:

```text
Startup time
Idle RAM
Idle CPU
Dashboard rendering
IPC frequency
SQLite query count
Filesystem watcher count
Process polling cost
```

Targets:

```text
Startup: preferably < 2 seconds
Idle CPU: preferably < 1–2%
Idle RAM: target < 150 MB
```

These are targets, not excuses to sacrifice correctness.

Optimize only after profiling.

---

# 33. PERFORMANCE RULES

DO:

- Cache parsed config
- Watch files
- Poll processes reasonably
- Batch DB writes
- Use incremental UI updates
- Dispose listeners correctly
- Lazy-load secondary pages

DO NOT:

- Scan the entire disk repeatedly
- Poll every 100ms
- Re-render every card every 100ms
- Store every resource sample forever
- Run unnecessary background workers
- Add a web server

---

# 34. TESTING STRATEGY

## Unit tests

Test:

```text
Agent detection
Config parsing
MCP parsing
Skill parsing
Plugin parsing
Process grouping
Status calculation
```

## Adapter fixture tests

Each adapter must have fixtures:

```text
fixtures/claude/
fixtures/opencode/
fixtures/codex/
```

## Integration tests

Test:

```text
Agent discovery
Database persistence
Event propagation
Process association
```

## UI tests

Test:

```text
Dashboard
Agent details
Loading states
Empty states
Error states
Unsupported capability states
Theme
Navigation
```

---

# 35. REQUIRED UI STATES

Every page must handle:

```text
Loading
Loaded
Empty
Error
Unavailable
Not supported
```

Example:

Instead of blank UI:

```text
MCP

No MCP servers detected.
```

Instead of fake zero:

```text
Subagents

Not available for this agent.
```

---

# 36. ACCESSIBILITY

Support:

- Keyboard navigation
- Visible focus
- Tooltips
- Proper labels
- Screen-reader-friendly controls
- Reduced motion
- Sufficient contrast

---

# 37. DOCUMENTATION

Maintain:

```text
README.md
MASTER_PLAN.md
docs/architecture.md
docs/agent-research.md
docs/development.md
docs/security.md
```

README must include:

- What the app does
- Screenshots
- Installation
- Development setup
- Build commands
- Supported agents
- Privacy statement

---

# 38. DEVELOPMENT COMMANDS

The final project should have simple commands similar to:

```bash
npm install
npm run dev
npm run build
npm run tauri dev
npm run tauri build
```

Exact package manager may be chosen during bootstrap.

If using pnpm, document it consistently.

---

# 39. GIT WORKFLOW

Use logical commits.

Recommended sequence:

```text
chore: initialize tauri app
feat: add design system
feat: add sqlite database
feat: add process monitor
feat: add agent manager
feat: add claude adapter
feat: add opencode adapter
feat: add codex adapter
feat: add dashboard
feat: add agent details
feat: add system tray
feat: add notifications
perf: optimize monitoring
fix: ...
```

Do not create giant commits containing unrelated work.

---

# 40. PHASE COMPLETION RULE

After every phase:

1. Run formatter.
2. Run type check.
3. Run Rust tests.
4. Run frontend tests.
5. Build the application.
6. Manually verify the changed feature.
7. Check for console errors.
8. Check for Rust warnings.
9. Update documentation.
10. Only then proceed.

---

# 41. AGENT-SPECIFIC RESEARCH RULE

AI coding agents evolve quickly.

Before implementing an adapter:

1. Inspect the currently installed version.
2. Inspect its actual configuration.
3. Inspect its CLI help/version output if useful.
4. Inspect official documentation when available.
5. Confirm local paths.
6. Create parser tests from real examples.
7. Keep version-specific behavior isolated.

Never assume an old blog post or memory is still correct.

---

# 42. MVP ORDER

Implement in EXACTLY this order unless a technical dependency requires otherwise:

```text
1. Project bootstrap
2. Design system
3. App shell
4. SQLite
5. Rust process monitor
6. Agent manager
7. Claude adapter
8. OpenCode adapter
9. Codex adapter
10. MCP engine
11. Skills engine
12. Plugins engine
13. Sessions/projects
14. Dashboard
15. Agent detail page
16. Logs
17. System tray
18. Notifications
19. Lifecycle controls
20. Security audit
21. Performance optimization
22. Packaging
23. Final QA
```

Do NOT jump to marketplace/custom adapters before the MVP is stable.

---

# 43. MVP DEFINITION OF DONE

MVP is complete when a fresh Windows installation can:

### Detect

- Claude Code
- OpenCode
- Codex CLI

### Show

- Installed/offline/running
- Version
- PID
- CPU
- RAM
- Process tree
- Projects where detectable
- Sessions where detectable
- MCP where detectable
- Skills where detectable
- Plugins where detectable
- Models where detectable
- Connections where safely detectable

### UI

- Modern dashboard
- Sidebar
- Agent cards
- Agent details
- Tabs
- Search
- Light/dark mode
- Tray

### Safety

- No secret leakage
- No cloud telemetry
- No fake data
- Graceful unsupported states

### Performance

- Fast startup
- Low idle CPU
- Low idle RAM
- No continuous full-disk scans

---

# 44. FUTURE ROADMAP — DO NOT IMPLEMENT NOW

Potential future versions:

## V2

```text
Gemini CLI
Aider
Cline
Roo Code
Kilo Code
Custom adapters
```

## V3

```text
Token usage
Cost tracking
Session analytics
Resource history
Agent comparison
```

## V4

```text
MCP manager
Skill manager
Plugin manager
Agent orchestration
Automatic recovery
```

## V5

```text
Community adapter system
Adapter marketplace
Agent automation
Advanced project-agent workflows
```

Future features must not compromise the lightweight/local-first architecture.

---

# 45. FINAL AI AGENT INSTRUCTION

When working on this project, behave as a senior desktop application engineer.

Before coding:

- Understand the current phase.
- Inspect existing code.
- Check the relevant documentation.
- Identify dependencies.
- Do not blindly overwrite existing implementation.

While coding:

- Keep modules small.
- Prefer simple solutions.
- Avoid unnecessary abstractions.
- Keep agent-specific code inside adapters.
- Keep OS logic in Rust.
- Keep UI logic in React.
- Handle errors explicitly.
- Never expose secrets.

After coding:

- Format.
- Type-check.
- Test.
- Build.
- Verify manually.
- Update documentation.

If something cannot be detected reliably:

> Do not guess.

Represent it explicitly as:

```text
Unknown
Unavailable
Not supported
```

The quality standard is:

> **Correct information is more important than showing more information.**

The final application should feel polished, modern, fast, lightweight, private, and trustworthy.

---

# 46. FIRST TASK

When this plan is given to an AI coding agent, DO NOT immediately implement the entire application.

Start with:

## Step 1

Inspect the environment and repository.

## Step 2

Create:

```text
docs/architecture.md
docs/agent-research.md
docs/development.md
```

## Step 3

Verify:

- Rust
- Cargo
- Node
- npm/pnpm
- Tauri
- Windows target

## Step 4

Create the Tauri + React project shell.

## Step 5

Implement the basic UI:

```text
Sidebar
Topbar
Dashboard placeholder
Agents page placeholder
Settings placeholder
```

## Step 6

Run the application.

## Step 7

Verify the Windows build.

## Step 8

Only after the shell is stable, begin Phase 2.

Do not implement agents, MCP, process monitoring, or lifecycle control during the first bootstrap task.

---

# 47. SUCCESS CRITERIA FOR THE FIRST CODING SESSION

At the end of the first coding session:

```text
✓ Project created
✓ Tauri runs
✓ React runs
✓ Rust backend runs
✓ Windows build works
✓ Sidebar exists
✓ Dashboard exists
✓ Theme exists
✓ No major warnings/errors
✓ Documentation exists
```

The first session is successful even if no AI agent detection exists yet.

Build incrementally.

---

# END OF MASTER PLAN
