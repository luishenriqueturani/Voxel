from voxel_worker.protocol import PATHS, Txt2ImgRequest


def test_contrato_de_texto_para_imagem() -> None:
    pedido: Txt2ImgRequest = {
        "jobId": "job",
        "prompt": "um gato",
        "modelId": "sd",
        "outputPath": "/tmp/out.png",
        "steps": 2,
        "width": 8,
        "height": 8,
        "seed": 1,
    }

    assert PATHS["imagesTxt2img"] == "/images/txt2img"
    assert pedido["prompt"] == "um gato"
