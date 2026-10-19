DESKTOP := apps/desktop
PYTHON_WORKER := workers/python
COVERAGE_MIN := 80

# O cache do uv e o projeto podem estar em sistemas de arquivos diferentes, o que impede hardlinks.
export UV_LINK_MODE := copy

# main.rs e lib.rs do app só fazem a ligação com o Tauri; a lógica fica em módulos e crates testáveis.
RUST_COVERAGE_IGNORE := apps/desktop/src-tauri/src/(main|lib)\.rs$$

SDCPP_SRC := workers/sdcpp/stable-diffusion.cpp
SDCPP_BUILD := workers/sdcpp/build/vulkan
SDCPP_COMMIT := b13a482
SDCPP_PREFIX := workers/sdcpp/build/prefix
SDCPP_HEADERS := workers/sdcpp/build/vulkan-headers/include

.PHONY: help install hooks dev build build-debug build-sdcpp \
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
	@echo "  build        Compila o app e o stable-diffusion.cpp com Vulkan"
	@echo "  build-debug  Compila o app em modo debug (usado pelos testes E2E)"
	@echo "  build-sdcpp  Compila o sd-cli com Vulkan"
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

dev: build-sdcpp
	cargo build -p voxel-sdcpp
	cd $(DESKTOP) && bun run tauri dev

build: build-sdcpp
	cargo build -p voxel-sdcpp --release
	cd $(DESKTOP) && bun run tauri build --no-bundle

build-debug: build-sdcpp
	cargo build -p voxel-sdcpp
	cd $(DESKTOP) && bun run tauri build --debug --no-bundle

# O código do upstream não entra no git. O commit é o que a pesquisa mediu.
build-sdcpp:
	@if [ ! -d $(SDCPP_SRC)/.git ]; then \
		git clone --filter=blob:none https://github.com/leejet/stable-diffusion.cpp $(SDCPP_SRC); \
	fi
	@if [ "$$(git -C $(SDCPP_SRC) rev-parse --short=7 HEAD)" != "$(SDCPP_COMMIT)" ]; then \
		git -C $(SDCPP_SRC) fetch --depth 1 origin $(SDCPP_COMMIT); \
		git -C $(SDCPP_SRC) checkout --detach $(SDCPP_COMMIT); \
	fi
	@if [ ! -f $(SDCPP_BUILD)/CMakeCache.txt ]; then \
		test -d $(SDCPP_HEADERS); \
		cmake -S $(SDCPP_SRC) -B $(SDCPP_BUILD) \
			-DSD_VULKAN=ON -DSD_SERVER_BUILD_FRONTEND=OFF -DCMAKE_BUILD_TYPE=Release \
			-DVulkan_INCLUDE_DIR="$(CURDIR)/$(SDCPP_HEADERS)" \
			-DVulkan_LIBRARY=/usr/lib/libvulkan.so \
			-DCMAKE_PREFIX_PATH="$(CURDIR)/$(SDCPP_PREFIX)" \
			-DCMAKE_CXX_FLAGS="-I$(CURDIR)/$(SDCPP_PREFIX)/include" \
			-DCMAKE_C_FLAGS="-I$(CURDIR)/$(SDCPP_PREFIX)/include"; \
	fi
	cmake --build $(SDCPP_BUILD) --target sd-cli

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
