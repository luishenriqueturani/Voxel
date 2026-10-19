"""Tipos gerados a partir de protocol/openapi.json. Não edite à mão."""

from typing import Literal, NotRequired, TypedDict


class Health(TypedDict):
    status: str


class Ack(TypedDict):
    accepted: bool


class ErrorBody(TypedDict):
    message: str


ModelKind = Literal["image", "mesh", "background"]


class LoadModelRequest(TypedDict):
    modelId: str
    modelPath: str
    kind: ModelKind


class CancelRequest(TypedDict):
    jobId: str


class Txt2ImgRequest(TypedDict):
    jobId: str
    prompt: str
    negativePrompt: NotRequired[str]
    modelId: str
    outputPath: str
    steps: int
    width: int
    height: int
    seed: int


class Img2ImgRequest(TypedDict):
    jobId: str
    prompt: str
    modelId: str
    imagePath: str
    outputPath: str
    steps: int
    strength: float


class InpaintRequest(TypedDict):
    jobId: str
    prompt: str
    modelId: str
    imagePath: str
    maskPath: str
    outputPath: str


class DecodeLatentsRequest(TypedDict):
    jobId: str
    modelId: str
    latentsPath: str
    outputPath: str


class RemoveBackgroundRequest(TypedDict):
    jobId: str
    imagePath: str
    outputPath: str


class ImageToMeshRequest(TypedDict):
    jobId: str
    imagePath: str
    modelId: str
    outputPath: str


class MeshPostProcessRequest(TypedDict):
    jobId: str
    meshPath: str
    outputPath: str
    decimate: bool
    normalize: bool
    maxTextures: NotRequired[int]


class JobOutput(TypedDict):
    jobId: str
    path: str


ProgressName = Literal["progress"]


CompletedName = Literal["completed"]


FailedName = Literal["failed"]


class ProgressEvent(TypedDict):
    event: ProgressName
    jobId: str
    step: int
    total: int
    previewPath: NotRequired[str]


class CompletedEvent(TypedDict):
    event: CompletedName
    jobId: str
    path: str


class FailedEvent(TypedDict):
    event: FailedName
    jobId: str
    message: str


PATHS: dict[str, str] = {
    "health": "/health",
    "modelsLoad": "/models/load",
    "modelsUnload": "/models/unload",
    "jobsCancel": "/jobs/cancel",
    "imagesTxt2img": "/images/txt2img",
    "imagesImg2img": "/images/img2img",
    "imagesInpaint": "/images/inpaint",
    "imagesDecodeLatents": "/images/decode-latents",
    "imagesRemoveBackground": "/images/remove-background",
    "meshesFromImage": "/meshes/from-image",
    "meshesPostProcess": "/meshes/post-process",
    "events": "/events",
}
