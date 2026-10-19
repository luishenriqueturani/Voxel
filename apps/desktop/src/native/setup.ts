import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export const SETUP_EVENT = "setup://progress";

export type SetupStep = {
  id: string;
  state: string;
  detail: string;
};

export type SetupStatus = {
  completed: boolean;
  profile: string | null;
  backend: string | null;
  steps: SetupStep[];
};

let inflight: Promise<SetupStatus | null> | null = null;

function asStatus(value: unknown): SetupStatus | null {
  if (typeof value !== "object" || value === null || !("completed" in value)) {
    return null;
  }
  const status = value as SetupStatus;
  if (typeof status.completed !== "boolean") {
    return null;
  }
  return {
    completed: status.completed,
    profile: typeof status.profile === "string" ? status.profile : null,
    backend: typeof status.backend === "string" ? status.backend : null,
    steps: Array.isArray(status.steps) ? status.steps : [],
  };
}

export async function loadSetupStatus(): Promise<SetupStatus | null> {
  if (!isTauri()) {
    return null;
  }
  return asStatus(await invoke("setup_status"));
}

export async function runSetup(): Promise<SetupStatus | null> {
  if (!isTauri()) {
    return null;
  }
  inflight ??= invoke("run_setup").then(asStatus);
  return inflight;
}

export async function resetSetup(): Promise<void> {
  inflight = null;
  if (!isTauri()) {
    return;
  }
  await invoke("reset_setup");
}

export async function listenToSetup(onStep: (step: SetupStep) => void): Promise<() => void> {
  if (!isTauri()) {
    return () => undefined;
  }
  return listen<SetupStep>(SETUP_EVENT, (event) => {
    onStep(event.payload);
  });
}
