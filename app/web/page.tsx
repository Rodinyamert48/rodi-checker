"use client";

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Pencil, Play, Plus, Trash2 } from "lucide-react";
import { useMemo, useState } from "react";

import { Badge, Button, Card, EmptyState, ErrorText, PageHeader, StatusBadge } from "@/components/ui";
import { MonitorDetail } from "@/components/web/monitor-detail";
import { MonitorForm } from "@/components/web/monitor-form";
import { useT } from "@/i18n";
import { api } from "@/lib/api";
import { cn } from "@/lib/cn";
import { useTauriEvent } from "@/lib/events";
import { formatDateTime, formatDuration } from "@/lib/format";
import { useSettings } from "@/lib/settings";
import type { Monitor, MonitorResultEvent, StoredResult } from "@/lib/types";

function describeTarget(monitor: Monitor): string {
  const c = monitor.config as Record<string, string | number | undefined>;
  switch (monitor.kind) {
    case "http":
      return `${c.method ?? "GET"} ${c.url}`;
    case "ssl":
      return `${c.host}:${c.port ?? 443}`;
    case "dns":
      return `${c.recordType ?? "A"} ${c.domain}`;
    case "tcp":
      return `${c.host}:${c.port}`;
    default:
      return "";
  }
}

export default function WebPage() {
  const t = useT();
  const { settings } = useSettings();
  const client = useQueryClient();
  const [editing, setEditing] = useState<Monitor | "new" | null>(null);
  const [selectedId, setSelectedId] = useState<number | null>(null);

  const monitors = useQuery({ queryKey: ["monitors"], queryFn: () => api.listMonitors() });
  const latest = useQuery({ queryKey: ["monitors", "latest"], queryFn: api.latestResults });

  const latestById = useMemo(
    () => new Map<number, StoredResult>((latest.data ?? []).map((r) => [r.monitorId, r])),
    [latest.data],
  );
  const webMonitors = (monitors.data ?? []).filter((m) => ["http", "ssl", "dns", "tcp"].includes(m.kind));
  const selected = webMonitors.find((m) => m.id === selectedId) ?? null;

  useTauriEvent<MonitorResultEvent>("monitor-result", () => {
    client.invalidateQueries({ queryKey: ["monitors"] });
  });

  const runNow = useMutation({
    mutationFn: api.runMonitorNow,
    onSettled: () => client.invalidateQueries({ queryKey: ["monitors"] }),
  });
  const remove = useMutation({
    mutationFn: api.deleteMonitor,
    onSuccess: (_, id) => {
      if (id === selectedId) setSelectedId(null);
      client.invalidateQueries({ queryKey: ["monitors"] });
    },
  });

  return (
    <>
      <PageHeader
        title={t.nav.web}
        subtitle={t.modules.web.description}
        actions={
          <Button variant="primary" onClick={() => setEditing("new")}>
            <Plus className="size-4" /> {t.web.add}
          </Button>
        }
      />

      <Card className="overflow-x-auto p-0">
        {webMonitors.length === 0 ? (
          <EmptyState>{monitors.isLoading ? t.common.loading : t.web.empty}</EmptyState>
        ) : (
          <table className="w-full text-sm">
            <thead className="text-left text-xs uppercase tracking-wide text-muted">
              <tr>
                <th className="px-4 py-3">{t.web.name}</th>
                <th className="px-4 py-3">{t.web.target}</th>
                <th className="px-4 py-3">{t.common.status}</th>
                <th className="px-4 py-3">{t.web.latency}</th>
                <th className="px-4 py-3">{t.web.lastCheck}</th>
                <th className="px-4 py-3" />
              </tr>
            </thead>
            <tbody>
              {webMonitors.map((m) => {
                const last = latestById.get(m.id);
                return (
                  <tr
                    key={m.id}
                    onClick={() => setSelectedId(m.id)}
                    className={cn(
                      "cursor-pointer border-t border-border hover:bg-fg/5",
                      m.id === selectedId && "bg-accent/5",
                    )}
                  >
                    <td className="px-4 py-2.5">
                      <div className="flex items-center gap-2">
                        <span className="font-medium">{m.name}</span>
                        <Badge className="bg-fg/5 text-muted">{t.web.kinds[m.kind as "http"]}</Badge>
                        {!m.enabled && <Badge className="bg-degraded/10 text-degraded">{t.web.paused}</Badge>}
                        {m.tags.map((tag) => (
                          <Badge key={tag} className="bg-accent/10 text-accent">
                            {tag}
                          </Badge>
                        ))}
                      </div>
                    </td>
                    <td className="max-w-72 truncate px-4 py-2.5 font-mono text-xs text-muted" title={describeTarget(m)}>
                      {describeTarget(m)}
                    </td>
                    <td className="px-4 py-2.5" title={last?.message ?? ""}>
                      <StatusBadge status={last?.status} />
                    </td>
                    <td className="px-4 py-2.5 tabular-nums">{formatDuration(last?.latencyMs)}</td>
                    <td className="px-4 py-2.5 text-xs text-muted">{formatDateTime(last?.checkedAt, settings.language)}</td>
                    <td className="px-4 py-2.5" onClick={(e) => e.stopPropagation()}>
                      <div className="flex justify-end gap-1">
                        <Button
                          size="sm"
                          variant="ghost"
                          title={t.web.runNow}
                          disabled={runNow.isPending && runNow.variables === m.id}
                          onClick={() => runNow.mutate(m.id)}
                        >
                          <Play className="size-4" />
                        </Button>
                        <Button size="sm" variant="ghost" title={t.common.edit} onClick={() => setEditing(m)}>
                          <Pencil className="size-4" />
                        </Button>
                        <Button
                          size="sm"
                          variant="ghost"
                          title={t.common.delete}
                          onClick={() => window.confirm(t.web.confirmDelete) && remove.mutate(m.id)}
                        >
                          <Trash2 className="size-4 text-down" />
                        </Button>
                      </div>
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        )}
      </Card>
      <div className="mt-2">
        <ErrorText error={runNow.error ?? remove.error} />
      </div>

      <div className="mt-6">
        {selected ? (
          <MonitorDetail key={selected.id} monitor={selected} />
        ) : (
          webMonitors.length > 0 && <p className="text-sm text-muted">{t.web.selectHint}</p>
        )}
      </div>

      {editing && <MonitorForm monitor={editing === "new" ? undefined : editing} onClose={() => setEditing(null)} />}
    </>
  );
}
