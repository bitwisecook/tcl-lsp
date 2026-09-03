#!/usr/bin/env bash
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

# ensure-test-deps.sh — install (or build) the optional full-test-suite dependencies.
#
# Covers tools whose absence currently turns into failures or test skips
# in the heavier pre-PR suites (test-ext, test-emacs, and friends):
#   * ``tclsh9.0`` / ``tclsh8.6`` — Tcl interpreters used by
#     ``scripts/capture/bytecode.sh``, the irule_test framework,
#     and the cli_venv tests.
#   * ``node`` / ``npm`` — the VS Code extension's TypeScript catalog
#     compile checks (``editors/vscode/node_modules/.bin/tsc``).
#   * ``kotlinc`` — the JetBrains plugin's DiagnosticCatalog.kt compile
#     check.
#   * ``rustup`` + Rust stable at least as new as the workspace's authoritative
#     ``rust-version`` + the ``wasm32-wasip2`` target — the Zed
#     extension's clippy check in ``make check-rust`` cross-compiles to
#     WASI Preview 2 and fails without that target installed.  Installs
#     the latest stable toolchain (rather than a pinned version) so it
#     tracks the same channel as the local ``cargo`` developers expect.
#   * ``wasmtime`` — the Rust WASM codegen tests run wasm32-wasi binaries
#     through the Wasmtime CLI.
#   * ``wasm-merge`` / ``wasm-opt`` — Binaryen tools used by bundled WASM
#     tests and asyncify runtime builds.
#   * ``wasi-sdk`` — clang + WASI sysroot for the wasm cross-compile of
#     libtommath (the numeric tower) in ``runtime/rust/build.rs``.
#   * ``emacs`` — the headless eglot regression suite.
#   * ``xvfb-run`` — Linux headless VS Code extension tests when DISPLAY is
#     unset.
#   * ``tshark`` — Wireshark's CLI, used by the slow integration tests
#     that validate the ``f5 enrich-wireshark`` profile by feeding it
#     into a real Wireshark and confirming the rules / column / hosts
#     mappings parse and apply.
#   * ``openssl`` — certificate generation for local HTTPS F5 fetch and
#     round-trip tests.
#   * ``ping`` — opt-in probe execution tests for the F5 query helpers.
#   * ``rgxg`` — drift checks for the generated BIG-IP redaction regexes.
#   * ``tmp/tcllib-2.0`` — upstream tcllib sources used by WASM tcllib
#     smoke coverage.
#
# Supported platforms: Debian/Ubuntu (apt-get), CentOS/RHEL/Rocky/Alma
# (dnf or yum), and macOS (Homebrew).  Anything else falls through with a
# clear "install <tool> manually" message and exits non-zero.
#
# Idempotent: each tool is checked first and the installer is only invoked
# when the binary is missing or, for Rust/Node, older than the workspace
# requirement. Builds Tcl 9 from the source tree the
# SessionStart hook has already laid down at ``tmp/tcl9.0.4/`` to avoid
# pulling distro packages that may lag the upstream release.
#
# Usage:
#   bash scripts/dev/ensure-test-deps.sh           # install everything missing
#   bash scripts/dev/ensure-test-deps.sh --check   # only report what's missing
#
# Skip individual tools with the matching env var, e.g. ``SKIP_TCLSH=1``,
# ``SKIP_PYTHON_TK=1``,
# ``SKIP_NODE=1``, ``SKIP_KOTLINC=1``, ``SKIP_RUST=1``,
# ``SKIP_WASMTIME=1``, ``SKIP_BINARYEN=1``, ``SKIP_WASI_SDK=1``,
# ``SKIP_EMACS=1``, ``SKIP_XVFB=1``, ``SKIP_TSHARK=1``,
# ``SKIP_OPENSSL=1``, ``SKIP_PING=1``, ``SKIP_RGXG=1``,
# ``SKIP_TCLLIB=1``, or ``SKIP_UV=1``.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
# shellcheck source=tcl-reference-toolchains.sh
. "$SCRIPT_DIR/tcl-reference-toolchains.sh"
tcl_reference_load_toolchains "$REPO_ROOT"

TCL_REFERENCE_SOURCE_PARENT="${TCL_LSP_TCL_SOURCE_PARENT:-$REPO_ROOT/tmp}"
TCL_REFERENCE_BIN_DIR="$(tcl_reference_bin_dir)"

# Most callers need the whole conformance matrix. Focused CI lanes may select
# a release-line subset (for example, SpecTcl needs only the pinned 9.0 oracle)
# without duplicating any patchlevel or source-tag facts.
requested_tcl_releases() {
    if [ -z "${TCL_LSP_TCL_RELEASES:-}" ]; then
        tcl_reference_releases
        return
    fi
    printf '%s\n' "$TCL_LSP_TCL_RELEASES" \
        | tr ', ' '\n' \
        | awk 'NF && !seen[$0]++ { print }'
}

WASMTIME_VERSION="43.0.1"
WASI_SDK_VERSION="25.0"
TCLLIB_TAG="tcllib-2-0"
TCLLIB_VERSION="2.0"
# Minimum Node.js major — must track Makefile Prerequisites, README,
# docs/design/contracts/development-environment.md,
# and the CI `node-version`. Distro apt/dnf packages (Ubuntu 24.04 ships Node
# 18) fall below this, so install from NodeSource when they would.
NODE_MIN_MAJOR="24"

CHECK_ONLY=0
if [ "${1:-}" = "--check" ]; then
    CHECK_ONLY=1
fi

# ---------------------------------------------------------------- platform

OS="$(uname -s)"
ARCH="$(uname -m)"
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
fi

# Privileged installs (writing to /opt, /usr/local/bin, apt/dnf) need sudo when
# not already root — on macOS too, where `ensure_wasi_sdk`/`ensure_wasmtime`
# `mkdir -p /opt/...` on root-owned /opt. Setting SUDO only in the Linux branch
# meant the Darwin path ran those as the invoking user and aborted under `set
# -e`. Homebrew invocations deliberately do not use $SUDO.
if [ "$(id -u)" != "0" ] && command -v sudo >/dev/null 2>&1; then
    SUDO="sudo"
fi

INSTALLABLE_SKIP_VARS=(
    SKIP_TCLSH
    SKIP_PYTHON_TK
    SKIP_NODE
    SKIP_KOTLINC
    SKIP_RUST
    SKIP_WASMTIME
    SKIP_BINARYEN
    SKIP_WASI_SDK
    SKIP_EMACS
    SKIP_XVFB
    SKIP_TSHARK
    SKIP_OPENSSL
    SKIP_PING
    SKIP_RGXG
    SKIP_TCLLIB
    SKIP_UV
)

# If the caller has opted out of every installable tool, there's nothing
# for the platform-specific package manager to do — succeed without ever
# needing one.
all_skipped=1
for skip_var in "${INSTALLABLE_SKIP_VARS[@]}"; do
    if [ "${!skip_var:-}" != "1" ]; then
        all_skipped=0
        break
    fi
done
if [ "$all_skipped" -eq 1 ]; then
    echo "ensure-test-deps: all installable dependency groups skipped — nothing to do."
    exit 0
fi

if [ -z "$PKG" ]; then
    echo "ensure-test-deps: unsupported platform ($OS / ${DISTRO:-unknown})." >&2
    echo "Install the host test tools manually, or set the matching SKIP_* env vars to bypass." >&2
    exit 2
fi

case "$PKG" in
    apt-get) PKG_INSTALL="$SUDO apt-get install -y --no-install-recommends" ;;
    dnf|yum) PKG_INSTALL="$SUDO $PKG install -y" ;;
    brew)    PKG_INSTALL="brew install" ;;
esac

# Best-effort installer for baseline tools the download helpers need —
# no-op when already present.
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

# The workspace manifest, not this script, owns the MSRV. Keep the parser
# deliberately small and dependency-free: this audit must still explain how
# to recover when Cargo itself cannot run with the installed compiler.
workspace_rust_version() {
    awk '
        /^\[workspace\.package\][[:space:]]*$/ { in_workspace_package = 1; next }
        in_workspace_package && /^\[[^]]+\][[:space:]]*$/ { exit }
        in_workspace_package {
            line = $0
            sub(/[[:space:]]*#.*/, "", line)
            if (line ~ /^[[:space:]]*rust-version[[:space:]]*=/) {
                sub(/^[^=]*=[[:space:]]*/, "", line)
                if (match(line, /^"[^"]*"/)) {
                    print substr(line, RSTART + 1, RLENGTH - 2)
                    exit
                }
            }
        }
    ' "$REPO_ROOT/Cargo.toml"
}

# Return success when $1 is a Rust release at least as new as $2. Rust's
# `rust-version` accepts a short `major.minor` spelling while `rustc --version`
# normally includes a patch number, so compare each numeric component rather
# than comparing the strings lexically. A prerelease/build suffix does not
# change the numeric comparison.
version_at_least() {
    local actual="${1%%[-+]*}" required="${2%%[-+]*}"
    local -a actual_parts required_parts
    local index actual_part required_part

    if ! [[ "$actual" =~ ^[0-9]+(\.[0-9]+){0,2}$ ]] \
        || ! [[ "$required" =~ ^[0-9]+(\.[0-9]+){0,2}$ ]]; then
        return 2
    fi
    IFS='.' read -r -a actual_parts <<< "$actual"
    IFS='.' read -r -a required_parts <<< "$required"
    for index in 0 1 2; do
        actual_part="${actual_parts[$index]:-0}"
        required_part="${required_parts[$index]:-0}"
        if (( 10#$actual_part > 10#$required_part )); then return 0; fi
        if (( 10#$actual_part < 10#$required_part )); then return 1; fi
    done
    return 0
}

WORKSPACE_RUST_VERSION="$(workspace_rust_version)"
if [ -z "$WORKSPACE_RUST_VERSION" ]; then
    echo "ensure-test-deps: could not read workspace.package.rust-version from Cargo.toml" >&2
    exit 2
fi

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

fetch_with_retry() {
    local url="$1"
    local dest="$2"
    local attempt
    for attempt in 1 2 3 4; do
        if curl -fsSL --retry 0 --connect-timeout 15 --max-time 600 -o "$dest" "$url"; then
            return 0
        fi
        if [ "$attempt" -lt 4 ]; then
            local wait=$((2 ** attempt))
            warn "retry $attempt after ${wait}s: $url"
            sleep "$wait"
        fi
    done
    return 1
}

sha256_file() {
    local path="$1"
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$path" | awk '{print $1}'
    elif command -v shasum >/dev/null 2>&1; then
        shasum -a 256 "$path" | awk '{print $1}'
    else
        warn "neither sha256sum nor shasum is available"
        return 1
    fi
}

ensure_download_tools() {
    if ! command -v curl >/dev/null 2>&1; then
        case "$PKG" in
            apt-get) run_install "curl" curl ca-certificates ;;
            dnf|yum) run_install "curl" curl ca-certificates ;;
            brew)    run_install "curl (Homebrew)" curl ;;
        esac
    fi
    if ! command -v tar >/dev/null 2>&1; then
        case "$PKG" in
            apt-get|dnf|yum) run_install "tar" tar ;;
        esac
    fi
    if ! command -v xz >/dev/null 2>&1; then
        case "$PKG" in
            apt-get) run_install "xz-utils" xz-utils ;;
            dnf|yum) run_install "xz" xz ;;
            brew)    run_install "xz (Homebrew)" xz ;;
        esac
    fi
}

install_symlink() {
    local source="$1"
    local dest="$2"
    local parent
    parent="$(dirname "$dest")"
    if { [ -d "$parent" ] && [ -w "$parent" ]; } \
        || { [ ! -e "$parent" ] && [ -w "$(dirname "$parent")" ]; }; then
        mkdir -p "$parent"
        ln -sfn "$source" "$dest"
    else
        $SUDO mkdir -p "$parent"
        $SUDO ln -sfn "$source" "$dest"
    fi
}

make_temp_file() {
    local dir="$1"
    local prefix="$2"
    mkdir -p "$dir"
    mktemp "$dir/${prefix}.XXXXXX"
}

# ---------------------------------------------------------------- tclsh

# The exact reference patchlevels are owned by tcl-dialect's manifest and
# loaded above. Distro/Homebrew names are insufficient: a present tclsh9.0 can
# still be an older 9.0.3 build. Every release is therefore probed before the
# all-ready return and repaired from its pinned source tree when necessary.
#
# Unlike 9.0/9.1 (which embed their script library via zipfs and run fine
# from anywhere), an 8.4/8.5 tclsh only finds init.tcl via a path relative
# to its own real on-disk location — a bare copy or symlink into
# /usr/local/bin breaks that lookup the moment argv[0] isn't the tree path
# itself. `install_reference_tclsh_wrapper` installs a thin wrapper script
# that execs the tree's real `tclsh` by absolute path instead, which keeps
# the relative library search intact regardless of the caller's
# TCL_LIBRARY (also explicitly unset, so a stale global export cannot shadow
# the version being probed).

# Install /usr/local/bin/tclsh<tag> as a wrapper execing $target by its real
# path — see the block comment above ensure_tclsh for why this beats a
# symlink for the 8.4/8.5 trees (9.0/9.1 would tolerate either).
install_reference_tclsh_wrapper() {
    local target="$1" dest="$2"
    local tmp parent
    tmp="$(mktemp)"
    {
        printf '#!/bin/sh\n'
        printf 'unset TCL_LIBRARY\n'
        printf 'exec "%s" "$@"\n' "$target"
    } > "$tmp"
    chmod 0755 "$tmp"
    parent="$(dirname "$dest")"
    if { [ -d "$parent" ] && [ -w "$parent" ]; } \
        || { [ ! -e "$parent" ] && [ -w "$(dirname "$parent")" ]; }; then
        mkdir -p "$parent"
        install -m 0755 "$tmp" "$dest"
    else
        $SUDO mkdir -p "$parent"
        $SUDO install -m 0755 "$tmp" "$dest"
    fi
    rm -f "$tmp"
}

# Build (idempotently) any reference interpreter from its pinned source tree
# and expose it through the managed bin directory. This one path owns all five
# axes, so 8.6 and 9.0 cannot silently retain weaker distro-only semantics.
ensure_reference_tclsh() {
    local release="$1"
    local expected
    expected="$(tcl_reference_patchlevel "$release")"
    local tree_dir="$TCL_REFERENCE_SOURCE_PARENT/tcl${expected}"
    local tclsh_bin="$tree_dir/unix/tclsh"
    local link="$TCL_REFERENCE_BIN_DIR/tclsh${release}"
    local command_name="tclsh${release}"
    local resolved="" actual="" repair=""

    if resolved="$(tcl_reference_resolve_tclsh "$release")"; then
        info "$command_name exact reference patchlevel $expected resolved at $resolved"
        return 0
    else
        local resolve_status=$?
        if [ "$resolve_status" -eq 2 ]; then
            return 1
        fi
    fi
    resolved="$(command -v "$command_name" 2>/dev/null || true)"
    if [ -n "$resolved" ]; then
        actual="$(tcl_reference_tclsh_patchlevel "$resolved" 2>/dev/null || true)"
    fi

    if tcl_reference_tclsh_reports_patchlevel "$tclsh_bin" "$expected"; then
        repair="would install a wrapper for $tclsh_bin at $link"
    else
        if [ -d "$tree_dir/unix" ]; then
            repair="would build from $tree_dir, then install a wrapper at $link"
        else
            repair="would fetch $expected, build it, then install a wrapper at $link"
        fi
    fi

    if [ "$CHECK_ONLY" -eq 1 ]; then
        if [ -n "$actual" ]; then
            note_missing "$command_name $expected (found $actual at $resolved; $repair)"
        elif [ -n "$resolved" ]; then
            note_missing "$command_name $expected (unusable interpreter at $resolved; $repair)"
        else
            note_missing "$command_name $expected ($repair)"
        fi
        return 0
    fi

    if ! tcl_reference_tclsh_reports_patchlevel "$tclsh_bin" "$expected"; then
        if [ ! -d "$tree_dir/unix" ]; then
            info "Fetching Tcl $release source ($expected) via fetch-tcl-source skill"
            bash "$REPO_ROOT/.claude/skills/fetch-tcl-source/fetch_tcl_source.sh" "$release"
        fi
        if ! command -v gcc >/dev/null 2>&1 && ! command -v cc >/dev/null 2>&1; then
            case "$PKG" in
                apt-get) run_install "C toolchain (apt)" build-essential ;;
                dnf|yum) run_install "C toolchain (dnf)" gcc make ;;
                brew)
                    echo "ERROR: a C compiler is required to build Tcl $expected" >&2
                    return 1
                    ;;
            esac
        fi
        info "Building $command_name ($expected) from $tree_dir"
        (
            cd "$tree_dir/unix"
            CFLAGS="-O2 -fcommon -Wno-implicit-int -Wno-implicit-function-declaration" \
                ./configure --disable-shared >/dev/null
            make -j"$(getconf _NPROCESSORS_ONLN 2>/dev/null || echo 2)" >/dev/null
        )
        if ! tcl_reference_tclsh_reports_patchlevel "$tclsh_bin" "$expected"; then
            echo "ERROR: built $tclsh_bin does not report patchlevel $expected" >&2
            return 1
        fi
    fi

    if ! tcl_reference_tclsh_reports_patchlevel "$link" "$expected"; then
        install_reference_tclsh_wrapper "$tclsh_bin" "$link"
        info "Installed $command_name → $link (wraps $tclsh_bin)"
    fi
    if ! tcl_reference_tclsh_reports_patchlevel "$link" "$expected"; then
        echo "ERROR: installed $link does not report Tcl $expected" >&2
        return 1
    fi
}

ensure_tclsh() {
    if [ "${SKIP_TCLSH:-}" = "1" ]; then info "SKIP_TCLSH=1 — skipping tclsh"; return 0; fi
    local release command_name resolved expected
    local all_ready=1
    local requested
    requested="$(requested_tcl_releases)"
    if [ -z "$requested" ]; then
        echo "ERROR: TCL_LSP_TCL_RELEASES selected no Tcl release lines" >&2
        return 1
    fi
    while IFS= read -r release; do
        if ! tcl_reference_patchlevel "$release" >/dev/null; then
            return 1
        fi
        if resolved="$(tcl_reference_resolve_tclsh "$release")"; then
            :
        else
            local resolve_status=$?
            if [ "$resolve_status" -eq 2 ]; then
                return 1
            fi
            all_ready=0
        fi
    done <<< "$requested"
    if [ "$all_ready" -eq 1 ]; then
        info "requested tclsh exact reference patchlevels already resolved"
        return 0
    fi

    while IFS= read -r release; do
        ensure_reference_tclsh "$release"
    done <<< "$requested"

    # The irule_test framework + cli_venv tests look for ``tclsh`` (no
    # version suffix). Add a symlink to the exact 9.0 reference when nothing
    # provides it. --check only reports this repair; it never mutates the host.
    if printf '%s\n' "$requested" | grep -qx 9.0 && ! command -v tclsh >/dev/null 2>&1; then
        local target
        target="$(command -v tclsh9.0 2>/dev/null || true)"
        if [ -n "$target" ]; then
            if [ "$CHECK_ONLY" -eq 1 ]; then
                note_missing "tclsh (would symlink $TCL_REFERENCE_BIN_DIR/tclsh to $target)"
            else
                install_symlink "$target" "$TCL_REFERENCE_BIN_DIR/tclsh"
                info "Symlinked tclsh → $target"
            fi
        fi
    fi
}

# ---------------------------------------------------------------- tcllib source

ensure_tcllib() {
    if [ "${SKIP_TCLLIB:-}" = "1" ]; then info "SKIP_TCLLIB=1 — skipping tcllib source"; return 0; fi

    local target_dir="$REPO_ROOT/tmp/tcllib-${TCLLIB_VERSION}"
    if [ -d "$target_dir/modules" ]; then
        info "tcllib ${TCLLIB_VERSION} source already present"
        return 0
    fi
    if [ "$CHECK_ONLY" -eq 1 ]; then
        note_missing "tcllib ${TCLLIB_VERSION} source (would download GitHub codeload tarball)"
        return 0
    fi

    ensure_download_tools
    local url="https://codeload.github.com/tcltk/tcllib/tar.gz/refs/tags/${TCLLIB_TAG}"
    mkdir -p "$REPO_ROOT/tmp"
    local tmp_tarball
    tmp_tarball="$(make_temp_file "$REPO_ROOT/tmp" "tcllib-${TCLLIB_VERSION}")"
    # shellcheck disable=SC2064
    trap "rm -f '$tmp_tarball'" RETURN

    info "Downloading tcllib ${TCLLIB_VERSION}"
    fetch_with_retry "$url" "$tmp_tarball"
    rm -rf "$target_dir"
    mkdir -p "$target_dir"
    tar -xzf "$tmp_tarball" -C "$target_dir" --strip-components=1
    if [ ! -d "$target_dir/modules" ]; then
        rm -rf "$target_dir"
        echo "ERROR: tcllib modules/ missing after extract" >&2
        return 1
    fi
    info "Extracted tcllib ${TCLLIB_VERSION} to $target_dir"
}

ensure_python_tk() {
    # The iRule test-framework suite cross-checks behaviour against a real Tcl
    # interpreter via ``tkinter.Tcl()``.  Without the Tk binding the whole
    # suite (~70 cases) silently skips, so install the ``python<X.Y>-tk``
    # package matching the interpreter that runs pytest.
    if [ "${SKIP_PYTHON_TK:-}" = "1" ]; then info "SKIP_PYTHON_TK=1 — skipping python tk"; return 0; fi
    if python3 -c "import tkinter" >/dev/null 2>&1; then
        info "python tkinter already importable"
        return 0
    fi
    local pyver
    pyver="$(python3 -c 'import sys; print(f"{sys.version_info.major}.{sys.version_info.minor}")' 2>/dev/null || echo "")"
    case "$PKG" in
        apt-get)
            # Prefer the version-specific package (python3.11-tk); fall back to
            # the generic python3-tk for distros that only ship the default.
            if [ -n "$pyver" ] && run_install "python tk (apt)" "python${pyver}-tk"; then
                :
            else
                run_install "python tk (apt)" python3-tk
            fi
            ;;
        dnf | yum) run_install "python tk (dnf)" python3-tkinter ;;
        brew) run_install "python tk (Homebrew)" python-tk ;;
    esac
}

# ---------------------------------------------------------------- node + npm

# Major version of the `node` on PATH, or empty if node is absent.
node_current_major() {
    command -v node >/dev/null 2>&1 || return 0
    node --version 2>/dev/null | sed -n 's/^v\([0-9]\{1,\}\)\..*/\1/p'
}

# Install Node from NodeSource (apt/dnf), which ships the current major, rather
# than the distro package (Ubuntu 24.04 apt = Node 18). The NodeSource `nodejs`
# package bundles npm.
install_node_from_nodesource() {
    local scheme="$1" # deb | rpm
    local setup="https://${scheme}.nodesource.com/setup_${NODE_MIN_MAJOR}.x"
    info "Adding NodeSource repo ($setup)"
    # Run the setup script as root. When already root `$SUDO` is empty, so drop
    # the `-E` (which is a sudo flag) rather than passing it as the command —
    # `$SUDO -E bash -` would expand to `-E bash -` and fail.
    local runner
    if [ -n "$SUDO" ]; then
        runner="$SUDO -E bash -"
    else
        runner="bash -"
    fi
    # shellcheck disable=SC2086
    if ! curl -fsSL --connect-timeout 15 --max-time 600 "$setup" | $runner; then
        warn "NodeSource setup failed; falling back to the distro nodejs package"
        return 1
    fi
    run_install "Node.js (NodeSource ${NODE_MIN_MAJOR}.x)" nodejs
}

ensure_node() {
    if [ "${SKIP_NODE:-}" = "1" ]; then info "SKIP_NODE=1 — skipping node"; return 0; fi
    local cur; cur="$(node_current_major)"
    if [ -n "$cur" ] && [ "$cur" -ge "$NODE_MIN_MAJOR" ] && command -v npm >/dev/null 2>&1; then
        info "node + npm already on PATH ($(node --version)); >= v${NODE_MIN_MAJOR}"
    elif [ "$CHECK_ONLY" -eq 1 ]; then
        if [ -n "$cur" ]; then
            note_missing "Node.js >= ${NODE_MIN_MAJOR} (found v${cur})"
        else
            note_missing "Node.js >= ${NODE_MIN_MAJOR}"
        fi
    else
        if [ -n "$cur" ]; then
            info "node v${cur} is below the v${NODE_MIN_MAJOR} minimum — upgrading"
        fi
        case "$PKG" in
            apt-get) install_node_from_nodesource deb || run_install "Node.js (apt)" nodejs npm ;;
            dnf|yum) install_node_from_nodesource rpm || run_install "Node.js (dnf)" nodejs npm ;;
            # Homebrew's `node` formula tracks the current major (>= 24).
            brew)    run_install "Node.js (Homebrew)" node ;;
        esac
    fi

    # Project-local tsc lives in editors/vscode/node_modules/.bin/tsc and
    # is what the diagnostic-manifest tests look for.  Run a hash-pinned
    # `npm ci` (lockfile is committed) there if it hasn't been done yet.
    local ext_dir="$REPO_ROOT/editors/vscode"
    if [ -f "$ext_dir/package.json" ] && [ ! -x "$ext_dir/node_modules/.bin/tsc" ]; then
        if [ "$CHECK_ONLY" -eq 1 ]; then
            note_missing "editors/vscode/node_modules (would run npm ci)"
        else
            info "Running npm ci in editors/vscode (project tsc)"
            (cd "$ext_dir" && npm ci --no-audit --no-fund >/dev/null)
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
    tmpzip="$(make_temp_file "${TMPDIR:-/tmp}" "kotlinc")"
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

    # Prefer rustup's shims when they exist.  Some macOS setups have
    # Homebrew's cargo/rustc earlier on PATH and ~/.cargo/bin later; in
    # that shape `rustup target list --installed` can report wasm32-wasip2
    # while the cargo that `make check-rust` runs still uses the Homebrew
    # toolchain, whose target libraries are not installed by rustup.
    if [ -x "$HOME/.cargo/bin/rustup" ]; then
        export PATH="$HOME/.cargo/bin:$PATH"
    elif [ -f "$HOME/.cargo/env" ]; then
        # shellcheck disable=SC1091
        . "$HOME/.cargo/env"
    fi

    local need_rust=0 need_rust_update=0 need_wasm=0 rust_version=""
    if ! command -v cargo >/dev/null 2>&1 \
        || ! command -v rustc >/dev/null 2>&1 \
        || ! command -v rustup >/dev/null 2>&1; then
        need_rust=1
    else
        rust_version="$(rustc --version 2>/dev/null | awk '{ print $2; exit }' || true)"
        if ! version_at_least "$rust_version" "$WORKSPACE_RUST_VERSION"; then
            need_rust_update=1
        fi
    fi
    local installed_targets
    installed_targets="$(rustup target list --installed 2>/dev/null || true)"
    if ! printf '%s\n' "$installed_targets" | grep -q '^wasm32-wasip2$' \
        || ! printf '%s\n' "$installed_targets" | grep -q '^wasm32-unknown-unknown$'; then
        need_wasm=1
    fi
    if [ "$need_rust" -eq 0 ] && [ "$need_rust_update" -eq 0 ] && [ "$need_wasm" -eq 0 ]; then
        info "rustup + Rust ${rust_version} (>= ${WORKSPACE_RUST_VERSION}) + wasm32-wasip2/unknown targets already present"
        return 0
    fi

    if [ "$CHECK_ONLY" -eq 1 ]; then
        if [ "$need_rust" -eq 1 ]; then
            note_missing "rustup + rust stable (would install via https://sh.rustup.rs)"
        elif [ "$need_rust_update" -eq 1 ]; then
            note_missing "Rust >= ${WORKSPACE_RUST_VERSION} (found ${rust_version:-an unreadable rustc version}; would run 'rustup update stable')"
        fi
        if [ "$need_wasm" -eq 1 ]; then
            note_missing "wasm32-wasip2 + wasm32-unknown-unknown targets (would add via 'rustup target add')"
        fi
        return 0
    fi

    if [ "$need_rust" -eq 1 ]; then
        info "Installing rustup + rust stable (latest)"
        local rustup_dir rustup_init
        rustup_dir="$(mktemp -d)"
        # Make sure the temp dir is cleaned up even when the caller
        # ^Cs out or rustup-init fails — `trap RETURN` runs on every
        # function exit path.
        # shellcheck disable=SC2064
        trap "rm -rf '$rustup_dir'" RETURN
        rustup_init="${rustup_dir}/rustup-init.sh"
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
        # Make cargo/rustup visible to the rest of this script + downstream
        # make targets in the same shell.
        export PATH="${HOME}/.cargo/bin:${PATH}"
    elif [ "$need_rust_update" -eq 1 ]; then
        info "Rust ${rust_version:-unknown} is below workspace rust-version ${WORKSPACE_RUST_VERSION} — updating stable"
        rustup update stable
    fi

    if [ -x "$HOME/.cargo/bin/rustup" ]; then
        export PATH="$HOME/.cargo/bin:$PATH"
    fi
    if ! command -v rustup >/dev/null 2>&1; then
        warn "rustup still not on PATH after install — add ~/.cargo/bin to PATH"
        return 1
    fi
    rust_version="$(rustc --version 2>/dev/null | awk '{ print $2; exit }' || true)"
    if ! version_at_least "$rust_version" "$WORKSPACE_RUST_VERSION"; then
        warn "rustc ${rust_version:-unknown} is still below workspace rust-version ${WORKSPACE_RUST_VERSION} after updating stable"
        return 1
    fi

    # The Zed extension's clippy check (make check-rust) cross-compiles
    # to wasm32-wasip2.  Without this target, `make check-rust` fails on
    # ``can't find crate for `core` `` during the futures-core build.
    if [ "$need_wasm" -eq 1 ] || ! rustup target list --installed | grep -q '^wasm32-wasip2$'; then
        info "Adding wasm32-wasip2 target"
        rustup target add wasm32-wasip2
    fi
    # The tcl-compiler `wasm_real_link` test compiles + links a generated guest
    # for wasm32-unknown-unknown; without this target it errors with
    # ``can't find crate for `std` `` and is silently skipped under
    # `cargo llvm-cov --ignore-run-fail`, losing the codegen/wasm coverage.
    if ! rustup target list --installed | grep -q '^wasm32-unknown-unknown$'; then
        info "Adding wasm32-unknown-unknown target"
        rustup target add wasm32-unknown-unknown
    fi
}

# ---------------------------------------------------------------- WASM tools

ensure_wasmtime() {
    if [ "${SKIP_WASMTIME:-}" = "1" ]; then info "SKIP_WASMTIME=1 — skipping wasmtime"; return 0; fi
    if command -v wasmtime >/dev/null 2>&1; then
        info "wasmtime already on PATH ($(wasmtime --version 2>/dev/null | head -1 || echo unknown))"
        return 0
    fi
    if [ "$CHECK_ONLY" -eq 1 ]; then
        note_missing "wasmtime ${WASMTIME_VERSION} (would install for $OS/$ARCH)"
        return 0
    fi

    if [ "$PKG" = "brew" ]; then
        run_install "Wasmtime (Homebrew)" wasmtime
        return 0
    fi

    local wasm_arch expected_sha
    case "$ARCH" in
        x86_64)  wasm_arch="x86_64-linux"; expected_sha="9f3cf977fc29e2ccab2d198435265b066dce3d608fc6692d700ed1b9b74c35a1" ;;
        aarch64) wasm_arch="aarch64-linux"; expected_sha="dbf36d4e9108df377ddfb88f2d8db4e07efce9726b68da53ae78ed5579293923" ;;
        *) echo "ERROR: unsupported architecture for Wasmtime: $ARCH" >&2; return 1 ;;
    esac

    ensure_download_tools
    local tarball="wasmtime-v${WASMTIME_VERSION}-${wasm_arch}.tar.xz"
    local url="https://github.com/bytecodealliance/wasmtime/releases/download/v${WASMTIME_VERSION}/${tarball}"
    local tmpdir
    tmpdir="$(mktemp -d)"
    # shellcheck disable=SC2064
    trap "rm -rf '$tmpdir'" RETURN

    info "Downloading Wasmtime ${WASMTIME_VERSION}"
    fetch_with_retry "$url" "$tmpdir/$tarball"
    local actual_sha
    actual_sha="$(sha256_file "$tmpdir/$tarball")"
    if [ "$actual_sha" != "$expected_sha" ]; then
        echo "ERROR: Wasmtime sha256 mismatch (expected $expected_sha, got $actual_sha)" >&2
        return 1
    fi

    local prefix="/opt/wasmtime-${WASMTIME_VERSION}"
    $SUDO rm -rf "$prefix"
    $SUDO mkdir -p "$prefix"
    $SUDO tar -xJf "$tmpdir/$tarball" -C "$prefix" --strip-components=1
    install_symlink "$prefix/wasmtime" /usr/local/bin/wasmtime
    info "Installed wasmtime to /usr/local/bin/wasmtime"
}

ensure_binaryen() {
    if [ "${SKIP_BINARYEN:-}" = "1" ]; then info "SKIP_BINARYEN=1 — skipping Binaryen"; return 0; fi
    if command -v wasm-merge >/dev/null 2>&1 && command -v wasm-opt >/dev/null 2>&1; then
        info "Binaryen tools already on PATH"
        return 0
    fi
    case "$PKG" in
        apt-get) run_install "Binaryen (apt)" binaryen ;;
        dnf|yum) run_install "Binaryen (dnf)" binaryen ;;
        brew)    run_install "Binaryen (Homebrew)" binaryen ;;
    esac
}

# ---------------------------------------------------------------- wasi-sdk
# clang + WASI sysroot for the wasm cross-compile of libtommath (the numeric
# tower) in runtime/rust/build.rs.  Installed to /opt/wasi-sdk-<ver> and
# symlinked /opt/wasi-sdk, which build.rs auto-discovers (or set WASI_SDK_PATH).
# Without it, runtime/rust's wasm build drops the tower (expr/mathop/mathfunc/
# lseq) with a warning.

ensure_wasi_sdk() {
    if [ "${SKIP_WASI_SDK:-}" = "1" ]; then info "SKIP_WASI_SDK=1 — skipping wasi-sdk"; return 0; fi
    local link="/opt/wasi-sdk"
    if [ -x "${WASI_SDK_PATH:-$link}/bin/clang" ]; then
        info "wasi-sdk already present ($("${WASI_SDK_PATH:-$link}/bin/clang" --version 2>/dev/null | head -1))"
        return 0
    fi
    if [ "$CHECK_ONLY" -eq 1 ]; then
        note_missing "wasi-sdk ${WASI_SDK_VERSION} (would install for $OS/$ARCH → ${link})"
        return 0
    fi

    local sdk_os sdk_arch
    case "$OS" in
        Linux)  sdk_os="linux" ;;
        Darwin) sdk_os="macos" ;;
        *) echo "ERROR: unsupported OS for wasi-sdk: $OS" >&2; return 1 ;;
    esac
    case "$ARCH" in
        x86_64)        sdk_arch="x86_64" ;;
        aarch64|arm64) sdk_arch="arm64" ;;
        *) echo "ERROR: unsupported architecture for wasi-sdk: $ARCH" >&2; return 1 ;;
    esac

    # Integrity pins, per platform, matching what every other tool here carries.
    #
    # Why pinned rather than verified against an upstream sidecar: wasi-sdk
    # publishes **no checksums at all**.  There is no `SHA256SUMS` asset on the
    # `wasi-sdk-25` release (the URL 404s), no per-asset `.sha256`, no hashes in
    # the release body, and the GitHub API reports `digest: null` for the
    # assets.  The same is true of every wasi-sdk release, so the sidecar this
    # function used to fetch could never have existed — the fail-closed branch
    # fired on every run and wasi-sdk was simply never installable.
    #
    # These values were computed from the published tarballs and then
    # corroborated against independent third parties that pin the same
    # artefacts, so they are not merely trust-on-first-use from one download:
    # `spack/spack-packages` records the identical sha256 for 25.0 x86_64-linux,
    # and `wevm/ox`'s `wasm/toolchain.json` records all four.
    #
    # To move `WASI_SDK_VERSION`: download each tarball, hash it, and replace
    # the four values below — mismatches are fatal by design.
    local expected_sha
    case "${sdk_arch}-${sdk_os}" in
        x86_64-linux) expected_sha="52640dde13599bf127a95499e61d6d640256119456d1af8897ab6725bcf3d89c" ;;
        arm64-linux)  expected_sha="47fccad8b2498f2239e05e1115c3ffc652bf37e7de2f88fb64b2d663c976ce2d" ;;
        x86_64-macos) expected_sha="55e3ff3fee1a15678a16eeccba0129276c9f6be481bc9c283e7f9f65bf055c11" ;;
        arm64-macos)  expected_sha="e1e529ea226b1db0b430327809deae9246b580fa3cae32d31c82dfe770233587" ;;
        *)
            echo "ERROR: no wasi-sdk sha256 pin for ${sdk_arch}-${sdk_os}" >&2
            return 1
            ;;
    esac

    ensure_download_tools
    local major="${WASI_SDK_VERSION%%.*}"
    local tarball="wasi-sdk-${WASI_SDK_VERSION}-${sdk_arch}-${sdk_os}.tar.gz"
    local base="https://github.com/WebAssembly/wasi-sdk/releases/download/wasi-sdk-${major}"
    local tmpdir
    tmpdir="$(mktemp -d)"
    # shellcheck disable=SC2064
    trap "rm -rf '$tmpdir'" RETURN

    info "Downloading wasi-sdk ${WASI_SDK_VERSION} (${sdk_arch}-${sdk_os})"
    fetch_with_retry "${base}/${tarball}" "$tmpdir/$tarball"

    # Still fail closed: a tarball that does not match its pin is refused rather
    # than installed on trust.
    local actual_sha
    actual_sha="$(sha256_file "$tmpdir/$tarball")"
    if [ "$actual_sha" != "$expected_sha" ]; then
        echo "ERROR: wasi-sdk sha256 mismatch (expected $expected_sha, got $actual_sha)" >&2
        return 1
    fi
    info "wasi-sdk checksum verified against the pin for ${sdk_arch}-${sdk_os}"

    local prefix="/opt/wasi-sdk-${WASI_SDK_VERSION}"
    $SUDO rm -rf "$prefix"
    $SUDO mkdir -p "$prefix"
    $SUDO tar -xzf "$tmpdir/$tarball" -C "$prefix" --strip-components=1
    $SUDO ln -sfn "$prefix" "$link"
    info "Installed wasi-sdk to ${prefix} (symlinked ${link}); runtime/rust/build.rs auto-discovers it"
}

# ---------------------------------------------------------------- tshark

ensure_tshark() {
    if [ "${SKIP_TSHARK:-}" = "1" ]; then info "SKIP_TSHARK=1 — skipping tshark"; return 0; fi
    if command -v tshark >/dev/null 2>&1; then
        info "tshark already on PATH ($(tshark --version 2>/dev/null | head -1))"
        return 0
    fi
    case "$PKG" in
        apt-get)
            # ``apt-get install tshark`` pulls in wireshark-common, whose
            # postinst pops a debconf prompt asking whether non-superusers
            # should be allowed to capture packets.  In a non-interactive
            # CI run that prompt blocks forever, so we pre-seed the
            # answer (``false`` keeps the safer default — only root can
            # capture) and force noninteractive frontend before running
            # the install.
            if [ "$CHECK_ONLY" -eq 1 ]; then
                note_missing "tshark (would: apt-get install tshark, debconf preseed)"
            else
                refresh_pkg_index
                if command -v debconf-set-selections >/dev/null 2>&1; then
                    echo "wireshark-common wireshark-common/install-setuid boolean false" \
                        | $SUDO debconf-set-selections
                fi
                info "Installing tshark (apt, noninteractive)"
                $SUDO env DEBIAN_FRONTEND=noninteractive \
                    apt-get install -y --no-install-recommends tshark
            fi
            ;;
        dnf|yum) run_install "tshark (dnf)" wireshark-cli ;;
        brew)    run_install "Wireshark (Homebrew)" wireshark ;;
    esac
}

# ---------------------------------------------------------------- editor / native integration tools

ensure_emacs() {
    if [ "${SKIP_EMACS:-}" = "1" ]; then info "SKIP_EMACS=1 — skipping emacs"; return 0; fi
    if command -v emacs >/dev/null 2>&1; then
        info "emacs already on PATH ($(emacs --version 2>/dev/null | head -1 || echo unknown))"
        return 0
    fi
    case "$PKG" in
        apt-get) run_install "Emacs (apt)" emacs-nox ;;
        dnf|yum) run_install "Emacs (dnf)" emacs-nox ;;
        brew)    run_install "Emacs (Homebrew)" emacs ;;
    esac
}

ensure_xvfb() {
    if [ "${SKIP_XVFB:-}" = "1" ]; then info "SKIP_XVFB=1 — skipping xvfb"; return 0; fi
    if [ "$OS" != "Linux" ]; then
        info "xvfb not needed on $OS"
        return 0
    fi
    if [ -n "${DISPLAY:-}" ]; then
        info "DISPLAY is set — xvfb-run not needed"
        return 0
    fi
    if command -v xvfb-run >/dev/null 2>&1; then
        info "xvfb-run already on PATH"
        return 0
    fi
    case "$PKG" in
        apt-get) run_install "xvfb (apt)" xvfb ;;
        dnf|yum) run_install "xvfb (dnf)" xorg-x11-server-Xvfb ;;
    esac
}

ensure_openssl() {
    if [ "${SKIP_OPENSSL:-}" = "1" ]; then info "SKIP_OPENSSL=1 — skipping openssl"; return 0; fi
    if command -v openssl >/dev/null 2>&1; then
        info "openssl already on PATH ($(openssl version 2>/dev/null || echo unknown))"
        return 0
    fi
    case "$PKG" in
        apt-get) run_install "OpenSSL (apt)" openssl ;;
        dnf|yum) run_install "OpenSSL (dnf)" openssl ;;
        brew)    run_install "OpenSSL (Homebrew)" openssl ;;
    esac
}

ensure_ping() {
    if [ "${SKIP_PING:-}" = "1" ]; then info "SKIP_PING=1 — skipping ping"; return 0; fi
    if command -v ping >/dev/null 2>&1; then
        info "ping already on PATH"
        return 0
    fi
    case "$PKG" in
        apt-get) run_install "ping (apt)" iputils-ping ;;
        dnf|yum) run_install "ping (dnf)" iputils ;;
        brew)
            if [ "$CHECK_ONLY" -eq 1 ]; then
                note_missing "ping (system networking tool missing)"
            else
                echo "ERROR: ping is missing; install the macOS system networking tools." >&2
                return 1
            fi
            ;;
    esac
}

ensure_rgxg() {
    if [ "${SKIP_RGXG:-}" = "1" ]; then info "SKIP_RGXG=1 — skipping rgxg"; return 0; fi
    if command -v rgxg >/dev/null 2>&1; then
        info "rgxg already on PATH"
        return 0
    fi
    case "$PKG" in
        apt-get) run_install "rgxg (apt)" rgxg ;;
        dnf|yum) run_install "rgxg (dnf)" rgxg ;;
        brew)    run_install "rgxg (Homebrew)" rgxg ;;
    esac
}

# uv — the Python environment manager used by the repo's remaining Python
# dev scripts.  No distro packages it reliably, so on Linux we use Astral's
# official installer (drops the binary in ~/.local/bin); macOS gets the
# Homebrew formula.
ensure_uv() {
    if [ "${SKIP_UV:-}" = "1" ]; then info "SKIP_UV=1 — skipping uv"; return 0; fi
    if command -v uv >/dev/null 2>&1; then
        info "uv already on PATH ($(uv --version 2>/dev/null || echo unknown))"
        return 0
    fi
    if [ "$CHECK_ONLY" -eq 1 ]; then
        note_missing "uv (Python environment manager)"
        return 0
    fi
    case "$PKG" in
        brew)
            run_install "uv (Homebrew)" uv
            ;;
        *)
            info "Installing uv via Astral installer (~/.local/bin)"
            if ! curl -fsSL --connect-timeout 15 --max-time 600 \
                    https://astral.sh/uv/install.sh | sh; then
                warn "uv install script failed"
                note_missing "uv (Python environment manager)"
                return 1
            fi
            ;;
    esac
}


# ---------------------------------------------------------------- main

ensure_tclsh
ensure_tcllib
ensure_python_tk
ensure_node
ensure_kotlinc
ensure_rust
ensure_wasmtime
ensure_binaryen
ensure_wasi_sdk
ensure_tshark
ensure_emacs
ensure_xvfb
ensure_openssl
ensure_ping
ensure_rgxg
ensure_uv

if [ "$CHECK_ONLY" -eq 1 ] && [ "${#missing[@]}" -gt 0 ]; then
    echo
    echo "ensure-test-deps: missing ${#missing[@]} optional dependencies:"
    for m in "${missing[@]}"; do echo "  - $m"; done
    exit 1
fi

info "ensure-test-deps: all dependencies satisfied"
