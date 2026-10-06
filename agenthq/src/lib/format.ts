/** Human size. Non-finite input stays Unknown — never invent a number. */
export function formatBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes < 0) return "Unknown";
  if (bytes === 0) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  const i = Math.min(
    Math.floor(Math.log(bytes) / Math.log(1024)),
    units.length - 1,
  );
  const value = bytes / 1024 ** i;
  const digits = value >= 10 || i === 0 ? 0 : 1;
  return `${value.toFixed(digits)} ${units[i]}`;
}

export function formatCpu(cpu: number): string {
  if (!Number.isFinite(cpu)) return "Unknown";
  return `${cpu.toFixed(1)}%`;
}

/** null/undefined means the capability has no producer (Not available). */
export function formatCount(n: number | null | undefined): string {
  if (n === null || n === undefined) return "Not available";
  if (!Number.isFinite(n)) return "Unknown";
  return String(n);
}

/** Epoch seconds → short relative label. */
export function formatAgo(ts: number): string {
  if (!Number.isFinite(ts) || ts <= 0) return "Unknown";
  const delta = Math.max(0, Math.floor(Date.now() / 1000) - ts);
  if (delta < 60) return `${delta}s ago`;
  if (delta < 3600) return `${Math.floor(delta / 60)}m ago`;
  if (delta < 86400) return `${Math.floor(delta / 3600)}h ago`;
  return `${Math.floor(delta / 86400)}d ago`;
}

export function errMessage(e: unknown): string {
  if (e instanceof Error && e.message) return e.message;
  if (typeof e === "string" && e) return e;
  return "Unknown error";
}
