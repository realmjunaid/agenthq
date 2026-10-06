# AgentHQ

**A local-first control center for your AI coding agents on Windows.**

AgentHQ watches the agents installed on your PC — Claude Code, OpenCode, Codex CLI, Grok CLI — and shows them in one calm dashboard: who's installed, who's running, what they're working on, how much CPU/RAM they use, which sessions, projects, MCP servers, skills, plugins and models each one has.

No cloud. No accounts. No telemetry. Everything stays on your machine.

- 🖥️ **Platform:** Windows 10/11 x64
- 🧱 **Stack:** Tauri 2 (Rust) + React + TypeScript + SQLite
- 🔒 **License:** MIT — free for personal and commercial use
- 🕊️ **Privacy:** local-only by design (see [Privacy](#-privacy) below)

---

## ✨ Features

| Area | What you get |
|---|---|
| **Dashboard** | Agent cards with live CPU/RAM, session counts, MCP/skill totals, recent activity feed |
| **Agents** | Installed / offline / running status, version, per-agent detail pages |
| **Agent detail** | Tabs for Overview, Sessions, Processes (indented tree), MCP, Skills, Plugins, Models, Logs, Configuration + inspector panel |
| **Monitoring** | System CPU/RAM/disk/network bars + per-agent resource table (2 s tick, page open only) |
| **Logs** | Local event viewer with level filters and search (`Ctrl+K`) |
| **Projects** | Directories agents work in, with one-click open |
| **System tray** | Agent status at a glance, Open Dashboard, Pause Monitoring, Settings, Exit; close-to-tray, start-with-Windows, launch-minimized |
| **Notifications** | Agent started/stopped/error, MCP removed, high CPU/RAM — every type toggleable, resource alerts fire once on the rising edge |
| **Lifecycle** | Start / Stop / Restart / Open Terminal with a confirm step — only processes whose executable matches the agent are ever touched |
| **File watcher** | Config edits are picked up automatically (known agent dirs only, debounced) |
| **Dark mode** | Light + dark themes, keyboard navigation, reduced-motion respected |

### Supported agents

| Agent | Install | Version | Processes | Sessions | MCP | Skills | Plugins | Models | Connections |
|---|---|---|---|---|---|---|---|---|---|
| Claude Code | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | env key names only |
| OpenCode | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | — |
| Codex CLI | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | — |
| Grok CLI | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | — |

Missing agents simply don't appear — no placeholders, no fake zeros. Anything an agent doesn't expose shows as `Unknown` or `Not available`. Subagents are `Not available` everywhere: no adapter today exposes a countable source, and AgentHQ will not guess.

---

## 📥 Install (Windows)

**Option A — installer (recommended)**

1. Go to [**Releases**](https://github.com/realmjunaid/agenthq/releases).
2. Download `agenthq_0.1.0_x64-setup.exe` (or the `.msi`) and run it.
3. Launch **AgentHQ** from the Start menu. Detection runs automatically — install Claude Code, OpenCode, Codex CLI or Grok CLI and their cards appear after a Refresh.

> No release published yet? Use Option B until the first release is cut.

**Option B — build from source** (see [Development](#-development)): `npm run tauri build`, then install from `agenthq/src-tauri/target/release/bundle/`.

**Requirements:** Windows 10/11 x64 + WebView2 runtime (preinstalled on Windows 11; Windows 10 gets it via Windows Update or the [evergreen installer](https://developer.microsoft.com/en-us/microsoft-edge/webview2/)).

---

## 🛠️ Development

### Prerequisites (Windows)

1. **Rust** (stable, MSVC): `winget install --id Rustlang.Rustup -e`, target `x86_64-pc-windows-msvc`.
2. **VS Build Tools 2022** with the C++ workload — the Rust linker must be on `PATH` in every build shell:
   ```powershell
   $env:Path = "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64;" + $env:Path
   ```
   (Version folder `14.44.*` may differ on your machine — use whatever is under `...\VC\Tools\MSVC\`.)
3. **Node 24 + npm** (npm only; no pnpm files in this repo).
4. **WebView2** (see above).

### Commands

```bash
cd agenthq
npm install          # frontend deps

npm run dev          # Vite frontend only
npm run tauri dev    # full desktop app (debug)

npm run build        # frontend production build
npm test             # frontend tests (vitest)
npx tsc --noEmit     # frontend typecheck
```

```bash
cd agenthq/src-tauri
cargo test --lib     # Rust tests (linker PATH from step 2 first)
cargo check          # Rust check — must finish with zero warnings
cargo fmt            # formatter
```

```bash
cd agenthq
npm run tauri build  # release MSI + NSIS setup → src-tauri/target/release/bundle/
```

> Note: `cargo test --bin agenthq` may be blocked by Windows application-control policy on some machines — the gate is `cargo test --lib` (all tests live in the lib target).

### Project layout

```text
agenthq/
├── src/                    # React frontend
│   ├── app/theme.css       # design tokens + component CSS
│   ├── components/ui/      # hand-rolled primitives (Button, Card, Tabs, …)
│   ├── layouts/            # AppShell, Sidebar, Topbar, panels
│   ├── pages/              # Dashboard, Agents, AgentDetail, Monitoring, Logs, Projects, Settings
│   ├── hooks/ lib/ types/  # IPC wrappers, formatting, TS mirrors of Rust types
├── src-tauri/
│   ├── src/
│   │   ├── agents/         # trait + manager + one adapter per agent (claude/opencode/codex/grok)
│   │   ├── monitoring/     # process monitor + disk/network sampler
│   │   ├── database/       # SQLite + migrations
│   │   ├── *.rs            # mcp/skills/plugins/sessions/models engines, events, lifecycle, tray, notify, watch
│   ├── migrations/         # numbered SQL, applied via PRAGMA user_version
│   ├── fixtures/{claude,opencode,codex,grok}/  # hand-written test shapes (no real user data)
│   └── capabilities/       # minimal Tauri ACL: event listen + file opener only
├── docs/                   # architecture, agent-research, development, security, performance
└── MASTER_PLAN.md / PROGRESS.md  # the build spec and its progress log (repo root)
```

### How detection works

Each launch (and each Refresh) the Rust core:

1. Finds executables via `PATH` + known install dirs, reads version CLIs with a timeout, checks config dirs.
2. Matches running processes by executable stem (a missing exe path never matches; its own PID is always excluded from lifecycle actions).
3. Parses local configs/transcripts into SQLite — MCP `env` blocks and settings values contribute **key names and counts only**.
4. Emits transition events (`agent.started`, `session.stopped`, …) on real changes only — identical refreshes stay silent.

Adapter code lives in `src-tauri/src/agents/<agent>/`; the generic core never hard-codes agent paths. New agents only need a new adapter folder + registration.

---

## 🔒 Privacy

- **Local-only.** No accounts, no analytics, no crash uploader, no background server. The database lives in your app-data folder and nowhere else.
- **Secrets are never collected.** API keys, OAuth tokens, `auth.json` files, and secret env values have no reader anywhere in the codebase — tests pin their absence with canary values. MCP `env` blocks and settings contribute names/counts, never values.
- **Minimum permissions.** The Tauri capability grants the UI event-subscribe + file-opener only. Lifecycle actions run in Rust, confirmed in the UI, scoped to exe-stem matches.
- **No fabrication.** Unverifiable data renders as `Unknown` / `Not available`, never as invented zeros.

---

## 🗺️ Roadmap

- [ ] Gemini CLI + other adapters (adapter architecture makes these cheap)
- [ ] Token usage / cost tracking
- [ ] MCP & skill management (enable/disable), automatic recovery
- [ ] Community adapter system

`MASTER_PLAN.md` §44 has the full V2–V5 sketch. Out-of-scope ideas stay out until the MVP is stable.

---

## 🤝 Contributing

1. Fork + branch. Keep modules small, UI logic in React, OS logic in Rust, agent specifics inside adapters.
2. TDD: write the failing test first (`cargo test --lib`, `npm test`), watch it fail, then implement.
3. Before pushing: `cargo fmt`, `cargo check` (zero warnings), `cargo test --lib`, `npm test`, `npx tsc --noEmit`, `npm run build`.
4. Never commit secrets, `.db` files, `target/`, or `node_modules/` (see `.gitignore`).

---

## 📄 License

MIT — see [LICENSE](LICENSE). Free for personal and commercial use.
