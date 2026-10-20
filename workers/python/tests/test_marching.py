from __future__ import annotations

import sys

from voxel_worker.marching import cpu_marching_cubes, install_marching_cubes


class _Volume:
    def detach(self) -> _Volume:
        return self

    def cpu(self) -> _Volume:
        return self

    def numpy(self) -> str:
        return "volume"


def test_o_substituto_chama_o_extrator_e_o_tensor() -> None:
    seen: list[tuple[object, float]] = []

    def cubes(array: object, level: float) -> tuple[object, object]:
        seen.append((array, level))
        return "verts", "faces"

    converted: list[object] = []

    def to_tensor(array: object) -> object:
        converted.append(array)
        return f"tensor:{array}"

    verts, faces = cpu_marching_cubes(_Volume(), 0.0, cubes=cubes, to_tensor=to_tensor)

    assert seen == [("volume", 0.0)]
    assert converted == ["verts", "faces"]
    assert verts == "tensor:verts"
    assert faces == "tensor:faces"


def test_instala_o_modulo_quando_o_pacote_nao_existe() -> None:
    sys.modules.pop("torchmcubes", None)
    try:
        install_marching_cubes()
        module = sys.modules["torchmcubes"]
        assert callable(module.marching_cubes)
    finally:
        sys.modules.pop("torchmcubes", None)
