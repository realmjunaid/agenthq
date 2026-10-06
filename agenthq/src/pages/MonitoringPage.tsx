import { useEffect, useState } from "react";
import PageHeader from "../layouts/PageHeader";
import ContentPanel from "../layouts/ContentPanel";
import { ErrorState } from "../components/ui/States";
import { getHostResources } from "../lib/api";
import { errMessage, formatBytes, formatCpu } from "../lib/format";
import { useDashboard } from "../hooks/useDashboard";
import type { HostView } from "../types/monitor";

const TICK_MS = 2000;

function usedRatio(used: number, total: number): number {
  if (!Number.isFinite(used) || !Number.isFinite(total) || total <= 0) return 0;
  return Math.min(100, Math.max(0, (used / total) * 100));
}

function Bar({ label, value, ratio }: { label: string; value: string; ratio: number }) {
  return (
    <div className="meter">
      <div className="meter-label">
        <span>{label}</span>
        <span>{value}</span>
      </div>
      <div className="meter-track" aria-hidden="true">
        <div className="meter-fill" style={{ width: `${ratio}%` }} />
      </div>
    </div>
  );
}

export default function MonitoringPage() {
  const { data } = useDashboard();
  const [host, setHost] = useState<HostView | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    async function tick() {
      try {
        const next = await getHostResources();
        if (!cancelled) {
          setHost(next);
          setError(null);
        }
      } catch (e) {
        if (!cancelled) setError(errMessage(e));
      }
    }
    void tick();
    const id = window.setInterval(() => void tick(), TICK_MS);
    return () => {
      cancelled = true;
      window.clearInterval(id);
    };
  }, []);

  const diskTotal = host?.disks.reduce((n, d) => n + d.total_bytes, 0) ?? 0;
  const diskFree = host?.disks.reduce((n, d) => n + d.available_bytes, 0) ?? 0;
  const rx = host?.networks.reduce((n, d) => n + d.received_bytes, 0) ?? 0;
  const tx = host?.networks.reduce((n, d) => n + d.transmitted_bytes, 0) ?? 0;

  return (
    <section>
      <PageHeader
        title="Monitoring"
        subtitle={host?.paused ? "Monitoring is paused" : "Refreshes every 2 seconds while this page is open"}
      />
      {error !== null && <ErrorState message={error} />}
      <div className="dash-stats">
        <ContentPanel>
          <Bar
            label="CPU"
            value={host ? formatCpu(host.cpu) : "Unknown"}
            ratio={host ? Math.min(100, host.cpu) : 0}
          />
        </ContentPanel>
        <ContentPanel>
          <Bar
            label="RAM"
            value={
              host
                ? `${formatBytes(host.used_mem_bytes)} / ${formatBytes(host.total_mem_bytes)}`
                : "Unknown"
            }
            ratio={host ? usedRatio(host.used_mem_bytes, host.total_mem_bytes) : 0}
          />
        </ContentPanel>
        <ContentPanel>
          <Bar
            label="Disk"
            value={
              host && diskTotal > 0
                ? `${formatBytes(diskTotal - diskFree)} / ${formatBytes(diskTotal)}`
                : "Unknown"
            }
            ratio={diskTotal > 0 ? usedRatio(diskTotal - diskFree, diskTotal) : 0}
          />
        </ContentPanel>
        <ContentPanel>
          <Bar
            label="Network"
            value={host ? `${formatBytes(rx)} in · ${formatBytes(tx)} out` : "Unknown"}
            ratio={0}
          />
        </ContentPanel>
      </div>
      <h3 className="dash-heading">Agents</h3>
      <ContentPanel>
        <table className="data-table">
          <thead>
            <tr>
              <th>Agent</th>
              <th>CPU</th>
              <th>RAM</th>
              <th>Sessions</th>
            </tr>
          </thead>
          <tbody>
            {(data?.agents ?? []).map((row) => (
              <tr key={row.agent.id}>
                <td>{row.agent.name}</td>
                <td>{formatCpu(row.cpu)}</td>
                <td>{formatBytes(row.ram_bytes)}</td>
                <td>{row.sessions}</td>
              </tr>
            ))}
            {(data?.agents.length ?? 0) === 0 && (
              <tr>
                <td colSpan={4}>No installed agents</td>
              </tr>
            )}
          </tbody>
        </table>
      </ContentPanel>
    </section>
  );
}
