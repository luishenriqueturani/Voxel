import type { GenerationRequest } from "../native/generation";

export type EditMode = "new" | "vary" | "inpaint" | "tile";

export type EditPayload = {
  mode: EditMode;
  strength: number;
  sourcePath: string | null;
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
    return edit.sourcePath !== null;
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
): GenerationRequest {
  const edits = edit.mode === "vary" || edit.mode === "inpaint";
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
    mode: edit.mode,
    strength: edit.strength,
    sourcePath: edits ? edit.sourcePath : null,
    maskPng: edit.mode === "inpaint" ? edit.maskPng : null,
  };
}
