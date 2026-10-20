from __future__ import annotations

import threading
from pathlib import Path

import pytest

from voxel_worker.triposr import (
    CONFIG,
    MC_RESOLUTION,
    Loaded,
    MeshError,
    device_of,
    image_to_mesh,
    load_modules,
)


class _Picture:
    def __init__(self) -> None:
        self.size = (4, 4)

    def convert(self, _mode: str) -> _Picture:
        return self


class _Image:
    def open(self, path: Path) -> _Picture:
        assert path.is_file()
        return _Picture()

    def new(
        self,
        _mode: str,
        _size: tuple[int, int],
        _color: tuple[int, int, int, int],
    ) -> _Picture:
        return _Picture()

    def alpha_composite(self, _background: _Picture, foreground: _Picture) -> _Picture:
        return foreground


class _Mesh:
    def export(self, path: str) -> None:
        Path(path).write_text("malha", encoding="utf-8")


class _Renderer:
    def set_chunk_size(self, size: int) -> None:
        self.size = size


class _System:
    def __init__(self) -> None:
        self.renderer = _Renderer()
        self.stopped = False

    def to(self, device: str) -> _System:
        self.device = device
        return self

    def __call__(self, images: list[object], device: str) -> object:
        assert images
        assert device == "cpu"
        return "cena"

    def extract_mesh(
        self,
        scene_codes: object,
        has_vertex_color: bool,
        resolution: int,
    ) -> list[object]:
        assert scene_codes == "cena"
        assert has_vertex_color
        assert resolution == MC_RESOLUTION
        return [_Mesh()]


class _Tsr:
    @staticmethod
    def from_pretrained(path: str, config_name: str, weight_name: str) -> _System:
        folder = Path(path)
        assert config_name == "config.yaml"
        assert weight_name == "model.ckpt"
        assert (folder / "config.yaml").is_file()
        assert (folder / "model.ckpt").is_symlink()
        return _System()


class _Cuda:
    @staticmethod
    def is_available() -> bool:
        return False


class _Torch:
    cuda = _Cuda()


def test_o_config_e_o_da_medicao() -> None:
    assert "cond_image_size: 512" in CONFIG
    assert "facebook/dino-vitb16" in CONFIG


def test_sem_o_codigo_do_modelo_a_mensagem_diz_o_que_falta() -> None:
    with pytest.raises(MeshError, match="ausente no ambiente 3D"):
        load_modules()


def test_device_cai_para_cpu_sem_cuda() -> None:
    assert device_of(_Torch()) == "cpu"


def test_gera_glb_e_obj_com_o_modelo_injetado(tmp_path: Path) -> None:
    image = tmp_path / "foto.png"
    weight = tmp_path / "model.ckpt"
    image.write_bytes(b"png")
    weight.write_bytes(b"peso")
    steps: list[int] = []
    result = image_to_mesh(
        image,
        tmp_path / "saida.glb",
        weight,
        threading.Event(),
        lambda step, _total: steps.append(step),
        modules=Loaded(_Torch(), _Image(), _Tsr()),
    )

    assert result == tmp_path / "saida.glb"
    assert result.read_text(encoding="utf-8") == "malha"
    assert (tmp_path / "saida.obj").read_text(encoding="utf-8") == "malha"
    assert steps == [1, 2, 3, 4]


def test_cancelado_antes_de_comecar(tmp_path: Path) -> None:
    image = tmp_path / "foto.png"
    image.write_bytes(b"png")
    cancelled = threading.Event()
    cancelled.set()
    with pytest.raises(MeshError, match="cancelado"):
        image_to_mesh(
            image,
            tmp_path / "saida.glb",
            tmp_path / "model.ckpt",
            cancelled,
            lambda _step, _total: None,
            modules=Loaded(_Torch(), _Image(), _Tsr()),
        )
