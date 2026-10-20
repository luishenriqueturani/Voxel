"""Servidor FastAPI do protocolo. Este processo só gera malha."""

import asyncio
import time
from pathlib import Path
from typing import Annotated

from fastapi import FastAPI, Query, Request, WebSocket
from fastapi.exceptions import RequestValidationError
from fastapi.responses import JSONResponse
from pydantic import BaseModel, ConfigDict, Field
from starlette.responses import Response

from voxel_worker.engine import MeshEngine, engine_from_env
from voxel_worker.jobs import Book, reporter
from voxel_worker.protocol import PATHS
from voxel_worker.triposr import MeshError

MESH_ONLY = "este worker só gera malha"


class _Body(BaseModel):
    model_config = ConfigDict(extra="forbid", populate_by_name=True)


class LoadBody(_Body):
    model_id: str = Field(alias="modelId")
    model_path: str = Field(alias="modelPath")
    kind: str


class CancelBody(_Body):
    job_id: str = Field(alias="jobId")


class MeshBody(_Body):
    job_id: str = Field(alias="jobId")
    image_path: str = Field(alias="imagePath")
    model_id: str = Field(alias="modelId")
    output_path: str = Field(alias="outputPath")


def create_app(token: str, engine: MeshEngine | None = None) -> FastAPI:
    app = FastAPI()
    book = Book()
    mesh = engine if engine is not None else engine_from_env()

    @app.exception_handler(RequestValidationError)
    async def invalid(_request: Request, _error: RequestValidationError) -> JSONResponse:
        return _message(400, "pedido fora do contrato")

    def guard(request: Request) -> JSONResponse | None:
        if request.headers.get("authorization") != f"Bearer {token}":
            return _message(401, "token recusado")
        return None

    @app.get(PATHS["health"])
    def health(request: Request) -> Response:
        refused = guard(request)
        if refused is not None:
            return refused
        return JSONResponse({"status": "ok"})

    @app.post(PATHS["modelsLoad"])
    def load_model(request: Request, body: LoadBody) -> Response:
        refused = guard(request)
        if refused is not None:
            return refused
        try:
            mesh.load(Path(body.model_path))
        except MeshError as error:
            return _message(400, str(error))
        return JSONResponse({"accepted": True})

    @app.post(PATHS["modelsUnload"])
    def unload_model(request: Request) -> Response:
        refused = guard(request)
        if refused is not None:
            return refused
        mesh.unload()
        return JSONResponse({"accepted": True})

    @app.post(PATHS["jobsCancel"])
    def cancel_job(request: Request, body: CancelBody) -> Response:
        refused = guard(request)
        if refused is not None:
            return refused
        book.cancel(body.job_id)
        return JSONResponse({"accepted": True})

    @app.post(PATHS["meshesFromImage"])
    def from_image(request: Request, body: MeshBody) -> Response:
        refused = guard(request)
        if refused is not None:
            return refused
        return _mesh(book, mesh, body)

    def refuse(request: Request) -> Response:
        if request.headers.get("authorization") != f"Bearer {token}":
            return _message(401, "token recusado")
        return _message(400, MESH_ONLY)

    for path in (
        PATHS["imagesTxt2img"],
        PATHS["imagesImg2img"],
        PATHS["imagesInpaint"],
        PATHS["imagesDecodeLatents"],
        PATHS["imagesRemoveBackground"],
        PATHS["meshesPostProcess"],
    ):
        app.add_api_route(path, refuse, methods=["POST"], name=path)

    @app.websocket(PATHS["events"])
    async def events(
        websocket: WebSocket,
        job_id: Annotated[str, Query(alias="jobId")],
    ) -> None:
        if websocket.headers.get("authorization") != f"Bearer {token}":
            await websocket.close(code=1008)
            return
        await websocket.accept()
        await _replay(websocket, book, job_id)

    return app


def _mesh(book: Book, engine: MeshEngine, body: MeshBody) -> Response:
    job = book.open(body.job_id)
    try:
        path = engine.image_to_mesh(
            Path(body.image_path),
            Path(body.output_path),
            job.cancelled,
            reporter(book, body.job_id),
        )
    except MeshError as error:
        book.push(body.job_id, {"event": "failed", "jobId": body.job_id, "message": str(error)})
        book.finish(body.job_id)
        status = 400 if str(error) == "cancelado" else 500
        return _message(status, str(error))
    text = str(path)
    book.push(body.job_id, {"event": "completed", "jobId": body.job_id, "path": text})
    book.finish(body.job_id)
    return JSONResponse({"jobId": body.job_id, "path": text})


async def _replay(websocket: WebSocket, book: Book, job_id: str) -> None:
    sent = 0
    deadline = time.monotonic() + 5
    while True:
        events, done, exists = book.snapshot(job_id)
        while sent < len(events):
            await websocket.send_json(events[sent])
            sent += 1
        if done or (not exists and time.monotonic() >= deadline):
            break
        await asyncio.sleep(0.02)
    await websocket.close()


def _message(status: int, message: str) -> JSONResponse:
    return JSONResponse(status_code=status, content={"message": message})
