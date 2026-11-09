import { useEffect, useState } from "react";
import { composeGeneration } from "../lib/templatePrompt";
import { loadTemplates, type PromptTemplate } from "../native/templates";

export function usePromptTemplates(prompt: string, negativePrompt: string, reloadKey: string) {
  const [templates, setTemplates] = useState<PromptTemplate[]>([]);
  const [templateId, setTemplateId] = useState("");

  useEffect(() => {
    if (reloadKey !== "workspace") {
      return;
    }
    void loadTemplates().then(setTemplates);
  }, [reloadKey]);

  const selected = templates.find((entry) => entry.id === templateId) ?? null;
  const composed =
    selected === null
      ? { prompt, negative: negativePrompt }
      : composeGeneration(selected, prompt, negativePrompt);

  return { templates, templateId, setTemplateId, composed };
}
