import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export const SETUP_EVENT = "setup://progress";

export type SetupStep = {
  id: string;
  state: string;
  detail: string;
};

export type ModelSuggestion = {
  id: string;
  bytes: number;
  license: string;
  optional: boolean;
};

export type ProfileCatalog = {
  profile: string;
  models: ModelSuggestion[];
};

export type SetupStatus = {
  completed: boolean;
  profile: string | null;
  backend: string | null;
  steps: SetupStep[];
  catalog: ProfileCatalog[];
};

export type SetupChoices = {
  models: string[];
  profile: string | null;
  backend: string | null;
};

export type SetupConfirmation = {
  completed: boolean;
  image: string | null;
  outcome: string;
  detail: string;
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
    catalog: Array.isArray(status.catalog) ? status.catalog : [],
  };
}

function asConfirmation(value: unknown): SetupConfirmation | null {
  if (typeof value !== "object" || value === null || !("outcome" in value)) {
    return null;
  }
  const confirmation = value as SetupConfirmation;
  if (typeof confirmation.outcome !== "string" || typeof confirmation.completed !== "boolean") {
    return null;
  }
  return {
    completed: confirmation.completed,
    image: typeof confirmation.image === "string" ? confirmation.image : null,
    outcome: confirmation.outcome,
    detail: typeof confirmation.detail === "string" ? confirmation.detail : "",
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

export async function confirmSetup(choices: SetupChoices): Promise<SetupConfirmation | null> {
  if (!isTauri()) {
    return null;
  }
  return asConfirmation(await invoke("confirm_setup", { choices }));
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
