# AgentHQ — Development

## Prerequisites (Windows)

1. Rust stable MSVC: `winget install --id Rustlang.Rustup -e` → toolchain `stable`,
   target `x86_64-pc-windows-msvc` (verified: rustc/cargo 1.99.0).
2. VS Build Tools 2022 + `Microsoft.VisualStudio.Workload.VCTools`:
   `winget install --id Microsoft.VisualStudio.2022.BuildTools -e --override "--wait --quiet --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"`
   → reboot, then `where.exe link.exe` must resolve.
3. Node 24 + npm (verified: v24.19.0 / v11.17.0). No pnpm.
4. WebView2 runtime (present via Edge; fresh machines need the evergreen installer).

## Commands (run in `agenthq/`)

```bash
npm install
npm run dev            # Vite frontend only
npm run tauri dev      # full desktop app
npm run tauri build    # release bundle -> src-tauri/target/release/bundle/
npx tsc --noEmit       # frontend typecheck
cargo check            # Rust check (in src-tauri/)
```

## Phase 0–1 status: PASS (2026-10-05)

Gate (MASTER_PLAN §47): project created ✓, Tauri runs ✓, React runs ✓,
Rust backend runs ✓, Windows build works (MSI + NSIS setup) ✓, Sidebar ✓,
Dashboard placeholder ✓, theme light/dark ✓, docs ✓, no major warnings ✓.
No agent detection yet — by design (§46 Step 8).

## Verification (MASTER_PLAN §40, every phase)

```bash
# Rust (in agenthq/src-tauri/, MSVC linker first on PATH)
cargo fmt
cargo test --lib
cargo check            # expect: Finished, zero warnings

# Frontend (in agenthq/)
npx tsc --noEmit       # expect: exit 0
npm run build          # expect: vite build pass
```

Note: `cargo test --bin agenthq` is blocked by the OS application-control
policy in some environments; the gate uses `cargo test --lib` (all 140+
tests live in the lib target; the bin holds no tests).

## MVP closeout status (2026-10-05)

- Models + connections persist via replace-per-agent transactions;
  `refresh_models/get_models`, `refresh_connections/get_connections`.
- Detail page: Models tab, Connections (Claude-only), indented process
  tree, Configuration from stored row fields. Projects page in nav.
- Docs: root `README.md`, `docs/security.md`, `docs/security-audit.md`,
  `docs/performance.md`.
- No frontend test runner (no vitest/jest): verification is tsc + build.
