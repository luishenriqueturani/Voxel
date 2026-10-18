DESKTOP := apps/desktop
PYTHON_WORKER := workers/python

# O cache do uv e o projeto podem estar em sistemas de arquivos diferentes, o que impede hardlinks.
export UV_LINK_MODE := copy

.PHONY: help install hooks dev build \
	test test-rust test-frontend test-python test-hooks \
	lint lint-rust lint-frontend lint-python \
	fmt fmt-check clean

help:
	@echo "Alvos disponíveis:"
	@echo "  install    Instala as dependências e ativa os hooks do git"
	@echo "  hooks      Ativa os hooks do git (.githooks)"
	@echo "  dev        Abre o app em modo de desenvolvimento"
	@echo "  build      Compila o app (sem gerar instaladores)"
	@echo "  test       Roda todos os testes"
	@echo "  lint       Roda todos os linters"
	@echo "  fmt        Formata todo o código"
	@echo "  fmt-check  Verifica a formatação sem alterar arquivos"
	@echo "  clean      Remove artefatos de build"

install: hooks
	cd $(DESKTOP) && bun install
	cd $(PYTHON_WORKER) && uv sync

hooks:
	git config core.hooksPath .githooks

dev:
	cd $(DESKTOP) && bun run tauri dev

build:
	cd $(DESKTOP) && bun run tauri build --no-bundle

test: test-rust test-frontend test-python test-hooks

test-rust:
	cargo test --workspace

test-frontend:
	cd $(DESKTOP) && bun run typecheck

# pytest sai com código 5 quando não há testes coletados.
test-python:
	cd $(PYTHON_WORKER) && { uv run pytest; status=$$?; [ $$status -eq 0 ] || [ $$status -eq 5 ]; }

test-hooks:
	sh .githooks/tests/test_commit_msg.sh

lint: lint-rust lint-frontend lint-python

lint-rust:
	cargo clippy --workspace --all-targets -- -D warnings

lint-frontend:
	cd $(DESKTOP) && bun run typecheck && bun run lint

lint-python:
	cd $(PYTHON_WORKER) && uv run ruff check . && uv run mypy

fmt:
	cargo fmt --all
	cd $(DESKTOP) && bunx biome check --write .
	cd $(PYTHON_WORKER) && uv run ruff format . && uv run ruff check --select I --fix .

fmt-check:
	cargo fmt --all -- --check
	cd $(DESKTOP) && bun run format:check
	cd $(PYTHON_WORKER) && uv run ruff format --check .

clean:
	cargo clean
	rm -rf $(DESKTOP)/dist $(DESKTOP)/node_modules $(PYTHON_WORKER)/.venv
