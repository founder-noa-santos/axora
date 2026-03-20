import type { EnvironmentName } from '../types.js';

export function cloneStructured<T>(value: T): T {
  return structuredClone(value);
}

export function deepFreeze<T>(value: T, seen = new WeakSet<object>()): T {
  if (value === null || typeof value !== 'object') {
    return value;
  }

  const objectValue = value as Record<string, unknown>;
  if (seen.has(objectValue)) {
    return value;
  }

  seen.add(objectValue);

  for (const key of Reflect.ownKeys(objectValue)) {
    const child = (objectValue as Record<PropertyKey, unknown>)[key];
    if (child !== null && (typeof child === 'object' || typeof child === 'function')) {
      deepFreeze(child, seen);
    }
  }

  return Object.freeze(objectValue) as T;
}

export function normalizeEnvironment(input?: string): EnvironmentName {
  if (input === 'production' || input === 'staging' || input === 'development') {
    return input;
  }

  return 'production';
}
