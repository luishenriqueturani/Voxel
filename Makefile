DESKTOP := apps/desktop
PYTHON_WORKER := workers/python
COVERAGE_MIN := 80

# O cache do uv e o projeto podem estar em sistemas de arquivos diferentes, o que impede hardlinks.
export UV_LINK_MODE := copy

# main.rs e lib.rs do app só fazem a ligação com o Tauri; a lógica fica em módulos e crates testáveis.
RUST_COVERAGE_IGNORE := apps/desktop/src-tauri/src/(main|lib)\.rs$$

.PHONY: help install hooks dev build build-debug \
	test test-rust test-frontend test-python test-hooks \
	coverage coverage-rust coverage-frontend coverage-python \
	e2e \
	lint lint-rust lint-frontend lint-python \
	fmt fmt-check clean

help:
	@echo "Alvos disponíveis:"
	@echo "  install      Instala as dependências e ativa os hooks do git"
	@echo "  hooks        Ativa os hooks do git (.githooks)"
	@echo "  dev          Abre o app em modo de desenvolvimento"
	@echo "  build        Compila o app (sem gerar instaladores)"
	@echo "  build-debug  Compila o app em modo debug (usado pelos testes E2E)"
	@echo "  test         Roda todos os testes unitários"
	@echo "  coverage     Roda os testes com cobertura mínima de $(COVERAGE_MIN)%"
	@echo "  e2e          Roda os testes ponta a ponta (requer tauri-driver e WebKitWebDriver)"
	@echo "  lint         Roda todos os linters"
	@echo "  fmt          Formata todo o código"
	@echo "  fmt-check    Verifica a formatação sem alterar arquivos"
	@echo "  clean        Remove artefatos de build"

install: hooks
	cd $(DESKTOP) && bun install
	cd $(PYTHON_WORKER) && uv sync

hooks:
	git config core.hooksPath .githooks

dev:
	cd $(DESKTOP) && bun run tauri dev

build:
	cd $(DESKTOP) && bun run tauri build --no-bundle

build-debug:
	cd $(DESKTOP) && bun run tauri build --debug --no-bundle

test: test-rust test-frontend test-python test-hooks

test-rust:
	cargo nextest run --workspace --no-tests=pass

test-frontend:
	cd $(DESKTOP) && bun run test

test-python:
	cd $(PYTHON_WORKER) && uv run pytest

test-hooks:
	sh .githooks/tests/test_commit_msg.sh

coverage: coverage-rust coverage-frontend coverage-python test-hooks

coverage-rust:
	cargo llvm-cov nextest --workspace --no-tests=pass \
		--ignore-filename-regex '$(RUST_COVERAGE_IGNORE)' \
		--fail-under-lines $(COVERAGE_MIN)

coverage-frontend:
	cd $(DESKTOP) && bun run coverage

coverage-python:
	cd $(PYTHON_WORKER) && uv run pytest --cov --cov-report=term --cov-report=xml

e2e: build-debug
	cd $(DESKTOP) && bun run e2e

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
	rm -rf $(DESKTOP)/dist $(DESKTOP)/node_modules $(DESKTOP)/coverage $(PYTHON_WORKER)/.venv
