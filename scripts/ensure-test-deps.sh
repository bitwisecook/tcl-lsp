#!/usr/bin/env bash
# ensure-test-deps.sh — install (or build) the optional test-slow dependencies.
#
# Covers tools whose absence currently turns into pytest skips during a
# pre-PR ``make test-slow`` run:
#   * ``tclsh9.0`` / ``tclsh8.6`` — Tcl interpreters used by
#     ``scripts/capture_reference_bytecode.sh``, the irule_test framework,
#     and the cli_venv tests.
#   * ``node`` / ``npm`` — the VS Code extension's TypeScript catalog
#     compile checks (``editors/vscode/node_modules/.bin/tsc``).
#   * ``kotlinc`` — the JetBrains plugin's DiagnosticCatalog.kt compile
#     check.
#   * ``rustup`` + Rust stable + the ``wasm32-wasip2`` target — the Zed
#     extension's clippy check in ``make check-rust`` cross-compiles to
#     WASI Preview 2 and fails without that target installed.  Installs
#     the latest stable toolchain (rather than a pinned version) so it
#     tracks the same channel as the local ``cargo`` developers expect.
#
# Supported platforms: Debian/Ubuntu (apt-get), CentOS/RHEL/Rocky/Alma
# (dnf or yum), and macOS (Homebrew).  Anything else falls through with a
# clear "install <tool> manually" message and exits non-zero.
#
# Idempotent: each tool is checked first and the installer is only invoked
# when the binary is missing.  Builds Tcl 9 from the source tree the
# SessionStart hook has already laid down at ``tmp/tcl9.0.3/`` to avoid
# pulling distro packages that may lag the upstream release.
#
# Usage:
#   bash scripts/ensure-test-deps.sh           # install everything missing
#   bash scripts/ensure-test-deps.sh --check   # only report what's missing
#
# Skip individual tools with the matching env var, e.g. ``SKIP_TCLSH=1``,
# ``SKIP_NODE=1``, ``SKIP_KOTLINC=1``, ``SKIP_RUST=1``.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

CHECK_ONLY=0
if [ "${1:-}" = "--check" ]; then
    CHECK_ONLY=1
fi

# ---------------------------------------------------------------- platform

OS="$(uname -s)"
DISTRO=""
PKG=""
SUDO=""

if [ "$OS" = "Darwin" ]; then
    PKG="brew"
elif [ "$OS" = "Linux" ]; then
    if [ -r /etc/os-release ]; then
        # shellcheck disable=SC1091
        . /etc/os-release
        DISTRO="${ID:-}"
    fi
    case "$DISTRO" in
        debian|ubuntu|linuxmint|pop) PKG="apt-get" ;;
        rhel|centos|rocky|almalinux|fedora) PKG="dnf" ;;
        *)
            if command -v apt-get >/dev/null 2>&1; then PKG="apt-get"
            elif command -v dnf >/dev/null 2>&1; then PKG="dnf"
            elif command -v yum >/dev/null 2>&1; then PKG="yum"
            fi
            ;;
    esac
    if [ "$(id -u)" != "0" ]; then SUDO="sudo"; fi
fi

# If the caller has opted out of every installable tool, there's
# nothing for the platform-specific package manager to do — succeed
# without ever needing one.
if [ "${SKIP_TCLSH:-}" = "1" ] && [ "${SKIP_NODE:-}" = "1" ] && [ "${SKIP_KOTLINC:-}" = "1" ]; then
    echo "ensure-test-deps: SKIP_TCLSH=SKIP_NODE=SKIP_KOTLINC=1 — nothing to do."
    exit 0
fi

if [ -z "$PKG" ]; then
    echo "ensure-test-deps: unsupported platform ($OS / ${DISTRO:-unknown})." >&2
    echo "Install tclsh9.0, node/npm, and kotlinc manually, or set the" >&2
    echo "SKIP_TCLSH / SKIP_NODE / SKIP_KOTLINC env vars to bypass." >&2
    exit 2
fi

case "$PKG" in
    apt-get) PKG_INSTALL="$SUDO apt-get install -y --no-install-recommends" ;;
    dnf|yum) PKG_INSTALL="$SUDO $PKG install -y" ;;
    brew)    PKG_INSTALL="brew install" ;;
esac

# Best-effort installer for a baseline tool the kotlinc downloader
# needs (curl) — no-op when already present.
ensure_baseline() {
    local cmd="$1"; shift
    if command -v "$cmd" >/dev/null 2>&1; then return 0; fi
    if [ "$CHECK_ONLY" -eq 1 ]; then
        note_missing "$cmd (would install via $PKG)"
        return 1
    fi
    case "$PKG" in
        apt-get|dnf|yum) run_install "$cmd ($PKG)" "$cmd" ;;
        brew)            run_install "$cmd (Homebrew)" "$cmd" ;;
    esac
}

PKG_REFRESHED=0
refresh_pkg_index() {
    if [ "$PKG_REFRESHED" -eq 1 ]; then return 0; fi
    case "$PKG" in
        apt-get) $SUDO apt-get update -y >/dev/null ;;
        brew)    brew update >/dev/null 2>&1 || true ;;
    esac
    PKG_REFRESHED=1
}

# ---------------------------------------------------------------- helpers

missing=()
note_missing() { missing+=("$1"); }

info()  { printf '==> %s\n' "$*"; }
warn()  { printf 'WARN: %s\n' "$*" >&2; }

run_install() {
    local what="$1"; shift
    if [ "$CHECK_ONLY" -eq 1 ]; then
        note_missing "$what"
        info "(check) would install: $*"
        return 0
    fi
    refresh_pkg_index
    info "Installing $what: $*"
    # shellcheck disable=SC2086
    $PKG_INSTALL "$@"
}

# ---------------------------------------------------------------- tclsh

ensure_tclsh() {
    if [ "${SKIP_TCLSH:-}" = "1" ]; then info "SKIP_TCLSH=1 — skipping tclsh"; return 0; fi
    if command -v tclsh9.0 >/dev/null 2>&1 && command -v tclsh8.6 >/dev/null 2>&1; then
        info "tclsh9.0 + tclsh8.6 already on PATH"
        return 0
    fi

    # Linux distro packages cover tclsh 8.6 well; 9.0 isn't packaged on
    # most distros yet so we build it from the source tree the
    # SessionStart hook drops at tmp/tcl9.0.3/.  macOS Homebrew has both.
    if [ "$PKG" = "brew" ]; then
        if ! command -v tclsh9.0 >/dev/null 2>&1; then
            run_install "Tcl 9 (Homebrew)" tcl-tk
        fi
        if ! command -v tclsh8.6 >/dev/null 2>&1; then
            run_install "Tcl 8.6 (Homebrew)" tcl-tk@8
        fi
        return 0
    fi

    # Linux: distro tclsh8.6 + source build for 9.0
    if ! command -v tclsh8.6 >/dev/null 2>&1; then
        case "$PKG" in
            apt-get) run_install "Tcl 8.6 (apt)" tcl8.6 ;;
            dnf|yum) run_install "Tcl 8.6 (dnf)" tcl tcl-devel ;;
        esac
    fi

    if ! command -v tclsh9.0 >/dev/null 2>&1; then
        local tcl_src="$REPO_ROOT/tmp/tcl9.0.3"
        if [ "$CHECK_ONLY" -eq 1 ]; then
            if [ -d "$tcl_src/unix" ]; then
                note_missing "tclsh9.0 (would build from source at $tcl_src)"
            else
                note_missing "tclsh9.0 (would fetch source via fetch-tcl-source skill, then build)"
            fi
            return 0
        fi
        if [ ! -d "$tcl_src/unix" ]; then
            info "Fetching Tcl 9.0 source via fetch-tcl-source skill"
            bash "$REPO_ROOT/.claude/skills/fetch-tcl-source/fetch_tcl_source.sh" 9.0
        fi
        # Need a C toolchain to compile.
        if ! command -v gcc >/dev/null 2>&1 && ! command -v cc >/dev/null 2>&1; then
            case "$PKG" in
                apt-get) run_install "C toolchain (apt)" build-essential ;;
                dnf|yum) run_install "C toolchain (dnf)" gcc make ;;
            esac
        fi
        info "Building Tcl 9.0 from $tcl_src"
        (
            cd "$tcl_src/unix"
            ./configure --prefix="$REPO_ROOT/tmp/tcl9-prefix" --disable-shared >/dev/null
            make -j"$(getconf _NPROCESSORS_ONLN 2>/dev/null || echo 2)" >/dev/null
        )
        $SUDO install -m 0755 "$tcl_src/unix/tclsh" /usr/local/bin/tclsh9.0
        info "Installed tclsh9.0 → /usr/local/bin/tclsh9.0"
    fi

    # The irule_test framework + cli_venv tests look for ``tclsh`` (no
    # version suffix).  Add a symlink to tclsh9.0 (or 8.6) when nothing
    # provides it; distro packages sometimes leave that alternative
    # unset.
    if ! command -v tclsh >/dev/null 2>&1; then
        local target=""
        if command -v tclsh8.6 >/dev/null 2>&1; then
            target="$(command -v tclsh8.6)"
        elif command -v tclsh9.0 >/dev/null 2>&1; then
            target="$(command -v tclsh9.0)"
        fi
        if [ -n "$target" ]; then
            $SUDO ln -sfn "$target" /usr/local/bin/tclsh
            info "Symlinked tclsh → $target"
        fi
    fi
}

# ---------------------------------------------------------------- node + npm

ensure_node() {
    if [ "${SKIP_NODE:-}" = "1" ]; then info "SKIP_NODE=1 — skipping node"; return 0; fi
    if command -v node >/dev/null 2>&1 && command -v npm >/dev/null 2>&1; then
        info "node + npm already on PATH ($(node --version))"
    else
        case "$PKG" in
            apt-get) run_install "Node.js (apt)" nodejs npm ;;
            dnf|yum) run_install "Node.js (dnf)" nodejs npm ;;
            brew)    run_install "Node.js (Homebrew)" node ;;
        esac
    fi

    # Project-local tsc lives in editors/vscode/node_modules/.bin/tsc and
    # is what the diagnostic-manifest tests look for.  Run npm install
    # there if it hasn't been done yet.
    local ext_dir="$REPO_ROOT/editors/vscode"
    if [ -f "$ext_dir/package.json" ] && [ ! -x "$ext_dir/node_modules/.bin/tsc" ]; then
        if [ "$CHECK_ONLY" -eq 1 ]; then
            note_missing "editors/vscode/node_modules (would run npm install)"
        else
            info "Running npm install in editors/vscode (project tsc)"
            (cd "$ext_dir" && npm install --no-audit --no-fund >/dev/null)
        fi
    fi
}

# ---------------------------------------------------------------- kotlinc

ensure_kotlinc() {
    if [ "${SKIP_KOTLINC:-}" = "1" ]; then info "SKIP_KOTLINC=1 — skipping kotlinc"; return 0; fi
    if command -v kotlinc >/dev/null 2>&1; then
        info "kotlinc already on PATH"
        return 0
    fi
    case "$PKG" in
        apt-get)
            # No reliable kotlinc apt package on Debian/Ubuntu — use the
            # upstream zip drop or Snap.  Prefer Snap when available;
            # otherwise download the binary distribution to /opt and
            # symlink into /usr/local/bin.
            if command -v snap >/dev/null 2>&1; then
                if [ "$CHECK_ONLY" -eq 1 ]; then
                    note_missing "kotlinc (would: snap install kotlin --classic)"
                else
                    $SUDO snap install kotlin --classic
                fi
                return 0
            fi
            install_kotlinc_zip
            ;;
        dnf|yum)
            if command -v snap >/dev/null 2>&1; then
                if [ "$CHECK_ONLY" -eq 1 ]; then
                    note_missing "kotlinc (would: snap install kotlin --classic)"
                else
                    $SUDO snap install kotlin --classic
                fi
                return 0
            fi
            install_kotlinc_zip
            ;;
        brew)
            run_install "Kotlin (Homebrew)" kotlin
            ;;
    esac
}

install_kotlinc_zip() {
    if [ "$CHECK_ONLY" -eq 1 ]; then
        note_missing "kotlinc (would download upstream zip into /opt/kotlinc)"
        return 0
    fi
    local ver="2.0.21"
    local url="https://github.com/JetBrains/kotlin/releases/download/v${ver}/kotlin-compiler-${ver}.zip"
    local tmpzip
    tmpzip="$(mktemp -t kotlinc.XXXXXX.zip)"
    # ``curl`` and ``unzip`` are missing on minimal images — install
    # them before we rely on them or the script aborts mid-download.
    if ! command -v curl >/dev/null 2>&1; then
        case "$PKG" in
            apt-get) run_install "curl" curl ca-certificates ;;
            dnf|yum) run_install "curl" curl ca-certificates ;;
            brew)    run_install "curl (Homebrew)" curl ;;
        esac
    fi
    if ! command -v unzip >/dev/null 2>&1; then
        case "$PKG" in
            apt-get) run_install "unzip" unzip ;;
            dnf|yum) run_install "unzip" unzip ;;
        esac
    fi
    info "Downloading kotlinc $ver from JetBrains release"
    curl -fsSL "$url" -o "$tmpzip"
    $SUDO rm -rf /opt/kotlinc
    $SUDO mkdir -p /opt
    $SUDO unzip -q "$tmpzip" -d /opt
    $SUDO ln -sfn /opt/kotlinc/bin/kotlinc /usr/local/bin/kotlinc
    rm -f "$tmpzip"
    info "Installed kotlinc → /usr/local/bin/kotlinc"
}

# ---------------------------------------------------------------- rustup + wasm32-wasip2

ensure_rust() {
    if [ "${SKIP_RUST:-}" = "1" ]; then info "SKIP_RUST=1 — skipping rust"; return 0; fi

    local need_rust=0 need_wasm=0
    if ! command -v cargo >/dev/null 2>&1 || ! command -v rustup >/dev/null 2>&1; then
        need_rust=1
    fi
    if ! rustup target list --installed 2>/dev/null | grep -q '^wasm32-wasip2$'; then
        need_wasm=1
    fi
    if [ "$need_rust" -eq 0 ] && [ "$need_wasm" -eq 0 ]; then
        info "rustup + cargo + wasm32-wasip2 target already present"
        return 0
    fi

    if [ "$CHECK_ONLY" -eq 1 ]; then
        if [ "$need_rust" -eq 1 ]; then
            note_missing "rustup + rust stable (would install via https://sh.rustup.rs)"
        fi
        if [ "$need_wasm" -eq 1 ]; then
            note_missing "wasm32-wasip2 target (would add via 'rustup target add wasm32-wasip2')"
        fi
        return 0
    fi

    if [ "$need_rust" -eq 1 ]; then
        info "Installing rustup + rust stable (latest)"
        local rustup_init
        rustup_init="$(mktemp -d)/rustup-init.sh"
        # Pull rustup-init from the official mirror and run it non-interactively.
        # `--profile minimal --default-toolchain stable -y` tracks the latest
        # stable toolchain and adds rustfmt/clippy explicitly so the
        # downstream check-rust target works.
        if command -v curl >/dev/null 2>&1; then
            curl -fsSL https://sh.rustup.rs -o "$rustup_init"
        elif command -v wget >/dev/null 2>&1; then
            wget -q https://sh.rustup.rs -O "$rustup_init"
        else
            warn "neither curl nor wget on PATH — install one or set up rustup manually"
            return 1
        fi
        chmod +x "$rustup_init"
        sh "$rustup_init" -y \
            --no-modify-path \
            --profile minimal \
            --default-toolchain stable \
            --component rustfmt --component clippy
        rm -f "$rustup_init"
        # Make cargo/rustup visible to the rest of this script + downstream
        # make targets in the same shell.
        export PATH="${HOME}/.cargo/bin:${PATH}"
    fi

    if ! command -v rustup >/dev/null 2>&1; then
        warn "rustup still not on PATH after install — add ~/.cargo/bin to PATH"
        return 1
    fi

    # The Zed extension's clippy check (make check-rust) cross-compiles
    # to wasm32-wasip2.  Without this target, `make test-slow` fails on
    # ``can't find crate for `core` `` during the futures-core build.
    if [ "$need_wasm" -eq 1 ] || ! rustup target list --installed | grep -q '^wasm32-wasip2$'; then
        info "Adding wasm32-wasip2 target"
        rustup target add wasm32-wasip2
    fi
}

# ---------------------------------------------------------------- main

ensure_tclsh
ensure_node
ensure_kotlinc
ensure_rust

if [ "$CHECK_ONLY" -eq 1 ] && [ "${#missing[@]}" -gt 0 ]; then
    echo
    echo "ensure-test-deps: missing ${#missing[@]} optional dependencies:"
    for m in "${missing[@]}"; do echo "  - $m"; done
    exit 1
fi

info "ensure-test-deps: all dependencies satisfied"
