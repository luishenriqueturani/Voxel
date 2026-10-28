type ChoiceModel = {
  id: string;
  name: string;
  installed: boolean;
  kind: string;
  architecture: string | null;
};

export function selectableModels(
  library: readonly ChoiceModel[],
  unknownLabel: string,
): { id: string; name: string }[] {
  return library
    .filter((model) => model.installed && model.kind !== "mesh" && model.kind !== "background")
    .map((model) => ({
      id: model.id,
      name:
        model.kind === "unknown" || model.architecture === "unknown"
          ? `${model.name} · ${unknownLabel}`
          : model.name,
    }));
}
