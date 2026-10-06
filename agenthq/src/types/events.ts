// Live-event contract for the agenthq://event Tauri channel.
// Payload mirrors the stored event row (agenthq/src-tauri/src/events.rs).
export const FRONTEND_EVENT = "agenthq://event";

export type AgentEventName =
  | "agent.installed"
  | "agent.started"
  | "agent.stopped"
  | "agent.error"
  | "agent.detect_failed"
  | "session.started"
  | "session.stopped"
  | "mcp.configured"
  | "mcp.removed"
  | "skill.changed"
  | "plugin.changed"
  | "config.changed";

export type BusEvent = {
  id: number;
  ts: number;
  level: string;
  agent_id: string | null;
  event: AgentEventName;
  message: string;
};
