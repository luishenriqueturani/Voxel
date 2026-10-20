# Formatos e quantização na busca de modelos

Nota para quando o selo deixar de estimar compatibilidade só pelo tamanho. A busca do Hugging Face já lista `.safetensors` e `.gguf` de repositórios que o Voxel ainda não mede. O exemplo abaixo veio de `abenzergs/Qwen-Image-2.1-Uncensored-Quantized` em 20/10/2026.

Hoje o gerador 2D é o stable-diffusion.cpp com o Stable Diffusion 1.5 em `.safetensors` (`v1-5-pruned-emaonly.safetensors`). Um arquivo baixado pela busca tem a arquitetura lida do cabeçalho e o selo estimado pelo tamanho contra a VRAM. Isso não diz se o runtime abre aquele formato.

## A caixa do arquivo

**Safetensors** guarda tensores nomeados, com um cabeçalho em texto. É o formato do PyTorch, do Diffusers e do stable-diffusion.cpp. O arquivo não executa código. Dois `.safetensors` do mesmo desenho podem exigir runtimes diferentes se a quantização for outra.

**GGUF** é o formato do llama.cpp e dos programas da mesma família. O arquivo traz a arquitetura e o tipo de quantização junto com os pesos. Foi feito para inferência. Não substitui um `.safetensors` do mesmo modelo: cada runtime abre a sua caixa.

## O que o nome costuma dizer

O número é, em geral, quantos bits ficam em cada peso. Menos bits deixam o arquivo menor e mais leve na memória, com mais perda. Mais bits ficam mais perto do original e ocupam mais.

| Sigla | Para que serve |
| --- | --- |
| MLX-4bit, MLX-6bit, MLX-8bit | Quantização do MLX, o framework da Apple. Os `.safetensors` com esse prefixo são para Mac com Apple Silicon. |
| NVFP4 | Ponto flutuante de 4 bits da NVIDIA, para placas NVIDIA recentes. |
| Q4, Q5, Q6, Q8 | Quantização GGUF. Q4 é o menor; Q8 é o mais fiel e o maior. |
| Q4_0 | 4 bits no esquema antigo, em blocos uniformes. |
| Q4_K_S, Q4_K_M, Q5_K_M, Q6_K | Esquema mais novo: camadas mais importantes ficam com mais bits. S é a variante menor, M a média (o equilíbrio usual), L a maior. |
| FP16, BF16 | 16 bits, quase sem compressão. É o arquivo grande de referência. |
| GPTQ, AWQ | Outra compressão, em geral dentro de `.safetensors`, para runtimes de GPU como o vLLM. |

No nome `qwen-image-2.1-UC-Q4_K_M.gguf`, UC é o variante sem censura daquele repositório, Q4_K_M é a compressão e `.gguf` é a caixa. `qwen-image-2.1-UC-MLX-4bit.safetensors` é o mesmo desenho de modelo noutro programa e noutro hardware.

## O que isso muda no selo

Na RX 7600 (Navi 33, Vulkan, sem NVIDIA e sem Apple Silicon), MLX e NVFP4 não rodam. Entre GGUF, Q4_K_M é o meio-termo; Q5 e Q6 sobem qualidade e tamanho; Q8 chega perto do original.

Um selo futuro precisa separar três coisas que hoje se misturam no tamanho do arquivo:

1. A caixa (`.safetensors` ou `.gguf`) e se o runtime instalado a abre.
2. O hardware da quantização (MLX, NVFP4, GPTQ/AWQ, GGUF Q*).
3. O custo de memória, que deixa de ser o tamanho do arquivo inteiro quando o peso já está comprimido.
