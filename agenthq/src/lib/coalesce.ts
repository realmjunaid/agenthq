/**
 * Trailing-edge coalescer: a burst of calls becomes one call,
 * `delayMs` after the last one. Event storms (refresh fan-out)
 * must not trigger one backend load per event.
 */
export function createCoalescer(delayMs: number, fn: () => void): () => void {
  let timer: ReturnType<typeof setTimeout> | undefined;
  return () => {
    if (timer !== undefined) clearTimeout(timer);
    timer = setTimeout(() => {
      timer = undefined;
      fn();
    }, delayMs);
  };
}
