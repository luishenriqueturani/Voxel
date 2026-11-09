import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { ExecutionSection } from "./ExecutionSection";

const stored = vi.hoisted(() => ({
  inhibitSleep: true,
  queueFinish: "none" as const,
}));

vi.mock("../../native/settings", () => ({
  loadStoredSettings: vi.fn(async () => ({
    language: null,
    theme: null,
    inhibitSleep: stored.inhibitSleep,
    queueFinish: stored.queueFinish,
  })),
  saveStoredSettings: vi.fn(async () => undefined),
}));

import { saveStoredSettings } from "../../native/settings";

describe("execução", () => {
  beforeEach(() => {
    stored.inhibitSleep = true;
    stored.queueFinish = "none";
    vi.mocked(saveStoredSettings).mockClear();
  });

  it("grava inibição e ação ao terminar", async () => {
    const user = userEvent.setup();
    render(<ExecutionSection />);
    const inhibit = await screen.findByRole("checkbox", {
      name: /Impedir suspensão durante a fila/,
    });
    expect(inhibit).toBeChecked();
    await user.click(inhibit);
    expect(saveStoredSettings).toHaveBeenCalledWith({ inhibitSleep: false });
    await user.selectOptions(
      screen.getByRole("combobox", { name: "Ao terminar a fila" }),
      "suspend",
    );
    expect(saveStoredSettings).toHaveBeenCalledWith({ queueFinish: "suspend" });
  });
});
