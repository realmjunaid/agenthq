// Mirror of serialized Rust McpServer (mcp engine).
export type McpStatus = "connected" | "configured" | "offline" | "error" | "unknown";

export type McpServer = {
  id: string;
  name: string;
  transport: string;
  command: string | null;
  args: string[];
  url: string | null;
  env_count: number;
  source: string;
  status: McpStatus;
  agent_id: string;
  project_id: string | null;
};
