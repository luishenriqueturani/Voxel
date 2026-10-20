# Voxel — TODO

Tarefas organizadas pelas fases do [planejamento](PLANEJAMENTO.md). As referências entre parênteses (ex.: §6.3) apontam para as seções do planejamento.

**Regras para todas as tarefas:**

- Uma tarefa só está concluída quando tem **testes unitários** cobrindo o código que desenvolvemos (§11) e o CI está verde.
- Todo texto da interface passa pelo i18n (pt-BR e en).
- Erros sempre geram log (§8); nada falha em silêncio.

Legenda: `[ ]` pendente · `[x]` concluída · `[~]` em andamento

---

## Fase 0 — Fundação

### Repositório e ferramentas

- [x] Criar estrutura do monorepo (`apps/desktop`, `crates/`, `workers/python`, `workers/sdcpp`, `protocol/`, `catalog/`) (§7.7)
- [x] Inicializar app Tauri 2 com React + TypeScript + Vite em `apps/desktop` (gerenciador de pacotes: bun)
- [x] Configurar workspace Cargo com os crates `hardware`, `queue`, `models`, `imaging`, `logging`, `protocol`
- [x] Inicializar projeto Python em `workers/python` com `uv` (`pyproject.toml` + `uv.lock`, Python 3.12)
- [x] Criar comando único de build e tarefas comuns (Makefile): `install`, `dev`, `build`, `test`, `lint`, `fmt`, `fmt-check`, `clean` (§12.3)
- [x] Atualizar `.gitignore` (target, node_modules, .venv, builds do sd.cpp, modelos)

### Qualidade de código

- [x] Rust: `rustfmt` + `clippy` com avisos como erro
- [x] TypeScript: Biome (lint + formatação) e `tsc` estrito
- [x] Python: `ruff` (lint + formatação) e `mypy` estrito
- [x] Hooks de pré-commit rodando formatação e lint
- [x] Definir padrão Conventional Commits (documentar no README e validar no hook `commit-msg`)

### Testes e CI

- [x] Configurar Vitest + Testing Library no frontend
- [x] Configurar `cargo test` / `cargo-nextest`, `mockall` e `insta` no Rust
- [x] Configurar pytest + `pytest-mock` no worker Python
- [x] Configurar relatório de cobertura nas três linguagens com mínimo de 80% por camada (`make coverage`)
- [x] Configurar E2E com WebdriverIO + `tauri-driver` (`make e2e`)
- [x] Workflow do GitHub Actions (Linux): formatação → lint → testes unitários com cobertura → E2E → build (§12.2)
- [x] Adicionar a etapa de testes de contrato ao CI quando o protocolo existir

### Interface base

- [x] Configurar Tailwind CSS e componentes com Radix (popover)
- [x] Configurar i18next com pt-BR e en, e troca de idioma em tempo de execução
- [x] Layout principal: painel esquerdo, área principal e barra de status (§3.1)
- [x] Painel esquerdo estático: prompt, prompt negativo recolhível, seleção de modelo, botão ⚙, opção 2D/3D, botão Gerar, lista de recentes (§3.2)
- [x] Menu nativo do Tauri com todos os menus e itens (ainda sem ação onde não houver funcionalidade) (§3.4)
- [x] Atalho `Ctrl+Enter` para gerar
- [x] Tema claro/escuro
- [x] Tela de configurações com as seções vazias (Geral, Hardware, Modelos, Ambiente 3D, Execução, Logs) (§3.5)

### Core e persistência

- [x] Resolver diretórios XDG (`~/.config/voxel`, `~/.local/share/voxel`, `~/.local/state/voxel`, `~/.cache/voxel`) (§10)
- [x] Banco SQLite com sistema de migrações
- [x] Leitura/gravação de configurações do usuário

### Logs e erros (§8)

- [x] Log geral do app com `tracing`, rotativo por dias
- [x] Arquivo de log por erro com mensagem, código de saída/sinal, últimas linhas de stdout/stderr, parâmetros do job, backend, modelo e resumo do hardware (§8.1)
- [x] Captura contínua de stdout/stderr dos processos filhos
- [x] Classificador de erros baseado em padrões, como tabela de dados (§8.3)
  - [x] Usar o log do InvokeAI (§6.2) como fixture de teste
- [x] Modal de erro: resumo em linguagem simples, log rolável, Copiar, Abrir arquivo de log, Abrir diagnóstico (§8.2)

### Protocolo e workers

- [x] Decidir protocolo core ↔ worker (HTTP/WebSocket local vs. stdio JSON-RPC) e registrar no planejamento (§7.4)
- [x] Escrever esquema do protocolo (OpenAPI/JSON Schema) e gerar tipos em Rust, TypeScript e Python
- [x] Interfaces de backend: `ImageBackend`, `MeshBackend`, `BackgroundRemover`, `MeshPostProcess` (§7.5)
- [x] Supervisor de workers no core: iniciar, autenticar com token, monitorar, detectar saída/sinal, reiniciar
- [x] Worker falso (mock) que devolve imagens/malhas de exemplo e progresso simulado (§11.2)
- [x] Worker falso capaz de simular falhas: abortar com sinal 6, imprimir `HSA_STATUS_ERROR_INVALID_ISA`, falta de memória
- [x] Testes de contrato entre core e workers

---

## Fase 1 — Pesquisa de compatibilidade

> Pode rodar em paralelo com a fase 0. O resultado de cada item é registrado em `docs/pesquisa/` e alimenta o catálogo e os perfis.

### 2D

- [ ] Compilar stable-diffusion.cpp com Vulkan e testar na RX 7600 (SD 1.5, SDXL, Flux quantizado)
- [ ] Compilar stable-diffusion.cpp com ROCm e comparar com Vulkan na RX 7600
- [x] Testar stable-diffusion.cpp em CPU
- [x] Verificar suporte a img2img, inpainting e padding circular (texturas tileáveis) no stable-diffusion.cpp
- [x] Verificar se é possível salvar latentes e decodificar separadamente

### Caso RX 7600 / ROCm (§6.2, §6.3)

- [x] Reproduzir o crash do VAE decode com PyTorch + ROCm puro e `HSA_OVERRIDE_GFX_VERSION=11.0.0`
- [x] Verificar se há build de PyTorch/ROCm com suporte nativo a `gfx1102` (sem override)
- [x] Validar mitigações: desligar kernels assembly da MIOpen, desligar MIOpen no PyTorch, VAE em tiles, VAE em fp32, VAE na CPU
- [x] Documentar a combinação de variáveis de ambiente que funciona e sua perda de desempenho

### 3D

- [x] Testar TripoSR na RX 7600 (ROCm e CPU)
- [x] Testar Stable Fast 3D na RX 7600 (ROCm e CPU)
- [x] Avaliar Hunyuan3D 2.x mini, TRELLIS e InstantMesh quanto à dependência de CUDA
- [ ] Testar em NVIDIA, se houver acesso

### Auxiliares

- [x] Testar remoção de fundo via ONNX Runtime (crate `ort`) em CPU com BiRefNet e U²-Net/IS-Net
- [x] Avaliar bibliotecas de decimação de malha (`meshoptimizer`, `trimesh`, `pymeshlab`)

### Consolidação

- [x] Medir VRAM, RAM e tempo de cada combinação modelo + backend
- [x] Preencher a tabela de perfis de hardware (§6.6)
- [x] Verificar licenças dos modelos candidatos
- [x] Definir os modelos e backends do MVP

---

## Fase 2 — MVP

### Detecção de hardware (§6.5)

- [x] Coletar distribuição, kernel e versão do driver (Mesa/AMDGPU, NVIDIA)
- [x] Coletar CPU (núcleos, AVX2/AVX-512), RAM total/disponível e espaço em disco
- [x] Enumerar GPUs via Vulkan: fabricante, modelo, VRAM
- [x] Detectar ISA real da GPU AMD (`rocminfo`) e presença de ROCm/CUDA (NVML)
- [x] Função pura "hardware → perfil" (§6.6), testada com vários cenários (RX 7600, RX 7900, GTX 1060, RTX 4070, Intel Arc, só CPU)
- [x] Na abertura, avisar power limit de 300 W se a GPU estiver na lista do conector 12V-2x6 (§6.9): RTX 3090 Ti, 4080, 4090, 5070 Ti, 5080, 5090, RTX PRO 6000 e RX 9070 XT. Não avisar quando o teto vigente já for de 300 W ou menos

### Setup inicial

- [x] Tela de boas-vindas com progresso do setup
- [x] Teste real de cada backend candidato: geração mínima completa, com VAE decode em fp16, em processo isolado (§6.3)
- [x] Ordem de fallback entre backends (§6.4)
- [x] Memória de compatibilidade local: registrar e evitar combinações que quebraram (§6.3)
- [x] Sugestão de modelos iniciais com tamanho e licença
- [x] Geração de teste ponta a ponta ao final do setup
- [x] Modo avançado para sobrescrever escolhas

### Worker stable-diffusion.cpp

- [x] Build do stable-diffusion.cpp com Vulkan integrado ao comando de build
- [x] Adaptador do worker implementando `ImageBackend` (txt2img)
- [x] Progresso em tempo real e prévia da imagem
- [x] Cancelamento de geração
- [ ] Salvar latentes antes do decode e refazer só o decode com fallback em caso de crash (§6.3)
- [x] Opção nas configurações para trocar de backend, com "voltar ao recomendado" no modal de erro (§6.4)

### Gerenciador de modelos (§9.4)

- [x] Formato do catálogo de modelos (JSON versionado) e primeiros modelos cadastrados
- [x] Download retomável com progresso e verificação de hash
- [x] Token do Hugging Face para modelos com acesso restrito
- [x] Cálculo do selo de compatibilidade (Compatível / Com ressalvas / Incompatível) com motivo
- [x] Ocultar incompatíveis por padrão, com opção "mostrar modelos incompatíveis"
- [x] Aviso e confirmação explícita ao baixar modelo incompatível
- [x] Exibir licença antes do download
- [x] Importar modelo local (`.safetensors`/`.gguf`) com identificação da arquitetura pelos metadados e estimativa de compatibilidade
- [x] Remover modelos e abrir pasta de modelos

### Geração 2D

- [x] Ligar o painel esquerdo ao worker: prompt, prompt negativo, modelo, parâmetros do ⚙
- [x] Parâmetros padrão vindos do perfil de hardware
- [x] Visualizador 2D: zoom, salvar, copiar, regerar com mesma seed
- [x] Salvar saídas na estrutura de pastas (§10) com `metadados.json` e metadados embutidos no PNG
- [x] Barra de status com GPU, backend, VRAM, CPU, RAM e temperaturas de CPU e GPU

### Remoção de fundo

- [x] `BackgroundRemover` no core via ONNX Runtime em CPU
- [x] Remover fundo de imagem gerada ou aberta do disco (Arquivo → Abrir imagem para edição)
- [x] Pré-visualização da transparência e export PNG transparente
- [x] Ação "Remover fundo" no visualizador e no menu Editar

### Histórico e galeria

- [x] Lista de prompts recentes no painel, com clique para recarregar prompt e parâmetros
- [x] Busca e fixação (★) de favoritos nos recentes
- [x] Galeria simples com as gerações

---

## Fase 3 — 1.0, parte 1: edição e 3D

### Edição de imagem (§4.2)

- [x] Img2img com controle de intensidade
- [x] Editor de máscara com pincel (tamanho, borracha, desfazer)
- [x] Inpainting usando a máscara
- [x] Texturas tileáveis (padding circular)
- [x] Pré-visualização de textura em mosaico 3×3

### Ambiente Python (§7.3)

- [x] Instalação via `uv` de Python isolado e venv na pasta de dados do app
- [x] Instalação do PyTorch na variante certa (CUDA, ROCm ou CPU) conforme o hardware
- [x] Aplicação das variáveis de ambiente validadas na fase 1 ao iniciar o worker
- [x] Gatilhos de intenção 3D: opção 3D no painel, "Gerar 3D" em item de fluxo, template com 3D, download de modelo 3D, "enviar para 3D"
- [x] Instalação em segundo plano com progresso e tamanho na barra de status
- [x] Gerações 3D pedidas durante a instalação aguardam o ambiente em vez de falhar
- [x] Teste real do ambiente ao final da instalação
- [x] Seção "Ambiente 3D" nas configurações: estado, reinstalar, remover

### Worker Python e 3D

- [x] Servidor do worker Python (FastAPI) seguindo o protocolo
- [x] Adaptador do primeiro modelo imagem → 3D implementando `MeshBackend`
- [x] Remoção de fundo automática antes do 3D
- [x] Descarregar o modelo 2D da VRAM antes da etapa 3D
- [x] Modo 3D do painel: texto → imagem → 3D
- [x] 3D a partir de imagem existente (arrastar para o painel e "enviar para 3D")
- [x] Export GLB e OBJ

### Visualizador 3D

- [x] Viewer com `react-three-fiber`: orbitar, zoom, wireframe, textura
- [x] Exibir contagem de polígonos

### Busca no Hugging Face

Os modelos medidos do catálogo ficam no topo, como recomendados. O restante é para experimentar: o selo é "não medido".

- [x] Seção "Recomendados" no gerenciador com os modelos do catálogo medido
- [x] Busca na API do Hugging Face por nome, mostrando nome, tamanho e licença de cada arquivo
- [x] Selo "não medido" nesses resultados, fora do filtro de incompatíveis
- [x] Escolher o arquivo `.safetensors` ou `.gguf` quando o repositório tiver mais de um
- [x] Download do resultado com retomada, hash quando o Hub informar, e o token já gravado se o repositório for restrito
- [x] Licença e aviso de que o modelo não foi medido neste computador antes de confirmar o download
- [x] Depois de baixado, ler a arquitetura do cabeçalho e estimar a compatibilidade pelo tamanho, como na importação local

### Pós-processamento 3D opcional (§4.3)

- [ ] Opção desligada por padrão nas configurações do modelo 3D
- [ ] Decimação com alvo de triângulos configurável
- [ ] Redimensionar texturas (512 / 1024 / 2048)
- [ ] Centralizar, normalizar escala e pivô no chão
- [ ] Export FBX
- [ ] Preservar o arquivo original e salvar o otimizado ao lado

---

## Fase 4 — 1.0, parte 2: fluxos

### Templates (§5.2)

- [ ] Modelo de dados do template: texto base com `{prompt}`, negativo, modelos 2D/3D, parâmetros, pós-processamento
- [ ] Biblioteca de templates: criar, duplicar, renomear, excluir (Fluxos → Gerenciar templates)
- [ ] Montagem do prompt final a partir do template + item

### Editor de fluxos (§5.3, §5.4)

- [ ] Criar, abrir, salvar fluxos (menu Arquivo)
- [ ] Adicionar, remover e reordenar itens
- [ ] Por item: prompt, quantidade de imagens, modo 3D
- [ ] 3D desmarcado por padrão com várias imagens, com confirmação ao marcar "Todas"
- [ ] Modos 3D: Não / Todas / Escolher depois
- [ ] Sobrescrever qualquer parâmetro do template por item

### Fila de execução (§5.5, §5.6)

- [ ] Fila persistida no SQLite, com retomada após fechar o app ou reiniciar o PC
- [ ] Execução intercalada: imagens do item → 3D do item → próximo item
- [ ] Falha isolada: marcar item como falho, gravar log e seguir
- [ ] Erros da fila não abrem modal; ficam acumulados para o relatório
- [ ] Retry automático com configurações mais leves em falta de VRAM
- [ ] Tentar novamente itens falhos ao final
- [ ] Pausar, retomar, cancelar e reordenar pendentes
- [ ] Impedir suspensão do sistema durante a fila (`systemd-inhibit`/D-Bus)
- [ ] Ação ao terminar a fila: nada / notificar / suspender / desligar
- [ ] Tela da fila de execução (Fluxos → Fila de execução) e progresso na barra de status

### Modo "Escolher depois" e relatório

- [ ] Tela para escolher as imagens favoritas de cada item
- [ ] Enviar as escolhidas para uma fila só de 3D
- [ ] Relatório final: concluídos, falhos com link para o log, tempo total
- [ ] Notificação do sistema ao terminar

### Galeria completa

- [ ] Filtros por data, modelo, fluxo e tipo (2D/3D)
- [ ] Comparar variações lado a lado

---

## Fase 5 — Polimento da 1.0

- [ ] Tela de diagnóstico com relatório copiável (hardware, ISA, driver, backend, versões, combinações que falharam, últimos erros) (§6.8)
- [ ] Botão "Refazer setup de hardware" (Configurações e Ajuda), limpando a memória de compatibilidade e preservando modelos (§6.7)
- [ ] Verificação rápida de hardware na inicialização e aviso de mudança (GPU, driver, ROCm)
- [ ] Ações "Tentar novamente com configuração segura" no modal de erro ligadas aos fallbacks
- [ ] Revisão completa das traduções pt-BR e en
- [ ] README: dependências por distribuição (Arch/CachyOS, Debian/Ubuntu, Fedora), verificação do Vulkan, compilação e uso
- [ ] Documentação de uso (geração, edição, 3D, fluxos)
- [ ] Checklist de testes manuais na RX 7600 antes de marcar a 1.0
- [ ] Tag e release `v1.0.0` no GitHub

---

## Após a 1.0

- [ ] Rigging automático de personagens (§4.4)
- [ ] API local / CLI para integração com engines e scripts (§4.4)

## Backlog

- [ ] Importação de itens de fluxo via CSV/planilha (§5.7)
- [ ] Wildcards no prompt (§5.7)
- [ ] Presets de estilo no painel (§4.5)
- [ ] LoRAs (§4.5)
- [ ] Selo que distingue formato e quantização (safetensors, GGUF, MLX, NVFP4, Q4_K_M e afins), além do tamanho do arquivo (`docs/pesquisa/formatos-de-modelo.md`)
- [ ] Upscale de imagens (§4.5)
- [ ] `PKGBUILD` (AUR) ou AppImage nas releases (§12.3)
