"use client";

import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { Button, Checkbox, ErrorText, Field, inputClass, Modal } from "@/components/ui";
import { useT } from "@/i18n";
import { api } from "@/lib/api";
import type { Monitor, NewMonitor, WebKind } from "@/lib/types";

const KINDS: WebKind[] = ["http", "ssl", "dns", "tcp"];
const METHODS = ["GET", "HEAD", "POST", "PUT", "PATCH", "DELETE", "OPTIONS"];
const RECORD_TYPES = ["A", "AAAA", "MX", "TXT", "CNAME", "NS"];

interface FormState {
  kind: WebKind;
  name: string;
  intervalSec: string;
  enabled: boolean;
  tags: string;
  url: string;
  method: string;
  headers: string;
  body: string;
  expectedStatus: string;
  keyword: string;
  keywordAbsent: boolean;
  followRedirects: boolean;
  degradedLatencyMs: string;
  host: string;
  port: string;
  warnDays: string;
  domain: string;
  recordType: string;
  expected: string;
}

const str = (v: unknown, fallback = "") => (v === undefined || v === null ? fallback : String(v));

function fromMonitor(monitor?: Monitor): FormState {
  const c = monitor?.config ?? {};
  const headers = (c.headers as Record<string, string> | undefined) ?? {};
  return {
    kind: (monitor?.kind as WebKind) ?? "http",
    name: monitor?.name ?? "",
    intervalSec: str(monitor?.intervalSec, "60"),
    enabled: monitor?.enabled ?? true,
    tags: monitor?.tags.join(", ") ?? "",
    url: str(c.url, "https://"),
    method: str(c.method, "GET"),
    headers: Object.entries(headers)
      .map(([k, v]) => `${k}: ${v}`)
      .join("\n"),
    body: str(c.body),
    expectedStatus: ((c.expectedStatus as number[] | undefined) ?? []).join(", "),
    keyword: str(c.keyword),
    keywordAbsent: Boolean(c.keywordAbsent),
    followRedirects: c.followRedirects === undefined ? true : Boolean(c.followRedirects),
    degradedLatencyMs: str(c.degradedLatencyMs),
    host: str(c.host),
    port: str(c.port, monitor?.kind === "ssl" ? "443" : ""),
    warnDays: str(c.warnDays, "14"),
    domain: str(c.domain),
    recordType: str(c.recordType, "A"),
    expected: str(c.expected),
  };
}

const optionalNumber = (value: string) => (value.trim() === "" ? null : Number(value));

export function toNewMonitor(f: FormState): NewMonitor {
  let config: Record<string, unknown>;
  switch (f.kind) {
    case "http":
      config = {
        url: f.url.trim(),
        method: f.method,
        headers: Object.fromEntries(
          f.headers
            .split("\n")
            .map((line) => line.split(/:(.*)/s).map((part) => part.trim()))
            .filter(([k, v]) => k && v !== undefined),
        ),
        body: f.body ? f.body : null,
        expectedStatus: f.expectedStatus
          .split(/[,\s]+/)
          .filter(Boolean)
          .map(Number)
          .filter((n) => Number.isInteger(n)),
        keyword: f.keyword || null,
        keywordAbsent: f.keywordAbsent,
        followRedirects: f.followRedirects,
        degradedLatencyMs: optionalNumber(f.degradedLatencyMs),
      };
      break;
    case "ssl":
      config = { host: f.host.trim(), port: Number(f.port || 443), warnDays: Number(f.warnDays || 14) };
      break;
    case "dns":
      config = { domain: f.domain.trim(), recordType: f.recordType, expected: f.expected || null };
      break;
    case "tcp":
      config = { host: f.host.trim(), port: Number(f.port) };
      break;
  }
  return {
    kind: f.kind,
    name: f.name.trim(),
    config,
    intervalSec: Number(f.intervalSec),
    enabled: f.enabled,
    tags: f.tags
      .split(",")
      .map((t) => t.trim())
      .filter(Boolean),
  };
}

export function MonitorForm({ monitor, onClose }: { monitor?: Monitor; onClose: () => void }) {
  const t = useT();
  const client = useQueryClient();
  const [form, setForm] = useState<FormState>(() => fromMonitor(monitor));
  const set = <K extends keyof FormState>(key: K, value: FormState[K]) => setForm((f) => ({ ...f, [key]: value }));

  const save = useMutation({
    mutationFn: (payload: NewMonitor) =>
      monitor ? api.updateMonitor(monitor.id, payload) : api.createMonitor(payload),
    onSuccess: () => {
      client.invalidateQueries({ queryKey: ["monitors"] });
      onClose();
    },
  });

  const text = (key: keyof FormState, label: string, props: Record<string, unknown> = {}, hint?: string) => (
    <Field label={label} hint={hint}>
      <input
        className={inputClass}
        value={form[key] as string}
        onChange={(e) => set(key, e.target.value as never)}
        {...props}
      />
    </Field>
  );

  return (
    <Modal
      open
      wide
      title={monitor ? t.web.edit : t.web.add}
      onClose={onClose}
      footer={
        <>
          <Button onClick={onClose}>{t.common.cancel}</Button>
          <Button variant="primary" disabled={save.isPending} onClick={() => save.mutate(toNewMonitor(form))}>
            {save.isPending ? t.common.saving : t.common.save}
          </Button>
        </>
      }
    >
      <div className="grid gap-4 sm:grid-cols-2">
        {text("name", t.web.name, { autoFocus: true, required: true })}
        <Field label={t.web.kind}>
          <select className={inputClass} value={form.kind} onChange={(e) => set("kind", e.target.value as WebKind)}>
            {KINDS.map((kind) => (
              <option key={kind} value={kind}>
                {t.web.kinds[kind]}
              </option>
            ))}
          </select>
        </Field>

        {form.kind === "http" && (
          <>
            <div className="sm:col-span-2">{text("url", t.web.url, { type: "url" })}</div>
            <Field label={t.web.method}>
              <select className={inputClass} value={form.method} onChange={(e) => set("method", e.target.value)}>
                {METHODS.map((m) => (
                  <option key={m}>{m}</option>
                ))}
              </select>
            </Field>
            {text("expectedStatus", t.web.expectedStatus, {}, t.web.expectedStatusHint)}
            {text("keyword", t.web.keyword)}
            {text("degradedLatencyMs", t.web.degradedLatency, { type: "number", min: 1 })}
            <div className="sm:col-span-2">
              <Field label={t.web.headers} hint={t.web.headersHint}>
                <textarea
                  rows={3}
                  className={`${inputClass} font-mono`}
                  value={form.headers}
                  onChange={(e) => set("headers", e.target.value)}
                />
              </Field>
            </div>
            {form.method !== "GET" && form.method !== "HEAD" && (
              <div className="sm:col-span-2">
                <Field label={t.web.body}>
                  <textarea
                    rows={3}
                    className={`${inputClass} font-mono`}
                    value={form.body}
                    onChange={(e) => set("body", e.target.value)}
                  />
                </Field>
              </div>
            )}
            <Checkbox label={t.web.keywordAbsent} checked={form.keywordAbsent} onChange={(v) => set("keywordAbsent", v)} />
            <Checkbox
              label={t.web.followRedirects}
              checked={form.followRedirects}
              onChange={(v) => set("followRedirects", v)}
            />
          </>
        )}

        {(form.kind === "ssl" || form.kind === "tcp") && (
          <>
            {text("host", t.web.host, { placeholder: "example.com" })}
            {text("port", t.web.port, { type: "number", min: 1, max: 65535 })}
          </>
        )}
        {form.kind === "ssl" && text("warnDays", t.web.warnDays, { type: "number", min: 1 })}

        {form.kind === "dns" && (
          <>
            {text("domain", t.web.domain, { placeholder: "example.com" })}
            <Field label={t.web.recordType}>
              <select className={inputClass} value={form.recordType} onChange={(e) => set("recordType", e.target.value)}>
                {RECORD_TYPES.map((r) => (
                  <option key={r}>{r}</option>
                ))}
              </select>
            </Field>
            {text("expected", t.web.expectedValue)}
          </>
        )}

        {text("intervalSec", t.web.interval, { type: "number", min: 5, max: 86400 })}
        {text("tags", t.web.tags, {}, t.web.tagsHint)}
        <Checkbox label={t.web.enabled} checked={form.enabled} onChange={(v) => set("enabled", v)} />
      </div>
      <div className="mt-4">
        <ErrorText error={save.error} />
      </div>
    </Modal>
  );
}
