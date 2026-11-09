import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import {
  createTemplate,
  type PromptTemplate,
  type TemplateDraft,
  updateTemplate,
} from "../../native/templates";
import { TemplatesView } from "./TemplatesView";

const store = vi.hoisted(() => ({ rows: [] as PromptTemplate[] }));

function row(draft: TemplateDraft, id: string): PromptTemplate {
  return { ...draft, id };
}

vi.mock("../../native/templates", () => ({
  loadTemplates: vi.fn(async () => store.rows.slice()),
  createTemplate: vi.fn(async (draft: TemplateDraft) => {
    const created = row(draft, `id-${store.rows.length + 1}`);
    store.rows = [...store.rows, created];
    return created;
  }),
  updateTemplate: vi.fn(async (id: string, draft: TemplateDraft) => {
    const updated = row(draft, id);
    store.rows = store.rows.map((item) => (item.id === id ? updated : item));
    return updated;
  }),
  duplicateTemplate: vi.fn(async (id: string, name: string) => {
    const source = store.rows.find((item) => item.id === id);
    if (source === undefined) {
      throw new Error("ausente");
    }
    const copy = row({ ...source, name }, `id-${store.rows.length + 1}`);
    store.rows = [...store.rows, copy];
    return copy;
  }),
  renameTemplate: vi.fn(async (id: string, name: string) => {
    const source = store.rows.find((item) => item.id === id);
    if (source === undefined) {
      throw new Error("ausente");
    }
    const renamed = { ...source, name };
    store.rows = store.rows.map((item) => (item.id === id ? renamed : item));
    return renamed;
  }),
  deleteTemplate: vi.fn(async (id: string) => {
    store.rows = store.rows.filter((item) => item.id !== id);
  }),
}));

describe("biblioteca de templates", () => {
  it("abre o cadastro vazio, guia a ênfase e grava ao salvar", async () => {
    store.rows = [];
    const user = userEvent.setup();
    render(<TemplatesView onBack={vi.fn()} />);
    expect(screen.getByText("Nenhum template")).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Como escrever o template" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Ênfase: destacar uma parte" })).toBeInTheDocument();
    expect(screen.getByText(/Use \(ideia:1\.3\)/)).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Criar" }));
    expect(screen.getByRole("textbox", { name: "Nome" })).toHaveValue("");
    expect(screen.getByRole("textbox", { name: "Texto base" })).toHaveValue("");
    expect(screen.getByRole("textbox", { name: "Prompt negativo" })).toHaveValue("");
    expect(screen.queryByRole("button", { name: "Novo template" })).not.toBeInTheDocument();
    expect(createTemplate).not.toHaveBeenCalled();

    const body = screen.getByRole("textbox", { name: "Texto base" });
    await user.click(body);
    await user.paste("T-pose");
    await user.type(screen.getByRole("textbox", { name: "Item de exemplo" }), "ferreiro");
    expect(screen.getByText("ferreiro, T-pose")).toBeInTheDocument();

    await user.click(
      screen.getByRole("checkbox", { name: "Colocar o texto base antes do prompt da geração" }),
    );
    expect(screen.getByText("T-pose, ferreiro")).toBeInTheDocument();

    await user.type(screen.getByRole("textbox", { name: "Nome" }), "Personagem");
    await user.click(screen.getByRole("button", { name: "Salvar" }));
    expect(createTemplate).toHaveBeenCalledWith(
      expect.objectContaining({
        name: "Personagem",
        body: "T-pose",
        placement: "before",
      }),
    );
    expect(screen.getByRole("button", { name: "Personagem", pressed: true })).toBeInTheDocument();

    await user.clear(screen.getByRole("textbox", { name: "Nome" }));
    await user.type(screen.getByRole("textbox", { name: "Nome" }), "Herói");
    await user.click(screen.getByRole("button", { name: "Salvar" }));
    expect(updateTemplate).toHaveBeenCalledWith(
      "id-1",
      expect.objectContaining({ name: "Herói", body: "T-pose" }),
    );

    await user.click(screen.getByRole("button", { name: "Duplicar" }));
    expect(screen.getByRole("button", { name: "Herói cópia", pressed: true })).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Excluir" }));
    expect(screen.queryByRole("button", { name: "Herói cópia" })).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Herói", pressed: true })).toBeInTheDocument();
  });
});
