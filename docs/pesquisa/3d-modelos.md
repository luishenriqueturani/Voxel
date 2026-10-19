# Modelos imagem → 3D na RX 7600

Não há NVIDIA nesta máquina (`nvidia-smi` ausente, sem `/dev/nvidia`). Tudo o que abaixo depende de uma placa NVIDIA fica para quando houver acesso. Os clones estão fora do git, em `~/.cache/voxel-pesquisa/src`.

| Repositório | Commit |
| --- | --- |
| VAST-AI-Research/TripoSR | `107cefd` |
| Stability-AI/stable-fast-3d | `ff21fc4` |
| Tencent-Hunyuan/Hunyuan3D-2 | `f8db630` |
| microsoft/TRELLIS | `442aa1e` |
| TencentARC/InstantMesh | `08822c5` |

O PyTorch é o 2.14.1 com HIP 7.2.53211. As execuções usam a ISA nativa, com `HSA_OVERRIDE_GFX_VERSION` desligado.

## TripoSR

A rede é PyTorch. A única extensão é o `torchmcubes`, e o 0.1.0 cai para marching cubes na CPU quando o módulo não tem kernel GPU. O build daqui ficou só na CPU. O `transformers` 5.x renomeia os pesos do ViT e o checkpoint não carrega; com 4.57.6 carrega.

Cena: imagem 512×512, fundo cinza e uma elipse clara, sem remoção de fundo. Marching cubes em 128³, chunk 8192. Os dois OBJ são válidos.

| Backend | Carga | Forward | Malha | Vértices | Faces |
| --- | --- | --- | --- | --- | --- |
| ROCm `gfx1102` | 4,0 s | 3,5 s | 0,5 s | 11494 | 22908 |
| CPU | 3,5 s | 7,8 s | 2,0 s | 11488 | 22908 |

No ROCm o SDPA avisou que a atenção eficiente ainda é experimental nesta GPU e seguiu sem ela. A malha saiu. Falta repetir esta cena numa NVIDIA.

## Stable Fast 3D

O checkpoint `stabilityai/stable-fast-3d` é gated. Com a licença aceita, o `model.safetensors` tem 4 024 289 892 bytes. A cena é a mesma imagem do TripoSR, com remoção de fundo, textura 1024 e sem remesh. Os dois GLB são válidos.

| Backend | Carga | Geração | Pico | Vértices | Faces |
| --- | --- | --- | --- | --- | --- |
| ROCm `gfx1102` | 7,1 s | 3,2 s | 6199 MB | 14357 | 24228 |
| CPU | 4,7 s | 38,8 s | — | 14814 | 24832 |

Sem `TORCH_ROCM_AOTRITON_ENABLE_EXPERIMENTAL=1`, a atenção cai no kernel matemático e pede 5,09 GiB com os pesos já ocupando 4,69 GiB. A placa tem 7,98 GiB e a geração aborta por falta de memória. Com a flag, a atenção experimental cabe e a malha sai. A primeira passagem, ainda compilando esses kernels, levou 81,2 s. O 3,2 s é a passagem seguinte.

As extensões:

- `uv_unwrapper` é C++ e importou.
- `texture_baker` tem kernel GPU em `baker_kernel.cu` e um fallback C++. O `setup.py` publicado só compila o GPU se `CUDA_HOME` existir. No PyTorch ROCm esse valor fica vazio, então o pacote publicado sai só em CPU. Um triângulo rasterizado em CPU cobriu 88 pixels de um mapa 16×16.
- Compilar o `.cu` com `hipcc` exige tratar o ROCm como backend GPU e passar `PYTORCH_ROCM_ARCH=gfx1102`, com o override desligado. Assim o mesmo triângulo rasterizou em `cuda:0`. Compilado sob o override `11.0.0` do fish, o código objeto aborta em `hip::StatCO::getStatFunc`.

Falta repetir esta cena numa NVIDIA.

## Dependência de CUDA

Nenhum destes três foi executado. A leitura é do código do commit acima.

**Hunyuan3D-2mini.** A forma (DiT + VAE) é PyTorch. O marching cubes padrão é o `skimage` (`mc_algo='mc'`, CPU). O caminho `dmc` pede o pacote `diso`, que é CUDA. A textura importa `custom_rasterizer`, e o `setup.py` só declara `CUDAExtension` com `rasterizer_gpu.cu`. Não há fallback CPU nem HIP. A forma pode ser portátil; a malha texturizada, do jeito que o repositório publica, não.

**TRELLIS.** O backend esparso padrão é `spconv`. A alternativa é `torchsparse`. Os dois são bibliotecas de convolução esparsa em CUDA, e não há um terceiro backend em PyTorch puro. O render usa `nvdiffrast` e, se estiver instalado, `diffoctreerast`. O `setup.sh` instala flash-attn, xformers, spconv, kaolin e nvdiffrast em cima de um toolkit CUDA. A geração em si depende de CUDA.

**InstantMesh.** O `requirements.txt` instala o `nvdiffrast` da NVIDIA. `src/utils/mesh_util.py` importa `nvdiffrast.torch` no carregamento do módulo, e o `run.py` importa esse utilitário. A difusão multi-view é diffusers, que poderia usar ROCm, mas o ponto de entrada oficial não sobe sem o rasterizador CUDA.
