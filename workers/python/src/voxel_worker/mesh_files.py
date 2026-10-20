"""GLB e OBJ mínimos, para o contrato e para o adaptador de teste."""

import json
import struct
from pathlib import Path


def write_pair(output: Path) -> tuple[Path, Path]:
    """Grava um triângulo nos dois formatos, ao lado do caminho pedido."""
    glb = output.with_suffix(".glb")
    obj = output.with_suffix(".obj")
    glb.parent.mkdir(parents=True, exist_ok=True)
    glb.write_bytes(triangle_glb())
    obj.write_text(triangle_obj(), encoding="utf-8")
    return glb, obj


def triangle_obj() -> str:
    return "v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n"


def triangle_glb() -> bytes:
    positions = struct.pack("<9f", 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0)
    indices = struct.pack("<3H", 0, 1, 2)
    blob = positions + indices + b"\x00\x00"
    payload = {
        "asset": {"version": "2.0"},
        "scene": 0,
        "scenes": [{"nodes": [0]}],
        "nodes": [{"mesh": 0}],
        "meshes": [{"primitives": [{"attributes": {"POSITION": 0}, "indices": 1}]}],
        "accessors": [
            {
                "bufferView": 0,
                "componentType": 5126,
                "count": 3,
                "type": "VEC3",
                "min": [0.0, 0.0, 0.0],
                "max": [1.0, 1.0, 0.0],
            },
            {"bufferView": 1, "componentType": 5123, "count": 3, "type": "SCALAR"},
        ],
        "bufferViews": [
            {"buffer": 0, "byteOffset": 0, "byteLength": len(positions)},
            {"buffer": 0, "byteOffset": len(positions), "byteLength": len(indices)},
        ],
        "buffers": [{"byteLength": len(blob)}],
    }
    encoded = json.dumps(payload, separators=(",", ":")).encode("utf-8")
    json_chunk = _chunk(b"JSON", encoded, b" ")
    bin_chunk = _chunk(b"BIN\x00", blob, b"\x00")
    total = 12 + len(json_chunk) + len(bin_chunk)
    header = struct.pack("<III", 0x46546C67, 2, total)
    return header + json_chunk + bin_chunk


def _chunk(kind: bytes, payload: bytes, pad: bytes) -> bytes:
    body = bytearray(payload)
    while len(body) % 4:
        body.append(pad[0])
    return struct.pack("<I", len(body)) + kind + bytes(body)
