"""O que o servidor chama para transformar uma imagem em malha."""

import os
import threading
from pathlib import Path
from typing import Protocol

from voxel_worker.jobs import Progress
from voxel_worker.mesh_files import write_pair
from voxel_worker.triposr import MeshError, image_to_mesh


class MeshEngine(Protocol):
    def load(self, model_path: Path) -> None: ...

    def unload(self) -> None: ...

    def image_to_mesh(
        self,
        image_path: Path,
        output_path: Path,
        cancelled: threading.Event,
        progress: Progress,
    ) -> Path: ...


class FakeEngine:
    """Triângulo válido, para o contrato sem GPU nem peso."""

    def __init__(self) -> None:
        self.model: Path | None = None

    def load(self, model_path: Path) -> None:
        if not model_path.is_file():
            raise MeshError("modelo ausente")
        self.model = model_path

    def unload(self) -> None:
        self.model = None

    def image_to_mesh(
        self,
        image_path: Path,
        output_path: Path,
        cancelled: threading.Event,
        progress: Progress,
    ) -> Path:
        if cancelled.is_set():
            raise MeshError("cancelado")
        if self.model is None:
            raise MeshError("modelo ausente")
        if not image_path.is_file():
            raise MeshError("imagem ausente")
        progress(1, 2)
        if cancelled.is_set():
            raise MeshError("cancelado")
        glb, _obj = write_pair(output_path)
        progress(2, 2)
        return glb


class TripoEngine:
    def __init__(self) -> None:
        self.model: Path | None = None

    def load(self, model_path: Path) -> None:
        if not model_path.is_file():
            raise MeshError("modelo ausente")
        self.model = model_path

    def unload(self) -> None:
        self.model = None

    def image_to_mesh(
        self,
        image_path: Path,
        output_path: Path,
        cancelled: threading.Event,
        progress: Progress,
    ) -> Path:
        if self.model is None:
            raise MeshError("modelo ausente")
        return image_to_mesh(image_path, output_path, self.model, cancelled, progress)


def engine_from_env() -> MeshEngine:
    if os.environ.get("VOXEL_MESH_BACKEND") == "fake":
        return FakeEngine()
    return TripoEngine()
