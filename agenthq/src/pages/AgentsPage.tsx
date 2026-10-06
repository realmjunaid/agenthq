import PageHeader from "../layouts/PageHeader";
import ContentPanel from "../layouts/ContentPanel";
import Button from "../components/ui/Button";
import Card from "../components/ui/Card";
import StatusDot, { type AgentStatus } from "../components/ui/StatusDot";
import { EmptyState, ErrorState } from "../components/ui/States";
import { useDashboard } from "../hooks/useDashboard";
import type { AgentRow } from "../types/agent";

function statusOf(agent: AgentRow): AgentStatus {
  if (agent.running) return "running";
  if (agent.installed) return "offline";
  return "unknown";
}

export default function AgentsPage({
  onOpenAgent,
}: {
  onOpenAgent: (id: string) => void;
}) {
  const { data, error, loading, load } = useDashboard();
  const agents = data?.agents ?? [];

  return (
    <section>
      <PageHeader title="Agents" subtitle="Installed and running agents" />
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
            hint="Detection runs at startup. Open the dashboard and refresh if this stays empty."
          />
        </ContentPanel>
      )}
      {agents.length > 0 && (
        <div className="dash-grid">
          {agents.map((row) => {
            const agent = row.agent;
            const status = agent ? statusOf(agent) : "unknown";
            return (
              <Card
                key={agent?.id ?? "unknown"}
                title={agent?.name ?? "Unknown"}
                actions={<StatusDot status={status} label={status === "running" ? "Running" : status} />}
              >
                {agent?.id && (
                  <Button variant="ghost" onClick={() => onOpenAgent(agent.id)}>
                    Open details
                  </Button>
                )}
                <div className="agent-vendor">{agent?.version ?? "Unknown"}</div>
                <div className="state-hint">
                  Sessions {row.sessions ?? 0} · MCP {row.mcp ?? 0} · Skills {row.skills ?? 0}
                </div>
              </Card>
            );
          })}
        </div>
      )}
    </section>
  );
}
