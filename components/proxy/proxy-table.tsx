"use client";

import { useMemo, useState } from "react";

import { Badge, Button, EmptyState, inputClass, StatusBadge } from "@/components/ui";
import { useT } from "@/i18n";
import { formatDuration } from "@/lib/format";
import { fill } from "@/lib/i18n-format";
import type { Anonymity, ProxyProtocol, Status, StoredProxy } from "@/lib/types";

const PAGE = 200;

type StatusFilter = "all" | "working" | "dead" | "unchecked";

export function filterProxies(
  proxies: StoredProxy[],
  { status, protocol, anonymity, query }: { status: StatusFilter; protocol: string; anonymity: string; query: string },
): StoredProxy[] {
  const q = query.trim().toLowerCase();
  const working = (s: Status | null) => s === "up" || s === "degraded";
  return proxies.filter(
    (p) =>
      (status === "all" ||
        (status === "working" && working(p.status)) ||
        (status === "dead" && p.status !== null && !working(p.status)) ||
        (status === "unchecked" && p.status === null)) &&
      (protocol === "all" || p.protocol === protocol) &&
      (anonymity === "all" || p.anonymity === anonymity) &&
      (!q || `${p.host}:${p.port} ${p.exitIp ?? ""} ${p.country ?? ""} ${p.username ?? ""}`.toLowerCase().includes(q)),
  );
}

export function ProxyTable({ proxies }: { proxies: StoredProxy[] }) {
  const t = useT();
  const [status, setStatus] = useState<StatusFilter>("all");
  const [protocol, setProtocol] = useState<ProxyProtocol | "all">("all");
  const [anonymity, setAnonymity] = useState<Anonymity | "all">("all");
  const [query, setQuery] = useState("");
  const [sortByLatency, setSortByLatency] = useState(false);
  const [limit, setLimit] = useState(PAGE);

  const rows = useMemo(() => {
    const filtered = filterProxies(proxies, { status, protocol, anonymity, query });
    if (sortByLatency) {
      filtered.sort((a, b) => (a.latencyMs ?? Infinity) - (b.latencyMs ?? Infinity));
    }
    return filtered;
  }, [proxies, status, protocol, anonymity, query, sortByLatency]);

  const select = "rounded-md border border-border bg-bg px-2 py-1.5 text-sm";

  return (
    <div>
      <div className="mb-3 flex flex-wrap items-center gap-2">
        <input
          className={`${inputClass} max-w-48`}
          placeholder={t.common.search}
          value={query}
          onChange={(e) => setQuery(e.target.value)}
        />
        <select className={select} value={status} onChange={(e) => setStatus(e.target.value as StatusFilter)}>
          <option value="all">{t.common.all}</option>
          <option value="working">{t.proxy.working}</option>
          <option value="dead">{t.status.down}</option>
          <option value="unchecked">{t.proxy.notChecked}</option>
        </select>
        <select className={select} value={protocol} onChange={(e) => setProtocol(e.target.value as ProxyProtocol | "all")}>
          <option value="all">{t.proxy.protocol}: {t.common.all}</option>
          {(["http", "https", "socks4", "socks5"] as const).map((p) => (
            <option key={p} value={p}>
              {p.toUpperCase()}
            </option>
          ))}
        </select>
        <select className={select} value={anonymity} onChange={(e) => setAnonymity(e.target.value as Anonymity | "all")}>
          <option value="all">{t.proxy.anonymity}: {t.common.all}</option>
          {(["elite", "anonymous", "transparent"] as const).map((a) => (
            <option key={a} value={a}>
              {t.proxy.anonymityLevels[a]}
            </option>
          ))}
        </select>
        <label className="flex items-center gap-1.5 text-sm">
          <input type="checkbox" checked={sortByLatency} onChange={(e) => setSortByLatency(e.target.checked)} />
          {t.web.latency} ↑
        </label>
      </div>

      {rows.length === 0 ? (
        <EmptyState>{t.web.noData}</EmptyState>
      ) : (
        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead className="text-left text-xs uppercase tracking-wide text-muted">
              <tr>
                <th className="px-3 py-2">{t.proxy.proxy}</th>
                <th className="px-3 py-2">{t.common.status}</th>
                <th className="px-3 py-2">{t.web.latency}</th>
                <th className="px-3 py-2">{t.proxy.anonymity}</th>
                <th className="px-3 py-2">{t.proxy.exitIp}</th>
                <th className="px-3 py-2">{t.proxy.country}</th>
                <th className="px-3 py-2">{t.web.message}</th>
              </tr>
            </thead>
            <tbody>
              {rows.slice(0, limit).map((p) => (
                <tr key={p.id} className="border-t border-border">
                  <td className="whitespace-nowrap px-3 py-1.5 font-mono text-xs">
                    <Badge className="mr-2 bg-fg/5 text-muted">{p.protocol}</Badge>
                    {p.username && <span className="text-muted">{p.username}@</span>}
                    {p.host}:{p.port}
                  </td>
                  <td className="px-3 py-1.5">
                    {p.status ? <StatusBadge status={p.status} /> : <span className="whitespace-nowrap text-xs text-muted">{t.proxy.notChecked}</span>}
                  </td>
                  <td className="px-3 py-1.5 tabular-nums">{formatDuration(p.latencyMs)}</td>
                  <td className="px-3 py-1.5">{p.anonymity ? t.proxy.anonymityLevels[p.anonymity] : "-"}</td>
                  <td className="whitespace-nowrap px-3 py-1.5 font-mono text-xs">{p.exitIp ?? "-"}</td>
                  <td className="px-3 py-1.5">{p.country ?? "-"}</td>
                  <td className="max-w-56 truncate px-3 py-1.5 text-xs text-muted" title={p.message ?? ""}>
                    {p.message}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
          <div className="mt-3 flex items-center justify-between text-xs text-muted">
            <span>{fill(t.proxy.showing, { shown: Math.min(limit, rows.length), total: rows.length })}</span>
            {rows.length > limit && (
              <Button size="sm" onClick={() => setLimit((l) => l + PAGE)}>
                {t.proxy.showMore}
              </Button>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
