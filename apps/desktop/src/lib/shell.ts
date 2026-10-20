import type { SetupStep } from "../native/setup";

export const SUMMARY_KEYS = [
  "error.summary.incompatibleKernel",
  "error.summary.outOfMemory",
  "error.summary.nativeCrash",
  "error.summary.network",
  "error.summary.unknown",
] as const;

export function mergeStep(steps: SetupStep[], step: SetupStep): SetupStep[] {
  const index = steps.findIndex((current) => current.id === step.id);
  if (index === -1) {
    return [...steps, step];
  }
  const next = steps.slice();
  next[index] = step;
  return next;
}

export function prefersDark(): boolean {
  return window.matchMedia?.("(prefers-color-scheme: dark)")?.matches ?? false;
}
