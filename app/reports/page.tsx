"use client";

import { useMutation, useQuery } from "@tanstack/react-query";
import { Download } from "lucide-react";
import { useState } from "react";

import { Button, Card, EmptyState, ErrorText, Field, inputClass, PageHeader } from "@/components/ui";
import { useT } from "@/i18n";
import { api } from "@/lib/api";
import { pickSavePath } from "@/lib/dialog";
import { fill } from "@/lib/i18n-format";
import type { ReportFormat, ReportKind, ReportRequest } from "@/lib/types";

const KINDS: ReportKind[] = ["monitor_results", "incidents", "proxies", "baselines", "system_samples"];
const RANGES = ["24", "168", "720", "8760"] as const;
const TIMED: ReportKind[] = ["monitor_results", "incidents", "system_samples"];

export default function ReportsPage() {
  const t = useT();
  const [kind, setKind] = useState<ReportKind>("monitor_results");
  const [range, setRange] = useState<(typeof RANGES)[number]>("24");
  const [targetId, setTargetId] = useState<number | null>(null);
  const [format, setFormat] = useState<ReportFormat>("html");

  const request: ReportRequest = { kind, hours: Number(range), targetId };

  const monitors = useQuery({ queryKey: ["monitors"], queryFn: () => api.listMonitors() });
  const lists = useQuery({ queryKey: ["proxy-lists"], queryFn: api.proxyLists });
  const preview = useQuery({ queryKey: ["report-preview", request], queryFn: () => api.reportPreview(request) });

  const targets =
    kind === "monitor_results" || kind === "incidents"
      ? (monitors.data ?? []).map((m) => ({ id: m.id, name: m.name }))
      : kind === "proxies"
        ? (lists.data ?? []).map((l) => ({ id: l.id, name: l.name }))
        : [];

  const exportReport = useMutation({
    mutationFn: async () => {
      const name = `${t.reports.kinds[kind]} ${new Date().toISOString().slice(0, 10)}.${format}`;
      const path = await pickSavePath(name, [{ name: format.toUpperCase(), extensions: [format] }]);
      if (!path) return null;
      return { count: await api.reportExport(request, format, path), path };
    },
  });

  const table = preview.data?.table;

  return (
    <>
      <PageHeader
        title={t.nav.reports}
        subtitle={t.modules.reports.description}
        actions={
          <Button variant="primary" disabled={exportReport.isPending} onClick={() => exportReport.mutate()}>
            <Download className="size-4" /> {t.common.export}
          </Button>
        }
      />

      <Card className="mb-4 grid gap-4 sm:grid-cols-2 xl:grid-cols-4">
        <Field label={t.reports.kind}>
          <select
            className={inputClass}
            value={kind}
            onChange={(e) => {
              setKind(e.target.value as ReportKind);
              setTargetId(null);
            }}
          >
            {KINDS.map((k) => (
              <option key={k} value={k}>
                {t.reports.kinds[k]}
              </option>
            ))}
          </select>
        </Field>
        <Field label={t.reports.range}>
          <select
            className={inputClass}
            value={range}
            disabled={!TIMED.includes(kind)}
            onChange={(e) => setRange(e.target.value as (typeof RANGES)[number])}
          >
            {RANGES.map((r) => (
              <option key={r} value={r}>
                {t.reports.ranges[r]}
              </option>
            ))}
          </select>
        </Field>
        <Field label={t.reports.target}>
          <select
            className={inputClass}
            value={targetId ?? ""}
            disabled={targets.length === 0}
            onChange={(e) => setTargetId(e.target.value ? Number(e.target.value) : null)}
          >
            <option value="">{t.reports.allTargets}</option>
            {targets.map((target) => (
              <option key={target.id} value={target.id}>
                {target.name}
              </option>
            ))}
          </select>
        </Field>
        <Field label={t.reports.format}>
          <select className={inputClass} value={format} onChange={(e) => setFormat(e.target.value as ReportFormat)}>
            <option value="html">HTML</option>
            <option value="csv">CSV (Excel)</option>
            <option value="json">JSON</option>
          </select>
        </Field>
      </Card>

      {exportReport.data && (
        <p className="mb-3 break-all text-sm text-up">
          {fill(t.reports.exported, { count: exportReport.data.count, path: exportReport.data.path })}
        </p>
      )}
      <ErrorText error={exportReport.error ?? preview.error} />

      <Card className="overflow-x-auto">
        <div className="mb-3 flex items-center justify-between">
          <h2 className="font-medium">
            {t.reports.preview}: {table?.title}
          </h2>
          {preview.data && (
            <span className="text-xs text-muted">
              {fill(t.reports.rows, { shown: preview.data.table.rows.length, total: preview.data.totalRows })}
            </span>
          )}
        </div>
        {!table || table.rows.length === 0 ? (
          <EmptyState>{preview.isLoading ? t.common.loading : t.reports.empty}</EmptyState>
        ) : (
          <table className="w-full text-sm">
            <thead className="text-left text-xs uppercase tracking-wide text-muted">
              <tr>
                {table.headers.map((h) => (
                  <th key={h} className="whitespace-nowrap px-3 py-2">
                    {h}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {table.rows.map((row, i) => (
                <tr key={i} className="border-t border-border">
                  {row.map((cell, j) => (
                    <td key={j} className="max-w-80 truncate whitespace-nowrap px-3 py-1.5" title={cell}>
                      {cell}
                    </td>
                  ))}
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </Card>
    </>
  );
}
