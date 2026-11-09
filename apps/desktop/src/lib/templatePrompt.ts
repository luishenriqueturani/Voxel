export type PromptPlacement = "before" | "after";

export type TemplateSource = {
  body: string;
  negative: string;
  placement?: PromptPlacement;
};

export type ComposedGeneration = {
  prompt: string;
  negative: string;
};

const MARKER = "{prompt}";

function joinParts(left: string, right: string): string {
  const first = left.trim();
  const second = right.trim();
  if (first === "") {
    return second;
  }
  if (second === "") {
    return first;
  }
  return `${first}, ${second}`;
}

export function composePrompt(
  body: string,
  itemPrompt: string,
  placement: PromptPlacement = "after",
): string {
  if (body.includes(MARKER)) {
    return body.replaceAll(MARKER, itemPrompt);
  }
  return placement === "before" ? joinParts(body, itemPrompt) : joinParts(itemPrompt, body);
}

export function composeNegative(templateNegative: string, itemNegative: string): string {
  return joinParts(templateNegative, itemNegative);
}

export function extraNegative(templateNegative: string, composed: string): string {
  const prefix = templateNegative.trim();
  const full = composed.trim();
  if (prefix === "") {
    return full;
  }
  if (full === prefix) {
    return "";
  }
  const withComma = `${prefix}, `;
  if (full.startsWith(withComma)) {
    return full.slice(withComma.length).trim();
  }
  return full;
}

export function composeGeneration(
  template: TemplateSource,
  itemPrompt: string,
  itemNegative = "",
): ComposedGeneration {
  return {
    prompt: composePrompt(template.body, itemPrompt, template.placement ?? "after"),
    negative: composeNegative(template.negative, itemNegative),
  };
}
