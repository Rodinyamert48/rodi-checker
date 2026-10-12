"use client";

import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { Button, ErrorText, Field, inputClass, Modal } from "@/components/ui";
import { useT } from "@/i18n";
import { api } from "@/lib/api";
import { pickFolder } from "@/lib/dialog";
import type { HashAlgorithm } from "@/lib/types";

export function BaselineForm({ onClose }: { onClose: () => void }) {
  const t = useT();
  const client = useQueryClient();
  const [name, setName] = useState("");
  const [root, setRoot] = useState("");
  const [algorithm, setAlgorithm] = useState<HashAlgorithm>("blake3");
  const [excludes, setExcludes] = useState("node_modules\n.git\n*.tmp");

  const create = useMutation({
    mutationFn: () =>
      api.filesCreateBaseline(
        name || root.split(/[\\/]/).filter(Boolean).pop() || root,
        root,
        algorithm,
        excludes.split("\n").map((l) => l.trim()).filter(Boolean),
      ),
    onSuccess: () => {
      client.invalidateQueries({ queryKey: ["baselines"] });
      onClose();
    },
  });

  return (
    <Modal
      open
      title={t.files.newBaseline}
      onClose={onClose}
      footer={
        <>
          <Button onClick={onClose}>{t.common.cancel}</Button>
          <Button variant="primary" disabled={!root || create.isPending} onClick={() => create.mutate()}>
            {create.isPending ? t.common.loading : t.common.create}
          </Button>
        </>
      }
    >
      <div className="space-y-4">
        <Field label={t.files.folder}>
          <div className="flex gap-2">
            <input className={inputClass} value={root} onChange={(e) => setRoot(e.target.value)} />
            <Button
              onClick={async () => {
                const folder = await pickFolder();
                if (folder) setRoot(folder);
              }}
            >
              {t.common.browse}
            </Button>
          </div>
        </Field>
        <Field label={t.files.name}>
          <input className={inputClass} value={name} placeholder={root.split(/[\\/]/).filter(Boolean).pop()} onChange={(e) => setName(e.target.value)} />
        </Field>
        <Field label={t.files.algorithm}>
          <select className={inputClass} value={algorithm} onChange={(e) => setAlgorithm(e.target.value as HashAlgorithm)}>
            <option value="blake3">BLAKE3</option>
            <option value="sha256">SHA-256</option>
          </select>
        </Field>
        <Field label={t.files.excludes} hint={t.files.excludesHint}>
          <textarea rows={4} className={`${inputClass} font-mono`} value={excludes} onChange={(e) => setExcludes(e.target.value)} />
        </Field>
        <ErrorText error={create.error} />
      </div>
    </Modal>
  );
}
