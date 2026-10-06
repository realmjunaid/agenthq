# Phase 0–1 (Discovery + Bootstrap) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Verify the Windows toolchain and produce a running Tauri 2 + React + TS app shell with docs.

**Architecture:** No code architecture yet — this phase installs prerequisites, records verified environment facts in `docs/`, then scaffolds the standard Tauri 2 + React-TS template and adds a minimal AppShell (Sidebar/Topbar/placeholders + theme tokens).

**Tech Stack:** Rust (stable, MSVC target), Tauri 2, Node 24 + npm, React + TypeScript + Vite, Tailwind (added at scaffold).

**Spec:** `MASTER_PLAN.md` §46 FIRST TASK + §47 SUCCESS CRITERIA (Steps 1–7; no agents/MCP/monitoring in this phase).

## Global Constraints

- Target: Windows 10/11 x64. Verified host: Windows 11 Pro 26200, x64.
- No Electron, no Node/Python background server (MASTER_PLAN §0 rules 2–3).
- Do NOT implement agent detection, MCP, process monitoring, or lifecycle control in this phase (MASTER_PLAN §46 Step 8).
- Never store or log secrets; nothing secret exists in this phase.
- Prefer npm (verified present v11.17.0); pnpm is NOT installed — do not require it.
- Commit messages follow `MASTER_PLAN.md` §39 (e.g. `chore: ...`, `feat: ...`, `docs: ...`).

## Verified Environment (2026-10-05, do not re-guess)

- Rust/Cargo: MISSING → Task 1 installs it.
- VS Build Tools / MSVC (`cl.exe`, `link.exe`): MISSING → Task 2 installs it.
- Node v24.19.0 + npm v11.17.0: present.
- WebView2: present via Edge 154.0.4258.53 (HKLM EdgeUpdate client version key).
- Claude Code 2.1.289: present. OpenCode 2.0.23: present. Codex CLI: MISSING (adapter research for Codex is docs-only until installed).
- pnpm: MISSING (do not depend on it).

## Review Focus

- Fresh PowerShell shells must see `cargo` (rustup PATH) — Task 1 pins a reopen-shell check, else later tasks fail opaquely.
- `link.exe` must exist before any `cargo` build — Task 2 pins the check, else Task 6 fails after a long compile.
- Scaffold templates drift (tauri-app output names/flags change) — Task 4 pins verification by file existence + `cargo check`, not by assumed template text.
- Codex CLI absent means Codex adapter research stays unverified — Task 3 records this explicitly instead of guessing paths.

---

### Task 1: Install Rust toolchain

**Files:**
- Modify: none (machine state)
- Test: shell probe (no repo test yet)

**Interfaces:**
- Consumes: none
- Produces: `rustc --version` and `cargo --version` succeed in a FRESH PowerShell; `rustup target list --installed` contains `x86_64-pc-windows-msvc`

- [ ] **Step 1: Install via rustup**
Run: `winget install --id Rustlang.Rustup -e` (fallback: download `rustup-init.exe` from https://rustup.rs and run with default stable toolchain)
Expected: installer exits 0
- [ ] **Step 2: Verify in a fresh shell**
Run (new `shell` invocation): `rustc --version; cargo --version`
Expected: both print versions, e.g. `rustc 1.8x.x`
- [ ] **Step 3: Verify MSVC target**
Run: `rustup target list --installed`
Expected: output contains `x86_64-pc-windows-msvc`

### Task 2: Install VS Build Tools (C++ linker)

**Files:**
- Modify: none (machine state)

**Interfaces:**
- Consumes: Task 1 (`cargo` on PATH, for later tasks only)
- Produces: `link.exe` resolvable via `vswhere` or VS installer path

- [ ] **Step 1: Install Build Tools with C++ workload**
Run: `winget install --id Microsoft.VisualStudio.2022.BuildTools -e --override "--wait --quiet --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"`
Expected: installer exits 0 (takes several minutes; use long timeout)
- [ ] **Step 2: Verify linker**
Run: `where.exe link.exe` (or `vswhere -latest -find VC\Tools\MSVC\*\bin\Hostx64\x64\link.exe`)
Expected: prints a path under `...\Microsoft Visual Studio\...`

### Task 3: Write Phase-0 docs

**Files:**
- Create: `docs/architecture.md`
- Create: `docs/agent-research.md`
- Create: `docs/development.md`

**Interfaces:**
- Consumes: Verified Environment block above
- Produces: three docs; `docs/agent-research.md` contains a per-agent table (executable, config dir, MCP location, session source, status: verified/unverified)

- [ ] **Step 1: Write `docs/architecture.md`**
Content: stack (Tauri 2 + Rust + React + TS + SQLite), adapter-trait rule, module map for `src-tauri/` (agents/traits, manager, monitoring, database, commands, events) and `src/` (app/components/pages/hooks/types) copied from MASTER_PLAN §5/§9. No code.
- [ ] **Step 2: Write `docs/agent-research.md`**
Content: probe each agent — executable on PATH (`where.exe claude/codex/opencode`), `%APPDATA%`/`.config` dirs, `--version` output. Record Codex as UNVERIFIED (CLI missing). Never invent a path; mark unknowns `Unknown`.
- [ ] **Step 3: Write `docs/development.md`**
Content: verified setup commands (rustup, VSBT, `npm install`, `npm run tauri dev`, `npm run tauri build`), Node/npm versions, Tauri Windows prerequisites link.
- [ ] **Step 4: Verify docs**
Run: `Get-ChildItem docs/*.md | Select-Object Name` lists all three; grep finds `Unknown` or `UNVERIFIED` in agent-research (proves no fabrication).
Expected: 3 files present, honesty markers present.

### Task 4: Scaffold Tauri 2 + React-TS app

**Files:**
- Create: `<APP_DIR>/` (scaffold output: `package.json`, `vite.config.ts`, `src/`, `src-tauri/`, `index.html`)
- Modify: none

**Interfaces:**
- Consumes: Tasks 1–2 (cargo + linker)
- Produces: `<APP_DIR>/package.json` with scripts `dev`, `build`, `tauri`; `<APP_DIR>/src-tauri/tauri.conf.json` with `productName`

- [ ] **Step 1: Scaffold with npm create**
Run in project root: `npm create tauri-app@latest <APP_DIR> -- --template react-ts --manager npm` (non-interactive flags; adjust only if the CLI rejects them)
Expected: exit 0, `<APP_DIR>/src-tauri/Cargo.toml` exists
- [ ] **Step 2: Install frontend deps**
Run: `npm install` in `<APP_DIR>`
Expected: exit 0, `node_modules/` exists
- [ ] **Step 3: Verify Rust backend compiles**
Run: `cargo check` in `<APP_DIR>/src-tauri`
Expected: exit 0 (first run compiles deps; allow long timeout)

### Task 5: Minimal AppShell UI

**Files:**
- Create: `<APP_DIR>/src/app/AppShell.tsx` (Sidebar + Topbar + main outlet)
- Create: `<APP_DIR>/src/app/theme.css` (design tokens from MASTER_PLAN §3 as CSS variables, light + dark)
- Create: `<APP_DIR>/src/pages/DashboardPage.tsx`, `AgentsPage.tsx`, `SettingsPage.tsx` (placeholder headings only)
- Modify: `<APP_DIR>/src/App.tsx` (render AppShell + placeholder route state; no router lib yet)

**Interfaces:**
- Consumes: Task 4 scaffold (`src/main.tsx` mounting `App`)
- Produces: `AppShell` component (props: none; internal `useState` page switch); CSS vars `--bg --surface --border --primary --text --muted` + `[data-theme="dark"]` overrides

- [ ] **Step 1: Write theme tokens**
`theme.css` defines the 8 light vars + 8 dark overrides with the exact hex values from MASTER_PLAN §3. No hard-coded colors elsewhere.
- [ ] **Step 2: Write AppShell + pages**
Sidebar: buttons Dashboard/Agents/Settings. Topbar: product name + theme toggle (toggles `data-theme` on root). Main: renders active placeholder page.
- [ ] **Step 3: Typecheck**
Run: `npx tsc --noEmit` in `<APP_DIR>`
Expected: exit 0, no errors

### Task 6: Run + Windows build verify

**Files:**
- Modify: none

**Interfaces:**
- Consumes: Tasks 4–5
- Produces: dev server boots; release bundle (`.exe`/installer under `src-tauri/target/release/bundle/`)

- [ ] **Step 1: Dev server boots**
Run: `npm run tauri dev` (background, then stop after window compiles — or `npm run dev` + `cargo check` if GUI unavailable headless)
Expected: Vite ready + Tauri compiled without errors
- [ ] **Step 2: Release build**
Run: `npm run tauri build`
Expected: exit 0; bundle artifact exists under `src-tauri/target/release/bundle/`
- [ ] **Step 3: Phase gate check**
Confirm MASTER_PLAN §47 list: project created, Tauri runs, React runs, Rust backend runs, Windows build works, Sidebar/Dashboard/theme exist, docs exist, no major warnings.
Expected: all true; record result at top of `docs/development.md` as `Phase 0–1 status: PASS (YYYY-MM-DD)`.
