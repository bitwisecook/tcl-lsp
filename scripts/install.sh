#!/usr/bin/env sh
# tcl-lsp CLI installer for the `tcl` and `f5` zipapps.
#
# Detects the host OS (macOS, Debian/Ubuntu, RHEL/CentOS/Fedora, Arch),
# checks for a usable Python 3.10+, installs dependencies through the
# native package manager when missing, downloads the latest release
# zipapps into a user-local bin directory, and optionally writes shell
# completion for bash/zsh/fish.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/bitwisecook/tcl-lsp/main/scripts/install.sh | sh
#
# Environment overrides:
#   TCL_LSP_VERSION    - release tag to install (default: latest)
#   TCL_LSP_PREFIX     - install dir for binaries (default: $HOME/.local/bin)
#   TCL_LSP_REPO       - GitHub owner/repo (default: bitwisecook/tcl-lsp)
#   TCL_LSP_ONLY       - "tcl", "f5", or "both" (default: both)
#   TCL_LSP_NO_DEPS    - set to 1 to skip Python install
#   TCL_LSP_NO_PATH    - set to 1 to skip PATH/rc modification
#   TCL_LSP_NO_COMP    - set to 1 to skip shell completion
#   TCL_LSP_ASSUME_YES - set to 1 to answer "yes" non-interactively

set -eu

REPO="${TCL_LSP_REPO:-bitwisecook/tcl-lsp}"
VERSION="${TCL_LSP_VERSION:-latest}"
PREFIX="${TCL_LSP_PREFIX:-$HOME/.local/bin}"
ONLY="${TCL_LSP_ONLY:-both}"

GREEN=''; YELLOW=''; RED=''; BOLD=''; RESET=''
if [ -t 1 ] && command -v tput >/dev/null 2>&1; then
    GREEN="$(tput setaf 2 2>/dev/null || true)"
    YELLOW="$(tput setaf 3 2>/dev/null || true)"
    RED="$(tput setaf 1 2>/dev/null || true)"
    BOLD="$(tput bold 2>/dev/null || true)"
    RESET="$(tput sgr0 2>/dev/null || true)"
fi

log()  { printf '%s==>%s %s\n'  "$GREEN" "$RESET" "$*"; }
warn() { printf '%swarn:%s %s\n' "$YELLOW" "$RESET" "$*" >&2; }
die()  { printf '%serror:%s %s\n' "$RED"   "$RESET" "$*" >&2; exit 1; }

ask() {
    # ask "Prompt? [Y/n] "; returns 0 for yes, 1 for no.
    if [ "${TCL_LSP_ASSUME_YES:-0}" = "1" ] || [ ! -t 0 ]; then
        return 0
    fi
    printf '%s ' "$1"
    read -r reply || return 1
    case "$reply" in
        ''|y|Y|yes|YES|Yes) return 0 ;;
        *) return 1 ;;
    esac
}

detect_os() {
    UNAME="$(uname -s 2>/dev/null || echo unknown)"
    case "$UNAME" in
        Darwin) OS=macos ;;
        Linux)
            if [ -r /etc/os-release ]; then
                # shellcheck disable=SC1091
                . /etc/os-release
                case "${ID:-}:${ID_LIKE:-}" in
                    debian:*|ubuntu:*|*:*debian*|*:*ubuntu*) OS=debian ;;
                    fedora:*|*:*fedora*)                     OS=fedora ;;
                    rhel:*|centos:*|rocky:*|almalinux:*|*:*rhel*|*:*centos*) OS=rhel ;;
                    arch:*|manjaro:*|*:*arch*)               OS=arch ;;
                    alpine:*|*:*alpine*)                     OS=alpine ;;
                    *) OS=linux ;;
                esac
            else
                OS=linux
            fi
            ;;
        *) die "unsupported OS: $UNAME (only macOS and Linux are supported)" ;;
    esac
    log "detected OS: $OS"
}

detect_shell() {
    # $SHELL is the login shell; not necessarily the shell running this
    # script (we are running under /bin/sh).  Trust it for rc-file paths.
    SHELL_NAME=""
    case "${SHELL:-}" in
        */zsh)  SHELL_NAME=zsh ;;
        */bash) SHELL_NAME=bash ;;
        */fish) SHELL_NAME=fish ;;
        */ksh)  SHELL_NAME=ksh ;;
        */dash) SHELL_NAME=dash ;;
        *) SHELL_NAME="$(basename "${SHELL:-sh}")" ;;
    esac

    case "$SHELL_NAME" in
        zsh)  RC="${ZDOTDIR:-$HOME}/.zshrc" ;;
        bash)
            if [ "$OS" = macos ]; then
                RC="$HOME/.bash_profile"
            else
                RC="$HOME/.bashrc"
            fi
            ;;
        fish) RC="${XDG_CONFIG_HOME:-$HOME/.config}/fish/config.fish" ;;
        *)    RC="$HOME/.profile" ;;
    esac
    log "detected shell: $SHELL_NAME (rc: $RC)"
}

have() { command -v "$1" >/dev/null 2>&1; }

python_ok() {
    # Returns 0 if $1 is Python >= 3.10
    [ -n "${1:-}" ] || return 1
    have "$1" || return 1
    "$1" - <<'PY' 2>/dev/null
import sys
sys.exit(0 if sys.version_info >= (3, 10) else 1)
PY
}

find_python() {
    PYTHON=""
    for candidate in \
        python3.14 python3.13 python3.12 python3.11 python3.10 \
        python3 python; do
        if python_ok "$candidate"; then
            PYTHON="$(command -v "$candidate")"
            return 0
        fi
    done
    return 1
}

install_python() {
    if [ "${TCL_LSP_NO_DEPS:-0}" = "1" ]; then
        die "Python 3.10+ not found and TCL_LSP_NO_DEPS=1 — install Python manually and retry."
    fi
    log "Python 3.10+ not found — attempting install via package manager"
    case "$OS" in
        macos)
            if ! have brew; then
                die "Homebrew not found. Install from https://brew.sh and re-run."
            fi
            brew install python@3.14 || brew install python@3.13 || brew install python3
            ;;
        debian)
            run_root apt-get update
            run_root apt-get install -y python3 ca-certificates curl
            ;;
        rhel|fedora)
            if have dnf; then
                run_root dnf install -y python3 ca-certificates curl
            else
                run_root yum install -y python3 ca-certificates curl
            fi
            ;;
        arch)
            run_root pacman -Sy --noconfirm python ca-certificates curl
            ;;
        alpine)
            run_root apk add --no-cache python3 ca-certificates curl
            ;;
        *)
            die "Cannot auto-install Python on this OS. Install Python 3.10+ manually and retry."
            ;;
    esac

    find_python || die "Python install completed but no python3.10+ found on PATH."
}

run_root() {
    if [ "$(id -u)" = 0 ]; then
        "$@"
    elif have sudo; then
        sudo "$@"
    elif have doas; then
        doas "$@"
    else
        die "Need root for: $*  (install sudo, or run this script as root)"
    fi
}

ensure_curl() {
    if have curl; then DOWNLOADER="curl"; return; fi
    if have wget; then DOWNLOADER="wget"; return; fi
    case "$OS" in
        macos)  warn "curl missing on macOS — install Xcode Command Line Tools" ;;
        debian) run_root apt-get install -y curl ;;
        rhel|fedora)
            if have dnf; then run_root dnf install -y curl; else run_root yum install -y curl; fi ;;
        arch)   run_root pacman -Sy --noconfirm curl ;;
        alpine) run_root apk add --no-cache curl ;;
        *)      die "Need curl or wget to download release artefacts." ;;
    esac
    DOWNLOADER="curl"
}

download() {
    # download URL OUTPUT
    url="$1"; out="$2"
    log "fetching $(basename "$out")"
    if [ "$DOWNLOADER" = "wget" ]; then
        wget -qO "$out" "$url"
    else
        curl -fsSL -o "$out" "$url"
    fi
}

release_url() {
    # release_url ASSET_NAME
    asset="$1"
    if [ "$VERSION" = "latest" ]; then
        echo "https://github.com/$REPO/releases/latest/download/$asset"
    else
        echo "https://github.com/$REPO/releases/download/$VERSION/$asset"
    fi
}

install_cli() {
    # install_cli NAME (one of "tcl" or "f5")
    name="$1"
    asset_glob="${name}-*.pyz"

    # Releases ship versioned filenames (e.g. tcl-1.2.3.pyz), but the
    # GitHub "latest" redirect requires the exact asset name. We resolve
    # via the latest release JSON when VERSION=latest, otherwise we
    # require the user to supply a tag.
    if [ "$VERSION" = "latest" ]; then
        api_url="https://api.github.com/repos/$REPO/releases/latest"
        if [ "$DOWNLOADER" = "wget" ]; then
            json="$(wget -qO- "$api_url" || true)"
        else
            json="$(curl -fsSL "$api_url" || true)"
        fi
        if [ -z "$json" ]; then
            die "could not query GitHub releases API. Set TCL_LSP_VERSION=vX.Y.Z to bypass."
        fi
        asset="$(printf '%s' "$json" \
            | grep -o "\"name\": *\"${name}-[^\"]*\\.pyz\"" \
            | head -n1 \
            | sed -E 's/.*"name": *"([^"]+)".*/\1/')"
        if [ -z "$asset" ]; then
            die "no '${name}-*.pyz' asset found in latest release."
        fi
        url="$(release_url "$asset")"
    else
        # Caller pinned a version; assume canonical naming.
        ver_no_v="${VERSION#v}"
        asset="${name}-${ver_no_v}.pyz"
        url="$(release_url "$asset")"
    fi

    mkdir -p "$PREFIX"
    tmpfile="$(mktemp "${TMPDIR:-/tmp}/${name}.XXXXXX.pyz")"
    download "$url" "$tmpfile"
    mv "$tmpfile" "$PREFIX/$name"
    chmod +x "$PREFIX/$name"
    log "installed $name -> $PREFIX/$name"
}

path_contains() {
    case ":${PATH}:" in
        *":$1:"*) return 0 ;;
        *) return 1 ;;
    esac
}

ensure_path() {
    if path_contains "$PREFIX"; then return; fi
    if [ "${TCL_LSP_NO_PATH:-0}" = "1" ]; then
        warn "$PREFIX is not on PATH — TCL_LSP_NO_PATH=1, skipping rc update."
        return
    fi
    if ! ask "Add $PREFIX to PATH in $RC? [Y/n]"; then
        warn "Skipped PATH update. Add this to $RC manually:"
        warn "  export PATH=\"$PREFIX:\$PATH\""
        return
    fi
    mkdir -p "$(dirname "$RC")"
    case "$SHELL_NAME" in
        fish)
            printf '\n# Added by tcl-lsp installer\nfish_add_path %s\n' "$PREFIX" >> "$RC"
            ;;
        *)
            printf '\n# Added by tcl-lsp installer\nexport PATH="%s:$PATH"\n' "$PREFIX" >> "$RC"
            ;;
    esac
    log "appended PATH entry to $RC"
}

install_completion() {
    # install_completion CLI_NAME
    name="$1"
    if [ "${TCL_LSP_NO_COMP:-0}" = "1" ]; then return; fi
    if ! ask "Install $name shell completion for $SHELL_NAME? [Y/n]"; then return; fi

    bin="$PREFIX/$name"
    case "$SHELL_NAME" in
        bash)
            dir="$HOME/.local/share/bash-completion/completions"
            mkdir -p "$dir"
            "$bin" completion bash > "$dir/$name" || warn "$name completion failed"
            log "bash completion -> $dir/$name"
            ;;
        zsh)
            dir="${ZDOTDIR:-$HOME}/.zsh/completions"
            mkdir -p "$dir"
            "$bin" completion zsh > "$dir/_$name" || warn "$name completion failed"
            log "zsh completion -> $dir/_$name"
            log "ensure your .zshrc has: fpath=($dir \$fpath) && autoload -Uz compinit && compinit"
            ;;
        fish)
            dir="${XDG_CONFIG_HOME:-$HOME/.config}/fish/completions"
            mkdir -p "$dir"
            "$bin" completion fish > "$dir/$name.fish" || warn "$name completion failed"
            log "fish completion -> $dir/$name.fish"
            ;;
        *)
            warn "no completion template for $SHELL_NAME — skipping"
            ;;
    esac
}

main() {
    detect_os
    detect_shell
    ensure_curl

    if ! find_python; then
        install_python
    fi
    log "using Python: $PYTHON"

    case "$ONLY" in
        tcl)  install_cli tcl ;;
        f5)   install_cli f5 ;;
        both) install_cli tcl; install_cli f5 ;;
        *) die "invalid TCL_LSP_ONLY: $ONLY (expected tcl|f5|both)" ;;
    esac

    ensure_path

    case "$ONLY" in
        tcl)  install_completion tcl ;;
        f5)   install_completion f5 ;;
        both) install_completion tcl; install_completion f5 ;;
    esac

    printf '\n%sInstall complete.%s\n' "$BOLD" "$RESET"
    if ! path_contains "$PREFIX"; then
        printf 'Open a new shell, or run:  %sexport PATH="%s:$PATH"%s\n' \
               "$BOLD" "$PREFIX" "$RESET"
    fi
    case "$ONLY" in
        tcl)  printf 'Verify:  %stcl --help%s\n' "$BOLD" "$RESET" ;;
        f5)   printf 'Verify:  %sf5 --help%s\n'  "$BOLD" "$RESET" ;;
        both) printf 'Verify:  %stcl --help && f5 --help%s\n' "$BOLD" "$RESET" ;;
    esac
}

main "$@"
