"use client";

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { FileUp } from "lucide-react";
import { useDeferredValue, useState } from "react";

import { Button, ErrorText, Field, inputClass, Modal } from "@/components/ui";
import { useT } from "@/i18n";
import { api } from "@/lib/api";
import { pickFile } from "@/lib/dialog";
import { fill } from "@/lib/i18n-format";
import type { ProxyListSummary, ProxyProtocol } from "@/lib/types";

const PROTOCOLS: ProxyProtocol[] = ["http", "https", "socks4", "socks5"];

export function ImportDialog({ onClose, onImported }: { onClose: () => void; onImported: (list: ProxyListSummary) => void }) {
  const t = useT();
  const client = useQueryClient();
  const [name, setName] = useState(() => `Liste ${new Date().toLocaleDateString()}`);
  const [protocol, setProtocol] = useState<ProxyProtocol>("http");
  const [text, setText] = useState("");
  const deferredText = useDeferredValue(text);

  const preview = useQuery({
    queryKey: ["proxy-preview", deferredText, protocol],
    queryFn: () => api.proxyPreview(deferredText, protocol),
    enabled: deferredText.trim().length > 0,
  });

  const loadFile = useMutation({
    mutationFn: async () => {
      const path = await pickFile([{ name: "Proxy list", extensions: ["txt", "csv", "list"] }]);
      if (!path) return;
      setText(await api.readTextFile(path));
      const base = path.split(/[\\/]/).pop();
      if (base) setName(base.replace(/\.[^.]+$/, ""));
    },
  });

  const save = useMutation({
    mutationFn: () => api.proxyImport(name, text, protocol),
    onSuccess: (list) => {
      client.invalidateQueries({ queryKey: ["proxy-lists"] });
      onImported(list);
    },
  });

  return (
    <Modal
      open
      wide
      title={t.proxy.newList}
      onClose={onClose}
      footer={
        <>
          <Button onClick={onClose}>{t.common.cancel}</Button>
          <Button
            variant="primary"
            disabled={save.isPending || !preview.data?.count}
            onClick={() => save.mutate()}
          >
            {t.common.import}
          </Button>
        </>
      }
    >
      <div className="space-y-4">
        <div className="grid gap-4 sm:grid-cols-2">
          <Field label={t.proxy.listName}>
            <input className={inputClass} value={name} onChange={(e) => setName(e.target.value)} />
          </Field>
          <Field label={t.proxy.defaultProtocol} hint={t.proxy.defaultProtocolHint}>
            <select className={inputClass} value={protocol} onChange={(e) => setProtocol(e.target.value as ProxyProtocol)}>
              {PROTOCOLS.map((p) => (
                <option key={p} value={p}>
                  {p.toUpperCase()}
                </option>
              ))}
            </select>
          </Field>
        </div>
        <Field label={t.proxy.paste} hint={t.proxy.pasteHint}>
          <textarea
            rows={10}
            spellCheck={false}
            className={`${inputClass} font-mono text-xs`}
            value={text}
            placeholder={"1.2.3.4:8080\nsocks5://user:pass@5.6.7.8:1080"}
            onChange={(e) => setText(e.target.value)}
          />
        </Field>
        <div className="flex items-center justify-between gap-4">
          <Button size="sm" onClick={() => loadFile.mutate()}>
            <FileUp className="size-4" /> {t.proxy.loadFile}
          </Button>
          {preview.data && (
            <p className="text-sm text-muted">
              {fill(t.proxy.preview, {
                count: preview.data.count,
                duplicates: preview.data.duplicates,
                errors: preview.data.errorCount,
              })}
            </p>
          )}
        </div>
        {preview.data && preview.data.errors.length > 0 && (
          <ul className="max-h-32 overflow-y-auto rounded-md border border-border p-2 font-mono text-xs text-down">
            {preview.data.errors.map((e) => (
              <li key={e.line}>
                {t.proxy.line} {e.line}: {e.text} — {e.reason}
              </li>
            ))}
          </ul>
        )}
        <ErrorText error={save.error ?? loadFile.error ?? preview.error} />
      </div>
    </Modal>
  );
}
