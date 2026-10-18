import tomllib
from pathlib import Path

import voxel_worker

PYPROJECT = Path(__file__).parent.parent / "pyproject.toml"


def test_versao_vem_do_pyproject() -> None:
    with PYPROJECT.open("rb") as arquivo:
        projeto = tomllib.load(arquivo)

    assert voxel_worker.__version__ == projeto["project"]["version"]
