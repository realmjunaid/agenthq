# AgentHQ — Agent Research (verified 2026-10-05, Windows 11 x64)

Method: `where.exe`, `--version`, config-dir existence. Unverified items are
marked — never guessed. Re-verify before implementing each adapter
(MASTER_PLAN §41).

| Agent | Executable | Version | Config dir | Status |
|---|---|---|---|---|
| Claude Code | `C:\Users\j4u87\.local\bin\claude.exe` | 2.1.289 | `~/.claude` present; `%APPDATA%\Claude` absent | VERIFIED present |
| OpenCode | `%APPDATA%\npm\opencode` (npm global) | 2.0.23 | `~/.config/opencode/opencode.json` (plugins key) | VERIFIED present |
| Codex CLI | not on PATH | Unknown | `~/.codex` absent | UNVERIFIED — CLI not installed; all Codex paths/behavior TBD |
| Grok CLI | `~/.grok/bin/grok.exe` (+PATH) | 1.0.46 | `~/.grok` present | VERIFIED present |

## Per-agent detail TODO (fill during adapter phases)

- Claude: VERIFIED 2026-10-05 (v2.1.289). Exe: PATH `claude` + `~/.local/bin/claude.exe`;
  `--version` → first token (`2.1.289`). Config: `~/.claude/` (settings.json keys
  model/env/hooks), `~/.claude.json` (per-project `mcpServers`,
  `enabled/disabledMcpjsonServers`, `lastSessionId`, token counters).
  Skills: `~/.claude/skills/*` dirs (SKILL.md `description:` parsed).
  Sessions: `~/.claude/projects/<slug>/<uuid>.jsonl` (typed lines; timestamp/cwd/
  sessionId/version on user+assistant, `message.model` on assistant; scan capped
  50 lines; status always Unknown). Plugins: `plugins/marketplaces/*/plugins/*`
  + `*/external_plugins/*` dir names. Models: settings `model` value.
  Connections: settings `env` KEY NAMES ONLY, values never read.
  Subagents: NOT counted (sidechain hints exist; counting = guesswork) → Unsupported.
  MCP liveness: never Connected (config-only: Configured/Offline in Phase 9).
  Capabilities: processes/sessions/mcp/skills/plugins/models/connections/
  lifecycle_control=true; subagents/logs=false.
- OpenCode: VERIFIED 2026-10-05 (v2.0.23). Exe: PATH `opencode` +
  `%APPDATA%/npm/opencode.cmd`; `--version` → last token strip `v` (`2.0.23`).
  Global config `~/.config/opencode/opencode.json` (`mcp` object keys = server
  names, `env` never read). Global skills `~/.config/opencode/skills/*`.
  Sessions: `opencode session list --format json -n 100` (per-project of cwd;
  keys id/title/updated/created/projectId/directory; ms epochs) — adapter takes
  optional project_dir, empty without it (Phase 12 wires projects).
  Plugins: `opencode plugin list` (ID column parsed, header skipped).
  Models: `opencode models` (`provider/model` lines). `opencode mcp list` empty here.
  Auth/credentials NEVER touched → connections Unsupported (no safe signal).
  Subagents: out of §17 scope → Unsupported.
  Capabilities: processes/sessions/mcp/skills/plugins/models/lifecycle_control=true;
  subagents/connections/logs=false.
- Codex: VERIFIED 2026-10-05 (codex-cli 0.160.0, RUNNING). Exe NOT on PATH;
  live at `~/.codex/packages/app-server-daemon/releases/<ver>-x86_64-pc-windows-msvc/bin/codex.exe`
  (self-updating; adapter globs + picks max version). `--version` → last token.
  Config `~/.codex/` (config.toml safe keys tui/windows/features; `auth.json`
  EXISTS — never opened). Sessions: `session_index.jsonl` `{id,thread_name,
  updated_at}` (+ rollout transcripts with session_meta incl. model_provider but
  no model name). Skills: `~/.codex/skills/*` dirs. Plugins: `~/.codex/plugins/*`.
  Models: `models_cache.json` `models[].slug`. MCP: config.toml `[mcp_servers.<name>]`
  headers (none configured here). Connections: Unsupported (only signal is
  auth.json presence — unsafe). Subagents: Unsupported (no inventing).
  Capabilities: processes/sessions/mcp/skills/plugins/models/lifecycle_control=true;
  subagents/connections/logs=false.
- Grok: VERIFIED 2026-10-05 (grok 1.0.46 `[stable]`). Exe `~/.grok/bin/grok.exe`
  (+PATH); `--version` → first version-shaped token (`1.0.46`).
  Config `~/.grok/` (config.toml sections cli/marketplace/ui/privacy — no MCP
  section; `auth.json` EXISTS — never opened; `settings_cache.json` keys
  payload/signature — opaque, never parsed). Sessions:
  `sessions/<url-encoded-project>/<hash>/` with `usage.json`
  `{sessionId, updatedAt, session.primaryModelId}` + project-level
  `prompt_history.jsonl` `{timestamp, session_id}` (RFC3339 hand-parsed, capped
  200 lines; status always Unknown; confidence Confirmed — file-derived).
  Skills: `~/.grok/skills/*` dirs (SKILL.md `description:` parsed, shared helper).
  Plugins: `installed-plugins/` dirs minus `*.lock` (only registry.lock here →
  empty — marketplace-cache/ is AVAILABLE not installed, not read).
  Models: `models_cache.json` `models` string array (file read, no CLI auth needed).
  MCP: `grok mcp list --json` → `[]` exit 0 (schema for non-empty unverified:
  objects with string `name`/`id` accepted, env keys counted never read).
  `sessions list` is cwd-scoped (empty here) → file discovery instead.
  Connections: Unsupported (only signals are secrets). Subagents: Unsupported.
  Capabilities: processes/sessions/mcp/skills/plugins/models/lifecycle_control=true;
  subagents/connections/logs=false.

## Confidence rule

Detection evidence ranks: running process > PATH executable > config dir >
CLI output. Anything below "detected" renders as `Unknown`/`Unavailable`.
