# Security audit — Phase 21 (2026-10-05)

Local-only desktop app. No remote server. Findings below are what was checked, not a claim that every future change stays safe.

## Checked

- **Secrets:** adapters read MCP/env *key names and counts only*. Fixture canaries (`SECRET2`, `SECRET3`, `CANARY-SECRET-1`, `FAKE-SECRET-xyz`) are pinned absent from adapter Debug output. `auth.json` has no reader in the Codex adapter (source pin).
- **Logs:** event builders use names, ids, and counts. Command lines are truncated to 512 chars at collection and are not written into the events table by the current commands.
- **SQLite:** no secret columns. Settings values are `0`/`1` and numeric thresholds.
- **IPC:** commands are Tauri commands in-process. There is no shell plugin, so the webview cannot spawn arbitrary programs. Start/stop/terminal run in Rust and only for an adapter id plus an exe-stem match.
- **Stop safety:** `owned_pids` drops `exe: None`, non-matching stems, and this process's PID. The UI confirms before stop/restart.
- **Filesystem:** no full-disk scan. Config reads are the known agent directories only.
- **Crash/debug:** release profile strips symbols and aborts on panic. No crash-report uploader.

## Changes in this pass

- CSP is no longer `null`. It is `default-src 'self'` plus the Tauri asset host and inline styles.
- Tray and notification plugins do not add a remote endpoint.

## Left as-is, on purpose

- `core:default` is the Tauri template capability. Tightening it to a command allow-list is safer later, but a wrong list breaks the app. Not changed without a runtime check.
- Project-scoped MCP ids can contain a project path (`claude:E:/demo:gh`). That is a path, not a token. A later pass can hash the path into the id if path display in the DB is unwanted.
- `taskkill /F` is destructive for the matched PID only. Confirm is required in the UI; a direct `invoke` from devtools can still call `stop_agent`. Acceptable for a single-user local app; not a multi-user boundary.
