import pytest

from voxel_worker.boot import bind_ready, require_token


def test_sem_token_nao_sobe(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.delenv("VOXEL_TOKEN", raising=False)
    with pytest.raises(SystemExit):
        require_token()


def test_anuncia_a_porta_antes_de_servir(capsys: pytest.CaptureFixture[str]) -> None:
    sock = bind_ready()
    try:
        announced = capsys.readouterr().out.strip()
        _host, port = sock.getsockname()
    finally:
        sock.close()
    assert announced == f"VOXEL_READY {port}"
    assert isinstance(port, int)
