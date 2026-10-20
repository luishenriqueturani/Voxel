import { fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { MaskEditor } from "./MaskEditor";

function loadMask(width = 4, height = 4) {
  const image = screen.getByRole("img", { name: "Máscara" });
  Object.defineProperty(image, "naturalWidth", { configurable: true, value: width });
  Object.defineProperty(image, "naturalHeight", { configurable: true, value: height });
  fireEvent.load(image);
}

class FakeImageData {
  readonly data: Uint8ClampedArray;
  readonly width: number;
  readonly height: number;
  colorSpace = "srgb" as const;

  constructor(width: number, height: number) {
    this.width = width;
    this.height = height;
    this.data = new Uint8ClampedArray(width * height * 4);
  }
}

describe("MaskEditor", () => {
  it("oferece pincel, borracha, desfazer e limpar", async () => {
    const user = userEvent.setup();
    render(<MaskEditor src="data:image/png;base64,aaaa" onMask={() => undefined} />);
    expect(screen.getByRole("slider", { name: "Pincel" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Desfazer" })).toBeDisabled();
    await user.click(screen.getByRole("button", { name: "Borracha" }));
    expect(screen.getByRole("button", { name: "Pintar" })).toHaveAttribute("aria-pressed", "true");
    expect(screen.getByRole("button", { name: "Limpar" })).toBeEnabled();
  });

  it("desfaz o traço e, sem traço, devolve a imagem anterior", async () => {
    const user = userEvent.setup();
    vi.stubGlobal("ImageData", FakeImageData);
    const pixels = new Uint8ClampedArray(4 * 4 * 4);
    vi.spyOn(HTMLCanvasElement.prototype, "getContext").mockReturnValue({
      getImageData: () => ({ data: pixels, width: 4, height: 4 }),
      putImageData: (image: ImageData) => {
        pixels.set(image.data);
      },
      clearRect: () => {
        pixels.fill(0);
      },
    } as unknown as CanvasRenderingContext2D);
    vi.spyOn(HTMLCanvasElement.prototype, "toDataURL").mockReturnValue(
      "data:image/png;base64,aaaa",
    );
    const onRevert = vi.fn();
    render(
      <MaskEditor
        src="data:image/png;base64,aaaa"
        onMask={() => undefined}
        canRevert
        onRevert={onRevert}
      />,
    );
    expect(screen.getByRole("button", { name: "Desfazer" })).toBeEnabled();
    loadMask();
    const canvas = document.querySelector("canvas");
    if (canvas === null) {
      throw new Error("canvas ausente");
    }
    fireEvent.pointerDown(canvas, { button: 0, clientX: 2, clientY: 2, pointerId: 1 });
    fireEvent.pointerUp(canvas, { button: 0, clientX: 2, clientY: 2, pointerId: 1 });
    fireEvent.load(screen.getByRole("img", { name: "Máscara" }));
    const undo = screen.getByRole("button", { name: "Desfazer" });
    expect(undo).toBeEnabled();
    await user.click(undo);
    expect(onRevert).not.toHaveBeenCalled();
    await user.click(undo);
    expect(onRevert).toHaveBeenCalledOnce();
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });
});
