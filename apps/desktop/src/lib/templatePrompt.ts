export type TemplateSource = {
  body: string;
  negative: string;
};

export type ComposedGeneration = {
  prompt: string;
  negative: string;
};

const MARKER = "{prompt}";

export function composePrompt(body: string, itemPrompt: string): string {
  return body.replaceAll(MARKER, itemPrompt);
}

export function composeGeneration(
  template: TemplateSource,
  itemPrompt: string,
): ComposedGeneration {
  return {
    prompt: composePrompt(template.body, itemPrompt),
    negative: template.negative,
  };
}
