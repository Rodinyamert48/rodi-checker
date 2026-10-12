"use client";

import { useSettings } from "@/lib/settings";
import type { Language } from "@/lib/types";

import en from "./en";
import tr, { type Dictionary } from "./tr";

const dictionaries: Record<Language, Dictionary> = { tr, en };

export function dictionary(language: Language): Dictionary {
  return dictionaries[language] ?? tr;
}

/** Returns the dictionary for the current language setting. */
export function useT(): Dictionary {
  const { settings } = useSettings();
  return dictionary(settings.language);
}

export type { Dictionary };
