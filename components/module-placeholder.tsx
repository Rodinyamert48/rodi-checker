"use client";

import { CheckCircle2 } from "lucide-react";

import { useT } from "@/i18n";

import { Badge, Card, PageHeader } from "./ui";

type ModuleKey = "web" | "proxy" | "files" | "system" | "reports";

/** Shown for modules whose implementation lands in a later phase of PLAN.md. */
export function ModulePlaceholder({ module, phase }: { module: ModuleKey; phase: number }) {
  const t = useT();
  const info = t.modules[module];

  return (
    <>
      <PageHeader
        title={t.nav[module]}
        subtitle={info.description}
        actions={
          <Badge className="bg-accent/10 text-accent">
            {t.common.comingSoon} · {t.common.phase} {phase}
          </Badge>
        }
      />
      <Card>
        <h2 className="mb-3 text-sm font-semibold uppercase tracking-wide text-muted">{t.common.plannedFeatures}</h2>
        <ul className="space-y-2">
          {info.features.map((feature) => (
            <li key={feature} className="flex items-start gap-2 text-sm">
              <CheckCircle2 className="mt-0.5 size-4 shrink-0 text-accent" aria-hidden />
              {feature}
            </li>
          ))}
        </ul>
      </Card>
    </>
  );
}
