import fs from "node:fs/promises";
import path from "node:path";
import YAML from "yaml";
import { AktaConfigSchema, type AktaConfig } from "./schema.js";

export class ConfigError extends Error {
  constructor(
    message: string,
    public readonly cause?: unknown,
  ) {
    super(message);
    this.name = "ConfigError";
  }
}

export async function loadConfig(configPath: string): Promise<AktaConfig> {
  let raw: string;
  try {
    raw = await fs.readFile(configPath, "utf8");
  } catch (e) {
    throw new ConfigError(`Cannot read config: ${configPath}`, e);
  }
  let data: unknown;
  try {
    data = YAML.parse(raw);
  } catch (e) {
    throw new ConfigError(`Invalid YAML in ${configPath}`, e);
  }
  const parsed = AktaConfigSchema.safeParse(data);
  if (!parsed.success) {
    throw new ConfigError(
      `Invalid .akta-config.yaml: ${parsed.error.flatten().formErrors.join("; ")}`,
      parsed.error,
    );
  }
  return parsed.data;
}

export function resolveConfigPath(root: string, explicit?: string): string {
  return path.resolve(explicit ?? path.join(root, ".akta-config.yaml"));
}
