import { invoke, isTauri } from "@tauri-apps/api/core";

export type MachineStatus = {
  gpuName: string | null;
  backend: string | null;
  vramUsedBytes: number | null;
  vramTotalBytes: number | null;
  gpuPercent: number | null;
  cpuPercent: number | null;
  ramUsedBytes: number | null;
  ramTotalBytes: number | null;
  cpuCelsius: number | null;
  gpuCelsius: number | null;
};

function isStatus(value: unknown): value is MachineStatus {
  if (typeof value !== "object" || value === null) {
    return false;
  }
  const status = value as MachineStatus;
  return typeof status.backend === "string";
}

export async function loadMachineStatus(): Promise<MachineStatus | null> {
  if (!isTauri()) {
    return null;
  }
  const value = await invoke("machine_status");
  return isStatus(value) ? value : null;
}
