"use client";

import { useQuery } from "@tanstack/react-query";
import Link from "next/link";

import { NAV_ITEMS } from "@/components/nav";
import { Card, PageHeader } from "@/components/ui";
import { useT } from "@/i18n";
import { api } from "@/lib/api";
import type { CheckKind } from "@/lib/types";

const MODULE_KINDS: Record<string, CheckKind[]> = {
  web: ["http", "ssl", "dns", "tcp"],
  system: ["system"],
};

export default function DashboardPage() {
  const t = useT();
  const info = useQuery({ queryKey: ["app-info"], queryFn: api.appInfo });
  const monitors = useQuery({ queryKey: ["monitors"], queryFn: () => api.listMonitors() });

  const modules = NAV_ITEMS.filter(
    (item): item is Extract<(typeof NAV_ITEMS)[number], { key: "web" | "proxy" | "files" | "system" }> =>
      item.key === "web" || item.key === "proxy" || item.key === "files" || item.key === "system",
  );

  return (
    <>
      <PageHeader title={t.dashboard.title} subtitle={t.dashboard.subtitle} />

      <div className="grid gap-4 sm:grid-cols-2 xl:grid-cols-4">
        {modules.map(({ href, key, icon: Icon }) => {
          const kinds = MODULE_KINDS[key];
          const count = kinds ? (monitors.data ?? []).filter((m) => kinds.includes(m.kind)).length : null;
          return (
            <Link key={key} href={href} className="group">
              <Card className="h-full transition-colors group-hover:border-accent">
                <Icon className="mb-3 size-6 text-accent" aria-hidden />
                <h2 className="font-medium">{t.nav[key]}</h2>
                <p className="mt-1 text-sm text-muted">{t.modules[key].description}</p>
                {count !== null && (
                  <p className="mt-3 text-sm">
                    <span className="font-semibold">{count}</span> {t.common.monitors}
                  </p>
                )}
              </Card>
            </Link>
          );
        })}
      </div>

      <div className="mt-6 grid gap-4 lg:grid-cols-3">
        <Card className="lg:col-span-2">
          <h2 className="mb-2 font-medium">{t.dashboard.recentEvents}</h2>
          <p className="text-sm text-muted">{t.dashboard.noEvents}</p>
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
    </>
  );
}
