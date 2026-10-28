import type { GenerationRequest } from "../native/generation";

export type EditMode = "new" | "vary" | "inpaint" | "tile";

export type EditPayload = {
  mode: EditMode;
  strength: number;
  sourcePath: string | null;
  referencePath?: string | null;
  maskPng: string | null;
  instruction: string;
};

export const blankEdit: EditPayload = {
  mode: "new",
  strength: 0.75,
  sourcePath: null,
  maskPng: null,
  instruction: "",
};

export function canStartEdit(edit: EditPayload): boolean {
  if (edit.mode === "vary") {
    return edit.sourcePath !== null || (edit.referencePath ?? null) !== null;
  }
  if (edit.mode === "inpaint") {
    return edit.sourcePath !== null && edit.maskPng !== null;
  }
  return true;
}

const CHANGE_WEIGHT = "1.6";

function literalPrompt(text: string): string {
  return text.replace(/[\\()[\]]/g, "\\$&").replaceAll("BREAK", "break");
}

function promptForEdit(prompt: string, edit: EditPayload): string {
  if (edit.mode !== "vary" && edit.mode !== "inpaint") {
    return prompt;
  }
  const change = edit.instruction.trim();
  if (change === "") {
    return prompt;
  }
  const lead = `(${literalPrompt(change)}:${CHANGE_WEIGHT})`;
  const base = prompt.trim();
  if (base === "") {
    return lead;
  }
  return `${lead} BREAK [${literalPrompt(base)}]`;
}

type FrameParams = {
  width: number;
  height: number;
  steps: number;
  cfg: number;
  count: number;
};

export function generationRequest(
  prompt: string,
  negativePrompt: string,
  modelId: string,
  params: FrameParams,
  seed: number | null,
  edit: EditPayload,
  backend?: string,
): GenerationRequest {
  const image = requestImage(edit);
  return {
    prompt: promptForEdit(prompt, edit),
    negativePrompt: negativePrompt === "" ? null : negativePrompt,
    modelId,
    width: params.width,
    height: params.height,
    steps: params.steps,
    cfg: params.cfg,
    seed,
    count: params.count,
    mode: image.mode,
    strength: edit.strength,
    sourcePath: image.sourcePath,
    maskPng: edit.mode === "inpaint" ? edit.maskPng : null,
    ...(backend === undefined || backend === "" ? {} : { backend }),
  };
}

function requestImage(edit: EditPayload): { mode: EditMode; sourcePath: string | null } {
  if (edit.mode === "tile") {
    return { mode: "tile", sourcePath: null };
  }
  if (edit.mode === "inpaint") {
    return { mode: "inpaint", sourcePath: edit.sourcePath };
  }
  const reference = edit.referencePath ?? null;
  if (reference !== null) {
    return { mode: "vary", sourcePath: reference };
  }
  if (edit.mode === "vary") {
    return { mode: "vary", sourcePath: edit.sourcePath };
  }
  return { mode: "new", sourcePath: null };
}
