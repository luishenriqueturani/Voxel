import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { GenerationParams, OutputMode } from "../components/layout/PromptPanel";
import { blankEdit } from "../lib/edit";
import type { GalleryImage } from "../native/generation";
import { generateImage } from "../native/generation";
import { meshFromImage } from "../native/mesh";
import { useGenerationRequest } from "./useGenerationRequest";

vi.mock("../native/generation", () => ({
  generateImage: vi.fn(),
  imageSrc: (preview: string) => `data:image/png;base64,${preview}`,
  isCancelled: () => false,
}));

vi.mock("../native/mesh", () => ({
  meshFromImage: vi.fn(),
  meshOptimize: () => null,
}));

const params: GenerationParams = {
  resolution: 512,
  aspect: "1:1",
  width: 512,
  height: 512,
  steps: 20,
  cfg: 7,
  seed: "",
  count: 2,
};

const slides: GalleryImage[] = [
  { src: "data:image/png;base64,aaa", path: "/tmp/frente.png", seed: 1 },
  { src: "data:image/png;base64,bbb", path: "/tmp/lado.png", seed: 2 },
];

const prepareEnvironment = vi.fn(() => Promise.resolve());
const noteIntent = vi.fn(() => Promise.resolve());
const reportFailure = vi.fn();
const generatingRef = { current: false };

function renderRequest(outputMode: OutputMode = "3d") {
  return renderHook(() =>
    useGenerationRequest({
      generatingRef,
      removing: false,
      editRef: { current: blankEdit },
      holdEdit: vi.fn(),
      commitEdit: vi.fn(),
      discardEdit: vi.fn(),
      restoreEdit: vi.fn(),
      setGenerating: vi.fn(),
      setSlides: vi.fn(),
      setSlideIndex: vi.fn(),
      setFrame: vi.fn(),
      setHistoryRevision: vi.fn(),
      params,
      prompt: "um gato",
      negativePrompt: "",
      modelId: "sd15",
      reportFailure,
      outputMode,
      prepareEnvironment,
      noteIntent,
      setWaitingEnvironment: vi.fn(),
      slides,
      slideIndex: 0,
    }),
  );
}

describe("useGenerationRequest", () => {
  beforeEach(() => {
    generatingRef.current = false;
    vi.mocked(generateImage).mockResolvedValue({
      path: "/tmp/lado.png",
      seed: 2,
      modelId: "sd15",
      preview: "bbb",
      images: slides,
    });
    vi.mocked(meshFromImage).mockResolvedValue({
      imagePath: "/tmp/frente.png",
      glbPath: "/tmp/frente.glb",
      objPath: "/tmp/frente.obj",
      preview: "ccc",
      optimizedGlb: null,
      optimizedObj: null,
      optimizedFbx: null,
      references: [],
    });
  });

  it("gera as imagens em texto e espera a seleção para a malha", async () => {
    const { result } = renderRequest();

    act(() => {
      result.current.request();
    });

    await waitFor(() => {
      expect(result.current.meshPick?.views.map((view) => view.path)).toEqual([
        "/tmp/frente.png",
        "/tmp/lado.png",
      ]);
    });
    expect(result.current.meshPick?.primaryPath).toBe("/tmp/lado.png");
    expect(meshFromImage).not.toHaveBeenCalled();

    const main = result.current.meshPick?.views[0];
    if (main === undefined) {
      throw new Error("seleção sem vista");
    }
    act(() => {
      result.current.confirmMesh([main], main.path);
    });

    await waitFor(() => {
      expect(meshFromImage).toHaveBeenCalledWith(["/tmp/frente.png"], "/tmp/frente.png", null);
    });
    expect(noteIntent).not.toHaveBeenCalled();
    expect(result.current.meshPick).toBeNull();
  });

  it("abre a seleção ao enviar para 3D e cancelar não gera malha", () => {
    const { result } = renderRequest("2d");

    act(() => {
      result.current.sendImage("/tmp/frente.png");
    });

    expect(result.current.meshPick?.views).toHaveLength(2);
    expect(result.current.meshPick?.primaryPath).toBe("/tmp/frente.png");
    expect(meshFromImage).not.toHaveBeenCalled();

    act(() => {
      result.current.cancelMesh();
    });

    expect(result.current.meshPick).toBeNull();
    expect(meshFromImage).not.toHaveBeenCalled();
    expect(noteIntent).not.toHaveBeenCalled();
  });

  it("confirma o envio pela vista principal", async () => {
    const { result } = renderRequest("2d");

    act(() => {
      result.current.sendImage("/tmp/c.png");
    });
    const only = result.current.meshPick?.views[0];
    if (only === undefined) {
      throw new Error("seleção sem vista");
    }
    act(() => {
      result.current.confirmMesh([only], only.path);
    });

    await waitFor(() => {
      expect(noteIntent).toHaveBeenCalledWith("send-to-3d");
      expect(meshFromImage).toHaveBeenCalledWith(["/tmp/c.png"], "/tmp/c.png", null);
    });
  });

  it("mantém as imagens prontas de um lote interrompido", async () => {
    const setSlides = vi.fn();
    const setHistoryRevision = vi.fn();
    const frente = slides[0];
    if (frente === undefined) {
      throw new Error("slide ausente");
    }
    vi.mocked(generateImage).mockResolvedValue({
      path: "/tmp/frente.png",
      seed: 1,
      modelId: "sd15",
      preview: "aaa",
      images: [frente],
    });
    const { result } = renderHook(() =>
      useGenerationRequest({
        generatingRef,
        removing: false,
        editRef: { current: blankEdit },
        holdEdit: vi.fn(),
        commitEdit: vi.fn(),
        discardEdit: vi.fn(),
        restoreEdit: vi.fn(),
        setGenerating: vi.fn(),
        setSlides,
        setSlideIndex: vi.fn(),
        setFrame: vi.fn(),
        setHistoryRevision,
        params,
        prompt: "um gato",
        negativePrompt: "",
        modelId: "sd15",
        reportFailure,
        outputMode: "2d",
        prepareEnvironment,
        noteIntent,
        setWaitingEnvironment: vi.fn(),
        slides: [],
        slideIndex: 0,
      }),
    );

    act(() => {
      result.current.request();
    });

    await waitFor(() => {
      expect(setSlides).toHaveBeenCalledWith([frente]);
      expect(setHistoryRevision).toHaveBeenCalled();
    });
  });

  it("no modo imagens abre a seleção sem gerar texto", () => {
    const { result } = renderRequest();

    act(() => {
      result.current.setMeshOrigin("images");
    });
    act(() => {
      result.current.request();
    });

    expect(generateImage).not.toHaveBeenCalled();
    expect(result.current.meshPick?.primaryPath).toBe("/tmp/frente.png");
    expect(result.current.meshPick?.views).toHaveLength(2);
    expect(meshFromImage).not.toHaveBeenCalled();
  });

  it("junta imagens do disco e da geração na seleção aberta", () => {
    const { result } = renderRequest();

    act(() => {
      result.current.addMeshPaths(["/tmp/fora.png"]);
    });
    expect(result.current.canAddCurrent).toBe(true);

    act(() => {
      result.current.addCurrentViews();
    });

    expect(result.current.meshPick?.views.map((view) => view.path)).toEqual([
      "/tmp/fora.png",
      "/tmp/frente.png",
      "/tmp/lado.png",
    ]);
    expect(meshFromImage).not.toHaveBeenCalled();
  });
});
