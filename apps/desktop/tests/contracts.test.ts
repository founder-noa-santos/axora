import { describe, expect, it } from "vitest";

import {
  defaultPreferences,
  preferencesPatchSchema,
  preferencesSchema,
} from "@/shared/contracts/desktop";

describe("desktop contracts", () => {
  it("accepts the default preferences", () => {
    expect(preferencesSchema.parse(defaultPreferences)).toEqual(defaultPreferences);
  });

  it("rejects invalid preference patches", () => {
    expect(() =>
      preferencesPatchSchema.parse({ themeMode: "neon" }),
    ).toThrowError();
  });
});
