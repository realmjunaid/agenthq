import type { Page } from "./AppShell";

const PAGES: { id: Page; label: string }[] = [
  { id: "dashboard", label: "Dashboard" },
  { id: "agents", label: "Agents" },
  { id: "monitoring", label: "Monitoring" },
  { id: "logs", label: "Logs" },
  { id: "projects", label: "Projects" },
  { id: "settings", label: "Settings" },
];

export default function Sidebar({
  page,
  onNavigate,
}: {
  page: Page;
  onNavigate: (page: Page) => void;
}) {
  return (
    <aside className="sidebar">
      <div className="brand">AgentHQ</div>
      <nav>
        {PAGES.map((p) => (
          <button
            key={p.id}
            className={`nav-btn${page === p.id || (page === "agent" && p.id === "agents") ? " active" : ""}`}
            aria-current={page === p.id || (page === "agent" && p.id === "agents") ? "page" : undefined}
            onClick={() => onNavigate(p.id)}
          >
            {p.label}
          </button>
        ))}
      </nav>
    </aside>
  );
}
