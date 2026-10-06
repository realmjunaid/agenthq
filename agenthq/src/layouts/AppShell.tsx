import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import Sidebar from "./Sidebar";
import Topbar from "./Topbar";
import DashboardPage from "../pages/DashboardPage";
import AgentsPage from "../pages/AgentsPage";
import AgentDetailPage from "../pages/AgentDetailPage";
import MonitoringPage from "../pages/MonitoringPage";
import LogsPage from "../pages/LogsPage";
import ProjectsPage from "../pages/ProjectsPage";
import SettingsPage from "../pages/SettingsPage";

export type Page = "dashboard" | "agents" | "agent" | "monitoring" | "logs" | "projects" | "settings";

export default function AppShell() {
  const [page, setPage] = useState<Page>("dashboard");
  const [agentId, setAgentId] = useState<string | null>(null);
  const [focusLogs, setFocusLogs] = useState(false);
  const [theme, setTheme] = useState<"light" | "dark">("light");

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let cancelled = false;
    listen<string>("agenthq://navigate", (event) => {
      if (event.payload === "settings") setPage("settings");
      if (event.payload === "dashboard") setPage("dashboard");
      if (event.payload === "logs") setPage("logs");
    })
      .then((stop) => {
        if (cancelled) stop();
        else unlisten = stop;
      })
      .catch(() => {});
    function onKey(e: KeyboardEvent) {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "k") {
        if (document.querySelector(".dialog")) return;
        e.preventDefault();
        setPage("logs");
        setFocusLogs(true);
      }
    }
    window.addEventListener("keydown", onKey);
    return () => {
      cancelled = true;
      unlisten?.();
      window.removeEventListener("keydown", onKey);
    };
  }, []);

  function toggleTheme() {
    const next = theme === "light" ? "dark" : "light";
    setTheme(next);
    document.documentElement.dataset.theme = next;
  }

  return (
    <div className="app-shell">
      <Sidebar page={page} onNavigate={setPage} />
      <div className="main-col">
        <Topbar theme={theme} onToggleTheme={toggleTheme} />
        <main>
          {page === "dashboard" && (
            <DashboardPage onOpenAgent={(id) => { setAgentId(id); setPage("agent"); }} />
          )}
          {page === "agents" && (
            <AgentsPage onOpenAgent={(id) => { setAgentId(id); setPage("agent"); }} />
          )}
          {page === "agent" && agentId !== null && (
            <AgentDetailPage agentId={agentId} onBack={() => setPage("agents")} />
          )}
          {page === "monitoring" && <MonitoringPage />}
          {page === "logs" && <LogsPage focusSearch={focusLogs} />}
          {page === "projects" && <ProjectsPage />}
          {page === "settings" && <SettingsPage />}
        </main>
      </div>
    </div>
  );
}
