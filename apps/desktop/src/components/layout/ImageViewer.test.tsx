import { fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { ImageViewer } from "./ImageViewer";

vi.mock("../../native/mesh", () => ({
  meshUrl: (path: string) => `mesh:${path}`,
}));

vi.mock("./MeshViewer", () => ({
  MeshViewer: ({
    url,
    wireframe,
    textured,
    onTriangles,
  }: {
    url: string | null;
    wireframe: boolean;
    textured: boolean;
    onTriangles: (count: number) => void;
  }) => (
    <div
      data-testid="mesh-viewer"
      data-url={url ?? ""}
      data-wireframe={String(wireframe)}
      data-textured={String(textured)}
    >
      <button type="button" onClick={() => onTriangles(12)}>
        contar
      </button>
    </div>
  ),
}));

describe("ImageViewer", () => {
  it("aproxima com ctrl e scroll, arrasta com o botão do meio, copia, salva e regera", async () => {
    const user = userEvent.setup();
    const onSave = vi.fn();
    const onCopy = vi.fn();
    const onRegenerate = vi.fn();
    render(
      <ImageViewer
        src="data:image/png;base64,aaaa"
        step={2}
        total={4}
        canSave
        canRegenerate
        onSave={onSave}
        onCopy={onCopy}
        onRegenerate={onRegenerate}
      />,
    );

    const stage = screen.getByTestId("viewer-stage");
    expect(screen.getByText("Zoom 100%")).toBeInTheDocument();
    fireEvent.wheel(stage, { ctrlKey: true, deltaY: -100 });
    expect(screen.getByText("Zoom 110%")).toBeInTheDocument();
    fireEvent.wheel(stage, { deltaY: -100 });
    expect(screen.getByText("Zoom 110%")).toBeInTheDocument();

    fireEvent.pointerDown(stage, { button: 1, clientX: 10, clientY: 10, pointerId: 1 });
    fireEvent.pointerMove(stage, { clientX: 30, clientY: 50, pointerId: 1 });
    expect(screen.getByRole("img").getAttribute("style")).toContain("20px");
    expect(screen.getByRole("img").getAttribute("style")).toContain("40px");

    await user.click(screen.getByRole("button", { name: "Salvar" }));
    await user.click(screen.getByRole("button", { name: "Copiar imagem" }));
    await user.click(screen.getByRole("button", { name: "Regerar com a mesma seed" }));
    expect(onSave).toHaveBeenCalledOnce();
    expect(onCopy).toHaveBeenCalledOnce();
    expect(onRegenerate).toHaveBeenCalledOnce();
  });

  it("volta e avança entre as imagens prontas", async () => {
    const user = userEvent.setup();
    const onPrevious = vi.fn();
    const onNext = vi.fn();
    const props = {
      step: 0,
      total: 0,
      canSave: true,
      canRegenerate: true,
      count: 2,
      onPrevious,
      onNext,
      onSave: vi.fn(),
      onCopy: vi.fn(),
      onRegenerate: vi.fn(),
    };
    const view = render(<ImageViewer {...props} src="data:image/png;base64,aaaa" index={0} />);

    expect(screen.getByText("1 / 2")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Anterior" })).toBeDisabled();
    await user.click(screen.getByRole("button", { name: "Próxima" }));
    expect(onNext).toHaveBeenCalledOnce();

    view.rerender(<ImageViewer {...props} src="data:image/png;base64,bbbb" index={1} />);
    expect(screen.getByRole("img")).toHaveAttribute("src", "data:image/png;base64,bbbb");
    expect(screen.getByText("2 / 2")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Próxima" })).toBeDisabled();
    await user.click(screen.getByRole("button", { name: "Anterior" }));
    expect(onPrevious).toHaveBeenCalledOnce();
  });

  it("mostra o xadrez e remove o fundo da imagem aberta", async () => {
    const user = userEvent.setup();
    const onRemoveBackground = vi.fn();
    const view = render(
      <ImageViewer
        src="data:image/png;base64,aaaa"
        step={0}
        total={0}
        canSave
        canRegenerate={false}
        canRemove
        transparent
        onSave={vi.fn()}
        onCopy={vi.fn()}
        onRegenerate={vi.fn()}
        onRemoveBackground={onRemoveBackground}
      />,
    );

    expect(screen.getByTestId("viewer-stage")).toHaveClass("transparency-grid");
    await user.click(screen.getByRole("button", { name: "Remover fundo" }));
    expect(onRemoveBackground).toHaveBeenCalledOnce();

    view.rerender(
      <ImageViewer
        src="data:image/png;base64,aaaa"
        step={0}
        total={0}
        canSave
        canRegenerate={false}
        canRemove
        removing
        transparent
        onSave={vi.fn()}
        onCopy={vi.fn()}
        onRegenerate={vi.fn()}
        onRemoveBackground={onRemoveBackground}
      />,
    );
    expect(screen.getByRole("button", { name: "Removendo fundo…" })).toBeDisabled();
  });

  it("orbita a malha, alterna arame e textura e conta os polígonos", async () => {
    const user = userEvent.setup();
    render(
      <ImageViewer
        src="data:image/png;base64,aaaa"
        step={0}
        total={0}
        canSave
        canRegenerate={false}
        meshFiles={{ glb: "/tmp/gato.glb", obj: "/tmp/gato.obj" }}
        onSave={vi.fn()}
        onCopy={vi.fn()}
        onRegenerate={vi.fn()}
      />,
    );

    const mesh = screen.getByTestId("mesh-viewer");
    expect(mesh).toHaveAttribute("data-url", "mesh:/tmp/gato.glb");
    expect(mesh).toHaveAttribute("data-textured", "true");
    expect(mesh).toHaveAttribute("data-wireframe", "false");
    expect(screen.getByText("Arraste para orbitar. A rolagem aproxima.")).toBeInTheDocument();
    expect(screen.getByText("GLB gato.glb · OBJ gato.obj")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Arame" }));
    expect(screen.getByTestId("mesh-viewer")).toHaveAttribute("data-wireframe", "true");
    await user.click(screen.getByRole("button", { name: "Textura" }));
    expect(screen.getByTestId("mesh-viewer")).toHaveAttribute("data-textured", "false");
    await user.click(screen.getByRole("button", { name: "contar" }));
    expect(screen.getByText("12 polígonos")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Ver imagem" }));
    expect(screen.queryByTestId("mesh-viewer")).not.toBeInTheDocument();
    expect(screen.getByRole("img")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Ver malha" }));
    expect(screen.getByTestId("mesh-viewer")).toBeInTheDocument();
  });

  it("mostra a cópia otimizada e o FBX ao lado do original", () => {
    render(
      <ImageViewer
        src="data:image/png;base64,aaaa"
        step={0}
        total={0}
        canSave
        canRegenerate={false}
        meshFiles={{
          glb: "/tmp/gato.glb",
          obj: "/tmp/gato.obj",
          optimizedGlb: "/tmp/gato_otimizado.glb",
          optimizedObj: "/tmp/gato_otimizado.obj",
          optimizedFbx: "/tmp/gato_otimizado.fbx",
        }}
        onSave={vi.fn()}
        onCopy={vi.fn()}
        onRegenerate={vi.fn()}
      />,
    );
    expect(screen.getByTestId("mesh-viewer")).toHaveAttribute(
      "data-url",
      "mesh:/tmp/gato_otimizado.glb",
    );
    expect(screen.getByText("GLB gato.glb · OBJ gato.obj")).toBeInTheDocument();
    expect(
      screen.getByText(
        "Otimizado GLB gato_otimizado.glb · OBJ gato_otimizado.obj · FBX gato_otimizado.fbx",
      ),
    ).toBeInTheDocument();
  });

  it("compara a versão anterior e esconde a edição quando é só exibição", async () => {
    const user = userEvent.setup();
    const onSelectVersion = vi.fn();
    render(
      <ImageViewer
        src="data:image/png;base64,nova"
        step={0}
        total={0}
        canSave
        canRegenerate
        canRemove
        versions={[
          { src: "data:image/png;base64,antiga", path: "/antiga.png", seed: 1 },
          { src: "data:image/png;base64,nova", path: "/nova.png", seed: 2 },
        ]}
        versionIndex={1}
        onSelectVersion={onSelectVersion}
        onRemoveBackground={vi.fn()}
        onSave={vi.fn()}
        onCopy={vi.fn()}
        onRegenerate={vi.fn()}
        viewOnly
      />,
    );
    expect(screen.queryByRole("button", { name: "Remover fundo" })).not.toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: "Regerar com a mesma seed" }),
    ).not.toBeInTheDocument();
    expect(screen.getByText("Versão 2 de 2")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Comparar" }));
    expect(screen.getByRole("img", { name: "Anterior" })).toHaveAttribute(
      "src",
      "data:image/png;base64,antiga",
    );
    expect(screen.getByRole("img", { name: "Esta" })).toHaveAttribute(
      "src",
      "data:image/png;base64,nova",
    );
    await user.click(screen.getByRole("button", { name: "Versão anterior" }));
    expect(onSelectVersion).toHaveBeenCalledWith(0);
  });

  it("compara a variação anterior lado a lado", async () => {
    const user = userEvent.setup();
    render(
      <ImageViewer
        src="data:image/png;base64,nova"
        previousSrc="data:image/png;base64,antiga"
        step={0}
        total={0}
        index={1}
        count={2}
        canSave={false}
        canRegenerate={false}
        onSave={vi.fn()}
        onCopy={vi.fn()}
        onRegenerate={vi.fn()}
        onPrevious={vi.fn()}
        onNext={vi.fn()}
      />,
    );
    await user.click(screen.getByRole("button", { name: "Comparar" }));
    expect(screen.getByRole("img", { name: "Anterior" })).toHaveAttribute(
      "src",
      "data:image/png;base64,antiga",
    );
    expect(screen.getByRole("img", { name: "Esta" })).toHaveAttribute(
      "src",
      "data:image/png;base64,nova",
    );
  });

  it("avisa que o prompt está sendo preparado antes do primeiro passo", () => {
    const { rerender } = render(
      <ImageViewer
        src={null}
        step={0}
        total={0}
        generating
        canSave={false}
        canRegenerate={false}
        onSave={vi.fn()}
        onCopy={vi.fn()}
        onRegenerate={vi.fn()}
      />,
    );
    expect(screen.getByText(/Preparando o prompt/)).toBeInTheDocument();

    rerender(
      <ImageViewer
        src="data:image/png;base64,aaaa"
        step={0}
        total={0}
        generating
        canSave={false}
        canRegenerate={false}
        onSave={vi.fn()}
        onCopy={vi.fn()}
        onRegenerate={vi.fn()}
      />,
    );
    expect(screen.getByText(/Preparando o prompt/)).toBeInTheDocument();

    rerender(
      <ImageViewer
        src="data:image/png;base64,aaaa"
        step={2}
        total={20}
        generating
        canSave={false}
        canRegenerate={false}
        onSave={vi.fn()}
        onCopy={vi.fn()}
        onRegenerate={vi.fn()}
      />,
    );
    expect(screen.queryByText(/Preparando o prompt/)).not.toBeInTheDocument();
    expect(screen.getByText("Passo 2 de 20")).toBeInTheDocument();
    expect(screen.queryByText(/Imagem \d de/)).not.toBeInTheDocument();
  });

  it("mostra qual imagem do lote está sendo gerada", () => {
    render(
      <ImageViewer
        src="data:image/png;base64,aaaa"
        step={5}
        total={20}
        image={2}
        images={4}
        generating
        canSave={false}
        canRegenerate={false}
        onSave={vi.fn()}
        onCopy={vi.fn()}
        onRegenerate={vi.fn()}
      />,
    );
    expect(screen.getAllByText("Imagem 2 de 4")).toHaveLength(2);
    expect(screen.getByText("Passo 5 de 20")).toBeInTheDocument();
  });
});
