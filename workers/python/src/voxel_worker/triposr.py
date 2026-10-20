"""Adaptador do TripoSR. O código do modelo entra pelo `PYTHONPATH`."""

from __future__ import annotations

import importlib.util
import shutil
import sys
import tempfile
import threading
import types
from collections.abc import Callable
from pathlib import Path
from typing import Protocol, cast

from voxel_worker.marching import install_marching_cubes

# Medido na RX 7600: cena 512, malha 128³, avaliação em blocos de 8192.
MC_RESOLUTION = 128
CHUNK_SIZE = 8192
FOREGROUND_RATIO = 0.85

CONFIG = """\
cond_image_size: 512

image_tokenizer_cls: tsr.models.tokenizers.image.DINOSingleImageTokenizer
image_tokenizer:
  pretrained_model_name_or_path: "facebook/dino-vitb16"

tokenizer_cls: tsr.models.tokenizers.triplane.Triplane1DTokenizer
tokenizer:
  plane_size: 32
  num_channels: 1024

backbone_cls: tsr.models.transformer.transformer_1d.Transformer1D
backbone:
  in_channels: ${tokenizer.num_channels}
  num_attention_heads: 16
  attention_head_dim: 64
  num_layers: 16
  cross_attention_dim: 768

post_processor_cls: tsr.models.network_utils.TriplaneUpsampleNetwork
post_processor:
  in_channels: 1024
  out_channels: 40

decoder_cls: tsr.models.network_utils.NeRFMLP
decoder:
  in_channels: 120 # 3 * 40
  n_neurons: 64
  n_hidden_layers: 9
  activation: silu

renderer_cls: tsr.models.nerf_renderer.TriplaneNeRFRenderer
renderer:
  radius: 0.87 # slightly larger than 0.5 * sqrt(3)
  feature_reduction: concat
  density_activation: exp
  density_bias: -1.0
  num_samples_per_ray: 128
"""


class MeshError(RuntimeError):
    pass


class _Renderer(Protocol):
    def set_chunk_size(self, size: int) -> None: ...


class _System(Protocol):
    renderer: _Renderer

    def to(self, device: str) -> _System: ...

    def __call__(self, images: list[object], device: str) -> object: ...

    def extract_mesh(
        self,
        scene_codes: object,
        has_vertex_color: bool,
        resolution: int,
    ) -> list[object]: ...


class _Tsr(Protocol):
    @staticmethod
    def from_pretrained(path: str, config_name: str, weight_name: str) -> _System: ...


class _ImageModule(Protocol):
    def open(self, path: Path) -> _Picture: ...

    def new(
        self,
        mode: str,
        size: tuple[int, int],
        color: tuple[int, int, int, int],
    ) -> _Picture: ...

    def alpha_composite(self, background: _Picture, foreground: _Picture) -> _Picture: ...


class _Picture(Protocol):
    size: tuple[int, int]

    def convert(self, mode: str) -> _Picture: ...


class Loaded:
    """Peças do TripoSR. Nos testes cada campo é um dublê."""

    def __init__(self, torch: object, image: object, tsr: object) -> None:
        self.torch = torch
        self.image = image
        self.tsr = tsr


Progress = Callable[[int, int], None]


def install_background_stub() -> None:
    """O TripoSR importa o `rembg` ao carregar. O fundo já saiu pelo U²-Net."""
    if importlib.util.find_spec("rembg") is not None:
        return
    module = types.ModuleType("rembg")

    def unused(*_args: object, **_kwargs: object) -> None:
        raise MeshError("o fundo já foi removido antes do TripoSR")

    module.remove = unused  # type: ignore[attr-defined]
    module.new_session = unused  # type: ignore[attr-defined]
    sys.modules["rembg"] = module


def load_modules() -> Loaded:
    install_marching_cubes()
    install_background_stub()
    try:
        import torch
        from PIL import Image
        from tsr.system import TSR
    except ImportError as error:
        missing = error.name or "dependência"
        raise MeshError(
            f"{missing} ausente no ambiente 3D. Reinstale o ambiente em Configurações."
        ) from error
    return Loaded(torch, Image, TSR)


def device_of(torch_module: object) -> str:
    cuda = getattr(torch_module, "cuda", None)
    available = getattr(cuda, "is_available", None)
    if callable(available) and bool(available()):
        return "cuda"
    return "cpu"


def stage_model(model_path: Path) -> Path:
    """Junta o config vendido e um atalho para o checkpoint, sem copiar o peso."""
    if not model_path.is_file():
        raise MeshError("modelo ausente")
    directory = Path(tempfile.mkdtemp(prefix="voxel-triposr-"))
    (directory / "config.yaml").write_text(CONFIG, encoding="utf-8")
    try:
        (directory / "model.ckpt").symlink_to(model_path)
    except OSError as error:
        shutil.rmtree(directory, ignore_errors=True)
        raise MeshError("não foi possível preparar o TripoSR") from error
    return directory


def image_to_mesh(
    image_path: Path,
    output_path: Path,
    model_path: Path,
    cancelled: threading.Event,
    progress: Progress,
    *,
    modules: Loaded | None = None,
) -> Path:
    if cancelled.is_set():
        raise MeshError("cancelado")
    if not image_path.is_file():
        raise MeshError("imagem ausente")
    loaded = modules if modules is not None else load_modules()
    progress(1, 4)
    directory = stage_model(model_path)
    try:
        return _run(loaded, directory, image_path, output_path, cancelled, progress)
    finally:
        shutil.rmtree(directory, ignore_errors=True)


def _run(
    loaded: Loaded,
    directory: Path,
    image_path: Path,
    output_path: Path,
    cancelled: threading.Event,
    progress: Progress,
) -> Path:
    if cancelled.is_set():
        raise MeshError("cancelado")
    device = device_of(loaded.torch)
    model = cast(_Tsr, loaded.tsr).from_pretrained(str(directory), "config.yaml", "model.ckpt")
    model.renderer.set_chunk_size(CHUNK_SIZE)
    model.to(device)
    picture = _prepare(cast(_ImageModule, loaded.image), image_path)
    progress(2, 4)
    if cancelled.is_set():
        raise MeshError("cancelado")
    scene = model([picture], device=device)
    progress(3, 4)
    if cancelled.is_set():
        raise MeshError("cancelado")
    meshes = model.extract_mesh(scene, True, resolution=MC_RESOLUTION)
    if not meshes:
        raise MeshError("o TripoSR não devolveu malha")
    exported = meshes[0]
    export = getattr(exported, "export", None)
    if not callable(export):
        raise MeshError("malha sem exportação")
    glb = output_path.with_suffix(".glb")
    obj = output_path.with_suffix(".obj")
    glb.parent.mkdir(parents=True, exist_ok=True)
    export(str(glb))
    export(str(obj))
    progress(4, 4)
    return glb


def _prepare(image: _ImageModule, image_path: Path) -> _Picture:
    rgba = image.open(image_path).convert("RGBA")
    try:
        from tsr.utils import resize_foreground

        rgba = resize_foreground(rgba, FOREGROUND_RATIO)
    except (ImportError, ValueError, AssertionError):
        pass
    background = image.new("RGBA", rgba.size, (128, 128, 128, 255))
    return image.alpha_composite(background, rgba).convert("RGB")
