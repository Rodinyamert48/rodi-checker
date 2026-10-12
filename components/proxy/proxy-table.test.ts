import { describe, expect, it } from "vitest";

import type { StoredProxy } from "@/lib/types";

import { filterProxies } from "./proxy-table";

const proxy = (id: number, overrides: Partial<StoredProxy>): StoredProxy => ({
  id,
  listId: 1,
  protocol: "http",
  host: `10.0.0.${id}`,
  port: 8080,
  username: null,
  hasPassword: false,
  status: null,
  latencyMs: null,
  anonymity: null,
  exitIp: null,
  country: null,
  message: null,
  checkedAt: null,
  ...overrides,
});

const all = { status: "all" as const, protocol: "all", anonymity: "all", query: "" };

describe("filterProxies", () => {
  const list = [
    proxy(1, { status: "up", anonymity: "elite", country: "TR" }),
    proxy(2, { status: "down", protocol: "socks5" }),
    proxy(3, {}),
    proxy(4, { status: "degraded", anonymity: "transparent" }),
  ];

  it("filters by status", () => {
    expect(filterProxies(list, { ...all, status: "working" }).map((p) => p.id)).toEqual([1, 4]);
    expect(filterProxies(list, { ...all, status: "dead" }).map((p) => p.id)).toEqual([2]);
    expect(filterProxies(list, { ...all, status: "unchecked" }).map((p) => p.id)).toEqual([3]);
  });

  it("filters by protocol, anonymity and text", () => {
    expect(filterProxies(list, { ...all, protocol: "socks5" }).map((p) => p.id)).toEqual([2]);
    expect(filterProxies(list, { ...all, anonymity: "elite" }).map((p) => p.id)).toEqual([1]);
    expect(filterProxies(list, { ...all, query: "tr" }).map((p) => p.id)).toEqual([1]);
    expect(filterProxies(list, { ...all, query: "10.0.0.3" }).map((p) => p.id)).toEqual([3]);
  });
});
