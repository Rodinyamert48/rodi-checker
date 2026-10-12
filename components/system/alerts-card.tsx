"use client";

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Plus, Trash2 } from "lucide-react";
import { useState } from "react";

import { Button, Card, EmptyState, ErrorText, inputClass } from "@/components/ui";
import { useT } from "@/i18n";
import { api } from "@/lib/api";
import type { AlertMetric } from "@/lib/types";

const METRICS: AlertMetric[] = ["cpu", "memory", "disk", "temperature", "service", "port"];
const DEFAULT_THRESHOLD: Record<AlertMetric, number> = { cpu: 90, memory: 90, disk: 90, temperature: 85, service: 0, port: 0 };

export function AlertsCard() {
  const t = useT();
  const client = useQueryClient();
  const [metric, setMetric] = useState<AlertMetric>("cpu");
  const [threshold, setThreshold] = useState(90);
  const [target, setTarget] = useState("");

  const rules = useQuery({ queryKey: ["alert-rules"], queryFn: api.alertRules });
  const firing = useQuery({ queryKey: ["alert-firing"], queryFn: api.firingAlerts, refetchInterval: 5000 });
  const refresh = () => {
    client.invalidateQueries({ queryKey: ["alert-rules"] });
    client.invalidateQueries({ queryKey: ["alert-firing"] });
  };

  const create = useMutation({
    mutationFn: () => api.createAlertRule({ metric, threshold, target: target.trim() || null, enabled: true }),
    onSuccess: () => {
      setTarget("");
      refresh();
    },
  });
  const toggle = useMutation({ mutationFn: (v: { id: number; enabled: boolean }) => api.toggleAlertRule(v.id, v.enabled), onSettled: refresh });
  const remove = useMutation({ mutationFn: api.deleteAlertRule, onSettled: refresh });

  const needsThreshold = !["service", "port"].includes(metric);
  const needsTarget = ["disk", "service", "port"].includes(metric);
  const describe = (m: AlertMetric, th: number, tg: string | null) =>
    `${t.system.metrics[m]}${["service", "port"].includes(m) ? "" : ` > ${th}`}${tg ? ` · ${tg}` : ""}`;

  return (
    <Card className="space-y-4">
      <div>
        <h2 className="mb-2 font-medium">{t.system.firing}</h2>
        {(firing.data ?? []).length === 0 ? (
          <p className="text-sm text-up">{t.system.noFiring}</p>
        ) : (
          <ul className="space-y-1 text-sm text-down">
            {firing.data!.map((f) => (
              <li key={f.ruleId}>• {f.message}</li>
            ))}
          </ul>
        )}
      </div>

      <div>
        <h2 className="mb-2 font-medium">{t.system.alerts}</h2>
        {(rules.data ?? []).length === 0 ? (
          <EmptyState>{t.system.noAlerts}</EmptyState>
        ) : (
          <ul className="divide-y divide-border text-sm">
            {rules.data!.map((r) => (
              <li key={r.id} className="flex items-center justify-between gap-2 py-1.5">
                <label className="flex items-center gap-2">
                  <input
                    type="checkbox"
                    className="size-4 accent-[var(--color-accent)]"
                    checked={r.enabled}
                    onChange={(e) => toggle.mutate({ id: r.id, enabled: e.target.checked })}
                  />
                  {describe(r.metric, r.threshold, r.target)}
                </label>
                <Button size="sm" variant="ghost" onClick={() => remove.mutate(r.id)} title={t.common.delete}>
                  <Trash2 className="size-4 text-down" />
                </Button>
              </li>
            ))}
          </ul>
        )}
      </div>

      <div className="flex flex-wrap items-end gap-2 border-t border-border pt-3">
        <select
          className={`${inputClass} w-44`}
          value={metric}
          onChange={(e) => {
            const m = e.target.value as AlertMetric;
            setMetric(m);
            setThreshold(DEFAULT_THRESHOLD[m]);
          }}
        >
          {METRICS.map((m) => (
            <option key={m} value={m}>
              {t.system.metrics[m]}
            </option>
          ))}
        </select>
        {needsThreshold && (
          <input
            type="number"
            className={`${inputClass} w-24`}
            value={threshold}
            onChange={(e) => setThreshold(Number(e.target.value))}
            aria-label={t.system.threshold}
          />
        )}
        {needsTarget && (
          <input
            className={`${inputClass} w-48`}
            placeholder={t.system.targetHint[metric as "disk" | "service" | "port"]}
            value={target}
            onChange={(e) => setTarget(e.target.value)}
          />
        )}
        <Button variant="primary" disabled={create.isPending} onClick={() => create.mutate()}>
          <Plus className="size-4" /> {t.system.addRule}
        </Button>
      </div>
      <ErrorText error={create.error ?? toggle.error ?? remove.error} />
    </Card>
  );
}
