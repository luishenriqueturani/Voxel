import { fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import type { QueueJob } from "../../native/queue";
import { FlowPreview } from "./FlowPreview";

const smith: QueueJob["items"][number] = {
  id: "a",
  status: "done",
  prompt: "ferreiro",
  count: 2,
  mesh: "none",
  images: 2,
  meshes: 0,
  imagePaths: ["/tmp/a.png", "/tmp/b.png"],
};

const running: QueueJob = {
  id: "job-1",
  name: "f1",
  status: "running",
  endRetry: false,
  items: [
    smith,
    {
      id: "b",
      status: "images",
      prompt: "guarda",
      count: 1,
      mesh: "none",
      images: 0,
      meshes: 0,
    },
  ],
};

const finished: QueueJob = {
  ...running,
  status: "done",
  items: [
    smith,
    {
      id: "b",
      status: "done",
      prompt: "guarda",
      count: 1,
      mesh: "none",
      images: 1,
      meshes: 0,
      imagePaths: ["/tmp/c.png"],
    },
  ],
};

const store = vi.hoisted(() => ({
  job: null as QueueJob | null,
}));

vi.mock("../../native/queue", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../../native/queue")>();
  return {
    ...actual,
    loadUnfinishedQueueJob: vi.fn(async () => {
      if (store.job === null || store.job.status === "done" || store.job.status === "cancelled") {
        return null;
      }
      return store.job;
    }),
    loadLatestQueueJob: vi.fn(async () => store.job),
    loadQueueThumbnails: vi.fn(async (paths: string[]) =>
      Object.fromEntries(paths.map((path) => [path, `data:image/jpeg;base64,${path}`])),
    ),
  };
});

describe("prévia da fila no fluxo", () => {
  it("pede para executar quando não há fila", async () => {
    store.job = null;
    render(<FlowPreview queued={false} />);
    expect(await screen.findByRole("complementary", { name: "Prévia" })).toBeInTheDocument();
    expect(
      screen.getByText("Execute o fluxo para ver o progresso e as imagens."),
    ).toBeInTheDocument();
  });

  it("mostra o progresso e o item em geração", async () => {
    store.job = running;
    render(<FlowPreview queued />);
    expect(await screen.findByText("1/2")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Item 1: ferreiro/ })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Item 2: guarda/ })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
    expect(screen.getByText(/Imagens · 0\/1/)).toBeInTheDocument();
  });

  it("depois de terminar deixa escolher o item e ver as imagens", async () => {
    store.job = finished;
    const user = userEvent.setup();
    render(<FlowPreview queued={false} />);
    expect(await screen.findByText("2/2")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: /Item 1: ferreiro/ }));
    expect(await screen.findByRole("img", { name: "Prévia da geração" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Imagem 1" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Imagem 2" })).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: /Item 2: guarda/ }));
    expect(screen.queryByRole("button", { name: "Imagem 2" })).not.toBeInTheDocument();
    expect(screen.getByRole("img", { name: "Prévia da geração" })).toHaveAttribute(
      "src",
      "data:image/jpeg;base64,/tmp/c.png",
    );
  });

  it("aproxima com ctrl e scroll e arrasta com o botão do meio", async () => {
    store.job = finished;
    const user = userEvent.setup();
    render(<FlowPreview queued={false} />);
    await user.click(await screen.findByRole("button", { name: /Item 1: ferreiro/ }));
    const stage = await screen.findByTestId("viewer-stage");
    expect(screen.getByText(/Zoom 100%/)).toBeInTheDocument();
    fireEvent.wheel(stage, { ctrlKey: true, deltaY: -100 });
    expect(screen.getByText(/Zoom 110%/)).toBeInTheDocument();
    fireEvent.pointerDown(stage, { button: 1, clientX: 10, clientY: 10, pointerId: 1 });
    fireEvent.pointerMove(stage, { clientX: 30, clientY: 50, pointerId: 1 });
    expect(screen.getByRole("img", { name: "Prévia da geração" }).getAttribute("style")).toContain(
      "20px",
    );
    expect(screen.getByRole("img", { name: "Prévia da geração" }).getAttribute("style")).toContain(
      "40px",
    );
  });
});
