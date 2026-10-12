"use client";

import { useEffect, useState } from "react";

import { Button, Card, Checkbox as Toggle, Field, inputClass, PageHeader } from "@/components/ui";
import { pickFile } from "@/lib/dialog";
import { useT } from "@/i18n";
import { useSaveSettings, useSettings } from "@/lib/settings";
import { DEFAULT_SETTINGS, type AppSettings, type Language, type Theme } from "@/lib/types";

export default function SettingsPage() {
  const t = useT();
  const { settings, isLoading } = useSettings();
  const save = useSaveSettings();
  const [draft, setDraft] = useState<AppSettings>(settings);

  useEffect(() => setDraft(settings), [settings]);

  const set = <K extends keyof AppSettings>(key: K, value: AppSettings[K]) =>
    setDraft((current) => ({ ...current, [key]: value }));

  if (isLoading) return <p className="text-sm text-muted">{t.common.loading}</p>;

  return (
    <form
      onSubmit={(e) => {
        e.preventDefault();
        save.mutate(draft);
      }}
    >
      <PageHeader
        title={t.settings.title}
        subtitle={t.settings.subtitle}
        actions={
          <div className="flex items-center gap-3">
            {save.isSuccess && <span className="text-sm text-up">{t.common.saved}</span>}
            {save.isError && <span className="text-sm text-down">{String(save.error)}</span>}
            <button
              type="button"
              className="rounded-md border border-border px-3 py-2 text-sm hover:bg-fg/5"
              onClick={() => setDraft(DEFAULT_SETTINGS)}
            >
              {t.common.reset}
            </button>
            <button
              type="submit"
              disabled={save.isPending}
              className="rounded-md bg-accent px-4 py-2 text-sm font-medium text-white disabled:opacity-60"
            >
              {save.isPending ? t.common.saving : t.common.save}
            </button>
          </div>
        }
      />

      <div className="grid gap-4 lg:grid-cols-2">
        <Card className="space-y-4">
          <h2 className="font-medium">{t.settings.appearance}</h2>
          <Field label={t.settings.language}>
            <select
              className={inputClass}
              value={draft.language}
              onChange={(e) => set("language", e.target.value as Language)}
            >
              <option value="tr">Türkçe</option>
              <option value="en">English</option>
            </select>
          </Field>
          <Field label={t.settings.theme}>
            <select className={inputClass} value={draft.theme} onChange={(e) => set("theme", e.target.value as Theme)}>
              {(["system", "light", "dark"] as const).map((theme) => (
                <option key={theme} value={theme}>
                  {t.settings.themes[theme]}
                </option>
              ))}
            </select>
          </Field>
        </Card>

        <Card className="space-y-1">
          <h2 className="mb-2 font-medium">{t.settings.behavior}</h2>
          <Toggle label={t.settings.notifications} checked={draft.notifications} onChange={(v) => set("notifications", v)} />
          <Toggle label={t.settings.minimizeToTray} checked={draft.minimizeToTray} onChange={(v) => set("minimizeToTray", v)} />
          <Toggle label={t.settings.startMinimized} checked={draft.startMinimized} onChange={(v) => set("startMinimized", v)} />
          <Toggle label={t.settings.autostart} checked={draft.autostart} onChange={(v) => set("autostart", v)} />
        </Card>

        <Card className="space-y-4 lg:col-span-2">
          <h2 className="font-medium">{t.settings.checks}</h2>
          <div className="grid gap-4 sm:grid-cols-3">
            <Field label={t.settings.timeout}>
              <input
                type="number"
                min={1}
                max={300}
                className={inputClass}
                value={draft.defaultTimeoutSecs}
                onChange={(e) => set("defaultTimeoutSecs", Number(e.target.value))}
              />
            </Field>
            <Field label={t.settings.concurrency}>
              <input
                type="number"
                min={1}
                max={1000}
                className={inputClass}
                value={draft.defaultConcurrency}
                onChange={(e) => set("defaultConcurrency", Number(e.target.value))}
              />
            </Field>
            <Field label={t.settings.retention}>
              <input
                type="number"
                min={1}
                max={3650}
                className={inputClass}
                value={draft.retentionDays}
                onChange={(e) => set("retentionDays", Number(e.target.value))}
              />
            </Field>
          </div>
          <Field label={t.settings.proxyJudge} hint={t.settings.proxyJudgeHint}>
            <input
              type="url"
              placeholder="https://judge.example.com/headers"
              className={inputClass}
              value={draft.proxyJudgeUrl ?? ""}
              onChange={(e) => set("proxyJudgeUrl", e.target.value || null)}
            />
          </Field>
          <Field label={t.settings.proxyTestUrl} hint={t.settings.proxyTestUrlHint}>
            <input
              type="url"
              placeholder="http://www.gstatic.com/generate_204"
              className={inputClass}
              value={draft.proxyTestUrl ?? ""}
              onChange={(e) => set("proxyTestUrl", e.target.value || null)}
            />
          </Field>
          <Field label={t.settings.geoip} hint={t.settings.geoipHint}>
            <div className="flex gap-2">
              <input
                className={inputClass}
                placeholder="C:\\GeoLite2-Country.mmdb"
                value={draft.geoipDbPath ?? ""}
                onChange={(e) => set("geoipDbPath", e.target.value || null)}
              />
              <Button
                onClick={async () => {
                  const path = await pickFile([{ name: "MaxMind DB", extensions: ["mmdb"] }]);
                  if (path) set("geoipDbPath", path);
                }}
              >
                {t.common.browse}
              </Button>
            </div>
          </Field>
        </Card>
      </div>
    </form>
  );
}
