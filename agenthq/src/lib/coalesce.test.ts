import { afterEach, describe, expect, it, vi } from "vitest";
import { createCoalescer } from "./coalesce";

describe("createCoalescer", () => {
  afterEach(() => {
    vi.useRealTimers();
  });

  it("collapses a burst into one trailing call", () => {
    vi.useFakeTimers();
    const fn = vi.fn();
    const fire = createCoalescer(1000, fn);
    fire();
    fire();
    fire();
    vi.advanceTimersByTime(999);
    expect(fn).not.toHaveBeenCalled();
    vi.advanceTimersByTime(1);
    expect(fn).toHaveBeenCalledTimes(1);
  });

  it("fires again after a quiet period", () => {
    vi.useFakeTimers();
    const fn = vi.fn();
    const fire = createCoalescer(1000, fn);
    fire();
    vi.advanceTimersByTime(1000);
    expect(fn).toHaveBeenCalledTimes(1);
    fire();
    vi.advanceTimersByTime(1000);
    expect(fn).toHaveBeenCalledTimes(2);
  });
});
