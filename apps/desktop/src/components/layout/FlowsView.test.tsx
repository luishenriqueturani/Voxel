import { fireEvent, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { blankItem, type FlowDocument, type FlowDraft, IDLE_FLOW_COMMAND } from "../../lib/flow";
import { updateFlow } from "../../native/flows";
import { FlowsView } from "./FlowsView";

const store = vi.hoisted(() => ({
  rows: [] as FlowDocument[],
  templates: [] as { id: string; name: string; body: string; count: number }[],
}));

function documentOf(draft: FlowDraft, id: string): FlowDocument {
  return { ...draft, id };
}

vi.mock("../../native/flows", () => ({
  loadFlows: vi.fn(async () => store.rows.slice()),
  createFlow: vi.fn(async (draft: FlowDraft) => {
    const created = documentOf(draft, `flow-${store.rows.length + 1}`);
    store.rows = [...store.rows, created];
    return created;
  }),
  updateFlow: vi.fn(async (id: string, draft: FlowDraft) => {
    const updated = documentOf(draft, id);
    store.rows = store.rows.map((flow) => (flow.id === id ? updated : flow));
    return updated;
  }),
  deleteFlow: vi.fn(async (id: string) => {
    store.rows = store.rows.filter((flow) => flow.id !== id);
  }),
  pickFlowToOpen: vi.fn(async () => "/tmp/vila.json"),
  pickFlowToSave: vi.fn(async () => "/tmp/vila.json"),
  importFlowFile: vi.fn(async () => {
    const imported: FlowDocument = {
      id: "imported",
      name: "NPCs da vila",
      templateId: null,
      path: "/tmp/vila.json",
      items: [
        {
          id: "i1",
          prompt: "aldeã",
          count: 4,
          mesh: "later",
          overrides: { seed: "1234" },
        },
      ],
    };
    store.rows = [...store.rows.filter((flow) => flow.id !== imported.id), imported];
    return imported;
  }),
  exportFlowFile: vi.fn(async (id: string, path: string) => {
    const current = store.rows.find((flow) => flow.id === id);
    if (current === undefined) {
      throw new Error("ausente");
    }
    const saved = { ...current, path };
    store.rows = store.rows.map((flow) => (flow.id === id ? saved : flow));
    return saved;
  }),
}));

vi.mock("../../native/templates", () => ({
  loadTemplates: vi.fn(async () => store.templates.slice()),
}));

describe("editor de fluxos", () => {
  it("cria itens, reordena, confirma Todas e sobrescreve a seed", async () => {
    store.rows = [];
    store.templates = [{ id: "tpl", name: "Personagem", body: "{prompt}, T-pose", count: 1 }];
    const user = userEvent.setup();
    render(<FlowsView onBack={vi.fn()} command={IDLE_FLOW_COMMAND} />);

    expect(screen.getByText("Nenhum fluxo")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Criar" }));
    expect(screen.getByRole("button", { name: "Novo fluxo", pressed: true })).toBeInTheDocument();

    await user.selectOptions(screen.getByRole("combobox", { name: "Template" }), "tpl");
    await user.type(screen.getByRole("textbox", { name: "Prompt do item" }), "ferreiro");
    expect(screen.getByText("Prompt final: ferreiro, T-pose")).toBeInTheDocument();

    fireEvent.change(screen.getByRole("spinbutton", { name: "Quantidade de imagens" }), {
      target: { value: "4" },
    });
    expect(screen.getByRole("spinbutton", { name: "Quantidade de imagens" })).toHaveValue(4);
    expect(screen.getByRole("radio", { name: "Não" })).toBeChecked();

    await user.click(screen.getByRole("radio", { name: "Todas" }));
    expect(
      screen.getByText("Este item tem 4 imagens. Gerar 3D de cada variação leva bem mais tempo."),
    ).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Cancelar" }));
    expect(screen.getByRole("radio", { name: "Não" })).toBeChecked();

    await user.click(screen.getByRole("radio", { name: "Todas" }));
    await user.click(screen.getByRole("button", { name: "Gerar todas" }));
    expect(screen.getByRole("radio", { name: "Todas" })).toBeChecked();

    await user.click(screen.getByRole("checkbox", { name: "Sobrescrever Seed" }));
    await user.type(await screen.findByRole("textbox", { name: "Seed" }), "1234");
    expect(updateFlow).toHaveBeenCalledWith(
      "flow-1",
      expect.objectContaining({
        items: expect.arrayContaining([
          expect.objectContaining({
            count: 4,
            mesh: "all",
            overrides: expect.objectContaining({ seed: "1234" }),
          }),
        ]),
      }),
    );

    await user.click(screen.getByRole("button", { name: "Adicionar item" }));
    expect(screen.getAllByRole("article")).toHaveLength(2);
    const first = screen.getAllByRole("article")[0];
    if (first === undefined) {
      throw new Error("item ausente");
    }
    await user.click(within(first).getByRole("button", { name: "Descer" }));
    expect(screen.getAllByRole("textbox", { name: "Prompt do item" })[1]).toHaveValue("ferreiro");
  });

  it("abre e salva o arquivo do fluxo", async () => {
    store.rows = [
      {
        id: "flow-1",
        name: "Rascunho",
        templateId: null,
        path: null,
        items: [blankItem("a")],
      },
    ];
    const user = userEvent.setup();
    render(<FlowsView onBack={vi.fn()} command={{ kind: "open", n: 1 }} />);
    expect(
      await screen.findByRole("button", { name: "NPCs da vila", pressed: true }),
    ).toBeInTheDocument();
    expect(screen.getByText("Arquivo: /tmp/vila.json")).toBeInTheDocument();
    expect(screen.getByRole("textbox", { name: "Prompt do item" })).toHaveValue("aldeã");

    await user.click(screen.getByRole("button", { name: "Salvar arquivo" }));
    expect(screen.getByText("Arquivo: /tmp/vila.json")).toBeInTheDocument();
  });
});
