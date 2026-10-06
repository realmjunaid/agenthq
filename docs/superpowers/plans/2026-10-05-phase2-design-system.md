# Phase 2 (Design System) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the reusable UI primitive + layout library the dashboard and agent pages will compose.

**Architecture:** Hand-rolled primitives in `agenthq/src/components/ui/` (one concern per file, props-typed, token-styled, no hard-coded colors), layout shell in `agenthq/src/layouts/` (existing `app/AppShell.tsx` split into Sidebar/Topbar + panels). No new npm dependencies — Tailwind v4 + CSS tokens cover styling.

**Tech Stack:** React 19 + TypeScript strict + Tailwind v4 + CSS vars (`src/app/theme.css`).

**Spec:** `MASTER_PLAN.md` §2 (design reference), §3 (tokens), §12 (component list), §35 (required UI states), §36 (accessibility).

## Global Constraints

- No hard-coded colors/spacing outside `theme.css` — every component uses `var(--*)` or Tailwind utilities that resolve to tokens.
- No new npm dependencies (hand-rolled over shadcn/Radix for Phase 2; "lightweight equivalent" per §12).
- Every interactive control keyboard-operable with visible focus (`:focus-visible` already global); Dialog/Dropdown close on Escape; appropriate `aria-*` (label, expanded, current, modal).
- No git in this repo yet — no commit steps; ledger is the record.
- Verification per task: `npx tsc --noEmit` (exit 0) + `npm run build` (exit 0). No test runner installed yet — component tests deferred to testing phase (see Ruling).

## Review Focus

- A Button rendered `disabled` must not fire `onClick` and must look disabled — Task 1 pins `disabled` styling + native `disabled` passthrough, verified by code read (no runner yet).
- Dropdown/Tooltip must not trap keyboard focus or leave orphan open state on unmount — Task 3 pins Escape-close + outside-click-close + cleanup in effect return.
- Empty/Error/Unsupported states must render words, never blank space or fake zeros — Task 4 pins exact copy per §35.
- Dark theme must restyle every primitive — Task 5 pins a token-audit grep (no `#[0-9a-f]{3,6}` outside theme.css).
- Long labels must not break card/sidebar layout — Task 5 pins overflow-ellipsis rules on Card title + nav buttons.

---

### Task 1: Atoms (Button, Badge, StatusDot, Input/Search, Metric)

**Files:**
- Create: `agenthq/src/components/ui/Button.tsx`, `Badge.tsx`, `StatusDot.tsx`, `Input.tsx`, `Metric.tsx`

**Interfaces:**
- Consumes: `theme.css` vars (`--primary --surface --border --text --muted --success --warning --error --info`)
- Produces:
  - `Button(props: ButtonHTMLAttributes<HTMLButtonElement> & { variant?: "primary" | "secondary" | "ghost" | "danger" })`
  - `Badge(props: { tone?: "neutral" | "success" | "warning" | "error" | "info"; children: ReactNode })`
  - `StatusDot(props: { status: "running" | "offline" | "warning" | "error" | "unknown"; label?: string })` — dot color maps running→success, offline→muted, warning→warning, error→error, unknown→muted hollow
  - `Input(props: InputHTMLAttributes<HTMLInputElement>)`, `Search(props: InputHTMLAttributes<HTMLInputElement> & { onSearch?: (v: string) => void })` — Search renders Input + Enter-key `onSearch`
  - `Metric(props: { label: string; value: string; hint?: string })`

- [ ] **Step 1: Write the five components**
Each: typed props above, `className` merge via template string, zero hard-coded colors.
- [ ] **Step 2: Verify types + build**
Run in `agenthq/`: `npx tsc --noEmit` then `npm run build`
Expected: both exit 0
- [ ] **Step 3: Verify disabled + token rules**
Grep `src/components/ui` for `#[0-9a-fA-F]{3,6}` → no matches; read Button file → `disabled` passed to native `<button>`.
Expected: no hex matches; native disabled present

### Task 2: Cards (Card, StatCard, AgentCard)

**Files:**
- Create: `agenthq/src/components/ui/Card.tsx`, `StatCard.tsx`, `AgentCard.tsx`

**Interfaces:**
- Consumes: Task 1 `Badge`, `StatusDot`, `Metric` (exact props above)
- Produces:
  - `Card(props: { title?: string; actions?: ReactNode; children: ReactNode; className?: string })`
  - `StatCard(props: { label: string; value: string; delta?: string; tone?: Badge["tone"] })` — Card + Metric + optional Badge
  - `AgentCard(props: { name: string; vendor: string; status: StatusDot["status"]; stats: { ram: string; cpu: string; sessions: string; subagents: string; mcp: string; skills: string } })` — Card + StatusDot header + Metric grid; unknown stats render `"Unknown"`, never `""`

- [ ] **Step 1: Write the three components**
Card title uses ellipsis (`overflow hidden`, `text-overflow ellipsis`, `white-space nowrap`).
- [ ] **Step 2: Verify types + build**
Run in `agenthq/`: `npx tsc --noEmit` then `npm run build`
Expected: both exit 0

### Task 3: Controls (Tabs, SegmentedControl, Dropdown, Tooltip, Dialog)

**Files:**
- Create: `agenthq/src/components/ui/Tabs.tsx`, `SegmentedControl.tsx`, `Dropdown.tsx`, `Tooltip.tsx`, `Dialog.tsx`

**Interfaces:**
- Consumes: Task 1 `Button` (ghost variant for triggers)
- Produces:
  - `Tabs(props: { tabs: { id: string; label: string }[]; active: string; onChange: (id: string) => void })` — buttons with `aria-selected`, arrow-key navigation optional (skip if costly → ledger note)
  - `SegmentedControl(props: { options: { value: string; label: string }[]; value: string; onChange: (v: string) => void })` — `role="radiogroup"`, options `role="radio"` + `aria-checked`
  - `Dropdown(props: { label: string; items: { id: string; label: string }[]; onSelect: (id: string) => void })` — `aria-expanded`, Escape closes, outside-click closes, effect cleanup removes listeners
  - `Tooltip(props: { tip: string; children: ReactNode })` — CSS-only on hover/focus, `role="tooltip"`
  - `Dialog(props: { open: boolean; title: string; onClose: () => void; children: ReactNode })` — `role="dialog"` + `aria-modal`, Escape calls `onClose`, renders null when closed

- [ ] **Step 1: Write the five components** per signatures above.
- [ ] **Step 2: Verify types + build**
Run in `agenthq/`: `npx tsc --noEmit` then `npm run build`
Expected: both exit 0
- [ ] **Step 3: Verify focus/close behavior by read**
Read Dropdown + Dialog files → Escape handler + effect-return cleanup both present.
Expected: both patterns present

### Task 4: States + panels (EmptyState, ErrorState, PageHeader, ContentPanel, InspectorPanel)

**Files:**
- Create: `agenthq/src/components/ui/States.tsx`, `agenthq/src/layouts/PageHeader.tsx`, `ContentPanel.tsx`, `InspectorPanel.tsx`

**Interfaces:**
- Consumes: Task 1 `Button`; Task 2 `Card`
- Produces:
  - `EmptyState(props: { title: string; hint?: string; action?: { label: string; onClick: () => void } })`
  - `ErrorState(props: { message: string; onRetry?: () => void })`
  - `UnsupportedState(props: { feature: string })` — copy: `"{feature} is not supported by this agent."` (§35 over §7)
  - `PageHeader(props: { title: string; subtitle?: string; actions?: ReactNode })`
  - `ContentPanel(props: { children: ReactNode })`, `InspectorPanel(props: { title: string; children: ReactNode })`

- [ ] **Step 1: Write the six components** with the exact `UnsupportedState` copy above.
- [ ] **Step 2: Verify types + build**
Run in `agenthq/`: `npx tsc --noEmit` then `npm run build`
Expected: both exit 0

### Task 5: Shell refactor onto the system

**Files:**
- Create: `agenthq/src/layouts/AppShell.tsx`, `Sidebar.tsx`, `Topbar.tsx`
- Modify: `agenthq/src/App.tsx` (import from layouts), `agenthq/src/app/AppShell.tsx` (delete after move), `agenthq/src/pages/*.tsx` (compose PageHeader + ContentPanel + EmptyState placeholders), `agenthq/src/app/theme.css` (only if a token is missing)

**Interfaces:**
- Consumes: Tasks 1–4 (all components above)
- Produces: `Sidebar(props: { page: Page; onNavigate: (p: Page) => void })` where `type Page = "dashboard" | "agents" | "settings"` (lives in `layouts/AppShell.tsx`, exported); `Topbar(props: { theme: "light" | "dark"; onToggleTheme: () => void })`; theme state stays in AppShell

- [ ] **Step 1: Move + split shell**
New AppShell owns `page`/`theme` state (same logic as current file); Sidebar/Topbar presentational; nav buttons get `aria-current` + ellipsis; toggle gets `aria-label`; delete old `app/AppShell.tsx`.
- [ ] **Step 2: Compose placeholders**
Each page: PageHeader (title) + ContentPanel + EmptyState (e.g. Dashboard → "No agents detected yet.").
- [ ] **Step 3: Verify types + build + token audit**
Run in `agenthq/`: `npx tsc --noEmit`, `npm run build`, then grep `src/` for `#[0-9a-fA-F]{3,6}` excluding `theme.css`.
Expected: exits 0, 0; grep shows matches only in `theme.css`
