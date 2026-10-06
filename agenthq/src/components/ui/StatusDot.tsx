export type AgentStatus =
  | "running"
  | "offline"
  | "warning"
  | "error"
  | "unknown";

const DOTS: Record<AgentStatus, string> = {
  running: "dot-running",
  offline: "dot-offline",
  warning: "dot-warning",
  error: "dot-error",
  unknown: "dot-unknown",
};

export default function StatusDot({
  status,
  label,
}: {
  status: AgentStatus;
  label?: string;
}) {
  return (
    <span className="status-dot-wrap">
      <span
        className={`status-dot ${DOTS[status]}`}
        aria-hidden="true"
      />
      {label !== undefined && <span>{label}</span>}
    </span>
  );
}
