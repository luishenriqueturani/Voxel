import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { StatusBar } from "./StatusBar";

vi.mock("@tauri-apps/api/core", () => ({
  isTauri: () => true,
}));

describe("StatusBar no app Tauri", () => {
  it("não mostra o atalho de preferências, que fica no menu nativo", () => {
    render(<StatusBar status={null} onOpenSettings={() => undefined} />);

    expect(screen.queryByRole("button", { name: "Preferências" })).not.toBeInTheDocument();
  });

  it("mostra GPU, backend, memórias e temperaturas", () => {
    render(
      <StatusBar
        status={{
          gpuName: "RX 7600",
          backend: "vulkan",
          vramUsedBytes: 1_073_741_824,
          vramTotalBytes: 8_589_934_592,
          gpuPercent: 37,
          cpuPercent: 12,
          ramUsedBytes: 8_589_934_592,
          ramTotalBytes: 34_359_738_368,
          cpuCelsius: 54,
          gpuCelsius: 48,
        }}
        onOpenSettings={() => undefined}
      />,
    );

    const bar = screen.getByRole("contentinfo");
    expect(bar).toHaveTextContent("GPU: RX 7600 (vulkan)");
    expect(bar).toHaveTextContent("VRAM: 1.0/8.0 GiB");
    expect(bar).toHaveTextContent("Uso GPU: 37%");
    expect(bar).toHaveTextContent("CPU: 12%");
    expect(bar).toHaveTextContent("RAM: 8.0/32.0 GiB");
    expect(bar).toHaveTextContent("Temp. CPU: 54 °C");
    expect(bar).toHaveTextContent("Temp. GPU: 48 °C");
  });
});
