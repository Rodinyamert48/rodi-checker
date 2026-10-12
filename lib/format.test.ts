import { describe, expect, it } from "vitest";

import { formatBytes, formatDuration } from "./format";

describe("format", () => {
  it("formats durations", () => {
    expect(formatDuration(null)).toBe("-");
    expect(formatDuration(120.4)).toBe("120 ms");
    expect(formatDuration(1500)).toBe("1.50 s");
  });

  it("formats bytes", () => {
    expect(formatBytes(512, "en")).toBe("512 B");
    expect(formatBytes(1536, "en")).toBe("1.5 KB");
    expect(formatBytes(5 * 1024 ** 3, "en")).toBe("5.0 GB");
  });
});
