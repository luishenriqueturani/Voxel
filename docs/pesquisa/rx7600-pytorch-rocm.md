# PyTorch + ROCm na RX 7600

Registro do caso §6.2. PyTorch do sistema: 2.14.1, HIP 7.2.53211, ROCm 7.2.4. A convolução abaixo é a operação do VAE decode que aborta no log do InvokeAI. Não rodei o VAE `AutoencoderKLWan` do modelo Anima.

## O que a placa reporta

`torch.cuda.get_arch_list()` deste build inclui `gfx1102`.

| Ambiente | `gcnArchName` | Conv fp16 |
| --- | --- | --- |
| Sem `HSA_OVERRIDE_GFX_VERSION` | `gfx1102` | Completa |
| `HSA_OVERRIDE_GFX_VERSION=11.0.0` | `gfx1100` | Aborta |

O override não é necessário neste ROCm. O build já tem a ISA nativa da RX 7600.

## O crash

Com o override e o restante das variáveis da MIOpen desligadas, `conv2d` fp16 morre dentro do `miopenFindConvolutionForwardAlgorithm`:

```
HSA_STATUS_ERROR_INVALID_ISA: The instruction set architecture is invalid. code: 0x100f
```

O processo sai com código 134 (sinal 6, `SIGABRT`). Acontece no filtro 2×3 com 16 canais, que é a forma do kernel `miopenSp3AsmConvFury_..._1536vgprs` do log original, e também num 3×3 com 128 canais. Tiles de 8, 16, 32 e 64 no filtro 2×3 abortam do mesmo jeito. Partir o VAE em tiles não desvia esse kernel.

## O que o ambiente do fish faz hoje

As variáveis universais exportam `MIOPEN_DEBUG_DISABLE_ASM_KERNELS=1`, `MIOPEN_DEBUG_CONV_IMPLICIT_GEMM=0`, `MIOPEN_DISABLE_CACHE=1` e `MIOPEN_FIND_ENFORCE=3`, junto com o override. Nesta `libMIOpen` a variável `MIOPEN_DEBUG_DISABLE_ASM_KERNELS` não existe. `MIOPEN_DEBUG_CONV_IMPLICIT_GEMM=0` também não segura o crash: a mesma conv aborta com a combinação inteira do fish.

A variável que existe e desliga os kernels em assembly é `MIOPEN_DEBUG_GCN_ASM_KERNELS=0`.

## Mitigações

Cena de tempo: conv 3×3, 512 canais, 128×128, média de 5 chamadas depois de um aquecimento. A referência é fp16 nativo, sem override: 2,13 ms.

| Configuração | Resultado | Tempo |
| --- | --- | --- |
| Sem override, fp16, MIOpen ligado | Completa | 2,13 ms |
| Override e `MIOPEN_DEBUG_GCN_ASM_KERNELS=0` | Completa | 3,15 ms |
| Override e `torch.backends.cudnn.enabled = False` | Completa | 3,50 ms |
| Sem override, fp32, MIOpen ligado | Completa | 3,98 ms |
| Override, fp32 | Completa | — |
| CPU, conv menor (128 canais, 64×64) | Completa | 2657 ms |

O fp32 com override foi medido só na conv menor, onde também completa (0,11 ms, igual ao fp32 nativo nessa cena). A CPU, na conv de 128 canais, leva 2657 ms contra 0,10 ms do fp16 nativo na GPU.

## Combinação que vale a pena

Não exportar `HSA_OVERRIDE_GFX_VERSION`. O PyTorch 2.14.1 enxerga `gfx1102` e a conv fp16 termina sem abortar. Essa é a configuração rápida.

Se algum outro pacote ainda exigir o override, o fallback que segura este crash é `MIOPEN_DEBUG_GCN_ASM_KERNELS=0`. Na conv grande isso custa cerca de 48% (3,15 ms contra 2,13 ms). Desligar a MIOpen inteira custa cerca de 64% (3,50 ms). fp32 custa cerca de 87% (3,98 ms). A CPU fica ordens de grandeza atrás.
