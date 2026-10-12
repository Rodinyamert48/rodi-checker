"use client";

import { CartesianGrid, Legend, Line, LineChart, ResponsiveContainer, Tooltip, XAxis, YAxis } from "recharts";

import { formatDateTime, formatTime } from "@/lib/format";
import type { Language } from "@/lib/types";

export interface UsagePoint {
  ts: string;
  cpu: number;
  memory: number;
}

export function UsageChart({ points, language, labels }: { points: UsagePoint[]; language: Language; labels: { cpu: string; memory: string } }) {
  return (
    <div className="h-56">
      <ResponsiveContainer width="100%" height="100%">
        <LineChart data={points} margin={{ top: 4, right: 8, bottom: 0, left: 0 }}>
          <CartesianGrid strokeDasharray="3 3" stroke="var(--color-border)" vertical={false} />
          <XAxis dataKey="ts" tickFormatter={(v) => formatTime(v, language)} stroke="var(--color-muted)" fontSize={11} minTickGap={40} />
          <YAxis domain={[0, 100]} unit="%" stroke="var(--color-muted)" fontSize={11} width={44} />
          <Tooltip
            contentStyle={{ background: "var(--color-surface)", border: "1px solid var(--color-border)" }}
            labelFormatter={(v) => formatDateTime(String(v), language)}
            formatter={(v) => `${Number(v).toFixed(1)}%`}
          />
          <Legend wrapperStyle={{ fontSize: 12 }} />
          <Line type="monotone" dataKey="cpu" name={labels.cpu} stroke="var(--color-accent)" dot={false} strokeWidth={2} isAnimationActive={false} />
          <Line type="monotone" dataKey="memory" name={labels.memory} stroke="var(--color-degraded)" dot={false} strokeWidth={2} isAnimationActive={false} />
        </LineChart>
      </ResponsiveContainer>
    </div>
  );
}
