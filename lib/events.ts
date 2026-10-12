"use client";

import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useEffect, useRef } from "react";

import { inTauri } from "./api";

/** Subscribes to a backend event for the lifetime of the component. */
export function useTauriEvent<T>(event: string, handler: (payload: T) => void) {
  const handlerRef = useRef(handler);
  useEffect(() => {
    handlerRef.current = handler;
  });

  useEffect(() => {
    if (!inTauri()) return;
    let unlisten: UnlistenFn | undefined;
    let cancelled = false;
    listen<T>(event, (e) => handlerRef.current(e.payload)).then((fn) => {
      if (cancelled) fn();
      else unlisten = fn;
    });
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [event]);
}
