import type { AnchorHTMLAttributes } from "react";

/**
 * Plain anchor used for in-app navigation instead of `next/link`.
 *
 * Client-side navigation in a static export fetches `*.txt` RSC payloads, which
 * Tauri's asset protocol serves as `text/plain`; Next.js then falls back to
 * opening the payload itself. A full page load of the exported HTML avoids it.
 */
export function AppLink(props: AnchorHTMLAttributes<HTMLAnchorElement> & { href: string }) {
  return <a {...props} />;
}
