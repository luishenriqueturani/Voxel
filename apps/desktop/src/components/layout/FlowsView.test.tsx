import { fireEvent, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { blankItem, type FlowDocument, type FlowDraft, IDLE_FLOW_COMMAND } from "../../lib/flow";
import { updateFlow } from "../../native/flows";
import type { ListedModel } from "../../native/models";
import { enqueueFlow } from "../../native/queue";
import { FlowsView } from "./FlowsView";

const library: ListedModel[] = [
  {
    id: "sd15",
    name: "Stable Diffusion 1.5",
    license: "MIT",
    bytes: 1,
    filename: "sd15",
    installed: true,
    catalog: true,
    compatibility: "compatible",
    reason: null,
    architecture: "sd15",
    kind: "image",
  },
  {
    id: "triposr",
    name: "TripoSR",
    license: "MIT",
    bytes: 1,
    filename: "triposr",
    installed: true,
    catalog: true,
    compatibility: "compatible",
    reason: null,
    architecture: null,
    kind: "mesh",
  },
];

const store = vi.hoisted(() => ({
  rows: [] as FlowDocument[],
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
          templateId: null,
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
  loadTemplates: vi.fn(async () => [
    {
      id: "tpl-1",
      name: "Personagem",
      body: "T-pose",
      negative: "fundo",
      placement: "after",
    },
  ]),
}));

vi.mock("../../native/queue", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../../native/queue")>();
  return {
    ...actual,
    enqueueFlow: vi.fn(async (id: string) => ({
      id: `job-${id}`,
      name: id,
      status: "pending",
      endRetry: false,
      items: [],
    })),
    loadUnfinishedQueueJob: vi.fn(async () => null),
  };
});

describe("editor de fluxos", () => {
  beforeEach(() => {
    vi.mocked(enqueueFlow).mockClear();
  });

  it("cria itens, escolhe 3D e os parâmetros sem checkbox", async () => {
    store.rows = [];
    const user = userEvent.setup();
    render(<FlowsView onBack={vi.fn()} command={IDLE_FLOW_COMMAND} library={library} />);

    expect(screen.getByText("Nenhum fluxo")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Criar" }));
    expect(screen.getByRole("button", { name: "Novo fluxo", pressed: true })).toBeInTheDocument();

    await user.type(screen.getByRole("textbox", { name: "Prompt do item" }), "ferreiro");
    expect(screen.getByRole("textbox", { name: "Prompt do item" })).toHaveValue("ferreiro");
    expect(screen.getByRole("combobox", { name: "Modelo" })).toHaveValue("sd15");
    expect(screen.getByRole("combobox", { name: "Resolução" })).toHaveValue("512");
    expect(screen.queryByRole("combobox", { name: "Modelo 3D" })).not.toBeInTheDocument();
    expect(screen.queryByRole("textbox", { name: "Seed" })).not.toBeInTheDocument();

    fireEvent.change(screen.getByRole("spinbutton", { name: "Quantidade de imagens" }), {
      target: { value: "4" },
    });
    expect(screen.getByRole("spinbutton", { name: "Quantidade de imagens" })).toHaveValue(4);
    expect(screen.getByRole("radio", { name: "2D" })).toBeChecked();

    await user.click(screen.getByRole("radio", { name: "3D" }));
    expect(screen.getByRole("radio", { name: "Escolher depois" })).toBeChecked();
    expect(screen.getByRole("combobox", { name: "Modelo 3D" })).toHaveValue("triposr");
    expect(screen.queryByRole("combobox", { name: "Modelo" })).not.toBeInTheDocument();
    expect(screen.queryByRole("combobox", { name: "Resolução" })).not.toBeInTheDocument();
    expect(screen.queryByRole("combobox", { name: "Proporção" })).not.toBeInTheDocument();
    expect(screen.queryByRole("spinbutton", { name: "Passos" })).not.toBeInTheDocument();
    expect(screen.queryByRole("spinbutton", { name: "CFG" })).not.toBeInTheDocument();
    expect(
      screen.queryByRole("spinbutton", { name: "Quantidade de imagens" }),
    ).not.toBeInTheDocument();

    await user.click(screen.getByRole("radio", { name: "Todas" }));
    expect(
      screen.getByText("Este item tem 4 imagens. Gerar 3D de cada variação leva bem mais tempo."),
    ).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Cancelar" }));
    expect(screen.getByRole("radio", { name: "Escolher depois" })).toBeChecked();

    await user.click(screen.getByRole("radio", { name: "Todas" }));
    await user.click(screen.getByRole("button", { name: "Gerar todas" }));
    expect(screen.getByRole("radio", { name: "Todas" })).toBeChecked();
    expect(updateFlow).toHaveBeenCalledWith(
      "flow-1",
      expect.objectContaining({
        items: expect.arrayContaining([
          expect.objectContaining({
            count: 4,
            mesh: "all",
          }),
        ]),
      }),
    );

    await user.click(screen.getByRole("radio", { name: "2D" }));
    await user.selectOptions(screen.getByRole("combobox", { name: "Resolução" }), "1024");
    expect(updateFlow).toHaveBeenCalledWith(
      "flow-1",
      expect.objectContaining({
        items: expect.arrayContaining([
          expect.objectContaining({
            overrides: expect.objectContaining({ resolution: 1024 }),
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

  it("enfileira o fluxo sem abrir o modal de erro", async () => {
    store.rows = [];
    const user = userEvent.setup();
    render(<FlowsView onBack={vi.fn()} command={IDLE_FLOW_COMMAND} />);
    await user.click(screen.getByRole("button", { name: "Criar" }));
    await user.click(screen.getByRole("button", { name: "Executar" }));
    expect(enqueueFlow).toHaveBeenCalledWith("flow-1");
    expect(
      screen.getByText("Enfileirado. A fila continua se o aplicativo fechar."),
    ).toBeInTheDocument();
    expect(screen.queryByRole("alertdialog")).not.toBeInTheDocument();
  });

  it("escolhe o template e mostra o prompt composto", async () => {
    store.rows = [];
    const user = userEvent.setup();
    render(<FlowsView onBack={vi.fn()} command={IDLE_FLOW_COMMAND} />);
    await user.click(screen.getByRole("button", { name: "Criar" }));
    const article = screen.getByRole("article");
    await within(article).findByRole("option", { name: "Personagem" });
    await user.selectOptions(within(article).getByRole("combobox", { name: "Template" }), "tpl-1");
    expect(updateFlow).toHaveBeenCalledWith(
      "flow-1",
      expect.objectContaining({
        items: [expect.objectContaining({ templateId: "tpl-1" })],
      }),
    );
    expect(within(article).getByRole("textbox", { name: "Prompt negativo" })).toHaveValue("fundo");
    await user.type(within(article).getByRole("textbox", { name: "Prompt do item" }), "ferreiro");
    expect(within(article).getByText("ferreiro, T-pose")).toBeInTheDocument();
  });

  it("pede itens antes de enfileirar um fluxo vazio", async () => {
    store.rows = [];
    const user = userEvent.setup();
    render(<FlowsView onBack={vi.fn()} command={IDLE_FLOW_COMMAND} />);
    await user.click(screen.getByRole("button", { name: "Criar" }));
    await user.click(within(screen.getByRole("article")).getByRole("button", { name: "Remover" }));
    expect(screen.queryByRole("article")).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Executar" }));
    expect(enqueueFlow).not.toHaveBeenCalled();
    expect(screen.getByRole("alert")).toHaveTextContent("Adicione itens antes de executar.");
  });
});
