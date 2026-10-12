"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";

import { useT } from "@/i18n";
import { cn } from "@/lib/cn";

import { AppIcon, NAV_ITEMS } from "./nav";

export function Sidebar() {
  const t = useT();
  const pathname = usePathname();

  return (
    <aside className="flex w-60 shrink-0 flex-col border-r border-border bg-surface">
      <div className="flex items-center gap-2 px-5 py-5">
        <AppIcon className="size-6 text-accent" aria-hidden />
        <div>
          <p className="font-semibold leading-tight">{t.app.name}</p>
          <p className="text-xs text-muted">{t.app.tagline}</p>
        </div>
      </div>
      <nav className="flex flex-1 flex-col gap-1 px-3">
        {NAV_ITEMS.map(({ href, key, icon: Icon }) => {
          const active = href === "/" ? pathname === "/" : pathname.startsWith(href.replace(/\/$/, ""));
          return (
            <Link
              key={key}
              href={href}
              className={cn(
                "flex items-center gap-3 rounded-md px-3 py-2 text-sm transition-colors",
                active ? "bg-accent/10 font-medium text-accent" : "text-fg/80 hover:bg-fg/5",
              )}
            >
              <Icon className="size-4" aria-hidden />
              {t.nav[key]}
            </Link>
          );
        })}
      </nav>
    </aside>
  );
}
