// Mirror of serialized Rust DashboardData (dashboard engine).
import type { AgentRow } from "./agent";
import type { SystemStats } from "./process";

export type DashboardAgent = {
  agent: AgentRow;
  ram_bytes: number;
  cpu: number;
  sessions: number;
  subagents: number | null;
  mcp: number;
  skills: number;
};

export type DashboardData = {
  agents: DashboardAgent[];
  total_ram_bytes: number;
  total_cpu: number;
  running: number;
  system: SystemStats;
};
