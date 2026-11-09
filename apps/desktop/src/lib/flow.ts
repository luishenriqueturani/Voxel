import { composeGeneration } from "./templatePrompt";

export const FLOW_FILE_VERSION = 1;
export const MESH_MODES = ["none", "all", "later"] as const;
export const FLOW_RESOLUTIONS = [512, 768, 1024] as const;

export type MeshMode = (typeof MESH_MODES)[number];

export type FlowOverrides = {
  negative?: string;
  model2d?: string;
  model3d?: string;
  resolution?: number;
  aspect?: string;
  steps?: number;
  cfg?: number;
  seed?: string;
  postprocess?: boolean;
};

export type FlowItem = {
  id: string;
  prompt: string;
  count: number;
  mesh: MeshMode;
  overrides: FlowOverrides;
};

export type FlowDraft = {
  name: string;
  templateId: string | null;
  path: string | null;
  items: FlowItem[];
};

export type FlowDocument = FlowDraft & { id: string };

export type FlowTemplate = {
  body: string;
  negative: string;
  model2d: string;
  model3d: string;
  resolution: number;
  aspect: string;
  steps: number;
  cfg: number;
  seed: string;
  postprocess: boolean;
};

export type ResolvedFlowItem = FlowTemplate & {
  prompt: string;
  count: number;
  mesh: MeshMode;
};

export type FlowFilePayload = {
  version: number;
  name: string;
  templateId: string | null;
  items: FlowItem[];
};

export type FlowMenuKind = "idle" | "new" | "open" | "save";

export type FlowMenuCommand = {
  kind: FlowMenuKind;
  n: number;
};

export const IDLE_FLOW_COMMAND: FlowMenuCommand = { kind: "idle", n: 0 };

const FALLBACK_TEMPLATE: FlowTemplate = {
  body: "{prompt}",
  negative: "",
  model2d: "",
  model3d: "",
  resolution: 512,
  aspect: "1:1",
  steps: 20,
  cfg: 7,
  seed: "",
  postprocess: false,
};

export function defaultMeshMode(_count: number): MeshMode {
  return "none";
}

export function needsAllConfirmation(count: number, mesh: MeshMode): boolean {
  return count > 1 && mesh === "all";
}

export function clampItemCount(value: number): number {
  if (!Number.isFinite(value)) {
    return 1;
  }
  return Math.min(64, Math.max(1, Math.floor(value)));
}

export function blankItem(id: string, count = 1): FlowItem {
  const nextCount = clampItemCount(count);
  return {
    id,
    prompt: "",
    count: nextCount,
    mesh: defaultMeshMode(nextCount),
    overrides: {},
  };
}

export function blankFlow(name: string, itemId: string): FlowDraft {
  return {
    name,
    templateId: null,
    path: null,
    items: [blankItem(itemId)],
  };
}

export function moveItem<T>(items: readonly T[], from: number, to: number): T[] {
  if (from === to || from < 0 || to < 0 || from >= items.length || to >= items.length) {
    return items.slice();
  }
  const next = items.slice();
  const [removed] = next.splice(from, 1);
  if (removed === undefined) {
    return items.slice();
  }
  next.splice(to, 0, removed);
  return next;
}

export function isMeshMode(value: unknown): value is MeshMode {
  return value === "none" || value === "all" || value === "later";
}

export function resolveItem(template: FlowTemplate | null, item: FlowItem): ResolvedFlowItem {
  const base: FlowTemplate = { ...FALLBACK_TEMPLATE, ...(template ?? {}) };
  const overrides = item.overrides;
  const composed = composeGeneration(
    { body: base.body, negative: overrides.negative ?? base.negative },
    item.prompt,
  );
  return {
    body: base.body,
    prompt: composed.prompt,
    negative: composed.negative,
    model2d: overrides.model2d ?? base.model2d,
    model3d: overrides.model3d ?? base.model3d,
    resolution: overrides.resolution ?? base.resolution,
    aspect: overrides.aspect ?? base.aspect,
    steps: overrides.steps ?? base.steps,
    cfg: overrides.cfg ?? base.cfg,
    seed: overrides.seed ?? base.seed,
    postprocess: overrides.postprocess ?? base.postprocess,
    count: item.count,
    mesh: item.mesh,
  };
}

export function serializeFlowFile(flow: Pick<FlowDraft, "name" | "templateId" | "items">): string {
  const payload: FlowFilePayload = {
    version: FLOW_FILE_VERSION,
    name: flow.name,
    templateId: flow.templateId,
    items: flow.items.map((item) => ({
      id: item.id,
      prompt: item.prompt,
      count: item.count,
      mesh: item.mesh,
      overrides: { ...item.overrides },
    })),
  };
  return `${JSON.stringify(payload, null, 2)}\n`;
}

export function parseFlowFile(text: string): FlowDraft {
  const value: unknown = JSON.parse(text);
  if (typeof value !== "object" || value === null) {
    throw new Error("arquivo de fluxo inválido");
  }
  const file = value as Record<string, unknown>;
  if (file.version !== FLOW_FILE_VERSION) {
    throw new Error("versão de fluxo desconhecida");
  }
  if (typeof file.name !== "string" || file.name.trim() === "") {
    throw new Error("nome de fluxo inválido");
  }
  const templateId =
    file.templateId === null || file.templateId === undefined
      ? null
      : typeof file.templateId === "string"
        ? file.templateId
        : null;
  if (!Array.isArray(file.items)) {
    throw new Error("itens de fluxo inválidos");
  }
  return {
    name: file.name.trim(),
    templateId,
    path: null,
    items: file.items.map((entry, index) => readItem(entry, index)),
  };
}

function readItem(value: unknown, index: number): FlowItem {
  if (typeof value !== "object" || value === null) {
    throw new Error("item de fluxo inválido");
  }
  const item = value as Record<string, unknown>;
  const count = clampItemCount(typeof item.count === "number" ? item.count : 1);
  if (!isMeshMode(item.mesh)) {
    throw new Error("modo 3D inválido");
  }
  return {
    id: typeof item.id === "string" && item.id !== "" ? item.id : `item-${index + 1}`,
    prompt: typeof item.prompt === "string" ? item.prompt : "",
    count,
    mesh: item.mesh,
    overrides: readOverrides(item.overrides),
  };
}

function readOverrides(value: unknown): FlowOverrides {
  if (value === undefined) {
    return {};
  }
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new Error("sobrescrita inválida");
  }
  const raw = value as Record<string, unknown>;
  const overrides: FlowOverrides = {};
  if (typeof raw.negative === "string") {
    overrides.negative = raw.negative;
  }
  if (typeof raw.model2d === "string") {
    overrides.model2d = raw.model2d;
  }
  if (typeof raw.model3d === "string") {
    overrides.model3d = raw.model3d;
  }
  if (raw.resolution === 512 || raw.resolution === 768 || raw.resolution === 1024) {
    overrides.resolution = raw.resolution;
  }
  if (typeof raw.aspect === "string") {
    overrides.aspect = raw.aspect;
  }
  if (typeof raw.steps === "number" && Number.isFinite(raw.steps)) {
    overrides.steps = raw.steps;
  }
  if (typeof raw.cfg === "number" && Number.isFinite(raw.cfg)) {
    overrides.cfg = raw.cfg;
  }
  if (typeof raw.seed === "string") {
    overrides.seed = raw.seed;
  }
  if (typeof raw.postprocess === "boolean") {
    overrides.postprocess = raw.postprocess;
  }
  return overrides;
}
