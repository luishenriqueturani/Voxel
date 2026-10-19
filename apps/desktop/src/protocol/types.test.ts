import { describe, expect, it } from "vitest";
import { paths, type Txt2ImgRequest } from "./types";

describe("contrato do worker", () => {
  it("expõe o caminho de texto para imagem", () => {
    const request = {
      jobId: "job",
      prompt: "um gato",
      modelId: "sd",
      outputPath: "/tmp/out.png",
      steps: 2,
      width: 8,
      height: 8,
      seed: 1,
    } satisfies Txt2ImgRequest;

    expect(paths.imagesTxt2img).toBe("/images/txt2img");
    expect(request.prompt).toBe("um gato");
  });
});
