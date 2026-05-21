#!/bin/bash
# SessionStart hook for tcl-lsp.
#
# Claude Code on the web runs in a sandbox that ships without some
# system tools and language toolchains the Makefile depends on.  This
# hook installs anything missing so `make prep-pr`, `make test-ext`,
# `make smoke-vsix`, bytecode comparison, the Zig WASM runtime build,
# and Wasmtime-based harnesses keep working across fresh sessions.
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
ZIG_VERSION="0.16.0"
WASMTIME_VERSION="43.0.1"
BINARYEN_VERSION="123"
RUST_VERSION="1.95.0"
TCLLIB_TAG="tcllib-2-0"
TCLLIB_VERSION="2.0"

ZIG_PREFIX="/opt/zig-${ZIG_VERSION}"
WASMTIME_PREFIX="/opt/wasmtime-${WASMTIME_VERSION}"
BINARYEN_PREFIX="/opt/binaryen-${BINARYEN_VERSION}"

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

# ---------------------------------------------------------------------------
# 3. Zig — prefer a community mirror, fall back to ziglang.org.
#
# Zig asks downloaders to be kind to their bandwidth and use a community
# mirror when possible (https://ziglang.org/download/community-mirrors/).
# We shuffle the mirror list and try each until one succeeds, then verify
# against the SHA-256 published by ziglang.org.
# ---------------------------------------------------------------------------
install_zig() {
    if [ -x "${ZIG_PREFIX}/zig" ] && [ -L /usr/local/bin/zig ] \
       && [ "$(readlink -f /usr/local/bin/zig)" = "${ZIG_PREFIX}/zig" ]; then
        echo "session-start: zig ${ZIG_VERSION} already installed"
        return 0
    fi

    case "$ARCH" in
        x86_64)  local zig_arch="x86_64-linux" ;;
        aarch64) local zig_arch="aarch64-linux" ;;
        *) echo "session-start: unsupported arch for zig: $ARCH" >&2; return 1 ;;
    esac

    local tarball="zig-${zig_arch}-${ZIG_VERSION}.tar.xz"
    local canonical="https://ziglang.org/download/${ZIG_VERSION}/${tarball}"
    local tmpdir
    tmpdir="$(mktemp -d)"
    trap 'rm -rf "$tmpdir"' RETURN

    # Shuffled community mirrors plus ziglang.org as the final fallback.
    # Mirror list snapshotted from https://ziglang.org/download/community-mirrors.txt
    # on 2026-04-20 and shuffled at download time.
    local mirrors=(
        "https://pkg.hexops.org/zig"
        "https://zigmirror.hryx.net/zig"
        "https://zig.linus.dev/zig"
        "https://zig.squirl.dev"
        "https://zig.mirror.mschae23.de/zig"
        "https://ziglang.freetls.fastly.net"
        "https://zig.tilok.dev"
        "https://zig-mirror.tsimnet.eu/zig"
        "https://zig.karearl.com/zig"
        "https://pkg.earth/zig"
        "https://fs.liujiacai.net/zigbuilds"
        "https://zigmirror.com"
        "https://zig.chainsafe.dev"
        "https://zig.savalione.com"
    )

    # Shuffle so we spread load across community mirrors.
    local shuffled
    mapfile -t shuffled < <(printf '%s\n' "${mirrors[@]}" | shuf)

    local ok=0
    for base in "${shuffled[@]}" ""; do
        local url
        if [ -n "$base" ]; then
            url="${base}/${tarball}?source=tcl-lsp-session-start"
            echo "session-start: fetching zig ${ZIG_VERSION} from community mirror ${base}"
        else
            url="$canonical"
            echo "session-start: falling back to ${canonical}"
        fi
        if fetch_with_retry "$url" "${tmpdir}/${tarball}"; then
            ok=1
            break
        fi
    done

    if [ "$ok" -ne 1 ]; then
        echo "session-start: failed to download zig ${ZIG_VERSION}" >&2
        return 1
    fi

    local expected_sha=""
    case "$zig_arch" in
        x86_64-linux)
            expected_sha="70e49664a74374b48b51e6f3fdfbf437f6395d42509050588bd49abe52ba3d00" ;;
        aarch64-linux)
            expected_sha="ea4b09bfb22ec6f6c6ceac57ab63efb6b46e17ab08d21f69f3a48b38e1534f17" ;;
    esac
    if [ -z "$expected_sha" ]; then
        echo "session-start: no pinned zig sha256 for ${zig_arch}" >&2
        return 1
    fi
    local actual_sha
    actual_sha="$(sha256sum "${tmpdir}/${tarball}" | awk '{print $1}')"
    if [ "$actual_sha" != "$expected_sha" ]; then
        echo "session-start: zig sha256 mismatch (expected $expected_sha, got $actual_sha)" >&2
        return 1
    fi

    rm -rf "$ZIG_PREFIX"
    mkdir -p "$ZIG_PREFIX"
    tar -xJf "${tmpdir}/${tarball}" -C "$ZIG_PREFIX" --strip-components=1
    ln -sfn "${ZIG_PREFIX}/zig" /usr/local/bin/zig
    echo "session-start: zig $(${ZIG_PREFIX}/zig version) installed at ${ZIG_PREFIX}"
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
    trap 'rm -rf "$tmpdir"' RETURN

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
            expected_sha="9f3cf977fc29e2ccab2d198435265b066dce3d608fc6692d700ed1b9b74c35a1" ;;
        aarch64-linux)
            expected_sha="dbf36d4e9108df377ddfb88f2d8db4e07efce9726b68da53ae78ed5579293923" ;;
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
    trap 'rm -rf "$tmpdir"' RETURN

    echo "session-start: fetching binaryen v${BINARYEN_VERSION}"
    if ! fetch_with_retry "$url" "${tmpdir}/${tarball}"; then
        echo "session-start: failed to download binaryen v${BINARYEN_VERSION}" >&2
        return 1
    fi

    # Binaryen releases do not publish SHA-256 sidecars; pin alongside
    # the version like wasmtime above.  Re-compute when bumping
    # BINARYEN_VERSION.
    local expected_sha=""
    case "$bin_arch" in
        x86_64-linux)
            expected_sha="e959f2170af4c20c552e9de3a0253704d6a9d2766e8fdb88e4d6ac4bae9388fe" ;;
        aarch64-linux)
            expected_sha="" ;; # fill on first aarch64 cold-start
    esac
    if [ -n "$expected_sha" ]; then
        local actual_sha
        actual_sha="$(sha256sum "${tmpdir}/${tarball}" | awk '{print $1}')"
        if [ "$actual_sha" != "$expected_sha" ]; then
            echo "session-start: binaryen sha256 mismatch (expected $expected_sha, got $actual_sha)" >&2
            return 1
        fi
    fi

    rm -rf "$BINARYEN_PREFIX"
    mkdir -p "$BINARYEN_PREFIX"
    tar -xzf "${tmpdir}/${tarball}" -C "$BINARYEN_PREFIX" --strip-components=1
    ln -sfn "${BINARYEN_PREFIX}/bin/wasm-merge" /usr/local/bin/wasm-merge
    ln -sfn "${BINARYEN_PREFIX}/bin/wasm-opt"   /usr/local/bin/wasm-opt
    echo "session-start: binaryen $(${BINARYEN_PREFIX}/bin/wasm-merge --version | head -n1) installed at ${BINARYEN_PREFIX}"
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
    echo "session-start: ensuring Tcl 8.4 / 8.5 / 8.6 / 9.0 source trees"
    bash "$fetcher" all
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
# 5c. Tcl regex engine sources — vendored at runtime/zig/vendor/tcl-regex/.
#     The WASM runtime build expects these files to be present (they're
#     compiled into the runtime); fetching once here keeps ``zig build``
#     hermetic and avoids the xdist race we'd otherwise hit if four
#     parallel worker processes each tried to fetch into the same dir.
# ---------------------------------------------------------------------------
install_tcl_regex() {
    local fetcher="${REPO_ROOT}/scripts/fetch_tcl_regex.sh"
    if [ ! -f "$fetcher" ]; then
        echo "session-start: fetch_tcl_regex.sh missing at $fetcher" >&2
        return 1
    fi
    echo "session-start: ensuring Tcl regex engine sources"
    bash "$fetcher"
}

# ---------------------------------------------------------------------------
# 6. Rust toolchain — rustup + stable (pinned).
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
        trap 'rm -rf "$tmpdir"' RETURN

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

        echo "session-start: installing rustup + rust ${RUST_VERSION}"
        "${tmpdir}/rustup-init" \
            -y \
            --no-modify-path \
            --profile minimal \
            --default-toolchain "${RUST_VERSION}" \
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
        if "$rustup_bin" toolchain install "${RUST_VERSION}" \
                --profile minimal --component rustfmt --component clippy \
                --no-self-update; then
            break
        fi
        if [ "$attempt" -lt 4 ]; then
            local wait=$((2 ** attempt))
            echo "session-start: rustup retry $attempt (waiting ${wait}s) ..."
            sleep "$wait"
        else
            echo "session-start: failed to install rust ${RUST_VERSION} after 4 attempts" >&2
            return 1
        fi
    done
    "$rustup_bin" default "${RUST_VERSION}"

    # The Zed extension's clippy check (`make check-rust`, used by
    # `make test-slow`) cross-compiles to wasm32-wasip2, so the target
    # has to be present in the toolchain.  Idempotent — rustup skips
    # already-installed targets.
    for attempt in 1 2 3 4; do
        if "$rustup_bin" target add wasm32-wasip2 \
                --toolchain "${RUST_VERSION}"; then
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
# 6. Remaining test-slow host tools (tclsh, node, kotlinc, emacs, xvfb,
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
    echo "session-start: installing remaining test-slow host tools"
    env \
        SKIP_ZIG=1 \
        SKIP_WASMTIME=1 \
        SKIP_BINARYEN=1 \
        SKIP_RUST=1 \
        SKIP_TCLLIB=1 \
        SKIP_TCL_REGEX=1 \
        bash "$installer"
}

# ---------------------------------------------------------------------------
# 7. Python venv — create the single well-known environment (.venv at the
#    repo root, the path uv and the Makefile already assume) and activate
#    it for this and every subsequent shell in the session.
# ---------------------------------------------------------------------------
setup_python_venv() {
    # uv was just installed by install_remaining_test_deps (Astral
    # installer drops it in ~/.local/bin); make sure it's reachable.
    export PATH="${HOME}/.local/bin:${PATH}"
    if ! command -v uv >/dev/null 2>&1; then
        echo "session-start: uv not found after install — skipping venv setup" >&2
        return 1
    fi

    echo "session-start: creating Python venv at ${REPO_ROOT}/.venv (uv sync)"
    ( cd "$REPO_ROOT" && uv sync --extra dev )

    local activate="${REPO_ROOT}/.venv/bin/activate"
    if [ ! -f "$activate" ]; then
        echo "session-start: expected venv activate script missing at $activate" >&2
        return 1
    fi

    # Activate for the rest of this hook.
    # shellcheck disable=SC1090
    . "$activate"

    # Persist activation for every subsequent Bash tool-call shell, which
    # are spawned fresh from the user's profile.  Idempotent: the guard
    # comment keeps re-runs on warm containers from stacking duplicates.
    local marker="# tcl-lsp: auto-activate project venv"
    if [ -n "${HOME:-}" ] && ! grep -qsF "$marker" "${HOME}/.bashrc" 2>/dev/null; then
        {
            printf '\n%s\n' "$marker"
            printf 'export PATH="%s/.local/bin:$PATH"\n' "$HOME"
            printf '[ -f "%s/.venv/bin/activate" ] && . "%s/.venv/bin/activate"\n' \
                "$REPO_ROOT" "$REPO_ROOT"
        } >> "${HOME}/.bashrc"
        echo "session-start: venv auto-activation added to ~/.bashrc"
    fi
    echo "session-start: venv ready (VIRTUAL_ENV=${VIRTUAL_ENV:-unset})"
}

# ---------------------------------------------------------------------------
# 8. TCL_LIBRARY — point it at the fetched Tcl 9 script library.
#
# ensure-test-deps builds tclsh9.0 with ``--disable-shared`` and installs
# only the ``tclsh`` binary (no ``make install``), so the script library
# is never laid down at the binary's compiled-in prefix.  Exporting
# TCL_LIBRARY to the source ``library/`` lets the moved binary find
# init.tcl etc.  Verified harmless to tclsh8.6 — Tcl falls back to its
# own bootstrap when the pointed-at library version mismatches.
# ---------------------------------------------------------------------------
setup_tcl_library() {
    local tcl_lib="${REPO_ROOT}/tmp/tcl9.0.3/library"
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

install_zig
install_wasmtime
install_binaryen
install_rust
install_tcl_sources
install_tcllib
install_tcl_regex
install_remaining_test_deps
setup_python_venv
setup_tcl_library

echo "session-start: done"
