// File pickers backed by the Tauri dialog plugin (desktop only).

import { open, save } from "@tauri-apps/plugin-dialog";

import { inTauri } from "./api";

type Filter = { name: string; extensions: string[] };

export async function pickFile(filters?: Filter[]): Promise<string | null> {
  if (!inTauri()) return null;
  const result = await open({ multiple: false, directory: false, filters });
  return typeof result === "string" ? result : null;
}

export async function pickFolder(): Promise<string | null> {
  if (!inTauri()) return null;
  const result = await open({ multiple: false, directory: true });
  return typeof result === "string" ? result : null;
}

export async function pickSavePath(defaultPath: string, filters?: Filter[]): Promise<string | null> {
  if (!inTauri()) return null;
  return save({ defaultPath, filters });
}
