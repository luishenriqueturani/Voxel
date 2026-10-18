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

- [ ] Criar estrutura do monorepo (`apps/desktop`, `crates/`, `workers/python`, `workers/sdcpp`, `protocol/`, `catalog/`) (§7.7)
- [ ] Inicializar app Tauri 2 com React + TypeScript + Vite em `apps/desktop`
- [ ] Configurar workspace Cargo com os crates `hardware`, `queue`, `models`, `imaging`, `logging`, `protocol`
- [ ] Inicializar projeto Python em `workers/python` com `uv` (`pyproject.toml` + `uv.lock`, Python 3.12)
- [ ] Criar comando único de build e tarefas comuns (`just` ou `make`): `build`, `dev`, `test`, `lint`, `fmt` (§12.3)
- [ ] Atualizar `.gitignore` (target, node_modules, .venv, builds do sd.cpp, modelos)

### Qualidade de código

- [ ] Rust: `rustfmt` + `clippy` com avisos como erro
- [ ] TypeScript: ESLint + Prettier (ou Biome) e `tsc --noEmit` estrito
- [ ] Python: `ruff` (lint + formatação) e `pyright`/`mypy`
- [ ] Hooks de pré-commit rodando formatação e lint
- [ ] Definir padrão Conventional Commits (documentar no README)

### Testes e CI

- [ ] Configurar Vitest + Testing Library no frontend
- [ ] Configurar `cargo test` / `cargo-nextest`, `mockall` e `insta` no Rust
- [ ] Configurar pytest + `pytest-mock` no worker Python
- [ ] Configurar relatório de cobertura nas três linguagens com mínimo por camada (ex.: 80%)
- [ ] Configurar E2E com WebdriverIO + `tauri-driver`
- [ ] Workflow do GitHub Actions (Linux): lint → testes unitários → contrato → E2E → build (§12.2)

### Interface base

- [ ] Configurar Tailwind CSS e biblioteca de componentes (shadcn/ui ou Radix)
- [ ] Configurar i18next com pt-BR e en, e troca de idioma em tempo de execução
- [ ] Layout principal: painel esquerdo, área principal e barra de status (§3.1)
- [ ] Painel esquerdo estático: prompt, prompt negativo recolhível, seleção de modelo, botão ⚙, opção 2D/3D, botão Gerar, lista de recentes (§3.2)
- [ ] Menu nativo do Tauri com todos os menus e itens (ainda sem ação onde não houver funcionalidade) (§3.4)
- [ ] Atalho `Ctrl+Enter` para gerar
- [ ] Tema claro/escuro
- [ ] Tela de configurações com as seções vazias (Geral, Hardware, Modelos, Ambiente 3D, Execução, Logs) (§3.5)

### Core e persistência

- [ ] Resolver diretórios XDG (`~/.config/voxel`, `~/.local/share/voxel`, `~/.local/state/voxel`, `~/.cache/voxel`) (§10)
- [ ] Banco SQLite com sistema de migrações
- [ ] Leitura/gravação de configurações do usuário

### Logs e erros (§8)

- [ ] Log geral do app com `tracing`, rotativo por dias
- [ ] Arquivo de log por erro com mensagem, código de saída/sinal, últimas linhas de stdout/stderr, parâmetros do job, backend, modelo e resumo do hardware (§8.1)
- [ ] Captura contínua de stdout/stderr dos processos filhos
- [ ] Classificador de erros baseado em padrões, como tabela de dados (§8.3)
  - [ ] Usar o log do InvokeAI (§6.2) como fixture de teste
- [ ] Modal de erro: resumo em linguagem simples, log rolável, Copiar, Abrir arquivo de log, Abrir diagnóstico (§8.2)

### Protocolo e workers

- [ ] Decidir protocolo core ↔ worker (HTTP/WebSocket local vs. stdio JSON-RPC) e registrar no planejamento (§7.4)
- [ ] Escrever esquema do protocolo (OpenAPI/JSON Schema) e gerar tipos em Rust, TypeScript e Python
- [ ] Interfaces de backend: `ImageBackend`, `MeshBackend`, `BackgroundRemover`, `MeshPostProcess` (§7.5)
- [ ] Supervisor de workers no core: iniciar, autenticar com token, monitorar, detectar saída/sinal, reiniciar
- [ ] Worker falso (mock) que devolve imagens/malhas de exemplo e progresso simulado (§11.2)
- [ ] Worker falso capaz de simular falhas: abortar com sinal 6, imprimir `HSA_STATUS_ERROR_INVALID_ISA`, falta de memória
- [ ] Testes de contrato entre core e workers

---

## Fase 1 — Pesquisa de compatibilidade

> Pode rodar em paralelo com a fase 0. O resultado de cada item é registrado em `docs/pesquisa/` e alimenta o catálogo e os perfis.

### 2D

- [ ] Compilar stable-diffusion.cpp com Vulkan e testar na RX 7600 (SD 1.5, SDXL, Flux quantizado)
- [ ] Compilar stable-diffusion.cpp com ROCm e comparar com Vulkan na RX 7600
- [ ] Testar stable-diffusion.cpp em CPU
- [ ] Verificar suporte a img2img, inpainting e padding circular (texturas tileáveis) no stable-diffusion.cpp
- [ ] Verificar se é possível salvar latentes e decodificar separadamente

### Caso RX 7600 / ROCm (§6.2, §6.3)

- [ ] Reproduzir o crash do VAE decode com PyTorch + ROCm puro e `HSA_OVERRIDE_GFX_VERSION=11.0.0`
- [ ] Verificar se há build de PyTorch/ROCm com suporte nativo a `gfx1102` (sem override)
- [ ] Validar mitigações: desligar kernels assembly da MIOpen, desligar MIOpen no PyTorch, VAE em tiles, VAE em fp32, VAE na CPU
- [ ] Documentar a combinação de variáveis de ambiente que funciona e sua perda de desempenho

### 3D

- [ ] Testar TripoSR na RX 7600 (ROCm e CPU)
- [ ] Testar Stable Fast 3D na RX 7600 (ROCm e CPU)
- [ ] Avaliar Hunyuan3D 2.x mini, TRELLIS e InstantMesh quanto à dependência de CUDA
- [ ] Testar em NVIDIA, se houver acesso

### Auxiliares

- [ ] Testar remoção de fundo via ONNX Runtime (crate `ort`) em CPU com BiRefNet e U²-Net/IS-Net
- [ ] Avaliar bibliotecas de decimação de malha (`meshoptimizer`, `trimesh`, `pymeshlab`)

### Consolidação

- [ ] Medir VRAM, RAM e tempo de cada combinação modelo + backend
- [ ] Preencher a tabela de perfis de hardware (§6.6)
- [ ] Verificar licenças dos modelos candidatos
- [ ] Definir os modelos e backends do MVP

---

## Fase 2 — MVP

### Detecção de hardware (§6.5)

- [ ] Coletar distribuição, kernel e versão do driver (Mesa/AMDGPU, NVIDIA)
- [ ] Coletar CPU (núcleos, AVX2/AVX-512), RAM total/disponível e espaço em disco
- [ ] Enumerar GPUs via Vulkan: fabricante, modelo, VRAM
- [ ] Detectar ISA real da GPU AMD (`rocminfo`) e presença de ROCm/CUDA (NVML)
- [ ] Função pura "hardware → perfil" (§6.6), testada com vários cenários (RX 7600, RX 7900, GTX 1060, RTX 4070, Intel Arc, só CPU)

### Setup inicial

- [ ] Tela de boas-vindas com progresso do setup
- [ ] Teste real de cada backend candidato: geração mínima completa, com VAE decode em fp16, em processo isolado (§6.3)
- [ ] Ordem de fallback entre backends (§6.4)
- [ ] Memória de compatibilidade local: registrar e evitar combinações que quebraram (§6.3)
- [ ] Sugestão de modelos iniciais com tamanho e licença
- [ ] Geração de teste ponta a ponta ao final do setup
- [ ] Modo avançado para sobrescrever escolhas

### Worker stable-diffusion.cpp

- [ ] Build do stable-diffusion.cpp com Vulkan integrado ao comando de build
- [ ] Adaptador do worker implementando `ImageBackend` (txt2img)
- [ ] Progresso em tempo real e prévia da imagem
- [ ] Cancelamento de geração
- [ ] Salvar latentes antes do decode e refazer só o decode com fallback em caso de crash (§6.3)
- [ ] Opção nas configurações para trocar de backend, com "voltar ao recomendado" no modal de erro (§6.4)

### Gerenciador de modelos (§9.4)

- [ ] Formato do catálogo de modelos (JSON versionado) e primeiros modelos cadastrados
- [ ] Download retomável com progresso e verificação de hash
- [ ] Token do Hugging Face para modelos com acesso restrito
- [ ] Cálculo do selo de compatibilidade (Compatível / Com ressalvas / Incompatível) com motivo
- [ ] Ocultar incompatíveis por padrão, com opção "mostrar modelos incompatíveis"
- [ ] Aviso e confirmação explícita ao baixar modelo incompatível
- [ ] Exibir licença antes do download
- [ ] Importar modelo local (`.safetensors`/`.gguf`) com identificação da arquitetura pelos metadados e estimativa de compatibilidade
- [ ] Remover modelos e abrir pasta de modelos

### Geração 2D

- [ ] Ligar o painel esquerdo ao worker: prompt, prompt negativo, modelo, parâmetros do ⚙
- [ ] Parâmetros padrão vindos do perfil de hardware
- [ ] Visualizador 2D: zoom, salvar, copiar, regerar com mesma seed
- [ ] Salvar saídas na estrutura de pastas (§10) com `metadados.json` e metadados embutidos no PNG
- [ ] Barra de status com GPU, backend e uso de VRAM

### Remoção de fundo

- [ ] `BackgroundRemover` no core via ONNX Runtime em CPU
- [ ] Remover fundo de imagem gerada ou aberta do disco (Arquivo → Abrir imagem para edição)
- [ ] Pré-visualização da transparência e export PNG transparente
- [ ] Ação "Remover fundo" no visualizador e no menu Editar

### Histórico e galeria

- [ ] Lista de prompts recentes no painel, com clique para recarregar prompt e parâmetros
- [ ] Busca e fixação (★) de favoritos nos recentes
- [ ] Galeria simples com as gerações

---

## Fase 3 — 1.0, parte 1: edição e 3D

### Edição de imagem (§4.2)

- [ ] Img2img com controle de intensidade
- [ ] Editor de máscara com pincel (tamanho, borracha, desfazer)
- [ ] Inpainting usando a máscara
- [ ] Texturas tileáveis (padding circular)
- [ ] Pré-visualização de textura em mosaico 3×3

### Ambiente Python (§7.3)

- [ ] Instalação via `uv` de Python isolado e venv na pasta de dados do app
- [ ] Instalação do PyTorch na variante certa (CUDA, ROCm ou CPU) conforme o hardware
- [ ] Aplicação das variáveis de ambiente validadas na fase 1 ao iniciar o worker
- [ ] Gatilhos de intenção 3D: opção 3D no painel, "Gerar 3D" em item de fluxo, template com 3D, download de modelo 3D, "enviar para 3D"
- [ ] Instalação em segundo plano com progresso e tamanho na barra de status
- [ ] Gerações 3D pedidas durante a instalação aguardam o ambiente em vez de falhar
- [ ] Teste real do ambiente ao final da instalação
- [ ] Seção "Ambiente 3D" nas configurações: estado, reinstalar, remover

### Worker Python e 3D

- [ ] Servidor do worker Python (FastAPI) seguindo o protocolo
- [ ] Adaptador do primeiro modelo imagem → 3D implementando `MeshBackend`
- [ ] Remoção de fundo automática antes do 3D
- [ ] Descarregar o modelo 2D da VRAM antes da etapa 3D
- [ ] Modo 3D do painel: texto → imagem → 3D
- [ ] 3D a partir de imagem existente (arrastar para o painel e "enviar para 3D")
- [ ] Export GLB e OBJ

### Visualizador 3D

- [ ] Viewer com `react-three-fiber`: orbitar, zoom, wireframe, textura
- [ ] Exibir contagem de polígonos

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
- [ ] Upscale de imagens (§4.5)
- [ ] `PKGBUILD` (AUR) ou AppImage nas releases (§12.3)
