import { mkdir, readFile, writeFile } from "node:fs/promises";
import { dirname, join } from "node:path";

import { app } from "electron";

import {
  defaultPreferences,
  preferencesPatchSchema,
  preferencesSchema,
  type DesktopPreferences,
  type DesktopPreferencesPatch,
} from "@/shared/contracts/desktop";

const PREFERENCES_PATH = join(app.getPath("userData"), "preferences.json");

async function ensurePreferencesDir() {
  await mkdir(dirname(PREFERENCES_PATH), { recursive: true });
}

export async function readPreferences(): Promise<DesktopPreferences> {
  try {
    const raw = await readFile(PREFERENCES_PATH, "utf8");
    return preferencesSchema.parse(JSON.parse(raw));
  } catch {
    await writePreferences(defaultPreferences);
    return defaultPreferences;
  }
}

export async function writePreferences(
  patch: DesktopPreferencesPatch,
): Promise<DesktopPreferences> {
  const safePatch = preferencesPatchSchema.parse(patch);
  const current = await readPreferences();
  const next = preferencesSchema.parse({ ...current, ...safePatch });

  await ensurePreferencesDir();
  await writeFile(PREFERENCES_PATH, JSON.stringify(next, null, 2), "utf8");

  return next;
}
