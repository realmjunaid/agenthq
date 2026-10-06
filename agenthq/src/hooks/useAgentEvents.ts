import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { FRONTEND_EVENT, type BusEvent } from "../types/events";

/**
 * Subscribe once to live agent events. `onEvent` must be stable
 * (useCallback) so the listener is not re-bound every render.
 * Cleanup unregisters even if listen resolves after unmount (StrictMode).
 */
export function useAgentEvents(onEvent: () => void) {
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let cancelled = false;
    listen<BusEvent>(FRONTEND_EVENT, () => {
      onEvent();
    })
      .then((stop) => {
        if (cancelled) stop();
        else unlisten = stop;
      })
      .catch(() => {
        // Live updates unavailable; Refresh still works.
      });
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [onEvent]);
}
