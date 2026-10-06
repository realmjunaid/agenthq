import { invoke } from "@tauri-apps/api/core";
import type { DashboardData } from "../types/dashboard";
import type { BusEvent } from "../types/events";
import type { McpServer } from "../types/mcp";
import type { Plugin } from "../types/plugin";
import type { HostView } from "../types/monitor";
import type { Connection, Model } from "../types/model";
import type { ProcessInfo } from "../types/process";
import type { Project, Session } from "../types/session";
import type { Skill } from "../types/skill";

export function getDashboard(): Promise<DashboardData> {
  return invoke<DashboardData>("get_dashboard");
}

export function getEvents(limit = 50): Promise<BusEvent[]> {
  return invoke<BusEvent[]>("get_events", { limit });
}

export function getSessions(agentId?: string): Promise<Session[]> {
  return invoke<Session[]>("get_sessions", { agentId: agentId ?? null });
}

export function getMcpServers(agentId?: string): Promise<McpServer[]> {
  return invoke<McpServer[]>("get_mcp_servers", { agentId: agentId ?? null });
}

export function getSkills(agentId?: string): Promise<Skill[]> {
  return invoke<Skill[]>("get_skills", { agentId: agentId ?? null });
}

export function getPlugins(agentId?: string): Promise<Plugin[]> {
  return invoke<Plugin[]>("get_plugins", { agentId: agentId ?? null });
}

export function getModels(agentId?: string): Promise<Model[]> {
  return invoke<Model[]>("get_models", { agentId: agentId ?? null });
}

export function getConnections(agentId?: string): Promise<Connection[]> {
  return invoke<Connection[]>("get_connections", { agentId: agentId ?? null });
}

export function getProjects(): Promise<Project[]> {
  return invoke<Project[]>("get_projects");
}

export function getProcessSnapshot(): Promise<ProcessInfo[]> {
  return invoke<ProcessInfo[]>("get_process_snapshot");
}

export function getHostResources(): Promise<HostView> {
  return invoke<HostView>("get_host_resources");
}

export function getSettings(): Promise<[string, string][]> {
  return invoke<[string, string][]>("get_settings");
}

export function setSetting(key: string, value: string): Promise<void> {
  return invoke("set_setting", { key, value });
}

export function stopAgent(agentId: string): Promise<number[]> {
  return invoke<number[]>("stop_agent", { agentId });
}

export function startAgent(agentId: string): Promise<void> {
  return invoke("start_agent", { agentId });
}

export function openAgentTerminal(agentId: string): Promise<void> {
  return invoke("open_agent_terminal", { agentId });
}

export function refreshAgents(): Promise<unknown> {
  return invoke("refresh_agents");
}

export function refreshSessions(projectDir?: string): Promise<unknown> {
  return invoke("refresh_sessions", { projectDir: projectDir ?? null });
}

export function refreshMcp(): Promise<unknown> {
  return invoke("refresh_mcp");
}

export function refreshSkills(): Promise<unknown> {
  return invoke("refresh_skills");
}

export function refreshPlugins(): Promise<unknown> {
  return invoke("refresh_plugins");
}

export function refreshModels(): Promise<unknown> {
  return invoke("refresh_models");
}

export function refreshConnections(): Promise<unknown> {
  return invoke("refresh_connections");
}

/** Sequential refresh so a failure names the step that broke. */
export async function refreshAll(): Promise<void> {
  await refreshAgents();
  await refreshSessions();
  await refreshMcp();
  await refreshSkills();
  await refreshPlugins();
  await refreshModels();
  await refreshConnections();
}
