# tcl-lsp — build, test, and package
#
# Targets:
#   make prep-pr       Run the full CI-equivalent gate before opening a PR
#   make vsix          Build the .vsix file (runs tests first)
#   make install       Build and install the .vsix into VS Code
#   make test          Run all tests (Python + VS Code extension)
#   make test-py       Run the Python test suite only (excludes VM tcltest tests)
#   make test-opt      Run optimiser coverage tests (not part of standard CI)
#   make test-fuzz     Run differential fuzz tests (pytest, FUZZ_ITERATIONS=N)
#   make fuzz          Run standalone fuzz campaign (N=iterations, SEED=base_seed)
#   make test-ext      Run VS Code extension integration tests
#   make lint-py       Lint Python code with Ruff
#   make format-py     Format and auto-fix Python code with Ruff
#   make format-ts     Format TypeScript extension code with Prettier
#   make typecheck-py-full Type-check all Python code with ty (broader coverage)
#   make typecheck-ts  Type-check TypeScript extension code with tsc
#   make npm-env       Install/update npm dependencies
#   make compile       Compile the TypeScript extension
#   make zipapp-tcl    Build the unified Tcl tools zipapp
#   make zipapp-cli    Build the CLI compiler explorer zipapp
#   make zipapp-gui    Build the standalone GUI zipapp (bundles Pyodide)
#   make zipapp-gui-cdn Build the CDN GUI zipapp (loads Pyodide from CDN)
#   make zipapp-lsp    Build the LSP server zipapp
#   make zipapp-wasm   Build the WASM compiler zipapp
#   make zipapp-ai     Build the AI analysis zipapp (for Claude Code skills)
#   make claude-skills Build the Claude Code skills release zip
#   make zipapps       Build all zipapps (Tcl, CLI, GUI, GUI-CDN, LSP, AI, MCP, WASM)
#   make jetbrains     Build the JetBrains plugin (.zip)
#   make sublime       Build the Sublime Text package (.sublime-package)
#   make zed           Build the Zed extension archive (.tar.gz)
#   make screenshots   Capture extension screenshots and build demo GIF (macOS)
#   make release       Build all release artifacts (parity with tagged CI release jobs)
#   make release-tag   Bump version, tag, and push (V=x.y.z)
#   make coverage      Generate all coverage reports (Python + VS Code)
#   make coverage-py   Run Python tests with coverage (HTML + XML in tmp/coverage/python/)
#   make coverage-ext  Run VS Code extension tests with coverage (HTML in tmp/coverage/vscode/)
#   make clean         Remove build artifacts
#   make distclean     Remove build artifacts and node_modules
#
# Prerequisites:
#   - Python 3.10+ with uv (https://docs.astral.sh/uv/)
#   - Node.js 20+ with npm
#

SHELL := /bin/bash
.DELETE_ON_ERROR:

# Directories
ROOT     := $(dir $(abspath $(lastword $(MAKEFILE_LIST))))
EXT_DIR  := $(ROOT)editors/vscode
LSP_DIR  := $(ROOT)lsp
PYCORE_DIR := $(ROOT)core
VM_DIR   := $(ROOT)vm
TEST_DIR := $(ROOT)tests
OUT_DIR  := $(EXT_DIR)/out
EXPLORER_DIR    := $(ROOT)explorer
EXPLORER_STATIC := $(EXPLORER_DIR)/static

# Build output — everything generated goes under build/
BUILD_DIR  := $(ROOT)build

# Tools
UV       := uv
NPM      := npm
NODE_BIN := $(EXT_DIR)/node_modules/.bin
TSC      := $(NODE_BIN)/tsc
VSCE     := $(NODE_BIN)/vsce
VSCODE   ?= code

# Stamps (used to avoid re-running expensive steps when deps haven't changed)
STAMP_DIR  := $(BUILD_DIR)/stamps
NPM_STAMP  := $(STAMP_DIR)/npm-install
UV_STAMP   := $(STAMP_DIR)/uv-sync
STAGE_DIR  := $(BUILD_DIR)/vsix-stage

# Version — derived from git describe (fallback: dev when unavailable)
GIT_DESCRIBE_RAW := $(shell git describe --tags --abbrev=1 --always --dirty=-dev 2>/dev/null || true)
GIT_DESCRIBE     := $(if $(strip $(GIT_DESCRIBE_RAW)),$(GIT_DESCRIBE_RAW),dev)
GIT_HASH         := $(shell git rev-parse --short HEAD 2>/dev/null || echo unknown)
VERSION          := $(shell echo "$(GIT_DESCRIBE)" | sed 's/^v//')
SEMVER_VERSION   := $(shell sh -c 'v="$(VERSION)"; if echo "$$v" | grep -Eq "^[0-9]+\\.[0-9]+\\.[0-9]+([-.][0-9A-Za-z.-]+)*$$"; then echo "$$v"; else echo "0.0.0-dev"; fi')
FULL_VERSION     := $(VERSION)
BUILD_TIMESTAMP := $(shell date -u +%Y-%m-%dT%H:%M:%SZ)

# Derived paths
VSIX_FILE := $(BUILD_DIR)/tcl-lsp-vscode-$(VERSION).vsix
LICENSE_SRC := $(ROOT)LICENSE

# Build-info files (generated, gitignored)
BUILD_INFO      := $(ROOT)lsp/_build_info.py
BUILD_INFO_JSON := $(EXPLORER_STATIC)/build_info.json

# Find all Python source files for dependency tracking
PY_SRCS  := $(shell find $(LSP_DIR) $(PYCORE_DIR) $(EXPLORER_DIR) -name '*.py' -not -path '*__pycache__*' -not -name '_build_info.py')
VM_SRCS  := $(shell find $(VM_DIR) -name '*.py' -not -path '*__pycache__*')
PY_TESTS := $(shell find $(TEST_DIR) -name '*.py' -not -path '*__pycache__*')
TS_SRCS  := $(shell find $(EXT_DIR)/src -name '*.ts' 2>/dev/null)

# Main targets

.PHONY: vsix verify-vsix install test test-py test-slow test-opt test-ext lint lint-py typecheck-py typecheck-py-full lint-ts format format-py format-ts typecheck-ts npm-env compile clean distclean help explorer-build explorer-build-cdn compiler-explorer-gui zipapp-tcl zipapp-cli zipapp-gui zipapp-gui-cdn zipapp-lsp zipapp-ai zipapp-mcp zipapp-wasm zipapps claude-skills package-vsix jetbrains sublime zed release release-tag build-info screenshot screenshots clean-screenshots prep-pr smoke-zipapps smoke-vsix copy-canonical coverage coverage-py coverage-ext generate check-generated .FORCE

help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## ' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-20s\033[0m %s\n", $$1, $$2}'

vsix: lint test compile verify-vsix ## Build the .vsix (tests must pass first)
install: package-vsix ## Build and install the .vsix into VS Code
	@echo "==> Installing VS Code extension"
	$(VSCODE) --install-extension $(VSIX_FILE) --force

README_SRC     := $(ROOT)README.md
SCREENSHOT_DIR := $(ROOT)docs/screenshots
SCREENSHOTS    := $(wildcard $(SCREENSHOT_DIR)/*.png $(SCREENSHOT_DIR)/*.gif)

$(VSIX_FILE): $(OUT_DIR)/extension.js $(PY_SRCS) $(EXT_DIR)/package.json $(EXT_DIR)/.vscodeignore $(LICENSE_SRC) $(README_SRC) $(SCREENSHOTS) $(BUILD_INFO) $(ROOT)scripts/build_zipapp.py $(ROOT)scripts/zipapp_lsp_main.py
	@echo "==> Preparing VSIX staging directory"
	rm -rf $(STAGE_DIR)
	mkdir -p $(STAGE_DIR)
	rsync -a --delete --delete-excluded \
		--exclude='.venv/' \
		--exclude='.pytest_cache/' \
		--exclude='.ruff_cache/' \
		--exclude='.mypy_cache/' \
		--exclude='.vscode-test/' \
		$(EXT_DIR)/ $(STAGE_DIR)/
	@# Inject version from git describe into staged package.json
	node -e "const f='$(STAGE_DIR)/package.json';const p=JSON.parse(require('fs').readFileSync(f));p.version='$(SEMVER_VERSION)';require('fs').writeFileSync(f,JSON.stringify(p,null,2)+'\n')"
	@echo "==> Building LSP server zipapp (minified)"
	python3 $(ROOT)scripts/build_zipapp.py --minify lsp \
		--version $(VERSION) \
		--output $(STAGE_DIR)/tcl-lsp-server.pyz
	cp $(LICENSE_SRC) $(STAGE_DIR)/LICENSE.txt
	cp $(README_SRC) $(STAGE_DIR)/README.md
	mkdir -p $(STAGE_DIR)/docs/screenshots
	cp $(SCREENSHOT_DIR)/*.png $(SCREENSHOT_DIR)/*.gif $(STAGE_DIR)/docs/screenshots/
	@echo "==> Optimising images for release"
	@if command -v pngquant >/dev/null 2>&1 && command -v optipng >/dev/null 2>&1; then \
		for f in $(STAGE_DIR)/docs/screenshots/*.png; do \
			pngquant --quality=65-80 --speed 1 --strip --force --output "$$f" "$$f" 2>/dev/null; \
			optipng -o5 -strip all -quiet "$$f" 2>/dev/null; \
		done; \
		echo "    PNG optimisation complete"; \
	else \
		echo "    WARN: pngquant/optipng not found — skipping PNG optimisation"; \
	fi
	@if command -v gifsicle >/dev/null 2>&1; then \
		for f in $(STAGE_DIR)/docs/screenshots/*.gif; do \
			gifsicle -O3 --lossy=80 --colors 128 "$$f" -o "$$f.opt" 2>/dev/null && mv "$$f.opt" "$$f"; \
		done; \
		echo "    GIF optimisation complete"; \
	else \
		echo "    WARN: gifsicle not found — skipping GIF optimisation"; \
	fi
	@echo "==> Packaging .vsix (stripped, not obfuscated)"
	cd $(STAGE_DIR) && $(VSCE) package --allow-missing-repository --no-update-package-json --no-git-tag-version -o $(VSIX_FILE)
	@echo ""
	@echo "Built: $(VSIX_FILE)"
	@ls -lh $(VSIX_FILE)

verify-vsix: $(VSIX_FILE) ## Fail if dev/cache artifacts leaked into the .vsix
	@echo "==> Verifying VSIX contents"
	@set -euo pipefail; \
		BAD_ENTRIES="$$(unzip -Z1 $(VSIX_FILE) | grep -E '^extension/(\.venv/|\.ruff_cache/|\.pytest_cache/|\.mypy_cache/|\.vscode-test/|\.stamps/|src/|testFixture/|out/test/|.*__pycache__/|.*\.pyc$$)' || true)"; \
		if [[ -n "$$BAD_ENTRIES" ]]; then \
			echo "VSIX contains dev/cache content that should be excluded:"; \
			echo "$$BAD_ENTRIES"; \
			exit 1; \
		fi
	@set -euo pipefail; \
		PYZ_COUNT="$$(unzip -Z1 $(VSIX_FILE) | grep -c '\.pyz$$' || true)"; \
		if [[ "$$PYZ_COUNT" -eq 0 ]]; then \
			echo "VSIX missing .pyz server bundle!"; \
			exit 1; \
		fi
	@set -euo pipefail; \
		RAW_SERVER="$$(unzip -Z1 $(VSIX_FILE) | grep -E '^extension/(lsp/|core/|pyproject\.toml$$|uv\.lock$$)' || true)"; \
		if [[ -n "$$RAW_SERVER" ]]; then \
			echo "VSIX contains raw lsp/core/pyproject.toml/uv.lock (should be .pyz only):"; \
			echo "$$RAW_SERVER"; \
			exit 1; \
		fi

# Test targets

test: test-py test-ext ## Run all tests (Python + VS Code extension)

lint: lint-py typecheck-py lint-ts ## Run all lint and style checks

format: format-py format-ts ## Format Python and TypeScript code

test-py: $(UV_STAMP) ## Run the Python test suite (excludes VM tcltest and fuzz campaign tests)
	@echo "==> Running Python tests"
	cd $(ROOT) && $(UV) run --extra dev pytest tests/ -q -n 4 --ignore-glob='*/test_vm_*_test.py' --ignore=tests/test_optimiser_coverage.py --ignore=tests/test_optimiser_vm_equivalence.py --ignore=tests/fuzz/

test-vm: $(UV_STAMP) ## Run VM tcltest suite (slow — runs Tcl test files through our VM)
	@echo "==> Running VM tcltest tests"
	cd $(ROOT) && $(UV) run --extra dev pytest tests/test_vm_*_test.py -q

lint-py: $(UV_STAMP) ## Lint Python code with Ruff (check, format, KCS docs)
	@echo "==> Checking KCS docs index links"
	cd $(ROOT) && $(UV) run python scripts/check_kcs_index_links.py
	@echo "==> Linting Python code with Ruff"
	cd $(ROOT) && $(UV) run --extra dev ruff check .
	@echo "==> Checking Python formatting with Ruff"
	cd $(ROOT) && $(UV) run --extra dev ruff format --check .

typecheck-py: $(UV_STAMP) $(BUILD_INFO) ## Type-check Python code with ty
	@echo "==> Type-checking Python code with ty"
	cd $(ROOT) && $(UV) run --extra dev ty check --exclude 'lsp/server.py' lsp core explorer tests scripts/tcl_test_client.py

typecheck-py-full: $(UV_STAMP) $(BUILD_INFO) ## Type-check all Python code with ty
	@echo "==> Type-checking all Python code with ty"
	cd $(ROOT) && $(UV) run --extra dev ty check --exclude 'lsp/server.py' ai core explorer lsp tests vm scripts

lint-ts: $(NPM_STAMP) ## Lint/format-check TypeScript extension code
	@echo "==> Linting TypeScript code (ESLint + Prettier check)"
	cd $(EXT_DIR) && $(NPM) run lint

format-py: $(UV_STAMP) ## Format and auto-fix Python code with Ruff
	@echo "==> Auto-fixing Python lint issues with Ruff"
	cd $(ROOT) && $(UV) run --extra dev ruff check --fix .
	@echo "==> Formatting Python code with Ruff"
	cd $(ROOT) && $(UV) run --extra dev ruff format .

format-ts: $(NPM_STAMP) ## Format TypeScript extension code with Prettier
	@echo "==> Formatting TypeScript code with Prettier"
	cd $(EXT_DIR) && $(NPM) run format

typecheck-ts: $(NPM_STAMP) copy-canonical ## Type-check TypeScript extension code with tsc
	@echo "==> Type-checking TypeScript code with tsc"
	cd $(EXT_DIR) && $(NPM) run compile

test-ext: compile ## Run VS Code extension integration tests
	@echo "==> Running VS Code extension tests"
	@if [[ "$$(uname -s)" == "Linux" && -z "$${DISPLAY:-}" ]]; then \
		if command -v xvfb-run >/dev/null 2>&1; then \
			echo "==> No DISPLAY detected; running VS Code tests under xvfb-run"; \
			cd $(EXT_DIR) && xvfb-run -a $(NPM) test; \
		else \
			echo "ERROR: DISPLAY is unset and xvfb-run is not available."; \
			echo "Install xvfb (provides xvfb-run) or set DISPLAY to run extension tests."; \
			exit 1; \
		fi; \
	else \
		cd $(EXT_DIR) && $(NPM) test; \
	fi

# Coverage targets (reports go to tmp/coverage/, which is gitignored)

COV_DIR := $(ROOT)tmp/coverage

coverage: coverage-py coverage-ext ## Generate coverage reports for Python and VS Code extension

coverage-py: $(UV_STAMP) ## Run Python tests with coverage (HTML + XML in tmp/coverage/python/)
	@echo "==> Running Python tests with coverage"
	@mkdir -p $(COV_DIR)/python
	cd $(ROOT) && $(UV) run --extra dev pytest tests/ -q \
		--ignore-glob='*/test_vm_*_test.py' \
		--cov --cov-report=html --cov-report=xml --cov-report=term-missing
	@echo ""
	@echo "Python coverage report: $(COV_DIR)/python/index.html"

coverage-ext: compile $(NPM_STAMP) ## Run VS Code extension tests with coverage (HTML in tmp/coverage/vscode/)
	@echo "==> Bundling extension with esbuild"
	cd $(EXT_DIR) && $(NPM) run bundle
	@echo "==> Running VS Code extension tests with coverage"
	@mkdir -p $(COV_DIR)/vscode $(COV_DIR)/.v8-coverage-vscode
	@if [[ "$$(uname -s)" == "Linux" && -z "$${DISPLAY:-}" ]]; then \
		if command -v xvfb-run >/dev/null 2>&1; then \
			echo "==> No DISPLAY detected; running under xvfb-run"; \
			cd $(EXT_DIR) && NODE_V8_COVERAGE=$(COV_DIR)/.v8-coverage-vscode \
				xvfb-run -a node ./out/test/runTest.js; \
		else \
			echo "ERROR: DISPLAY is unset and xvfb-run is not available."; \
			exit 1; \
		fi; \
	else \
		cd $(EXT_DIR) && NODE_V8_COVERAGE=$(COV_DIR)/.v8-coverage-vscode \
			node ./out/test/runTest.js; \
	fi
	cd $(EXT_DIR) && node scripts/coverage-report.cjs
	@echo ""
	@echo "VS Code extension coverage report: $(COV_DIR)/vscode/index.html"

# Phase targets for parallel prep-pr execution
_prep-pr-checks: lint-py typecheck-py lint-ts typecheck-ts
_prep-pr-tests: test-py test-opt
_prep-pr-smoke: smoke-zipapps smoke-vsix

NPROC := $(shell nproc 2>/dev/null || sysctl -n hw.ncpu 2>/dev/null || echo 4)

prep-pr: format ## Fast pre-PR gate (format + lint + typecheck + fast tests, no UI/smoke)
	@$(MAKE) -j $(NPROC) _prep-pr-checks _prep-pr-tests

test-slow: ## Slow tests: VS Code extension tests + smoke tests (zipapp + VSIX)
	@$(MAKE) -j $(NPROC) test-ext _prep-pr-smoke

test-opt: $(UV_STAMP) ## Run optimiser coverage tests (not part of standard CI)
	@echo "==> Running optimiser coverage tests"
	cd $(ROOT) && $(UV) run --extra dev pytest tests/test_optimiser_coverage.py tests/test_optimiser_vm_equivalence.py -v

test-fuzz: $(UV_STAMP) ## Run differential fuzz tests (FUZZ_ITERATIONS=N to control size)
	@echo "==> Running differential fuzz tests"
	cd $(ROOT) && $(UV) run --extra dev pytest tests/test_fuzz_differential.py -v

fuzz: $(UV_STAMP) ## Run a standalone fuzz campaign (N=iterations, SEED=base_seed)
	@echo "==> Running fuzz campaign ($(or $(N),1000) iterations)"
	cd $(ROOT) && $(UV) run --extra dev python -m tests.fuzz -n $(or $(N),1000) $(if $(SEED),--seed $(SEED)) -v

fuzz-cov: $(UV_STAMP) ## Coverage-guided fuzz campaign (N=iterations, SEED=base_seed)
	@echo "==> Running coverage-guided fuzz campaign ($(or $(N),500) iterations)"
	cd $(ROOT) && $(UV) run --extra dev python -m tests.fuzz -n $(or $(N),500) $(if $(SEED),--seed $(SEED)) --coverage-guided -v

_smoke-zipapp-ai: $(BUILD_INFO)
	@echo "==> Smoke-testing AI zipapp"
	python3 $(ROOT)scripts/build_zipapp.py ai --version $(VERSION) --output $(BUILD_DIR)/smoke-ai.pyz
	python3 $(BUILD_DIR)/smoke-ai.pyz context samples/for_screenshots/ai-scene.irul > /dev/null
	@rm -f $(BUILD_DIR)/smoke-ai.pyz

_smoke-zipapp-mcp: $(BUILD_INFO)
	@echo "==> Smoke-testing MCP zipapp"
	python3 $(ROOT)scripts/build_zipapp.py mcp --version $(VERSION) --output $(BUILD_DIR)/smoke-mcp.pyz
	python3 $(BUILD_DIR)/smoke-mcp.pyz --help > /dev/null
	@rm -f $(BUILD_DIR)/smoke-mcp.pyz

_smoke-zipapp-lsp: $(BUILD_INFO)
	@echo "==> Smoke-testing LSP zipapp"
	python3 $(ROOT)scripts/build_zipapp.py lsp --version $(VERSION) --output $(BUILD_DIR)/smoke-lsp.pyz
	python3 $(BUILD_DIR)/smoke-lsp.pyz --help > /dev/null
	@rm -f $(BUILD_DIR)/smoke-lsp.pyz

_smoke-zipapp-tcl: $(BUILD_INFO) $(KCS_DB)
	@echo "==> Smoke-testing unified Tcl zipapp"
	python3 $(ROOT)scripts/build_zipapp.py tcl --version $(VERSION) --output $(BUILD_DIR)/smoke-tcl.pyz
	python3 $(BUILD_DIR)/smoke-tcl.pyz --help > /dev/null
	python3 $(BUILD_DIR)/smoke-tcl.pyz format samples/for_screenshots/ai-scene.irul > /dev/null
	python3 $(BUILD_DIR)/smoke-tcl.pyz lint --source "set x 1" > /dev/null
	python3 $(BUILD_DIR)/smoke-tcl.pyz symbols samples/for_screenshots/ai-scene.irul --json > /dev/null
	python3 $(BUILD_DIR)/smoke-tcl.pyz callgraph samples/for_screenshots/ai-scene.irul --json > /dev/null
	python3 $(BUILD_DIR)/smoke-tcl.pyz event-info HTTP_REQUEST --json > /dev/null
	python3 $(BUILD_DIR)/smoke-tcl.pyz command-info HTTP::uri --dialect f5-irules --json > /dev/null
	python3 $(BUILD_DIR)/smoke-tcl.pyz convert samples/for_screenshots/ai-scene.irul --json > /dev/null
	python3 $(BUILD_DIR)/smoke-tcl.pyz highlight samples/for_screenshots/ai-scene.irul --no-colour > /dev/null
	python3 $(BUILD_DIR)/smoke-tcl.pyz diff samples/for_screenshots/ai-scene.irul samples/for_screenshots/ai-scene.irul --show ast --json > /dev/null
	python3 $(BUILD_DIR)/smoke-tcl.pyz help taint --dialect f5-irules > /dev/null
	ln -sfn smoke-tcl.pyz $(BUILD_DIR)/irule
	python3 $(BUILD_DIR)/irule help --help | tr '\n' ' ' | tr -s ' ' | grep -q "default: f5-irules"
	@rm -f $(BUILD_DIR)/smoke-tcl.pyz
	@rm -f $(BUILD_DIR)/irule

_smoke-zipapp-cli: $(BUILD_INFO)
	@echo "==> Smoke-testing CLI zipapp"
	python3 $(ROOT)scripts/build_zipapp.py cli --version $(VERSION) --output $(BUILD_DIR)/smoke-cli.pyz
	python3 $(BUILD_DIR)/smoke-cli.pyz --help > /dev/null
	@rm -f $(BUILD_DIR)/smoke-cli.pyz

smoke-zipapps: _smoke-zipapp-ai _smoke-zipapp-mcp _smoke-zipapp-lsp _smoke-zipapp-tcl _smoke-zipapp-cli ## Build and smoke-test all zipapps
	@echo "All zipapp smoke tests passed."

smoke-vsix: compile $(BUILD_INFO) ## Build and verify the VSIX packages without error
	@echo "==> Smoke-testing VSIX build"
	$(MAKE) package-vsix

# npm / TypeScript

npm-env: $(NPM_STAMP) ## Install/update npm dependencies

$(NPM_STAMP): $(EXT_DIR)/package.json
	@echo "==> Installing npm dependencies"
	cd $(EXT_DIR) && $(NPM) install
	@mkdir -p $(STAMP_DIR)
	@touch $@

# Copy canonical AI data into the extension source tree for esbuild

CANONICAL_DIR := $(EXT_DIR)/src/chat/canonical
CANONICAL_DIAG := $(CANONICAL_DIR)/diagnostics.json
CANONICAL_MANIFEST := $(CANONICAL_DIR)/manifest.json
CANONICAL_IRULES_MD := $(CANONICAL_DIR)/irules_system.md
CANONICAL_TCL_MD := $(CANONICAL_DIR)/tcl_system.md
CANONICAL_TK_MD := $(CANONICAL_DIR)/tk_system.md

copy-canonical: $(CANONICAL_DIAG) $(CANONICAL_MANIFEST) $(CANONICAL_IRULES_MD) $(CANONICAL_TCL_MD) $(CANONICAL_TK_MD)

$(CANONICAL_DIAG): $(ROOT)ai/shared/diagnostics.json
	@mkdir -p $(CANONICAL_DIR)
	@echo "==> Copying canonical diagnostics.json"
	cp $< $@

$(CANONICAL_MANIFEST): $(ROOT)ai/prompts/manifest.json
	@mkdir -p $(CANONICAL_DIR)
	@echo "==> Copying canonical manifest.json"
	cp $< $@

$(CANONICAL_IRULES_MD): $(ROOT)ai/prompts/irules_system.md
	@mkdir -p $(CANONICAL_DIR)
	@echo "==> Copying canonical irules_system.md"
	cp $< $@

$(CANONICAL_TCL_MD): $(ROOT)ai/prompts/tcl_system.md
	@mkdir -p $(CANONICAL_DIR)
	@echo "==> Copying canonical tcl_system.md"
	cp $< $@

$(CANONICAL_TK_MD): $(ROOT)ai/prompts/tk_system.md
	@mkdir -p $(CANONICAL_DIR)
	@echo "==> Copying canonical tk_system.md"
	cp $< $@

compile: $(OUT_DIR)/extension.js ## Compile the TypeScript extension

$(OUT_DIR)/extension.js: $(TS_SRCS) $(EXT_DIR)/tsconfig.json $(NPM_STAMP) $(CANONICAL_DIAG) $(CANONICAL_MANIFEST) $(CANONICAL_IRULES_MD) $(CANONICAL_TCL_MD) $(CANONICAL_TK_MD)
	@echo "==> Compiling TypeScript"
	cd $(EXT_DIR) && $(TSC) -p ./
	@mkdir -p $(OUT_DIR)/chat/canonical
	@cp $(CANONICAL_DIR)/* $(OUT_DIR)/chat/canonical/
	@cp $(ROOT)explorer/static/explorer-core.js $(OUT_DIR)/explorer-core.js

# Python environment

$(UV_STAMP): $(ROOT)pyproject.toml
	@echo "==> Syncing Python environment"
	cd $(ROOT) && $(UV) sync --extra dev
	@mkdir -p $(STAMP_DIR)
	@touch $@

# Build metadata

.FORCE:

build-info: $(BUILD_INFO) ## Generate build-info files

$(BUILD_INFO): .FORCE
	@printf '"""Generated at build time — do not edit."""\n\nVERSION: str = "%s"\nGIT_DESCRIBE: str = "%s"\nGIT_HASH: str = "%s"\nFULL_VERSION: str = "%s"\nBUILD_TIMESTAMP: str = "%s"\n' \
		"$(VERSION)" "$(GIT_DESCRIBE)" "$(GIT_HASH)" "$(FULL_VERSION)" "$(BUILD_TIMESTAMP)" > $@

$(BUILD_INFO_JSON): .FORCE
	@printf '{"version":"%s","git_describe":"%s","git_hash":"%s","full_version":"%s","build_timestamp":"%s"}\n' \
		"$(VERSION)" "$(GIT_DESCRIBE)" "$(GIT_HASH)" "$(FULL_VERSION)" "$(BUILD_TIMESTAMP)" > $@

# Generated editor catalogs

generate: $(UV_STAMP) ## Regenerate editor catalog files from the registry
	@echo "==> Generating editor catalogs"
	cd $(ROOT) && $(UV) run --extra dev python scripts/generate_catalogs.py

check-generated: $(UV_STAMP) ## Verify generated catalogs are up to date
	@echo "==> Checking generated catalogs are up to date"
	@TMPDIR=$$(mktemp -d) && \
	cd $(ROOT) && $(UV) run --extra dev python scripts/generate_catalogs.py --output-dir "$$TMPDIR" && \
	diff -q "$$TMPDIR/tcl_commands.json" editors/zed/src/generated/tcl_commands.json && \
	diff -q "$$TMPDIR/irule_events.json" editors/zed/src/generated/irule_events.json && \
	diff -q "$$TMPDIR/iruleEvents.json" editors/vscode/src/generated/iruleEvents.json && \
	rm -rf "$$TMPDIR" && \
	echo "Generated catalogs are up to date." || \
	(rm -rf "$$TMPDIR" && echo "ERROR: Generated catalogs are stale — run 'make generate'" >&2 && exit 1)

# Compiler Explorer (WASM GUI)

PYODIDE_VERSION  := 0.27.3
PYODIDE_DIR      := $(EXPLORER_STATIC)/pyodide
PYODIDE_TARBALL  := $(BUILD_DIR)/cache/pyodide-$(PYODIDE_VERSION).tar.bz2
PYODIDE_CDN      := https://github.com/pyodide/pyodide/releases/download/$(PYODIDE_VERSION)/pyodide-$(PYODIDE_VERSION).tar.bz2
MERMAID_VERSION  := 11
MERMAID_JS       := $(EXPLORER_STATIC)/mermaid.min.js
MERMAID_CDN      := https://cdn.jsdelivr.net/npm/mermaid@$(MERMAID_VERSION)/dist/mermaid.min.js

$(PYODIDE_TARBALL):
	@echo "==> Downloading Pyodide $(PYODIDE_VERSION)"
	@mkdir -p $(BUILD_DIR)/cache
	curl -fSL -o $@ $(PYODIDE_CDN)

$(PYODIDE_DIR)/pyodide.js: $(PYODIDE_TARBALL)
	@echo "==> Extracting Pyodide to $(PYODIDE_DIR)"
	@rm -rf $(PYODIDE_DIR)
	@mkdir -p $(PYODIDE_DIR)
	tar xjf $(PYODIDE_TARBALL) --strip-components=1 -C $(PYODIDE_DIR)
	@touch $@

$(MERMAID_JS):
	@echo "==> Downloading Mermaid.js $(MERMAID_VERSION)"
	curl -fSL -o $@ $(MERMAID_CDN)

explorer-build: $(UV_STAMP) $(PYODIDE_DIR)/pyodide.js $(MERMAID_JS) $(BUILD_INFO_JSON) ## Build the WASM compiler explorer (offline)
	@echo "==> Building wheel for Pyodide"
	cd $(ROOT) && $(UV) build --wheel --out-dir $(EXPLORER_STATIC)
	@echo "Built wheel:"
	@ls -lh $(EXPLORER_STATIC)/tcl_lsp-*.whl
	@echo "Pyodide: $(PYODIDE_DIR)"

compiler-explorer-gui: explorer-build ## Build and serve the static compiler explorer
	@echo "==> Serving compiler explorer at http://localhost:8080"
	cd $(EXPLORER_STATIC) && python3 -m http.server 8080

# CDN variant — lightweight build that loads Pyodide + Mermaid from CDN
EXPLORER_CDN_DIR := $(BUILD_DIR)/explorer-cdn
PYODIDE_CDN_BASE := https://cdn.jsdelivr.net/pyodide/v$(PYODIDE_VERSION)/full/
MERMAID_CDN_URL  := https://cdn.jsdelivr.net/npm/mermaid@$(MERMAID_VERSION)/dist/mermaid.min.js

explorer-build-cdn: $(UV_STAMP) $(BUILD_INFO_JSON) ## Build the CDN compiler explorer (no Pyodide download)
	@echo "==> Building CDN explorer"
	@rm -rf $(EXPLORER_CDN_DIR)
	@mkdir -p $(EXPLORER_CDN_DIR)
	cd $(ROOT) && $(UV) build --wheel --out-dir $(EXPLORER_CDN_DIR)
	cp $(BUILD_INFO_JSON) $(EXPLORER_CDN_DIR)/
	sed 's|<script src="mermaid.min.js"></script>|<script src="$(MERMAID_CDN_URL)"></script>|' \
		$(EXPLORER_STATIC)/index.html > $(EXPLORER_CDN_DIR)/index.html
	sed -e 's|// All assets are local.*|// Pyodide loaded from CDN.|' \
	    -e 's|const baseUrl = new URL.*|const baseUrl = new URL(".", self.location.href).href;|' \
	    -e 's|const pyodideUrl = baseUrl + "pyodide/";|const pyodideUrl = "$(PYODIDE_CDN_BASE)";|' \
		$(EXPLORER_STATIC)/worker.js > $(EXPLORER_CDN_DIR)/worker.js
	@echo "CDN explorer built in $(EXPLORER_CDN_DIR)"
	@ls -lh $(EXPLORER_CDN_DIR)/

# Zipapp targets

ZIPAPP_TCL     := $(BUILD_DIR)/tcl-$(VERSION).pyz
ZIPAPP_CLI     := $(BUILD_DIR)/tcl-lsp-explorer-cli-$(VERSION).pyz
ZIPAPP_GUI     := $(BUILD_DIR)/tcl-lsp-explorer-gui-$(VERSION).pyz
ZIPAPP_GUI_CDN := $(BUILD_DIR)/tcl-lsp-explorer-gui-cdn-$(VERSION).pyz
ZIPAPP_LSP     := $(BUILD_DIR)/tcl-lsp-server-$(VERSION).pyz
ZIPAPP_AI      := $(BUILD_DIR)/tcl-lsp-ai-$(VERSION).pyz
ZIPAPP_MCP     := $(BUILD_DIR)/tcl-lsp-mcp-server-$(VERSION).pyz
ZIPAPP_WASM    := $(BUILD_DIR)/tcl-wasm-compiler-$(VERSION).pyz
CLAUDE_SKILLS  := $(BUILD_DIR)/tcl-lsp-claude-skills-$(VERSION).zip
KCS_DB         := core/help/kcs_help.db

zipapps: zipapp-tcl zipapp-cli zipapp-gui zipapp-gui-cdn zipapp-lsp zipapp-ai zipapp-mcp zipapp-wasm ## Build all zipapps

zipapp-tcl: $(ZIPAPP_TCL) ## Build the unified Tcl tools zipapp

$(ZIPAPP_TCL): $(PY_SRCS) $(VM_SRCS) $(BUILD_INFO) $(KCS_DB)
	@echo "==> Building unified Tcl zipapp"
	python3 $(ROOT)scripts/build_zipapp.py tcl \
		--version $(VERSION) \
		--output $@

zipapp-cli: $(ZIPAPP_CLI) ## Build the CLI compiler explorer zipapp

$(ZIPAPP_CLI): $(PY_SRCS) $(BUILD_INFO)
	@echo "==> Building CLI zipapp"
	python3 $(ROOT)scripts/build_zipapp.py cli \
		--version $(VERSION) \
		--output $@

zipapp-gui: $(ZIPAPP_GUI) ## Build the standalone GUI zipapp (bundles Pyodide)

$(ZIPAPP_GUI): explorer-build $(BUILD_INFO_JSON)
	@echo "==> Building standalone GUI zipapp"
	python3 $(ROOT)scripts/build_zipapp.py gui \
		--version $(VERSION) \
		--output $@ \
		--static-dir $(EXPLORER_STATIC)

zipapp-gui-cdn: $(ZIPAPP_GUI_CDN) ## Build the CDN GUI zipapp (loads Pyodide from CDN)

$(ZIPAPP_GUI_CDN): explorer-build-cdn
	@echo "==> Building CDN GUI zipapp"
	python3 $(ROOT)scripts/build_zipapp.py gui-cdn \
		--version $(VERSION) \
		--output $@ \
		--static-dir $(EXPLORER_CDN_DIR)

zipapp-lsp: $(ZIPAPP_LSP) ## Build the LSP server zipapp

$(ZIPAPP_LSP): $(PY_SRCS) $(BUILD_INFO)
	@echo "==> Building LSP server zipapp"
	python3 $(ROOT)scripts/build_zipapp.py lsp \
		--version $(VERSION) \
		--output $@

zipapp-ai: $(ZIPAPP_AI) ## Build the AI analysis zipapp

$(ZIPAPP_AI): $(PY_SRCS) $(BUILD_INFO)
	@echo "==> Building AI analysis zipapp"
	python3 $(ROOT)scripts/build_zipapp.py ai \
		--version $(VERSION) \
		--output $@

zipapp-mcp: $(ZIPAPP_MCP) ## Build the MCP server zipapp

$(ZIPAPP_MCP): $(PY_SRCS) $(BUILD_INFO)
	@echo "==> Building MCP server zipapp"
	python3 $(ROOT)scripts/build_zipapp.py mcp \
		--version $(VERSION) \
		--output $@

zipapp-wasm: $(ZIPAPP_WASM) ## Build the WASM compiler zipapp

$(ZIPAPP_WASM): $(PY_SRCS) $(BUILD_INFO)
	@echo "==> Building WASM compiler zipapp"
	python3 $(ROOT)scripts/build_zipapp.py wasm \
		--version $(VERSION) \
		--output $@

claude-skills: $(CLAUDE_SKILLS) ## Build Claude Code skills release zip

$(CLAUDE_SKILLS): $(ZIPAPP_AI)
	@echo "==> Building Claude skills release zip"
	python3 $(ROOT)scripts/build_zipapp.py claude-skills \
		--version $(VERSION) \
		--output $@ \
		--ai-pyz $(ZIPAPP_AI)

package-vsix: compile $(VSIX_FILE) verify-vsix ## Package VSIX (skip lint/test, for CI)

# JetBrains plugin

JB_DIR     := $(ROOT)editors/jetbrains
JB_PLUGIN  := $(BUILD_DIR)/tcl-lsp-jetbrains-$(VERSION).zip

jetbrains: $(JB_PLUGIN) ## Build JetBrains plugin (.zip)

$(JB_PLUGIN): $(PY_SRCS) $(BUILD_INFO)
	@echo "==> Building JetBrains plugin"
	@# Inject version into gradle.properties
	python3 -c "import re,pathlib; p=pathlib.Path('$(JB_DIR)/gradle.properties'); p.write_text(re.sub(r'^pluginVersion=.*', 'pluginVersion=$(SEMVER_VERSION)', p.read_text(), flags=re.MULTILINE))"
	@# Copy shared resources into plugin resources
	mkdir -p $(JB_DIR)/src/main/resources/syntaxes
	cp $(EXT_DIR)/syntaxes/tcl.tmLanguage.json $(JB_DIR)/src/main/resources/syntaxes/
	@# Build LSP server zipapp into plugin resources
	python3 $(ROOT)scripts/build_zipapp.py lsp \
		--version $(VERSION) \
		--output $(JB_DIR)/src/main/resources/tcl-lsp-server.pyz
	@# Extract compiler explorer HTML from VS Code extension
	cd $(EXT_DIR) && node -e " \
		const {getWebviewHtml} = require('./out/compilerExplorerHtml'); \
		require('fs').writeFileSync('$(JB_DIR)/src/main/resources/compilerExplorer.html', getWebviewHtml()); \
	" 2>/dev/null || echo "(compiler explorer HTML extraction skipped — compile TS first)"
	@# Build plugin
	cd $(JB_DIR) && ./gradlew buildPlugin
	mkdir -p $(BUILD_DIR)
	cp $(JB_DIR)/build/distributions/tcl-lsp-jetbrains-$(SEMVER_VERSION).zip $(JB_PLUGIN)
	@echo ""
	@echo "Built: $(JB_PLUGIN)"
	@ls -lh $(JB_PLUGIN)

# Sublime Text package

ST_DIR      := $(ROOT)editors/sublime-text
ST_PACKAGE  := $(BUILD_DIR)/tcl-lsp-sublime-$(VERSION).sublime-package

sublime: $(ST_PACKAGE) ## Build Sublime Text package (.sublime-package)

$(ST_PACKAGE): $(PY_SRCS) $(BUILD_INFO)
	@echo "==> Building Sublime Text package"
	@rm -rf $(BUILD_DIR)/sublime-stage
	@mkdir -p $(BUILD_DIR)/sublime-stage
	cp -r $(ST_DIR)/. $(BUILD_DIR)/sublime-stage/
	find $(BUILD_DIR)/sublime-stage -name '__pycache__' -type d -exec rm -rf {} + 2>/dev/null || true
	find $(BUILD_DIR)/sublime-stage -name '.DS_Store' -delete 2>/dev/null || true
	rm -f $(BUILD_DIR)/sublime-stage/README.md
	@echo "==> Bundling raw server source files"
	@mkdir -p $(BUILD_DIR)/sublime-stage/server
	cp -r $(ROOT)lsp $(BUILD_DIR)/sublime-stage/server/lsp
	cp -r $(ROOT)core $(BUILD_DIR)/sublime-stage/server/core
	cp -r $(ROOT)explorer $(BUILD_DIR)/sublime-stage/server/explorer
	rm -rf $(BUILD_DIR)/sublime-stage/server/explorer/static
	find $(BUILD_DIR)/sublime-stage/server -name '__pycache__' -type d -exec rm -rf {} + 2>/dev/null || true
	find $(BUILD_DIR)/sublime-stage/server -name '*.pyc' -delete 2>/dev/null || true
	python3 -m pip install --target $(BUILD_DIR)/sublime-stage/server --no-user --quiet \
		"pygls>=2.0" "lsprotocol>=2024.0.0"
	find $(BUILD_DIR)/sublime-stage/server -name '*.dist-info' -type d -exec rm -rf {} + 2>/dev/null || true
	find $(BUILD_DIR)/sublime-stage/server -name '__pycache__' -type d -exec rm -rf {} + 2>/dev/null || true
	find $(BUILD_DIR)/sublime-stage/server -name '*.so' -delete 2>/dev/null || true
	find $(BUILD_DIR)/sublime-stage/server -name '*.pyd' -delete 2>/dev/null || true
	cp $(ROOT)scripts/zipapp_lsp_main.py $(BUILD_DIR)/sublime-stage/server/__main__.py
	cp $(LICENSE_SRC) $(BUILD_DIR)/sublime-stage/LICENSE.txt
	@echo "==> Packaging .sublime-package"
	cd $(BUILD_DIR)/sublime-stage && zip -r $(ST_PACKAGE) . -x '__pycache__/*'
	cp $(ST_PACKAGE) $(BUILD_DIR)/Tcl.sublime-package
	@echo ""
	@echo "Built: $(ST_PACKAGE)"
	@echo "       $(BUILD_DIR)/Tcl.sublime-package  (ready to install)"
	@ls -lh $(ST_PACKAGE)

# Zed extension

ZED_DIR     := $(ROOT)editors/zed
ZED_ARCHIVE := $(BUILD_DIR)/tcl-lsp-zed-$(VERSION).tar.gz
ZED_SRCS    := $(shell find $(ZED_DIR)/src -name '*.rs' 2>/dev/null)
ZED_BUNDLED := $(ZED_DIR)/bundled

zed: $(ZED_ARCHIVE) ## Build Zed extension archive (.tar.gz)

$(ZED_ARCHIVE): $(ZED_DIR)/Cargo.toml $(ZED_DIR)/extension.toml $(ZED_SRCS) $(PY_SRCS) $(BUILD_INFO)
	@echo "==> Building LSP + MCP server zipapps for bundling"
	@mkdir -p $(ZED_BUNDLED)
	python3 $(ROOT)scripts/build_zipapp.py lsp \
		--version $(VERSION) \
		--output $(ZED_BUNDLED)/tcl-lsp-server.pyz
	python3 $(ROOT)scripts/build_zipapp.py mcp \
		--version $(VERSION) \
		--output $(ZED_BUNDLED)/tcl-lsp-mcp-server.pyz
	@echo "==> Building Zed extension WASM (with bundled servers)"
	cd $(ZED_DIR) && TCL_LSP_BUNDLED_VERSION="$(VERSION)" cargo build --target wasm32-wasip2 --release
	@echo "==> Staging Zed extension archive"
	@rm -rf $(BUILD_DIR)/zed-stage
	@mkdir -p $(BUILD_DIR)/zed-stage
	cp $(ZED_DIR)/extension.toml $(BUILD_DIR)/zed-stage/
	python3 -c "import re,pathlib; p=pathlib.Path('$(BUILD_DIR)/zed-stage/extension.toml'); p.write_text(re.sub(r'^version = .*', 'version = \"$(SEMVER_VERSION)\"', p.read_text(), flags=re.MULTILINE))"
	cp $(ZED_DIR)/target/wasm32-wasip2/release/tcl_lsp_zed.wasm $(BUILD_DIR)/zed-stage/extension.wasm
	cp -r $(ZED_DIR)/languages $(BUILD_DIR)/zed-stage/
	cp -r $(ZED_DIR)/snippets $(BUILD_DIR)/zed-stage/
	@echo "==> Packaging Zed extension archive"
	mkdir -p $(BUILD_DIR)
	tar -czf $(ZED_ARCHIVE) -C $(BUILD_DIR)/zed-stage .
	@rm -rf $(ZED_BUNDLED)
	@echo ""
	@echo "Built: $(ZED_ARCHIVE)"
	@ls -lh $(ZED_ARCHIVE)

# Release

release: package-vsix zipapp-cli zipapp-tcl zipapp-gui-cdn zipapp-lsp claude-skills zipapp-mcp zipapp-wasm jetbrains sublime zed ## Build all release artifacts (parity with tagged CI release jobs)
	@echo ""
	@echo "Built release artifacts in $(BUILD_DIR)"

release-tag: ## Bump version, annotated-tag, and push (V=x.y.z)
	@bash $(ROOT)scripts/release.sh $(V)

# KCS help database

kcs-db: $(KCS_DB) ## Build the KCS help database from docs/kcs/features/

$(KCS_DB): $(wildcard docs/kcs/features/kcs-feature-*.md) $(wildcard docs/screenshots/*.png docs/screenshots/*.gif) scripts/build_kcs_db.py
	@echo "==> Building KCS help database"
	python3 $(ROOT)scripts/build_kcs_db.py --out $@

clean-kcs-db: ## Remove the generated KCS help database
	rm -f $(KCS_DB)

# Screenshots

screenshot: screenshots ## Alias for make screenshots

screenshots: compile ## Capture extension screenshots and build animated GIF (macOS)
	@echo "==> Building extension in screenshot mode"
	cd $(EXT_DIR) && $(NPM) run bundle:screenshots
	@echo "==> Running screenshot capture"
	TCL_LSP_SCREENSHOT_AUTO_BREW=$${TCL_LSP_SCREENSHOT_AUTO_BREW:-1} bash $(ROOT)scripts/screenshots.sh

clean-screenshots: ## Remove captured screenshots
	rm -rf $(SCREENSHOT_DIR)/*.png $(SCREENSHOT_DIR)/*.gif

# Cleanup

clean: ## Remove build artifacts
	rm -rf $(BUILD_DIR)
	rm -rf $(OUT_DIR)
	rm -f  $(BUILD_INFO)
	rm -f  $(BUILD_INFO_JSON)
	rm -rf $(PYODIDE_DIR)
	rm -f  $(EXPLORER_STATIC)/*.whl
	rm -f  $(MERMAID_JS)
	rm -rf $(ZED_DIR)/bundled
	find $(ROOT) -type d -name __pycache__ -exec rm -rf {} + 2>/dev/null || true

distclean: clean ## Remove build artifacts and node_modules
	rm -rf $(EXT_DIR)/node_modules
	rm -f  $(EXT_DIR)/package-lock.json
