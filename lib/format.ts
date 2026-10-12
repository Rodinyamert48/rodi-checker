import type { Language } from "./types";

const locale = (language: Language) => (language === "tr" ? "tr-TR" : "en-US");

export function formatDateTime(value: string | null | undefined, language: Language): string {
  if (!value) return "-";
  const date = new Date(value);
  return Number.isNaN(date.getTime())
    ? "-"
    : date.toLocaleString(locale(language), { dateStyle: "short", timeStyle: "medium" });
}

export function formatTime(value: string, language: Language): string {
  return new Date(value).toLocaleTimeString(locale(language), { hour: "2-digit", minute: "2-digit" });
}

export function formatNumber(value: number, language: Language, digits = 0): string {
  return value.toLocaleString(locale(language), { maximumFractionDigits: digits, minimumFractionDigits: digits });
}

export function formatBytes(bytes: number, language: Language): string {
  const units = ["B", "KB", "MB", "GB", "TB"];
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }
  return `${formatNumber(value, language, unit === 0 ? 0 : 1)} ${units[unit]}`;
}

export function formatDuration(ms: number | null | undefined): string {
  if (ms == null) return "-";
  return ms < 1000 ? `${Math.round(ms)} ms` : `${(ms / 1000).toFixed(2)} s`;
}
