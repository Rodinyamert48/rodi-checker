import { describe, expect, it } from "vitest";

import { toNewMonitor } from "./monitor-form";

const base = {
  kind: "http" as const,
  name: " Site ",
  intervalSec: "30",
  enabled: true,
  tags: "prod, , web",
  url: " https://example.com ",
  method: "POST",
  headers: "Authorization: Bearer a:b\nX-Empty:\nbroken",
  body: "{}",
  expectedStatus: "200, 201 abc",
  keyword: "",
  keywordAbsent: false,
  followRedirects: true,
  degradedLatencyMs: "",
  host: "",
  port: "",
  warnDays: "14",
  domain: "",
  recordType: "A",
  expected: "",
};

describe("toNewMonitor", () => {
  it("builds an http monitor", () => {
    const m = toNewMonitor(base);
    expect(m.name).toBe("Site");
    expect(m.intervalSec).toBe(30);
    expect(m.tags).toEqual(["prod", "web"]);
    expect(m.config).toMatchObject({
      url: "https://example.com",
      method: "POST",
      headers: { Authorization: "Bearer a:b", "X-Empty": "" },
      body: "{}",
      expectedStatus: [200, 201],
      keyword: null,
      degradedLatencyMs: null,
    });
  });

  it("builds tcp and dns monitors", () => {
    expect(toNewMonitor({ ...base, kind: "tcp", host: " db ", port: "5432" }).config).toEqual({ host: "db", port: 5432 });
    expect(toNewMonitor({ ...base, kind: "dns", domain: "example.com", recordType: "MX" }).config).toEqual({
      domain: "example.com",
      recordType: "MX",
      expected: null,
    });
  });
});
