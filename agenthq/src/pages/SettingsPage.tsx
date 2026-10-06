import { useEffect, useState } from "react";
import PageHeader from "../layouts/PageHeader";
import ContentPanel from "../layouts/ContentPanel";
import { ErrorState } from "../components/ui/States";
import { getSettings, setSetting } from "../lib/api";
import { errMessage } from "../lib/format";

const TOGGLES: { key: string; label: string; defaultOn: boolean }[] = [
  { key: "tray.start_with_windows", label: "Start with Windows", defaultOn: false },
  { key: "tray.launch_minimized", label: "Launch minimized", defaultOn: false },
  { key: "tray.close_to_tray", label: "Close to tray", defaultOn: true },
  { key: "monitoring.paused", label: "Pause monitoring", defaultOn: false },
  { key: "notify.agent_started", label: "Notify when an agent starts", defaultOn: true },
  { key: "notify.agent_stopped", label: "Notify when an agent stops", defaultOn: true },
  { key: "notify.agent_error", label: "Notify on agent errors", defaultOn: true },
  { key: "notify.mcp_disconnected", label: "Notify when an MCP server is removed", defaultOn: true },
  { key: "notify.high_cpu", label: "Notify on high CPU", defaultOn: true },
  { key: "notify.high_ram", label: "Notify on high RAM", defaultOn: true },
];

export default function SettingsPage() {
  const [values, setValues] = useState<Record<string, string>>({});
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    getSettings()
      .then((rows) => {
        const next: Record<string, string> = {};
        for (const [key, value] of rows ?? []) next[key] = value;
        setValues(next);
      })
      .catch((e) => setError(errMessage(e)));
  }, []);

  function on(key: string, defaultOn: boolean): boolean {
    const raw = values[key];
    if (raw === undefined) return defaultOn;
    return raw === "1";
  }

  async function toggle(key: string, defaultOn: boolean) {
    const next = on(key, defaultOn) ? "0" : "1";
    setValues((prev) => ({ ...prev, [key]: next }));
    try {
      await setSetting(key, next);
      setError(null);
    } catch (e) {
      setError(errMessage(e));
    }
  }

  return (
    <section>
      <PageHeader title="Settings" subtitle="Tray, monitoring, and notifications" />
      {error !== null && <ErrorState message={error} />}
      <ContentPanel>
        <ul className="settings-list">
          {TOGGLES.map((item) => (
            <li key={item.key}>
              <label>
                <input
                  type="checkbox"
                  checked={on(item.key, item.defaultOn)}
                  onChange={() => void toggle(item.key, item.defaultOn)}
                />
                {item.label}
              </label>
            </li>
          ))}
        </ul>
        <p className="state-hint">
          High CPU defaults to 80% and high RAM to 85% of system memory. Alerts fire once when the
          line is crossed, not on every refresh.
        </p>
      </ContentPanel>
    </section>
  );
}
