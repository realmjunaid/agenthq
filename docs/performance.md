# Performance notes — Phase 22 + gap closeout (2026-10-05)

Measured on the release exe (`target/release/agenthq.exe`), Windows 11 Pro 26200 x64, 12 logical cores, machine idle, app idle on dashboard with no interaction:

| Metric | Measured | Target |
|---|---|---|
| Startup (process start → visible window, MainWindowHandle poll) | 802 ms | < 2 s ✓ |
| Idle CPU (10 s sample, process CPU delta ÷ cores) | 0% | < 1–2% ✓ |
| Idle WorkingSet | 31.3 MB | < 150 MB ✓ |
| Idle private bytes | 9.7 MB | < 150 MB ✓ |

Method: PowerShell `Get-Process agenthq` (`CPU` delta over 10 s, `WorkingSet64`, `PrivateMemorySize64`). One sample each, app open ~10 min, no interaction during the window. No tuning was needed — all targets met as built, so no code changed for performance.

## What the code already does

- Process/disk/network refresh is skipped while monitoring is paused.
- The monitoring page polls every 2 seconds, and only while that page is mounted. Unmount clears the interval. Other pages do not poll.
- No 100ms loop. No full-disk scan. Command lines are capped at 512 characters.
- Dashboard is one IPC call, not one call per card.
- sysinfo is built without the `multithread` (rayon) feature.

## Not done, because there was no profile

- No startup-time change. No RAM-target change. Optimizing those without a measurement would be guessing.
- Filesystem watcher (notify 8, known config dirs only, 2 s debounce) now exists — refresh is explicit (button, event, watcher) or the monitoring tick.
