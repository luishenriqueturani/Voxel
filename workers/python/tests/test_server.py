from pathlib import Path

import pytest
from fastapi.testclient import TestClient
from starlette.websockets import WebSocketDisconnect

from voxel_worker.engine import FakeEngine
from voxel_worker.server import MESH_ONLY, create_app

TOKEN = "segredo"
HEADERS = {"Authorization": f"Bearer {TOKEN}"}


def _client(tmp_path: Path) -> tuple[TestClient, Path, Path]:
    model = tmp_path / "model.ckpt"
    image = tmp_path / "foto.png"
    model.write_bytes(b"peso")
    image.write_bytes(b"png")
    return TestClient(create_app(TOKEN, FakeEngine())), model, image


def test_saude_malha_e_eventos(tmp_path: Path) -> None:
    client, model, image = _client(tmp_path)
    with client:
        assert client.get("/health").status_code == 401
        assert client.get("/health", headers=HEADERS).json() == {"status": "ok"}
        loaded = client.post(
            "/models/load",
            headers=HEADERS,
            json={"modelId": "triposr", "modelPath": str(model), "kind": "mesh"},
        )
        assert loaded.json() == {"accepted": True}
        output = tmp_path / "saida.glb"
        response = client.post(
            "/meshes/from-image",
            headers=HEADERS,
            json={
                "jobId": "job-1",
                "imagePath": str(image),
                "modelId": "triposr",
                "outputPath": str(output),
            },
        )
        assert response.status_code == 200
        assert response.json()["path"] == str(output)
        assert output.read_bytes().startswith(b"glTF")
        assert "f 1 2 3" in output.with_suffix(".obj").read_text(encoding="utf-8")
        with client.websocket_connect("/events?jobId=job-1", headers=HEADERS) as socket:
            names = [socket.receive_json()["event"] for _ in range(3)]
        assert names == ["progress", "progress", "completed"]


def test_rotas_de_imagem_recusam_e_o_cancelamento_vale(tmp_path: Path) -> None:
    client, model, image = _client(tmp_path)
    with client:
        assert client.post("/images/txt2img").status_code == 401
        refused = client.post("/images/txt2img", headers=HEADERS, json={})
        assert refused.status_code == 400
        assert refused.json()["message"] == MESH_ONLY
        assert client.post("/models/load", headers=HEADERS, json={"extra": 1}).status_code == 400
        missing = client.post(
            "/models/load",
            headers=HEADERS,
            json={"modelId": "triposr", "modelPath": str(tmp_path / "nao.ckpt"), "kind": "mesh"},
        )
        assert missing.status_code == 400
        client.post("/jobs/cancel", headers=HEADERS, json={"jobId": "job-2"})
        client.post(
            "/models/load",
            headers=HEADERS,
            json={"modelId": "triposr", "modelPath": str(model), "kind": "mesh"},
        )
        cancelled = client.post(
            "/meshes/from-image",
            headers=HEADERS,
            json={
                "jobId": "job-2",
                "imagePath": str(image),
                "modelId": "triposr",
                "outputPath": str(tmp_path / "outro.glb"),
            },
        )
        assert cancelled.status_code == 400
        assert cancelled.json()["message"] == "cancelado"
        with (
            pytest.raises(WebSocketDisconnect),
            client.websocket_connect(
                "/events?jobId=ausente",
                headers={"Authorization": "errado"},
            ),
        ):
            pass
