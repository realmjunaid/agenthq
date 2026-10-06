import { describe, expect, it } from "vitest";
import { treeDepths } from "./AgentDetailPage";
import type { ProcessInfo } from "../types/process";

function row(pid: number, parent_pid: number | null): ProcessInfo {
  return {
    pid,
    parent_pid,
    name: `p${pid}`,
    exe: null,
    cpu: 0,
    ram_bytes: 0,
    started_at: 0,
    cmdline: null,
  };
}

describe("treeDepths", () => {
  it("roots orphans at zero when the parent pid is absent", () => {
    const depths = treeDepths([row(2, 999), row(1, null)]);
    expect(depths.get(2)).toBe(0);
    expect(depths.get(1)).toBe(0);
  });
  it("nests children by chain length", () => {
    const depths = treeDepths([row(1, null), row(2, 1), row(3, 2)]);
    expect(depths.get(3)).toBe(2);
  });
  it("stays bounded on cycles", () => {
    const depths = treeDepths([row(1, 2), row(2, 1)]);
    expect((depths.get(1) ?? 99) <= 16).toBe(true);
    expect((depths.get(2) ?? 99) <= 16).toBe(true);
  });
});
