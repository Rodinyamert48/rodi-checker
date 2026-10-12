"use client";

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Download, Play, Plus, Square, Trash2, X } from "lucide-react";
import { useState } from "react";

import { ExportDialog } from "@/components/proxy/export-dialog";
import { ImportDialog } from "@/components/proxy/import-dialog";
import { ProxyTable } from "@/components/proxy/proxy-table";
import { Button, Card, EmptyState, ErrorText, PageHeader, Stat } from "@/components/ui";
import { useT } from "@/i18n";
import { api } from "@/lib/api";
import { cn } from "@/lib/cn";
import { useTauriEvent } from "@/lib/events";
import { fill } from "@/lib/i18n-format";
import { useSettings } from "@/lib/settings";
import type { ProxyProgressEvent } from "@/lib/types";

export default function ProxyPage() {
  const t = useT();
  const { settings } = useSettings();
  const client = useQueryClient();
  const [selectedId, setSelectedId] = useState<number | null>(null);
  const [importing, setImporting] = useState(false);
  const [exporting, setExporting] = useState(false);
  const [progress, setProgress] = useState<Record<number, ProxyProgressEvent>>({});
  const [notice, setNotice] = useState<string | null>(null);

  const lists = useQuery({ queryKey: ["proxy-lists"], queryFn: api.proxyLists });
  const running = useQuery({ queryKey: ["proxy-running"], queryFn: api.proxyRunning });
  const selected = (lists.data ?? []).find((l) => l.id === selectedId) ?? lists.data?.[0] ?? null;
  const isRunning = (id: number) => progress[id]?.running ?? (running.data ?? []).includes(id);

  const items = useQuery({
    queryKey: ["proxy-items", selected?.id],
    queryFn: () => api.proxyItems(selected!.id),
    enabled: selected != null,
    refetchInterval: selected && isRunning(selected.id) ? 1500 : false,
  });

  useTauriEvent<ProxyProgressEvent>("proxy-progress", (event) => {
    setProgress((current) => ({ ...current, [event.listId]: event }));
    if (!event.running) {
      client.invalidateQueries({ queryKey: ["proxy-lists"] });
      client.invalidateQueries({ queryKey: ["proxy-items", event.listId] });
      client.invalidateQueries({ queryKey: ["proxy-running"] });
    }
  });

  const refresh = () => {
    client.invalidateQueries({ queryKey: ["proxy-lists"] });
    client.invalidateQueries({ queryKey: ["proxy-items"] });
    client.invalidateQueries({ queryKey: ["proxy-running"] });
  };

  const check = useMutation({ mutationFn: api.proxyCheck, onSettled: refresh });
  const cancel = useMutation({ mutationFn: api.proxyCancel });
  const removeDead = useMutation({
    mutationFn: api.proxyRemoveDead,
    onSuccess: (count) => {
      setNotice(fill(t.proxy.removedDead, { count }));
      refresh();
    },
  });
  const deleteList = useMutation({
    mutationFn: api.proxyDeleteList,
    onSuccess: () => {
      setSelectedId(null);
      refresh();
    },
  });

  const current = selected ? progress[selected.id] : undefined;

  return (
    <>
      <PageHeader
        title={t.nav.proxy}
        subtitle={t.modules.proxy.description}
        actions={
          <Button variant="primary" onClick={() => setImporting(true)}>
            <Plus className="size-4" /> {t.proxy.newList}
          </Button>
        }
      />

      {!settings.proxyJudgeUrl && <p className="mb-4 text-sm text-degraded">{t.proxy.judgeMissing}</p>}

      <div className="grid gap-4 lg:grid-cols-[13rem_1fr]">
        <Card className="h-fit p-2">
          <h2 className="px-2 py-1 text-xs font-semibold uppercase tracking-wide text-muted">{t.proxy.lists}</h2>
          {(lists.data ?? []).length === 0 ? (
            <EmptyState>{t.proxy.noLists}</EmptyState>
          ) : (
            <ul>
              {lists.data!.map((list) => (
                <li key={list.id}>
                  <button
                    type="button"
                    onClick={() => setSelectedId(list.id)}
                    className={cn(
                      "w-full rounded-md px-2 py-2 text-left text-sm hover:bg-fg/5",
                      list.id === selected?.id && "bg-accent/10",
                    )}
                  >
                    <span className="block truncate font-medium">{list.name}</span>
                    <span className="text-xs text-muted">
                      {list.working}/{list.total} {t.proxy.working.toLowerCase()}
                      {isRunning(list.id) && ` · ${t.proxy.checking}`}
                    </span>
                  </button>
                </li>
              ))}
            </ul>
          )}
        </Card>

        {selected && (
          <div className="min-w-0 space-y-4">
            <div className="flex flex-wrap items-center justify-between gap-2">
              <h2 className="text-lg font-semibold">{selected.name}</h2>
              <div className="flex flex-wrap gap-2">
                {isRunning(selected.id) ? (
                  <Button onClick={() => cancel.mutate(selected.id)}>
                    <Square className="size-4" /> {t.common.stop}
                  </Button>
                ) : (
                  <Button variant="primary" onClick={() => check.mutate(selected.id)}>
                    <Play className="size-4" /> {t.proxy.check}
                  </Button>
                )}
                <Button onClick={() => setExporting(true)}>
                  <Download className="size-4" /> {t.common.export}
                </Button>
                <Button disabled={isRunning(selected.id)} onClick={() => removeDead.mutate(selected.id)}>
                  <X className="size-4" /> {t.proxy.removeDead}
                </Button>
                <Button
                  variant="ghost"
                  title={t.common.delete}
                  onClick={() => window.confirm(t.proxy.confirmDeleteList) && deleteList.mutate(selected.id)}
                >
                  <Trash2 className="size-4 text-down" />
                </Button>
              </div>
            </div>

            <div className="grid gap-3 sm:grid-cols-3">
              <Stat label={t.proxy.total} value={selected.total} />
              <Stat label={t.proxy.checked} value={selected.checked} />
              <Stat label={t.proxy.working} value={<span className="text-up">{selected.working}</span>} />
            </div>

            {current?.running && (
              <Card className="py-3">
                <p className="mb-2 text-sm">
                  {fill(t.proxy.progress, { done: current.done, total: current.total, working: current.working })}
                </p>
                <div className="h-2 overflow-hidden rounded-full bg-fg/10">
                  <div
                    className="h-full bg-accent transition-all"
                    style={{ width: `${current.total ? (current.done / current.total) * 100 : 0}%` }}
                  />
                </div>
              </Card>
            )}

            {notice && <p className="text-sm text-up">{notice}</p>}
            <ErrorText error={check.error ?? removeDead.error ?? deleteList.error} />

            <Card>
              <ProxyTable proxies={items.data ?? []} />
            </Card>
          </div>
        )}
      </div>

      {importing && (
        <ImportDialog
          onClose={() => setImporting(false)}
          onImported={(list) => {
            setImporting(false);
            setSelectedId(list.id);
          }}
        />
      )}
      {exporting && selected && <ExportDialog list={selected} onClose={() => setExporting(false)} />}
    </>
  );
}
