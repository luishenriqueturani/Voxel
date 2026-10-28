import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { type PromptTemplate, type TemplateDraft, updateTemplate } from "../../native/templates";
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
  it("cria, duplica, renomeia, exclui e monta o prompt do item", async () => {
    store.rows = [];
    const user = userEvent.setup();
    render(<TemplatesView onBack={vi.fn()} />);
    expect(screen.getByText("Nenhum template")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Criar" }));
    expect(
      screen.getByRole("button", { name: "Novo template", pressed: true }),
    ).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Prompt final" })).toBeInTheDocument();
    expect(screen.getByText("ferreiro")).toBeInTheDocument();

    const body = screen.getByRole("textbox", { name: "Texto base" });
    await user.clear(body);
    await user.click(body);
    await user.paste("{prompt}, T-pose");
    expect(screen.getByText("ferreiro, T-pose")).toBeInTheDocument();

    const name = screen.getByRole("textbox", { name: "Nome" });
    await user.clear(name);
    await user.type(name, "Personagem");
    await user.click(screen.getByRole("button", { name: "Renomear" }));
    expect(screen.getByRole("button", { name: "Personagem", pressed: true })).toBeInTheDocument();
    await user.click(screen.getByRole("checkbox", { name: "Pós-processamento 3D" }));
    await user.click(screen.getByRole("button", { name: "Salvar" }));
    expect(updateTemplate).toHaveBeenCalledWith(
      "id-1",
      expect.objectContaining({
        name: "Personagem",
        body: "{prompt}, T-pose",
        model2d: "sd15",
        model3d: "triposr",
        postprocess: true,
      }),
    );

    await user.click(screen.getByRole("button", { name: "Duplicar" }));
    expect(
      screen.getByRole("button", { name: "Personagem cópia", pressed: true }),
    ).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Excluir" }));
    expect(screen.queryByRole("button", { name: "Personagem cópia" })).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Personagem", pressed: true })).toBeInTheDocument();
  });
});
