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
RUST_VERSION="1.95.0"
TCLLIB_TAG="tcllib-2-0"
TCLLIB_VERSION="2.0"

ZIG_PREFIX="/opt/zig-${ZIG_VERSION}"
WASMTIME_PREFIX="/opt/wasmtime-${WASMTIME_VERSION}"

# ---------------------------------------------------------------------------
# 1. System packages (apt).
# ---------------------------------------------------------------------------
declare -A REQUIRED_PKGS=(
    [rsync]="rsync"
    [xz]="xz-utils"
)

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
    if [ "$zig_arch" = "x86_64-linux" ]; then
        expected_sha="70e49664a74374b48b51e6f3fdfbf437f6395d42509050588bd49abe52ba3d00"
    fi
    if [ -n "$expected_sha" ]; then
        local actual_sha
        actual_sha="$(sha256sum "${tmpdir}/${tarball}" | awk '{print $1}')"
        if [ "$actual_sha" != "$expected_sha" ]; then
            echo "session-start: zig sha256 mismatch (expected $expected_sha, got $actual_sha)" >&2
            return 1
        fi
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

    rm -rf "$WASMTIME_PREFIX"
    mkdir -p "$WASMTIME_PREFIX"
    tar -xJf "${tmpdir}/${tarball}" -C "$WASMTIME_PREFIX" --strip-components=1
    ln -sfn "${WASMTIME_PREFIX}/wasmtime" /usr/local/bin/wasmtime
    echo "session-start: wasmtime $(${WASMTIME_PREFIX}/wasmtime --version) installed at ${WASMTIME_PREFIX}"
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
# 6. Rust toolchain — rustup + stable (pinned).
#    Uses the official rust-lang.org installer so we get signed binaries.
# ---------------------------------------------------------------------------
install_rust() {
    export RUSTUP_HOME="${RUSTUP_HOME:-/root/.rustup}"
    export CARGO_HOME="${CARGO_HOME:-/root/.cargo}"

    local need_install=0
    if ! command -v rustup >/dev/null 2>&1; then
        need_install=1
    fi

    if [ "$need_install" -eq 1 ]; then
        case "$ARCH" in
            x86_64)  local rust_arch="x86_64-unknown-linux-gnu" ;;
            aarch64) local rust_arch="aarch64-unknown-linux-gnu" ;;
            *) echo "session-start: unsupported arch for rust: $ARCH" >&2; return 1 ;;
        esac

        local tmpdir
        tmpdir="$(mktemp -d)"
        trap 'rm -rf "$tmpdir"' RETURN

        echo "session-start: fetching rustup-init (${rust_arch})"
        if ! fetch_with_retry \
                "https://static.rust-lang.org/rustup/dist/${rust_arch}/rustup-init" \
                "${tmpdir}/rustup-init"; then
            echo "session-start: failed to download rustup-init" >&2
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
    fi

    # Make sure the requested toolchain is present even on warm containers.
    # static.rust-lang.org can 503 briefly, so retry with exponential backoff.
    local attempt
    for attempt in 1 2 3 4; do
        if "${CARGO_HOME}/bin/rustup" toolchain install "${RUST_VERSION}" \
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
    "${CARGO_HOME}/bin/rustup" default "${RUST_VERSION}"

    # Expose cargo/rustc without relying on PATH hacks in downstream shells.
    ln -sfn "${CARGO_HOME}/bin/cargo"   /usr/local/bin/cargo
    ln -sfn "${CARGO_HOME}/bin/rustc"   /usr/local/bin/rustc
    ln -sfn "${CARGO_HOME}/bin/rustup"  /usr/local/bin/rustup
    ln -sfn "${CARGO_HOME}/bin/rustfmt" /usr/local/bin/rustfmt
    ln -sfn "${CARGO_HOME}/bin/clippy-driver" /usr/local/bin/clippy-driver

    echo "session-start: rust $(${CARGO_HOME}/bin/rustc --version) ready"
}

install_zig
install_wasmtime
install_rust
install_tcl_sources
install_tcllib

echo "session-start: done"
