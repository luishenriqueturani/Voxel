import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { StatusBar } from "./StatusBar";

vi.mock("@tauri-apps/api/core", () => ({
  isTauri: () => true,
}));

describe("StatusBar no app Tauri", () => {
  it("não mostra o atalho de preferências, que fica no menu nativo", () => {
    render(<StatusBar onOpenSettings={() => undefined} />);

    expect(screen.queryByRole("button", { name: "Preferências" })).not.toBeInTheDocument();
  });
});
