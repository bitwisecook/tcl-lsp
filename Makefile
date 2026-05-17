# tcl-lsp — build, test, and package
#
# Targets:
#   make ci-fast       Fast CI gate (lint + typecheck + LSP e2e); mirrors GitHub PR job
#   make check-all     Pre-push gate: full lint + typecheck across all languages; writes tmp/check-all.stamp
#   make test-slow     Pre-PR gate: comprehensive (everything); writes tmp/check-all.stamp + tmp/test-slow.stamp
#   make install-hooks Install pre-push hook that enforces the check-all stamp
#   make prep-pr       Fast pre-PR gate (format + codegen + lint + typecheck + fast tests)
#   make vsix          Build the .vsix file (runs tests first)
#   make install       Build and install the .vsix into VS Code
#   make publish-vsix  Publish the .vsix to the VS Code Marketplace
#   make test          Run all tests (Python + VS Code extension)
#   make test-py       Run the Python test suite only (excludes VM tcltest tests)
#   make test-opt      Run optimiser coverage tests (not part of standard CI)
#   make test-fuzz     Run differential fuzz tests (pytest, FUZZ_ITERATIONS=N)
#   make fuzz          Run standalone fuzz campaign (N=iterations, SEED=base_seed)
#   make test-ext      Run VS Code extension integration tests
#   make test-emacs    Run headless eglot regression suite (Emacs 29+)
#   make lint-py       Lint Python code with Ruff
#   make format-py     Format and auto-fix Python code with Ruff
#   make format-ts     Format TypeScript extension code with Prettier
#   make typecheck-py-full Type-check all Python code with ty (broader coverage)
#   make typecheck-ts  Type-check TypeScript extension code with tsc
#   make npm-env       Install/update npm dependencies
#   make compile       Compile the TypeScript extension
#   make zipapp-tcl    Build the unified Tcl tools zipapp
#   make zipapp-cli    Build the CLI compiler explorer zipapp
#   make zipapp-f5     Build the F5 BIG-IP CLI zipapp
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
KCS_DB     := core/help/kcs_help.db

# Tools
UV       := uv
PYTHON   := $(UV) run python3
NPM      := npm
NODE_BIN := $(EXT_DIR)/node_modules/.bin
TSC      := $(NODE_BIN)/tsc
VSCE     := $(NODE_BIN)/vsce
OVSX     := $(NODE_BIN)/ovsx
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
# Wheel filename tracks pyproject.toml's [project].version (what `uv build`
# reads), not the git-describe VERSION above — so that worker.js can discover
# the wheel at runtime via build_info.json rather than hard-coding a number.
PYPROJECT_VERSION := $(shell grep -E '^version = ' $(ROOT)pyproject.toml | head -1 | sed 's/.*= *"//;s/".*//')
WHEEL_FILENAME   := tcl_lsp-$(PYPROJECT_VERSION)-py3-none-any.whl
BUILD_TIMESTAMP := $(shell date -u +%Y-%m-%dT%H:%M:%SZ)

# Derived paths
VSIX_FILE      := $(BUILD_DIR)/tcl-lsp-vscode-$(VERSION).vsix
LICENSE_SRC    := $(ROOT)LICENSE
README_SRC     := $(ROOT)README.md
SCREENSHOT_DIR := $(ROOT)docs/screenshots
SCREENSHOTS    := $(wildcard $(SCREENSHOT_DIR)/*.png $(SCREENSHOT_DIR)/*.gif)
VSCE_PUBLISHER := bitwisecook

# Build-info files (generated, gitignored)
BUILD_INFO      := $(ROOT)lsp/_build_info.py
BUILD_INFO_JSON := $(EXPLORER_STATIC)/build_info.json

# Zipapps
ZIPAPP_TCL     := $(BUILD_DIR)/tcl-$(VERSION).pyz
ZIPAPP_CLI     := $(BUILD_DIR)/tcl-lsp-explorer-cli-$(VERSION).pyz
ZIPAPP_F5      := $(BUILD_DIR)/f5-$(VERSION).pyz
ZIPAPP_GUI     := $(BUILD_DIR)/tcl-lsp-explorer-gui-$(VERSION).pyz
ZIPAPP_GUI_CDN := $(BUILD_DIR)/tcl-lsp-explorer-gui-cdn-$(VERSION).pyz
ZIPAPP_LSP     := $(BUILD_DIR)/tcl-lsp-server-$(VERSION).pyz
ZIPAPP_AI      := $(BUILD_DIR)/tcl-lsp-ai-$(VERSION).pyz
ZIPAPP_MCP     := $(BUILD_DIR)/tcl-lsp-mcp-server-$(VERSION).pyz
ZIPAPP_WASM    := $(BUILD_DIR)/tcl-wasm-compiler-$(VERSION).pyz
CLAUDE_SKILLS  := $(BUILD_DIR)/tcl-lsp-claude-skills-$(VERSION).zip

# Parallelism
NPROC := $(shell nproc 2>/dev/null || sysctl -n hw.ncpu 2>/dev/null || echo 4)

# Find all Python source files for dependency tracking
PY_SRCS  := $(shell find $(LSP_DIR) $(PYCORE_DIR) $(EXPLORER_DIR) -name '*.py' -not -path '*__pycache__*' -not -name '_build_info.py')
VM_SRCS  := $(shell find $(VM_DIR) -name '*.py' -not -path '*__pycache__*')
PY_TESTS := $(shell find $(TEST_DIR) -name '*.py' -not -path '*__pycache__*')
TS_SRCS  := $(shell find $(EXT_DIR)/src -name '*.ts' 2>/dev/null)

# Main targets

.PHONY: vsix verify-vsix install publish-vsix publish-openvsx publish-jetbrains publish-sublime publish-zed publish-all publish-verify test test-py test-slow test-opt test-ext test-emacs test-zig test-rust lint lint-py typecheck-py typecheck-py-full lint-ts format format-py format-ts typecheck-ts npm-env compile clean distclean help explorer-build explorer-build-cdn compiler-explorer-gui zipapp-tcl zipapp-cli zipapp-f5 zipapp-gui zipapp-gui-cdn zipapp-lsp zipapp-ai zipapp-mcp zipapp-wasm zipapps claude-skills package-vsix jetbrains sublime zed release release-tag build-info screenshot screenshots clean-screenshots prep-pr smoke-zipapps smoke-vsix copy-canonical coverage coverage-py coverage-ext generate check-generated ci-fast check-all check-zig check-rust install-hooks capture-bytecode-refs ensure-test-deps ensure-python-test-deps ensure-tcl-deps ensure-check-zig-deps ensure-test-zig-deps ensure-rust-deps ensure-emacs-deps ensure-vscode-test-deps .FORCE

help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## ' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-20s\033[0m %s\n", $$1, $$2}'

vsix: lint test compile verify-vsix ## Build the .vsix (tests must pass first)
install: package-vsix ## Build and install the .vsix into VS Code
	@echo "==> Installing VS Code extension"
	$(VSCODE) --install-extension $(VSIX_FILE) --force

publish-vsix: package-vsix ## Publish the .vsix to the VS Code Marketplace
	@echo "==> Verifying VS Code Marketplace credentials"
	@if [ -n "$$VSCE_PAT" ]; then \
		echo "    Using VSCE_PAT from environment (non-interactive)."; \
	elif ! $(VSCE) verify-pat $(VSCE_PUBLISHER) 2>/dev/null; then \
		echo "    No valid cached PAT for publisher '$(VSCE_PUBLISHER)' and"; \
		echo "    VSCE_PAT is not set."; \
		echo "    Launching interactive login (create a PAT at https://dev.azure.com if needed)..."; \
		$(VSCE) login $(VSCE_PUBLISHER); \
	fi
	@echo "==> Publishing $(VSIX_FILE) to VS Code Marketplace"
	cd $(STAGE_DIR) && $(VSCE) publish --packagePath $(VSIX_FILE)

publish-openvsx: package-vsix ## Publish the .vsix to Open VSX Registry (Cursor, Windsurf, VSCodium, code-server, Theia, Gitpod)
	@echo "==> Verifying Open VSX Registry credentials"
	@if [ -z "$$OVSX_PAT" ]; then \
		echo "error: OVSX_PAT environment variable is not set"; \
		echo "       Create a token at https://open-vsx.org/user-settings/tokens"; \
		echo "       The '$(VSCE_PUBLISHER)' namespace must be claimed at"; \
		echo "       https://open-vsx.org/user-settings/namespaces before"; \
		echo "       the first publish (one-time, via the Open VSX web UI)."; \
		exit 1; \
	fi
	@if [ ! -x $(OVSX) ]; then \
		echo "error: $(OVSX) not found. Run 'make npm-env' (or 'cd $(EXT_DIR) && npm install') first."; \
		exit 1; \
	fi
	@echo "==> Publishing $(VSIX_FILE) to Open VSX Registry"
	cd $(STAGE_DIR) && $(OVSX) publish $(VSIX_FILE) --pat $$OVSX_PAT

$(VSIX_FILE): $(OUT_DIR)/extension.js $(PY_SRCS) $(EXT_DIR)/package.json $(EXT_DIR)/.vscodeignore $(LICENSE_SRC) $(README_SRC) $(SCREENSHOTS) $(BUILD_INFO) $(ROOT)scripts/build_zipapp.py $(ROOT)scripts/zipapp_lsp_main.py $(ROOT)scripts/filter_readme.py
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
	@echo "==> Building LSP server zipapp"
	$(PYTHON) $(ROOT)scripts/build_zipapp.py lsp \
		--version $(VERSION) \
		--output $(STAGE_DIR)/tcl-lsp-server.pyz
	cp $(LICENSE_SRC) $(STAGE_DIR)/LICENSE.txt
	$(PYTHON) $(ROOT)scripts/filter_readme.py --editor "VS Code" $(README_SRC) -o $(STAGE_DIR)/README.md
	mkdir -p $(STAGE_DIR)/docs/screenshots
	cp $(SCREENSHOT_DIR)/*.png $(SCREENSHOT_DIR)/*.gif $(STAGE_DIR)/docs/screenshots/
	cp "$(ROOT)docs/Tcl LSP Logo-8bit-128.png" $(STAGE_DIR)/docs/icon.png
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

test: test-py test-ext test-zig ## Run all tests (Python + VS Code extension + Zig WASM runtime)

lint: lint-py typecheck-py lint-ts ## Run all lint and style checks

format: format-py format-ts ## Format Python and TypeScript code

test-py: $(UV_STAMP) ensure-python-test-deps ## Run the Python test suite (excludes VM tcltest and fuzz campaign tests)
	@echo "==> Running Python tests"
	cd $(ROOT) && $(UV) run --extra dev pytest tests/ -q -n 4 --ignore-glob='*/test_vm_*_test.py' --ignore=tests/test_optimiser_coverage.py --ignore=tests/test_optimiser_vm_equivalence.py

test-tclpkg: $(UV_STAMP) ensure-tcl-deps ## Run tclpkg package manager tests only
	@echo "==> Running tclpkg tests"
	cd $(ROOT) && $(UV) run --extra dev pytest tests/tclpkg/ tests/test_vm_safe_mode.py -v

test-tclpkg-tcl: ensure-tcl-deps ## Run pure-Tcl tclpkg tests (requires tclsh8.6+)
	@echo "==> Running pure-Tcl tclpkg tests"
	cd $(ROOT)/tclpkg-tcl && for t in tests/*_test.tcl; do tclsh8.6 "$$t" || exit 1; done

test-vm: $(UV_STAMP) ## Run VM tcltest suite (slow — runs Tcl test files through our VM); skip with SKIP_TEST_VM=1
	@set -eu; \
	if [ -n "$${SKIP_TEST_VM:-}" ]; then \
		echo "==> SKIP_TEST_VM set — skipping VM tcltest suite"; \
		exit 0; \
	fi; \
	echo "==> Running VM tcltest tests"; \
	cd $(ROOT) && $(UV) run --extra dev pytest tests/test_vm_*_test.py -q

test-tcl9: $(UV_STAMP) test-tcl9-samples ## Run Tcl 9 correctness harness + emit tmp/tcl9-report.json
	@echo "==> Running Tcl 9 correctness harness"
	@mkdir -p $(ROOT)tmp
	cd $(ROOT) && $(UV) run --extra dev pytest tests/external/run_tcl9_tests.py -q \
		--tcl9-report=tmp/tcl9-report.json

test-tcl9-samples: $(UV_STAMP) ## Run tcltest-free primitive smoke samples
	@echo "==> Running Tcl 9 smoke samples"
	cd $(ROOT) && $(UV) run --extra dev pytest tests/external/run_tcl9_samples.py -q

test-tcl9-full: $(UV_STAMP) ## Full Tcl 9 suite; requires upstream source (nightly)
	@echo "==> Running full Tcl 9 correctness harness"
	@mkdir -p $(ROOT)tmp
	cd $(ROOT) && $(UV) run --extra dev pytest tests/external/run_tcl9_tests.py -q \
		--tcl9-required --tcl9-report=tmp/tcl9-report-full.json

test-tcl9-vm-core: $(UV_STAMP) ## Run the Tcl 9 core slice regression gate (asserts no stem regresses against tests/baselines/tcl9-tcltest-vm/summary.json)
	@echo "==> Running Tcl 9 core slice regression gate (real init.tcl + tcltest.tcl)"
	@mkdir -p $(ROOT)tmp
	cd $(ROOT) && RUN_VM_TCL9_CORE=1 $(UV) run --extra dev pytest tests/test_vm_tcl9_core_baseline.py -q

refresh-tcl9-vm-core-baseline: $(UV_STAMP) ## Snapshot tests/baselines/tcl9-tcltest-vm/ from the current VM (use after a confirmed fix)
	@echo "==> Refreshing Tcl 9 core slice baseline"
	@mkdir -p $(ROOT)tmp
	cd $(ROOT) && $(UV) run --extra dev python scripts/dev/run_tcl9_vm_core.py --refresh-baseline

test-tcl9-wasm-core: $(UV_STAMP) ## Run the Tcl 9 core slice WASM regression gate (asserts no stem regresses against tests/baselines/tcl9-tcltest-wasm/summary.json — production ship gate)
	@echo "==> Running Tcl 9 core slice WASM regression gate (Zig runtime + WASM codegen, real init.tcl + tcltest.tcl)"
	@mkdir -p $(ROOT)tmp
	cd $(ROOT) && RUN_WASM_TCL9_CORE=1 $(UV) run --extra dev pytest tests/test_wasm_tcl9_core_baseline.py -q

refresh-tcl9-wasm-core-baseline: $(UV_STAMP) ## Snapshot tests/baselines/tcl9-tcltest-wasm/ from the current WASM runtime (use after a confirmed runtime/codegen fix)
	@echo "==> Refreshing Tcl 9 core slice WASM baseline"
	@mkdir -p $(ROOT)tmp
	cd $(ROOT) && $(UV) run --extra dev python scripts/dev/run_tcl9_wasm_core.py \
		--refresh-baseline --workers 4 --timeout 240 --run-timeout 180

check-tcl9-tcltest-io: $(UV_STAMP) ## Run the four upstream I/O tcltest suites against the baseline (issue #276)
	@echo "==> Running Tcl 9 I/O tcltest suites (chan / chanio / io / ioCmd) against baseline"
	@mkdir -p $(ROOT)tmp
	cd $(ROOT) && $(UV) run --extra dev pytest tests/external/run_tcl9_tests.py -q \
		--tcl9-required --tcl9-report=tmp/tcl9-report-io.json \
		-k "TestTcl9_chan or TestTcl9_chanio or TestTcl9_io or TestTcl9_ioCmd"

tcl9-triage: $(UV_STAMP) ## Refresh docs/kcs/kcs-tcl9-triage.md from tmp/tcl9-report.json
	@echo "==> Refreshing Tcl 9 triage table"
	cd $(ROOT) && $(UV) run python scripts/dev/tcl9_triage_report.py tmp/tcl9-report.json

lint-py: $(UV_STAMP) ## Lint Python code with Ruff (check, format, KCS docs)
	@echo "==> Checking KCS docs index links"
	cd $(ROOT) && $(UV) run python scripts/check_kcs_index_links.py
	@echo "==> Linting Python code with Ruff"
	cd $(ROOT) && $(UV) run --extra dev ruff check .
	@echo "==> Checking Python formatting with Ruff"
	cd $(ROOT) && $(UV) run --extra dev ruff format --check .

typecheck-py: $(UV_STAMP) $(BUILD_INFO) ## Type-check Python code with ty
	@echo "==> Type-checking Python code with ty"
	cd $(ROOT) && $(UV) run --extra dev ty check --exclude 'lsp/server.py' --exclude 'lsp/commands.py' lsp core explorer tclpkg tests scripts/dev/tcl_test_client.py

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

test-ext: compile ensure-vscode-test-deps ## Run VS Code extension integration tests
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

coverage-ext: compile $(NPM_STAMP) ensure-vscode-test-deps ## Run VS Code extension tests with coverage (HTML in tmp/coverage/vscode/)
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

check-wasm-parity: $(UV_STAMP) ## Check WASM command parity (registry vs Zig runtime) against tests/baselines/wasm_command_parity.json; skip with SKIP_CHECK_WASM_PARITY=1
	@set -eu; \
	if [ -n "$${SKIP_CHECK_WASM_PARITY:-}" ]; then \
		echo "==> SKIP_CHECK_WASM_PARITY set — skipping WASM command parity check"; \
		exit 0; \
	fi; \
	echo "==> Checking WASM command parity"; \
	cd $(ROOT) && $(UV) run python scripts/check_wasm_command_parity.py --check

snapshot-wasm-parity: $(UV_STAMP) ## Refresh tests/baselines/wasm_command_parity.json from current sources
	@echo "==> Snapshotting WASM command parity baseline"
	cd $(ROOT) && $(UV) run python scripts/check_wasm_command_parity.py --snapshot

# Phase targets for parallel prep-pr execution
_prep-pr-checks: lint-py typecheck-py lint-ts typecheck-ts check-editor-settings check-wasm-parity
_prep-pr-tests: test-py test-opt
_prep-pr-smoke: smoke-zipapps smoke-vsix

# Fast CI gate (target wall-clock < 20s) — what GitHub Actions runs on PRs.
# Covers: lint + typecheck + structural invariants + a tightly scoped pytest
# subset that exercises the LSP server end-to-end.  Everything else is the
# responsibility of `make test-slow` (run locally, gated by the pre-push hook).
# Use a fixed worker count (not NPROC) so we don't over-subscribe when this
# runs in parallel with ty in _ci-fast-checks.  2 workers keeps the LSP
# subset under 8s on its own while leaving CPU headroom for the typecheck.
_ci-fast-pytest: $(UV_STAMP)
	@echo "==> Running LSP end-to-end pytest subset"
	cd $(ROOT) && $(UV) run --extra dev pytest -q -n 2 \
		tests/test_server_commands.py \
		tests/test_server_config.py \
		tests/test_per_folder_config_e2e.py \
		tests/test_proc_lookup_lsp_features.py \
		tests/test_completion.py \
		tests/test_hover.py \
		tests/test_definition.py \
		tests/test_references.py \
		tests/test_diagnostics.py \
		tests/test_semantic_tokens.py \
		tests/test_code_actions.py \
		tests/test_document_symbols.py \
		tests/test_signature_help.py \
		tests/test_rename.py \
		-m "not slow"

# Python-only check phase for ci-fast (no TS lint/typecheck — those run in
# test-slow locally and on push:main in GitHub Actions).  Uses the full
# ruff/ty scope (lsp + core + explorer + tests + scripts) to match prep-pr.
_ci-fast-checks: lint-py typecheck-py check-editor-settings check-wasm-parity

ci-fast: $(UV_STAMP) $(BUILD_INFO) ## Fast CI gate — lint + typecheck + LSP e2e (mirrors GitHub Actions PR job)
	@$(MAKE) -j $(NPROC) _ci-fast-checks _ci-fast-pytest
	@mkdir -p $(ROOT)tmp
	@$(ROOT)scripts/worktree-fingerprint.sh > $(ROOT)tmp/ci-fast.stamp
	@echo "==> ci-fast: PASSED — stamped $(ROOT)tmp/ci-fast.stamp"

prep-pr: format codegen ## Fast pre-PR gate (format + codegen + lint + typecheck + fast tests, no UI/smoke)
	@$(MAKE) -j $(NPROC) _prep-pr-checks _prep-pr-tests

# Optional Rust test step.  Cargo tests run only if a workspace exists at the
# repo root (some branches add Rust code beyond the Zed extension); otherwise
# this is a no-op.  Set SKIP_TEST_RUST=1 to skip explicitly.
test-rust: ## Run Rust workspace tests if a top-level Cargo.toml is present (skip with SKIP_TEST_RUST=1)
	@set -eu; \
	if [ -n "$${SKIP_TEST_RUST:-}" ]; then \
		echo "==> SKIP_TEST_RUST set — skipping Rust tests"; \
		exit 0; \
	fi; \
	if [ ! -f "$(ROOT)Cargo.toml" ]; then \
		echo "==> No top-level Cargo.toml — skipping Rust tests"; \
		exit 0; \
	fi; \
	if ! command -v cargo >/dev/null 2>&1; then \
		echo "ERROR: 'cargo' not found on PATH (need Rust 1.95+)."; \
		echo "       Set SKIP_TEST_RUST=1 to skip this target."; \
		exit 1; \
	fi; \
	echo "==> Running Rust workspace tests"; \
	cd $(ROOT) && cargo test --workspace --all-features

## Pre-push gate: full lint + typecheck across every language (Python, TS,
## Zig, Rust).  This is what the pre-push hook checks via tmp/check-all.stamp.
## Tests are NOT included here — those are gated separately by test-slow
## before PR creation.

# Zig: format check + full compile (Zig has no separate type-checker; the
# build itself catches type errors).  Skip with SKIP_CHECK_ZIG=1.
check-zig: ensure-check-zig-deps ## Zig format check + compile (no tests); skip with SKIP_CHECK_ZIG=1
	@set -eu; \
	if [ -n "$${SKIP_CHECK_ZIG:-}" ]; then \
		echo "==> SKIP_CHECK_ZIG set — skipping Zig lint/typecheck"; \
		exit 0; \
	fi; \
	if ! command -v zig >/dev/null 2>&1; then \
		echo "ERROR: 'zig' not found on PATH (need Zig 0.16+)."; \
		echo "       Set SKIP_CHECK_ZIG=1 to skip."; \
		exit 1; \
	fi; \
	echo "==> Checking Zig formatting"; \
	cd $(ROOT)runtime/zig && zig fmt --check .; \
	echo "==> Compiling Zig (type-check via build)"; \
	cd $(ROOT)runtime/zig && zig build install

# Rust: cargo fmt --check + cargo clippy on the Zed extension (always
# present) and on a top-level Cargo.toml when it exists (Rust branches).
# Skip with SKIP_CHECK_RUST=1.
check-rust: ensure-rust-deps ## Rust fmt-check + clippy on Zed extension and top-level workspace if present
	@set -eu; \
	if [ -n "$${SKIP_CHECK_RUST:-}" ]; then \
		echo "==> SKIP_CHECK_RUST set — skipping Rust lint/typecheck"; \
		exit 0; \
	fi; \
	if [ -x "$$HOME/.cargo/bin/rustup" ]; then \
		export PATH="$$HOME/.cargo/bin:$$PATH"; \
	elif [ -f "$$HOME/.cargo/env" ]; then \
		. "$$HOME/.cargo/env"; \
	fi; \
	if ! command -v cargo >/dev/null 2>&1; then \
		echo "ERROR: 'cargo' not found on PATH (need Rust 1.95+)."; \
		echo "       Set SKIP_CHECK_RUST=1 to skip."; \
		exit 1; \
	fi; \
	if [ -f "$(ROOT)Cargo.toml" ]; then \
		echo "==> Checking top-level Rust workspace (fmt + clippy)"; \
		cd $(ROOT) && cargo fmt --all --check && \
			cargo clippy --workspace --all-targets -- -D warnings; \
	fi; \
	if [ -f "$(ZED_DIR)/Cargo.toml" ]; then \
		echo "==> Checking Zed extension (fmt + clippy --target wasm32-wasip2)"; \
		cd $(ZED_DIR) && cargo fmt --all --check && \
			cargo clippy --target wasm32-wasip2 --all-targets -- -D warnings; \
	fi

# All-languages lint + typecheck.  Mirrors GitHub Actions' pr-gate plus the
# extra languages CI doesn't cover (Zig, Rust, full TS).  On success writes
# tmp/check-all.stamp — the pre-push hook requires this stamp to match the
# current worktree before allowing a push.
check-all: $(UV_STAMP) $(BUILD_INFO) ## Full lint + typecheck (Python, TS, Zig, Rust); writes tmp/check-all.stamp on success
	@$(MAKE) -j $(NPROC) _prep-pr-checks check-zig check-rust
	@mkdir -p $(ROOT)tmp
	@$(ROOT)scripts/worktree-fingerprint.sh > $(ROOT)tmp/check-all.stamp
	@echo "==> check-all: PASSED — stamped $(ROOT)tmp/check-all.stamp"

# Comprehensive local gate — must pass before opening a PR.  On success,
# writes BOTH tmp/check-all.stamp and tmp/test-slow.stamp (since test-slow
# subsumes check-all by running prep-pr).
#
# Covers: prep-pr (format/codegen/lint/typecheck/test-py/test-opt/parity) +
# Zig & Rust lint/typecheck + VM tcltest + tclpkg + VS Code extension +
# Zig WASM runtime tests + Emacs eglot + zipapp & VSIX smokes + Rust
# workspace tests (when present).
test-slow: ## Comprehensive local gate (everything); writes tmp/check-all.stamp + tmp/test-slow.stamp on success
	@if [ "$${AUTO_INSTALL_DEPS:-0}" = "1" ]; then \
		echo "==> test-slow: AUTO_INSTALL_DEPS=1 — installing optional test deps"; \
		bash $(ROOT)scripts/dev/ensure-test-deps.sh; \
	else \
		echo "==> test-slow: dependency check (set AUTO_INSTALL_DEPS=1 to install missing tools)"; \
		bash $(ROOT)scripts/dev/ensure-test-deps.sh --check || \
			echo "    -> proceeding; the missing tools above will turn into pytest skips"; \
	fi
	@$(MAKE) capture-bytecode-refs
	@echo "==> test-slow: running prep-pr (format + codegen + lint + typecheck + fast tests)"
	@$(MAKE) prep-pr
	@echo "==> test-slow: running cross-language lint/typecheck + heavy suites in parallel"
	@$(MAKE) -j $(NPROC) check-zig check-rust test-vm test-tclpkg test-ext _prep-pr-smoke test-zig test-emacs test-rust
	@mkdir -p $(ROOT)tmp
	@$(ROOT)scripts/worktree-fingerprint.sh | tee $(ROOT)tmp/check-all.stamp > $(ROOT)tmp/test-slow.stamp
	@echo "==> test-slow: PASSED — stamped tmp/check-all.stamp + tmp/test-slow.stamp"

install-hooks: ## Install project git hooks (pre-push gate enforcing check-all stamp)
	@bash $(ROOT)scripts/install-hooks.sh

ensure-test-deps: ## Install optional test-slow host deps for the host platform
	@bash $(ROOT)scripts/dev/ensure-test-deps.sh

ensure-python-test-deps: ## Install host deps exercised by the full Python pytest suite
	@env \
		SKIP_RUST=1 \
		SKIP_WASMTIME=1 \
		SKIP_EMACS=1 \
		SKIP_XVFB=1 \
		bash $(ROOT)scripts/dev/ensure-test-deps.sh

ensure-tcl-deps: ## Install Tcl shells needed by Tcl/tclpkg tests and bytecode capture
	@env \
		SKIP_NODE=1 \
		SKIP_KOTLINC=1 \
		SKIP_RUST=1 \
		SKIP_ZIG=1 \
		SKIP_WASMTIME=1 \
		SKIP_BINARYEN=1 \
		SKIP_TCL_REGEX=1 \
		SKIP_EMACS=1 \
		SKIP_XVFB=1 \
		SKIP_TSHARK=1 \
		SKIP_OPENSSL=1 \
		SKIP_PING=1 \
		SKIP_RGXG=1 \
		SKIP_TCLLIB=1 \
		bash $(ROOT)scripts/dev/ensure-test-deps.sh

ensure-check-zig-deps: ## Install Zig build deps needed by check-zig
	@if [ -n "$${SKIP_CHECK_ZIG:-}" ]; then \
		echo "==> Zig dependency install skipped"; \
	else \
		env \
			SKIP_TCLSH=1 \
			SKIP_NODE=1 \
			SKIP_KOTLINC=1 \
			SKIP_RUST=1 \
			SKIP_WASMTIME=1 \
			SKIP_BINARYEN=1 \
			SKIP_EMACS=1 \
			SKIP_XVFB=1 \
			SKIP_TSHARK=1 \
			SKIP_OPENSSL=1 \
			SKIP_PING=1 \
			SKIP_RGXG=1 \
			SKIP_TCLLIB=1 \
			bash $(ROOT)scripts/dev/ensure-test-deps.sh; \
	fi

ensure-test-zig-deps: ## Install Zig + Wasmtime CLI deps needed by test-zig
	@if [ -n "$${SKIP_TEST_ZIG:-}" ]; then \
		echo "==> Zig test dependency install skipped"; \
	else \
		env \
			SKIP_TCLSH=1 \
			SKIP_NODE=1 \
			SKIP_KOTLINC=1 \
			SKIP_RUST=1 \
			SKIP_BINARYEN=1 \
			SKIP_EMACS=1 \
			SKIP_XVFB=1 \
			SKIP_TSHARK=1 \
			SKIP_OPENSSL=1 \
			SKIP_PING=1 \
			SKIP_RGXG=1 \
			SKIP_TCLLIB=1 \
			bash $(ROOT)scripts/dev/ensure-test-deps.sh; \
	fi

ensure-rust-deps: ## Install Rust/rustup + wasm32-wasip2 target needed by check-rust
	@if [ -n "$${SKIP_CHECK_RUST:-}" ] || [ -n "$${SKIP_RUST:-}" ]; then \
		echo "==> Rust dependency install skipped"; \
	else \
		env \
			SKIP_TCLSH=1 \
			SKIP_NODE=1 \
			SKIP_KOTLINC=1 \
			SKIP_ZIG=1 \
			SKIP_WASMTIME=1 \
			SKIP_BINARYEN=1 \
			SKIP_TCL_REGEX=1 \
			SKIP_EMACS=1 \
			SKIP_XVFB=1 \
			SKIP_TSHARK=1 \
			SKIP_OPENSSL=1 \
			SKIP_PING=1 \
			SKIP_RGXG=1 \
			SKIP_TCLLIB=1 \
			bash $(ROOT)scripts/dev/ensure-test-deps.sh; \
	fi

ensure-emacs-deps: ## Install Emacs needed by test-emacs
	@if [ -n "$${SKIP_TEST_EMACS:-}" ]; then \
		echo "==> Emacs dependency install skipped"; \
	else \
		env \
			SKIP_TCLSH=1 \
			SKIP_NODE=1 \
			SKIP_KOTLINC=1 \
			SKIP_RUST=1 \
			SKIP_ZIG=1 \
			SKIP_WASMTIME=1 \
			SKIP_BINARYEN=1 \
			SKIP_TCL_REGEX=1 \
			SKIP_XVFB=1 \
			SKIP_TSHARK=1 \
			SKIP_OPENSSL=1 \
			SKIP_PING=1 \
			SKIP_RGXG=1 \
			SKIP_TCLLIB=1 \
			bash $(ROOT)scripts/dev/ensure-test-deps.sh; \
	fi

ensure-vscode-test-deps: ## Install xvfb for Linux headless VS Code extension tests
	@env \
		SKIP_TCLSH=1 \
		SKIP_NODE=1 \
		SKIP_KOTLINC=1 \
		SKIP_RUST=1 \
		SKIP_ZIG=1 \
		SKIP_WASMTIME=1 \
		SKIP_BINARYEN=1 \
		SKIP_TCL_REGEX=1 \
		SKIP_EMACS=1 \
		SKIP_TSHARK=1 \
		SKIP_OPENSSL=1 \
		SKIP_PING=1 \
		SKIP_RGXG=1 \
		SKIP_TCLLIB=1 \
		bash $(ROOT)scripts/dev/ensure-test-deps.sh

capture-bytecode-refs: ensure-tcl-deps ## Capture missing tests/bytecode_reference/<ver>/*.disasm files using local tclsh
	@set -eu; \
	missing=0; \
	for snippet in $(ROOT)tests/bytecode_snippets/*.tcl; do \
		stem=$$(basename $$snippet .tcl); \
		[ -f "$(ROOT)tests/bytecode_reference/9.0/$${stem}.disasm" ] || missing=$$((missing+1)); \
	done; \
	if [ $$missing -eq 0 ]; then \
		echo "==> capture-bytecode-refs: 9.0 reference disasm complete (no action)"; \
		exit 0; \
	fi; \
	if ! command -v tclsh9.0 >/dev/null 2>&1; then \
		echo "==> capture-bytecode-refs: $$missing reference disasm files missing, but tclsh9.0 isn't on PATH."; \
		echo "    Run 'AUTO_INSTALL_DEPS=1 make ensure-test-deps' (or install tclsh9.0 manually), then re-run this target."; \
		echo "    Skipping for now — affected snippets will pytest-skip with 'no reference file: ...'."; \
		exit 0; \
	fi; \
	echo "==> capture-bytecode-refs: $$missing missing — running scripts/capture_reference_bytecode.sh"; \
	bash $(ROOT)scripts/capture_reference_bytecode.sh

test-emacs: ensure-emacs-deps ## Run headless eglot regression suite for tcl-lsp (issue #333 + delta correctness)
	@set -eu; \
	if [ -n "$${SKIP_TEST_EMACS:-}" ]; then \
		echo "==> SKIP_TEST_EMACS set — skipping Emacs eglot tests"; \
		exit 0; \
	fi; \
	echo "==> Running Emacs eglot regression suite"; \
	if ! command -v emacs >/dev/null 2>&1; then \
		echo "ERROR: 'emacs' not found on PATH (need Emacs 29+; install with 'sudo apt-get install -y emacs-nox' on Debian/Ubuntu)."; \
		echo "       Set SKIP_TEST_EMACS=1 to skip this target."; \
		exit 1; \
	fi; \
	bash $(ROOT)scripts/eglot_test/run.sh

test-zig: ensure-test-zig-deps ## Run Zig WASM runtime unit tests (test_*.zig under runtime/zig/) — set SKIP_TEST_ZIG=1 to skip
	@set -eu; \
	if [ -n "$${SKIP_TEST_ZIG:-}" ]; then \
		echo "==> SKIP_TEST_ZIG set — skipping Zig WASM runtime tests"; \
		exit 0; \
	fi; \
	echo "==> Running Zig WASM runtime tests"; \
	if ! command -v zig >/dev/null 2>&1; then \
		echo "ERROR: 'zig' not found on PATH (need Zig 0.16+; the SessionStart hook installs it at /opt/zig-0.16.0)."; \
		echo "       Set SKIP_TEST_ZIG=1 to skip this target."; \
		exit 1; \
	fi; \
	if ! command -v wasmtime >/dev/null 2>&1; then \
		echo "ERROR: 'wasmtime' not found on PATH — required because the runtime tests are wasm32-wasi binaries."; \
		echo "       The SessionStart hook installs it at /opt/wasmtime-43.0.1; outside Claude Code, install from https://wasmtime.dev/."; \
		echo "       Set SKIP_TEST_ZIG=1 to skip this target."; \
		exit 1; \
	fi; \
	cd $(ROOT)runtime/zig && zig build test

test-opt: $(UV_STAMP) ## Run optimiser coverage tests (not part of standard CI)
	@echo "==> Running optimiser coverage tests"
	cd $(ROOT) && $(UV) run --extra dev pytest tests/test_optimiser_coverage.py tests/test_optimiser_vm_equivalence.py -v

test-fuzz: $(UV_STAMP) ## Run differential fuzz tests (FUZZ_ITERATIONS=N to control size)
	@echo "==> Running differential fuzz tests"
	cd $(ROOT) && $(UV) run --extra dev pytest fuzzing/tests/test_fuzz_differential.py -v

fuzz: $(UV_STAMP) ## Run a standalone fuzz campaign (N=iterations, SEED=base_seed)
	@echo "==> Running fuzz campaign ($(or $(N),1000) iterations)"
	cd $(ROOT) && $(UV) run --extra dev python -m fuzzing -n $(or $(N),1000) $(if $(SEED),--seed $(SEED)) -v

fuzz-cov: $(UV_STAMP) ## Coverage-guided fuzz campaign (N=iterations, SEED=base_seed)
	@echo "==> Running coverage-guided fuzz campaign ($(or $(N),500) iterations)"
	cd $(ROOT) && $(UV) run --extra dev python -m fuzzing -n $(or $(N),500) $(if $(SEED),--seed $(SEED)) --coverage-guided -v

_smoke-zipapp-ai: $(BUILD_INFO)
	@echo "==> Smoke-testing AI zipapp"
	$(PYTHON) $(ROOT)scripts/build_zipapp.py ai --version $(VERSION) --output $(BUILD_DIR)/smoke-ai.pyz
	$(PYTHON) $(BUILD_DIR)/smoke-ai.pyz context samples/for_screenshots/ai-scene.irul > /dev/null
	@rm -f $(BUILD_DIR)/smoke-ai.pyz

_smoke-zipapp-mcp: $(BUILD_INFO)
	@echo "==> Smoke-testing MCP zipapp"
	$(PYTHON) $(ROOT)scripts/build_zipapp.py mcp --version $(VERSION) --output $(BUILD_DIR)/smoke-mcp.pyz
	$(PYTHON) $(BUILD_DIR)/smoke-mcp.pyz --help > /dev/null
	@rm -f $(BUILD_DIR)/smoke-mcp.pyz

_smoke-zipapp-lsp: $(BUILD_INFO)
	@echo "==> Smoke-testing LSP zipapp"
	$(PYTHON) $(ROOT)scripts/build_zipapp.py lsp --version $(VERSION) --output $(BUILD_DIR)/smoke-lsp.pyz
	$(PYTHON) $(BUILD_DIR)/smoke-lsp.pyz --help > /dev/null
	@rm -f $(BUILD_DIR)/smoke-lsp.pyz

_smoke-zipapp-tcl: $(BUILD_INFO) $(KCS_DB)
	@echo "==> Smoke-testing unified Tcl zipapp"
	$(PYTHON) $(ROOT)scripts/build_zipapp.py tcl --version $(VERSION) --output $(BUILD_DIR)/smoke-tcl.pyz
	$(PYTHON) $(BUILD_DIR)/smoke-tcl.pyz --help > /dev/null
	$(PYTHON) $(BUILD_DIR)/smoke-tcl.pyz format samples/for_screenshots/ai-scene.irul > /dev/null
	$(PYTHON) $(BUILD_DIR)/smoke-tcl.pyz lint --source "set x 1" > /dev/null
	$(PYTHON) $(BUILD_DIR)/smoke-tcl.pyz symbols samples/for_screenshots/ai-scene.irul --json > /dev/null
	$(PYTHON) $(BUILD_DIR)/smoke-tcl.pyz callgraph samples/for_screenshots/ai-scene.irul --json > /dev/null
	$(PYTHON) $(BUILD_DIR)/smoke-tcl.pyz command-info HTTP::uri --dialect f5-irules --json > /dev/null
	$(PYTHON) $(BUILD_DIR)/smoke-tcl.pyz find-legacy samples/for_screenshots/ai-scene.irul --json > /dev/null
	$(PYTHON) $(BUILD_DIR)/smoke-tcl.pyz highlight samples/for_screenshots/ai-scene.irul --no-colour > /dev/null
	$(PYTHON) $(BUILD_DIR)/smoke-tcl.pyz diff samples/for_screenshots/ai-scene.irul samples/for_screenshots/ai-scene.irul --show ast --json > /dev/null
	$(PYTHON) $(BUILD_DIR)/smoke-tcl.pyz help taint --dialect f5-irules > /dev/null
	# Completion scripts are bundled and printable from inside the zipapp.
	$(PYTHON) $(BUILD_DIR)/smoke-tcl.pyz completion bash > $(BUILD_DIR)/smoke-tcl.bash
	$(PYTHON) $(BUILD_DIR)/smoke-tcl.pyz completion fish > $(BUILD_DIR)/smoke-tcl.fish
	$(PYTHON) $(BUILD_DIR)/smoke-tcl.pyz completion zsh  > $(BUILD_DIR)/smoke-tcl.zsh
	bash -n $(BUILD_DIR)/smoke-tcl.bash
	@rm -f $(BUILD_DIR)/smoke-tcl.pyz $(BUILD_DIR)/smoke-tcl.bash $(BUILD_DIR)/smoke-tcl.fish $(BUILD_DIR)/smoke-tcl.zsh

_smoke-zipapp-cli: $(BUILD_INFO)
	@echo "==> Smoke-testing CLI zipapp"
	$(PYTHON) $(ROOT)scripts/build_zipapp.py cli --version $(VERSION) --output $(BUILD_DIR)/smoke-cli.pyz
	$(PYTHON) $(BUILD_DIR)/smoke-cli.pyz --help > /dev/null
	@rm -f $(BUILD_DIR)/smoke-cli.pyz

_smoke-zipapp-f5: $(BUILD_INFO)
	@echo "==> Smoke-testing F5 BIG-IP zipapp"
	$(PYTHON) $(ROOT)scripts/build_zipapp.py f5 --version $(VERSION) --output $(BUILD_DIR)/smoke-f5.pyz
	$(PYTHON) $(BUILD_DIR)/smoke-f5.pyz --help > /dev/null
	$(PYTHON) $(BUILD_DIR)/smoke-f5.pyz cleanup samples/bigip/bigip.conf > /dev/null
	$(PYTHON) $(BUILD_DIR)/smoke-f5.pyz cleanup --json samples/bigip/bigip.conf > /dev/null
	# `f5 irule` sub-verbs (event-order, event-info).
	$(PYTHON) $(BUILD_DIR)/smoke-f5.pyz irule event-info HTTP_REQUEST --json > /dev/null
	$(PYTHON) $(BUILD_DIR)/smoke-f5.pyz irule event-order --source 'when HTTP_REQUEST { return }' --json > /dev/null
	# Completion scripts are bundled and printable from inside the zipapp.
	$(PYTHON) $(BUILD_DIR)/smoke-f5.pyz completion bash > $(BUILD_DIR)/smoke-f5.bash
	$(PYTHON) $(BUILD_DIR)/smoke-f5.pyz completion fish > $(BUILD_DIR)/smoke-f5.fish
	$(PYTHON) $(BUILD_DIR)/smoke-f5.pyz completion zsh  > $(BUILD_DIR)/smoke-f5.zsh
	bash -n $(BUILD_DIR)/smoke-f5.bash
	@rm -f $(BUILD_DIR)/smoke-f5.pyz $(BUILD_DIR)/smoke-f5.bash $(BUILD_DIR)/smoke-f5.fish $(BUILD_DIR)/smoke-f5.zsh

smoke-zipapps: _smoke-zipapp-ai _smoke-zipapp-mcp _smoke-zipapp-lsp _smoke-zipapp-tcl _smoke-zipapp-cli _smoke-zipapp-f5 ## Build and smoke-test all zipapps
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
	@printf '{"version":"%s","git_describe":"%s","git_hash":"%s","full_version":"%s","build_timestamp":"%s","wheel_filename":"%s"}\n' \
		"$(VERSION)" "$(GIT_DESCRIBE)" "$(GIT_HASH)" "$(FULL_VERSION)" "$(BUILD_TIMESTAMP)" "$(WHEEL_FILENAME)" > $@

# Generated editor catalogs
#
# Depends on: the generator script + command registry specs.
REGISTRY_SRCS := $(shell find $(PYCORE_DIR)/commands/registry -name '*.py' -not -path '*__pycache__*')
_CATALOG_DEPS := $(UV_STAMP) scripts/generate_catalogs.py $(REGISTRY_SRCS)

editors/zed/src/generated/tcl_commands.json editors/zed/src/generated/irule_events.json editors/vscode/src/generated/iruleEvents.json &: $(_CATALOG_DEPS)
	@echo "==> Generating editor catalogs"
	cd $(ROOT) && $(UV) run --extra dev python scripts/generate_catalogs.py

core/bigip/_port_names_table.py: scripts/generate_port_names.py core/bigip/data/scf_port_names.csv $(UV_STAMP)
	@echo "==> Generating BIG-IP port-name table"
	cd $(ROOT) && $(UV) run --extra dev python scripts/generate_port_names.py

generate: editors/zed/src/generated/tcl_commands.json core/bigip/_port_names_table.py ## Regenerate editor catalog files from the registry

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
	@echo "==> Checking generated BIG-IP port-name table is up to date"
	@cd $(ROOT) && $(UV) run --extra dev python scripts/generate_port_names.py --check

# Generated editor settings from code registry
#
# Depends on: the generator script + diagnostic/optimisation code
# definitions + formatter config + Jinja2 templates.
CODES_SRCS    := $(shell find $(PYCORE_DIR)/common -name 'codes*.py' -not -path '*__pycache__*')
OPTIMISER_SRCS := $(shell find $(PYCORE_DIR)/compiler/optimiser -name '*.py' -not -path '*__pycache__*')
CHECKS_SRCS   := $(shell find $(PYCORE_DIR)/analysis/checks -name '*.py' -not -path '*__pycache__*')
ANALYSER_SRCS := $(shell find $(PYCORE_DIR)/analysis/_analyser -name '*.py' -not -path '*__pycache__*')
SETTINGS_SRCS := $(CODES_SRCS) $(OPTIMISER_SRCS) $(CHECKS_SRCS) $(ANALYSER_SRCS) \
	$(PYCORE_DIR)/formatting/config.py \
	$(PYCORE_DIR)/common/optimisation_profiles.py \
	$(PYCORE_DIR)/analysis/irules_checks.py \
	$(PYCORE_DIR)/compiler/compiler_checks.py \
	$(PYCORE_DIR)/compiler/gvn.py \
	$(PYCORE_DIR)/compiler/shimmer.py
SETTINGS_J2   := $(wildcard docs/generated/*.j2 editors/vscode/src/generated/*.j2 editors/jetbrains/src/main/kotlin/com/tcllsp/jetbrains/settings/generated/*.j2 ai/prompts/*.j2 ai/claude/skills/*/*.j2)
_SETTINGS_DEPS := $(UV_STAMP) scripts/generate_editor_settings.py $(SETTINGS_SRCS) $(SETTINGS_J2)

editors/vscode/src/generated/diagnosticCatalog.ts: $(_SETTINGS_DEPS)
	@echo "==> Generating editor settings from code registry"
	cd $(ROOT) && $(UV) run --extra dev python scripts/generate_editor_settings.py

gen-editor-settings: editors/vscode/src/generated/diagnosticCatalog.ts ## Regenerate editor diagnostic/optimiser settings from code registry

check-editor-settings: $(UV_STAMP) ## Verify editor settings match code registry
	@echo "==> Checking editor settings are up to date"
	cd $(ROOT) && $(UV) run --extra dev python scripts/generate_editor_settings.py --check

# Unified codegen — regenerate ALL generated files from registries

codegen: generate gen-editor-settings ## Regenerate ALL generated files (catalogs + editor settings + AI prompts)

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
	cd $(EXPLORER_STATIC) && $(PYTHON) -m http.server 8080

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
	cp $(EXPLORER_STATIC)/explorer-core.js $(EXPLORER_CDN_DIR)/
	sed 's|<script src="mermaid.min.js"></script>|<script src="$(MERMAID_CDN_URL)"></script>|' \
		$(EXPLORER_STATIC)/index.html > $(EXPLORER_CDN_DIR)/index.html
	sed -e 's|// All assets are local.*|// Pyodide loaded from CDN.|' \
	    -e 's|const baseUrl = new URL.*|const baseUrl = new URL(".", self.location.href).href;|' \
	    -e 's|const pyodideUrl = baseUrl + "pyodide/";|const pyodideUrl = "$(PYODIDE_CDN_BASE)";|' \
		$(EXPLORER_STATIC)/worker.js > $(EXPLORER_CDN_DIR)/worker.js
	@echo "CDN explorer built in $(EXPLORER_CDN_DIR)"
	@ls -lh $(EXPLORER_CDN_DIR)/

# Zipapp targets

zipapps: zipapp-tcl zipapp-cli zipapp-f5 zipapp-gui zipapp-gui-cdn zipapp-lsp zipapp-ai zipapp-mcp zipapp-wasm ## Build all zipapps

zipapp-tcl: $(ZIPAPP_TCL) ## Build the unified Tcl tools zipapp

$(ZIPAPP_TCL): $(PY_SRCS) $(VM_SRCS) $(BUILD_INFO) $(KCS_DB)
	@echo "==> Building unified Tcl zipapp"
	$(PYTHON) $(ROOT)scripts/build_zipapp.py tcl \
		--version $(VERSION) \
		--output $@

zipapp-cli: $(ZIPAPP_CLI) ## Build the CLI compiler explorer zipapp

$(ZIPAPP_CLI): $(PY_SRCS) $(BUILD_INFO)
	@echo "==> Building CLI zipapp"
	$(PYTHON) $(ROOT)scripts/build_zipapp.py cli \
		--version $(VERSION) \
		--output $@

zipapp-f5: $(ZIPAPP_F5) ## Build the F5 BIG-IP CLI zipapp

$(ZIPAPP_F5): $(PY_SRCS) $(BUILD_INFO)
	@echo "==> Building F5 BIG-IP CLI zipapp"
	$(PYTHON) $(ROOT)scripts/build_zipapp.py f5 \
		--version $(VERSION) \
		--output $@

zipapp-gui: $(ZIPAPP_GUI) ## Build the standalone GUI zipapp (bundles Pyodide)

$(ZIPAPP_GUI): explorer-build $(BUILD_INFO_JSON)
	@echo "==> Building standalone GUI zipapp"
	$(PYTHON) $(ROOT)scripts/build_zipapp.py gui \
		--version $(VERSION) \
		--output $@ \
		--static-dir $(EXPLORER_STATIC)

zipapp-gui-cdn: $(ZIPAPP_GUI_CDN) ## Build the CDN GUI zipapp (loads Pyodide from CDN)

$(ZIPAPP_GUI_CDN): explorer-build-cdn
	@echo "==> Building CDN GUI zipapp"
	$(PYTHON) $(ROOT)scripts/build_zipapp.py gui-cdn \
		--version $(VERSION) \
		--output $@ \
		--static-dir $(EXPLORER_CDN_DIR)

zipapp-lsp: $(ZIPAPP_LSP) ## Build the LSP server zipapp

$(ZIPAPP_LSP): $(PY_SRCS) $(BUILD_INFO)
	@echo "==> Building LSP server zipapp"
	$(PYTHON) $(ROOT)scripts/build_zipapp.py lsp \
		--version $(VERSION) \
		--output $@

zipapp-ai: $(ZIPAPP_AI) ## Build the AI analysis zipapp

$(ZIPAPP_AI): $(PY_SRCS) $(BUILD_INFO)
	@echo "==> Building AI analysis zipapp"
	$(PYTHON) $(ROOT)scripts/build_zipapp.py ai \
		--version $(VERSION) \
		--output $@

zipapp-mcp: $(ZIPAPP_MCP) ## Build the MCP server zipapp

$(ZIPAPP_MCP): $(PY_SRCS) $(BUILD_INFO)
	@echo "==> Building MCP server zipapp"
	$(PYTHON) $(ROOT)scripts/build_zipapp.py mcp \
		--version $(VERSION) \
		--output $@

zipapp-wasm: $(ZIPAPP_WASM) ## Build the WASM compiler zipapp

$(ZIPAPP_WASM): $(PY_SRCS) $(BUILD_INFO)
	@echo "==> Building WASM compiler zipapp"
	$(PYTHON) $(ROOT)scripts/build_zipapp.py wasm \
		--version $(VERSION) \
		--output $@

claude-skills: $(CLAUDE_SKILLS) ## Build Claude Code skills release zip

$(CLAUDE_SKILLS): $(ZIPAPP_AI)
	@echo "==> Building Claude skills release zip"
	$(PYTHON) $(ROOT)scripts/build_zipapp.py claude-skills \
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
	$(PYTHON) -c "import re,pathlib; p=pathlib.Path('$(JB_DIR)/gradle.properties'); p.write_text(re.sub(r'^pluginVersion=.*', 'pluginVersion=$(SEMVER_VERSION)', p.read_text(), flags=re.MULTILINE))"
	@# Copy shared resources into plugin resources
	mkdir -p $(JB_DIR)/src/main/resources/syntaxes
	cp $(EXT_DIR)/syntaxes/tcl.tmLanguage.json $(JB_DIR)/src/main/resources/syntaxes/
	@# Build LSP server zipapp into plugin resources
	$(PYTHON) $(ROOT)scripts/build_zipapp.py lsp \
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

publish-jetbrains: jetbrains ## Publish JetBrains plugin to JetBrains Marketplace
	@echo "==> Verifying JetBrains Marketplace credentials"
	@if [ -z "$$JETBRAINS_TOKEN" ]; then \
		echo "error: JETBRAINS_TOKEN environment variable is not set"; \
		echo "       Create a token at https://plugins.jetbrains.com/author/me/tokens"; \
		echo "       Plugin page: https://plugins.jetbrains.com/plugin/31801-tcl-language-support"; \
		exit 1; \
	fi
	@echo "==> Publishing JetBrains plugin to Marketplace"
	cd $(JB_DIR) && ./gradlew publishPlugin

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
	$(UV) pip install --target $(BUILD_DIR)/sublime-stage/server --quiet \
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

publish-sublime: sublime ## Publish Sublime Text package (push build/sublime-stage to the tcl-lsp-sublime-text mirror so Package Control sees the new tag)
	@bash $(ROOT)scripts/publish_sublime.sh

# Zed extension

ZED_DIR     := $(ROOT)editors/zed
ZED_ARCHIVE := $(BUILD_DIR)/tcl-lsp-zed-$(VERSION).zip
ZED_SRCS    := $(shell find $(ZED_DIR)/src -name '*.rs' 2>/dev/null)
ZED_BUNDLED := $(ZED_DIR)/bundled

zed: $(ZED_ARCHIVE) ## Build Zed extension archive (.zip)

$(ZED_ARCHIVE): $(ZED_DIR)/Cargo.toml $(ZED_DIR)/extension.toml $(ZED_SRCS) $(PY_SRCS) $(BUILD_INFO)
	@echo "==> Building LSP + MCP server zipapps for bundling"
	@mkdir -p $(ZED_BUNDLED)
	$(PYTHON) $(ROOT)scripts/build_zipapp.py lsp \
		--version $(VERSION) \
		--output $(ZED_BUNDLED)/tcl-lsp-server.pyz
	$(PYTHON) $(ROOT)scripts/build_zipapp.py mcp \
		--version $(VERSION) \
		--output $(ZED_BUNDLED)/tcl-lsp-mcp-server.pyz
	@echo "==> Building Zed extension WASM (with bundled servers)"
	@if [ -f "$$HOME/.cargo/env" ]; then . "$$HOME/.cargo/env"; fi; \
	if ! rustup target list --installed 2>/dev/null | grep -q wasm32-wasip2; then \
		echo "  -> Installing wasm32-wasip2 target via rustup"; \
		rustup target add wasm32-wasip2; \
	fi
	@if [ -f "$$HOME/.cargo/env" ]; then . "$$HOME/.cargo/env"; fi; \
	cd $(ZED_DIR) && TCL_LSP_BUNDLED_VERSION="$(VERSION)" cargo build --target wasm32-wasip2 --release
	@echo "==> Staging Zed extension archive"
	@rm -rf $(BUILD_DIR)/zed-stage
	@mkdir -p $(BUILD_DIR)/zed-stage
	cp $(ZED_DIR)/extension.toml $(BUILD_DIR)/zed-stage/
	$(PYTHON) -c "import re,pathlib; p=pathlib.Path('$(BUILD_DIR)/zed-stage/extension.toml'); p.write_text(re.sub(r'^version = .*', 'version = \"$(SEMVER_VERSION)\"', p.read_text(), flags=re.MULTILINE))"
	cp $(ZED_DIR)/target/wasm32-wasip2/release/tcl_lsp_zed.wasm $(BUILD_DIR)/zed-stage/extension.wasm
	cp -r $(ZED_DIR)/languages $(BUILD_DIR)/zed-stage/
	cp -r $(ZED_DIR)/snippets $(BUILD_DIR)/zed-stage/
	@echo "==> Packaging Zed extension archive"
	mkdir -p $(BUILD_DIR)
	cd $(BUILD_DIR)/zed-stage && zip -qr $(abspath $(ZED_ARCHIVE)) .
	@rm -rf $(ZED_BUNDLED)
	@echo ""
	@echo "Built: $(ZED_ARCHIVE)"
	@ls -lh $(ZED_ARCHIVE)

publish-zed: zed ## Publish Zed extension (prep local PR branch for zed-industries/extensions; you push + open the PR)
	@bash $(ROOT)scripts/publish_zed.sh

# Release

release: package-vsix zipapp-cli zipapp-tcl zipapp-f5 zipapp-gui-cdn zipapp-lsp claude-skills zipapp-mcp zipapp-wasm jetbrains sublime zed release-sums ## Build all release artifacts (parity with tagged CI release jobs)
	@echo ""
	@echo "Built release artifacts in $(BUILD_DIR)"

# Aggregate sha256 hashes for every release artefact in BUILD_DIR. The
# CI publish-checksums job hashes every release-asset file (except
# SHA256SUMS itself and its signature bundle); this target mirrors that
# selection so developers can compare locally-built SUMS against the
# published file.
.PHONY: release-sums
release-sums: zipapp-cli zipapp-tcl zipapp-f5 zipapp-gui-cdn zipapp-lsp zipapp-mcp zipapp-wasm claude-skills package-vsix jetbrains sublime zed
	@cd $(BUILD_DIR) && \
	    if command -v sha256sum >/dev/null 2>&1; then h="sha256sum"; \
	    else h="shasum -a 256"; fi; \
	    files=$$(find . -maxdepth 1 -type f \
	        ! -name 'SHA256SUMS' ! -name 'SHA256SUMS.*' \
	        | sed 's|^\./||' \
	        | LC_ALL=C sort); \
	    if [ -z "$$files" ]; then \
	        : > SHA256SUMS; \
	    else \
	        printf '%s\n' $$files | xargs $$h > SHA256SUMS; \
	    fi
	@echo "Wrote $(BUILD_DIR)/SHA256SUMS"

release-tag: ## Bump version, annotated-tag, and push (V=x.y.z)
	@bash $(ROOT)scripts/release.sh $(V)

publish-all: publish-vsix publish-openvsx publish-jetbrains publish-sublime publish-zed ## Publish to all editor marketplaces

publish-verify: ## Sanity-check publishing readiness (credentials, tool versions, remote reach) without shipping
	@bash $(ROOT)scripts/publish_verify.sh

# KCS help database

kcs-db: $(KCS_DB) ## Build the KCS help database from docs/kcs/features/

$(KCS_DB): $(wildcard docs/kcs/features/kcs-feature-*.md) $(wildcard docs/screenshots/*.png docs/screenshots/*.gif) scripts/build_kcs_db.py
	@echo "==> Building KCS help database"
	$(PYTHON) $(ROOT)scripts/build_kcs_db.py --out $@

clean-kcs-db: ## Remove the generated KCS help database
	rm -f $(KCS_DB)

# Screenshots

screenshot: screenshots ## Alias for make screenshots

screenshots: compile ## Capture extension screenshots and build animated GIF (macOS)
	@echo "==> Building extension in screenshot mode"
	cd $(EXT_DIR) && $(NPM) run bundle:screenshots
	@echo "==> Running screenshot capture"
	TCL_LSP_SCREENSHOT_AUTO_BREW=$${TCL_LSP_SCREENSHOT_AUTO_BREW:-1} bash $(ROOT)scripts/screenshots.sh
	@echo "==> Optimising screenshots"
	@if command -v pngquant >/dev/null 2>&1 && command -v optipng >/dev/null 2>&1; then \
		for f in $(SCREENSHOT_DIR)/*.png; do \
			if ! file "$$f" | grep -q 'colormap'; then \
				pngquant --quality=65-80 --speed 1 --strip --force --output "$$f" "$$f" 2>/dev/null; \
				optipng -o5 -strip all -quiet "$$f" 2>/dev/null; \
			fi; \
		done; \
		echo "    PNG optimisation complete"; \
	else \
		echo "    WARN: pngquant/optipng not found — skipping PNG optimisation"; \
	fi
	@if command -v gifsicle >/dev/null 2>&1; then \
		for f in $(SCREENSHOT_DIR)/*.gif; do \
			gifsicle -O3 --lossy=80 --colors 128 "$$f" -o "$$f.opt" 2>/dev/null && mv "$$f.opt" "$$f"; \
		done; \
		echo "    GIF optimisation complete"; \
	else \
		echo "    WARN: gifsicle not found — skipping GIF optimisation"; \
	fi

clean-screenshots: ## Remove captured screenshots
	rm -rf $(SCREENSHOT_DIR)/*.png $(SCREENSHOT_DIR)/*.gif

# Cleanup

clean: ## Remove build artifacts
	rm -rf $(BUILD_DIR)
	rm -rf $(OUT_DIR)
	rm -f  $(BUILD_INFO)
	rm -f  $(BUILD_INFO_JSON)
	rm -f  $(KCS_DB)
	rm -rf $(PYODIDE_DIR)
	rm -f  $(EXPLORER_STATIC)/*.whl
	rm -f  $(MERMAID_JS)
	rm -rf $(ZED_DIR)/bundled
	find $(ROOT) -type d -name __pycache__ -exec rm -rf {} + 2>/dev/null || true

distclean: clean ## Remove build artifacts and node_modules
	rm -rf $(EXT_DIR)/node_modules
	rm -f  $(EXT_DIR)/package-lock.json

# Zig runtime (WASM) — built ad-hoc by contributors today; targets
# below provide a scriptable entry-point and the leak-check variant
# used by S0.2.

.PHONY: build-runtime build-runtime-leakcheck

build-runtime: ## Build runtime/zig (default debug build) → tcl_runtime.wasm
	cd runtime/zig && zig build

build-runtime-leakcheck: ## Build runtime with -Dleak-check=true (S0.2 instrumentation)
	cd runtime/zig && rm -rf .zig-cache && zig build -Dleak-check=true

.PHONY: leakcheck leakcheck-diff snapshot-leak-baseline

leakcheck: build-runtime-leakcheck ## Run the in-scope tcltest suite under the leak-check runtime; emit per-file alloc / double-free counts.
	uv run --with pytest --with wasmtime python scripts/dev/leak_sweep.py

leakcheck-diff: ## Diff the latest leak sweep against tests/baselines/wasm_leak_baseline.json
	uv run python scripts/dev/diff_leak_sweep.py

snapshot-leak-baseline: ## Promote tmp/perf-output/leak_sweep_results.json to the committed baseline
	cp tmp/perf-output/leak_sweep_results.json tests/baselines/wasm_leak_baseline.json

# ---------------------------------------------------------------------------
# Sphinx — f5q Python API reference
# ---------------------------------------------------------------------------

.PHONY: docs docs-html docs-clean docs-linkcheck

docs: docs-html  ## Build the f5q Sphinx HTML docs (alias for docs-html)

docs-html: $(UV_STAMP)  ## Build the f5q Sphinx API reference (docs/sphinx/_build/html)
	uv run --extra docs sphinx-build -b html docs/sphinx docs/sphinx/_build/html
	@echo "==> docs built → docs/sphinx/_build/html/index.html"

docs-linkcheck: $(UV_STAMP)  ## Check every external link in the f5q docs
	uv run --extra docs sphinx-build -b linkcheck docs/sphinx docs/sphinx/_build/linkcheck

docs-clean:  ## Remove generated Sphinx output
	rm -rf docs/sphinx/_build
