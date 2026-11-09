import { invoke, isTauri } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import { type FlowDocument, type FlowDraft, type FlowItem, isMeshMode } from "../lib/flow";

const memory: FlowDocument[] = [];

function isItem(value: unknown): value is FlowItem {
  if (typeof value !== "object" || value === null) {
    return false;
  }
  const item = value as FlowItem;
  return (
    typeof item.id === "string" &&
    typeof item.prompt === "string" &&
    typeof item.count === "number" &&
    isMeshMode(item.mesh) &&
    (item.templateId === null ||
      item.templateId === undefined ||
      typeof item.templateId === "string") &&
    typeof item.overrides === "object" &&
    item.overrides !== null
  );
}

function isFlow(value: unknown): value is FlowDocument {
  if (typeof value !== "object" || value === null) {
    return false;
  }
  const flow = value as FlowDocument;
  return (
    typeof flow.id === "string" &&
    typeof flow.name === "string" &&
    (flow.templateId === null || typeof flow.templateId === "string") &&
    (flow.path === null || typeof flow.path === "string") &&
    Array.isArray(flow.items) &&
    flow.items.every(isItem)
  );
}

function localId(prefix: string): string {
  return `${prefix}-${memory.length + 1}-${Date.now()}`;
}

export async function loadFlows(): Promise<FlowDocument[]> {
  if (!isTauri()) {
    return memory.slice();
  }
  const value = await invoke("list_flows");
  return Array.isArray(value) ? value.filter(isFlow) : [];
}

export async function createFlow(draft: FlowDraft): Promise<FlowDocument> {
  if (!isTauri()) {
    const created: FlowDocument = { ...draft, id: localId("flow") };
    memory.push(created);
    return created;
  }
  const value = await invoke("create_flow", { draft });
  if (!isFlow(value)) {
    throw new Error("fluxo inválido");
  }
  return value;
}

export async function updateFlow(id: string, draft: FlowDraft): Promise<FlowDocument> {
  if (!isTauri()) {
    const updated: FlowDocument = { ...draft, id };
    const index = memory.findIndex((flow) => flow.id === id);
    if (index < 0) {
      throw new Error("fluxo ausente");
    }
    memory[index] = updated;
    return updated;
  }
  const value = await invoke("update_flow", { id, draft });
  if (!isFlow(value)) {
    throw new Error("fluxo inválido");
  }
  return value;
}

export async function deleteFlow(id: string): Promise<void> {
  if (!isTauri()) {
    const index = memory.findIndex((flow) => flow.id === id);
    if (index >= 0) {
      memory.splice(index, 1);
    }
    return;
  }
  await invoke("delete_flow", { id });
}

export async function pickFlowToOpen(): Promise<string | null> {
  if (!isTauri()) {
    return null;
  }
  const selected = await open({
    multiple: false,
    filters: [{ name: "Fluxo", extensions: ["json"] }],
  });
  return typeof selected === "string" ? selected : null;
}

export async function pickFlowToSave(defaultPath: string | null): Promise<string | null> {
  if (!isTauri()) {
    return null;
  }
  const destination = await save({
    defaultPath: defaultPath ?? "fluxo.json",
    filters: [{ name: "Fluxo", extensions: ["json"] }],
  });
  return typeof destination === "string" ? destination : null;
}

export async function importFlowFile(path: string): Promise<FlowDocument> {
  if (!isTauri()) {
    throw new Error("abrir fluxo exige o aplicativo");
  }
  const value = await invoke("import_flow_file", { path });
  if (!isFlow(value)) {
    throw new Error("fluxo inválido");
  }
  return value;
}

export async function exportFlowFile(id: string, path: string): Promise<FlowDocument> {
  if (!isTauri()) {
    throw new Error("salvar fluxo exige o aplicativo");
  }
  const value = await invoke("export_flow_file", { id, path });
  if (!isFlow(value)) {
    throw new Error("fluxo inválido");
  }
  return value;
}
