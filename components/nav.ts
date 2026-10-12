import { Activity, FileCheck2, FileText, Gauge, Globe, LayoutDashboard, Settings, Shuffle } from "lucide-react";

export const NAV_ITEMS = [
  { href: "/", key: "dashboard", icon: LayoutDashboard },
  { href: "/web/", key: "web", icon: Globe },
  { href: "/proxy/", key: "proxy", icon: Shuffle },
  { href: "/files/", key: "files", icon: FileCheck2 },
  { href: "/system/", key: "system", icon: Gauge },
  { href: "/reports/", key: "reports", icon: FileText },
  { href: "/settings/", key: "settings", icon: Settings },
] as const;

export type NavKey = (typeof NAV_ITEMS)[number]["key"];

export const AppIcon = Activity;
