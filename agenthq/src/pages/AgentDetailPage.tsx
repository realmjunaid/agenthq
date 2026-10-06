import { useCallback, useEffect, useState } from "react";
import { openPath } from "@tauri-apps/plugin-opener";
import Dialog from "../components/ui/Dialog";
import PageHeader from "../layouts/PageHeader";
import ContentPanel from "../layouts/ContentPanel";
import InspectorPanel from "../layouts/InspectorPanel";
import Button from "../components/ui/Button";
import Tabs from "../components/ui/Tabs";
import Metric from "../components/ui/Metric";
import StatusDot, { type AgentStatus } from "../components/ui/StatusDot";
import { EmptyState, ErrorState } from "../components/ui/States";
import {
  getConnections,
  getEvents,
  getMcpServers,
  getModels,
  getPlugins,
  getProcessSnapshot,
  getProjects,
  getSessions,
  getSkills,
  refreshAll,
  openAgentTerminal,
  startAgent,
  stopAgent,
} from "../lib/api";
import { errMessage, formatAgo, formatBytes, formatCount, formatCpu } from "../lib/format";
import { useDashboard } from "../hooks/useDashboard";
import type { AgentRow } from "../types/agent";
import type { BusEvent } from "../types/events";
import type { McpServer } from "../types/mcp";
import type { Connection, Model } from "../types/model";
import type { Plugin } from "../types/plugin";
import type { ProcessInfo } from "../types/process";
import type { Project, Session } from "../types/session";
import type { Skill } from "../types/skill";

// Mirrors the adapter capabilities in src-tauri/src/agents/traits.rs:
// connections() is Unsupported by default and only the Claude adapter
// overrides it (settings env key names). All three adapters implement
// models(), so an empty model list means "none detected", not unsupported.
const CONNECTIONS_AGENTS = ["claude"];

const TABS = [
  { id: "overview", label: "Overview" },
  { id: "sessions", label: "Sessions" },
  { id: "processes", label: "Processes" },
  { id: "mcp", label: "MCP" },
  { id: "skills", label: "Skills" },
  { id: "plugins", label: "Plugins" },
  { id: "models", label: "Models" },
  { id: "logs", label: "Logs" },
  { id: "configuration", label: "Configuration" },
];

function statusOf(agent: AgentRow): AgentStatus {
  if (agent.running) return "running";
  if (agent.installed) return "offline";
  return "unknown";
}

function stem(path: string | null | undefined): string {
  if (!path) return "";
  const base = path.split(/[/\\]/).pop() ?? "";
  return base.replace(/\.(exe|cmd)$/i, "").toLowerCase();
}

/** Depth of each row by parent_pid chain. Orphans and cycles stay at 0. */
export function treeDepths(rows: ProcessInfo[]): Map<number, number> {
  const byPid = new Map(rows.map((p) => [p.pid, p]));
  const depths = new Map<number, number>();
  for (const row of rows) {
    let depth = 0;
    let cursor = row.parent_pid;
    const seen = new Set<number>([row.pid]);
    while (cursor !== null && cursor !== undefined && byPid.has(cursor) && depth < 16) {
      if (seen.has(cursor)) break;
      seen.add(cursor);
      depth += 1;
      cursor = byPid.get(cursor)?.parent_pid ?? null;
    }
    depths.set(row.pid, depth);
  }
  return depths;
}

export default function AgentDetailPage({
  agentId,
  onBack,
}: {
  agentId: string;
  onBack: () => void;
}) {
  const { data, error: dashError, loading, load } = useDashboard();
  const row = data?.agents.find((a) => a.agent?.id === agentId) ?? null;
  const agent = row?.agent ?? null;

  const [tab, setTab] = useState("overview");
  const [refreshing, setRefreshing] = useState(false);
  const [confirm, setConfirm] = useState<"stop" | "restart" | null>(null);
  const [actionError, setActionError] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [sessions, setSessions] = useState<Session[]>([]);
  const [processes, setProcesses] = useState<ProcessInfo[]>([]);
  const [mcp, setMcp] = useState<McpServer[]>([]);
  const [skills, setSkills] = useState<Skill[]>([]);
  const [plugins, setPlugins] = useState<Plugin[]>([]);
  const [models, setModels] = useState<Model[]>([]);
  const [connections, setConnections] = useState<Connection[]>([]);
  const [logs, setLogs] = useState<BusEvent[]>([]);
  const [projects, setProjects] = useState<Project[]>([]);

  const loadDetail = useCallback(async () => {
    try {
      const [sess, procs, servers, skillRows, pluginRows, modelRows, connRows, events, projectRows] =
        await Promise.all([
          getSessions(agentId),
          getProcessSnapshot(),
          getMcpServers(agentId),
          getSkills(agentId),
          getPlugins(agentId),
          getModels(agentId),
          getConnections(agentId),
          getEvents(50),
          getProjects(),
        ]);
      setSessions((sess ?? []).filter((s) => s.agent_id === agentId && s.status !== "stopped"));
      setProcesses(
        (procs ?? []).filter((p) => stem(p.exe) === agentId.toLowerCase()),
      );
      setMcp(servers ?? []);
      setSkills(skillRows ?? []);
      setPlugins(pluginRows ?? []);
      setModels(modelRows ?? []);
      setConnections(connRows ?? []);
      setLogs((events ?? []).filter((e) => e.agent_id === agentId));
      setProjects(
        (projectRows ?? []).filter((p) => (p.agent_ids ?? []).includes(agentId)),
      );
      setError(null);
    } catch (e) {
      setError(errMessage(e));
    }
  }, [agentId]);

  useEffect(() => {
    void loadDetail();
  }, [loadDetail]);

  const shownError = error ?? dashError;

  async function runStop() {
    try {
      await stopAgent(agentId);
      setConfirm(null);
      setActionError(null);
      await load();
    } catch (e) {
      setActionError(errMessage(e));
    }
  }

  async function runStart() {
    try {
      await startAgent(agentId);
      setActionError(null);
      await load();
    } catch (e) {
      setActionError(errMessage(e));
    }
  }

  async function runRestart() {
    try {
      await stopAgent(agentId);
      await startAgent(agentId);
      setConfirm(null);
      setActionError(null);
      await load();
    } catch (e) {
      setActionError(errMessage(e));
    }
  }

  async function runRefresh() {
    setRefreshing(true);
    try {
      await refreshAll();
      setActionError(null);
      await load();
      await loadDetail();
    } catch (e) {
      setActionError(errMessage(e));
    } finally {
      setRefreshing(false);
    }
  }

  async function openProject() {
    const path = projects.find((p) => p.path)?.path;
    if (!path) return;
    await openPath(path);
  }

  return (
    <section>
      <PageHeader
        title={agent?.name ?? "Agent"}
        subtitle={agent?.version ?? "Unknown"}
        actions={
          <>
            <Button variant="ghost" onClick={onBack}>
              Back
            </Button>
            <Button
              variant="secondary"
              onClick={() => void runRefresh()}
              disabled={refreshing}
            >
              {refreshing ? "Refreshing…" : "Refresh"}
            </Button>
            {agent?.executable_path && !agent.running && (
              <Button variant="secondary" onClick={() => void runStart()}>
                Start
              </Button>
            )}
            {agent?.running && (
              <Button variant="danger" onClick={() => setConfirm("stop")}>
                Stop
              </Button>
            )}
            {agent?.running && agent.executable_path && (
              <Button variant="secondary" onClick={() => setConfirm("restart")}>
                Restart
              </Button>
            )}
            {projects.length > 0 && (
              <Button variant="ghost" onClick={() => void openAgentTerminal(agentId).catch((e) => setActionError(errMessage(e)))}>
                Open Terminal
              </Button>
            )}
            {projects.length > 0 && (
              <Button variant="secondary" onClick={() => void openProject()}>
                Open Project
              </Button>
            )}
          </>
        }
      />
      {actionError !== null && <ErrorState message={actionError} />}
      {shownError !== null && (
        <ErrorState
          message={shownError}
          onRetry={() => {
            void load();
            void loadDetail();
          }}
        />
      )}
      {loading && agent === null && shownError === null && (
        <ContentPanel>
          <div className="state-hint">Loading…</div>
        </ContentPanel>
      )}
      <Dialog
        open={confirm !== null}
        title={confirm === "restart" ? "Restart agent?" : "Stop agent?"}
        onClose={() => setConfirm(null)}
      >
        <p>
          This only targets processes whose executable matches this agent. Processes with no
          executable path, and unrelated programs, are not touched.
        </p>
        <Button
          variant="danger"
          onClick={() => void (confirm === "restart" ? runRestart() : runStop())}
        >
          {confirm === "restart" ? "Restart" : "Stop"}
        </Button>
      </Dialog>
      {agent !== null && (
        <div className="detail-layout">
          <div className="detail-main">
            <Tabs tabs={TABS} active={tab} onChange={setTab} />
            <div className="detail-body">
              {tab === "overview" && (
                <>
                  <ContentPanel>
                    <div className="agent-stats">
                      <Metric label="Status" value={statusOf(agent)} />
                      <Metric label="Version" value={agent.version ?? "Unknown"} />
                      <Metric label="CPU" value={formatCpu(row?.cpu ?? 0)} />
                      <Metric label="RAM" value={formatBytes(row?.ram_bytes ?? 0)} />
                    </div>
                  </ContentPanel>
                  <h3 className="dash-heading">Connections</h3>
                  {CONNECTIONS_AGENTS.includes(agentId) ? (
                    <List
                      empty="No connections detected"
                      rows={connections.map((c) => `${c.name} · ${c.provider}`)}
                    />
                  ) : (
                    <ContentPanel>
                      <div className="state-hint">
                        Not available for this agent.
                      </div>
                    </ContentPanel>
                  )}
                </>
              )}
              {tab === "sessions" && (
                <List
                  empty="No sessions"
                  rows={sessions.map((s) => s.id)}
                />
              )}
              {tab === "processes" && (
                <TreeList
                  empty="No matching processes"
                  rows={processes}
                />
              )}
              {tab === "mcp" && (
                <List
                  empty="No MCP servers"
                  rows={mcp.map((s) => `${s.name} · ${s.status}`)}
                />
              )}
              {tab === "skills" && (
                <List
                  empty="No skills"
                  rows={skills.map((s) => s.name)}
                />
              )}
              {tab === "plugins" && (
                <List
                  empty="No plugins"
                  rows={plugins.map((p) => p.name)}
                />
              )}
              {tab === "models" && (
                <List
                  empty="No models detected"
                  rows={models.map((m) =>
                    m.provider ? `${m.name} · ${m.provider}` : m.name,
                  )}
                />
              )}
              {tab === "logs" && (
                <List
                  empty="No logs"
                  rows={logs.map((e) => e.message || e.event)}
                />
              )}
              {tab === "configuration" && (
                <ContentPanel>
                  <div className="agent-stats">
                    <Metric label="ID" value={agent.id} />
                    <Metric label="Type" value={agent.agent_type} />
                    <Metric label="Version" value={agent.version ?? "Unknown"} />
                    <Metric
                      label="Executable"
                      value={agent.executable_path ?? "Unknown"}
                    />
                    <Metric
                      label="Installed"
                      value={agent.installed ? "Yes" : "No"}
                    />
                    <Metric label="Running" value={agent.running ? "Yes" : "No"} />
                    <Metric
                      label="Last seen"
                      value={
                        agent.last_seen ? formatAgo(agent.last_seen) : "Unknown"
                      }
                    />
                  </div>
                </ContentPanel>
              )}
            </div>
          </div>
          <InspectorPanel title="Inspector">
            <div className="inspector-status">
              <StatusDot
                status={statusOf(agent)}
                label={agent.running ? "Running" : statusOf(agent)}
              />
            </div>
            <Metric label="Version" value={agent.version ?? "Unknown"} />
            <Metric label="CPU" value={formatCpu(row?.cpu ?? 0)} />
            <Metric label="RAM" value={formatBytes(row?.ram_bytes ?? 0)} />
            <Metric label="Processes" value={String(processes.length)} />
            <Metric label="Sessions" value={formatCount(row?.sessions)} />
            <Metric label="Subagents" value={formatCount(row?.subagents)} />
            <Metric label="MCP" value={formatCount(row?.mcp)} />
            <Metric label="Skills" value={formatCount(row?.skills)} />
          </InspectorPanel>
        </div>
      )}
    </section>
  );
}

function TreeList({ empty, rows }: { empty: string; rows: ProcessInfo[] }) {
  if (rows.length === 0) {
    return (
      <ContentPanel>
        <EmptyState title={empty} />
      </ContentPanel>
    );
  }
  const depths = treeDepths(rows);
  const sorted = [...rows].sort((a, b) => a.pid - b.pid);
  return (
    <ContentPanel>
      <ul className="activity-list">
        {sorted.map((p) => {
          const depth = depths.get(p.pid) ?? 0;
          return (
            <li key={p.pid} className="activity-item">
              <span style={{ paddingLeft: `${depth * 1.25}rem` }}>
                {depth > 0 ? "├─ " : ""}{p.name} · pid {p.pid} ·{" "}
                {formatCpu(p.cpu)} · {formatBytes(p.ram_bytes)}
              </span>
            </li>
          );
        })}
      </ul>
    </ContentPanel>
  );
}

function List({ empty, rows }: { empty: string; rows: string[] }) {
  if (rows.length === 0) {
    return (
      <ContentPanel>
        <EmptyState title={empty} />
      </ContentPanel>
    );
  }
  return (
    <ContentPanel>
      <ul className="activity-list">
        {rows.map((row) => (
          <li key={row} className="activity-item">
            <span>{row}</span>
          </li>
        ))}
      </ul>
    </ContentPanel>
  );
}
