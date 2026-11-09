import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import type { QueueJob } from "../../native/queue";
import { QueuePicks } from "./QueuePicks";

const job: QueueJob = {
  id: "job-1",
  name: "NPCs",
  status: "done",
  endRetry: false,
  items: [
    {
      id: "a",
      status: "done",
      prompt: "ferreiro",
      count: 2,
      mesh: "later",
      images: 2,
      meshes: 0,
      imagePaths: ["/tmp/a.png", "/tmp/b.png"],
    },
  ],
};

const native = vi.hoisted(() => ({
  enqueueMeshPicks: vi.fn(async () => ({ ...job, status: "pending", name: "NPCs · 3D" })),
  loadQueueThumbnails: vi.fn(async () => ({})),
}));

vi.mock("../../native/queue", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../../native/queue")>();
  return {
    ...actual,
    enqueueMeshPicks: native.enqueueMeshPicks,
    loadQueueThumbnails: native.loadQueueThumbnails,
  };
});

describe("escolha para o 3D", () => {
  it("envia só as imagens marcadas", async () => {
    const user = userEvent.setup();
    const onQueued = vi.fn();
    render(<QueuePicks job={job} onQueued={onQueued} onFailure={() => undefined} />);
    expect(screen.getByRole("heading", { name: "Escolher imagens para o 3D" })).toBeInTheDocument();
    const send = screen.getByRole("button", { name: "Enviar para 3D" });
    expect(send).toBeDisabled();
    await user.click(screen.getByRole("checkbox", { name: "ferreiro · variação 2" }));
    expect(send).toBeEnabled();
    await user.click(send);
    expect(native.enqueueMeshPicks).toHaveBeenCalledWith("job-1", [
      { itemId: "a", paths: ["/tmp/b.png"] },
    ]);
    expect(onQueued).toHaveBeenCalled();
  });
});
