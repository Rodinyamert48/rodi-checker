"use client";

import { useQuery } from "@tanstack/react-query";
import { useState } from "react";
import { CartesianGrid, Line, LineChart, ResponsiveContainer, Tooltip, XAxis, YAxis } from "recharts";

import { Card, EmptyState, Stat, StatusBadge } from "@/components/ui";
import { useT } from "@/i18n";
import { api } from "@/lib/api";
import { cn } from "@/lib/cn";
import { formatDateTime, formatDuration, formatNumber, formatTime } from "@/lib/format";
import { useSettings } from "@/lib/settings";
import type { Monitor } from "@/lib/types";

const RANGES = ["24", "168", "720"] as const;

export function MonitorDetail({ monitor }: { monitor: Monitor }) {
  const t = useT();
  const { settings } = useSettings();
  const lang = settings.language;
  const [range, setRange] = useState<(typeof RANGES)[number]>("24");
  const hours = Number(range);

  const results = useQuery({
    queryKey: ["monitors", monitor.id, "results", hours],
    queryFn: () => api.monitorResults(monitor.id, hours),
  });
  const stats = useQuery({
    queryKey: ["monitors", monitor.id, "stats", hours],
    queryFn: () => api.monitorStats(monitor.id, hours),
  });
  const incidents = useQuery({
    queryKey: ["monitors", monitor.id, "incidents"],
    queryFn: () => api.listIncidents(monitor.id, 20),
  });

  const points = (results.data ?? []).map((r) => ({
    time: r.checkedAt,
    latency: r.latencyMs,
    status: r.status,
  }));
  const recent = [...(results.data ?? [])].reverse().slice(0, 15);

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h2 className="text-lg font-semibold">{monitor.name}</h2>
        <div className="flex rounded-md border border-border p-0.5 text-xs">
          {RANGES.map((r) => (
            <button
              key={r}
              type="button"
              onClick={() => setRange(r)}
              className={cn("rounded px-2.5 py-1", r === range ? "bg-accent text-white" : "hover:bg-fg/5")}
            >
              {t.web.range[r]}
            </button>
          ))}
        </div>
      </div>

      <div className="grid gap-3 sm:grid-cols-3">
        <Stat
          label={t.web.uptime}
          value={stats.data?.uptime == null ? "-" : `%${formatNumber(stats.data.uptime, lang, 2)}`}
          hint={stats.data ? `${stats.data.total} ${t.web.checks}` : undefined}
        />
        <Stat label={t.web.avgLatency} value={formatDuration(stats.data?.avgLatencyMs)} />
        <Stat
          label={t.web.incidents}
          value={(incidents.data ?? []).filter((i) => !i.resolvedAt).length > 0 ? t.web.ongoing : (incidents.data?.length ?? 0)}
        />
      </div>

      <Card>
        <h3 className="mb-3 text-sm font-medium">{t.web.responseTime}</h3>
        {points.length === 0 ? (
          <EmptyState>{t.web.noData}</EmptyState>
        ) : (
          <div className="h-56">
            <ResponsiveContainer width="100%" height="100%">
              <LineChart data={points} margin={{ top: 4, right: 8, bottom: 0, left: 0 }}>
                <CartesianGrid strokeDasharray="3 3" stroke="var(--color-border)" vertical={false} />
                <XAxis
                  dataKey="time"
                  tickFormatter={(v) => formatTime(v, lang)}
                  stroke="var(--color-muted)"
                  fontSize={11}
                  minTickGap={40}
                />
                <YAxis stroke="var(--color-muted)" fontSize={11} unit=" ms" width={64} />
                <Tooltip
                  contentStyle={{ background: "var(--color-surface)", border: "1px solid var(--color-border)" }}
                  labelFormatter={(v) => formatDateTime(String(v), lang)}
                  formatter={(v) => [formatDuration(Number(v)), t.web.latency]}
                />
                <Line type="monotone" dataKey="latency" stroke="var(--color-accent)" dot={false} strokeWidth={2} connectNulls />
              </LineChart>
            </ResponsiveContainer>
          </div>
        )}
      </Card>

      <div className="grid gap-4 xl:grid-cols-2">
        <Card>
          <h3 className="mb-3 text-sm font-medium">{t.web.recentResults}</h3>
          {recent.length === 0 ? (
            <EmptyState>{t.web.noData}</EmptyState>
          ) : (
            <table className="w-full text-sm">
              <tbody>
                {recent.map((r) => (
                  <tr key={r.id} className="border-t border-border first:border-0">
                    <td className="py-1.5 pr-2 text-xs text-muted">{formatDateTime(r.checkedAt, lang)}</td>
                    <td className="py-1.5 pr-2">
                      <StatusBadge status={r.status} />
                    </td>
                    <td className="py-1.5 pr-2 tabular-nums">{formatDuration(r.latencyMs)}</td>
                    <td className="max-w-48 truncate py-1.5 text-xs text-muted" title={r.message ?? ""}>
                      {r.message}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </Card>
        <Card>
          <h3 className="mb-3 text-sm font-medium">{t.web.incidents}</h3>
          {(incidents.data ?? []).length === 0 ? (
            <EmptyState>{t.web.noIncidents}</EmptyState>
          ) : (
            <ul className="space-y-2 text-sm">
              {incidents.data!.map((i) => (
                <li key={i.id} className="rounded-md border border-border p-2">
                  <div className="flex justify-between">
                    <span>{formatDateTime(i.startedAt, lang)}</span>
                    <span className={i.resolvedAt ? "text-up" : "text-down"}>
                      {i.resolvedAt ? `${t.web.resolvedAt}: ${formatDateTime(i.resolvedAt, lang)}` : t.web.ongoing}
                    </span>
                  </div>
                  {i.cause && <p className="mt-1 text-xs text-muted">{i.cause}</p>}
                </li>
              ))}
            </ul>
          )}
        </Card>
      </div>
    </div>
  );
}
