# stable-diffusion.cpp na RX 7600

Registro da fase 1 para os itens de 2D do `docs/TODO.md`. O código do upstream fica em `workers/sdcpp/stable-diffusion.cpp` e não entra no git. Os binários ficam em `workers/sdcpp/build/`.

Commit testado: `b13a482` (clone raso de 19/10/2026).

## Máquina

| Peça | Valor |
| --- | --- |
| CPU | AMD Ryzen 7 5700X, 8 núcleos / 16 threads, AVX2, sem AVX-512 |
| RAM | 32 GB |
| GPU | AMD Radeon RX 7600, Navi 33, `deviceID` `0x7480` |
| Vulkan | Mesa RADV 26.2.4, API 1.4.354, dispositivo `AMD Radeon RX 7600 (RADV NAVI33)` |
| ROCm | 7.2.4, com bibliotecas rocBLAS para `gfx1102` |

Sem `HSA_OVERRIDE_GFX_VERSION`, o `rocminfo` mostra a ISA real `gfx1102`. Com o override `11.0.0` que está exportado nas variáveis universais do fish, o mesmo comando passa a anunciar `gfx1100`. O ambiente também exporta `PYTORCH_ROCM_ARCH=gfx1100`, `MIOPEN_DEBUG_DISABLE_ASM_KERNELS=1`, `MIOPEN_DEBUG_CONV_IMPLICIT_GEMM=0`, `MIOPEN_DISABLE_CACHE=1` e `MIOPEN_FIND_ENFORCE=3`.

## Build Vulkan

O pacote de sistema `vulkan-headers` e o `spirv-headers` não estão instalados. O loader (`libvulkan.so`), o `glslc` e o `glslang` estão. A configuração abaixo usa cabeçalhos baixados só para este build:

```
cmake -S workers/sdcpp/stable-diffusion.cpp -B workers/sdcpp/build/vulkan \
  -DSD_VULKAN=ON -DSD_SERVER_BUILD_FRONTEND=OFF -DCMAKE_BUILD_TYPE=Release \
  -DVulkan_INCLUDE_DIR="$PWD/workers/sdcpp/build/vulkan-headers/include" \
  -DVulkan_LIBRARY=/usr/lib/libvulkan.so \
  -DCMAKE_PREFIX_PATH="$PWD/workers/sdcpp/build/prefix" \
  -DCMAKE_CXX_FLAGS="-I$PWD/workers/sdcpp/build/prefix/include" \
  -DCMAKE_C_FLAGS="-I$PWD/workers/sdcpp/build/prefix/include"
```

O binário `workers/sdcpp/build/vulkan/bin/sd-cli` sobe e lista a RX 7600:

```
Vulkan0  AMD Radeon RX 7600 (RADV NAVI33)
CPU      AMD Ryzen 7 5700X 8-Core Processor
```

## Build CPU

`workers/sdcpp/build/cpu` sem `SD_VULKAN`. O binário só lista o Ryzen 7 5700X.

## Build ROCm

O binário HIP foi gerado para a ISA nativa da placa. A execução precisa do override desligado (`env -u HSA_OVERRIDE_GFX_VERSION -u PYTORCH_ROCM_ARCH`); com `11.0.0` o runtime procura `gfx1100`.

```
cmake -S workers/sdcpp/stable-diffusion.cpp -B workers/sdcpp/build/hip \
  -DSD_HIPBLAS=ON -DAMDGPU_TARGETS=gfx1102 -DSD_SERVER_BUILD_FRONTEND=OFF \
  -DCMAKE_BUILD_TYPE=Release -DCMAKE_PREFIX_PATH=/opt/rocm
```

`--list-devices` desse binário, com o override removido:

```
ROCm0  AMD Radeon RX 7600, gfx1102 (0x1102), wave 32, 8176 MiB
CPU    AMD Ryzen 7 5700X 8-Core Processor
```

Não houve `SIGABRT` nem `HSA_STATUS_ERROR_INVALID_ISA`.

## SD 1.5

Modelo: `stable-diffusion-v1-5/stable-diffusion-v1-5`, arquivo `v1-5-pruned-emaonly.safetensors` (4 265 146 304 bytes), em `models/sd15/` (fora do git).

Cena comum: 512×512, 4 passos, Euler A, seed 1, prompt "a red square on a white background". Os três PNG são válidos.

| Backend | Pesos | `generate_image` | Amostragem | Decode |
| --- | --- | --- | --- | --- |
| ROCm `gfx1102` | 2784 MB na VRAM (UNet, CLIP e VAE) | 3,34 s | 2,37 s | 0,69 s |
| Vulkan | UNet e CLIP na VRAM, VAE na RAM | 7,10 s | 4,52 s | 1,13 s |
| CPU | 2784 MB na RAM | 65,06 s | 42,70 s | 22,00 s |

A medição acima é com o jogo fechado. O desktop ocupa cerca de 1,6 GB (`mem_info_vram_used` ~1665 MB de 8573 MB). Uma passagem anterior, com o Genshin Impact aberto e ~5,4 GB já ocupados, deixou todos os pesos do Vulkan na RAM: um segmento (`unet.output_blocks.5`) não coube, o passo seguinte completou e a imagem saiu em 18,37 s. Esse número fica de fora.

O auto-fit do ROCm enxergou 8076 MiB livres e coube o modelo inteiro. O do Vulkan enxergou 6588 MiB e reservou o VAE para a RAM. O RADV conta a memória do desktop; o ROCm, nesta máquina, não. Uma passagem intermediária, já com ~1,6 GB ocupados, tinha dado 4,16 s no ROCm e 6,67 s no Vulkan.

SDXL e Flux quantizado ficam para uma passagem seguinte. O SD 1.5 já compara os três backends; esses dois modelos é que apertam os 8 GB.

## Img2img, inpainting e textura tileável

As flags existem neste binário e as três rodaram no HIP, na mesma cena de 4 passos:

- Img2img: `-i` / `--init-img` com `--strength 0.4` (o padrão da flag é 0.75). Completou em 2,99 s.
- Inpainting: `--init-img` com `--mask` (máscara 512×512, retângulo branco no centro). Completou em 3,82 s. Um modelo de inpaint dedicado exige os dois; sem máscara o pipeline usa uma máscara cheia.
- Textura tileável: `--circular` (também existem `--circularx` e `--circulary`). Completou em 3,53 s.
- Memória do VAE: `--vae-tiling`, com tile padrão de 256×256 pixels. Não foi necessário nesta cena.

## Latentes separados

A API pública (`include/stable-diffusion.h`) só expõe `generate_image`. Ela codifica, faz o denoising e decodifica no mesmo processo. Não há função nem flag para gravar o latente e decodificar depois, noutro processo. O fallback previsto no planejamento (§6.3, item 5) não dá para montar em cima deste binário sem mudar o upstream. O que existe hoje para o VAE estourar memória é `--vae-tiling`.
