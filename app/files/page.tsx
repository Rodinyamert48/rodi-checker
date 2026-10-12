"use client";

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { GitCompare, Plus, RefreshCw, Trash2 } from "lucide-react";
import { useState } from "react";

import { BaselineForm } from "@/components/files/baseline-form";
import { CompareView } from "@/components/files/compare-view";
import { VerifyCard } from "@/components/files/verify-card";
import { Badge, Button, Card, EmptyState, ErrorText, PageHeader } from "@/components/ui";
import { useT } from "@/i18n";
import { api } from "@/lib/api";
import { useTauriEvent } from "@/lib/events";
import { formatBytes, formatDateTime } from "@/lib/format";
import { fill } from "@/lib/i18n-format";
import { useSettings } from "@/lib/settings";
import type { Baseline, FilesChangeEvent, FilesProgressEvent } from "@/lib/types";

export default function FilesPage() {
  const t = useT();
  const { settings } = useSettings();
  const lang = settings.language;
  const client = useQueryClient();
  const [creating, setCreating] = useState(false);
  const [progress, setProgress] = useState<FilesProgressEvent | null>(null);
  const [changes, setChanges] = useState<(FilesChangeEvent & { at: string })[]>([]);

  const baselines = useQuery({ queryKey: ["baselines"], queryFn: api.filesBaselines });
  const watching = useQuery({ queryKey: ["baselines", "watching"], queryFn: api.filesWatching });

  useTauriEvent<FilesProgressEvent>("files-progress", (event) => setProgress(event.running ? event : null));
  useTauriEvent<FilesChangeEvent>("files-change", (event) =>
    setChanges((current) => [{ ...event, at: new Date().toISOString() }, ...current].slice(0, 50)),
  );

  const compare = useMutation({ mutationFn: api.filesCompare });
  const update = useMutation({
    mutationFn: (b: Baseline) =>
      api.filesCreateBaseline(b.name, b.rootPath, b.algorithm, b.excludes.split("\n").filter(Boolean)),
    onSuccess: () => {
      compare.reset();
      client.invalidateQueries({ queryKey: ["baselines"] });
    },
  });
  const remove = useMutation({
    mutationFn: api.filesDeleteBaseline,
    onSuccess: () => client.invalidateQueries({ queryKey: ["baselines"] }),
  });
  const watch = useMutation({
    mutationFn: ({ id, enabled }: { id: number; enabled: boolean }) => api.filesWatch(id, enabled),
    onSettled: () => client.invalidateQueries({ queryKey: ["baselines", "watching"] }),
  });

  const busy = progress !== null || compare.isPending || update.isPending;

  return (
    <>
      <PageHeader
        title={t.nav.files}
        subtitle={t.modules.files.description}
        actions={
          <Button variant="primary" disabled={busy} onClick={() => setCreating(true)}>
            <Plus className="size-4" /> {t.files.newBaseline}
          </Button>
        }
      />

      <div className="space-y-4">
        <VerifyCard />

        {progress && (
          <Card className="py-3">
            <div className="mb-2 flex items-center justify-between text-sm">
              <span>{fill(t.files.scanning, { done: progress.done, total: progress.total })}</span>
              <Button size="sm" onClick={() => api.filesCancel()}>
                {t.common.stop}
              </Button>
            </div>
            <div className="h-2 overflow-hidden rounded-full bg-fg/10">
              <div
                className="h-full bg-accent transition-all"
                style={{ width: `${progress.total ? (progress.done / progress.total) * 100 : 0}%` }}
              />
            </div>
          </Card>
        )}

        <Card className="overflow-x-auto p-0">
          <h2 className="px-5 pt-4 font-medium">{t.files.baselines}</h2>
          {(baselines.data ?? []).length === 0 ? (
            <EmptyState>{t.files.noBaselines}</EmptyState>
          ) : (
            <table className="mt-2 w-full text-sm">
              <thead className="text-left text-xs uppercase tracking-wide text-muted">
                <tr>
                  <th className="px-5 py-2">{t.files.name}</th>
                  <th className="px-3 py-2">{t.files.files}</th>
                  <th className="px-3 py-2">{t.files.size}</th>
                  <th className="px-3 py-2">{t.files.created}</th>
                  <th className="px-3 py-2">{t.files.watch}</th>
                  <th className="px-3 py-2" />
                </tr>
              </thead>
              <tbody>
                {baselines.data!.map((b) => (
                  <tr key={b.id} className="border-t border-border">
                    <td className="px-5 py-2">
                      <div className="flex items-center gap-2 font-medium">
                        {b.name}
                        <Badge className="bg-fg/5 text-muted">{b.algorithm.toUpperCase()}</Badge>
                      </div>
                      <div className="break-all font-mono text-xs text-muted">{b.rootPath}</div>
                    </td>
                    <td className="px-3 py-2 tabular-nums">{b.fileCount}</td>
                    <td className="whitespace-nowrap px-3 py-2">{formatBytes(b.totalSize, lang)}</td>
                    <td className="px-3 py-2 text-xs text-muted">{formatDateTime(b.createdAt.replace(" ", "T"), lang)}</td>
                    <td className="px-3 py-2">
                      <input
                        type="checkbox"
                        className="size-4 accent-[var(--color-accent)]"
                        checked={(watching.data ?? []).includes(b.id)}
                        onChange={(e) => watch.mutate({ id: b.id, enabled: e.target.checked })}
                      />
                    </td>
                    <td className="px-3 py-2">
                      <div className="flex justify-end gap-1">
                        <Button size="sm" disabled={busy} onClick={() => compare.mutate(b.id)}>
                          <GitCompare className="size-4" /> {t.files.compare}
                        </Button>
                        <Button
                          size="sm"
                          variant="ghost"
                          title={t.files.updateHint}
                          disabled={busy}
                          onClick={() => window.confirm(t.files.confirmUpdate) && update.mutate(b)}
                        >
                          <RefreshCw className="size-4" />
                        </Button>
                        <Button
                          size="sm"
                          variant="ghost"
                          title={t.common.delete}
                          onClick={() => window.confirm(t.files.confirmDelete) && remove.mutate(b.id)}
                        >
                          <Trash2 className="size-4 text-down" />
                        </Button>
                      </div>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </Card>
        <ErrorText error={compare.error ?? update.error ?? remove.error ?? watch.error} />

        {compare.data && <CompareView result={compare.data} />}

        <Card>
          <h2 className="mb-2 font-medium">{t.files.liveChanges}</h2>
          {changes.length === 0 ? (
            <p className="text-sm text-muted">{t.files.noChanges}</p>
          ) : (
            <ul className="space-y-2 text-sm">
              {changes.map((c, i) => (
                <li key={`${c.at}-${i}`} className="rounded-md border border-border p-2">
                  <div className="flex justify-between">
                    <span className="font-medium">{c.baselineName}</span>
                    <span className="text-xs text-muted">{formatDateTime(c.at, lang)}</span>
                  </div>
                  <p className="mt-1 break-all font-mono text-xs text-muted">{c.paths.slice(0, 10).join(", ")}</p>
                </li>
              ))}
            </ul>
          )}
        </Card>
      </div>

      {creating && <BaselineForm onClose={() => setCreating(false)} />}
    </>
  );
}
