export type DebugRun = {
  modelId: string;
  family: string;
  backend: string;
  mode: string;
  edit: string;
  width: number;
  height: number;
  steps: number;
  cfg: number;
  seed: string;
  count: number;
  resolution: number;
  aspect: string;
};

let current: DebugRun | null = null;

export function rememberRun(run: DebugRun): void {
  current = run;
}

export function currentRun(): DebugRun | null {
  return current;
}

export function renderDebug(
  summary: string,
  log: string,
  run: DebugRun | null,
  when: Date,
): string {
  const stamp = new Intl.DateTimeFormat("pt-BR", {
    dateStyle: "short",
    timeStyle: "medium",
  }).format(when);
  const settings =
    run === null
      ? "Configuração: nenhuma geração nesta sessão\n"
      : `Modelo: ${run.modelId}
Família: ${run.family}
Backend: ${run.backend}
Modo: ${run.mode}
Edição: ${run.edit}
Largura: ${run.width}
Altura: ${run.height}
Passos: ${run.steps}
CFG: ${run.cfg}
Seed: ${run.seed}
Quantidade: ${run.count}
Resolução: ${run.resolution}
Proporção: ${run.aspect}
`;
  return `Voxel — debug
Quando: ${stamp}
Resumo: ${summary}
${settings}
--- erro ---
${log}
`;
}
