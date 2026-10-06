// Mirror of the serialized Rust AgentRow (database::repository).
// Phase 14 wires AgentsPage to the `list_agents` Tauri command.
export type AgentRow = {
  id: string;
  name: string;
  agent_type: string;
  version: string | null;
  executable_path: string | null;
  installed: boolean;
  running: boolean;
  last_seen: number | null;
};
