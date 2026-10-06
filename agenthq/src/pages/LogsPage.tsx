import { useEffect, useMemo, useState } from "react";
import PageHeader from "../layouts/PageHeader";
import ContentPanel from "../layouts/ContentPanel";
import { Search } from "../components/ui/Input";
import { ErrorState } from "../components/ui/States";
import { getEvents } from "../lib/api";
import { errMessage, formatAgo } from "../lib/format";
import type { BusEvent } from "../types/events";

const FILTERS = ["all", "info", "warning", "error"] as const;

export default function LogsPage({ focusSearch }: { focusSearch: boolean }) {
  const [rows, setRows] = useState<BusEvent[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [filter, setFilter] = useState<(typeof FILTERS)[number]>("all");
  const [query, setQuery] = useState("");
  useEffect(() => {
    let cancelled = false;
    getEvents(200)
      .then((events) => {
        if (!cancelled) setRows(Array.isArray(events) ? events : []);
      })
      .catch((e) => {
        if (!cancelled) setError(errMessage(e));
      });
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    if (focusSearch) document.getElementById("log-search")?.focus();
  }, [focusSearch]);

  const shown = useMemo(() => {
    const q = query.trim().toLowerCase();
    return rows.filter((row) => {
      const level = row.level.toLowerCase();
      if (filter === "warning" && level !== "warn" && level !== "warning") return false;
      if (filter === "error" && level !== "error") return false;
      if (filter === "info" && level !== "info") return false;
      if (!q) return true;
      return `${row.agent_id ?? ""} ${row.event} ${row.message}`.toLowerCase().includes(q);
    });
  }, [rows, filter, query]);

  return (
    <section>
      <PageHeader title="Logs" subtitle="Local events. Ctrl+K focuses search." />
      {error !== null && <ErrorState message={error} />}
      <div className="log-tools">
        <Search
          id="log-search"
          placeholder="Search events"
          value={query}
          onChange={(e) => setQuery(e.currentTarget.value)}
          aria-label="Search logs"
        />
        <div className="log-filters" role="group" aria-label="Level filter">
          {FILTERS.map((id) => (
            <button
              key={id}
              type="button"
              className={`nav-btn${filter === id ? " active" : ""}`}
              onClick={() => setFilter(id)}
            >
              {id === "all" ? "All" : id[0].toUpperCase() + id.slice(1)}
            </button>
          ))}
        </div>
      </div>
      <ContentPanel>
        <table className="data-table">
          <thead>
            <tr>
              <th>Time</th>
              <th>Level</th>
              <th>Agent</th>
              <th>Event</th>
              <th>Message</th>
            </tr>
          </thead>
          <tbody>
            {shown.map((row) => (
              <tr key={row.id}>
                <td>{formatAgo(row.ts)}</td>
                <td>{row.level}</td>
                <td>{row.agent_id ?? "—"}</td>
                <td>{row.event}</td>
                <td>{row.message}</td>
              </tr>
            ))}
            {shown.length === 0 && (
              <tr>
                <td colSpan={5}>No matching events</td>
              </tr>
            )}
          </tbody>
        </table>
      </ContentPanel>
    </section>
  );
}
