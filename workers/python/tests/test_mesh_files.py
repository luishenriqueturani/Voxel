from pathlib import Path

from voxel_worker.mesh_files import triangle_glb, triangle_obj, write_pair


def test_o_par_grava_glb_e_obj(tmp_path: Path) -> None:
    glb, obj = write_pair(tmp_path / "modelo.glb")

    assert glb.read_bytes().startswith(b"glTF")
    assert "f 1 2 3" in obj.read_text(encoding="utf-8")
    assert triangle_glb().startswith(b"glTF")
    assert triangle_obj().startswith("v 0 0 0")
