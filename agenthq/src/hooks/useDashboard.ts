import { useCallback, useEffect, useRef, useState } from "react";
import { getDashboard, getEvents, refreshAll } from "../lib/api";
import { errMessage } from "../lib/format";
import type { DashboardData } from "../types/dashboard";
import type { BusEvent } from "../types/events";
import { useAgentEvents } from "./useAgentEvents";

export function useDashboard() {
  const [data, setData] = useState<DashboardData | null>(null);
  const [events, setEvents] = useState<BusEvent[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [refreshing, setRefreshing] = useState(false);

  const request = useRef(0);

  const load = useCallback(async () => {
    const id = ++request.current;
    try {
      const [dash, ev] = await Promise.all([getDashboard(), getEvents(20)]);
      if (id !== request.current) return;
      setData(
        dash ?? {
          agents: [],
          total_ram_bytes: 0,
          total_cpu: 0,
          running: 0,
          system: { total_cpu: 0, used_mem_bytes: 0, total_mem_bytes: 0 },
        },
      );
      setEvents(Array.isArray(ev) ? ev : []);
      setError(null);
    } catch (e) {
      if (id !== request.current) return;
      setError(errMessage(e));
    } finally {
      if (id === request.current) setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  useAgentEvents(load);

  const refresh = useCallback(async () => {
    setRefreshing(true);
    try {
      await refreshAll();
      await load();
    } catch (e) {
      setError(errMessage(e));
    } finally {
      setRefreshing(false);
    }
  }, [load]);

  return { data, events, error, loading, refreshing, load, refresh };
}
