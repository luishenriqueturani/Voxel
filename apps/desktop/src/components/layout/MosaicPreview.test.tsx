import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { MosaicPreview } from "./MosaicPreview";

describe("MosaicPreview", () => {
  it("repete a textura nove vezes", () => {
    render(<MosaicPreview src="data:image/png;base64,aaaa" />);
    const mosaic = screen.getByTestId("texture-mosaic");
    expect(mosaic).toHaveTextContent("Textura repetida em mosaico 3 por 3");
    expect(mosaic.querySelectorAll("img")).toHaveLength(9);
  });
});
