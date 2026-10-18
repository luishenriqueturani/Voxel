DESKTOP := apps/desktop
PYTHON_WORKER := workers/python

.PHONY: help install dev build test test-rust test-frontend test-python lint lint-rust lint-frontend lint-python fmt fmt-check clean

help:
	@echo "Alvos disponíveis:"
	@echo "  install    Instala as dependências do frontend e do worker Python"
	@echo "  dev        Abre o app em modo de desenvolvimento"
	@echo "  build      Compila o app (sem gerar instaladores)"
	@echo "  test       Roda todos os testes"
	@echo "  lint       Roda todos os linters"
	@echo "  fmt        Formata todo o código"
	@echo "  fmt-check  Verifica a formatação sem alterar arquivos"
	@echo "  clean      Remove artefatos de build"

install:
	cd $(DESKTOP) && bun install
	cd $(PYTHON_WORKER) && uv sync

dev:
	cd $(DESKTOP) && bun run tauri dev

build:
	cd $(DESKTOP) && bun run tauri build --no-bundle

test: test-rust test-frontend test-python

test-rust:
	cargo test --workspace

test-frontend:
	cd $(DESKTOP) && bun run typecheck

# pytest sai com código 5 quando não há testes coletados.
test-python:
	cd $(PYTHON_WORKER) && { uv run pytest; status=$$?; [ $$status -eq 0 ] || [ $$status -eq 5 ]; }

lint: lint-rust lint-frontend lint-python

lint-rust:
	cargo clippy --workspace --all-targets -- -D warnings

lint-frontend:
	cd $(DESKTOP) && bun run typecheck

lint-python:
	cd $(PYTHON_WORKER) && uv run ruff check .

fmt:
	cargo fmt --all
	cd $(PYTHON_WORKER) && uv run ruff format .

fmt-check:
	cargo fmt --all -- --check
	cd $(PYTHON_WORKER) && uv run ruff format --check .

clean:
	cargo clean
	rm -rf $(DESKTOP)/dist $(DESKTOP)/node_modules $(PYTHON_WORKER)/.venv
