import tomllib
from importlib.metadata import PackageNotFoundError
from pathlib import Path

import pytest

import voxel_worker

PYPROJECT = Path(__file__).parent.parent / "pyproject.toml"


def test_versao_vem_do_pyproject() -> None:
    with PYPROJECT.open("rb") as arquivo:
        projeto = tomllib.load(arquivo)

    assert voxel_worker.__version__ == projeto["project"]["version"]


def test_versao_sem_pacote_instalado(monkeypatch: pytest.MonkeyPatch) -> None:
    def ausente(_name: str) -> str:
        raise PackageNotFoundError

    monkeypatch.setattr(voxel_worker, "version", ausente)
    assert voxel_worker.package_version() == "0.1.0"
