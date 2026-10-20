"""Marching cubes em CPU, no lugar do `torchmcubes`, que não tem wheel."""

from __future__ import annotations

import importlib.util
import sys
import types
from collections.abc import Callable
from typing import Protocol


class _Array(Protocol):
    def detach(self) -> _Array: ...

    def cpu(self) -> _Array: ...

    def numpy(self) -> object: ...


Cubes = Callable[[object, float], tuple[object, object]]
ToTensor = Callable[[object], object]


def install_marching_cubes() -> None:
    """Publica `torchmcubes` só quando o pacote compilado não está instalado."""
    if importlib.util.find_spec("torchmcubes") is not None:
        return
    module = types.ModuleType("torchmcubes")
    module.marching_cubes = cpu_marching_cubes  # type: ignore[attr-defined]
    sys.modules["torchmcubes"] = module


def cpu_marching_cubes(
    volume: _Array,
    thresh: float,
    *,
    cubes: Cubes | None = None,
    to_tensor: ToTensor | None = None,
) -> tuple[object, object]:
    """Extrai a superfície. `skimage` devolve os eixos na ordem que o TripoSR espera."""
    array = volume.detach().cpu().numpy()
    verts, faces = (cubes or _cubes)(array, float(thresh))
    convert = to_tensor or _as_torch
    return convert(verts), convert(faces)


def _cubes(array: object, level: float) -> tuple[object, object]:
    from skimage.measure import marching_cubes

    verts, faces, _normals, _values = marching_cubes(array, level=level)
    return verts, faces


def _as_torch(array: object) -> object:
    import numpy as np
    import torch

    values = np.ascontiguousarray(array)
    return torch.from_numpy(values)
