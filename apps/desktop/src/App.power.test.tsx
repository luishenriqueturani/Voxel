import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import App from "./App";
import { loadPowerNotice } from "./native/power";

vi.mock("./native/power", () => ({
  loadPowerNotice: vi.fn(async () => "NVIDIA GeForce RTX 5090"),
}));

vi.mock("./native/errors", () => ({
  listenToErrors: vi.fn(async () => () => undefined),
}));

vi.mock("./native/menu", () => ({
  syncNativeMenu: vi.fn(async () => undefined),
  listenToMenu: vi.fn(async () => () => undefined),
}));

describe("aviso de conector na abertura", () => {
  it("mostra a placa e some ao dispensar", async () => {
    const user = userEvent.setup();
    render(<App />);

    const status = await screen.findByRole("status");
    expect(status).toHaveTextContent("NVIDIA GeForce RTX 5090");
    expect(status).toHaveTextContent("300 W");

    await user.click(screen.getByRole("button", { name: "Entendi" }));
    await waitFor(() => expect(screen.queryByRole("status")).not.toBeInTheDocument());
    expect(loadPowerNotice).toHaveBeenCalled();
  });
});
