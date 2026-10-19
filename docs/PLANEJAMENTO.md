# Voxel — Planejamento do Projeto

> Aplicação desktop (Tauri) para Linux, de código aberto, para geração e edição de **imagens 2D** e geração de **modelos 3D** com IA rodando **100% local**, funcionando bem em hardware comum (não apenas em RTX/Radeon de última geração).

Legenda usada neste documento:

- **[Ideia original]** — definido pelo autor do projeto.
- **[Sugestão]** — proposta adicional, ainda a ser aprovada ou descartada.
- **[Backlog]** — aprovado como ideia, mas sem data; reavaliar no futuro.

## Decisões tomadas

| Tema | Decisão |
| --- | --- |
| Distribuição | Projeto pessoal, **não comercial**, código aberto (MIT) no GitHub; quem quiser usar, baixa, compila e usa |
| Plataforma | **Somente Linux** (não há planos para Windows/macOS) |
| Frontend | React + TypeScript + Vite |
| Idiomas | pt-BR e inglês desde o início |
| Backend 2D padrão | stable-diffusion.cpp com Vulkan (recomendado), com opção de o usuário escolher outro backend ou modelo |
| Ambiente Python | Configurado automaticamente quando o usuário demonstra intenção de gerar 3D |
| Erros | Modal com o log ao quebrar + arquivo de log de erros |
| Compatibilidade de modelos | Avisar antes do download quando o modelo não é compatível com o hardware atual |
| MVP | Inclui remoção de fundo (também como ferramenta de edição rápida) |
| Versão 1.0 | 3D, img2img/inpainting, texturas tileáveis, fluxos, visualizador 3D, galeria |
| Fluxos | Templates reutilizáveis, sobrescrita de parâmetros por item, modo 3D "Escolher depois", execução **intercalada** (imagem → 3D → próximo item) |
| Pós-processamento 3D | Opção do usuário, desligada por padrão |
| Painel esquerdo | Prompt negativo recolhível, 3D a partir de imagem existente, busca e favoritos nos recentes |
| Execução de filas | Impedir suspensão e ação ao terminar (notificar / suspender / desligar) |
| Modelo incompatível | Download permitido somente após aviso e confirmação explícita |
| Após a 1.0 | Rigging automático, API local/CLI |
| Backlog | Importação CSV, wildcards, presets de estilo, LoRAs, upscale |
| Descartado | Estimativa de tempo (ETA), ordem de execução agrupada |

---

## Sumário

1. [Visão e público-alvo](#1-visão-e-público-alvo)
2. [Princípios do projeto](#2-princípios-do-projeto)
3. [Interface](#3-interface)
4. [Funcionalidades](#4-funcionalidades)
5. [Fluxos de geração (batch)](#5-fluxos-de-geração-batch)
6. [Compatibilidade de hardware e setup automático](#6-compatibilidade-de-hardware-e-setup-automático)
7. [Arquitetura técnica](#7-arquitetura-técnica)
8. [Tratamento de erros e logs](#8-tratamento-de-erros-e-logs)
9. [Modelos de IA candidatos](#9-modelos-de-ia-candidatos)
10. [Dados e armazenamento](#10-dados-e-armazenamento)
11. [Estratégia de testes](#11-estratégia-de-testes)
12. [Qualidade, CI e distribuição](#12-qualidade-ci-e-distribuição)
13. [Roadmap](#13-roadmap)
14. [Riscos conhecidos](#14-riscos-conhecidos)
15. [Questões em aberto](#15-questões-em-aberto)

---

## 1. Visão e público-alvo

**Visão:** uma ferramenta simples de usar, que a pessoa compila, abre e já consegue gerar imagens e modelos 3D sem precisar entender Python, CUDA, ROCm, VRAM ou quantização. A aplicação descobre o hardware e se configura sozinha.

**Público principal:**

- **Desenvolvedores de jogos independentes** — precisam de muitos assets (NPCs, props, itens, texturas), muitas vezes genéricos, e não têm orçamento para modelagem manual de tudo.
- **Artistas e hobbystas** — querem explorar ideias, gerar referências e variações, e fazer edições rápidas.
- **Usuários Linux com hardware modesto** — GPUs de entrada/intermediárias (ex.: RX 7600, RTX 3060, GTX 1660, Intel Arc) ou até apenas CPU.

**Caso de uso âncora [Ideia original]:** o dev configura um fluxo com 30 NPCs, clica em "Executar", deixa o PC trabalhando durante a noite e no dia seguinte tem todas as imagens e modelos 3D prontos.

---

## 2. Princípios do projeto

1. **Simplicidade primeiro** — o caminho padrão (digitar prompt → gerar) deve exigir zero configuração. Opções avançadas ficam atrás de botões de configuração.
2. **Funciona no hardware do usuário** — preferir backends portáteis (Vulkan) e degradar com elegância (modelos menores, quantizados, offload para RAM, CPU) em vez de quebrar.
3. **Nunca falhar em silêncio** — quando algo quebrar, o usuário vê o motivo (modal com log) e o erro fica registrado em arquivo.
4. **100% local e offline após o setup** — nenhum prompt ou imagem sai da máquina. Rede apenas para baixar modelos e dependências.
5. **Robustez para execuções longas** — uma fila de 10 horas não pode ser perdida por um erro em um item, falta de memória ou reinício do PC.
6. **Tudo o que desenvolvermos tem testes unitários [Ideia original]** — bibliotecas de terceiros não são testadas diretamente, mas nossas integrações com elas sim (com mocks).
7. **Reprodutibilidade** — toda geração guarda prompt, seed, modelo e parâmetros, permitindo regerar o mesmo resultado.

---

## 3. Interface

### 3.1 Layout geral

```
┌──────────────────────────────────────────────────────────────────────┐
│ Arquivo  Editar  Gerar  Fluxos  Modelos  Exibir  Ajuda               │  ← menu nativo
├────────────────────────┬─────────────────────────────────────────────┤
│ ┌────────────────────┐ │                                             │
│ │ Prompt...          │ │                                             │
│ │                    │ │                                             │
│ └────────────────────┘ │                                             │
│ Modelo: [SDXL Q8  ▾][⚙]│          Área principal                     │
│ Saída:  (•) 2D  ( ) 3D │   (visualização da imagem / editor /        │
│ [      Gerar       ]   │    viewer 3D / galeria / editor de fluxo)   │
│────────────────────────│                                             │
│ Prompts recentes       │                                             │
│  • guerreiro orc ...   │                                             │
│  • baú de madeira ...  │                                             │
│  • aldeã medieval ...  │                                             │
│  • ...                 │                                             │
├────────────────────────┴─────────────────────────────────────────────┤
│ GPU: RX 7600 (Vulkan) · VRAM 3.1/8 GB · Fila: 4/30 · Ambiente 3D: ok │  ← barra de status
└──────────────────────────────────────────────────────────────────────┘
```

### 3.2 Painel esquerdo [Ideia original]

De cima para baixo:

1. **Campo de prompt** — multilinha, já focado ao abrir. `Ctrl+Enter` dispara a geração.
2. **Seleção de modelo** — dropdown com os modelos instalados e compatíveis com o modo escolhido (2D/3D).
3. **Botão de configurações do modelo (⚙)** — abre um popover/diálogo com parâmetros: resolução, passos, CFG, sampler, seed, prompt negativo, quantidade de imagens etc. Valores padrão vêm do perfil de hardware.
4. **Opção 2D ou 3D** — seleção marcável.
   - **2D:** gera a imagem.
   - **3D:** gera a imagem e, em seguida, o modelo 3D a partir dela (texto → imagem → 3D). Marcar esta opção pela primeira vez dispara a configuração automática do ambiente 3D (ver 7.3).
5. **Lista dos últimos prompts** — clique carrega o prompt (e opcionalmente os parâmetros) de volta no campo.

Complementos do painel:

- **Prompt negativo** recolhível abaixo do prompt principal.
- **3D a partir de imagem existente** — arrastar uma imagem (gerada ou do disco) para o painel e gerar só o 3D.
- **Busca e fixação (★) na lista de recentes**, para prompts favoritos não sumirem.
- **[Backlog]** Presets de estilo (ex.: "Low poly", "Pixel art", "Realista", "Cartoon") que adicionam termos ao prompt.

### 3.3 Área principal

- **Visualizador 2D** — imagem atual com zoom, comparar variações lado a lado, ações: salvar, copiar, regerar com mesma seed, variar, remover fundo, editar (img2img/inpainting), enviar para 3D.
- **Editor rápido de imagem** — remoção de fundo (MVP), e na 1.0 máscara com pincel para inpainting (ver 4.2).
- **Visualizador 3D** — viewer interativo (orbitar, zoom, wireframe, ver textura, contagem de polígonos) usando three.js.
- **Galeria** — histórico de tudo gerado, com filtros por data, modelo, fluxo e tipo (2D/3D).
- **Editor de fluxos** — ver seção 5.

### 3.4 Menu superior nativo [Ideia original]

Usando a API de menu nativa do Tauri 2 (menu real da janela, não simulado em HTML).

| Menu | Itens |
| --- | --- |
| **Arquivo** | Novo fluxo, Abrir fluxo, Salvar fluxo, Abrir imagem para edição, Exportar resultados, Abrir pasta de saída, Sair |
| **Editar** | Desfazer, Refazer, Copiar, Colar, Remover fundo, Preferências |
| **Gerar** | Gerar, Cancelar geração atual, Regerar última |
| **Fluxos** | Gerenciar fluxos, Gerenciar templates, Fila de execução, Pausar/Retomar fila |
| **Modelos** | Gerenciar modelos (baixar, remover, atualizar), Importar modelo local, Abrir pasta de modelos |
| **Exibir** | Galeria, Visualizador, Tema claro/escuro, Logs |
| **Ajuda** | Documentação, Diagnóstico de hardware, Refazer setup de hardware, Abrir pasta de logs, Sobre |

### 3.5 Configurações do ambiente

- **Geral:** idioma (pt-BR / en), tema, pasta de saída, formato de nome dos arquivos.
- **Hardware:** perfil detectado, backend em uso (com opção de trocar), limite de VRAM/RAM, **botão "Refazer setup de hardware" [Ideia original]**.
- **Modelos:** pasta de modelos, token do Hugging Face (para modelos com acesso restrito), limpeza de cache, "mostrar modelos incompatíveis".
- **Ambiente 3D:** estado do ambiente Python (instalado / não instalado / com erro), botões para reinstalar e remover.
- **Execução:** impedir suspensão do PC durante filas, ação ao terminar a fila (nada / notificar / suspender / desligar).
- **Logs:** nível de log, pasta de logs, abrir último log de erro.

---

## 4. Funcionalidades

### 4.1 MVP (primeira versão utilizável)

- Geração 2D a partir de texto.
- **Remoção de fundo** de qualquer imagem (gerada ou aberta do disco), com pré-visualização da transparência e export em PNG transparente. Útil para sprites, para edição rápida e como etapa obrigatória antes do 3D.
- Detecção de hardware e setup automático no primeiro uso.
- Gerenciador de modelos (download com progresso, verificação de integridade e aviso de incompatibilidade).
- Histórico de prompts e galeria simples.
- Modal de erro com log e arquivo de log de erros.
- Menu nativo e configurações básicas.

### 4.2 Versão 1.0

- **Geração 3D** (imagem → malha texturizada) com export GLB/OBJ e visualizador 3D.
- **Img2img** — gerar uma nova imagem a partir de uma existente (variar, mudar estilo), com controle de intensidade.
- **Inpainting** — pintar uma máscara com pincel sobre a imagem e regerar só aquela área (corrigir mãos, rosto, trocar um objeto).
- **Texturas tileáveis** — texturas que se repetem sem emendas (chão, parede, grama), com pré-visualização em mosaico 3×3 para conferir as bordas. Técnica usual: padding circular nas convoluções; confirmar suporte no backend na fase de pesquisa.
- **Fluxos de geração** em lote com fila persistente e templates (seção 5).
- Galeria completa.

### 4.3 Pós-processamento 3D para jogos (opcional)

Etapa **opcional, desligada por padrão**, que o usuário ativa nas configurações do modelo 3D ou por item de fluxo:

- Redução de polígonos (decimação) com alvo configurável (ex.: 5k, 10k, 20k triângulos).
- Tamanho de textura (512 / 1024 / 2048).
- Centralizar, normalizar escala e orientar o modelo (pivô no chão).
- Formatos de export adicionais (ex.: FBX).

O arquivo original gerado é sempre preservado; o pós-processado é salvo ao lado.

### 4.4 Após a 1.0

- **Rigging automático** de personagens (exportar já com esqueleto, pronto para animar).
- **API local / CLI** para integrar com engines (Godot, Unity, Unreal) ou scripts.

### 4.5 Backlog geral

- **[Backlog]** Presets de estilo no painel.
- **[Backlog]** LoRAs para manter um estilo consistente entre todos os assets de um jogo.
- **[Backlog]** Upscale de imagens.

---

## 5. Fluxos de geração (batch)

### 5.1 Conceito [Ideia original]

Um **fluxo** é uma lista de itens a gerar, combinados com um **template de prompt** que vale para todos os itens. A aplicação executa um item por vez: gera a(s) imagem(ns) e, se configurado, gera o modelo 3D a partir dela, antes de passar ao próximo item.

Não é exclusivo para 3D: pode ser usado só para gerar muitas imagens, inclusive várias variações para o mesmo prompt.

### 5.2 Templates reutilizáveis [Ideia original]

O usuário cria quantos templates quiser e os reutiliza em qualquer fluxo. Um template guarda:

- O texto base com o marcador `{prompt}` (ex.: `"{prompt}, corpo inteiro, T-pose, fundo branco liso, estilo low poly"`).
- O prompt negativo.
- Modelos 2D e 3D padrão e seus parâmetros.
- Se o pós-processamento 3D está ativo.

Exemplos: "Personagem para 3D", "Prop para 3D", "Ícone de item", "Textura tileável". Templates ficam em uma biblioteca (criar, duplicar, renomear, excluir) acessível pelo menu **Fluxos → Gerenciar templates**.

### 5.3 Estrutura de um fluxo

```
Fluxo: "NPCs da vila"
├── Template:   "Personagem para 3D"
│               "{prompt}, personagem de corpo inteiro, T-pose, vista frontal,
│                fundo branco liso, estilo low poly estilizado"
├── Negativo:   "múltiplas pessoas, cortado, fundo complexo"
├── Modelo 2D:  SDXL (Q8)        Modelo 3D: TripoSR
├── Padrões:    1024x1024, 25 passos, seed aleatória
└── Itens:
    1. "ferreiro careca com avental de couro"   imagens: 1   3D: sim
    2. "aldeã com cesta de pães"                imagens: 1   3D: sim
    3. "guarda com lança e armadura de ferro"   imagens: 4   3D: escolher depois
    4. "mendigo idoso com bengala"              imagens: 3   3D: não   seed: 1234
```

### 5.4 Configuração por item [Ideia original]

- **Prompt do item** — inserido no template no lugar de `{prompt}`.
- **Quantidade de imagens** — quantas variações diferentes (seeds diferentes).
- **Gerar 3D?**
  - Com 1 imagem: sim/não.
  - Com várias imagens: o 3D vem **desmarcado por padrão** e, ao marcar, a aplicação pede confirmação, já que gerar 3D para todas as variações é menos comum e custa bastante tempo.
- **Sobrescrever parâmetros** — qualquer parâmetro do template (resolução, modelo, seed fixa, passos, pós-processamento) pode ser alterado só para aquele item.

Para itens com múltiplas imagens, há três modos de 3D:

| Modo | Comportamento |
| --- | --- |
| **Não** | Apenas imagens. |
| **Todas** | Gera 3D de cada variação (com a confirmação descrita acima). |
| **Escolher depois** | Gera só as imagens durante a noite; de manhã o usuário escolhe a(s) favorita(s) e as envia para uma segunda fila só de 3D. |

### 5.5 Ordem de execução

Execução **intercalada**: imagem(ns) do item 1 → 3D do item 1 → imagem(ns) do item 2 → 3D do item 2…

Motivo: se houver problema no 3D, ele aparece logo no primeiro item, e o usuário pode agir antes de a noite inteira passar. Entre a etapa 2D e a 3D, o modelo anterior é descarregado da VRAM para que o próximo caiba.

### 5.6 Robustez da fila (essencial para execução noturna)

- **Fila persistida em disco** — se o app fechar, travar ou o PC reiniciar, a fila retoma do ponto onde parou.
- **Falha isolada** — erro em um item marca o item como falho, registra o log e segue para o próximo. Opção de tentar novamente ao final.
- **Erros não bloqueiam a fila** — durante uma fila, o modal de erro não interrompe a execução; os erros ficam acumulados e são exibidos no relatório final (ver seção 8).
- **Recuperação de falta de memória** — se faltar VRAM, tentar novamente com configurações mais leves (resolução menor, offload) antes de desistir.
- **Impedir suspensão/hibernação** do sistema enquanto a fila roda (via `systemd-inhibit`/D-Bus).
- **Relatório ao final** — quantos itens concluídos, falhos (com link para o log de cada um), tempo total; notificação do sistema.
- **Pausar/retomar/cancelar** a qualquer momento, e reordenar itens pendentes.

### 5.7 Backlog dos fluxos

- **[Backlog]** Importar itens de CSV/planilha — útil para listas grandes de NPCs.
- **[Backlog]** Variáveis/wildcards no prompt: `guarda {humano|elfo|anão} com {espada|lança}` gera combinações automaticamente.

---

## 6. Compatibilidade de hardware e setup automático

### 6.1 O problema

A maioria das ferramentas de IA local assume NVIDIA + CUDA. Em GPUs AMD de consumo, o PyTorch com ROCm muitas vezes não reconhece a placa oficialmente, e o "jeitinho" comum (forçar outra arquitetura com `HSA_OVERRIDE_GFX_VERSION`) funciona só em parte. O resultado são aplicações que quebram no meio do trabalho.

### 6.2 Caso real: InvokeAI 6.14.2 na RX 7600

Log do autor (ROCm, `HSA_OVERRIDE_GFX_VERSION=11.0.0`, modelo Anima com VAE `AutoencoderKLWan`). Trecho relevante:

```
Denoising (Anima) (#0): 100%|████████| 30/30 [01:40<00:00,  3.34s/it]
...
[MODEL CACHE] Loaded model '...:vae' (AutoencoderKLWan) onto cuda device #0
Kernel Name: miopenSp3AsmConvFury_v2_4_1_gfx11_1536vgprs_fp16_fp16acc_f2x3_c16_stride1
...
Callback: Queue 0x7f910b200000 aborting with error : HSA_STATUS_ERROR_INVALID_ISA:
The instruction set architecture is invalid. code: 0x100f
Invoke process was terminated with signal 6, exit code 0
```

**O que aconteceu:**

1. A geração em si (denoising, 30 passos) **funcionou**: PyTorch + ROCm rodam na placa para a maior parte das operações.
2. O crash ocorre no **VAE decode** (transformar o resultado latente em imagem), que é feito basicamente de **convoluções**.
3. Para as convoluções, a MIOpen (biblioteca da AMD equivalente ao cuDNN) escolheu um kernel em **assembly escrito à mão** para `gfx11` assumindo **1536 VGPRs** (registradores).
4. **Causa provável (a confirmar na fase de pesquisa):** a RX 7600 é Navi 33 (`gfx1102`), que tem um banco de registradores **menor** que a Navi 31 (`gfx1100`, RX 7900). O override faz o runtime tratar a placa como `gfx1100`; kernels genéricos funcionam, mas kernels assembly otimizados para a Navi 31 são inválidos no hardware real e o processo é abortado (`SIGABRT`).
5. Como o processo morre, **todo o trabalho do denoising (1m40s) é perdido**.

Ruídos no log que não são a causa, mas também são lições:

- `amdgpu.ids: No such file or directory` — apenas cosmético (nome da placa aparece como "AMD Radeon Graphics").
- `patchmatch failed to load or compile` — dependência que tenta compilar C em tempo de execução. Evitar esse tipo de dependência.
- `Root directory = /home/luis` — app espalhando arquivos na home. Usar sempre os diretórios XDG.

### 6.3 Requisitos derivados deste caso

1. **Vulkan como padrão para 2D.** O caminho stable-diffusion.cpp + Vulkan não usa ROCm nem MIOpen, então essa classe inteira de problema não existe nele.
2. **Evitar o override quando houver suporte nativo.** Preferir builds de PyTorch/ROCm que incluam `gfx1102` nativamente (verificar com `rocminfo` qual ISA o driver reporta e quais arquiteturas o PyTorch instalado suporta). O override vira último recurso, nunca o padrão.
3. **Se o override for necessário, desativar os kernels assembly arriscados** da MIOpen (via variáveis de ambiente de debug da MIOpen ou desativando o backend cuDNN/MIOpen no PyTorch para usar as convoluções nativas, mais lentas, porém seguras). Combinações exatas a validar na fase 1.
4. **O teste do setup tem que exercitar as operações perigosas.** Não basta multiplicar matrizes: o teste roda uma geração mínima **completa, incluindo o VAE decode em fp16**, em processo isolado. Se quebrar, a aplicação testa o próximo conjunto de configurações.
5. **Salvar os latentes antes do VAE decode.** Se o decode quebrar, um novo processo tenta decodificar de novo com configuração de fallback (VAE em tiles, fp32, VAE na CPU, convoluções sem MIOpen) **sem refazer o denoising**.
6. **Memória de compatibilidade local.** Combinações que já quebraram nesta máquina (modelo + backend + configuração) ficam registradas e são evitadas automaticamente nas próximas vezes, até o próximo "Refazer setup".
7. **Processo isolado sempre.** Um `SIGABRT` no worker nunca derruba a interface; o core captura o código de saída e o log (seção 8).

### 6.4 Estratégia: múltiplos backends com fallback

A aplicação tenta o melhor backend disponível e cai para o próximo se o teste falhar:

| Hardware | 2D (padrão → fallbacks) | 3D / Python (padrão → fallbacks) |
| --- | --- | --- |
| NVIDIA | Vulkan ou CUDA → CPU | PyTorch CUDA → CPU |
| AMD | **Vulkan** → ROCm → CPU | PyTorch ROCm nativo → ROCm com override seguro → CPU |
| Intel Arc / iGPU | Vulkan → CPU | PyTorch XPU → CPU |
| Sem GPU dedicada | CPU (modelos pequenos/quantizados) | CPU (lento) |

**Vulkan é a peça-chave da compatibilidade:** funciona em praticamente qualquer GPU dos últimos ~8 anos (AMD, NVIDIA, Intel) sem instalar CUDA/ROCm. O projeto [stable-diffusion.cpp](https://github.com/leejet/stable-diffusion.cpp) roda SD 1.5, SDXL, SD3 e Flux via Vulkan, com modelos quantizados (GGUF) que reduzem bastante o uso de VRAM.

**Escolha do usuário:** stable-diffusion.cpp + Vulkan é o **recomendado**, mas nas configurações o usuário pode trocar para outro backend (stable-diffusion.cpp com ROCm/CUDA, ou PyTorch/diffusers) e baixar e usar outros modelos. Se o backend escolhido quebrar, o modal de erro mostra o log e oferece voltar para o recomendado.

### 6.5 Setup inicial automático [Ideia original]

Executado na primeira abertura, com uma tela de boas-vindas mostrando o progresso:

1. **Detecção de hardware**
   - Distribuição Linux, kernel, versão do driver (Mesa/AMDGPU, NVIDIA).
   - CPU (núcleos, suporte a AVX2/AVX-512).
   - RAM total e disponível.
   - GPUs: fabricante, modelo, ISA real (ex.: `gfx1102`), VRAM, backends disponíveis (Vulkan, CUDA, ROCm).
   - Espaço livre em disco.
2. **Teste real de cada backend candidato** — geração mínima completa em processo isolado (ver 6.3, item 4). Detectar não basta; é aqui que as outras aplicações falham.
3. **Escolha do perfil** — ver 6.6.
4. **Sugestão de modelos iniciais** — recomenda modelos compatíveis com o perfil, mostrando tamanho de download e licença; o usuário confirma.
5. **Geração de teste** — gera uma imagem pequena para validar tudo de ponta a ponta.

O ambiente Python **não** é instalado aqui; ele é configurado quando o usuário demonstra intenção de usar 3D (ver 7.3). O usuário pode sobrescrever qualquer escolha manualmente (modo avançado).

### 6.6 Perfis de hardware

A única máquina medida é a RX 7600 (8 GB) com 32 GB de RAM. As linhas Mínimo e Intermediário saem dessa medição. Básico, Avançado e Alto continuam sem teste próprio: SDXL, Flux e as placas NVIDIA seguem em aberto. O detalhe está em `docs/pesquisa/consolidacao.md`.

| Perfil | Exemplo de hardware | 2D | 3D |
| --- | --- | --- | --- |
| **Mínimo** | Só CPU, 16 GB RAM | SD 1.5, 512 px, pelo stable-diffusion.cpp em CPU. Aqui a geração levou 32 s com o grafo em cache e 65 s na passagem anterior. O processo chegou a 4,5 GB de RAM | TripoSR em CPU. O processo ficou em 4,4 GB. O Stable Fast 3D em CPU chegou a 14 GB e não entra neste perfil |
| **Básico** | GPU 4–6 GB (GTX 1660, RX 6500) | SD 1.5. Não houve placa dessa faixa. Os pesos ocupam 2,8 GB; com pouca VRAM livre o Vulkan já tira o VAE da placa | TripoSR. Nesta placa o pico foi 3,8 GB, então 4 GB fica apertado e não foi comprovado |
| **Intermediário** | GPU 8 GB (RX 7600 medida; RTX 3060 Ti/4060 por semelhança) | SD 1.5. Vulkan em 6,2 s, com pico de 5,0 GB. ROCm em 3,4 s, com os pesos inteiros na placa | TripoSR no ROCm (pico 3,8 GB). Stable Fast 3D cabe se a atenção experimental estiver ligada: 6,2 GB alocados e a placa chegou a 8,1 GB. Sem essa flag a geração estoura |
| **Avançado** | GPU 12–16 GB | Sem medição. SDXL e Flux quantizado continuam pendentes | Hunyuan3D, TRELLIS e InstantMesh não rodaram: a textura ou a geração dependem de CUDA |
| **Alto** | GPU 24 GB+ | Sem medição | Sem medição |

A função que escolhe o perfil usa a maior VRAM, em GiB binários: sem GPU é Mínimo; até 6 GiB é Básico; até 10 GiB é Intermediário; até 18 GiB é Avançado; acima disso é Alto. A GTX 1060 de 6 GB fica em Básico, a RX 7600 medida (8.176 MiB) em Intermediário, a RTX 4070 de 12 GB e a Intel Arc de 16 GB em Avançado, e a RX 7900 de 20 GB ou mais em Alto.

**Decisão do MVP.** O backend 2D é o stable-diffusion.cpp, com Vulkan como caminho padrão, ROCm quando a placa AMD tem a ISA nativa e o override está desligado, e CPU por último. O modelo 2D é o SD 1.5. O backend 3D é PyTorch, e o modelo é o TripoSR. O Stable Fast 3D fica como opção texturizada do perfil intermediário. A remoção de fundo é o U²-Net em CPU, pela crate `ort`. A decimação é o `meshoptimizer`.

### 6.7 Refazer setup [Ideia original]

Botão em **Configurações → Hardware → Refazer setup de hardware** (e também em **Ajuda**). Refaz detecção, testes e perfil, limpa a memória de compatibilidade local, preserva os modelos já baixados e mostra o que mudou ("GPU nova detectada: RTX 4070. Novo perfil: Avançado. Deseja baixar modelos recomendados?").

A aplicação também verifica rapidamente o hardware a cada inicialização e **sugere** refazer o setup se detectar mudança (nova GPU, driver ou ROCm atualizado).

### 6.8 Diagnóstico

Tela "Diagnóstico de hardware" que gera um relatório copiável (hardware, ISA, driver, backend, versões, combinações que falharam, últimos erros). Facilita abrir uma issue no GitHub quando algo não funcionar na máquina de alguém.

### 6.9 Aviso de conector de energia

Há registros públicos e repetidos de derretimento do conector 12VHPWR / 12V-2x6 nestas placas: GeForce RTX 3090 Ti, 4080, 4090, 5070 Ti, 5080 e 5090, RTX PRO 6000, e Radeon RX 9070 XT nas versões que usam esse conector. A RX 7600 fica de fora: o teto de firmware dela é 145 W.

Na abertura, se a GPU detectada estiver nessa lista e o teto de potência vigente passar de 300 W, o app mostra uma notificação recomendando um power limit de 300 W. Não aplica o limite sozinho. Se o teto já estiver em 300 W ou menos, a notificação não aparece.

---

## 7. Arquitetura técnica

### 7.1 Visão geral

A aplicação será híbrida em três camadas, cada uma fazendo aquilo em que sua linguagem é melhor:

```
┌──────────────────────────────────────────────┐
│  Frontend (TypeScript + React)               │  Interface, editor, viewer 3D
└───────────────┬──────────────────────────────┘
                │ Tauri IPC (commands + events)
┌───────────────▼──────────────────────────────┐
│  Core (Rust / Tauri)                         │  Orquestração, fila, banco,
│  - detecção de hardware                      │  downloads, logs, gerência
│  - fila de jobs e persistência               │  dos processos de inferência
│  - gerenciador de modelos e compatibilidade  │
│  - remoção de fundo (ONNX Runtime)           │
│  - supervisor de workers                     │
└───────┬─────────────────────────┬────────────┘
        │ processo filho           │ processo filho
┌───────▼─────────────┐   ┌───────▼─────────────────────┐
│ Worker nativo       │   │ Worker Python               │
│ stable-diffusion.cpp│   │ (PyTorch / modelos 3D /     │
│ (Vulkan/ROCm/CUDA)  │   │  diffusers opcional)        │
└─────────────────────┘   └─────────────────────────────┘
```

### 7.2 Por que essa divisão

- **Rust (core):** é o processo principal do Tauri. Fica com tudo que precisa ser confiável: fila, banco de dados, downloads, logs, detecção de hardware e supervisão dos workers. Se um worker travar ou abortar, o core continua vivo, registra o erro e reinicia o worker.
- **Remoção de fundo no core via ONNX Runtime** (crate `ort`): os modelos de remoção de fundo existem em formato ONNX e rodam bem em CPU, então essa função (que está no MVP) funciona para todo mundo **sem precisar do Python**.
- **Worker nativo (stable-diffusion.cpp):** geração 2D, img2img e inpainting, com a melhor compatibilidade (Vulkan) e sem Python.
- **Worker Python:** necessário porque quase todos os modelos de imagem → 3D só existem em Python/PyTorch. Também pode servir como backend 2D alternativo (diffusers) para quem escolher.
- **Processos separados** em vez de embutir Python no Rust: isolamento de falhas (como o `SIGABRT` do caso 6.2), liberação total de VRAM ao encerrar o processo e possibilidade de trocar workers independentemente.

### 7.3 Como lidar com o Python (a parte mais delicada)

O usuário **não deve precisar instalar Python** nem mexer em ambientes virtuais.

- O app usa o **[uv](https://github.com/astral-sh/uv)** (gerenciador de pacotes Python em um único executável) para baixar uma **distribuição Python isolada** e criar um ambiente virtual dentro da pasta de dados do app (nunca na home do usuário nem no Python do sistema).
- Instala o **PyTorch na variante certa para o hardware** detectado (CUDA, ROCm ou CPU), com dependências travadas em um `uv.lock` versionado no repositório.
- Variáveis de ambiente específicas são aplicadas pelo core ao iniciar o worker, conforme o resultado dos testes (ver 6.3).
- Só usar dependências com wheels prontos; nada que compile C/C++ na máquina do usuário em tempo de execução.

**Configuração automática por intenção [Ideia original]:** o ambiente Python só é instalado quando necessário, mas sem o usuário precisar pedir. A instalação começa sozinha, em segundo plano, assim que o usuário demonstra intenção de usar 3D:

- marca a opção **3D** no painel esquerdo;
- marca **Gerar 3D** em um item de fluxo ou usa um template com 3D;
- baixa um modelo 3D no gerenciador de modelos;
- usa "enviar para 3D" em uma imagem.

Durante a instalação, a barra de status mostra o progresso ("Preparando ambiente 3D… 45%") e o tamanho do download. Uma geração 3D pedida nesse meio tempo espera o ambiente ficar pronto, em vez de falhar. Ao final, o ambiente passa pelo mesmo teste real descrito em 6.3.

### 7.4 Comunicação entre core e workers

**Decisão (fase 0):** HTTP para comandos e WebSocket para progresso. JSON-RPC por stdin/stdout fica descartado: é mais simples, mas pior para prévias e para depurar o worker com as ferramentas HTTP comuns.

- Cada worker é um servidor local escutando apenas em `127.0.0.1`, em **porta aleatória** e com **token** gerado pelo core a cada execução (impede que outros programas usem o worker).
- **HTTP** para comandos (carregar modelo, gerar, cancelar, status) e **WebSocket** para progresso em tempo real (passo atual, prévia da imagem).
- Imagens, latentes e malhas são gravados em disco pelo worker; pelo protocolo trafegam apenas caminhos e metadados.
- O contrato da API é definido em um esquema único (OpenAPI/JSON Schema), do qual geramos os tipos em Rust, TypeScript e Python, evitando divergências.

**Alternativa considerada:** JSON-RPC via stdin/stdout. É mais simples (sem portas), mas pior para streaming de prévias e para depurar manualmente.

### 7.5 Interface comum de backends

Todos os backends implementam a mesma interface lógica, o que permite trocar um pelo outro e testar com um **backend falso (mock)**:

```
ImageBackend:   load_model, txt2img, img2img, inpaint, decode_latents, cancel, unload
MeshBackend:    load_model, image_to_mesh(imagem, params) -> malha, cancel, unload
BackgroundRemover: remove_background(imagem) -> imagem com alfa
MeshPostProcess:   decimate, resize_textures, normalize -> malha
```

### 7.6 Stack proposta

| Camada | Tecnologia | Motivo |
| --- | --- | --- |
| Desktop | **Tauri 2** | Leve, menu nativo, sidecars |
| Frontend | **React + TypeScript + Vite** | Maior ecossistema; `react-three-fiber` para o viewer 3D |
| UI | Tailwind CSS + componentes acessíveis (shadcn/ui ou Radix) | Interface limpa e consistente rapidamente |
| Estado | Zustand + TanStack Query | Simples, fácil de testar |
| i18n | i18next (`react-i18next`) | pt-BR e en |
| Core | **Rust** (tokio, serde, rusqlite/sqlx, reqwest, tracing) | Padrão do Tauri; `tracing` para logs estruturados |
| Hardware | Vulkan (via `ash`), `sysinfo`, NVML, `rocminfo` | Detecção de GPU, VRAM e ISA real |
| Remoção de fundo | ONNX Runtime (crate `ort`) | Roda em CPU, sem Python |
| Banco | **SQLite** | Local, sem servidor, confiável |
| 2D nativo | **stable-diffusion.cpp** (sidecar) | Compatibilidade via Vulkan, GGUF, img2img/inpainting |
| Python | **uv** + Python 3.12, PyTorch, FastAPI | Modelos 3D e backend 2D alternativo |
| Viewer 3D | three.js (`react-three-fiber`) | GLB/OBJ na interface |

### 7.7 Estrutura de pastas proposta

```
voxel/
├── apps/
│   └── desktop/
│       ├── src/                 # frontend React
│       └── src-tauri/           # core Rust (Tauri)
├── crates/                      # crates Rust reutilizáveis
│   ├── hardware/                # detecção, perfis e memória de compatibilidade
│   ├── queue/                   # fila de jobs
│   ├── models/                  # catálogo, downloads e checagem de compatibilidade
│   ├── imaging/                 # remoção de fundo e utilidades de imagem
│   ├── logging/                 # logs e relatórios de erro
│   └── protocol/                # tipos do protocolo com os workers
├── workers/
│   ├── python/                  # worker Python (pyproject + uv.lock)
│   ├── mock/                    # worker falso para testes de contrato
│   └── sdcpp/                   # integração/build do stable-diffusion.cpp
├── protocol/                    # esquema OpenAPI/JSON Schema compartilhado
├── catalog/                     # catálogo de modelos (JSON versionado)
├── docs/
└── .github/workflows/
```

---

## 8. Tratamento de erros e logs

### 8.1 Arquivos de log

- Logs ficam em `~/.local/state/voxel/logs/` (diretório XDG de estado).
- Um log geral do app (rotativo, mantém os últimos N dias).
- **Um arquivo por erro** (`erro_2026-10-18_14-32-05_<job>.log`) contendo: mensagem de erro, código de saída/sinal do processo, as últimas linhas de saída (stdout/stderr) do worker, parâmetros do job, backend, modelo e resumo do hardware. É exatamente o que seria necessário para diagnosticar um caso como o 6.2.
- O core captura stdout/stderr dos workers continuamente, então mesmo um `SIGABRT` sem mensagem estruturada deixa o log completo.

### 8.2 Modal de erro

Quando uma geração avulsa quebra, abre um modal com:

- Resumo em linguagem simples ("O processo de geração foi encerrado pelo driver da GPU durante a decodificação da imagem").
- Log completo em área rolável, com botão **Copiar**.
- Botões: **Abrir arquivo de log**, **Tentar novamente com configuração segura** (aplica o fallback correspondente, ex.: VAE na CPU), **Voltar ao backend recomendado** (se o usuário trocou), **Abrir diagnóstico**.

Durante a execução de um fluxo, o modal **não** é aberto a cada erro, para não travar a fila; os erros vão para o relatório final, cada um com acesso ao seu log.

### 8.3 Classificação de erros

O core tenta reconhecer padrões conhecidos na saída do worker para dar uma mensagem útil e escolher o fallback automaticamente:

| Padrão | Interpretação | Ação |
| --- | --- | --- |
| `HSA_STATUS_ERROR_INVALID_ISA`, kernel `miopen...Asm...` | Kernel incompatível com a GPU real | Repetir sem kernels assembly / sem MIOpen; registrar na memória de compatibilidade |
| `out of memory`, `hipErrorOutOfMemory`, `CUDA out of memory` | Falta de VRAM | Repetir com configurações mais leves (tiles, offload, resolução menor) |
| Processo encerrado por sinal (6, 11) | Crash nativo | Reiniciar worker, aplicar fallback, registrar |
| Falha de rede/download | Problema de conexão | Retomar download depois |

Essa tabela é dado testável e cresce com novos casos reais.

---

## 9. Modelos de IA candidatos

> Lista inicial para a fase de pesquisa. Requisitos de VRAM, compatibilidade e licenças precisam ser confirmados um a um antes de entrar no catálogo.

### 9.1 Imagem (texto → imagem, img2img, inpainting)

| Modelo | Observação |
| --- | --- |
| Stable Diffusion 1.5 (e derivados) | Muito leve, roda até em CPU; tem variantes próprias de inpainting |
| SDXL (e derivados) | Bom equilíbrio; versões quantizadas cabem em 8 GB; tem variantes de inpainting |
| SD 3.5 Medium | Boa aderência ao prompt; verificar licença |
| FLUX.1 schnell | Licença Apache 2.0, poucos passos; pesado sem quantização |
| FLUX.1 dev | Alta qualidade; licença não comercial |

### 9.2 Imagem → 3D

| Modelo | Observação |
| --- | --- |
| TripoSR | Leve e rápido, licença MIT; qualidade de textura limitada |
| Stable Fast 3D (SF3D) | Rápido, gera UV e textura; licença comunitária da Stability |
| Hunyuan3D 2.x (incl. versões mini) | Ótima qualidade; pesado; partes dependem de kernels CUDA; licença com restrições territoriais |
| TRELLIS | Ótima qualidade; depende fortemente de CUDA |
| InstantMesh | Multi-view → malha; mais etapas |

**Ponto de atenção:** os modelos 3D de melhor qualidade costumam depender de extensões CUDA específicas. Estratégia: começar por um modelo **leve e portátil** (ex.: TripoSR ou SF3D) que funcione em todos os perfis e oferecer os pesados apenas onde comprovadamente funcionam. O catálogo registra isso, e o gerenciador de modelos avisa antes do download (ver 9.4).

### 9.3 Auxiliares

- **Remoção de fundo (ONNX):** BiRefNet, U²-Net/IS-Net e similares (verificar licenças; algumas variantes, como RMBG, são não comerciais).
- **Upscale [Backlog]:** Real-ESRGAN.
- **Pós-processamento de malha:** decimação e limpeza (ex.: `meshoptimizer`, `trimesh`, `pymeshlab`).

### 9.4 Catálogo de modelos e checagem de compatibilidade

Um arquivo de catálogo versionado (JSON) descreve cada modelo: origem (URL do Hugging Face), hash, tamanho, tipo (2D, inpainting, 3D, remoção de fundo), **backends suportados**, **VRAM/RAM mínima por backend**, dependências especiais (ex.: "requer CUDA") e licença.

No gerenciador de modelos, cada modelo recebe um selo calculado a partir do perfil de hardware atual:

| Selo | Significado |
| --- | --- |
| **Compatível** | Roda bem no hardware atual |
| **Compatível com ressalvas** | Roda, mas com limitações (ex.: "lento, usará offload para RAM") |
| **Incompatível** | Não roda aqui, com o motivo (ex.: "requer CUDA — sua GPU é AMD", "requer 16 GB de VRAM — você tem 8 GB") |

- Modelos incompatíveis ficam ocultos por padrão (opção "mostrar modelos incompatíveis").
- Ao tentar baixar um modelo incompatível, a aplicação avisa e pede confirmação explícita, evitando que o usuário perca tempo e espaço.
- Modelos importados de fora do catálogo (`.safetensors`/`.gguf`) têm a arquitetura identificada pelos metadados do arquivo e a compatibilidade estimada pelo tamanho vs. VRAM disponível.
- A licença do modelo é mostrada antes do download. O Voxel é não comercial, mas quem usar o Voxel para criar assets de um jogo comercial precisa saber se o modelo permite isso.

---

## 10. Dados e armazenamento

Seguindo os diretórios XDG (nada espalhado na home):

| Conteúdo | Local padrão |
| --- | --- |
| Configurações | `~/.config/voxel/` |
| Banco SQLite, ambiente Python, catálogo em cache | `~/.local/share/voxel/` |
| Logs e memória de compatibilidade | `~/.local/state/voxel/` |
| Cache temporário (latentes, prévias) | `~/.cache/voxel/` |
| Modelos | configurável (pode ficar em outro disco — modelos ocupam dezenas de GB) |
| Saídas | configurável (padrão `~/Imagens/Voxel/`) |

Organização das saídas por data e/ou fluxo:

```
Voxel/
└── 2026-10-18_npcs-da-vila/
    ├── 001_ferreiro/
    │   ├── imagem_01.png
    │   ├── imagem_01_sem_fundo.png
    │   ├── modelo_01.glb
    │   ├── modelo_01_otimizado.glb
    │   └── metadados.json
    └── ...
```

- **Metadados** (prompt, seed, modelo, parâmetros, backend, tempo de geração) salvos no banco, em `metadados.json` e embutidos no PNG, permitindo reabrir/regerar.
- **Downloads** retomáveis, com verificação de hash.

---

## 11. Estratégia de testes

**Regra [Ideia original]:** todo código nosso tem testes unitários. Bibliotecas de terceiros não são testadas, mas nossas camadas de integração com elas são testadas com mocks/fakes.

### 11.1 Por camada

| Camada | Ferramentas | O que testar |
| --- | --- | --- |
| Frontend | **Vitest** + Testing Library | Componentes, stores, montagem de prompts, editor de máscara, formulários de fluxo e templates, modal de erro |
| Core Rust | `cargo test` (+ `cargo-nextest`, `mockall`, `insta` para snapshots) | Fila (ordem intercalada, retomada, falhas), templates e sobrescrita de parâmetros, perfis de hardware, checagem de compatibilidade de modelos, classificação de erros, downloads, supervisor de workers, gatilhos do ambiente 3D |
| Worker Python | **pytest** (+ `pytest-mock`) | Rotas da API, adaptadores de modelo (com modelo falso), pós-processamento de malha |
| Protocolo | Testes de contrato | Garantir que core e workers falam a mesma versão do protocolo |
| Ponta a ponta | WebdriverIO + `tauri-driver` | Fluxos principais da interface com backend falso |

### 11.2 Princípios de testabilidade

- **Backend falso (mock worker):** implementa o mesmo protocolo e devolve imagens/malhas de exemplo instantaneamente. Também consegue **simular falhas** (abortar com sinal 6, imprimir `HSA_STATUS_ERROR_INVALID_ISA`, falta de memória) para testar o modal, os logs, os fallbacks e a fila sem GPU.
- **Logs reais como fixtures:** o log do caso 6.2 vira fixture de teste da classificação de erros; cada novo caso real reportado também.
- **Detecção de hardware desacoplada:** a lógica "dado este hardware, qual perfil e quais modelos são compatíveis?" é uma função pura, testada com muitos cenários (RX 7600, RX 7900, GTX 1060, RTX 4070, Intel Arc, só CPU). A leitura real do hardware fica em uma camada fina separada.
- **Sistema de arquivos e relógio injetáveis** onde fizer sentido (retomada de fila, rotação de logs).
- **Testes de integração com GPU real** existem, mas ficam opcionais/manuais (marcados), já que o CI não tem GPU.
- **Cobertura mínima** definida por camada (ex.: 80%) e verificada no CI.
- **TDD sempre que possível**, especialmente na fila, nos templates, na classificação de erros e nos perfis de hardware.

---

## 12. Qualidade, CI e distribuição

### 12.1 Qualidade de código

- Rust: `rustfmt`, `clippy` (avisos tratados como erro).
- TypeScript: Biome (lint + formatação), `tsc` estrito.
- Python: `ruff` (lint + formatação), `mypy` estrito.
- Hooks do git em `.githooks/` (ativados com `make hooks`): `pre-commit` roda formatação e lint; `commit-msg` valida o padrão.
- Commits no padrão Conventional Commits (regras no README).

### 12.2 CI (GitHub Actions)

- Somente Linux.
- Etapas: lint → testes unitários (as três linguagens) → testes de contrato → E2E com backend falso → build.
- Relatório de cobertura.

### 12.3 Distribuição

- Código no GitHub, licença MIT. Sem instaladores oficiais, sem atualização automática.
- **Compilar deve ser fácil:** um único comando (`make build`) compila o frontend, o core e o stable-diffusion.cpp com Vulkan.
- README com as dependências de sistema para as principais famílias de distribuição (Arch/CachyOS, Debian/Ubuntu, Fedora) e como verificar se o Vulkan está funcionando (`vulkaninfo`).
- Modelos e ambiente Python são baixados sob demanda pelo app, nunca versionados no repositório.
- **[Sugestão]** Futuramente, se houver interesse, publicar um `PKGBUILD` (AUR) ou AppImage gerado pelo CI nas releases do GitHub — sem compromisso.

---

## 13. Roadmap

### Fase 0 — Fundação

- Estrutura do monorepo, CI, lint, testes rodando nas três linguagens.
- App Tauri vazio com layout (painel esquerdo, área principal, menu nativo, barra de status).
- Estrutura de i18n (pt-BR e en) desde o primeiro componente.
- Sistema de logs e modal de erro (com worker falso simulando falhas).
- Definição do protocolo core ↔ worker e worker falso.

### Fase 1 — Pesquisa de compatibilidade (em paralelo com a fase 0)

- Testar stable-diffusion.cpp via Vulkan e ROCm na RX 7600, e em CPU.
- Reproduzir o crash do caso 6.2 com PyTorch/ROCm puro e validar as mitigações (build nativo `gfx1102`, override sem kernels assembly, VAE em tiles/fp32/CPU).
- Testar modelos 3D candidatos na RX 7600 (ROCm/CPU) e, se possível, em NVIDIA.
- Testar remoção de fundo via ONNX Runtime em CPU.
- Verificar suporte a img2img, inpainting e padding circular (texturas tileáveis) no stable-diffusion.cpp.
- Medir VRAM e tempos para preencher a tabela de perfis e o catálogo.

### Fase 2 — MVP

- Worker stable-diffusion.cpp integrado (txt2img).
- Detecção de hardware + setup inicial com teste real + perfis.
- Gerenciador de modelos com catálogo e selos de compatibilidade.
- Remoção de fundo e export PNG transparente.
- Geração 2D com progresso, histórico de prompts, galeria simples.

### Fase 3 — 1.0, parte 1: edição e 3D

- Img2img e inpainting (editor de máscara com pincel).
- Texturas tileáveis com pré-visualização em mosaico.
- Configuração automática do ambiente Python por intenção.
- Worker Python com o primeiro modelo imagem → 3D.
- Visualizador 3D, export GLB/OBJ e pós-processamento 3D opcional.

### Fase 4 — 1.0, parte 2: fluxos

- Biblioteca de templates e editor de fluxos (com sobrescrita por item).
- Fila persistente intercalada, retomada, isolamento de falhas, prevenção de suspensão.
- Modo "Escolher depois" e relatório final com logs.

### Fase 5 — Polimento da 1.0

- Diagnóstico, refazer setup, aviso de mudança de hardware, notificações.
- Revisão das traduções pt-BR/en.
- README e documentação de compilação e uso.

### Após a 1.0

- Rigging automático de personagens.
- API local / CLI.
- Itens do backlog: importação CSV, wildcards, presets de estilo, LoRAs, upscale.

---

## 14. Riscos conhecidos

| Risco | Impacto | Mitigação |
| --- | --- | --- |
| Kernels ROCm/MIOpen inválidos em GPUs AMD de consumo (caso 6.2) | Crash no meio da geração | Vulkan como padrão; teste real com VAE decode no setup; latentes salvos; fallbacks; memória de compatibilidade |
| Modelos 3D de qualidade dependem de CUDA | 3D bom só em NVIDIA | Começar com modelos portáteis; selos de compatibilidade; acompanhar novos modelos |
| Tamanho dos downloads (PyTorch + modelos = vários GB) | Setup demorado | Ambiente Python só por intenção, downloads retomáveis, tamanhos visíveis |
| Falta de VRAM ao alternar modelos 2D e 3D | Falhas na fila | Descarregar o modelo anterior entre etapas, retry com configurações leves |
| Compilação difícil para outras pessoas | Pouca adoção | Comando único de build, README por distribuição, CI garantindo que compila |
| Ecossistema de modelos muda rápido | Catálogo desatualizado | Catálogo como dado versionado, atualizável sem nova versão do app |
| Testar sem GPU no CI | Bugs só em hardware real | Worker falso que simula falhas + checklist de testes manuais na RX 7600 |

---

## 15. Questões em aberto

Nenhuma no momento. O protocolo com os workers foi fechado na seção 7.4.
