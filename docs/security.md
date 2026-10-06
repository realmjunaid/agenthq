# AgentHQ — Security (MASTER_PLAN §37)

Minimum-permissions principle: the app requires only what a local
observer needs. No admin rights, no remote endpoints, no credential storage.

## What is never collected

- API keys, OAuth tokens, passwords, private keys — no reader exists.
- Secret env values: MCP `env` blocks and settings `env` contribute key
  names and counts only. Fixture canaries pin their absence from adapter output.
- Codex `auth.json`: no code path opens it (source pin in tests).
- Command lines are truncated to 512 chars at collection and never enter the events table.

## Boundaries

- Webview cannot spawn programs: there is no shell plugin. Start/stop/terminal
  run in Rust, scoped to an adapter id plus an exe-stem match. `exe: None`
  never matches; this process's own PID is always excluded; the UI confirms
  destructive actions.
- SQLite holds names, counts, paths of the user's own config, and event text
  built from names/ids/counts. No tokens.
- Project-scoped MCP ids can contain a project path (`claude:E:/demo:gh`).
  A path, not a secret — but noted for a future id-hashing pass.
- Release profile strips symbols and aborts on panic. No crash reporter.

## IPC and capabilities

- Commands are in-process Tauri commands. Capability is minimal:
  `core:event:allow-listen`, `core:event:allow-unlisten`, `opener:default`.
  App-command `invoke` needs no entry (no app ACL manifest); backend calls
  need none. Runtime-checked on the release build (dashboard data loads).
- Notifications and tray show names and counts only.

Full audit trail: `docs/security-audit.md`.
