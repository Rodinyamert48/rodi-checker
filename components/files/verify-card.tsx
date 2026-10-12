"use client";

import { useMutation } from "@tanstack/react-query";
import { CheckCircle2, FileSearch, XCircle } from "lucide-react";
import { useState } from "react";

import { Button, Card, ErrorText, inputClass } from "@/components/ui";
import { useT } from "@/i18n";
import { api } from "@/lib/api";
import { pickFile } from "@/lib/dialog";
import { formatBytes } from "@/lib/format";
import { useSettings } from "@/lib/settings";

const normalize = (s: string) => s.trim().toLowerCase().replace(/[\s:]/g, "");

export function VerifyCard() {
  const t = useT();
  const { settings } = useSettings();
  const [path, setPath] = useState<string | null>(null);
  const [expected, setExpected] = useState("");

  const hash = useMutation({
    mutationFn: async () => {
      const picked = await pickFile();
      if (!picked) return null;
      setPath(picked);
      return api.filesHash(picked);
    },
  });

  const result = hash.data;
  const wanted = normalize(expected);
  const matched = result && wanted ? [result.sha256, result.blake3].includes(wanted) : null;

  return (
    <Card>
      <div className="mb-3 flex items-center justify-between">
        <h2 className="font-medium">{t.files.verifyTitle}</h2>
        <Button disabled={hash.isPending} onClick={() => hash.mutate()}>
          <FileSearch className="size-4" /> {hash.isPending ? t.common.loading : t.files.pickFile}
        </Button>
      </div>
      {path && <p className="mb-2 break-all font-mono text-xs text-muted">{path}</p>}
      {result && (
        <dl className="mb-3 grid gap-1 text-sm sm:grid-cols-[6rem_1fr]">
          <dt className="text-muted">SHA-256</dt>
          <dd className="break-all font-mono text-xs">{result.sha256}</dd>
          <dt className="text-muted">BLAKE3</dt>
          <dd className="break-all font-mono text-xs">{result.blake3}</dd>
          <dt className="text-muted">{t.files.size}</dt>
          <dd>{formatBytes(result.size, settings.language)}</dd>
        </dl>
      )}
      <div className="flex items-center gap-3">
        <input
          className={`${inputClass} font-mono`}
          placeholder={t.files.expected}
          value={expected}
          onChange={(e) => setExpected(e.target.value)}
        />
        {matched === true && (
          <span className="flex shrink-0 items-center gap-1 text-sm text-up">
            <CheckCircle2 className="size-4" /> {t.files.match}
          </span>
        )}
        {matched === false && (
          <span className="flex shrink-0 items-center gap-1 text-sm text-down">
            <XCircle className="size-4" /> {t.files.noMatch}
          </span>
        )}
      </div>
      <ErrorText error={hash.error} />
    </Card>
  );
}
