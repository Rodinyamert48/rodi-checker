"use client";

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import { api } from "./api";
import { DEFAULT_SETTINGS, type AppSettings } from "./types";

const KEY = ["settings"] as const;

export function useSettings() {
  const query = useQuery({ queryKey: KEY, queryFn: api.getSettings, staleTime: Infinity });
  return { ...query, settings: query.data ?? DEFAULT_SETTINGS };
}

export function useSaveSettings() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: (settings: AppSettings) => api.saveSettings(settings),
    onSuccess: (saved) => client.setQueryData(KEY, saved),
  });
}
