import PageHeader from "../layouts/PageHeader";
import ContentPanel from "../layouts/ContentPanel";
import Button from "../components/ui/Button";
import StatCard from "../components/ui/StatCard";
import AgentCard from "../components/ui/AgentCard";
import type { AgentStatus } from "../components/ui/StatusDot";
import { EmptyState, ErrorState } from "../components/ui/States";
import { useDashboard } from "../hooks/useDashboard";
import { formatAgo, formatBytes, formatCount, formatCpu } from "../lib/format";
import type { AgentRow } from "../types/agent";
import type { DashboardAgent } from "../types/dashboard";

function statusOf(agent: AgentRow): AgentStatus {
  if (agent.running) return "running";
  if (agent.installed) return "offline";
  return "unknown";
}

function cardStats(row: DashboardAgent) {
  return {
    ram: formatBytes(row.ram_bytes ?? 0),
    cpu: formatCpu(row.cpu ?? 0),
    sessions: formatCount(row.sessions),
    subagents: formatCount(row.subagents),
    mcp: formatCount(row.mcp),
    skills: formatCount(row.skills),
  };
}

export default function DashboardPage({
  onOpenAgent,
}: {
  onOpenAgent: (id: string) => void;
}) {
  const { data, events, error, loading, refreshing, load, refresh } =
    useDashboard();

  const agents = data?.agents ?? [];
  const mcpTotal = agents.reduce((n, a) => n + (a.mcp ?? 0), 0);

  return (
    <section>
      <PageHeader
        title="Dashboard"
        subtitle="Agent overview"
        actions={
          <Button variant="secondary" onClick={() => void refresh()} disabled={refreshing}>
            {refreshing ? "Refreshing…" : "Refresh"}
          </Button>
        }
      />
      {error !== null && <ErrorState message={error} onRetry={() => void load()} />}
      {loading && data === null && error === null && (
        <ContentPanel>
          <div className="state-hint">Loading…</div>
        </ContentPanel>
      )}
      {data !== null && agents.length === 0 && error === null && (
        <ContentPanel>
          <EmptyState
            title="No agents detected yet"
            hint="Install Claude Code, OpenCode, or Codex, then refresh."
            action={{ label: "Refresh", onClick: () => void refresh() }}
          />
        </ContentPanel>
      )}
      {data !== null && agents.length > 0 && (
        <>
          <div className="dash-stats">
            <StatCard label="Agents" value={String(agents.length)} />
            <StatCard label="Running" value={String(data.running ?? 0)} />
            <StatCard label="MCP" value={String(mcpTotal)} />
            <StatCard
              label="Agent RAM"
              value={formatBytes(data.total_ram_bytes ?? 0)}
              delta={`${formatBytes(data.system?.used_mem_bytes ?? 0)} / ${formatBytes(data.system?.total_mem_bytes ?? 0)} system`}
            />
          </div>
          <div className="dash-grid">
            {agents.map((row) => (
              <AgentCard
                key={row.agent?.id ?? row.agent?.name}
                name={row.agent?.name ?? "Unknown"}
                vendor={row.agent?.version ?? "Unknown"}
                status={statusOf(row.agent ?? { id: "", name: "", agent_type: "", version: null, executable_path: null, installed: false, running: false, last_seen: null })}
                stats={cardStats(row)}
                onSelect={
                  row.agent?.id ? () => onOpenAgent(row.agent.id) : undefined
                }
              />
            ))}
          </div>
          <h3 className="dash-heading">Recent activity</h3>
          <ContentPanel>
            {events.length === 0 ? (
              <div className="state-hint">No events yet.</div>
            ) : (
              <ul className="activity-list">
                {events.map((ev) => (
                  <li key={ev.id} className="activity-item">
                    <span>{ev.message || ev.event || "Unknown"}</span>
                    <span className="activity-time">{formatAgo(ev.ts)}</span>
                  </li>
                ))}
              </ul>
            )}
          </ContentPanel>
        </>
      )}
    </section>
  );
}
