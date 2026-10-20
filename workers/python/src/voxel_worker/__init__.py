"""Worker Python do Voxel."""

from importlib.metadata import PackageNotFoundError, version


def package_version() -> str:
    """Versão do pacote. No aplicativo o código entra pelo PYTHONPATH, sem instalação."""
    try:
        return version("voxel-worker")
    except PackageNotFoundError:
        return "0.1.0"


__version__ = package_version()
