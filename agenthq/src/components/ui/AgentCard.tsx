import Card from "./Card";
import StatusDot, { type AgentStatus } from "./StatusDot";
import Metric from "./Metric";

export type AgentStats = {
  ram: string;
  cpu: string;
  sessions: string;
  subagents: string;
  mcp: string;
  skills: string;
};

const STAT_LABELS: { key: keyof AgentStats; label: string }[] = [
  { key: "ram", label: "RAM" },
  { key: "cpu", label: "CPU" },
  { key: "sessions", label: "Sessions" },
  { key: "subagents", label: "Subagents" },
  { key: "mcp", label: "MCP" },
  { key: "skills", label: "Skills" },
];

export default function AgentCard({
  name,
  vendor,
  status,
  stats,
  onSelect,
}: {
  name: string;
  vendor: string;
  status: AgentStatus;
  stats: AgentStats;
  onSelect?: () => void;
}) {
  return (
    <Card
      title={name}
      actions={
        <StatusDot
          status={status}
          label={status === "running" ? "Running" : status}
        />
      }
    >
      <div className="agent-vendor">{vendor}</div>
      {onSelect !== undefined && (
        <button type="button" className="card-select" onClick={onSelect}>
          Open details
        </button>
      )}
      <div className="agent-stats">
        {STAT_LABELS.map(({ key, label }) => (
          <Metric key={key} label={label} value={stats[key] || "Unknown"} />
        ))}
      </div>
    </Card>
  );
}
