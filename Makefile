# tcl-lsp — build, test, and package
#
# Quick reference (see `make help` for the full list, all docstrings are
# the source of truth):
#
#   make check-all     Pre-push gate — full lint+typecheck across all languages.
#   make prep-pr       Fast pre-PR gate — format + codegen + lint + tests.
#   make build-editor-vsix          Build the VS Code .vsix (runs tests first).
#   make release       Build every release artefact.
#
# Prerequisites:
#   - Rust stable with cargo (via rustup).  The workspace tracks the floating
#     `stable` channel pinned in rust-toolchain.toml; current stable is 1.98.0,
#     released 2026-08-18.  `Cargo.toml` `rust-version` is authoritative.
#   - Node.js 24+ with npm
#   - macOS WASM builds: `make ensure-rust-deps` installs the pinned wasi-sdk;
#     stock Apple clang has no wasm32 backend.
#

SHELL := /bin/bash
.DELETE_ON_ERROR:

# ---------------------------------------------------------------------------
# Directories
# ---------------------------------------------------------------------------

ROOT     := $(dir $(abspath $(lastword $(MAKEFILE_LIST))))

EXT_DIR         := $(ROOT)editors/vscode
OUT_DIR         := $(EXT_DIR)/out
# Shared TypeScript front-end for the BIG-IP report generators (built to
# rust/bigip-report-gen/frontend/dist and synced into the Python f5report package).
REPORT_SHARED_DIR := $(ROOT)rust/bigip-report-gen/frontend
# TypeScript front-end for the command-registry spec studio (bundled into the
# studio's dist directory by rust/tcl-spec-studio-wasm/build-wasm.sh).
SPEC_STUDIO_WEB := $(ROOT)rust/tcl-spec-studio/web
# The compiler-explorer GUI shell lives in the `tcl` crate; `make explorer-wasm`
# builds the Rust → WASM core + Mermaid into it, and `build.rs` embeds the whole
# bundle into the `tcl` binary (served by `tcl explore --serve`).
EXPLORER_STATIC := $(ROOT)rust/tcl-cli/gui

# Build output — everything generated goes under build/
BUILD_DIR  := $(ROOT)build

# Tools
NPM      := npm
NODE_BIN := $(EXT_DIR)/node_modules/.bin
TSC      := $(NODE_BIN)/tsc
VSCE     := $(NODE_BIN)/vsce
OVSX     := $(NODE_BIN)/ovsx
VSCODE   ?= code

# Stamps (used to avoid re-running expensive steps when deps haven't changed)
STAMP_DIR  := $(BUILD_DIR)/stamps
NPM_STAMP  := $(STAMP_DIR)/npm-install
REPORT_NPM_STAMP := $(STAMP_DIR)/report-npm-install
SPEC_STUDIO_NPM_STAMP := $(STAMP_DIR)/spec-studio-npm-install
STAGE_DIR  := $(BUILD_DIR)/vsix-stage

# Version — derived from git describe (fallback: dev when unavailable)
GIT_DESCRIBE_RAW := $(shell git describe --tags --abbrev=1 --always --dirty=-dev 2>/dev/null || true)
GIT_DESCRIBE     := $(if $(strip $(GIT_DESCRIBE_RAW)),$(GIT_DESCRIBE_RAW),dev)
GIT_HASH         := $(shell git rev-parse --short HEAD 2>/dev/null || echo unknown)
VERSION          := $(shell echo "$(GIT_DESCRIBE)" | sed 's/^v//')
SEMVER_VERSION   := $(shell sh -c 'v="$(VERSION)"; if echo "$$v" | grep -Eq "^[0-9]+\\.[0-9]+\\.[0-9]+([-.][0-9A-Za-z.-]+)*$$"; then echo "$$v"; else echo "0.0.0-dev"; fi')
FULL_VERSION     := $(VERSION)
# Pre-release channel switch — the VS Code odd/even-minor convention.
# `scripts/release/prerelease.sh` is the single source of truth: a 2.x
# release with an ODD minor (2.1.x) is a pre-release; 1.x and even-minor
# 2.x (2.2.0) are stable.  `VSCE_PRERELEASE_FLAG` expands to
# `--pre-release` for the pre-release line and to nothing for stable, so
# the same `vsce package` / `vsce publish` recipes serve both channels.
IS_PRERELEASE       := $(shell bash $(ROOT)scripts/release/prerelease.sh "$(VERSION)" 2>/dev/null || echo false)
VSCE_PRERELEASE_FLAG := $(if $(filter true,$(IS_PRERELEASE)),--pre-release,)
# JetBrains has no per-version pre-release flag — it uses named release
# channels instead.  Map the same convention onto a channel: "eap" for the
# pre-release line, empty (the default Stable channel) for stable.
JETBRAINS_CHANNEL   := $(if $(filter true,$(IS_PRERELEASE)),eap,)

# Derived paths
VSIX_FILE      := $(BUILD_DIR)/tcl-lsp-vscode-$(VERSION)-universal.vsix
# Set (via a recursive `$(MAKE) VSCE_TARGET=...`) to bake a `vsce package
# --target <platform>` tag into the $(VSIX_FILE) recipe below.  Empty by
# default, which packages the untargeted "universal" VSIX.
# package-vsix-targets drives this for the six platform-targeted packages.
VSCE_TARGET ?=
# Self-contained BIG-IP report .pyz (native `_engine` + MiniJinja bundled by shiv).
# Native + abi3, so the artefact is OS/arch-specific but runs on any CPython
# >= 3.9 for that platform; the tag keeps CI matrix outputs from clobbering.
REPORT_PY_DIR  := $(ROOT)rust/bigip-report-gen/python
REPORT_WHEELS  := $(BUILD_DIR)/report-wheels
REPORT_PYZ_OS   := $(shell uname -s | tr '[:upper:]' '[:lower:]')
REPORT_PYZ_ARCH := $(shell uname -m)
REPORT_PYZ     := $(BUILD_DIR)/f5-report-$(VERSION)-$(REPORT_PYZ_OS)-$(REPORT_PYZ_ARCH).pyz
LICENSE_SRC    := $(ROOT)LICENSE
README_SRC     := $(ROOT)README.md
SCREENSHOT_DIR := $(ROOT)docs/screenshots
SCREENSHOTS    := $(wildcard $(SCREENSHOT_DIR)/*.png $(SCREENSHOT_DIR)/*.gif)
VSCE_PUBLISHER := bitwisecook

# Cargo build profile for the native Rust LSP server (rust-server target).
PROFILE ?= release

# Native-server cross-compilation.  Two VS Code packaging strategies share
# this one 7-triple matrix:
#   - $(VSIX_FILE), the untargeted "universal" VSIX: bundles one
#     `tcl-lsp-server` binary per platform under `server/<platform>-<arch>/`.
#     Published with no vsce --target, it is the Marketplace's fallback for
#     any client with no dedicated targeted package below (namely riscv64
#     Linux — vsce has no --target string for it), and the artefact for a
#     manual "Install from VSIX" side-load.
#   - package-vsix-targets: six small, single-binary VSIXes, one per vsce
#     --target platform, so the Marketplace serves each client only its own
#     binary instead of all seven.
# Each SERVER_TARGET_MAP entry maps a Rust target triple to the VSIX bundle
# directory, which equals Node's `process.platform-process.arch` AND (for
# six of the seven) a valid vsce --target string.
SERVER_TARGET_MAP := \
	x86_64-apple-darwin:darwin-x64 \
	aarch64-apple-darwin:darwin-arm64 \
	x86_64-unknown-linux-gnu:linux-x64 \
	aarch64-unknown-linux-gnu:linux-arm64 \
	riscv64gc-unknown-linux-gnu:linux-riscv64 \
	x86_64-pc-windows-msvc:win32-x64 \
	aarch64-pc-windows-msvc:win32-arm64
SERVER_TARGETS_ALL := $(foreach p,$(SERVER_TARGET_MAP),$(firstword $(subst :, ,$(p))))

# The bundled SpecTcl loadables (`docs/design/spec-packs.md`): the EDA vendor
# libraries are `.tclspec` packs, not compiled-in Rust, so a shipped server is
# incomplete without them. `tcl_spectcl::discovery::bundled_dir` looks for a
# `specs/` directory *beside the executable*, so every place that stages a
# `tcl-lsp-server` binary stages this directory next to it.
SPEC_PACK_SRC   := $(ROOT)specs
SPEC_PACK_FILES := $(wildcard $(SPEC_PACK_SRC)/*.tclspec)

# The WASI language server — the same `LspService<Backend>` as the native
# binary, linked for wasm32-wasip1 and driven over Content-Length-framed stdio
# (`docs/design/rust/lsp-runtime-and-transports.md`).  Declared up here rather
# than beside its build target further down because $(VSIX_FILE) names the
# module as a *prerequisite*, and prerequisites expand when the rule is read.
LSP_SERVER_WASI_DIR    := $(ROOT)rust/tcl-lsp-server-wasi
LSP_SERVER_WASI_MODULE := $(LSP_SERVER_WASI_DIR)/dist/tcl-lsp-server-wasi.wasm
# The module's own sources, so the file rule further down can tell a stale
# module from an up-to-date one.  Same `shell find` idiom as $(TS_SRCS); the
# crate is its own workspace with its own lockfile, so its lockfile is the one
# named here.  Declared beside the module for the same reason it is: this
# expands where the rule is read.
LSP_SERVER_WASI_SRCS := $(shell find $(LSP_SERVER_WASI_DIR)/src -type f 2>/dev/null) \
	$(LSP_SERVER_WASI_DIR)/Cargo.toml $(LSP_SERVER_WASI_DIR)/Cargo.lock \
	$(LSP_SERVER_WASI_DIR)/build-wasi.sh
# Where the universal VSIX stages it, relative to the extension root.  Mirrored
# by `WASI_MODULE_RELATIVE_PATH` in editors/vscode/src/serverResolution.ts.
VSIX_WASI_DIR := server/wasm

# vsce's supported --target platform strings (see "Platform-specific
# extensions" at code.visualstudio.com/api/working-with-extensions/publishing-extension).
# Six of the seven SERVER_TARGET_MAP bundle dirs are also valid vsce
# targets; linux-riscv64 is not (vsce has no RISC-V target), so riscv64
# Linux users get the untargeted $(VSIX_FILE) fallback instead.
VSCE_TARGETS := win32-x64 win32-arm64 linux-x64 linux-arm64 darwin-x64 darwin-arm64

# Targets the current host can build natively / with cross-linkers.  Linux
# builds the three Linux triples (x86_64 native + aarch64/riscv64 cross);
# macOS builds both Darwin triples; a Windows runner builds both win32 triples.
SERVER_UNAME_S := $(shell uname -s)
ifeq ($(SERVER_UNAME_S),Linux)
  SERVER_TARGETS_HOST := x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu riscv64gc-unknown-linux-gnu
else ifeq ($(SERVER_UNAME_S),Darwin)
  SERVER_TARGETS_HOST := aarch64-apple-darwin x86_64-apple-darwin
else
  SERVER_TARGETS_HOST := x86_64-pc-windows-msvc aarch64-pc-windows-msvc
endif

# The set of triples staged into $(VSIX_FILE) (the universal/fallback VSIX).
# Defaults to the host-buildable subset (a partial-universal VSIX for local
# dev); CI overrides with the full matrix: `make package-vsix
# BUNDLED_TARGETS="$(SERVER_TARGETS_ALL)"`.
BUNDLED_TARGETS ?= $(SERVER_TARGETS_HOST)

# JetBrains ships one universal plugin bundling every platform except
# riscv64 Linux — no official JetBrains IDE build targets it, and the IDE's
# own CpuArch detection only distinguishes x86/ARM anyway. Derived from
# SERVER_TARGETS_ALL (not hardcoded) so a future 8th non-riscv target picks
# this up automatically.
SERVER_TARGETS_JETBRAINS := $(filter-out riscv64gc-unknown-linux-gnu,$(SERVER_TARGETS_ALL))
JB_BUNDLED_TARGETS ?= $(filter-out riscv64gc-unknown-linux-gnu,$(SERVER_TARGETS_HOST))

# This host's own Rust target triple — the one binary cargo can always build
# with no cross toolchain.  Used by the `smoke-vsix` gate for a dependency-light
# native-only VSIX (the full multi-platform build is CI's job).
SERVER_TARGET_NATIVE := $(shell rustc -vV 2>/dev/null | sed -n 's/^host: //p')

CLAUDE_SKILLS  := $(BUILD_DIR)/tcl-lsp-claude-skills-$(VERSION).zip

# Parallelism
NPROC := $(shell nproc 2>/dev/null || sysctl -n hw.ncpu 2>/dev/null || echo 4)

# Source-file lists for dependency tracking.
TS_SRCS  := $(shell find $(EXT_DIR)/src -name '*.ts' 2>/dev/null)

# ---------------------------------------------------------------------------
# Phony targets — declared once at the top, organised by section.  File-
# producing rules (VSIX, plugin/package archives, generated catalogs, etc.) are
# NOT phony — they live further down with real file deps.
# ---------------------------------------------------------------------------

.PHONY: help
# Top-level gates
.PHONY: rust-check check-all prep-pr _prep-pr-checks _prep-pr-tests _prep-pr-smoke _prep-pr-smoke-tier
# Tests
.PHONY: test test-ext test-emacs test-rust rust-server rust-tcl rust-f5 rust-mcp rust-clis ensure-server-cross-deps server-cross-build server-cross-build-all mcp-cross-build-all cli-cross-build-all server-cross-test server-cross-test-build print-server-targets-all print-server-targets-jetbrains
.PHONY: xtask-check xtask-editor-extensions xtask-kcs-index-links xtask-diag-tables xtask-diag-emission-check xtask-gen-editor-catalogs xtask-gen-bundled-environments xtask-gen-editor-dialects xtask-gen-irule-test-data xtask-gen-zed-queries xtask-gen-editor-settings xtask-gen-vscode-package xtask-gen-jetbrains-catalog xtask-gen-ai-diagnostics xtask-owner-resolution xtask-command-backing xtask-audit-option-dialects xtask-registry-oracle xtask-sslictcl-data xtask-runtime-stdlib tcltest-sweep tcltest-sweep-check xtask-f5query-builtins-doc xtask-bigip-data-schema xtask-c-api-ownership check-c-api-ownership
.PHONY: xtask-workflow-sync xtask-resolution-drift xtask-retired-api-gate xtask-pack-goldens xtask-number-drift xtask-gen-tmlanguage-keywords xtask-option-registry-drift xtask-callback-inventory check-tcl-reference-toolchains check-spectcl-compat-paths check-runtime-rust-paths check-rust-tests-runner check-wasm-cc-env xtask-dialect-drift
# Lint / format / typecheck
.PHONY: lint format lint-ts format-ts typecheck-ts check-rust check-rust-pr _check-rust-pr rust-deny
.PHONY: build-report-assets build-report-pyz lint-report-ts typecheck-report-ts check-report-assets lint-spec-studio-ts typecheck-spec-studio-ts
# Coverage
.PHONY: coverage coverage-ext
# Compile + codegen + generated assets
.PHONY: compile codegen generate check-generated gen-editor-settings check-editor-settings gen-irule-test-data copy-canonical npm-env logo
.PHONY: update-source-data check-source-data
# Compiler explorer (WASM GUI)
.PHONY: explorer-wasm explorer-build compiler-explorer-gui
# Skills bundle + smoke tests
.PHONY: claude-skills
.PHONY: smoke-vsix
# Packaging + publish + release
.PHONY: build-editors build-editor-vsix verify-vsix install package-vsix publish-vsix publish-openvsx
.PHONY: build-editor-vsix-targets package-vsix-targets publish-vsix-targets publish-openvsx-targets
.PHONY: build-editor-jetbrains verify-jetbrains-server verify-editor-jetbrains publish-jetbrains build-editor-sublime verify-standalone-eda build-editor-zed publish-zed publish-all publish-verify publish-flow
.PHONY: release release-tag release-sums
.PHONY: release-perf release-notes-perf release-verify release-prepare release-rust-tag
# Rust runtime port
.PHONY: runtime-rust-test runtime-rust-lint zed-query-check vm-test vm-lint
# Screenshots
.PHONY: screenshot screenshots clean-screenshots
# Cleanup
.PHONY: clean distclean
# Dep-installer helpers
.PHONY: ensure-test-deps install-test-deps ensure-tcl-deps ensure-tcl90-reference ensure-rust-deps ensure-emacs-deps ensure-vscode-test-deps

help: ## Show this help
	@grep -E '^[a-zA-Z][a-zA-Z0-9_-]*:.*?## ' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-24s\033[0m %s\n", $$1, $$2}'

build-editors: build-editor-vsix build-editor-vsix-targets build-editor-jetbrains build-editor-sublime build-editor-zed ## Build all editor extension artefacts (VS Code / JetBrains / Sublime / Zed)

build-editor-vsix: lint test compile verify-vsix ## Build the .vsix (tests must pass first)
install: package-vsix ## Build and install the .vsix into VS Code
	@echo "==> Installing VS Code extension"
	$(VSCODE) --install-extension $(VSIX_FILE) --force

publish-vsix: package-vsix ## Publish the .vsix to the VS Code Marketplace (laptop fallback; CI is the primary path)
	@echo "==> Publishing $(VSIX_FILE) to VS Code Marketplace"
	@# Releases normally publish VSCE from CI (job publish-vsix-marketplace,
	@# secrets.VSCE_PAT on the protected marketplace-vscode Environment).
	@# This laptop target is the fallback for when that CI job fails.
	@# It prefers a keyless Azure Entra session via the Azure CLI
	@# (`az login --allow-no-subscriptions`), feeding vsce's
	@# DefaultAzureCredential through `--azure-credential` — no PAT at rest.
	@# Set VSCE_PAT to force the legacy stored-PAT path (discouraged; Azure
	@# DevOps global PATs retire 2026-12-01).
	@if [ -n "$(VSCE_PRERELEASE_FLAG)" ]; then \
		echo "    Pre-release channel (odd-minor $(VERSION)) — publishing with --pre-release."; \
	fi
	@if [ -n "$$VSCE_PAT" ]; then \
		echo "    VSCE_PAT set — using the legacy stored PAT (override)."; \
		cd $(STAGE_DIR) && $(VSCE) publish $(VSCE_PRERELEASE_FLAG) --skip-duplicate --packagePath $(VSIX_FILE); \
	elif az account show >/dev/null 2>&1; then \
		echo "    Keyless publish via Azure Entra (--azure-credential, no PAT)."; \
		cd $(STAGE_DIR) && $(VSCE) publish $(VSCE_PRERELEASE_FLAG) --azure-credential --skip-duplicate --packagePath $(VSIX_FILE); \
	else \
		echo "    No Azure CLI session for keyless publishing."; \
		echo "    Run:  az login --allow-no-subscriptions"; \
		echo "    (or set VSCE_PAT to use the legacy stored-PAT path instead.)"; \
		exit 1; \
	fi

publish-openvsx: package-vsix ## Publish the .vsix to Open VSX (code-server / openvscode-server / Gitpod / Theia; laptop fallback; CI is the primary path)
	@echo "==> Publishing $(VSIX_FILE) to Open VSX"
	@# Releases normally publish via ovsx from CI (job publish-vsix-openvsx,
	@# secrets.OVSX_PAT on the protected marketplace-openvsx Environment).
	@# This laptop target is the fallback for when that CI job fails.
	@# Open VSX has no keyless credential flow like vsce's Azure Entra path —
	@# an account-wide OVSX_PAT (a token from
	@# https://open-vsx.org/user-settings/tokens; publishing to the
	@# bitwisecook namespace needs that account to be a member/owner of it)
	@# must be set.
	@# No --pre-release flag here: ovsx ignores it for an already-packaged
	@# VSIX (ovsx_publish.sh explains why). The channel is already baked
	@# into $(VSIX_FILE)'s manifest by `vsce package $(VSCE_PRERELEASE_FLAG)`
	@# above, which open-vsx.org reads.
	@if [ -z "$${OVSX_PAT:-}" ]; then \
		echo "    OVSX_PAT is not set."; \
		echo "    Generate a token at https://open-vsx.org/user-settings/tokens and export OVSX_PAT."; \
		exit 1; \
	fi
	cd $(STAGE_DIR) && $(OVSX) publish --skip-duplicate --packagePath $(VSIX_FILE)

$(VSIX_FILE): spec-studio-wasm $(if $(VSCE_TARGET),,$(LSP_SERVER_WASI_MODULE)) $(OUT_DIR)/extension.js $(EXT_DIR)/package.json $(EXT_DIR)/.vscodeignore $(LICENSE_SRC) $(README_SRC) $(SCREENSHOTS) $(ROOT)scripts/install/filter-readme.mjs
	@echo "==> Preparing VSIX staging directory"
	rm -rf $(STAGE_DIR)
	mkdir -p $(STAGE_DIR)
	rsync -a --delete --delete-excluded \
		--exclude='.venv/' \
		--exclude='.pytest_cache/' \
		--exclude='.ruff_cache/' \
		--exclude='.mypy_cache/' \
		--exclude='.vscode-test/' \
		--exclude='.vscode-test-web/' \
		$(EXT_DIR)/ $(STAGE_DIR)/
	mkdir -p $(STAGE_DIR)/spec-studio
	cp -R $(ROOT)rust/tcl-spec-studio-wasm/dist/. $(STAGE_DIR)/spec-studio/
	@# The browser language server (package.json `browser`), for vscode.dev /
	@# github.dev and every other web extension host.  Staged from
	@# `make lsp-server-wasm`'s dist rather than trusting the extension
	@# directory's own copy, so a VSIX can never ship a stale worker.  The
	@# `spec-studio-wasm` prerequisite above already builds it.
	@#
	@# It ships in EVERY VSIX flavour, targeted or not: one package.json declares
	@# both entry points, so a targeted package whose `browser` entry had no
	@# assets would fail on the web while claiming to support it.
	@echo "==> Staging the browser language server (dist/web)"
	mkdir -p $(STAGE_DIR)/dist/web/specs
	cp $(LSP_SERVER_WASM_DIR)/dist/worker.js \
		$(LSP_SERVER_WASM_DIR)/dist/tcl_lsp_server_wasm.js \
		$(LSP_SERVER_WASM_DIR)/dist/tcl_lsp_server_wasm_bg.wasm \
		$(STAGE_DIR)/dist/web/
	@# The bundled SpecTcl loadables again: the native server finds them in a
	@# `specs/` directory beside its executable, and the browser server — which
	@# has no executable — gets them upserted into the virtual pack mount at
	@# startup (docs/design/contracts/lsp-source-store.md).
	cp $(SPEC_PACK_FILES) $(STAGE_DIR)/dist/web/specs/
	@# …plus a manifest naming them.  An installed web extension's files are
	@# served over http, where VS Code's filesystem provider answers `readFile`
	@# only, so the browser entry cannot list this directory to find them.
	node -e "const fs=require('fs');const d='$(STAGE_DIR)/dist/web/specs';const p=fs.readdirSync(d).filter(f=>f.endsWith('.tclspec')).sort();fs.writeFileSync(d+'/index.json',JSON.stringify(p,null,2)+'\n')"
	@# Inject version from git describe into staged package.json
	node -e "const f='$(STAGE_DIR)/package.json';const p=JSON.parse(require('fs').readFileSync(f));p.version='$(SEMVER_VERSION)';require('fs').writeFileSync(f,JSON.stringify(p,null,2)+'\n')"
	@echo "==> Bundling native tcl-lsp-server binaries: $(BUNDLED_TARGETS)"
	@set -eu; \
		missing=""; \
		for pair in $(SERVER_TARGET_MAP); do \
			triple="$${pair%%:*}"; dir="$${pair##*:}"; \
			case " $(BUNDLED_TARGETS) " in *" $$triple "*) ;; *) continue;; esac; \
			case "$$triple" in *windows*) exe="tcl-lsp-server.exe";; *) exe="tcl-lsp-server";; esac; \
			src="$(ROOT)target/$$triple/release/$$exe"; \
			if [ ! -f "$$src" ]; then missing="$$missing $$triple"; continue; fi; \
			mkdir -p "$(STAGE_DIR)/server/$$dir"; \
			cp "$$src" "$(STAGE_DIR)/server/$$dir/$$exe"; \
			chmod +x "$(STAGE_DIR)/server/$$dir/$$exe"; \
			mkdir -p "$(STAGE_DIR)/server/$$dir/specs"; \
			cp $(SPEC_PACK_SRC)/*.tclspec "$(STAGE_DIR)/server/$$dir/specs/"; \
			echo "    server/$$dir/$$exe (+ specs/)"; \
		done; \
		if [ -n "$$missing" ]; then \
			echo "ERROR: missing built server binaries for:$$missing"; \
			echo "Build them first: make server-cross-build  (host targets)"; \
			echo "             or:  make server-cross-build-all  (all 7 — needs cross deps)"; \
			exit 1; \
		fi
	@# The WASI language server — the universal package's last rung, for an
	@# architecture none of the seven cross-compiled triples covers.  TRANSITIONAL:
	@# the universal VSIX carries the natives AND this module; the six targeted
	@# packages carry only their own native binary and must NOT inherit it, which
	@# is why this whole block is conditional on an empty VSCE_TARGET (and why
	@# `verify-vsix` asserts both halves).
	@#
	@# The spec packs ride along in `specs/` beside the module, mirroring the
	@# native staging above.  A WASI guest has no executable for
	@# `tcl_spectcl::discovery::bundled_dir` to sit beside, so editors/vscode's
	@# wasiServer.ts mounts this directory into the guest and names it with
	@# TCL_LSP_SPEC_PACK_DIR instead.
	@set -eu; \
		if [ -n "$(VSCE_TARGET)" ]; then \
			echo "==> Skipping the WASI module (platform-targeted VSIX: $(VSCE_TARGET))"; \
		else \
			echo "==> Staging the WASI language server ($(VSIX_WASI_DIR)/)"; \
			mkdir -p "$(STAGE_DIR)/$(VSIX_WASI_DIR)/specs"; \
			cp "$(LSP_SERVER_WASI_MODULE)" "$(STAGE_DIR)/$(VSIX_WASI_DIR)/"; \
			cp $(SPEC_PACK_FILES) "$(STAGE_DIR)/$(VSIX_WASI_DIR)/specs/"; \
			echo "    $(VSIX_WASI_DIR)/$(notdir $(LSP_SERVER_WASI_MODULE)) (+ specs/)"; \
		fi
	cp $(LICENSE_SRC) $(STAGE_DIR)/LICENSE.txt
	node $(ROOT)scripts/install/filter-readme.mjs $(README_SRC) --editor "VS Code" -o $(STAGE_DIR)/README.md
	mkdir -p $(STAGE_DIR)/docs/screenshots
	cp $(SCREENSHOT_DIR)/*.png $(SCREENSHOT_DIR)/*.gif $(STAGE_DIR)/docs/screenshots/
	cp "$(ROOT)docs/Tcl LSP Logo-8bit-256.png" $(STAGE_DIR)/docs/icon.png
	@echo "==> Packaging .vsix (stripped, not obfuscated)$(if $(VSCE_PRERELEASE_FLAG), [pre-release],)$(if $(VSCE_TARGET), [target: $(VSCE_TARGET)],)"
	cd $(STAGE_DIR) && $(VSCE) package $(VSCE_PRERELEASE_FLAG) $(if $(VSCE_TARGET),--target $(VSCE_TARGET),) --allow-missing-repository --no-update-package-json --no-git-tag-version -o $(VSIX_FILE)
	@echo ""
	@echo "Built: $(VSIX_FILE)"
	@ls -lh $(VSIX_FILE)

verify-vsix: $(VSIX_FILE) ## Fail if dev/cache artifacts leaked into the .vsix
	@echo "==> Verifying VSIX contents"
	@set -euo pipefail; \
		BAD_ENTRIES="$$(unzip -Z1 $(VSIX_FILE) | grep -E '^extension/(\.venv/|\.ruff_cache/|\.pytest_cache/|\.mypy_cache/|\.vscode-test/|\.vscode-test-web/|\.stamps/|src/|testFixture/|out/test/|.*__pycache__/|.*\.pyc$$)' || true)"; \
		if [[ -n "$$BAD_ENTRIES" ]]; then \
			echo "VSIX contains dev/cache content that should be excluded:"; \
			echo "$$BAD_ENTRIES"; \
			exit 1; \
		fi
	@# The extension ships native binaries only — the Python server and its
	@# .pyz are gone, so any Python in the package is a packaging regression.
	@set -euo pipefail; \
		PY_ENTRIES="$$(unzip -Z1 $(VSIX_FILE) | grep -E '(\.pyz|\.py)$$|^extension/(pyproject\.toml|uv\.lock)$$' || true)"; \
		if [[ -n "$$PY_ENTRIES" ]]; then \
			echo "VSIX contains Python content (the extension ships native binaries only):"; \
			echo "$$PY_ENTRIES"; \
			exit 1; \
		fi
	@# Every requested target must ship a binary under server/<dir>/.
	@set -euo pipefail; \
		entries="$$(unzip -Z1 $(VSIX_FILE))"; \
		want=0; have=0; missing=""; \
		for pair in $(SERVER_TARGET_MAP); do \
			triple="$${pair%%:*}"; dir="$${pair##*:}"; \
			case " $(BUNDLED_TARGETS) " in *" $$triple "*) ;; *) continue;; esac; \
			case "$$triple" in *windows*) exe="tcl-lsp-server.exe";; *) exe="tcl-lsp-server";; esac; \
			want=$$((want+1)); \
			if echo "$$entries" | grep -qx "extension/server/$$dir/$$exe"; then \
				have=$$((have+1)); \
			else \
				missing="$$missing server/$$dir/$$exe"; \
			fi; \
			for pack in $(notdir $(SPEC_PACK_FILES)); do \
				echo "$$entries" | grep -qx "extension/server/$$dir/specs/$$pack" \
					|| missing="$$missing server/$$dir/specs/$$pack"; \
			done; \
		done; \
		if [ -n "$$missing" ]; then \
			echo "VSIX missing expected native server binaries / spec packs:$$missing"; \
			exit 1; \
		fi; \
		echo "==> VSIX bundles $$have/$$want native server binaries, each with the shipped spec packs"
	@# The browser half.  `package.json` declares `browser`, which is what makes
	@# the Marketplace flag the extension as web-supported, so a package that
	@# ships the manifest without the worker assets advertises a language server
	@# it cannot start.
	@set -euo pipefail; \
		entries="$$(unzip -Z1 $(VSIX_FILE))"; \
		missing=""; \
		for f in out/extension.browser.js dist/web/worker.js dist/web/tcl_lsp_server_wasm.js \
			dist/web/tcl_lsp_server_wasm_bg.wasm; do \
			echo "$$entries" | grep -qx "extension/$$f" || missing="$$missing $$f"; \
		done; \
		echo "$$entries" | grep -qx "extension/dist/web/specs/index.json" \
			|| missing="$$missing dist/web/specs/index.json"; \
		for pack in $(notdir $(SPEC_PACK_FILES)); do \
			echo "$$entries" | grep -qx "extension/dist/web/specs/$$pack" \
				|| missing="$$missing dist/web/specs/$$pack"; \
		done; \
		if [ -n "$$missing" ]; then \
			echo "VSIX missing the browser (web extension host) payload:$$missing"; \
			echo "Build it first: make lsp-server-wasm"; \
			exit 1; \
		fi; \
		echo "==> VSIX carries the browser language server (dist/web) and its spec packs"
	@# The WASI rung.  TRANSITIONAL split: the untargeted universal package
	@# carries the module (it is the fallback for architectures with no native
	@# binary), the six platform-targeted packages must NOT — each already ships
	@# its own binary, and 6 x ~19 MiB of module nobody would ever run is pure
	@# download weight.  Asserted in both directions so neither half can drift.
	@set -euo pipefail; \
		entries="$$(unzip -Z1 $(VSIX_FILE))"; \
		wasi="extension/$(VSIX_WASI_DIR)/$(notdir $(LSP_SERVER_WASI_MODULE))"; \
		if [ -n "$(VSCE_TARGET)" ]; then \
			carried="$$(echo "$$entries" | grep -E "^extension/$(VSIX_WASI_DIR)/" || true)"; \
			if [ -n "$$carried" ]; then \
				echo "Platform-targeted VSIX ($(VSCE_TARGET)) must not carry the WASI module:"; \
				echo "$$carried"; \
				exit 1; \
			fi; \
			echo "==> VSIX is platform-targeted ($(VSCE_TARGET)) and correctly omits $(VSIX_WASI_DIR)/"; \
		else \
			missing=""; \
			echo "$$entries" | grep -qx "$$wasi" || missing="$$missing $(VSIX_WASI_DIR)/$(notdir $(LSP_SERVER_WASI_MODULE))"; \
			for pack in $(notdir $(SPEC_PACK_FILES)); do \
				echo "$$entries" | grep -qx "extension/$(VSIX_WASI_DIR)/specs/$$pack" \
					|| missing="$$missing $(VSIX_WASI_DIR)/specs/$$pack"; \
			done; \
			if [ -n "$$missing" ]; then \
				echo "Universal VSIX missing the WASI language server payload:$$missing"; \
				echo "Build it first: make lsp-server-wasi"; \
				exit 1; \
			fi; \
			echo "==> VSIX carries the WASI language server ($(VSIX_WASI_DIR)) and its spec packs"; \
		fi

# ---------------------------------------------------------------------------
# Platform-targeted VSIX packaging — six small, single-binary VSIXes
# published with `vsce package/publish --target <platform>` so the
# Marketplace serves each client only its own binary instead of the
# untargeted $(VSIX_FILE) above.  Reuses the $(VSIX_FILE) rule verbatim via
# recursive `$(MAKE)` calls that override BUNDLED_TARGETS/VSCE_TARGET/
# VSIX_FILE per target, so staging and verify-vsix's checks never drift
# from the universal build's.
# ---------------------------------------------------------------------------

build-editor-vsix-targets: lint test compile package-vsix-targets ## Build the six platform-targeted .vsix files (tests must pass first)

# Depends on package-vsix (not just compile) even though the two artefacts
# are otherwise independent: both stage through the same $(STAGE_DIR), so
# under `make -j` (e.g. `make -j release`) an unordered pair would race —
# one recipe's `rm -rf $(STAGE_DIR)` can wipe the other's in-flight staging.
# This edge forces the universal build to finish first every time.
package-vsix-targets: package-vsix ## Package the six platform-targeted VSIXes (CI; skips lint/test)
	@set -eu; \
	for vt in $(VSCE_TARGETS); do \
		triple=""; \
		for pair in $(SERVER_TARGET_MAP); do \
			t="$${pair%%:*}"; d="$${pair##*:}"; \
			[ "$$d" = "$$vt" ] && triple="$$t"; \
		done; \
		if [ -z "$$triple" ]; then echo "ERROR: no SERVER_TARGET_MAP entry for vsce target $$vt"; exit 1; fi; \
		out="$(BUILD_DIR)/tcl-lsp-vscode-$(VERSION)-$$vt.vsix"; \
		echo "==> Packaging platform-targeted VSIX: $$vt ($$triple)"; \
		$(MAKE) --no-print-directory package-vsix BUNDLED_TARGETS="$$triple" VSCE_TARGET="$$vt" VSIX_FILE="$$out"; \
	done
	@echo "==> Built $(words $(VSCE_TARGETS)) platform-targeted VSIXes"

publish-vsix-targets: package-vsix-targets ## Publish the six platform-targeted .vsix files to the VS Code Marketplace (laptop fallback; CI is the primary path)
	@set -eu; \
	for vt in $(VSCE_TARGETS); do \
		f="$(BUILD_DIR)/tcl-lsp-vscode-$(VERSION)-$$vt.vsix"; \
		echo "==> Publishing $$f to VS Code Marketplace"; \
		if [ -n "$${VSCE_PAT:-}" ]; then \
			(cd $(STAGE_DIR) && $(VSCE) publish $(VSCE_PRERELEASE_FLAG) --skip-duplicate --packagePath "$$f"); \
		elif az account show >/dev/null 2>&1; then \
			(cd $(STAGE_DIR) && $(VSCE) publish $(VSCE_PRERELEASE_FLAG) --azure-credential --skip-duplicate --packagePath "$$f"); \
		else \
			echo "    No Azure CLI session for keyless publishing."; \
			echo "    Run:  az login --allow-no-subscriptions"; \
			echo "    (or set VSCE_PAT to use the legacy stored-PAT path instead.)"; \
			exit 1; \
		fi; \
	done

publish-openvsx-targets: package-vsix-targets ## Publish the six platform-targeted .vsix files to Open VSX (laptop fallback; CI is the primary path)
	@set -eu; \
	if [ -z "$${OVSX_PAT:-}" ]; then \
		echo "    OVSX_PAT is not set."; \
		echo "    Generate a token at https://open-vsx.org/user-settings/tokens and export OVSX_PAT."; \
		exit 1; \
	fi; \
	for vt in $(VSCE_TARGETS); do \
		f="$(BUILD_DIR)/tcl-lsp-vscode-$(VERSION)-$$vt.vsix"; \
		echo "==> Publishing $$f to Open VSX"; \
		(cd $(STAGE_DIR) && $(OVSX) publish --skip-duplicate --packagePath "$$f"); \
	done

# Test targets

test: test-rust test-ext runtime-rust-test zed-query-check ## Run all tests (Rust workspace + VS Code extension + Rust runtime port)

lint: lint-ts lint-py ## Run all lint and style checks

format: format-ts format-py ## Format TypeScript and Python code

# Python tooling. Versions are pinned so a new ruff/ty/pyright release cannot
# change the verdict of a gate between a local run and CI — the failure mode the
# floating Rust `stable` channel already gives us (see rust-toolchain.toml).
RUFF_VERSION    := 0.15.20
TY_VERSION      := 0.0.57
PYRIGHT_VERSION := 1.1.411

# The typecheck venv installs f5report — which maturin-compiles the native
# `_engine` extension — plus pytest, so ty and pyright resolve every import for
# real instead of suppressing `unresolved-import`. The Sublime host APIs
# (`sublime`, `sublime_plugin`, `LSP.plugin`) only exist inside the editor, so
# they are declared by hand-written stubs under typings/.
PY_VENV := $(ROOT).venv-typecheck

# `git ls-files` rather than a directory walk: it is the same file set every
# gate uses, and it skips build outputs, the venv, and untracked scratch files.
PY_FILES = $(shell git -C $(ROOT) ls-files '*.py')

.PHONY: lint-py format-py typecheck-py py-venv

lint-py: ## Lint + format-check every tracked Python file (ruff)
	@echo "==> Linting Python (ruff format --check + ruff check)"
	@cd $(ROOT) && uvx ruff@$(RUFF_VERSION) format --check $(PY_FILES)
	@cd $(ROOT) && uvx ruff@$(RUFF_VERSION) check $(PY_FILES)

format-py: ## Format every tracked Python file (ruff)
	@echo "==> Formatting Python with ruff"
	@cd $(ROOT) && uvx ruff@$(RUFF_VERSION) format $(PY_FILES)

py-venv: ## Build the typecheck venv (f5report + native _engine + pytest)
	@echo "==> Building Python typecheck venv ($(PY_VENV))"
	@cd $(ROOT) && uv venv --quiet --allow-existing $(PY_VENV)
	@# --reinstall-package: the wheel is cached by content, but the source tree
	@# changes under it, so a plain install can leave a stale f5report behind.
	@# The reinstall maturin-compiles the native `_engine` in release mode
	@# (~4 min), so gate it on a content hash of the package tree: ty/pyright
	@# read the committed `_engine.pyi` stub, never the compiled module, so the
	@# venv only needs a rebuild when the Python package itself changes.
	@cd $(ROOT) && \
	  stamp="$(PY_VENV)/.f5report-src-hash"; \
	  files=$$(git ls-files -- rust/bigip-report-gen/python | LC_ALL=C sort); \
	  hash=$$( { printf '%s\n' "$$files"; printf '%s\n' "$$files" | xargs git hash-object; } | git hash-object --stdin ); \
	  if [ ! -f "$$stamp" ] || [ "$$(cat "$$stamp")" != "$$hash" ]; then \
	    uv pip install --python $(PY_VENV) --quiet \
	        --reinstall-package f5report ./rust/bigip-report-gen/python pytest; \
	    printf '%s' "$$hash" > "$$stamp"; \
	  else \
	    echo "    f5report source unchanged — venv already current"; \
	  fi

typecheck-py: py-venv ## Type-check every tracked Python file (ty + pyright)
	@echo "==> Type-checking Python (ty)"
	@cd $(ROOT) && uvx ty@$(TY_VERSION) check \
	    --python $(PY_VENV) --extra-search-path typings \
	    --extra-search-path .claude/skills/lsp-client $(PY_FILES)
	@echo "==> Type-checking Python (pyright)"
	@cd $(ROOT) && uvx pyright@$(PYRIGHT_VERSION)

# The lsp_e2e suite is native: rust/tcl-lsp-server/tests/*_e2e.rs, run by
# `cargo test` (see test-rust). tclpkg is the `tcl-pkg` Rust crate, exercised by
# `test-rust`.

lint-ts: $(NPM_STAMP) ## Lint/format-check TypeScript extension code
	@echo "==> Linting TypeScript code (ESLint + Prettier check)"
	cd $(EXT_DIR) && $(NPM) run lint

format-ts: $(NPM_STAMP) ## Format TypeScript extension code with Prettier
	@echo "==> Formatting TypeScript code with Prettier"
	cd $(EXT_DIR) && $(NPM) run format

typecheck-ts: $(NPM_STAMP) copy-canonical ## Type-check TypeScript extension code with tsc
	@echo "==> Type-checking TypeScript code with tsc"
	cd $(EXT_DIR) && $(NPM) run compile

build-report-assets: $(REPORT_NPM_STAMP) ## Build the shared BIG-IP report front-end (TS -> dist/, synced into f5report)
	@echo "==> Building shared BIG-IP report front-end (esbuild)"
	cd $(REPORT_SHARED_DIR) && $(NPM) run build

build-report-pyz: build-report-assets ## Build a self-contained f5report .pyz (native engine + MiniJinja, via maturin + shiv)
	@echo "==> Building self-contained f5report .pyz for $(REPORT_PYZ_OS)-$(REPORT_PYZ_ARCH)"
	@command -v maturin >/dev/null 2>&1 || { echo "ERROR: 'maturin' not found — install with 'pip install maturin'"; exit 1; }
	@command -v shiv    >/dev/null 2>&1 || { echo "ERROR: 'shiv' not found — install with 'pip install shiv'"; exit 1; }
	@rm -rf $(REPORT_WHEELS)
	@mkdir -p $(REPORT_WHEELS) $(BUILD_DIR)
	@# Build the platform wheel. GIT_HASH is stamped into the native engine by
	@# the crate's build.rs so the report footer shows the commit even when the
	@# .pyz is later run outside any git checkout.
	GIT_HASH=$(GIT_HASH) maturin build --release \
		--manifest-path $(REPORT_PY_DIR)/Cargo.toml \
		--out $(REPORT_WHEELS)
	@rm -f $(REPORT_PYZ)
	@# shiv bundles the wheel + its deps (minijinja) into one .pyz; on first run it
	@# unpacks to a per-user cache (needed because CPython can't import the native
	@# `_engine` .so straight from the zip). `-c f5-report` is the project script.
	shiv --console-script f5-report --output-file $(REPORT_PYZ) \
		--find-links $(REPORT_WHEELS) --reproducible \
		f5report
	@echo "==> Built $(REPORT_PYZ)"

lint-report-ts: $(REPORT_NPM_STAMP) ## Lint/format-check the shared report search modules
	@echo "==> Linting shared report front-end (ESLint + Prettier check)"
	cd $(REPORT_SHARED_DIR) && $(NPM) run lint

typecheck-report-ts: $(REPORT_NPM_STAMP) ## Type-check the shared report front-end with tsc
	@echo "==> Type-checking shared report front-end with tsc"
	cd $(REPORT_SHARED_DIR) && $(NPM) run typecheck

lint-spec-studio-ts: $(SPEC_STUDIO_NPM_STAMP) ## Lint/format-check the spec studio front-end
	@echo "==> Linting spec studio front-end (ESLint + Prettier check)"
	cd $(SPEC_STUDIO_WEB) && $(NPM) run lint

typecheck-spec-studio-ts: $(SPEC_STUDIO_NPM_STAMP) ## Type-check the spec studio front-end with tsc
	@echo "==> Type-checking spec studio front-end with tsc"
	cd $(SPEC_STUDIO_WEB) && $(NPM) run typecheck

# Drift gate: rebuild the shared front-end and fail if the committed dist/ or the
# assets synced into the Python f5report package are stale (mirrors the vendored
# generated-artifact checks). Regenerate with `make build-report-assets`.
check-report-assets: build-report-assets ## Verify committed report dist/ + f5report-synced assets are up to date
	@echo "==> Checking shared report assets are in sync"
	@cd $(ROOT) && git diff --exit-code -- \
		rust/bigip-report-gen/frontend/dist \
		rust/bigip-report-gen/python/python/f5report/templates \
		rust/bigip-report-gen/python/python/f5report/vendor \
		|| { echo "ERROR: report assets are stale — run 'make build-report-assets' and commit the result"; exit 1; }

test-ext: ## Run VS Code extension integration tests (single-root + multi-root); skip with SKIP_TEST_EXT=1
	@# Single-shell recipe so SKIP_TEST_EXT=1 truly bypasses everything
	@# (compile + xvfb install + test host).  Without ``set -eu`` the
	@# early ``exit 0`` would only end its own recipe-line shell and
	@# make would run the next lines anyway.
	@#
	@# The extension is native-only (no Python fallback), so the Rust
	@# tcl-lsp-server binary must exist before the VS Code test host starts.
	@# Build it here (idempotent) and point the extension at it via
	@# TCL_LSP_SERVER_BIN, rather than relying on a separately-run
	@# `test-rust` winning the race.  A pre-set TCL_LSP_SERVER_BIN is
	@# honoured so callers can supply their own binary.
	@set -eu; \
	if [ -n "$${SKIP_TEST_EXT:-}" ]; then \
		echo "==> SKIP_TEST_EXT set — skipping VS Code extension tests"; \
		exit 0; \
	fi; \
	echo "==> Validating generated editor assets (Zed query registry-drift + grammar)"; \
	"$(MAKE)" xtask-gen-zed-queries zed-query-check; \
	if [ -z "$${TCL_LSP_SERVER_BIN:-}" ]; then \
		"$(MAKE)" rust-server; \
		export TCL_LSP_SERVER_BIN="$(ROOT)target/$(PROFILE)/tcl-lsp-server"; \
	fi; \
	"$(MAKE)" compile ensure-vscode-test-deps; \
	echo "==> Running VS Code extension tests (native server: $${TCL_LSP_SERVER_BIN})"; \
	if [[ "$$(uname -s)" == "Linux" && -z "$${DISPLAY:-}" ]]; then \
		if command -v xvfb-run >/dev/null 2>&1; then \
			echo "==> No DISPLAY detected; running VS Code tests under xvfb-run"; \
			cd "$(EXT_DIR)" && xvfb-run -a "$(NPM)" test; \
		else \
			echo "ERROR: DISPLAY is unset and xvfb-run is not available." >&2; \
			echo "Install xvfb (provides xvfb-run) or set DISPLAY to run extension tests." >&2; \
			exit 1; \
		fi; \
	else \
		cd "$(EXT_DIR)" && "$(NPM)" test; \
	fi; \
	echo "==> Running multi-root VS Code extension tests"; \
	if [[ "$$(uname -s)" == "Linux" && -z "$${DISPLAY:-}" ]]; then \
		cd "$(EXT_DIR)" && xvfb-run -a "$(NPM)" run test:multi-folder; \
	else \
		cd "$(EXT_DIR)" && "$(NPM)" run test:multi-folder; \
	fi

# Coverage targets (reports go to tmp/coverage/, which is gitignored)

COV_DIR := $(ROOT)tmp/coverage

coverage: coverage-ext ## Generate coverage reports for the VS Code extension

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

# --- Native (cargo xtask) check gates.  These need the Rust toolchain, so CI
# runs them in the rust-tests job (rust-gate.yml / ci.yml).  `xtask-check` is
# the CI aggregate.
xtask-check: check-tcl-reference-toolchains check-spectcl-compat-paths check-runtime-rust-paths check-rust-tests-runner check-wasm-cc-env xtask-workflow-sync xtask-kcs-index-links xtask-diag-tables xtask-diag-emission-check xtask-gen-editor-catalogs xtask-gen-bundled-environments xtask-gen-editor-dialects xtask-gen-irule-test-data xtask-gen-zed-queries xtask-gen-tmlanguage-keywords xtask-gen-editor-settings xtask-gen-vscode-package xtask-gen-jetbrains-catalog xtask-gen-ai-diagnostics xtask-owner-resolution xtask-resolution-drift xtask-retired-api-gate xtask-pack-goldens xtask-number-drift xtask-command-backing xtask-callback-inventory xtask-option-registry-drift xtask-sslictcl-data xtask-runtime-stdlib xtask-editor-extensions xtask-f5query-builtins-doc xtask-bigip-data-schema xtask-c-api-ownership ## Rust-side check gates (docs index coverage + generated-table/catalog drift) xtask-dialect-drift

check-tcl-reference-toolchains: ## Verify pinned C Tcl patchlevels across shell setup and Rust oracle discovery
	@echo "==> Checking C Tcl reference toolchain ownership"
	@bash scripts/dev/test-reference-tcl-toolchains.sh
	@cargo test -p tcl-test-support

check-spectcl-compat-paths: ## Verify CI's SpecTcl dependency closure and central shipped-pack inventory
	@echo "==> Checking SpecTcl compatibility path ownership"
	@bash scripts/dev/test-spectcl-compat-paths.sh

check-runtime-rust-paths: ## Verify CI's standalone-runtime dependency closure and job wiring (#1768)
	@echo "==> Checking standalone runtime CI path ownership"
	@bash scripts/dev/test-runtime-rust-paths.sh

check-rust-tests-runner: ## Verify trusted rust-tests jobs serialize on the shared tank host
	@echo "==> Checking self-hosted Rust test scheduling"
	@sh scripts/dev/test-rust-tests-runner.sh

check-wasm-cc-env: ## Verify wasm32 C-compiler selection and dependency diagnostics
	@echo "==> Checking wasm32 C compiler bootstrap"
	@bash scripts/dev/test-wasm-cc-env.sh

xtask-runtime-stdlib: ## Verify the embedded Tcl stdlib version, provenance, hashes, and FILES table
	@echo "==> Checking embedded Tcl standard-library provenance (cargo xtask)"
	cd $(ROOT) && cargo xtask runtime-stdlib

xtask-gen-bundled-environments: ## Verify the compiled environment seed matches the bundled packs' environment blocks (drift gate)
	@echo "==> Checking the bundled-pack environment seed against specs/ (cargo xtask)"
	cd $(ROOT) && cargo xtask gen-bundled-environments --check

xtask-gen-editor-dialects: ## Verify editor selectable dialect lists match DialectProfile::all
	@echo "==> Checking generated editor dialect lists (cargo xtask)"
	cd $(ROOT) && cargo xtask gen-editor-dialects --check

xtask-editor-extensions: ## Verify the editors' extension/language lists match the dialect catalog + bundled packs (drift gate)
	@echo "==> Checking editor extension/language lists against the dialect catalog (cargo xtask)"
	cd $(ROOT) && cargo xtask gen-editor-extensions --check

xtask-owner-resolution: ## Verify the shared semantic-owner contract resolves to live source and gates
	@echo "==> Checking shared semantic-owner contract (cargo xtask)"
	cd $(ROOT) && cargo xtask owner-resolution

xtask-workflow-sync: ## Verify .github/workflows/ copies match their canonical deploy sources (drift gate)
	@echo "==> Checking installed workflows match their canonical sources (cargo xtask)"
	cd $(ROOT) && cargo xtask workflow-sync --check

xtask-resolution-drift: ## Flag namespace-blind simple-name scans over all_procs/all_classes (name-resolution drift gate)
	@echo "==> Checking for name-resolution drift (cargo xtask)"
	cd $(ROOT) && cargo xtask resolution-drift

xtask-retired-api-gate: ## Flag code uses of the P1-G-retired dialect/registry APIs (zero-reference gate)
	@echo "==> Checking for retired dialect/registry API uses (cargo xtask)"
	cd $(ROOT) && cargo xtask retired-api-gate

xtask-pack-goldens: ## Verify every shipped .tclspec still loads to its checked-in golden snapshot
	@echo "==> Checking shipped spec-pack golden snapshots (cargo xtask)"
	cd $(ROOT) && cargo xtask pack-goldens --check

xtask-number-drift: ## Check Tcl numeral parser and expression-boundary scanner ownership (numeric-grammar drift gate)
	@echo "==> Checking for numeric-grammar drift (cargo xtask)"
	cd $(ROOT) && cargo xtask number-drift

xtask-dialect-drift: ## Check that document text is re-read under the ingress-resolved grammar only (dialect-grammar drift gate)
	@echo "==> Checking for dialect-grammar drift (cargo xtask)"
	cd $(ROOT) && cargo xtask dialect-drift

xtask-kcs-index-links: ## Validate docs links + design/KCS index coverage
	@echo "==> Checking docs links + index coverage (cargo xtask)"
	cd $(ROOT) && cargo xtask kcs-index-links

xtask-diag-tables: ## Verify docs/generated/ code tables are in sync with the DiagCode catalogue (drift gate)
	@echo "==> Checking generated DiagCode tables are in sync (cargo xtask)"
	cd $(ROOT) && cargo xtask diag-tables --check

xtask-diag-emission-check: ## Verify every non-internal, non-reserved DiagCode has a real emission site (issue #1317)
	@echo "==> Checking every non-reserved diagnostic code has a producer (cargo xtask)"
	cd $(ROOT) && cargo xtask diag-emission-check

xtask-gen-editor-catalogs: ## Verify the Zed/VS Code editor catalogs are in sync with the registry (drift gate)
	@echo "==> Checking generated editor catalogs are in sync (cargo xtask)"
	cd $(ROOT) && cargo xtask gen-editor-catalogs --check

xtask-gen-irule-test-data: ## Verify generated iRule-test Tcl event/mock data is in sync with the registry (drift gate)
	@echo "==> Checking generated iRule-test data is in sync (cargo xtask)"
	cd $(ROOT) && cargo xtask gen-irule-test-data --check

xtask-gen-zed-queries: ## Verify the generated Zed tree-sitter highlight queries are in sync with the registry (drift gate)
	@echo "==> Checking generated Zed highlight queries are in sync (cargo xtask)"
	cd $(ROOT) && cargo xtask gen-zed-queries --check

xtask-gen-tmlanguage-keywords: ## Verify the VS Code/JetBrains/Sublime TextMate grammar keyword lists are in sync with the registry (drift gate)
	@echo "==> Checking generated TextMate grammar keyword lists are in sync (cargo xtask)"
	cd $(ROOT) && cargo xtask gen-tmlanguage-keywords --check

xtask-gen-editor-settings: ## Verify the VS Code diagnosticCatalog.ts is in sync with the DiagCode catalogue (drift gate)
	@echo "==> Checking generated diagnosticCatalog.ts is in sync (cargo xtask)"
	cd $(ROOT) && cargo xtask gen-editor-settings --check

xtask-gen-vscode-package: ## Verify the VS Code package.json tclLsp.* sections are in sync with the registries (drift gate)
	@echo "==> Checking generated package.json sections are in sync (cargo xtask)"
	cd $(ROOT) && cargo xtask gen-vscode-package --check

xtask-gen-jetbrains-catalog: ## Verify the JetBrains Kotlin catalog/settings/panel are in sync with the DiagCode catalogue (drift gate)
	@echo "==> Checking generated JetBrains files are in sync (cargo xtask)"
	cd $(ROOT) && cargo xtask gen-jetbrains-catalog --check

xtask-gen-ai-diagnostics: ## Verify MCP/AI diagnostic catalogues + AI prompt/skill files are in sync with DiagCode (drift gate)
	@echo "==> Checking generated MCP/AI diagnostic files are in sync (cargo xtask)"
	cd $(ROOT) && cargo xtask gen-ai-diagnostics --check

xtask-command-backing: ## Verify the WASM runtime backs every core-Tcl registry command (drift + gap gate)
	@echo "==> Checking WASM command backing coverage is in sync (cargo xtask)"
	cd $(ROOT) && cargo xtask command-backing --check

xtask-callback-inventory: ## Verify executable/callback registry coverage, the authored coverage manifest, and the generated reports (issue #1706)
	@echo "==> Checking executable/callback surface inventory + coverage manifest (cargo xtask)"
	cd $(ROOT) && cargo xtask callback-inventory --check

xtask-f5query-builtins-doc: ## Verify docs/references/f5_query/builtins.md documents exactly the registered f5-query builtins (coverage drift gate, issue #1404)
	@echo "==> Checking f5-query builtins reference doc coverage (cargo xtask)"
	cd $(ROOT) && cargo xtask f5-query-builtins-doc --check

xtask-bigip-data-schema: ## Verify the hand-maintained BIG-IP object-spec data is internally consistent (schema/consistency gate, issue #1404)
	@echo "==> Checking BIG-IP object-spec data consistency (cargo xtask)"
	cd $(ROOT) && cargo xtask bigip-data-schema --check

xtask-audit-option-dialects: ## Regenerate tmp/option_dialect_audit.json from built tclsh trees (on-demand; needs tmp/tcl*/unix)
	@echo "==> Auditing OptionSpec dialect gates (cargo xtask)"
	cd $(ROOT) && cargo xtask audit-option-dialects

xtask-option-registry-drift: ## Verify every option the dialect audit probes is declared in the registry (audit<->registry drift gate)
	@echo "==> Checking the dialect audit matches the registry option surface (cargo xtask)"
	cd $(ROOT) && cargo xtask audit-option-dialects --check

xtask-sslictcl-data: check-source-data ## Verify the embedded SslicTcl source-data bundle (offline)

update-source-data: ## Refresh embedded SslicTcl source data (explicit network-capable operation)
	@echo "==> Updating embedded SslicTcl source data"
	cd $(ROOT) && cargo xtask sslictcl-data update

check-source-data: ## Verify embedded SslicTcl data, provenance, hashes, and generated output (offline)
	@echo "==> Checking embedded SslicTcl source data (offline)"
	cd $(ROOT) && cargo xtask sslictcl-data check $(if $(SOURCE_DATA_MAX_AGE_DAYS),--max-age-days $(SOURCE_DATA_MAX_AGE_DAYS))

xtask-c-api-ownership: check-c-api-ownership ## Verify runtime/rust/src/capi.rs exports each have a row in c-api-ownership-contract.md (drift gate, issue #1404)

check-c-api-ownership: ## Verify the C-API ownership/error contract: capi.rs exports <-> docs/design/runtime/c-api-ownership-contract.md rows (offline; pass TCL_SOURCE=path for the fuller header cross-check)
	@echo "==> Running the C-API ownership checker's own regression tests"
	python3 $(ROOT)scripts/check_c_api_ownership.py --self-test
	@echo "==> Checking C-API ownership contract coverage (offline)"
	python3 $(ROOT)scripts/check_c_api_ownership.py $(if $(TCL_SOURCE),--tcl-source $(TCL_SOURCE))

xtask-registry-oracle: ## Audit the iRules registry against a local BIG-IP extract (IRULES_ORACLE_ROOT=/path/to/bigip-extract)
	@test -n "$(IRULES_ORACLE_ROOT)" || (echo "Set IRULES_ORACLE_ROOT=/path/to/bigip-extract"; exit 2)
	@echo "==> Auditing the iRules registry against the local BIG-IP source oracle (cargo xtask)"
	cd $(ROOT) && cargo xtask registry-oracle --irules-root "$(IRULES_ORACLE_ROOT)" --check

tcltest-sweep: ## Regenerate the VM-vs-C tcltest parity scoreboard (runs the suite through the VM + reference tclsh; slow, on-demand)
	@echo "==> Sweeping the C tcltest suite through the VM + reference tclsh (cargo xtask)"
	cd $(ROOT) && cargo xtask tcltest-sweep --backend both

tcltest-sweep-check: ## Verify the committed tcltest parity scoreboard is in sync (VM re-run vs cached C baseline; slow, nightly)
	@echo "==> Checking the tcltest parity scoreboard is in sync (cargo xtask)"
	cd $(ROOT) && cargo xtask tcltest-sweep --backend vm --check

# Phase targets for parallel prep-pr execution.  (`_prep-pr-smoke` is VSIX
# packaging smoke + standalone-EDA verification — a different thing from the
# `smoke` test tier below; hence the distinct `_prep-pr-smoke-tier` name.)
_prep-pr-checks: lint-ts typecheck-ts check-editor-settings typecheck-report-ts lint-report-ts check-report-assets typecheck-spec-studio-ts lint-spec-studio-ts test-installer
_prep-pr-tests: test-rust
_prep-pr-smoke: smoke-vsix verify-standalone-eda
_prep-pr-smoke-tier: smoke

# Rust-side check gate (fmt + clippy + generated-file drift).  Mirrors the
# pr-gate job in GitHub Actions (ci.yml).
rust-check: check-rust-pr xtask-check ## Rust fmt + clippy + generated-file drift gates (mirrors the GitHub Actions PR gate)

# The local pre-push gate: format + codegen + lint/typecheck + the smoke test
# tier.  Deliberately NOT the full test suite — CI runs the deep suites
# (rust-tests / rust-tests-heavy / lsp-e2e / test-ext / python) on every PR
# and push.  `make test` remains available to reproduce CI locally.
prep-pr: format codegen ## Fast local gate (format + codegen + lint + typecheck + smoke tier) — deep suites run in CI
	@$(MAKE) -j $(NPROC) _prep-pr-checks _prep-pr-smoke-tier

.PHONY: smoke smoke-p test-installer test-exhaustive fuzz test-spectcl-compat

# One fail-closed compatibility lane for the complete SpecTcl contract: legacy
# 1.x sources through TclVM (15), 2.0 golden upgrades (3), live 1.x/2.0 hook
# execution (4), shipped corpus/containment (2), and real-C-Tcl parse validity
# (1). The installer selects release line 9.0, while tcl-dialect's manifest
# supplies and validates its exact patchlevel.
test-spectcl-compat: ensure-tcl90-reference ## Run SpecTcl 1.x/2.0/TclVM/real-Tcl compatibility against the exact pinned Tcl 9.0 oracle
	@set -eu; \
		. scripts/dev/tcl-reference-toolchains.sh; \
		tcl_reference_load_toolchains "$(ROOT)"; \
		tclsh="$$(tcl_reference_resolve_tclsh 9.0)"; \
		TCL_LSP_TCLSH90="$$tclsh" TCL_REQUIRE_SPECTCL_COMPAT=1 \
		cargo test -p tcl-spectcl --test eval_loader --test golden_packs --test pack_source_e2e --test spec_corpus --test pack_is_real_tcl

test-installer: ## Test installer platform, UI, and legacy-migration decisions (no network)
	@bash scripts/install/test_installer.sh
	@bash scripts/release/test_smoke_installer.sh

# Whole-workspace smoke tier: the fastest meaningful per-module subset, run
# locally right after `cargo build`/`cargo check` — seconds warm, NOT a
# substitute for CI's deep suites.  A test is "smoke" by naming convention: a
# unit test fn named `smoke_*` (or nested under `mod smoke`), or an
# integration-test file named `*_smoke.rs` (see .config/nextest.toml's
# [profile.smoke]).  Reuses whatever build already exists (default features,
# dev profile) — deliberately NOT --all-features, so it never forces a
# recompile after a normal `cargo build`.
smoke: ## Fast per-module smoke tier (seconds; run after compile, before push)
	@set -eu; \
	if ! command -v cargo >/dev/null 2>&1; then \
		echo "ERROR: 'cargo' not found on PATH."; exit 1; \
	fi; \
	cd $(ROOT) && \
	if command -v cargo-nextest >/dev/null 2>&1; then \
		echo "==> cargo nextest run --profile smoke"; \
		cargo nextest run --profile smoke --workspace; \
	else \
		echo "==> cargo-nextest not found; falling back to 'cargo test smoke'"; \
		echo "    (install: cargo install cargo-nextest --locked)"; \
		cargo test --workspace smoke; \
	fi

# Single-crate smoke: make smoke-p P=tcl-compiler
smoke-p: ## Smoke tier for one crate: make smoke-p P=<crate-name>
	@set -eu; \
	if [ -z "$${P:-}" ]; then \
		echo "ERROR: usage: make smoke-p P=<crate-name>"; exit 1; \
	fi; \
	cd $(ROOT) && \
	if command -v cargo-nextest >/dev/null 2>&1; then \
		echo "==> cargo nextest run --profile smoke -p $(P)"; \
		cargo nextest run --profile smoke -p "$(P)"; \
	else \
		echo "==> cargo-nextest not found; falling back to 'cargo test -p $(P) smoke'"; \
		cargo test -p "$(P)" smoke; \
	fi

# Manual-only tier: every #[ignore]d corpus sweep, differential-fuzz gate,
# and privileged/environment-gated test in the workspace (see
# docs/design/contracts/test-tiers-and-ci-gates.md).  NEVER wired into
# prep-pr, test, check-all, or CI.
# The corpus tests need the tmp/tcl8.x / tcl9.x / tcllib trees fetched
# (fetch-tcl-source skill / session-start hook); the bpf-tcl privileged tests
# need root + iproute2/bpftool + a live Linux kernel.
test-exhaustive: ## Run every #[ignore]d corpus/fuzz/privileged test (slow, manual-only, never in CI)
	@set -eu; \
	if ! command -v cargo >/dev/null 2>&1; then \
		echo "ERROR: 'cargo' not found on PATH."; exit 1; \
	fi; \
	cd $(ROOT) && \
	if command -v cargo-nextest >/dev/null 2>&1; then \
		echo "==> cargo nextest run --profile exhaustive --run-ignored ignored-only"; \
		cargo nextest run --profile exhaustive --run-ignored ignored-only --workspace --all-features --no-fail-fast; \
	else \
		echo "==> cargo-nextest not found; falling back to 'cargo test -- --ignored'"; \
		cargo test --workspace --all-features --no-fail-fast -- --ignored; \
	fi

# Differential-fuzz campaign entry point.  Campaign shape (engine pair,
# duration, seed replay) is owned by the fuzz-findings skill / tcl-fuzz's own
# CLI — this is a documented front door, manual-only by design.
fuzz: ## Run a tcl-fuzz differential campaign (manual-only; see the fuzz-findings skill)
	@echo "==> See the 'fuzz-findings' skill for engine pairs, seed replay, and the findings registry."; \
	cd $(ROOT) && cargo run -p tcl-fuzz --release -- run --iterations "$${ITERATIONS:-1000}"

# The workspace test suite, including the native lsp_e2e suite
# (rust/tcl-lsp-server/tests/*_e2e.rs).  Set SKIP_TEST_RUST=1 to skip.
#
# Prefers nextest when it is installed, the way `smoke` does, and for the same
# reason CI's rust-tests job uses it: nextest runs every test binary's tests in
# ONE global parallel pool, where plain `cargo test` runs the binaries serially.
# nextest cannot run doctests, so those follow in a second pass — the same split
# the rust-tests job makes.  The `cargo test` fallback keeps the target working
# on a machine without nextest.
test-rust: ## Run Rust workspace tests + the native-server lsp_e2e suite (skip with SKIP_TEST_RUST=1)
	@set -eu; \
	if [ -n "$${SKIP_TEST_RUST:-}" ]; then \
		echo "==> SKIP_TEST_RUST set — skipping Rust tests"; \
		exit 0; \
	fi; \
	if ! command -v cargo >/dev/null 2>&1; then \
		echo "ERROR: 'cargo' not found on PATH (need a current Rust stable toolchain)."; \
		echo "       Set SKIP_TEST_RUST=1 to skip this target."; \
		exit 1; \
	fi; \
	echo "==> Running Rust workspace tests (includes the native lsp_e2e suite)"; \
	cd $(ROOT) && \
	if command -v cargo-nextest >/dev/null 2>&1; then \
		echo "==> cargo nextest run --workspace --all-features"; \
		cargo nextest run --workspace --all-features --no-fail-fast; \
		echo "==> cargo test --doc (nextest cannot run doctests)"; \
		cargo test --workspace --all-features --doc --no-fail-fast; \
	else \
		echo "==> cargo-nextest not found; falling back to 'cargo test'"; \
		echo "    (nextest pools every test binary; plain cargo test runs them serially)"; \
		cargo test --workspace --all-features; \
	fi

# Build the native Rust LSP server binary (target/release/tcl-lsp-server) —
# the only server there is, and the one every harness drives (test-ext points
# the extension at it via TCL_LSP_SERVER_BIN).  Release by default for usable
# latency; pass PROFILE=debug for a faster build (see PROFILE above).
rust-server: ## Build the native Rust LSP server (PROFILE=release|debug)
	@set -eu; \
	if ! command -v cargo >/dev/null 2>&1; then \
		echo "ERROR: 'cargo' not found on PATH (need a current Rust stable toolchain)."; exit 1; \
	fi; \
	echo "==> Building native tcl-lsp-server ($(PROFILE))"; \
	cd $(ROOT) && cargo build -p tcl-lsp-server $(if $(filter release,$(PROFILE)),--release,); \
	echo "==> Built $(ROOT)target/$(PROFILE)/tcl-lsp-server"

# Build the native Rust `tcl` CLI binary (target/release/tcl).  Mirrors
# rust-server: release by default, PROFILE=debug for a faster build.
rust-tcl: ## Build the native Rust `tcl` CLI (PROFILE=release|debug)
	@set -eu; \
	if ! command -v cargo >/dev/null 2>&1; then \
		echo "ERROR: 'cargo' not found on PATH (need a current Rust stable toolchain)."; exit 1; \
	fi; \
	echo "==> Building native tcl CLI ($(PROFILE))"; \
	cd $(ROOT) && cargo build -p tcl-cli $(if $(filter release,$(PROFILE)),--release,); \
	echo "==> Built $(ROOT)target/$(PROFILE)/tcl"

# Build the native Rust `f5-query` CLI binary (target/release/f5-query).
rust-f5: ## Build the native Rust `f5-query` CLI (PROFILE=release|debug)
	@set -eu; \
	if ! command -v cargo >/dev/null 2>&1; then \
		echo "ERROR: 'cargo' not found on PATH (need a current Rust stable toolchain)."; exit 1; \
	fi; \
	echo "==> Building native f5-query CLI ($(PROFILE))"; \
	cd $(ROOT) && cargo build -p f5-cli $(if $(filter release,$(PROFILE)),--release,); \
	echo "==> Built $(ROOT)target/$(PROFILE)/f5-query"

# Build the native Rust MCP server binary (target/release/tcl-mcp).  The repo
# `.mcp.json` (via scripts/tcl-mcp) launches this for Claude Code / Codex.
rust-mcp: ## Build the native Rust `tcl-mcp` MCP server (PROFILE=release|debug)
	@set -eu; \
	if ! command -v cargo >/dev/null 2>&1; then \
		echo "ERROR: 'cargo' not found on PATH (need a current Rust stable toolchain)."; exit 1; \
	fi; \
	echo "==> Building native tcl-mcp server ($(PROFILE))"; \
	cd $(ROOT) && cargo build -p tcl-mcp $(if $(filter release,$(PROFILE)),--release,); \
	echo "==> Built $(ROOT)target/$(PROFILE)/tcl-mcp"

# Build both native Rust CLIs in one go.
rust-clis: rust-tcl rust-f5 ## Build the native Rust `tcl` + `f5-query` CLIs

# Cross-compile + smoke-test tcl-lsp-server for the multi-platform universal
# VSIX.  Every workspace crate is pure Rust, so only the linker varies per
# target (see .cargo/config.toml).  Linux uses QEMU user-mode to smoke foreign
# arches; macOS runs Darwin binaries natively; Windows targets build on a
# Windows runner.

ensure-server-cross-deps: ## Install cross-compile deps (rustup targets + linkers) for this host
	@set -eu; \
	if ! command -v rustup >/dev/null 2>&1; then \
		echo "ERROR: rustup not found — install Rust via rustup (need a current stable toolchain)."; exit 1; \
	fi; \
	case "$(SERVER_UNAME_S)" in \
	Linux) \
		echo "==> Adding Linux cross targets"; \
		rustup target add aarch64-unknown-linux-gnu riscv64gc-unknown-linux-gnu >/dev/null 2>&1 || true; \
		if ! command -v aarch64-linux-gnu-gcc >/dev/null 2>&1 || ! command -v riscv64-linux-gnu-gcc >/dev/null 2>&1; then \
			echo "==> Installing cross-linkers (sudo apt-get)"; \
			sudo apt-get install -y gcc-aarch64-linux-gnu gcc-riscv64-linux-gnu; \
		fi; \
		if ! command -v qemu-aarch64 >/dev/null 2>&1; then \
			echo "==> Installing qemu-user"; \
			sudo apt-get install -y qemu-user; \
		fi; \
		;; \
	Darwin) \
		echo "==> Adding Darwin cross targets"; \
		rustup target add aarch64-apple-darwin x86_64-apple-darwin >/dev/null 2>&1 || true; \
		echo "  (macOS uses the system cc — no extra linker or QEMU needed)"; \
		;; \
	*) \
		echo "==> Adding Windows cross targets"; \
		rustup target add x86_64-pc-windows-msvc aarch64-pc-windows-msvc >/dev/null 2>&1 || true; \
		;; \
	esac; \
	echo "==> Cross-compile deps ready for $(SERVER_UNAME_S)"

server-cross-build: ## Cross-compile tcl-lsp-server for this host's native + cross targets
	@set -eu; \
	if ! command -v cargo >/dev/null 2>&1; then echo "ERROR: cargo not found."; exit 1; fi; \
	echo "==> Cross-compiling tcl-lsp-server: $(SERVER_TARGETS_HOST)"; \
	for t in $(SERVER_TARGETS_HOST); do \
		echo "  building $$t..."; \
		cd $(ROOT) && cargo build -p tcl-lsp-server --release --target $$t --quiet; \
	done; \
	echo "==> Done"

server-cross-build-all: ## Cross-compile tcl-lsp-server for all 7 targets (CI fan-in)
	@set -eu; \
	if ! command -v cargo >/dev/null 2>&1; then echo "ERROR: cargo not found."; exit 1; fi; \
	echo "==> Cross-compiling tcl-lsp-server for all targets: $(SERVER_TARGETS_ALL)"; \
	for t in $(SERVER_TARGETS_ALL); do \
		echo "  building $$t..."; \
		cd $(ROOT) && cargo build -p tcl-lsp-server --release --target $$t --quiet \
			|| { echo "    (skipped $$t — not buildable on this host)"; continue; }; \
	done; \
	echo "==> Done"

# Cross-compile the native `tcl-mcp` MCP server for all release targets — local
# parity with the CI build-server-matrix job that publishes the per-triple
# `tcl-mcp-<triple>` release assets fetched by install.sh / the launcher.
mcp-cross-build-all: ## Cross-compile tcl-mcp for all 7 targets (release-asset parity)
	@set -eu; \
	if ! command -v cargo >/dev/null 2>&1; then echo "ERROR: cargo not found."; exit 1; fi; \
	echo "==> Cross-compiling tcl-mcp for all targets: $(SERVER_TARGETS_ALL)"; \
	for t in $(SERVER_TARGETS_ALL); do \
		echo "  building $$t..."; \
		cd $(ROOT) && cargo build -p tcl-mcp --release --target $$t --quiet \
			|| { echo "    (skipped $$t — not buildable on this host)"; continue; }; \
	done; \
	echo "==> Done"

# Cross-compile the native `tcl` + `f5-query` CLIs for all release targets —
# local parity with the CI build-server-matrix job that publishes the per-triple
# `tcl-<triple>` / `f5-query-<triple>` release assets.
cli-cross-build-all: ## Cross-compile tcl + f5-query for all 7 targets (release-asset parity)
	@set -eu; \
	if ! command -v cargo >/dev/null 2>&1; then echo "ERROR: cargo not found."; exit 1; fi; \
	echo "==> Cross-compiling tcl + f5-query for all targets: $(SERVER_TARGETS_ALL)"; \
	for t in $(SERVER_TARGETS_ALL); do \
		echo "  building $$t..."; \
		cd $(ROOT) && cargo build -p tcl-cli -p f5-cli --release --target $$t --quiet \
			|| { echo "    (skipped $$t — not buildable on this host)"; continue; }; \
	done; \
	echo "==> Done"

print-server-targets-all: ## Print the full set of native-server target triples (CI helper)
	@echo $(SERVER_TARGETS_ALL)

print-server-targets-jetbrains: ## Print the JetBrains-eligible target triples — SERVER_TARGETS_ALL minus riscv64 (CI helper)
	@echo $(SERVER_TARGETS_JETBRAINS)

server-cross-test: ## Smoke-test built tcl-lsp-server binaries (QEMU on Linux, native on macOS)
	@bash $(ROOT)scripts/test-cross-server.sh

server-cross-test-build: ## Cross-build then smoke-test tcl-lsp-server binaries
	@bash $(ROOT)scripts/test-cross-server.sh --build

## Full lint + typecheck across every language (TS, Rust).
## Tests are NOT included here — run them separately (test-ext, test-rust,
## runtime-rust-test, test-emacs) before PR creation.

# CI-facing Rust fast gate: the root workspace plus the standalone runtime.
# `check-rust` reuses this target before checking the remaining excluded
# crates, so neither CI nor the broader local gate duplicates these commands.
check-rust-pr: ensure-rust-deps
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
		echo "ERROR: 'cargo' not found on PATH (need a current Rust stable toolchain)."; \
		echo "       Set SKIP_CHECK_RUST=1 to skip."; \
		exit 1; \
	fi; \
	$(MAKE) --no-print-directory -C $(ROOT) _check-rust-pr

_check-rust-pr:
	@set -eu; \
	echo "==> Checking top-level Rust workspace (fmt + clippy)"; \
	cd $(ROOT); \
	cargo fmt --all --check; \
	cargo clippy --workspace --all-targets -- -D warnings; \
	if [ -f "$(RUNTIME_RUST_DIR)/Cargo.toml" ]; then \
		echo "==> Checking runtime/rust (fmt + clippy)"; \
		$(MAKE) --no-print-directory -C $(ROOT) runtime-rust-lint; \
	fi

# Broader local Rust gate: run the CI-facing workspace/runtime contract above,
# then every other crate excluded from the root workspace (Zed and the wasm32
# cdylibs, which have their own targets and lockfiles). Skip with
# SKIP_CHECK_RUST=1.
check-rust: ensure-rust-deps ## Rust fmt-check + clippy on the workspace and excluded standalone crates
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
		echo "ERROR: 'cargo' not found on PATH (need a current Rust stable toolchain)."; \
		echo "       Set SKIP_CHECK_RUST=1 to skip."; \
		exit 1; \
	fi; \
	. $(ROOT)scripts/dev/wasm-cc-env.sh; \
	wasm_cc_prepare; \
	$(MAKE) --no-print-directory -C $(ROOT) _check-rust-pr; \
	if [ -f "$(ZED_DIR)/Cargo.toml" ]; then \
		echo "==> Checking Zed extension (fmt + clippy --target wasm32-wasip2 + host tests)"; \
		cd $(ZED_DIR); \
		cargo fmt --all --check; \
		cargo clippy --target wasm32-wasip2 --all-targets -- -D warnings; \
		cargo test --lib; \
	fi; \
	if [ -f "$(EXPLORER_WASM_DIR)/Cargo.toml" ] && \
			rustup target list --installed 2>/dev/null | grep -q wasm32-unknown-unknown; then \
		echo "==> Checking tcl-explorer-wasm (fmt + clippy --target wasm32-unknown-unknown)"; \
		cd $(EXPLORER_WASM_DIR); \
		cargo fmt --all --check; \
		cargo clippy --target wasm32-unknown-unknown --all-targets -- -D warnings; \
	fi; \
	if [ -f "$(ROOT)rust/tcl-vm-wasm/Cargo.toml" ] && \
			rustup target list --installed 2>/dev/null | grep -q wasm32-unknown-unknown; then \
		echo "==> Checking tcl-vm-wasm (fmt + clippy --target wasm32-unknown-unknown)"; \
		cd $(ROOT)rust/tcl-vm-wasm; \
		cargo fmt --all --check; \
		cargo clippy --target wasm32-unknown-unknown --all-targets -- -D warnings; \
	fi; \
	if [ -f "$(ROOT)rust/tcl-spec-studio-wasm/Cargo.toml" ] && \
			rustup target list --installed 2>/dev/null | grep -q wasm32-unknown-unknown; then \
		echo "==> Checking tcl-spec-studio-wasm (fmt + clippy --target wasm32-unknown-unknown)"; \
		cd $(ROOT)rust/tcl-spec-studio-wasm; \
		cargo fmt --all --check; \
		cargo clippy --target wasm32-unknown-unknown --all-targets -- -D warnings; \
	fi; \
	if [ -f "$(ROOT)rust/tcl-lsp-server-wasm/Cargo.toml" ] && \
			rustup target list --installed 2>/dev/null | grep -q wasm32-unknown-unknown; then \
		echo "==> Checking tcl-lsp-server-wasm (fmt + clippy --target wasm32-unknown-unknown)"; \
		cd $(ROOT)rust/tcl-lsp-server-wasm; \
		cargo fmt --all --check; \
		cargo clippy --target wasm32-unknown-unknown --all-targets -- -D warnings; \
	fi; \
	if [ -f "$(ROOT)rust/tcl-lsp-server-wasi/Cargo.toml" ] && \
			rustup target list --installed 2>/dev/null | grep -q wasm32-wasip1; then \
		echo "==> Checking tcl-lsp-server-wasi (fmt + clippy --target wasm32-wasip1)"; \
		cd $(ROOT)rust/tcl-lsp-server-wasi; \
		cargo fmt --all --check; \
		cargo clippy --target wasm32-wasip1 --all-targets -- -D warnings; \
	fi

# Supply-chain audit for every tracked Cargo lockfile: RustSec advisories,
# license policy, banned/duplicate crates, and source allowlist — all four
# checks configured in the repo-root deny.toml.  The root workspace excludes
# several independently locked WASM, WASI, runtime, editor, and extension
# crates, so auditing only Cargo.toml would leave their resolved graphs unseen.
# CI runs this same helper after installing the pinned cargo-deny binary.
rust-deny: ## Audit every locked Rust workspace with cargo-deny (advisories/licenses/bans/sources via deny.toml)
	@set -eu; \
	if ! command -v cargo >/dev/null 2>&1; then \
		echo "ERROR: 'cargo' not found on PATH (need a current Rust stable toolchain)."; exit 1; \
	fi; \
	if ! cargo deny --version >/dev/null 2>&1; then \
		echo "==> cargo-deny not found — installing (cargo install cargo-deny)"; \
		cargo install cargo-deny --locked; \
	fi; \
	echo "==> Auditing every locked Rust workspace with cargo-deny (deny.toml)"; \
	cd $(ROOT) && bash scripts/dev/cargo-deny-all.sh

# All-languages lint + typecheck.  Mirrors GitHub Actions' pr-gate plus the
# extra languages CI doesn't cover (Rust, full TS).
check-all: ## Full lint + typecheck (TS, Rust, Python)
	@$(MAKE) -j $(NPROC) _prep-pr-checks check-rust xtask-workflow-sync lint-py typecheck-py
	@echo "==> check-all: PASSED"

ensure-test-deps: ## Install optional host test deps for the host platform
	@bash $(ROOT)scripts/dev/ensure-test-deps.sh

install-test-deps: ## Install EVERYTHING the full test suite needs (system toolchain) on Debian/Ubuntu, Fedora/CentOS/RHEL, or macOS Homebrew
	@echo "==> install-test-deps: installing system toolchain"
	@bash $(ROOT)scripts/dev/ensure-test-deps.sh
	@echo "==> install-test-deps: done — run 'make check-all test-ext test-rust runtime-rust-test test-emacs' next"

ensure-tcl-deps: ## Install Tcl shells needed by Tcl/tclpkg tests and bytecode capture
	@env \
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
		SKIP_WASI_SDK=1 \
		SKIP_PYTHON_TK=1 \
		SKIP_UV=1 \
		SKIP_TCLLIB=1 \
		bash $(ROOT)scripts/dev/ensure-test-deps.sh

# Tcl 9.0 is the shared gold-standard reference lane for the deep Rust and
# SpecTcl suites. The release manifest owns its exact patchlevel and source tag;
# callers select only the release line through this entry point.
ensure-tcl90-reference: ## Install the exact manifest-pinned Tcl 9.0 reference interpreter
	@$(MAKE) TCL_LSP_TCL_RELEASES=9.0 ensure-tcl-deps

ensure-rust-deps: ## Install Rust targets and macOS's wasm32-capable C compiler needed by check-rust
	@if [ -n "$${SKIP_CHECK_RUST:-}" ] || [ -n "$${SKIP_RUST:-}" ]; then \
		echo "==> Rust dependency install skipped"; \
	else \
		env \
			SKIP_TCLSH=1 \
			SKIP_NODE=1 \
			SKIP_KOTLINC=1 \
			SKIP_WASMTIME=1 \
			SKIP_BINARYEN=1 \
			SKIP_EMACS=1 \
			SKIP_XVFB=1 \
			SKIP_TSHARK=1 \
			SKIP_OPENSSL=1 \
			SKIP_PING=1 \
			SKIP_RGXG=1 \
			$(if $(filter Darwin,$(SERVER_UNAME_S)),,SKIP_WASI_SDK=1) \
			SKIP_PYTHON_TK=1 \
			SKIP_UV=1 \
			SKIP_TCLLIB=1 \
			bash $(ROOT)scripts/dev/ensure-test-deps.sh; \
		echo "==> Ensuring wasm32-unknown-unknown target (compiler-explorer WASM)"; \
		rustup target add wasm32-unknown-unknown >/dev/null 2>&1 || true; \
		command -v wasm-pack >/dev/null 2>&1 || \
			echo "    note: wasm-pack not found — 'cargo install wasm-pack' for 'make explorer-wasm'"; \
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
			SKIP_WASMTIME=1 \
			SKIP_BINARYEN=1 \
			SKIP_XVFB=1 \
			SKIP_TSHARK=1 \
			SKIP_OPENSSL=1 \
			SKIP_PING=1 \
			SKIP_RGXG=1 \
			SKIP_WASI_SDK=1 \
			SKIP_PYTHON_TK=1 \
			SKIP_UV=1 \
			SKIP_TCLLIB=1 \
			bash $(ROOT)scripts/dev/ensure-test-deps.sh; \
	fi

ensure-vscode-test-deps: ## Install xvfb for Linux headless VS Code extension tests
	@if [ -n "$${SKIP_TEST_EXT:-}" ]; then \
		echo "==> ensure-vscode-test-deps: SKIP_TEST_EXT set — skipping xvfb install"; \
		exit 0; \
	fi; \
	env \
		SKIP_TCLSH=1 \
		SKIP_NODE=1 \
		SKIP_KOTLINC=1 \
		SKIP_RUST=1 \
		SKIP_WASMTIME=1 \
		SKIP_BINARYEN=1 \
		SKIP_EMACS=1 \
		SKIP_TSHARK=1 \
		SKIP_OPENSSL=1 \
		SKIP_PING=1 \
		SKIP_RGXG=1 \
		SKIP_WASI_SDK=1 \
		SKIP_PYTHON_TK=1 \
		SKIP_UV=1 \
		SKIP_TCLLIB=1 \
		bash $(ROOT)scripts/dev/ensure-test-deps.sh

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

# ---------------------------------------------------------------------------
# VSIX smoke test
# ---------------------------------------------------------------------------

smoke-vsix: compile ## Build and verify the VSIX packages without error
	@echo "==> Smoke-testing VSIX build (native server only: $(SERVER_TARGET_NATIVE))"
	@set -eu; \
	if ! command -v cargo >/dev/null 2>&1 || [ -z "$(SERVER_TARGET_NATIVE)" ]; then \
		echo "ERROR: cargo/rustc are required to build the native tcl-lsp-server for the VSIX smoke."; \
		exit 1; \
	fi; \
	echo "==> Building native tcl-lsp-server ($(SERVER_TARGET_NATIVE))"; \
	cd $(ROOT) && cargo build -p tcl-lsp-server --release --target $(SERVER_TARGET_NATIVE) --quiet
	$(MAKE) package-vsix BUNDLED_TARGETS="$(SERVER_TARGET_NATIVE)"

# npm / TypeScript

npm-env: $(NPM_STAMP) ## Install/update npm dependencies

$(NPM_STAMP): $(EXT_DIR)/package.json
	@echo "==> Installing npm dependencies"
	cd $(EXT_DIR) && $(NPM) install
	@mkdir -p $(STAMP_DIR)
	@touch $@

$(REPORT_NPM_STAMP): $(REPORT_SHARED_DIR)/package.json
	@echo "==> Installing shared report front-end npm dependencies"
	cd $(REPORT_SHARED_DIR) && $(NPM) install
	@mkdir -p $(STAMP_DIR)
	@touch $@

$(SPEC_STUDIO_NPM_STAMP): $(SPEC_STUDIO_WEB)/package.json
	@echo "==> Installing spec studio front-end npm dependencies"
	cd $(SPEC_STUDIO_WEB) && $(NPM) install
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

# The ai/prompts/*.md system prompts are GENERATED (not checked in) by
# `cargo xtask gen-ai-diagnostics` from ai/shared/diagnostics.json +
# ai/prompts/*.j2 + ai/prompts/manifest.json, so the copy rules below have a
# source to copy for the VSIX build.
AI_PROMPT_SRCS  := $(ROOT)ai/shared/diagnostics.json $(ROOT)ai/prompts/manifest.json \
	$(wildcard $(ROOT)ai/prompts/*.j2)

# tcl_system.md / irules_system.md are GENERATED from the .j2 templates by
# xtask; tk_system.md is NOT — it is a static domain-knowledge prompt that lives
# in ai/claude/skills/_prompts/, so it is not produced here (see CANONICAL_TK_MD
# below).
#
# One xtask run emits both prompts.  GNU Make's grouped-target syntax (`&:`)
# says that directly, but macOS ships Make 3.81, which parses the `&` as a third
# target and attaches the recipe to every output — so `make -j` launches one
# xtask per output.  Expressing it as a primary output plus a dependent one
# keeps the single-run guarantee on every Make version.
$(ROOT)ai/prompts/tcl_system.md: $(AI_PROMPT_SRCS)
	@echo "==> Generating AI system prompts (cargo xtask gen-ai-diagnostics)"
	cd $(ROOT) && cargo xtask gen-ai-diagnostics

# Written by the run above; the touch only settles mtime ordering so the copy
# rules below don't re-fire on every build.
$(ROOT)ai/prompts/irules_system.md: $(ROOT)ai/prompts/tcl_system.md
	@touch $@

$(CANONICAL_IRULES_MD): $(ROOT)ai/prompts/irules_system.md
	@mkdir -p $(CANONICAL_DIR)
	@echo "==> Copying canonical irules_system.md"
	cp $< $@

$(CANONICAL_TCL_MD): $(ROOT)ai/prompts/tcl_system.md
	@mkdir -p $(CANONICAL_DIR)
	@echo "==> Copying canonical tcl_system.md"
	cp $< $@

# Static prompt (no .j2), relocated to ai/claude/skills/_prompts/ by the
# skills-off-Python migration (d3d5c4f74). Copy it from its real location.
$(CANONICAL_TK_MD): $(ROOT)ai/claude/skills/_prompts/tk_system.md
	@mkdir -p $(CANONICAL_DIR)
	@echo "==> Copying canonical tk_system.md"
	cp $< $@

compile: $(OUT_DIR)/extension.js ## Compile the TypeScript extension

$(OUT_DIR)/extension.js: $(TS_SRCS) $(EXT_DIR)/tsconfig.json $(NPM_STAMP) $(CANONICAL_DIAG) $(CANONICAL_MANIFEST) $(CANONICAL_IRULES_MD) $(CANONICAL_TCL_MD) $(CANONICAL_TK_MD)
	@echo "==> Compiling TypeScript"
	cd $(EXT_DIR) && $(TSC) -p ./
	@mkdir -p $(OUT_DIR)/chat/canonical
	@cp $(CANONICAL_DIR)/* $(OUT_DIR)/chat/canonical/
	@cp $(EXPLORER_STATIC)/explorer-core.js $(OUT_DIR)/explorer-core.js
	@# Mermaid for the iRule diagram webview. Bundled rather than pulled from a
	@# CDN at render time, so the panel works offline and needs no remote origin
	@# in its CSP. Downloaded on demand by the $(MERMAID_JS) rule.
	@if [ ! -f $(MERMAID_JS) ]; then $(MAKE) --no-print-directory $(MERMAID_JS); fi
	@cp $(MERMAID_JS) $(OUT_DIR)/mermaid.min.js
	@# Bundle the Rust → WASM explorer module so the webview compiles in-process
	@# (no LSP roundtrip). Best-effort: built by `make explorer-wasm`; when it is
	@# absent the webview degrades to host-brokered compilation.
	@if [ -f $(EXPLORER_STATIC)/tcl_explorer_wasm.js ] && [ -f $(EXPLORER_STATIC)/tcl_explorer_wasm_bg.wasm ]; then \
		cp $(EXPLORER_STATIC)/tcl_explorer_wasm.js $(OUT_DIR)/tcl_explorer_wasm.js; \
		cp $(EXPLORER_STATIC)/tcl_explorer_wasm_bg.wasm $(OUT_DIR)/tcl_explorer_wasm_bg.wasm; \
		echo "==> Bundled tcl-explorer-wasm into $(OUT_DIR)"; \
	else \
		echo "==> tcl-explorer-wasm not built — webview will use host-brokered compile (run 'make explorer-wasm')"; \
	fi
	@# Stage the browser language server into editors/vscode/dist/web so a local
	@# web run (`npm run test:web`, `vscode-test-web`) finds it.  Best-effort:
	@# `make package-vsix` stages it itself from the same source and
	@# `verify-vsix` asserts the result, so packaging never depends on this.
	@cd $(EXT_DIR) && node scripts/copy-web-assets.cjs

# Generated editor catalogs
#
# Depends on: the Rust generator (xtask) + the command/event registries.
# tcl-registry is the source of truth; the catalog carries the full command
# surface, including Tk.
REGISTRY_SRCS := $(shell find $(ROOT)rust/tcl-registry/src $(ROOT)rust/xtask/src -name '*.rs')
_CATALOG_DEPS := $(REGISTRY_SRCS)

# One xtask run emits all three catalogs — see the AI-prompt rules above for why
# this is spelled as a primary output plus dependants rather than `&:`.
editors/zed/src/generated/tcl_commands.json: $(_CATALOG_DEPS)
	@echo "==> Generating editor catalogs (cargo xtask)"
	cd $(ROOT) && cargo xtask gen-editor-catalogs

editors/zed/src/generated/irule_events.json editors/vscode/src/generated/iruleEvents.json: editors/zed/src/generated/tcl_commands.json
	@touch $@

editors/zed/languages/tcl/highlights.scm: $(_CATALOG_DEPS)
	@echo "==> Generating Zed tree-sitter highlight queries (cargo xtask)"
	cd $(ROOT) && cargo xtask gen-zed-queries

# These projections are owned by the crates their generators consume.  Keep
# the ownership explicit so a `make generate` after a dialect, lexical grammar,
# or command-registry change always refreshes every affected editor surface.
_EDITOR_DIALECT_DEPS := $(shell find $(ROOT)rust/tcl-dialect/src $(ROOT)rust/xtask/src -name '*.rs')
_EDITOR_DIALECT_OUTPUTS := editors/vscode/package.json editors/vscode/src/extension.ts editors/jetbrains/src/main/kotlin/com/tcllsp/jetbrains/settings/TclLspSettings.kt editors/sublime-text/plugin.py editors/sublime-text/README.md editors/sublime-text/sublime-package.json
$(_EDITOR_DIALECT_OUTPUTS): $(_EDITOR_DIALECT_DEPS)

_TMLANGUAGE_KEYWORD_DEPS := $(shell find $(ROOT)rust/tcl-registry/src $(ROOT)rust/tcl-dialect/src $(ROOT)rust/tcl-syntax/src $(ROOT)rust/xtask/src -name '*.rs')
_TMLANGUAGE_KEYWORD_OUTPUTS := editors/vscode/syntaxes/tcl.tmLanguage.json editors/jetbrains/src/main/resources/syntaxes/tcl.tmLanguage.json editors/sublime-text/Tcl.sublime-syntax
$(_TMLANGUAGE_KEYWORD_OUTPUTS): $(_TMLANGUAGE_KEYWORD_DEPS)

generate: editors/zed/src/generated/tcl_commands.json editors/zed/languages/tcl/highlights.scm $(_EDITOR_DIALECT_OUTPUTS) $(_TMLANGUAGE_KEYWORD_OUTPUTS) gen-irule-test-data ## Regenerate editor catalogs, dialect projections, lexical grammars, and iRule-test data
	@echo "==> Generating the bundled-pack environment seed (cargo xtask)"
	cd $(ROOT) && cargo xtask gen-bundled-environments
	@echo "==> Generating editor dialect projections (cargo xtask)"
	cd $(ROOT) && cargo xtask gen-editor-dialects
	@echo "==> Generating TextMate keyword grammars (cargo xtask)"
	cd $(ROOT) && cargo xtask gen-tmlanguage-keywords

gen-irule-test-data: ## Generate iRule-test Tcl assets from the Rust registries
	@echo "==> Generating iRule-test data (cargo xtask)"
	cd $(ROOT) && cargo xtask gen-irule-test-data

check-generated: ## Verify generated catalogs are up to date
	@echo "==> Checking generated editor catalogs are up to date (cargo xtask)"
	cd $(ROOT) && cargo xtask gen-editor-catalogs --check
	cd $(ROOT) && cargo xtask gen-zed-queries --check

# Generated editor settings from the Rust code registry
#
# Depends on: the Rust registries (source of truth) + the Jinja2 templates
# the xtask generators render.
SETTINGS_J2   := $(wildcard docs/generated/*.j2 editors/vscode/src/generated/*.j2 editors/jetbrains/src/main/kotlin/com/tcllsp/jetbrains/settings/generated/*.j2 ai/prompts/*.j2 ai/claude/skills/*/*.j2)
_SETTINGS_DEPS := $(SETTINGS_J2)

editors/vscode/src/generated/diagnosticCatalog.ts: $(_SETTINGS_DEPS) $(REGISTRY_SRCS)
	@echo "==> Generating editor settings + AI files (cargo xtask, from the Rust registries)"
	cd $(ROOT) && cargo xtask gen-editor-settings
	cd $(ROOT) && cargo xtask gen-vscode-package
	cd $(ROOT) && cargo xtask gen-jetbrains-catalog
	cd $(ROOT) && cargo xtask gen-ai-diagnostics

gen-editor-settings: editors/vscode/src/generated/diagnosticCatalog.ts ## Regenerate editor diagnostic/optimiser settings from code registry

check-editor-settings: ## Verify editor settings match code registry
	@echo "==> Checking editor settings + AI files are up to date"
	cd $(ROOT) && cargo xtask gen-editor-settings --check
	cd $(ROOT) && cargo xtask gen-vscode-package --check
	cd $(ROOT) && cargo xtask gen-jetbrains-catalog --check
	cd $(ROOT) && cargo xtask gen-ai-diagnostics --check

# Logo assets — rasterise the canonical SVG logos to the shipped PNGs

logo: ## Render docs/*.svg logos to the committed 8-bit PNGs (light + dark)
	@echo "==> Rendering logo PNGs from docs/tcl-lsp-logo*.svg"
	bash $(ROOT)scripts/build/render_logo.sh

# Unified codegen — regenerate ALL generated files from registries

codegen: generate gen-editor-settings ## Regenerate ALL generated files (catalogs + editor settings + AI prompts + iRule-test data)

# Compiler Explorer (WASM GUI)
#
# The GUI is a static web app embedded into the `tcl` binary: a checked-in shell
# (`rust/tcl-cli/gui/index.html` + `explorer-core.js` + `worker.js` + assets)
# plus the Rust → WASM compiler core and Mermaid, which the targets below build
# into that same dir. `build.rs` then embeds the whole bundle, and
# `tcl explore --serve` serves it from memory — no CDN, no network at runtime.

MERMAID_VERSION  := 11
MERMAID_JS       := $(EXPLORER_STATIC)/mermaid.min.js
MERMAID_CDN      := https://cdn.jsdelivr.net/npm/mermaid@$(MERMAID_VERSION)/dist/mermaid.min.js

$(MERMAID_JS):
	@echo "==> Downloading Mermaid.js $(MERMAID_VERSION)"
	curl -fSL -o $@ $(MERMAID_CDN)

EXPLORER_WASM_DIR := $(ROOT)rust/tcl-explorer-wasm

explorer-wasm: ## Build the Rust → WASM compiler-explorer core into the tcl GUI dir
	@command -v wasm-pack >/dev/null 2>&1 || { \
		echo "wasm-pack not found — run 'make ensure-rust-deps' or 'cargo install wasm-pack'"; \
		exit 1; }
	@set -eu; \
	. $(ROOT)scripts/dev/wasm-cc-env.sh; \
	wasm_cc_prepare; \
	echo "==> Building tcl-explorer-wasm (wasm-pack --target no-modules)"; \
	cd $(EXPLORER_WASM_DIR) && wasm-pack build --target no-modules --release \
		--out-dir $(BUILD_DIR)/explorer-wasm --out-name tcl_explorer_wasm
	@# wasm-opt is intentionally NOT run (also disabled inside wasm-pack via
	@# tcl-explorer-wasm/Cargo.toml): on modern rustc layouts, binaryen 120
	@# rebinds the `__wbindgen_externrefs` export from the growable externref
	@# table onto the fixed-size funcref table, so `Table.grow` throws at runtime
	@# ("could not grow the table") and the GUI never initialises. The raw
	@# wasm-bindgen output has the correct binding; gzipped it is within a few KB
	@# of the -O3 output, so Pages serves essentially the same bytes.
	cp $(BUILD_DIR)/explorer-wasm/tcl_explorer_wasm_bg.wasm $(EXPLORER_STATIC)/tcl_explorer_wasm_bg.wasm
	cp $(BUILD_DIR)/explorer-wasm/tcl_explorer_wasm.js $(EXPLORER_STATIC)/tcl_explorer_wasm.js
	@# Guard against a silent regression of the above: assert the externref
	@# table can actually grow (best-effort — needs node, present in CI).
	@command -v node >/dev/null 2>&1 \
		&& node $(ROOT)scripts/verify-wasm-externref.mjs $(EXPLORER_STATIC)/tcl_explorer_wasm_bg.wasm \
		|| echo "    note: node not found — skipping wasm growability check"
	@ls -lh $(EXPLORER_STATIC)/tcl_explorer_wasm_bg.wasm

explorer-build: explorer-wasm spec-studio-wasm $(MERMAID_JS) $(ROOT)rust/web-shared/site-update.js ## Build the compiler-explorer GUI bundle (Rust → WASM, offline)
	cp $(ROOT)rust/web-shared/site-update.js $(EXPLORER_STATIC)/site-update.js
	rm -rf $(EXPLORER_STATIC)/editor
	mkdir -p $(EXPLORER_STATIC)/editor
	cp -R $(ROOT)rust/tcl-spec-studio-wasm/dist/assets $(EXPLORER_STATIC)/editor/assets
	cp -R $(ROOT)rust/tcl-spec-studio-wasm/dist/lsp $(EXPLORER_STATIC)/editor/lsp
	cp $(ROOT)rust/tcl-spec-studio-wasm/dist/build-info.json $(EXPLORER_STATIC)/editor/build-info.json
	@echo "==> Compiler explorer bundle ready in $(EXPLORER_STATIC) — rebuild the tcl binary to embed it"

TCL_VM_WASM_DIR := $(ROOT)rust/tcl-vm-wasm

.PHONY: tcl-vm-wasm
tcl-vm-wasm: ## Build the bytecode VM as a self-contained wasm32 cdylib (the primary wasm compile target)
	@rustup target list --installed 2>/dev/null | grep -q wasm32-unknown-unknown \
		|| rustup target add wasm32-unknown-unknown
	@set -eu; \
	. $(ROOT)scripts/dev/wasm-cc-env.sh; \
	wasm_cc_prepare; \
	echo "==> Building tcl-vm-wasm (VM + compiler → wasm32-unknown-unknown cdylib, no imports/WASI)"; \
	cd $(TCL_VM_WASM_DIR) && cargo build --release --target wasm32-unknown-unknown
	@mkdir -p $(BUILD_DIR)/tcl-vm-wasm
	cp $(TCL_VM_WASM_DIR)/target/wasm32-unknown-unknown/release/tcl_vm_wasm.wasm \
		$(BUILD_DIR)/tcl-vm-wasm/vm.wasm
	@# Best-effort self-check: run a coroutine script through the module under node.
	@command -v node >/dev/null 2>&1 \
		&& node $(TCL_VM_WASM_DIR)/verify.mjs $(BUILD_DIR)/tcl-vm-wasm/vm.wasm \
		|| echo "    note: node not found — skipping vm.wasm run check"
	@ls -lh $(BUILD_DIR)/tcl-vm-wasm/vm.wasm

.PHONY: report-wasm
report-wasm: ## Build the in-browser BIG-IP report generator (Rust → WASM) into rust/bigip-report-gen/wasm/dist/
	@command -v wasm-bindgen >/dev/null 2>&1 || { \
		echo "wasm-bindgen not found — 'cargo install wasm-bindgen-cli --version 0.2.127'"; exit 1; }
	bash $(ROOT)rust/bigip-report-gen/wasm/build-wasm.sh

.PHONY: spec-studio-assets spec-studio-wasm
spec-studio-assets: ## Build the spec studio's TypeScript front-end (src/ -> web/dist/studio.js)
	@echo "==> Building the spec studio front-end (TypeScript)"
	cd $(SPEC_STUDIO_WEB) && $(NPM) ci && $(NPM) run build

# Depends on `lsp-server-wasm`: the studio's Pack DSL and Test panes are Monaco
# backed by the *real* language server, whose worker + wasm build-wasm.sh copies
# into `dist/lsp/`. Without it the target fails loudly rather than shipping a
# page whose editor can never analyse anything.
spec-studio-wasm: spec-studio-assets lsp-server-wasm ## Build the command-registry spec studio (Rust → WASM) into rust/tcl-spec-studio-wasm/dist/
	@command -v wasm-bindgen >/dev/null 2>&1 || { \
		echo "wasm-bindgen not found — 'cargo install wasm-bindgen-cli'"; exit 1; }
	bash $(ROOT)rust/tcl-spec-studio-wasm/build-wasm.sh

.PHONY: spec-studio-test spec-studio-boot
spec-studio-test: ## Unit-test the spec studio front-end (node's test runner)
	cd $(SPEC_STUDIO_WEB) && $(NPM) ci && $(NPM) test

# Serves the assembled dist over HTTP and drives it in headless chromium: the
# registry loads, Monaco mounts on both editor tabs, and the language server
# worker reaches "initialized". Skips (exit 0) where playwright is not
# installed, so it is safe to call from a build that may run anywhere.
spec-studio-boot: ## Boot the assembled spec studio in headless chromium
	@command -v node >/dev/null 2>&1 || { \
		echo "node not found — run 'make ensure-test-deps'"; exit 1; }
	node $(ROOT)rust/tcl-spec-studio-wasm/test/boot.mjs

LSP_SERVER_WASM_DIR := $(ROOT)rust/tcl-lsp-server-wasm

.PHONY: lsp-server-wasm lsp-server-wasm-test
lsp-server-wasm: ## Build the LSP server as a browser Web Worker (Rust → WASM) into rust/tcl-lsp-server-wasm/dist/
	@rustup target list --installed 2>/dev/null | grep -q wasm32-unknown-unknown \
		|| rustup target add wasm32-unknown-unknown
	@command -v wasm-bindgen >/dev/null 2>&1 || { \
		echo "wasm-bindgen not found — 'cargo install wasm-bindgen-cli'"; exit 1; }
	bash $(LSP_SERVER_WASM_DIR)/build-wasm.sh

lsp-server-wasm-test: lsp-server-wasm ## Drive the wasm LSP server through a scripted session under node
	@command -v node >/dev/null 2>&1 || { \
		echo "node not found — run 'make ensure-test-deps'"; exit 1; }
	node $(LSP_SERVER_WASM_DIR)/test/e2e.mjs

.PHONY: lsp-server-wasi lsp-server-wasi-test
# The WASI sibling of `lsp-server-wasm`: the same `LspService<Backend>`, driven
# over Content-Length-framed stdio inside a wasm32-wasip1 sandbox instead of
# over `postMessage` in a browser worker.  Unlike the browser build this one
# DOES run `wasm-opt -Os` — see build-wasi.sh for why that is safe here and not
# there.
lsp-server-wasi: ## Build the LSP server as a WASI stdio program (Rust → wasm32-wasip1) into rust/tcl-lsp-server-wasi/dist/
	@rustup target list --installed 2>/dev/null | grep -q wasm32-wasip1 \
		|| rustup target add wasm32-wasip1
	bash $(LSP_SERVER_WASI_DIR)/build-wasi.sh

# The universal VSIX's prerequisite on the module, as a FILE rule rather than a
# dependency on the phony target above.  CI's `build-vsix` downloads the module
# the `lsp-server-wasi` job already built, optimised, and signed; a phony
# prerequisite would rebuild it there (a wasip1 toolchain plus a release LTO
# link) on top of the download.
#
# The prerequisites are what keep "present ⇒ used as-is" honest.  `dist/` is
# git-ignored, so it survives every branch switch: with no prerequisites at all
# the documented laptop fallbacks (`make publish-vsix` / `publish-openvsx`)
# would happily package whatever module an earlier branch left behind.  Naming
# the crate's own sources means a source change re-links and an untouched
# module is reused.  Note this decides only whether cargo is *consulted* — the
# recipe's `cargo build` still fingerprints the whole dependency closure, so a
# change under `rust/tcl-lsp-server` re-links once anything here (or `dist/`)
# is touched.  Delete `dist/` (or run `make lsp-server-wasi`) to force a build.
#
# CI's direction holds: `build-vsix` checks out first and downloads second, and
# `actions/download-artifact` extracts with `unzip-stream`, which never
# restores the zip's stored mtimes — the module lands at download time, newer
# than every prerequisite, so nothing re-links there.
$(LSP_SERVER_WASI_MODULE): $(LSP_SERVER_WASI_SRCS)
	$(MAKE) --no-print-directory lsp-server-wasi

# The fixture directory is preopened by the harness itself (`wasmtime --dir`),
# so the multi-file scenario reads a sourced sibling that only ever exists on
# the host filesystem — the proof that `vfs::NativeStore` works over preopens.
lsp-server-wasi-test: lsp-server-wasi ## Drive the WASI LSP server through scripted sessions under wasmtime
	@command -v node >/dev/null 2>&1 || { \
		echo "node not found — run 'make ensure-test-deps'"; exit 1; }
	@command -v wasmtime >/dev/null 2>&1 || { \
		echo "wasmtime not found — install the wasmtime CLI (v47+)"; exit 1; }
	@echo "==> framing unit tests (wasm32-wasip1, run under wasmtime)"
	cd $(LSP_SERVER_WASI_DIR) && CARGO_TARGET_DIR=$(LSP_SERVER_WASI_DIR)/target \
		cargo test --target wasm32-wasip1
	@echo "==> scripted LSP sessions"
	node $(LSP_SERVER_WASI_DIR)/test/e2e.mjs

compiler-explorer-gui: explorer-build ## Build the GUI bundle and serve it via the native tcl binary
	@echo "==> Building tcl (embeds the GUI) and serving at http://localhost:8080"
	cargo run -p tcl-cli --release --bin tcl -- explore --serve --open
	@ls -lh $(EXPLORER_STATIC)/

claude-skills: $(CLAUDE_SKILLS) ## Build Claude Code skills release zip

# Skills bundle: a plain zip of the (native, MCP-driven) skills tree; the
# domain-knowledge prompts live in skills/_prompts/. Layout:
# tcl-lsp-claude-skills-<v>/skills/<skill>/…, which install.sh extracts into
# ~/.claude/skills/.
$(CLAUDE_SKILLS): $(shell find $(ROOT)ai/claude/skills -type f)
	@echo "==> Building Claude skills release zip"
	@command -v zip >/dev/null 2>&1 || { echo "ERROR: 'zip' not found."; exit 1; }
	@rm -rf $(BUILD_DIR)/claude-skills-stage
	@mkdir -p $(BUILD_DIR)/claude-skills-stage/tcl-lsp-claude-skills-$(VERSION)/skills
	@cp -R $(ROOT)ai/claude/skills/. \
		$(BUILD_DIR)/claude-skills-stage/tcl-lsp-claude-skills-$(VERSION)/skills/
# The spec-author skill's background reading is assembled from the living
# docs at build time — a committed snapshot drifted (the installed 2.1.21
# skill shipped a pre-1.1 syntax memo), so the zip now copies the current
# files and rewrites the SKILL.md paths to the bundled layout.
	@mkdir -p $(BUILD_DIR)/claude-skills-stage/tcl-lsp-claude-skills-$(VERSION)/skills/spec-author/references
	@cp -R $(ROOT)docs/design/spec-dsl-examples \
		$(BUILD_DIR)/claude-skills-stage/tcl-lsp-claude-skills-$(VERSION)/skills/spec-author/references/
	@cp $(ROOT)docs/kcs/kcs-howto-create-command-specs-without-rust.md \
		$(ROOT)docs/kcs/kcs-howto-annotate-commands-with-stubs.md \
		$(BUILD_DIR)/claude-skills-stage/tcl-lsp-claude-skills-$(VERSION)/skills/spec-author/references/
	@cp $(ROOT)docs/design/compiler/command-registry.md \
		$(ROOT)docs/design/contracts/proc-arg-traits.md \
		$(BUILD_DIR)/claude-skills-stage/tcl-lsp-claude-skills-$(VERSION)/skills/spec-author/references/
	@sed -i.bak \
		-e 's|docs/design/spec-dsl-examples|references/spec-dsl-examples|g' \
		-e 's|docs/kcs/kcs-howto-|references/kcs-howto-|g' \
		-e 's|docs/design/compiler/command-registry.md|references/command-registry.md|g' \
		-e 's|docs/design/contracts/proc-arg-traits.md|references/proc-arg-traits.md|g' \
		$(BUILD_DIR)/claude-skills-stage/tcl-lsp-claude-skills-$(VERSION)/skills/spec-author/SKILL.md \
		&& rm -f $(BUILD_DIR)/claude-skills-stage/tcl-lsp-claude-skills-$(VERSION)/skills/spec-author/SKILL.md.bak
	@mkdir -p $(BUILD_DIR)
	@rm -f $@
	@cd $(BUILD_DIR)/claude-skills-stage && zip -qr $(abspath $@) tcl-lsp-claude-skills-$(VERSION)
	@rm -rf $(BUILD_DIR)/claude-skills-stage
	@echo "==> Built $@"

package-vsix: compile $(VSIX_FILE) verify-vsix ## Package VSIX (skip lint/test, for CI)

# JetBrains plugin

JB_DIR     := $(ROOT)editors/jetbrains
JB_PLUGIN  := $(BUILD_DIR)/tcl-lsp-jetbrains-$(VERSION).zip

build-editor-jetbrains: $(JB_PLUGIN) verify-jetbrains-server ## Build JetBrains plugin (.zip), universal across all platforms except riscv64

# $(JB_PLUGIN)'s own prerequisites are staged binaries checked at recipe
# time (below), not tracked by Make as file dependencies — so without a
# forcing prerequisite, a stale zip from a previous run would silently
# survive untouched after only the native binaries were rebuilt. The old
# `rust-server` prerequisite happened to force this (it's phony); this
# sentinel keeps that always-rebuild behaviour explicit now that staging
# no longer depends on rust-server building just one binary.
.PHONY: jb-plugin-force
jb-plugin-force:

$(JB_PLUGIN): jb-plugin-force spec-studio-wasm
	@echo "==> Building JetBrains plugin"
	@# build.gradle.kts reads RELEASE_VERSION from the environment first, so
	@# the gradle.properties source file is never mutated by the build.
	@# Copy shared resources into plugin resources
	mkdir -p $(JB_DIR)/src/main/resources/syntaxes
	cp $(EXT_DIR)/syntaxes/tcl.tmLanguage.json $(JB_DIR)/src/main/resources/syntaxes/
	@# Bundle one native LSP server binary per platform into server/<dir>/,
	@# the same layout and SERVER_TARGET_MAP the VS Code universal VSIX
	@# uses (minus riscv64 — see SERVER_TARGETS_JETBRAINS).
	@# ``build.gradle.kts`` registers a ``prepareSandbox`` copy that picks up
	@# the whole tree from here and drops it at the plugin root in the
	@# distribution — same layout JetBrains' own Prisma ORM plugin uses to
	@# ship its bundled language server.
	rm -rf $(JB_DIR)/server
	@set -eu; \
		missing=""; \
		for pair in $(SERVER_TARGET_MAP); do \
			triple="$${pair%%:*}"; dir="$${pair##*:}"; \
			case " $(JB_BUNDLED_TARGETS) " in *" $$triple "*) ;; *) continue;; esac; \
			case "$$triple" in *windows*) exe="tcl-lsp-server.exe";; *) exe="tcl-lsp-server";; esac; \
			src="$(ROOT)target/$$triple/release/$$exe"; \
			if [ ! -f "$$src" ]; then missing="$$missing $$triple"; continue; fi; \
			mkdir -p "$(JB_DIR)/server/$$dir"; \
			cp "$$src" "$(JB_DIR)/server/$$dir/$$exe"; \
			chmod +x "$(JB_DIR)/server/$$dir/$$exe"; \
			mkdir -p "$(JB_DIR)/server/$$dir/specs"; \
			cp $(SPEC_PACK_SRC)/*.tclspec "$(JB_DIR)/server/$$dir/specs/"; \
			echo "    server/$$dir/$$exe (+ specs/)"; \
		done; \
		if [ -n "$$missing" ]; then \
			echo "ERROR: missing built server binaries for:$$missing"; \
			echo "Build them first: make server-cross-build  (host targets)"; \
			echo "             or:  make server-cross-build-all  (all 7 — needs cross deps)"; \
			exit 1; \
		fi
	@# Extract compiler explorer HTML from VS Code extension
	cd $(EXT_DIR) && node -e " \
		const {getWebviewHtml} = require('./out/compilerExplorerHtml'); \
		require('fs').writeFileSync('$(JB_DIR)/src/main/resources/compilerExplorer.html', getWebviewHtml()); \
	" 2>/dev/null || echo "(compiler explorer HTML extraction skipped — compile TS first)"
	rm -rf $(JB_DIR)/src/main/resources/spec-studio
	mkdir -p $(JB_DIR)/src/main/resources/spec-studio
	cp -R $(ROOT)rust/tcl-spec-studio-wasm/dist/. $(JB_DIR)/src/main/resources/spec-studio/
	@# Build plugin — pass version via env so build.gradle.kts picks it up
	cd $(JB_DIR) && RELEASE_VERSION="$(SEMVER_VERSION)" ./gradlew buildPlugin
	mkdir -p $(BUILD_DIR)
	cp $(JB_DIR)/build/distributions/tcl-lsp-jetbrains-$(SEMVER_VERSION).zip $(JB_PLUGIN)
	@echo ""
	@echo "Built: $(JB_PLUGIN)"
	@ls -lh $(JB_PLUGIN)

verify-jetbrains-server: $(JB_PLUGIN) ## Fail if the JetBrains plugin is missing an expected platform server binary
	@echo "==> Verifying JetBrains plugin server binaries"
	@set -euo pipefail; \
		entries="$$(unzip -Z1 $(JB_PLUGIN))"; \
		want=0; have=0; missing=""; \
		for pair in $(SERVER_TARGET_MAP); do \
			triple="$${pair%%:*}"; dir="$${pair##*:}"; \
			case " $(JB_BUNDLED_TARGETS) " in *" $$triple "*) ;; *) continue;; esac; \
			case "$$triple" in *windows*) exe="tcl-lsp-server.exe";; *) exe="tcl-lsp-server";; esac; \
			want=$$((want+1)); \
			if echo "$$entries" | grep -q "/server/$$dir/$$exe$$"; then \
				have=$$((have+1)); \
			else \
				missing="$$missing server/$$dir/$$exe"; \
			fi; \
			for pack in $(notdir $(SPEC_PACK_FILES)); do \
				echo "$$entries" | grep -q "/server/$$dir/specs/$$pack$$" \
					|| missing="$$missing server/$$dir/specs/$$pack"; \
			done; \
		done; \
		if [ -n "$$missing" ]; then \
			echo "JetBrains plugin missing expected native server binaries / spec packs:$$missing"; \
			exit 1; \
		fi; \
		echo "==> JetBrains plugin bundles $$have/$$want native server binaries, each with the shipped spec packs"

verify-editor-jetbrains: ## Run the IntelliJ Plugin Verifier over the JetBrains plugin (binary-compat gate)
	@echo "==> Verifying JetBrains plugin against the configured IDE targets"
	@# Runs the JetBrains IntelliJ Plugin Verifier (bytecode-level binary
	@# compatibility analysis) against every IDE in the pluginVerification.ides
	@# block of build.gradle.kts. This is what catches the moved-API /
	@# `sendRequestSync$default` class of NoSuchMethodError regressions before
	@# they ship. Downloads each target IDE on first run (multi-GB, cached).
	@# See the jetbrains-plugin-compat skill for reading the verdicts.
	cd $(JB_DIR) && ./gradlew verifyPlugin

publish-jetbrains: build-editor-jetbrains ## Publish JetBrains plugin to JetBrains Marketplace
	@echo "==> Resolving JetBrains Marketplace credentials"
	@JETBRAINS_TOKEN="$$(bash $(ROOT)scripts/release/jetbrains_token.sh)" || exit 1; \
	export JETBRAINS_TOKEN; \
	echo "==> Publishing JetBrains plugin to Marketplace$(if $(JETBRAINS_CHANNEL), (channel: $(JETBRAINS_CHANNEL)), (channel: Stable))"; \
	cd $(JB_DIR) && RELEASE_VERSION="$(SEMVER_VERSION)" JETBRAINS_CHANNEL="$(JETBRAINS_CHANNEL)" ./gradlew publishPlugin

# Sublime Text package
#
# Unlike the VSIX / JetBrains bundles this package ships NO native binary:
# Package Control serves one artefact to every platform, so a bundled
# server would be right for one platform and wrong for the other five.
# `plugin.py` instead downloads the `tcl-lsp-server-<triple>` asset for the
# host from the release named in `server_version.json`, verified against the
# digest pinned in that same file, into LSP's package storage.
#
# The SpecTcl loadables (`docs/design/spec-packs.md`) still ship *in* the
# package: they are plain data, identical on every platform, and the plugin
# stages them beside the downloaded binary (TCL_LSP_SPEC_PACK_DIR).  Without
# them the shipped EDA syntaxes (Cadence/Xilinx/Quartus/Mentor/Synopsys)
# would highlight commands the server reports as unknown.

ST_DIR      := $(ROOT)editors/sublime-text
# Package Control resolves the release asset by exact name, and a manual
# install must land in `Installed Packages/` under the package name, so the
# artefact is named for the package rather than the version.
ST_PACKAGE  := $(BUILD_DIR)/TclLsp.sublime-package

build-editor-sublime: $(ST_PACKAGE) ## Build Sublime Text package (.sublime-package)

.PHONY: $(ST_PACKAGE)
$(ST_PACKAGE):
	@echo "==> Building Sublime Text package"
	@rm -rf $(BUILD_DIR)/sublime-stage
	@mkdir -p $(BUILD_DIR)/sublime-stage
	cp -r $(ST_DIR)/. $(BUILD_DIR)/sublime-stage/
	find $(BUILD_DIR)/sublime-stage -name '__pycache__' -type d -exec rm -rf {} + 2>/dev/null || true
	find $(BUILD_DIR)/sublime-stage -name '.DS_Store' -delete 2>/dev/null || true
	find $(BUILD_DIR)/sublime-stage -name '*.pyc' -delete 2>/dev/null || true
	@# The README's relative links only resolve inside the monorepo; Package
	@# Control renders the copy on GitHub instead (see SUBMITTING.md).
	rm -f $(BUILD_DIR)/sublime-stage/README.md $(BUILD_DIR)/sublime-stage/SUBMITTING.md
	@mkdir -p $(BUILD_DIR)/sublime-stage/specs
	cp $(SPEC_PACK_SRC)/*.tclspec $(BUILD_DIR)/sublime-stage/specs/
	cp $(LICENSE_SRC) $(BUILD_DIR)/sublime-stage/LICENSE.txt
	@# The release the plugin downloads its server from, plus the SHA-256 of
	@# each server binary that release will carry.  Pinning the digests here
	@# is what makes the download trustworthy: the plugin then verifies
	@# against a digest that reached the user inside the package (via Package
	@# Control), not against the SHA256SUMS served from the same release as
	@# the binary — replacing a release asset does not also replace this.
	@# Verifying SHA256SUMS.cosign.bundle instead is not open to us: Sublime
	@# Text's plugin host is stdlib-only, with no X.509 or ECDSA to verify a
	@# sigstore bundle with, and vendoring a crypto stack into a Package
	@# Control submission trades one risk for a worse one.
	@#
	@# SERVER_BINS_DIR is where CI's build-server-matrix artefacts land
	@# (`<triple>/release/tcl-lsp-server[.exe]`) — the same bytes
	@# publish-native-binaries attaches to the release.  A local build leaves
	@# it unset and ships no pins; plugin.py then falls back to the release
	@# SHA256SUMS and says so on the console.
	@set -eu; \
	pins=""; \
	if [ -n "$(SERVER_BINS_DIR)" ]; then \
		for bin in $(SERVER_BINS_DIR)/*/release/tcl-lsp-server $(SERVER_BINS_DIR)/*/release/tcl-lsp-server.exe; do \
			[ -f "$$bin" ] || continue; \
			triple="$$(printf '%s\n' "$$bin" | sed -E 's#.*/([^/]+)/release/.*#\1#')"; \
			sum="$$(sha256sum "$$bin" | cut -d' ' -f1)"; \
			pins="$$pins        \"$$triple\": \"$$sum\",\n"; \
		done; \
		[ -n "$$pins" ] || { echo "SERVER_BINS_DIR=$(SERVER_BINS_DIR) holds no tcl-lsp-server binaries"; exit 1; }; \
	fi; \
	{ \
		printf '{\n    "version": "$(VERSION)"'; \
		if [ -n "$$pins" ]; then \
			printf ',\n    "servers": {\n'; \
			printf "$$pins" | sed '$$ s/,$$//'; \
			printf '    }'; \
		fi; \
		printf '\n}\n'; \
	} > $(BUILD_DIR)/sublime-stage/server_version.json
	@python3 -c "import json,sys; d=json.load(open('$(BUILD_DIR)/sublime-stage/server_version.json')); \
		print('==> Server pins: %d platform(s)' % len(d.get('servers') or {}) if d.get('servers') \
		      else '==> No server pins (local build) — plugin.py will fall back to the release SHA256SUMS')"
	@echo "==> Packaging .sublime-package"
	@rm -f $(ST_PACKAGE)
	cd $(BUILD_DIR)/sublime-stage && zip -qr $(ST_PACKAGE) . -x '__pycache__/*'
	@# Packaging gate: the resources Sublime Text and Package Control need,
	@# the spec packs the EDA dialects need, and no stray native binary.
	@set -eu; \
	entries="$$(unzip -Z1 $(ST_PACKAGE))"; \
	missing=""; \
	for want in plugin.py .python-version LSP-Tcl.sublime-settings \
	            Tcl.sublime-syntax iRule.sublime-syntax \
	            Default.sublime-commands messages.json LICENSE.txt \
	            server_version.json; do \
		echo "$$entries" | grep -qx "$$want" || missing="$$missing $$want"; \
	done; \
	for pack in $(notdir $(SPEC_PACK_FILES)); do \
		echo "$$entries" | grep -qx "specs/$$pack" \
			|| missing="$$missing specs/$$pack"; \
	done; \
	if [ -n "$$missing" ]; then \
		echo "Sublime package missing expected resources:$$missing"; \
		exit 1; \
	fi; \
	if echo "$$entries" | grep -q "^server/"; then \
		echo "Sublime package bundles a native server; it must download one instead"; \
		exit 1; \
	fi; \
	echo "==> Sublime package carries every resource and spec pack, and no native binary"
	@echo ""
	@echo "Built: $(ST_PACKAGE)  (ready to install)"
	@ls -lh $(ST_PACKAGE)


# The standalone-binary packaging gate. The VSIX / JetBrains / Sublime
# bundles all stage `specs/` beside the server binary (SPEC_PACK_SRC above)
# and the three gates above this one prove that staging happened. The
# `tcl-lsp-server-<triple>` / `tcl-mcp-<triple>` / `tcl-<triple>` /
# `f5-query-<triple>` release assets (publish-native-binaries in ci.yml)
# ship as *bare* binaries with nothing staged beside them at all — what the
# Zed extension downloads at runtime, and what every "point an LSP client at
# a binary" editor in INSTALL-editors.md (Neovim, Emacs, Helix, Vim, Kate,
# Kakoune, Notepad++, Geany, Lite XL, micro, CudaText, JupyterLab) is told to
# fetch. There is nothing to stage for those, so there is nothing a content
# listing can check; `rust/tcl-spectcl/src/bundled.rs` instead compiles the
# six EDA `.tclspec` sources into the binary as a fallback the loader falls
# back to only when no `specs/` directory exists anywhere `bundled_dir`
# looks — see that module's "The embedded fallback" doc section. This gate
# proves the fallback behaviourally: build the `tcl` CLI in release (so the
# `#[cfg(debug_assertions)]` dev-checkout fallback in `bundled_dir` is
# compiled out, not just absent at runtime), copy *only* the binary into an
# empty directory, and ask it to resolve a shipped Xilinx EDA command with
# `TCL_LSP_SPEC_PACK_DIR` unset. `rust/tcl-spectcl/src/bundled.rs`'s own test
# suite (`cargo test -p tcl-spectcl bundled::`, part of every `make test-rust`
# run) proves the same fallback at the registry level on every PR; this is
# the once-per-release confirmation that an actual compiled, isolated binary
# does too.
verify-standalone-eda: ## Prove a bare release binary with no specs/ beside it still resolves an EDA command (the embedded-pack fallback)
	@set -eu; \
	if ! command -v cargo >/dev/null 2>&1; then \
		echo "ERROR: cargo is required."; exit 1; \
	fi; \
	echo "==> Building native tcl CLI (release) for the standalone-EDA check"; \
	cd $(ROOT) && cargo build -p tcl-cli --release --quiet; \
	iso="$$(mktemp -d)"; \
	trap 'rm -rf "$$iso"' EXIT; \
	cp "$(ROOT)target/release/tcl" "$$iso/tcl"; \
	chmod +x "$$iso/tcl"; \
	echo "==> Running $$iso/tcl (alone, no specs/, TCL_LSP_SPEC_PACK_DIR unset) against synth_design"; \
	if ! (cd "$$iso" && env -u TCL_LSP_SPEC_PACK_DIR ./tcl command-info synth_design --dialect xilinx-eda-tcl); then \
		echo "FAILED: a standalone tcl binary with no specs/ beside it could not resolve"; \
		echo "        the Xilinx EDA command 'synth_design' — the embedded-pack fallback"; \
		echo "        in rust/tcl-spectcl/src/bundled.rs is broken or was bypassed."; \
		exit 1; \
	fi; \
	echo "==> OK: standalone tcl CLI resolved synth_design with no specs/ present"

# Zed extension

ZED_DIR     := $(ROOT)editors/zed
ZED_ARCHIVE := $(BUILD_DIR)/tcl-lsp-zed-$(VERSION).zip
ZED_SRCS    := $(shell find $(ZED_DIR)/src -name '*.rs' 2>/dev/null)

build-editor-zed: $(ZED_ARCHIVE) ## Build Zed extension archive (.zip)

$(ZED_ARCHIVE): $(ZED_DIR)/Cargo.toml $(ZED_DIR)/extension.toml $(ZED_SRCS)
	@# A Zed extension is a single cross-platform WASM module, so it cannot
	@# embed a per-platform native binary. Instead the extension downloads the
	@# matching `tcl-lsp-server-<triple>` / `tcl-mcp-<triple>` release asset for
	@# the user's platform at runtime (issue #826). We only stamp the release
	@# version here so the extension pins its downloads to the right tag.
	@echo "==> Building Zed extension WASM (native servers are downloaded at runtime)"
	@if [ -f "$$HOME/.cargo/env" ]; then . "$$HOME/.cargo/env"; fi; \
	if ! rustup target list --installed 2>/dev/null | grep -q wasm32-wasip2; then \
		echo "  -> Installing wasm32-wasip2 target via rustup"; \
		rustup target add wasm32-wasip2; \
	fi
	@if [ -f "$$HOME/.cargo/env" ]; then . "$$HOME/.cargo/env"; fi; \
	cd $(ZED_DIR) && TCL_LSP_BUNDLED_VERSION="$(SEMVER_VERSION)" cargo build --target wasm32-wasip2 --release
	@echo "==> Staging Zed extension archive"
	@rm -rf $(BUILD_DIR)/zed-stage
	@mkdir -p $(BUILD_DIR)/zed-stage
	cp $(ZED_DIR)/extension.toml $(BUILD_DIR)/zed-stage/
	node -e "const f='$(BUILD_DIR)/zed-stage/extension.toml';const fs=require('fs');fs.writeFileSync(f,fs.readFileSync(f,'utf8').replace(/^version = .*/m,'version = \"$(SEMVER_VERSION)\"'))"
	cp $(ZED_DIR)/target/wasm32-wasip2/release/tcl_lsp_zed.wasm $(BUILD_DIR)/zed-stage/extension.wasm
	cp -r $(ZED_DIR)/languages $(BUILD_DIR)/zed-stage/
	cp -r $(ZED_DIR)/snippets $(BUILD_DIR)/zed-stage/
	@echo "==> Packaging Zed extension archive"
	mkdir -p $(BUILD_DIR)
	cd $(BUILD_DIR)/zed-stage && zip -qr $(abspath $(ZED_ARCHIVE)) .
	@echo ""
	@echo "Built: $(ZED_ARCHIVE)"
	@ls -lh $(ZED_ARCHIVE)

publish-zed: build-editor-zed ## Publish Zed extension (prep local PR branch for zed-industries/extensions; you push + open the PR)
	@bash $(ROOT)scripts/release/publish_zed.sh

# Release

release: package-vsix package-vsix-targets claude-skills build-editor-jetbrains build-editor-sublime verify-standalone-eda build-editor-zed release-sums ## Build all release artifacts (parity with tagged CI release jobs)
	@echo ""
	@echo "Built release artifacts in $(BUILD_DIR)"

# Aggregate sha256 hashes for every release artefact in BUILD_DIR. The
# CI publish-checksums job hashes every release-asset file (except
# SHA256SUMS itself and its signature bundle); this target mirrors that
# selection so developers can compare locally-built SUMS against the
# published file.
release-sums: claude-skills package-vsix package-vsix-targets build-editor-jetbrains build-editor-sublime build-editor-zed
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

release-tag: ## Create + push the annotated release tag (V=x.y.z)
	@bash $(ROOT)scripts/release/tag.sh $(V)

# The 2.1.x pre-release line's prepare-then-tag flow. `rust_release.sh tag`
# re-runs release-verify and checks the notes actually merged before handing
# off to tag.sh, so it is the entry point to prefer over release-tag for a
# pre-release; release-tag stays the bare tagging primitive (and the only
# entry point for stable releases cut from rust).
release-perf: ## Benchmark V=x.y.z and regenerate the release-notes graphs
	@bash $(ROOT)scripts/release/perf_release.sh $(V) $(PERF_ARGS)

release-notes-perf: ## Write the performance section of RELEASE_NOTES.md for V=x.y.z
	@python3 $(ROOT)scripts/release/perf_notes.py $(V)

release-verify: ## Check the committed perf results, graphs and notes agree for V=x.y.z
	@bash $(ROOT)scripts/release/rust_release.sh verify $(V)

release-prepare: ## Preflight + benchmark + notes + verify + commit for V=x.y.z (rust line)
	@bash $(ROOT)scripts/release/rust_release.sh prepare $(V) $(PREPARE_ARGS)

release-rust-tag: ## Verify the prepared artefacts, then tag V=x.y.z (rust line)
	@bash $(ROOT)scripts/release/rust_release.sh tag $(V)

publish-all: publish-vsix publish-vsix-targets publish-openvsx publish-openvsx-targets publish-jetbrains publish-zed ## Publish to all editor marketplaces

publish-verify: ## Sanity-check publishing readiness (credentials, tool versions, remote reach) without shipping
	@bash $(ROOT)scripts/release/publish_verify.sh

publish-flow: ## Print the release + marketplace publish cheat-sheet
	@echo "Release + publish flow."
	@echo ""
	@echo "  Source data (required before release; explicit network refresh):"
	@echo "    make update-source-data"
	@echo "    make check-source-data SOURCE_DATA_MAX_AGE_DAYS=180"
	@echo "    # If an upstream is unavailable, document the reason and set"
	@echo "    # SSLICTCL_SOURCE_DATA_WAIVER for release preflight."
	@echo ""
	@echo "  Channels (the tag decides — scripts/release/prerelease.sh):"
	@echo "    stable / default   v2.2.0 (even 2.x minor)         cut from rust"
	@echo "    pre-release/brave  v2.1.x (odd 2.x minor)         cut from rust"
	@echo "    # odd-minor 2.x -> GitHub --prerelease + VS Code --pre-release channel;"
	@echo "    # the 1.x Python line is frozen on the locked legacy-py archive."
	@echo ""
	@echo "  For the 2.1.x pre-release line, steps 1-2 are one program:"
	@echo "    make release-prepare V=X.Y.Z     # preflight, benchmark, graphs, notes, verify, commit"
	@echo "    # ...open + merge the notes PR against rust, pull, then:"
	@echo "    make release-rust-tag V=X.Y.Z    # re-verifies, then tags"
	@echo ""
	@echo "  1. make publish-verify             # check that local credentials + tooling are ready"
	@echo "  2. make release-tag V=X.Y.Z        # creates + pushes the annotated tag (e.g. 2.1.0)"
	@echo "     # CI builds + signs + attaches every release artefact to the GitHub Release"
	@echo "     # then VS Code + Open VSX + JetBrains publish from CI behind the approval gate"
	@echo "     # (see docs/design/contracts/release-and-publish.md)"
	@echo "  3. wait for ci.yml to finish on the tag; approve the marketplace-vscode,"
	@echo "     marketplace-openvsx, and marketplace-jetbrains deployments when prompted"
	@echo "  4. make publish-zed                # local; Zed only"
	@echo "     # Sublime needs no publish step: Package Control reads the"
	@echo "     # TclLsp.sublime-package asset straight off the GitHub Release"
	@echo ""
	@echo "  Marketplaces:"
	@echo "    VS Code    -> CI job publish-vsix-marketplace      (secrets.VSCE_PAT, marketplace-vscode)"
	@echo "    Open VSX   -> CI job publish-vsix-openvsx          (secrets.OVSX_PAT, marketplace-openvsx)"
	@echo "    JetBrains  -> CI job publish-jetbrains-marketplace (secrets.JETBRAINS_TOKEN, marketplace-jetbrains)"
	@echo "    Sublime    -> nothing to run (Package Control pulls the release asset)"
	@echo "    Zed        -> make publish-zed       (laptop; preps a local PR for review)"
	@echo ""
	@echo "  Laptop fallbacks for the CI marketplaces: make publish-vsix / publish-openvsx / publish-jetbrains"

# The KCS help database is no longer a build step: the native `tcl` binary
# embeds its help pages directly (see the tcl crate's build.rs).

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
	@# GUI build products embedded into the tcl binary (the checked-in shell stays).
	rm -f  $(EXPLORER_STATIC)/tcl_explorer_wasm.js $(EXPLORER_STATIC)/tcl_explorer_wasm_bg.wasm
	rm -f  $(MERMAID_JS)
	rm -rf $(ZED_DIR)/bundled
	find $(ROOT) -type d -name __pycache__ -exec rm -rf {} + 2>/dev/null || true

distclean: clean ## Remove build artifacts and node_modules
	rm -rf $(EXT_DIR)/node_modules
	rm -f  $(EXT_DIR)/package-lock.json

# Rust runtime port (runtime/rust) — standalone crate, excluded from the root
# workspace (it is `unsafe`; root forbids `unsafe`). These are the direct gates
# the rust-runtime-port doc + runtime/rust/README cite. `check-rust` reuses the
# lint target so the local and CI PR gates cannot drift from this definition.
RUNTIME_RUST_DIR := $(ROOT)runtime/rust

# The standalone runtime's own suite. `runtime/rust` is its OWN cargo
# workspace, so the root `cargo test --workspace` never reaches it and
# `wasm-real-link` only builds/links it — this target is the only thing that
# executes these tests, locally or in CI's `runtime-rust-tests` job (#1768).
#
# `--locked` keeps CI and a checkout resolving the identical dependency graph.
#
# TCL_TOMMATH_DIR is passed EXPLICITLY, never left to the build script's
# fallback: without libtommath `build.rs` prints a `cargo:warning` and builds
# with the bignum backend disabled, which un-registers `expr` (and the
# `::tcl::mathfunc`/`mathop` ensembles) entirely. The suite still runs and can
# still be green — while silently covering far less than it appears to, the
# same trap issue #1542 documents for the real link. `ensure-tcl90-reference`
# fetches the tree; if it is genuinely absent the build's own warning is the
# loud part, so the variable is only exported when the directory exists.
runtime-rust-test: ## Run the Rust runtime port's cargo test (leak round-trip + unit/parse/eval suite)
	@set -eu; \
	tommath="$${TCL_TOMMATH_DIR:-$(ROOT)tmp/tcl9.0.4/libtommath}"; \
	if [ ! -f "$$tommath/tommath.h" ]; then \
		echo "WARNING: no libtommath at $$tommath — the numeric tower (and"; \
		echo "         therefore 'expr') will be DISABLED in this run."; \
		echo "         Fetch it with: make ensure-tcl90-reference"; \
		tommath=""; \
	fi; \
	cd $(RUNTIME_RUST_DIR) && TCL_TOMMATH_DIR="$$tommath" cargo test --locked

runtime-rust-lint: ## Rust runtime port: cargo fmt --check + locked clippy -D warnings
	cd $(RUNTIME_RUST_DIR) && cargo fmt --check && cargo clippy --locked --all-targets -- -D warnings

zed-query-check: ## Validate the generated Zed highlight queries against the pinned tree-sitter grammar
	cd $(ROOT)rust/zed-query-check && cargo test

vm-test: ## Run the bytecode VM crates' cargo test (tcl-bytecode + tcl-runtime-api + tcl-vm)
	cargo test -p tcl-bytecode -p tcl-runtime-api -p tcl-vm

vm-lint: ## Bytecode VM crates: cargo fmt --check + clippy -D warnings
	cargo fmt -p tcl-bytecode -p tcl-runtime-api -p tcl-vm --check
	cargo clippy -p tcl-bytecode -p tcl-runtime-api -p tcl-vm --all-targets -- -D warnings
