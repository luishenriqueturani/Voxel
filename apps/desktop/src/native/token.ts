import { invoke, isTauri } from "@tauri-apps/api/core";

export async function loadHfTokenStatus(): Promise<boolean> {
  if (!isTauri()) {
    return false;
  }
  return (await invoke("hf_token_status")) === true;
}

export async function saveHfToken(token: string): Promise<boolean> {
  if (!isTauri()) {
    return false;
  }
  return (await invoke("set_hf_token", { token })) === true;
}
