import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import type { QueueJob } from "../../native/queue";
import { QueueReport } from "./QueueReport";

vi.mock("../../native/files", () => ({
  openLocalPath: vi.fn(async () => undefined),
}));

import { openLocalPath } from "../../native/files";

const job: QueueJob = {
  id: "job-1",
  name: "NPCs",
  status: "done",
  endRetry: false,
  createdAt: "2026-11-09T10:00:00Z",
  updatedAt: "2026-11-09T12:10:00Z",
  items: [
    {
      id: "a",
      status: "done",
      prompt: "ferreiro",
      count: 1,
      mesh: "none",
      images: 1,
      meshes: 0,
    },
    {
      id: "b",
      status: "failed",
      prompt: "guarda",
      count: 1,
      mesh: "all",
      images: 0,
      meshes: 0,
      errorLog: "/tmp/guarda.log",
    },
  ],
};

describe("relatório da fila", () => {
  it("mostra totais, tempo e o log do falho", async () => {
    const user = userEvent.setup();
    render(<QueueReport job={job} />);
    expect(screen.getByRole("heading", { name: "Relatório" })).toBeInTheDocument();
    expect(screen.getByText("Concluídos: 1")).toBeInTheDocument();
    expect(screen.getByText("Falhos: 1")).toBeInTheDocument();
    expect(screen.getByText("Tempo total: 2h 10min")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Abrir log" }));
    expect(openLocalPath).toHaveBeenCalledWith("/tmp/guarda.log");
  });
});
