import { useEffect, useState } from "react";
import { openPath } from "@tauri-apps/plugin-opener";
import PageHeader from "../layouts/PageHeader";
import ContentPanel from "../layouts/ContentPanel";
import Button from "../components/ui/Button";
import { EmptyState, ErrorState } from "../components/ui/States";
import { getProjects } from "../lib/api";
import { errMessage } from "../lib/format";
import type { Project } from "../types/session";

export default function ProjectsPage() {
  const [projects, setProjects] = useState<Project[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    let cancelled = false;
    getProjects()
      .then((rows) => {
        if (!cancelled) {
          setProjects(rows ?? []);
          setError(null);
        }
      })
      .catch((e) => {
        if (!cancelled) setError(errMessage(e));
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  return (
    <section>
      <PageHeader title="Projects" subtitle="Directories agents work in" />
      {error !== null && <ErrorState message={error} />}
      {loading && <div className="state-hint">Loading…</div>}
      {!loading && error === null && projects.length === 0 && (
        <ContentPanel>
          <EmptyState
            title="No projects detected yet"
            hint="Projects appear after a session refresh finds them."
          />
        </ContentPanel>
      )}
      {projects.length > 0 && (
        <ContentPanel>
          <ul className="activity-list">
            {projects.map((p) => (
              <li key={p.id} className="activity-item">
                <span>
                  {p.name} · {p.path} · {(p.agent_ids ?? []).join(", ") || "Unknown"}
                </span>
                <Button
                  variant="ghost"
                  onClick={() => void openPath(p.path).catch(() => {})}
                >
                  Open
                </Button>
              </li>
            ))}
          </ul>
        </ContentPanel>
      )}
    </section>
  );
}
