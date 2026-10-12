"use client";

import { Card } from "@/components/ui";
import { useT } from "@/i18n";
import { cn } from "@/lib/cn";
import { formatBytes } from "@/lib/format";
import { fill } from "@/lib/i18n-format";
import { useSettings } from "@/lib/settings";
import type { CompareResult } from "@/lib/types";

const LIMIT = 300;

function Section({ title, tone, rows }: { title: string; tone: string; rows: { path: string; detail: string }[] }) {
  const t = useT();
  if (rows.length === 0) return null;
  return (
    <div>
      <h3 className={cn("mb-1 text-sm font-medium", tone)}>
        {title} ({rows.length})
      </h3>
      <ul className="max-h-72 overflow-y-auto rounded-md border border-border font-mono text-xs">
        {rows.slice(0, LIMIT).map((row) => (
          <li key={row.path} className="flex justify-between gap-4 border-t border-border px-2 py-1 first:border-0">
            <span className="break-all">{row.path}</span>
            <span className="shrink-0 text-muted">{row.detail}</span>
          </li>
        ))}
        {rows.length > LIMIT && <li className="px-2 py-1 text-muted">{fill(t.files.moreRows, { count: rows.length - LIMIT })}</li>}
      </ul>
    </div>
  );
}

export function CompareView({ result }: { result: CompareResult }) {
  const t = useT();
  const { settings } = useSettings();
  const size = (n: number) => formatBytes(n, settings.language);
  const { diff } = result;
  const clean = !diff.added.length && !diff.removed.length && !diff.modified.length;

  return (
    <Card className="space-y-4">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <h2 className="font-medium">{fill(t.files.result, { name: result.baseline.name })}</h2>
        <div className="flex gap-3 text-sm">
          <span className="text-up">+{diff.added.length} {t.files.added}</span>
          <span className="text-down">−{diff.removed.length} {t.files.removed}</span>
          <span className="text-degraded">~{diff.modified.length} {t.files.modified}</span>
          <span className="text-muted">{diff.unchanged} {t.files.unchanged}</span>
        </div>
      </div>
      {clean && <p className="text-sm text-up">{fill(t.files.clean, { count: diff.unchanged })}</p>}
      <Section title={t.files.modified} tone="text-degraded" rows={diff.modified.map((m) => ({ path: m.path, detail: `${size(m.oldSize)} → ${size(m.newSize)}` }))} />
      <Section title={t.files.added} tone="text-up" rows={diff.added.map((e) => ({ path: e.path, detail: size(e.size) }))} />
      <Section title={t.files.removed} tone="text-down" rows={diff.removed.map((e) => ({ path: e.path, detail: size(e.size) }))} />
      <Section title={t.files.unreadable} tone="text-muted" rows={result.errors.map(([path, reason]) => ({ path, detail: reason }))} />
    </Card>
  );
}
