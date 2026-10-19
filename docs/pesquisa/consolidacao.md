# Consolidação: memória, licenças e MVP

A cena 2D é a mesma de antes: SD 1.5, 512×512, 4 passos, Euler A, seed 1. A cena 3D é a elipse de 512×512. O desktop ocupava cerca de 1,5 GB dos 8176 MiB da RX 7600. A RAM do processo é o pico de RSS da árvore; a VRAM é o pico de `mem_info_vram_used` menos a base, e também o valor absoluto.

SDXL, Flux, SD 3.5 e qualquer combinação NVIDIA não entram na tabela. Os dois primeiros continuam pendentes no TODO. Hunyuan3D, TRELLIS e InstantMesh não executam nesta máquina.

## Medições

| Combinação | Tempo | RAM | VRAM |
| --- | --- | --- | --- |
| SD 1.5, Vulkan | geração 6,24 s | 1,3 GB | pico 5,0 GB (delta 3,5 GB); os 2784 MB de pesos couberam na placa |
| SD 1.5, ROCm `gfx1102` | geração 3,38 s | 1,5 GB | pico 5,4 GB (delta 3,9 GB); pesos inteiros na placa |
| SD 1.5, CPU | geração 31,8 s | 4,5 GB | nenhuma |
| TripoSR, ROCm | carga 5,8 s, forward 3,1 s, malha 0,3 s | 4,4 GB | pico 3,8 GB (delta 2,4 GB) |
| TripoSR, CPU | carga 3,9 s, forward 7,7 s, malha 1,6 s | 4,4 GB | nenhuma |
| Stable Fast 3D, ROCm | carga 6,1 s; 3,2 s e depois 1,3 s no mesmo processo | 11,2 GB | 6199 MB alocados pelo PyTorch |
| Stable Fast 3D, CPU | carga 7,5 s, malha 39 s | 14,3 GB | nenhuma |
| U²-Net, CPU | carga 0,29 s, inferência 0,24 s | 0,6 GB | nenhuma |
| IS-Net, CPU | carga 0,36 s, inferência 0,62 s | 0,9 GB | nenhuma |
| BiRefNet lite, CPU | carga 2,4 s, inferência 5,5 s | 7,4 GB | nenhuma |
| BiRefNet, CPU | carga 4,2 s, inferência 9,9 s | 9,1 GB | nenhuma |

Uma passagem anterior do SD 1.5 em CPU, sem o grafo em cache, levou 65 s. O número de 31,8 s é o desta passagem. No Vulkan, com menos VRAM livre, o auto-fit já chegou a deixar o VAE na RAM; aqui, com 6676 MiB livres, os três componentes foram para a placa.

No Stable Fast 3D a primeira geração deste processo leva 3,2 s e a seguinte 1,3 s, com pico alocado de 6199 MB. Isso só acontece com `TORCH_ROCM_AOTRITON_ENABLE_EXPERIMENTAL=1` e com o cache da MIOpen ligado. Sem a flag, a atenção pede 5 GB além dos pesos e a placa de 8 GB estoura, como já registrado. Com `MIOPEN_DISABLE_CACHE=1`, que está nas variáveis universais do fish, cada processo paga cerca de 79 s e o sysfs viu a placa em 8100 MB de 8176. O worker não deve herdar essa variável. Os 11,2 GB e os 14,3 GB de RAM incluem o `rembg` que o script oficial carrega antes da malha.

## Licenças

Conferido no card do modelo ou no `LICENSE` do repositório, em 19/10/2026.

| Modelo | Licença | No MVP |
| --- | --- | --- |
| SD 1.5 | CreativeML OpenRAIL-M. Uso comercial permitido, com restrições de uso | Sim, modelo 2D |
| SDXL | CreativeML Open RAIL++-M. A mesma família, com as restrições repassadas a derivados | Não, ainda sem medição |
| SD 3.5 Medium | Stability AI Community License. Grátis abaixo de US$ 1 milhão de receita anual, com atribuição | Não, sem medição |
| FLUX.1 schnell | Apache 2.0 | Não, sem medição |
| FLUX.1 dev | FLUX.1 [dev] Non-Commercial License. Só uso não comercial e fora de produção | Não |
| TripoSR | MIT, no repositório e no card | Sim, modelo 3D |
| Stable Fast 3D | Stability AI Community License, a mesma do SD 3.5 | Opção texturizada do perfil de 8 GB |
| Hunyuan3D 2.0 | Tencent Hunyuan Community. Não vale na União Europeia, no Reino Unido nem na Coreia do Sul | Não |
| TRELLIS | MIT para o modelo e a maior parte do código. Submódulos CUDA têm licença própria | Não, depende de CUDA |
| InstantMesh | Apache 2.0 | Não, o ponto de entrada importa CUDA |
| U²-Net e IS-Net (DIS) | Apache 2.0 | U²-Net é o removedor |
| BiRefNet | MIT | Não entra como padrão: 7–9 GB para a mesma máscara nesta cena |
| RMBG / BRIA | Não comercial | Fora |

O aplicativo mostra a licença antes do download. OpenRAIL e a licença comunitária da Stability não são “não comerciais”, mas têm restrições que o usuário precisa ver. FLUX.1 dev e RMBG ficam de fora por isso.

## MVP

- **2D.** stable-diffusion.cpp. Vulkan é o padrão, porque rodou o SD 1.5 inteiro nesta placa sem ROCm. ROCm é o caminho mais rápido quando a ISA nativa existe e `HSA_OVERRIDE_GFX_VERSION` está desligado: 3,4 s contra 6,2 s. CPU é o último recurso e cabe em 16 GB.
- **3D.** PyTorch, com ROCm nativo nesta placa e CPU em qualquer máquina. O modelo é o TripoSR. O Stable Fast 3D é a opção com UV e textura para quem tem 8 GB, aceita a licença comunitária e liga a atenção experimental.
- **Fundo.** U²-Net em CPU, pela crate `ort`.
- **Malha.** `meshoptimizer` no core.

Os perfis preenchidos com esses limites estão em `docs/PLANEJAMENTO.md`, §6.6.
