// Gerado a partir de protocol/openapi.json. Não edite à mão.

export type Health = {
  status: string;
};

export type Ack = {
  accepted: boolean;
};

export type ErrorBody = {
  message: string;
};

export type ModelKind = "image" | "mesh" | "background";

export type LoadModelRequest = {
  modelId: string;
  modelPath: string;
  kind: ModelKind;
};

export type CancelRequest = {
  jobId: string;
};

export type Txt2ImgRequest = {
  jobId: string;
  prompt: string;
  negativePrompt?: string;
  modelId: string;
  outputPath: string;
  steps: number;
  width: number;
  height: number;
  seed: number;
  cfgScale?: number;
  circular?: boolean;
};

export type Img2ImgRequest = {
  jobId: string;
  prompt: string;
  negativePrompt?: string;
  modelId: string;
  imagePath: string;
  outputPath: string;
  steps: number;
  strength: number;
  width?: number;
  height?: number;
  seed?: number;
  cfgScale?: number;
  circular?: boolean;
};

export type InpaintRequest = {
  jobId: string;
  prompt: string;
  negativePrompt?: string;
  modelId: string;
  imagePath: string;
  maskPath: string;
  outputPath: string;
  steps: number;
  strength: number;
  width?: number;
  height?: number;
  seed?: number;
  cfgScale?: number;
  circular?: boolean;
};

export type DecodeLatentsRequest = {
  jobId: string;
  modelId: string;
  latentsPath: string;
  outputPath: string;
};

export type RemoveBackgroundRequest = {
  jobId: string;
  imagePath: string;
  outputPath: string;
};

export type ImageToMeshRequest = {
  jobId: string;
  imagePath: string;
  modelId: string;
  outputPath: string;
};

export type MeshPostProcessRequest = {
  jobId: string;
  meshPath: string;
  outputPath: string;
  decimate: boolean;
  normalize: boolean;
  maxTextures?: number;
};

export type JobOutput = {
  jobId: string;
  path: string;
};

export type ProgressName = "progress";

export type CompletedName = "completed";

export type FailedName = "failed";

export type ProgressEvent = {
  event: ProgressName;
  jobId: string;
  step: number;
  total: number;
  previewPath?: string;
};

export type CompletedEvent = {
  event: CompletedName;
  jobId: string;
  path: string;
};

export type FailedEvent = {
  event: FailedName;
  jobId: string;
  message: string;
};

export const paths = {
  health: "/health",
  modelsLoad: "/models/load",
  modelsUnload: "/models/unload",
  jobsCancel: "/jobs/cancel",
  imagesTxt2img: "/images/txt2img",
  imagesImg2img: "/images/img2img",
  imagesInpaint: "/images/inpaint",
  imagesDecodeLatents: "/images/decode-latents",
  imagesRemoveBackground: "/images/remove-background",
  meshesFromImage: "/meshes/from-image",
  meshesPostProcess: "/meshes/post-process",
  events: "/events",
} as const;
