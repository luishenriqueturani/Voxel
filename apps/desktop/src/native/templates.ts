import { invoke, isTauri } from "@tauri-apps/api/core";
import type { PromptPlacement } from "../lib/templatePrompt";

export type PromptTemplate = {
  id: string;
  name: string;
  body: string;
  negative: string;
  placement: PromptPlacement;
};

export type TemplateDraft = Omit<PromptTemplate, "id">;

function isPlacement(value: unknown): value is PromptPlacement {
  return value === "before" || value === "after";
}

function isTemplate(value: unknown): value is PromptTemplate {
  if (typeof value !== "object" || value === null) {
    return false;
  }
  const template = value as PromptTemplate;
  return (
    typeof template.id === "string" &&
    typeof template.name === "string" &&
    typeof template.body === "string" &&
    typeof template.negative === "string" &&
    isPlacement(template.placement)
  );
}

export async function loadTemplates(): Promise<PromptTemplate[]> {
  if (!isTauri()) {
    return [];
  }
  const value = await invoke("list_templates");
  return Array.isArray(value) ? value.filter(isTemplate) : [];
}

export async function createTemplate(draft: TemplateDraft): Promise<PromptTemplate> {
  const value = await invoke("create_template", { draft });
  if (!isTemplate(value)) {
    throw new Error("template inválido");
  }
  return value;
}

export async function updateTemplate(id: string, draft: TemplateDraft): Promise<PromptTemplate> {
  const value = await invoke("update_template", { id, draft });
  if (!isTemplate(value)) {
    throw new Error("template inválido");
  }
  return value;
}

export async function duplicateTemplate(id: string, name: string): Promise<PromptTemplate> {
  const value = await invoke("duplicate_template", { id, name });
  if (!isTemplate(value)) {
    throw new Error("template inválido");
  }
  return value;
}

export async function renameTemplate(id: string, name: string): Promise<PromptTemplate> {
  const value = await invoke("rename_template", { id, name });
  if (!isTemplate(value)) {
    throw new Error("template inválido");
  }
  return value;
}

export async function deleteTemplate(id: string): Promise<void> {
  if (!isTauri()) {
    return;
  }
  await invoke("delete_template", { id });
}
