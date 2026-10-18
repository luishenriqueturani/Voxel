import { renderHook } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { useGenerateShortcut } from "./useGenerateShortcut";

describe("useGenerateShortcut", () => {
  it("dispara a geração com Ctrl+Enter ou Meta+Enter", async () => {
    const user = userEvent.setup();
    const onGenerate = vi.fn();
    renderHook(() => useGenerateShortcut(onGenerate));

    await user.keyboard("{Control>}{Enter}{/Control}");
    await user.keyboard("{Meta>}{Enter}{/Meta}");

    expect(onGenerate).toHaveBeenCalledTimes(2);
  });

  it("não dispara com Enter sozinho nem com a tecla repetida", async () => {
    const user = userEvent.setup();
    const onGenerate = vi.fn();
    renderHook(() => useGenerateShortcut(onGenerate));

    await user.keyboard("{Enter}");
    window.dispatchEvent(
      new KeyboardEvent("keydown", { key: "Enter", ctrlKey: true, repeat: true }),
    );

    expect(onGenerate).not.toHaveBeenCalled();
  });
});
