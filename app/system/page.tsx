"use client";

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { BellPlus, X } from "lucide-react";
import { useMemo, useState } from "react";

import { AlertsCard } from "@/components/system/alerts-card";
import { Meter } from "@/components/system/meter";
import { UsageChart, type UsagePoint } from "@/components/system/usage-chart";
import { Badge, Button, Card, EmptyState, ErrorText, inputClass, PageHeader, Stat } from "@/components/ui";
import { useT } from "@/i18n";
import { api } from "@/lib/api";
import { cn } from "@/lib/cn";
import { useTauriEvent } from "@/lib/events";
import { formatBytes, formatNumber } from "@/lib/format";
import { fill } from "@/lib/i18n-format";
import { useSettings } from "@/lib/settings";
import type { ServiceState, SystemSnapshot } from "@/lib/types";

const LIVE_POINTS = 120; // 10 minutes at 5 s

type Range = "live" | "24" | "168";

const toPoint = (s: SystemSnapshot): UsagePoint => ({
  ts: s.ts,
  cpu: s.cpu,
  memory: s.memTotal ? (s.memUsed / s.memTotal) * 100 : 0,
});

const serviceTone: Record<ServiceState, string> = {
  running: "bg-up/10 text-up",
  stopped: "bg-fg/10 text-muted",
  starting: "bg-degraded/10 text-degraded",
  stopping: "bg-degraded/10 text-degraded",
  paused: "bg-degraded/10 text-degraded",
  unknown: "bg-fg/10 text-muted",
};

export default function SystemPage() {
  const t = useT();
  const { settings } = useSettings();
  const lang = settings.language;
  const client = useQueryClient();
  const [live, setLive] = useState<UsagePoint[]>([]);
  const [range, setRange] = useState<Range>("live");
  const [sort, setSort] = useState<"cpu" | "memory">("cpu");
  const [serviceQuery, setServiceQuery] = useState("");

  const initial = useQuery({ queryKey: ["system-snapshot"], queryFn: api.systemSnapshot });
  const [snapshot, setSnapshot] = useState<SystemSnapshot | null>(null);
  const current = snapshot ?? initial.data ?? null;
  const first = initial.data;
  const points = live.length === 0 && first ? [toPoint(first)] : live;

  useTauriEvent<SystemSnapshot>("system-snapshot", (s) => {
    setSnapshot(s);
    setLive((current) => {
      const seeded = current.length === 0 && first ? [toPoint(first)] : current;
      return [...seeded, toPoint(s)].slice(-LIVE_POINTS);
    });
  });

  const history = useQuery({
    queryKey: ["system-history", range],
    queryFn: () => api.systemHistory(Number(range)),
    enabled: range !== "live",
  });
  const processes = useQuery({
    queryKey: ["processes", sort],
    queryFn: () => api.systemProcesses(sort, 30),
    refetchInterval: 5000,
  });
  const ports = useQuery({ queryKey: ["ports"], queryFn: api.systemPorts, refetchInterval: 15000 });
  const services = useQuery({ queryKey: ["services"], queryFn: api.systemServices, refetchInterval: 30000 });

  const kill = useMutation({
    mutationFn: api.systemKill,
    onSettled: () => client.invalidateQueries({ queryKey: ["processes"] }),
  });
  const watchService = useMutation({
    mutationFn: (name: string) => api.createAlertRule({ metric: "service", threshold: 0, target: name, enabled: true }),
    onSuccess: () => client.invalidateQueries({ queryKey: ["alert-rules"] }),
  });

  const chartPoints: UsagePoint[] =
    range === "live"
      ? points
      : (history.data ?? []).map((s) => ({ ts: s.ts, cpu: s.cpu, memory: s.memTotal ? (s.memUsed / s.memTotal) * 100 : 0 }));

  const filteredServices = useMemo(() => {
    const q = serviceQuery.trim().toLowerCase();
    return (services.data ?? []).filter((s) => !q || `${s.name} ${s.displayName}`.toLowerCase().includes(q));
  }, [services.data, serviceQuery]);

  const uptime = (secs: number) => {
    const d = Math.floor(secs / 86400);
    const h = Math.floor((secs % 86400) / 3600);
    const m = Math.floor((secs % 3600) / 60);
    return d > 0 ? fill(t.system.days, { d, h }) : fill(t.system.hours, { h, m });
  };

  const memPercent = current && current.memTotal ? (current.memUsed / current.memTotal) * 100 : 0;
  const mainDisk = current?.disks[0];

  return (
    <>
      <PageHeader
        title={t.nav.system}
        subtitle={current ? [current.hostname, current.os].filter(Boolean).join(" · ") : t.modules.system.description}
      />

      {!current ? (
        <EmptyState>{initial.error ? t.common.desktopOnly : t.common.loading}</EmptyState>
      ) : (
        <div className="space-y-4">
          <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
            <Stat label={t.system.cpu} value={`%${formatNumber(current.cpu, lang, 0)}`} hint={current.cpuName} />
            <Stat
              label={t.system.memory}
              value={`%${formatNumber(memPercent, lang, 0)}`}
              hint={`${formatBytes(current.memUsed, lang)} / ${formatBytes(current.memTotal, lang)}`}
            />
            <Stat
              label={t.system.network}
              value={
                <span className="text-base">
                  ↓ {formatBytes(current.net.rxPerSec, lang)}/s · ↑ {formatBytes(current.net.txPerSec, lang)}/s
                </span>
              }
            />
            <Stat
              label={t.system.uptime}
              value={uptime(current.uptimeSecs)}
              hint={fill(t.system.processCount, { count: current.processCount })}
            />
          </div>

          <Card>
            <div className="mb-3 flex items-center justify-between">
              <h2 className="font-medium">{t.system.usage}</h2>
              <div className="flex rounded-md border border-border p-0.5 text-xs">
                {(["live", "24", "168"] as const).map((r) => (
                  <button
                    key={r}
                    type="button"
                    onClick={() => setRange(r)}
                    className={cn("rounded px-2.5 py-1", r === range ? "bg-accent text-white" : "hover:bg-fg/5")}
                  >
                    {r === "live" ? t.system.live : t.system.history[r]}
                  </button>
                ))}
              </div>
            </div>
            {chartPoints.length < 2 ? (
              <EmptyState>{t.common.loading}</EmptyState>
            ) : (
              <UsageChart points={chartPoints} language={lang} labels={{ cpu: t.system.cpu, memory: t.system.memory }} />
            )}
          </Card>

          <div className="grid gap-4 xl:grid-cols-2">
            <Card>
              <h2 className="mb-3 font-medium">{t.system.disks}</h2>
              <ul className="space-y-3">
                {current.disks.map((d) => {
                  const used = d.total ? ((d.total - d.available) / d.total) * 100 : 0;
                  return (
                    <li key={`${d.name}-${d.mountPoint}`}>
                      <div className="mb-1 flex justify-between text-sm">
                        <span className="font-medium">
                          {d.mountPoint} <span className="text-xs text-muted">{d.fileSystem}</span>
                        </span>
                        <span className="text-xs text-muted">
                          {formatBytes(d.available, lang)} {t.system.free} / {formatBytes(d.total, lang)}
                        </span>
                      </div>
                      <Meter percent={used} />
                    </li>
                  );
                })}
              </ul>
              {mainDisk === undefined && <EmptyState>-</EmptyState>}
            </Card>

            <Card>
              <h2 className="mb-3 font-medium">
                {t.system.cores} ({current.cpuPerCore.length})
              </h2>
              <div className="grid grid-cols-2 gap-x-4 gap-y-2 sm:grid-cols-4">
                {current.cpuPerCore.map((value, i) => (
                  <div key={i}>
                    <div className="flex justify-between text-xs text-muted">
                      <span>#{i}</span>
                      <span>{value.toFixed(0)}%</span>
                    </div>
                    <Meter percent={value} />
                  </div>
                ))}
              </div>
              {current.temperatures.length > 0 && (
                <>
                  <h3 className="mb-2 mt-4 text-sm font-medium">{t.system.temperatures}</h3>
                  <ul className="grid grid-cols-2 gap-1 text-sm">
                    {current.temperatures.map((temp) => (
                      <li key={temp.label} className="flex justify-between">
                        <span className="truncate text-muted">{temp.label}</span>
                        <span className={temp.celsius > 85 ? "text-down" : ""}>{temp.celsius.toFixed(0)}°C</span>
                      </li>
                    ))}
                  </ul>
                </>
              )}
            </Card>
          </div>

          <AlertsCard />

          <Card className="overflow-x-auto">
            <div className="mb-3 flex items-center justify-between">
              <h2 className="font-medium">{t.system.processes}</h2>
              <div className="flex rounded-md border border-border p-0.5 text-xs">
                {(["cpu", "memory"] as const).map((s) => (
                  <button
                    key={s}
                    type="button"
                    onClick={() => setSort(s)}
                    className={cn("rounded px-2.5 py-1", s === sort ? "bg-accent text-white" : "hover:bg-fg/5")}
                  >
                    {s === "cpu" ? t.system.sortCpu : t.system.sortMemory}
                  </button>
                ))}
              </div>
            </div>
            <table className="w-full text-sm">
              <thead className="text-left text-xs uppercase tracking-wide text-muted">
                <tr>
                  <th className="py-2 pr-3">{t.system.pid}</th>
                  <th className="py-2 pr-3">{t.system.name}</th>
                  <th className="py-2 pr-3">CPU</th>
                  <th className="py-2 pr-3">{t.system.memory}</th>
                  <th />
                </tr>
              </thead>
              <tbody>
                {(processes.data ?? []).map((p) => (
                  <tr key={p.pid} className="border-t border-border">
                    <td className="py-1.5 pr-3 tabular-nums text-muted">{p.pid}</td>
                    <td className="max-w-72 truncate py-1.5 pr-3" title={p.exe ?? ""}>
                      {p.name}
                    </td>
                    <td className="py-1.5 pr-3 tabular-nums">{p.cpu.toFixed(1)}%</td>
                    <td className="py-1.5 pr-3 tabular-nums">{formatBytes(p.memory, lang)}</td>
                    <td className="py-1.5 text-right">
                      <Button
                        size="sm"
                        variant="ghost"
                        title={t.system.kill}
                        onClick={() =>
                          window.confirm(fill(t.system.confirmKill, { name: p.name, pid: p.pid })) && kill.mutate(p.pid)
                        }
                      >
                        <X className="size-4 text-down" />
                      </Button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
            <ErrorText error={kill.error} />
          </Card>

          <div className="grid gap-4 xl:grid-cols-2">
            <Card className="overflow-x-auto">
              <h2 className="mb-3 font-medium">{t.system.ports}</h2>
              <div className="max-h-96 overflow-y-auto">
                <table className="w-full text-sm">
                  <thead className="sticky top-0 bg-surface text-left text-xs uppercase tracking-wide text-muted">
                    <tr>
                      <th className="py-2 pr-3">{t.system.port}</th>
                      <th className="py-2 pr-3">{t.system.protocol}</th>
                      <th className="py-2 pr-3">{t.system.address}</th>
                      <th className="py-2">{t.system.process}</th>
                    </tr>
                  </thead>
                  <tbody>
                    {(ports.data ?? []).map((p) => (
                      <tr key={`${p.protocol}-${p.address}-${p.port}`} className="border-t border-border">
                        <td className="py-1.5 pr-3 tabular-nums font-medium">{p.port}</td>
                        <td className="py-1.5 pr-3 uppercase text-muted">{p.protocol}</td>
                        <td className="py-1.5 pr-3 font-mono text-xs">{p.address}</td>
                        <td className="py-1.5 text-xs">
                          {p.process ?? "-"} {p.pids.length > 0 && <span className="text-muted">({p.pids.join(", ")})</span>}
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </Card>

            <Card>
              <div className="mb-3 flex items-center justify-between gap-2">
                <h2 className="font-medium">{t.system.services}</h2>
                <input
                  className={`${inputClass} max-w-48`}
                  placeholder={t.common.search}
                  value={serviceQuery}
                  onChange={(e) => setServiceQuery(e.target.value)}
                />
              </div>
              {filteredServices.length === 0 ? (
                <EmptyState>{services.isLoading ? t.common.loading : "-"}</EmptyState>
              ) : (
                <ul className="max-h-96 divide-y divide-border overflow-y-auto text-sm">
                  {filteredServices.map((s) => (
                    <li key={s.name} className="flex items-center justify-between gap-2 py-1.5">
                      <div className="min-w-0">
                        <p className="truncate">{s.displayName}</p>
                        <p className="truncate font-mono text-xs text-muted">{s.name}</p>
                      </div>
                      <div className="flex shrink-0 items-center gap-1">
                        <Badge className={serviceTone[s.state]}>{t.system.serviceStates[s.state]}</Badge>
                        <Button size="sm" variant="ghost" title={t.system.watchService} onClick={() => watchService.mutate(s.name)}>
                          <BellPlus className="size-4" />
                        </Button>
                      </div>
                    </li>
                  ))}
                </ul>
              )}
              <ErrorText error={watchService.error} />
            </Card>
          </div>
        </div>
      )}
    </>
  );
}
