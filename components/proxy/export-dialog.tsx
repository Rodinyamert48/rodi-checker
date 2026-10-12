"use client";

import { useMutation } from "@tanstack/react-query";
import { useState } from "react";

import { Button, Checkbox, ErrorText, Field, inputClass, Modal } from "@/components/ui";
import { useT } from "@/i18n";
import { api } from "@/lib/api";
import { pickSavePath } from "@/lib/dialog";
import { fill } from "@/lib/i18n-format";
import type { ProxyListSummary } from "@/lib/types";

export function ExportDialog({ list, onClose }: { list: ProxyListSummary; onClose: () => void }) {
  const t = useT();
  const [format, setFormat] = useState<"txt" | "csv">("txt");
  const [onlyWorking, setOnlyWorking] = useState(true);
  const [includePasswords, setIncludePasswords] = useState(false);

  const run = useMutation({
    mutationFn: async () => {
      const path = await pickSavePath(`${list.name}.${format}`, [{ name: format.toUpperCase(), extensions: [format] }]);
      if (!path) return null;
      return api.proxyExport(list.id, path, format, onlyWorking, includePasswords);
    },
  });

  return (
    <Modal
      open
      title={t.proxy.exportTitle}
      onClose={onClose}
      footer={
        <>
          <Button onClick={onClose}>{t.common.close}</Button>
          <Button variant="primary" disabled={run.isPending} onClick={() => run.mutate()}>
            {t.common.export}
          </Button>
        </>
      }
    >
      <div className="space-y-3">
        <Field label={t.proxy.format}>
          <select className={inputClass} value={format} onChange={(e) => setFormat(e.target.value as "txt" | "csv")}>
            <option value="txt">TXT (scheme://host:port)</option>
            <option value="csv">CSV</option>
          </select>
        </Field>
        <Checkbox label={t.proxy.onlyWorking} checked={onlyWorking} onChange={setOnlyWorking} />
        <Checkbox label={t.proxy.includePasswords} checked={includePasswords} onChange={setIncludePasswords} />
        {run.data != null && <p className="text-sm text-up">{fill(t.proxy.exported, { count: run.data })}</p>}
        <ErrorText error={run.error} />
      </div>
    </Modal>
  );
}
