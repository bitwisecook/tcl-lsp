#!/bin/bash
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

# SessionStart hook for tcl-lsp.
#
# Claude Code on the web runs in a sandbox that ships without some
# system tools and language toolchains the Makefile depends on.  This
# hook installs anything missing so `make prep-pr`, `make test-ext`,
# `make smoke-vsix`, bytecode comparison, and Wasmtime-based harnesses
# keep working across fresh sessions.
#
# Runs only in remote sessions — skips locally so developer machines
# are never touched.

set -euo pipefail

# Skip outside Claude Code on the web.
if [ "${CLAUDE_CODE_REMOTE:-}" != "true" ]; then
    exit 0
fi

ARCH="$(uname -m)"
REPO_ROOT="${CLAUDE_PROJECT_DIR:-$(cd "$(dirname "$0")/../.." && pwd)}"

# Pinned toolchain versions. Bump these when new stable releases land.
WASMTIME_VERSION="47.0.3"
BINARYEN_VERSION="132"
WASI_SDK_VERSION="33.0"
# Rust tracks the floating `stable` channel to match `rust-toolchain.toml`
# (see docs/rust-rewrite.md). Installing the channel — rather than a pinned
# version — keeps it auto-updating to the latest stable and, critically,
# means the wasm32-wasip2 target lands on the toolchain the project actually
# uses. No manual bump needed when a new stable lands.
RUST_TOOLCHAIN="stable"
TCLLIB_TAG="tcllib-2-0"
TCLLIB_VERSION="2.0"

WASMTIME_PREFIX="/opt/wasmtime-${WASMTIME_VERSION}"
BINARYEN_PREFIX="/opt/binaryen-${BINARYEN_VERSION}"
WASI_SDK_PREFIX="/opt/wasi-sdk-${WASI_SDK_VERSION}"

# ---------------------------------------------------------------------------
# 1. System packages (apt).
# ---------------------------------------------------------------------------
declare -A REQUIRED_PKGS=(
    [curl]="curl"
    [rsync]="rsync"
    [xz]="xz-utils"
)

# ca-certificates is required for TLS verification on any HTTPS fetch
# below but doesn't ship its own canonical binary, so check for the
# bundle directly.
if [ ! -f /etc/ssl/certs/ca-certificates.crt ]; then
    REQUIRED_PKGS[ca-certificates]="ca-certificates"
fi

missing_pkgs=()
for bin in "${!REQUIRED_PKGS[@]}"; do
    if ! command -v "$bin" >/dev/null 2>&1; then
        missing_pkgs+=("${REQUIRED_PKGS[$bin]}")
    fi
done

if [ "${#missing_pkgs[@]}" -gt 0 ]; then
    echo "session-start: installing missing system packages: ${missing_pkgs[*]}"
    export DEBIAN_FRONTEND=noninteractive
    if [ "$(id -u)" -eq 0 ]; then
        APT="apt-get"
    else
        APT="sudo apt-get"
    fi
    $APT update -qq
    $APT install -y -qq --no-install-recommends "${missing_pkgs[@]}"
else
    echo "session-start: all required system packages already present"
fi

# ---------------------------------------------------------------------------
# 2. Download helper — retries with exponential backoff, be kind to mirrors.
# ---------------------------------------------------------------------------
fetch_with_retry() {
    local url="$1"
    local dest="$2"
    local attempt
    for attempt in 1 2 3 4; do
        if curl -fsSL --retry 0 --connect-timeout 15 --max-time 600 \
               -o "$dest" "$url"; then
            return 0
        fi
        if [ "$attempt" -lt 4 ]; then
            local wait=$((2 ** attempt))
            echo "    retry $attempt (waiting ${wait}s): $url"
            sleep "$wait"
        fi
    done
    return 1
}

# Cleanup handler for the installers' `local tmpdir` scratch directories.
#
# Bash `RETURN` traps are *global* unless `set -o functrace` is in effect, so a
# trap set inside one installer also fires when the **next** function returns —
# by which point that installer's `local tmpdir` is out of scope and `set -u`
# aborts the whole hook with `tmpdir: unbound variable`.  That defeats
# `run_step`, whose entire purpose is to isolate a failing step so the
# remaining ones still run: one failed download took out every later step
# (Tcl sources, tcllib, test tools).  Guard on the variable being set, and
# deregister the trap once it has run so it cannot fire a second time.
cleanup_tmpdir() {
    if [ -n "${tmpdir:-}" ]; then
        rm -rf "$tmpdir"
    fi
    trap - RETURN
}

# ---------------------------------------------------------------------------
# 4. Wasmtime — pinned release from GitHub.
# ---------------------------------------------------------------------------
install_wasmtime() {
    if [ -x "${WASMTIME_PREFIX}/wasmtime" ] && [ -L /usr/local/bin/wasmtime ] \
       && [ "$(readlink -f /usr/local/bin/wasmtime)" = "${WASMTIME_PREFIX}/wasmtime" ]; then
        echo "session-start: wasmtime v${WASMTIME_VERSION} already installed"
        return 0
    fi

    case "$ARCH" in
        x86_64)  local wasm_arch="x86_64-linux" ;;
        aarch64) local wasm_arch="aarch64-linux" ;;
        *) echo "session-start: unsupported arch for wasmtime: $ARCH" >&2; return 1 ;;
    esac

    local tarball="wasmtime-v${WASMTIME_VERSION}-${wasm_arch}.tar.xz"
    local url="https://github.com/bytecodealliance/wasmtime/releases/download/v${WASMTIME_VERSION}/${tarball}"
    local tmpdir
    tmpdir="$(mktemp -d)"
    trap cleanup_tmpdir RETURN

    echo "session-start: fetching wasmtime v${WASMTIME_VERSION}"
    if ! fetch_with_retry "$url" "${tmpdir}/${tarball}"; then
        echo "session-start: failed to download wasmtime v${WASMTIME_VERSION}" >&2
        return 1
    fi

    # Wasmtime releases do not publish SHA-256 sidecars, so we pin the
    # hashes here alongside the version.  Re-compute these when bumping
    # WASMTIME_VERSION above.
    local expected_sha=""
    case "$wasm_arch" in
        x86_64-linux)
            expected_sha="ca1fc56d1afc40c8782e96c297fd182a0da162f9a8f52a1e7b094e1dd648e178" ;;
        aarch64-linux)
            expected_sha="497b518db00ae585f04390758eaa99ad555bee50612dce7d102602778fb46ff0" ;;
    esac
    if [ -z "$expected_sha" ]; then
        echo "session-start: no pinned wasmtime sha256 for ${wasm_arch}" >&2
        return 1
    fi
    local actual_sha
    actual_sha="$(sha256sum "${tmpdir}/${tarball}" | awk '{print $1}')"
    if [ "$actual_sha" != "$expected_sha" ]; then
        echo "session-start: wasmtime sha256 mismatch (expected $expected_sha, got $actual_sha)" >&2
        return 1
    fi

    rm -rf "$WASMTIME_PREFIX"
    mkdir -p "$WASMTIME_PREFIX"
    tar -xJf "${tmpdir}/${tarball}" -C "$WASMTIME_PREFIX" --strip-components=1
    ln -sfn "${WASMTIME_PREFIX}/wasmtime" /usr/local/bin/wasmtime
    echo "session-start: wasmtime $(${WASMTIME_PREFIX}/wasmtime --version) installed at ${WASMTIME_PREFIX}"
}

# ---------------------------------------------------------------------------
# 4b. Binaryen — provides ``wasm-merge`` (post-codegen bundler that fuses
#     runtime + user code + optional extensions into a single .wasm) and
#     ``wasm-opt`` (asyncify post-pass for the coroutine build).  Skipped
#     when both binaries already point at the pinned prefix.
# ---------------------------------------------------------------------------
install_binaryen() {
    if [ -x "${BINARYEN_PREFIX}/bin/wasm-merge" ] \
       && [ -L /usr/local/bin/wasm-merge ] \
       && [ "$(readlink -f /usr/local/bin/wasm-merge)" = "${BINARYEN_PREFIX}/bin/wasm-merge" ]; then
        echo "session-start: binaryen v${BINARYEN_VERSION} already installed"
        return 0
    fi

    case "$ARCH" in
        x86_64)  local bin_arch="x86_64-linux" ;;
        aarch64) local bin_arch="aarch64-linux" ;;
        *) echo "session-start: unsupported arch for binaryen: $ARCH" >&2; return 1 ;;
    esac

    local tarball="binaryen-version_${BINARYEN_VERSION}-${bin_arch}.tar.gz"
    local url="https://github.com/WebAssembly/binaryen/releases/download/version_${BINARYEN_VERSION}/${tarball}"
    local tmpdir
    tmpdir="$(mktemp -d)"
    trap cleanup_tmpdir RETURN

    echo "session-start: fetching binaryen v${BINARYEN_VERSION}"
    if ! fetch_with_retry "$url" "${tmpdir}/${tarball}"; then
        echo "session-start: failed to download binaryen v${BINARYEN_VERSION}" >&2
        return 1
    fi

    # Binaryen releases do not publish SHA-256 sidecars; pin alongside
    # the version like wasmtime above.  Re-compute when bumping
    # BINARYEN_VERSION.  Fail closed: an arch with no pin is a policy hole,
    # not a licence to install unverified, so refuse rather than skip.
    local expected_sha=""
    case "$bin_arch" in
        x86_64-linux)
            expected_sha="195ddc94f9bc89f45abdabb0b9eea86023d727ba90eac8b35b80f2544fc30572" ;;
        aarch64-linux)
            expected_sha="c58562417836c5d0493d89bdefc434933bdc097db641b483df86bcfa557a107f" ;;
    esac
    local actual_sha
    actual_sha="$(sha256sum "${tmpdir}/${tarball}" | awk '{print $1}')"
    if [ -z "$expected_sha" ]; then
        echo "session-start: no pinned binaryen sha256 for ${bin_arch}; refusing to install unverified." >&2
        echo "session-start: downloaded artifact hashes to ${actual_sha} — pin it in install_binaryen and re-run." >&2
        return 1
    fi
    if [ "$actual_sha" != "$expected_sha" ]; then
        echo "session-start: binaryen sha256 mismatch (expected $expected_sha, got $actual_sha)" >&2
        return 1
    fi

    rm -rf "$BINARYEN_PREFIX"
    mkdir -p "$BINARYEN_PREFIX"
    tar -xzf "${tmpdir}/${tarball}" -C "$BINARYEN_PREFIX" --strip-components=1
    ln -sfn "${BINARYEN_PREFIX}/bin/wasm-merge" /usr/local/bin/wasm-merge
    ln -sfn "${BINARYEN_PREFIX}/bin/wasm-opt"   /usr/local/bin/wasm-opt
    echo "session-start: binaryen $(${BINARYEN_PREFIX}/bin/wasm-merge --version | head -n1) installed at ${BINARYEN_PREFIX}"
}

# ---------------------------------------------------------------------------
# 4b. wasi-sdk — clang + WASI sysroot for the wasm cross-compile of libtommath
#     (the numeric tower) in runtime/rust/build.rs.  Installed to
#     /opt/wasi-sdk-<ver> and symlinked /opt/wasi-sdk, which build.rs
#     auto-discovers.  Without it the wasm runtime build drops the tower.
# ---------------------------------------------------------------------------
install_wasi_sdk() {
    if [ -x "${WASI_SDK_PREFIX}/bin/clang" ] && [ -L /opt/wasi-sdk ] \
       && [ "$(readlink -f /opt/wasi-sdk)" = "${WASI_SDK_PREFIX}" ]; then
        echo "session-start: wasi-sdk ${WASI_SDK_VERSION} already installed"
        return 0
    fi

    case "$ARCH" in
        x86_64)  local sdk_arch="x86_64" ;;
        aarch64) local sdk_arch="arm64" ;;
        *) echo "session-start: unsupported arch for wasi-sdk: $ARCH" >&2; return 1 ;;
    esac

    local major="${WASI_SDK_VERSION%%.*}"
    local tarball="wasi-sdk-${WASI_SDK_VERSION}-${sdk_arch}-linux.tar.gz"
    local base="https://github.com/WebAssembly/wasi-sdk/releases/download/wasi-sdk-${major}"
    local tmpdir
    tmpdir="$(mktemp -d)"
    trap cleanup_tmpdir RETURN

    echo "session-start: fetching wasi-sdk ${WASI_SDK_VERSION}"
    if ! fetch_with_retry "${base}/${tarball}" "${tmpdir}/${tarball}"; then
        echo "session-start: failed to download wasi-sdk ${WASI_SDK_VERSION}" >&2
        return 1
    fi

    # The wasi-sdk releases publish no SHA-256 sidecar: the `SHA256SUMS` asset
    # this step used to fetch returns 404, which failed the step closed and
    # meant wasi-sdk never installed (so `runtime/rust/build.rs` silently built
    # every wasm artefact without the libtommath numeric tower).  Pin the
    # hashes here alongside the version, exactly as wasmtime and binaryen above
    # do; re-compute them when bumping WASI_SDK_VERSION.  Still fail closed: an
    # arch with no pin is refused rather than installed unverified.
    local expected actual
    case "$sdk_arch" in
        x86_64)
            expected="0ba8b5bfaeb2adf3f29bab5841d76cf5318ab8e1642ea195f88baba1abd47bce" ;;
        arm64)
            expected="4f98ee738c7abb45c81a94d1461fc53cc569d1cd01498951c8184d841a027844" ;;
        *)
            expected="" ;;
    esac
    actual="$(sha256sum "${tmpdir}/${tarball}" | awk '{print $1}')"
    if [ -z "$expected" ]; then
        echo "session-start: no pinned wasi-sdk sha256 for ${sdk_arch}; refusing to install unverified." >&2
        return 1
    fi
    if [ "$actual" != "$expected" ]; then
        echo "session-start: wasi-sdk sha256 mismatch (expected $expected, got $actual)" >&2
        return 1
    fi

    rm -rf "$WASI_SDK_PREFIX"
    mkdir -p "$WASI_SDK_PREFIX"
    tar -xzf "${tmpdir}/${tarball}" -C "$WASI_SDK_PREFIX" --strip-components=1
    ln -sfn "$WASI_SDK_PREFIX" /opt/wasi-sdk
    echo "session-start: wasi-sdk $(${WASI_SDK_PREFIX}/bin/clang --version | head -n1) installed at ${WASI_SDK_PREFIX} (symlinked /opt/wasi-sdk)"
}

# ---------------------------------------------------------------------------
# 5. Tcl source trees (8.4, 8.5, 8.6, 9.0) — delegate to the existing skill.
#    Idempotent: skips versions already fetched into tmp/.
# ---------------------------------------------------------------------------
install_tcl_sources() {
    local fetcher="${REPO_ROOT}/.claude/skills/fetch-tcl-source/fetch_tcl_source.sh"
    if [ ! -x "$fetcher" ]; then
        echo "session-start: fetch-tcl-source skill missing at $fetcher" >&2
        return 1
    fi
    echo "session-start: ensuring Tcl 8.4 / 8.5 / 8.6 / 9.0 / 9.1 source trees"
    bash "$fetcher" all
}

# ---------------------------------------------------------------------------
# 5a. Tk source trees, matching the Tcl releases above — same skill, the
#     tk* selectors. Idempotent: skips versions already fetched into tmp/.
# ---------------------------------------------------------------------------
install_tk_sources() {
    local fetcher="${REPO_ROOT}/.claude/skills/fetch-tcl-source/fetch_tcl_source.sh"
    if [ ! -x "$fetcher" ]; then
        echo "session-start: fetch-tcl-source skill missing at $fetcher" >&2
        return 1
    fi
    echo "session-start: ensuring Tk 8.4 / 8.5 / 8.6 / 9.0 / 9.1 source trees"
    bash "$fetcher" tkall
}

# ---------------------------------------------------------------------------
# 5b. tcllib — latest stable, full source tarball from GitHub codeload.
#     Extracted to tmp/tcllib-<version>/ alongside the tcl source trees.
# ---------------------------------------------------------------------------
install_tcllib() {
    local target_dir="${REPO_ROOT}/tmp/tcllib-${TCLLIB_VERSION}"
    if [ -d "${target_dir}/modules" ]; then
        echo "session-start: tcllib ${TCLLIB_VERSION} already present at ${target_dir}"
        return 0
    fi

    local url="https://codeload.github.com/tcltk/tcllib/tar.gz/refs/tags/${TCLLIB_TAG}"
    mkdir -p "${REPO_ROOT}/tmp"
    local tmp_tarball
    tmp_tarball="$(mktemp -p "${REPO_ROOT}/tmp" "tcllib-${TCLLIB_VERSION}.XXXXXX.tar.gz")"
    # shellcheck disable=SC2064
    trap "rm -f '$tmp_tarball'" RETURN

    echo "session-start: downloading tcllib ${TCLLIB_VERSION} tarball"
    if ! fetch_with_retry "$url" "$tmp_tarball"; then
        echo "session-start: failed to download tcllib ${TCLLIB_VERSION}" >&2
        return 1
    fi

    rm -rf "$target_dir"
    mkdir -p "$target_dir"
    tar -xzf "$tmp_tarball" -C "$target_dir" --strip-components=1

    if [ ! -d "${target_dir}/modules" ]; then
        echo "session-start: tcllib modules/ missing after extract" >&2
        rm -rf "$target_dir"
        return 1
    fi
    local size
    size=$(du -sh "$target_dir" | awk '{print $1}')
    echo "session-start: tcllib ${TCLLIB_VERSION} extracted to ${target_dir} (${size})"
}

# ---------------------------------------------------------------------------
# 6. Rust toolchain — rustup + floating `stable` (matches rust-toolchain.toml).
#    Uses the official rust-lang.org installer so we get signed binaries.
# ---------------------------------------------------------------------------
install_rust() {
    # Default RUSTUP_HOME/CARGO_HOME under the invoking user's $HOME so the
    # hook works equally in root and non-root remote sandboxes.  If HOME is
    # unset for some reason, fall back to the HOME field in /etc/passwd.
    local user_home="${HOME:-}"
    if [ -z "$user_home" ]; then
        user_home="$(getent passwd "$(id -u)" 2>/dev/null | awk -F: '{print $6}')"
    fi
    user_home="${user_home:-/root}"
    export RUSTUP_HOME="${RUSTUP_HOME:-${user_home}/.rustup}"
    export CARGO_HOME="${CARGO_HOME:-${user_home}/.cargo}"

    # Resolve the rustup binary we'll actually use.  Prefer one already on
    # PATH (covers distro installs and pre-warmed containers) and fall
    # back to the one we bootstrap under CARGO_HOME.  Never assume both
    # paths exist — the install below may create only one of them.
    local rustup_bin
    rustup_bin="$(command -v rustup 2>/dev/null || true)"
    if [ -z "$rustup_bin" ] && [ -x "${CARGO_HOME}/bin/rustup" ]; then
        rustup_bin="${CARGO_HOME}/bin/rustup"
    fi

    if [ -z "$rustup_bin" ]; then
        case "$ARCH" in
            x86_64)  local rust_arch="x86_64-unknown-linux-gnu" ;;
            aarch64) local rust_arch="aarch64-unknown-linux-gnu" ;;
            *) echo "session-start: unsupported arch for rust: $ARCH" >&2; return 1 ;;
        esac

        local tmpdir
        tmpdir="$(mktemp -d)"
        trap cleanup_tmpdir RETURN

        local rustup_url="https://static.rust-lang.org/rustup/dist/${rust_arch}/rustup-init"

        echo "session-start: fetching rustup-init (${rust_arch})"
        if ! fetch_with_retry "$rustup_url" "${tmpdir}/rustup-init"; then
            echo "session-start: failed to download rustup-init" >&2
            return 1
        fi
        if ! fetch_with_retry "${rustup_url}.sha256" "${tmpdir}/rustup-init.sha256"; then
            echo "session-start: failed to download rustup-init sha256 sidecar" >&2
            return 1
        fi
        # sha256sum -c expects the referenced file to live next to the
        # checksum file under the name recorded in it, so verify from
        # within tmpdir.
        if ! ( cd "$tmpdir" && sha256sum -c rustup-init.sha256 >/dev/null ); then
            echo "session-start: rustup-init checksum verification failed" >&2
            return 1
        fi
        chmod +x "${tmpdir}/rustup-init"

        echo "session-start: installing rustup + rust ${RUST_TOOLCHAIN}"
        "${tmpdir}/rustup-init" \
            -y \
            --no-modify-path \
            --profile minimal \
            --default-toolchain "${RUST_TOOLCHAIN}" \
            --component rustfmt,clippy

        rustup_bin="${CARGO_HOME}/bin/rustup"
    fi

    if [ ! -x "$rustup_bin" ]; then
        echo "session-start: rustup not executable at $rustup_bin" >&2
        return 1
    fi

    # Make sure the requested toolchain is present even on warm containers.
    # static.rust-lang.org can 503 briefly, so retry with exponential backoff.
    local attempt
    for attempt in 1 2 3 4; do
        if "$rustup_bin" toolchain install "${RUST_TOOLCHAIN}" \
                --profile minimal --component rustfmt --component clippy \
                --no-self-update; then
            break
        fi
        if [ "$attempt" -lt 4 ]; then
            local wait=$((2 ** attempt))
            echo "session-start: rustup retry $attempt (waiting ${wait}s) ..."
            sleep "$wait"
        else
            echo "session-start: failed to install rust ${RUST_TOOLCHAIN} after 4 attempts" >&2
            return 1
        fi
    done
    "$rustup_bin" default "${RUST_TOOLCHAIN}"

    # The Zed extension's clippy check (`make check-rust`) cross-compiles
    # to wasm32-wasip2, so the target has to be present in the toolchain.
    # Idempotent — rustup skips already-installed targets.
    for attempt in 1 2 3 4; do
        if "$rustup_bin" target add wasm32-wasip2 \
                --toolchain "${RUST_TOOLCHAIN}"; then
            break
        fi
        if [ "$attempt" -lt 4 ]; then
            local wait=$((2 ** attempt))
            echo "session-start: rustup target retry $attempt (waiting ${wait}s) ..."
            sleep "$wait"
        else
            echo "session-start: failed to install wasm32-wasip2 target after 4 attempts" >&2
            return 1
        fi
    done

    # Resolve the cargo/rustc binaries the active rustup is configured
    # to front so symlinks always match the toolchain we just installed.
    local cargo_bin rustc_bin rustfmt_bin clippy_bin
    cargo_bin="$("$rustup_bin" which cargo)"
    rustc_bin="$("$rustup_bin" which rustc)"
    rustfmt_bin="$("$rustup_bin" which rustfmt)"
    clippy_bin="$("$rustup_bin" which clippy-driver)"

    # Expose cargo/rustc without relying on PATH hacks in downstream shells.
    ln -sfn "$cargo_bin"   /usr/local/bin/cargo
    ln -sfn "$rustc_bin"   /usr/local/bin/rustc
    ln -sfn "$rustup_bin"  /usr/local/bin/rustup
    ln -sfn "$rustfmt_bin" /usr/local/bin/rustfmt
    ln -sfn "$clippy_bin"  /usr/local/bin/clippy-driver

    echo "session-start: rust $("$rustc_bin" --version) ready"
}

# ---------------------------------------------------------------------------
# 6. Remaining host test tools (tclsh, node, kotlinc, emacs, xvfb,
#    tshark, openssl, ping, rgxg, uv).  Delegated to the shared
#    cross-platform installer so there's a single source of truth; the
#    toolchains this hook installs bespoke above (with pinned versions +
#    checksums) are skipped to avoid double work.
# ---------------------------------------------------------------------------
install_remaining_test_deps() {
    local installer="${REPO_ROOT}/scripts/dev/ensure-test-deps.sh"
    if [ ! -f "$installer" ]; then
        echo "session-start: ensure-test-deps.sh missing at $installer" >&2
        return 1
    fi
    echo "session-start: installing remaining host test tools"
    env \
        SKIP_WASMTIME=1 \
        SKIP_BINARYEN=1 \
        SKIP_RUST=1 \
        SKIP_TCLLIB=1 \
        bash "$installer"
}

# ---------------------------------------------------------------------------
# 7. TCL_LIBRARY — point it at the fetched Tcl 9 script library.
#
# (The project retired Python: there is no pyproject.toml / uv.lock, so there
# is no venv to create — the old `uv sync --extra dev` step was removed.)
#
# ensure-test-deps builds tclsh9.0 with ``--disable-shared`` and installs
# only the ``tclsh`` binary (no ``make install``), so the script library
# is never laid down at the binary's compiled-in prefix.  Exporting
# TCL_LIBRARY to the source ``library/`` lets the moved binary find
# init.tcl etc.  Verified harmless to tclsh8.6 — Tcl falls back to its
# own bootstrap when the pointed-at library version mismatches.
# ---------------------------------------------------------------------------
setup_tcl_library() {
    local tcl_lib="${REPO_ROOT}/tmp/tcl9.0.4/library"
    if [ ! -f "${tcl_lib}/init.tcl" ]; then
        echo "session-start: Tcl 9 library not found at ${tcl_lib} — skipping TCL_LIBRARY" >&2
        return 0
    fi

    export TCL_LIBRARY="$tcl_lib"

    local marker="# tcl-lsp: point TCL_LIBRARY at the fetched Tcl 9 library"
    if [ -n "${HOME:-}" ] && ! grep -qsF "$marker" "${HOME}/.bashrc" 2>/dev/null; then
        {
            printf '\n%s\n' "$marker"
            printf 'export TCL_LIBRARY="%s"\n' "$tcl_lib"
        } >> "${HOME}/.bashrc"
        echo "session-start: TCL_LIBRARY export added to ~/.bashrc"
    fi
    echo "session-start: TCL_LIBRARY=${TCL_LIBRARY}"
}

# The setup steps are independent: install_wasmtime / install_binaryen /
# install_wasi_sdk pull separate GitHub downloads, and a transient failure in
# any one of them must NOT abort the hook before install_rust (which pins the
# toolchain to the required stable) or setup_tcl_library run. Under
# `set -euo pipefail` a bare `install_wasmtime; install_rust; ...` chain does
# exactly that — one flaky download leaves a stale pre-baked toolchain. Isolate
# each step so a failure is logged and the rest still run, while still
# surfacing failures (and a genuine everything-failed state) at the end.
FAILED_STEPS=()

run_step() {
    local step="$1"
    if "$step"; then
        return 0
    fi
    echo "session-start: step '$step' failed — continuing with remaining steps" >&2
    FAILED_STEPS+=("$step")
    return 0
}

STEPS=(
    install_wasmtime
    install_binaryen
    install_wasi_sdk
    install_rust
    install_tcl_sources
    install_tk_sources
    install_tcllib
    install_remaining_test_deps
    setup_tcl_library
)

for step in "${STEPS[@]}"; do
    run_step "$step"
done

if [ "${#FAILED_STEPS[@]}" -gt 0 ]; then
    echo "session-start: completed with failed steps: ${FAILED_STEPS[*]}" >&2
    # A genuine everything-failed state (no step succeeded) points at a broken
    # environment rather than an isolated network hiccup — surface it loudly.
    if [ "${#FAILED_STEPS[@]}" -eq "${#STEPS[@]}" ]; then
        echo "session-start: all setup steps failed — environment is not usable" >&2
        exit 1
    fi
fi

echo "session-start: done"
