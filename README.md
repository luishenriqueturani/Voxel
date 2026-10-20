# Voxel

Aplicação desktop para Linux que gera e edita imagens 2D e gera modelos 3D com IA rodando 100% local, pensada para funcionar bem em hardware comum (inclusive GPUs AMD de consumo, via Vulkan).

> Em desenvolvimento inicial. O planejamento completo está em [`docs/PLANEJAMENTO.md`](docs/PLANEJAMENTO.md) e as tarefas em [`docs/TODO.md`](docs/TODO.md).

## Requisitos

- [Rust](https://rustup.rs/) 1.92 ou superior
- [Bun](https://bun.sh/)
- [uv](https://github.com/astral-sh/uv)
- GNU Make
- Dependências de sistema do Tauri

No Arch/CachyOS:

```sh
sudo pacman -S --needed base-devel webkit2gtk-4.1 libappindicator-gtk3 librsvg openssl uv
```

Para outras distribuições, veja os [pré-requisitos do Tauri](https://tauri.app/start/prerequisites/#linux).

Para desenvolver (testes, cobertura e testes ponta a ponta):

```sh
rustup component add llvm-tools-preview
cargo install --locked cargo-nextest cargo-llvm-cov tauri-driver
sudo pacman -S --needed webkitgtk-6.0   # fornece o WebKitWebDriver
```

## Compilando e rodando

```sh
make install   # dependências do frontend e do worker Python, e ativa os hooks do git
make dev       # abre o app em modo de desenvolvimento
make build     # compila o app em target/release/voxel
```

## Desenvolvimento

| Comando | O que faz |
| --- | --- |
| `make test` | Roda os testes unitários de todas as camadas |
| `make coverage` | Roda os testes exigindo cobertura mínima de 80% em cada camada |
| `make e2e` | Compila o app em modo debug e roda os testes ponta a ponta (WebdriverIO + `tauri-driver`) |
| `make lint` | Clippy, TypeScript + Biome, ruff + mypy |
| `make fmt` | Formata o código (rustfmt, Biome, ruff) |
| `make fmt-check` | Verifica a formatação sem alterar arquivos |
| `make hooks` | Ativa os hooks do git em `.githooks/` |

Estrutura:

```
apps/desktop/      app Tauri (frontend React em src/, core Rust em src-tauri/)
crates/            crates Rust do core (hardware, queue, models, imaging, logging, protocol)
workers/python/    worker Python (modelos 3D)
workers/sdcpp/     integração com o stable-diffusion.cpp
protocol/          esquema do protocolo core ↔ workers
catalog/           catálogo de modelos
docs/              planejamento e tarefas
```

### Hooks do git

Ativados com `make hooks` (ou `make install`):

- **pre-commit** — roda `make fmt-check lint`.
- **commit-msg** — valida a mensagem no padrão Conventional Commits.

### Padrão de commits

O projeto segue o [Conventional Commits](https://www.conventionalcommits.org/pt-br/v1.0.0/):

```
<tipo>(<escopo opcional>): <descrição>

<corpo opcional>
```

| Tipo | Uso |
| --- | --- |
| `feat` | Nova funcionalidade |
| `fix` | Correção de bug |
| `docs` | Apenas documentação |
| `style` | Formatação, sem mudança de comportamento |
| `refactor` | Mudança de código que não corrige bug nem adiciona funcionalidade |
| `perf` | Melhoria de desempenho |
| `test` | Adição ou ajuste de testes |
| `build` | Sistema de build e dependências |
| `ci` | Configuração de CI |
| `chore` | Tarefas de manutenção |
| `revert` | Reversão de commit |

Regras:

- Tipo e escopo em minúsculas; o escopo costuma ser o crate ou a área (`queue`, `hardware`, `desktop`, `worker`).
- Descrição no imperativo, em português, sem ponto final (ex.: `feat(queue): retomar fila após reinício`).
- Primeira linha com no máximo 100 caracteres.
- Mudanças incompatíveis levam `!` após o tipo/escopo (ex.: `feat(protocol)!: trocar formato das mensagens`).

## Licença

[MIT](LICENSE)
