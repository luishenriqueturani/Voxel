"""Sobe o worker em `127.0.0.1` e anuncia a porta com `VOXEL_READY`."""

import asyncio
import os
import socket
import sys

from voxel_worker.server import create_app


def require_token() -> str:
    token = os.environ.get("VOXEL_TOKEN", "")
    if token == "":
        print("VOXEL_TOKEN ausente", file=sys.stderr)
        raise SystemExit(2)
    return token


def bind_ready() -> socket.socket:
    sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    sock.bind(("127.0.0.1", 0))
    sock.listen(128)
    _host, port = sock.getsockname()
    print(f"VOXEL_READY {port}", flush=True)
    return sock


def main() -> None:
    import uvicorn

    token = require_token()
    sock = bind_ready()
    config = uvicorn.Config(create_app(token), log_level="warning", access_log=False)
    server = uvicorn.Server(config)
    asyncio.run(server.serve(sockets=[sock]))
