import { describe, expect, it } from "vitest";
import { errMessage, formatAgo, formatBytes, formatCount, formatCpu } from "./format";

describe("formatBytes", () => {
  it("renders zero bytes", () => {
    expect(formatBytes(0)).toBe("0 B");
  });
  it("renders kilobytes with one decimal under 10", () => {
    expect(formatBytes(1536)).toBe("1.5 KB");
  });
  it("refuses non-finite and negative input", () => {
    expect(formatBytes(NaN)).toBe("Unknown");
    expect(formatBytes(-1)).toBe("Unknown");
  });
});

describe("formatCpu", () => {
  it("renders one decimal", () => {
    expect(formatCpu(4.26)).toBe("4.3%");
  });
  it("refuses non-finite input", () => {
    expect(formatCpu(NaN)).toBe("Unknown");
  });
});

describe("formatCount", () => {
  it("marks missing capability as not available", () => {
    expect(formatCount(null)).toBe("Not available");
    expect(formatCount(undefined)).toBe("Not available");
  });
  it("renders zero as a real zero", () => {
    expect(formatCount(0)).toBe("0");
  });
});

describe("formatAgo", () => {
  it("renders seconds ago", () => {
    const ts = Math.floor(Date.now() / 1000) - 30;
    expect(formatAgo(ts)).toBe("30s ago");
  });
  it("refuses non-positive input", () => {
    expect(formatAgo(0)).toBe("Unknown");
  });
});

describe("errMessage", () => {
  it("unwraps errors and strings", () => {
    expect(errMessage(new Error("boom"))).toBe("boom");
    expect(errMessage("plain")).toBe("plain");
  });
  it("falls back for unknown shapes", () => {
    expect(errMessage({})).toBe("Unknown error");
  });
});
