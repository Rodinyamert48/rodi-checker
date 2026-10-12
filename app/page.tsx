"use client";

import { useQuery } from "@tanstack/react-query";
import { AlertTriangle, CheckCircle2 } from "lucide-react";
import type { ReactNode } from "react";

import { AppLink } from "@/components/app-link";
import { NAV_ITEMS } from "@/components/nav";
import { Card, PageHeader } from "@/components/ui";
import { useT } from "@/i18n";
import { api } from "@/lib/api";
import { useTauriEvent } from "@/lib/events";
import { formatDateTime, formatNumber } from "@/lib/format";
import { fill } from "@/lib/i18n-format";
import { useSettings } from "@/lib/settings";

type ModuleKey = "web" | "proxy" | "files" | "system";

export default function DashboardPage() {
  const t = useT();
  const { settings } = useSettings();
  const lang = settings.language;
  const info = useQuery({ queryKey: ["app-info"], queryFn: api.appInfo });
  const summary = useQuery({ queryKey: ["dashboard"], queryFn: api.dashboard, refetchInterval: 10000 });

  useTauriEvent("monitor-result", () => summary.refetch());

  const s = summary.data;
  const lines: Record<ModuleKey, ReactNode> = {
    web: s && (
      <>
        <p className="text-lg font-semibold">{fill(t.dashboard.monitorsUp, { up: s.monitors.up, total: s.monitors.total })}</p>
        <p className="text-xs text-muted">
          {fill(t.dashboard.monitorsDetail, { down: s.monitors.down, degraded: s.monitors.degraded, paused: s.monitors.paused })}
        </p>
      </>
    ),
    proxy: s && (
      <>
        <p className="text-lg font-semibold">{fill(t.dashboard.proxySummary, { working: s.proxiesWorking, total: s.proxiesTotal })}</p>
        <p className="text-xs text-muted">{fill(t.dashboard.proxyLists, { count: s.proxyLists })}</p>
      </>
    ),
    files: s && (
      <>
        <p className="text-lg font-semibold">{fill(t.dashboard.baselineSummary, { count: s.baselines })}</p>
        <p className="text-xs text-muted">{fill(t.dashboard.watchingSummary, { count: s.watching })}</p>
      </>
    ),
    system: s && s.cpu !== null && (
      <p className="text-lg font-semibold">
        {fill(t.dashboard.systemSummary, {
          cpu: formatNumber(s.cpu, lang, 0),
          mem: formatNumber(s.memPercent ?? 0, lang, 0),
        })}
      </p>
    ),
  };

  const modules = NAV_ITEMS.filter((item): item is Extract<(typeof NAV_ITEMS)[number], { key: ModuleKey }> =>
    ["web", "proxy", "files", "system"].includes(item.key),
  );
  const problems = (s?.monitors.down ?? 0) + (s?.firing.length ?? 0);

  return (
    <>
      <PageHeader title={t.dashboard.title} subtitle={t.dashboard.subtitle} />

      <div className="grid gap-4 sm:grid-cols-2 xl:grid-cols-4">
        {modules.map(({ href, key, icon: Icon }) => (
          <AppLink key={key} href={href} className="group">
            <Card className="h-full transition-colors group-hover:border-accent">
              <div className="mb-3 flex items-center gap-2">
                <Icon className="size-5 text-accent" aria-hidden />
                <h2 className="font-medium">{t.nav[key]}</h2>
              </div>
              {lines[key] ?? <p className="text-sm text-muted">{t.modules[key].description}</p>}
            </Card>
          </AppLink>
        ))}
      </div>

      <div className="mt-6 grid gap-4 lg:grid-cols-3">
        <Card className="lg:col-span-2">
          <h2 className="mb-3 font-medium">{t.dashboard.recentEvents}</h2>
          {(s?.incidents ?? []).length === 0 ? (
            <p className="text-sm text-muted">{t.dashboard.noEvents}</p>
          ) : (
            <ul className="divide-y divide-border text-sm">
              {s!.incidents.map((i) => (
                <li key={i.id} className="flex items-start justify-between gap-4 py-2">
                  <div>
                    <p className="font-medium">{i.monitorName}</p>
                    {i.cause && <p className="text-xs text-muted">{i.cause}</p>}
                  </div>
                  <div className="shrink-0 text-right text-xs">
                    <p>{formatDateTime(i.startedAt, lang)}</p>
                    <p className={i.resolvedAt ? "text-up" : "text-down"}>
                      {i.resolvedAt ? `${t.web.resolvedAt}: ${formatDateTime(i.resolvedAt, lang)}` : t.web.ongoing}
                    </p>
                  </div>
                </li>
              ))}
            </ul>
          )}
        </Card>

        <div className="space-y-4">
          <Card>
            <h2 className="mb-2 font-medium">{t.dashboard.activeAlerts}</h2>
            {problems === 0 ? (
              <p className="flex items-center gap-2 text-sm text-up">
                <CheckCircle2 className="size-4" /> {t.dashboard.allGood}
              </p>
            ) : (
              <ul className="space-y-1 text-sm text-down">
                {(s?.monitors.down ?? 0) > 0 && (
                  <li className="flex items-center gap-2">
                    <AlertTriangle className="size-4" />
                    {s!.monitors.down} {t.status.down.toLowerCase()}
                  </li>
                )}
                {s?.firing.map((f) => (
                  <li key={f.ruleId} className="flex items-center gap-2">
                    <AlertTriangle className="size-4" /> {f.message}
                  </li>
                ))}
              </ul>
            )}
          </Card>
          <Card>
            <dl className="space-y-3 text-sm">
              <div>
                <dt className="text-muted">{t.dashboard.version}</dt>
                <dd className="font-medium">{info.data?.version ?? t.common.loading}</dd>
              </div>
              <div>
                <dt className="text-muted">{t.dashboard.database}</dt>
                <dd className="break-all font-mono text-xs">{info.data?.databasePath ?? t.common.loading}</dd>
              </div>
            </dl>
          </Card>
        </div>
      </div>
    </>
  );
}
