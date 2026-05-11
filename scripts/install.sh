#!/usr/bin/env sh
# tcl-lsp installer for the released `tcl` / `f5` CLIs, the MCP server,
# and the Claude Code skills bundle.
#
# Detects the host OS (macOS, Debian/Ubuntu, RHEL/CentOS/Fedora, Arch),
# checks for a usable Python 3.10+, installs dependencies through the
# native package manager when missing, prompts for an install
# directory (scanning $PATH for writable candidates), and optionally
# writes shell completion. If the `claude` or `codex` CLI is detected
# (or ~/.claude / ~/.codex exists), offers to install the MCP server
# and — for Claude Code — the skills zip from the same GitHub release.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/bitwisecook/tcl-lsp/main/scripts/install.sh | sh
#
# Environment overrides:
#   TCL_LSP_VERSION     - release tag to install (default: latest)
#   TCL_LSP_PREFIX      - install dir (default: prompt; non-interactive: ~/.local/bin)
#   TCL_LSP_REPO        - GitHub owner/repo (default: bitwisecook/tcl-lsp)
#   TCL_LSP_ONLY        - "tcl", "f5", or "both" (default: both)
#   TCL_LSP_NO_DEPS     - set to 1 to skip Python install
#   TCL_LSP_NO_PATH     - set to 1 to skip PATH/rc modification
#   TCL_LSP_NO_COMP     - set to 1 to skip shell completion
#   TCL_LSP_NO_MCP      - set to 1 to skip MCP-server install for AI clients
#   TCL_LSP_NO_SKILLS   - set to 1 to skip Claude Code skills install
#   TCL_LSP_NO_CLAUDE   - set to 1 to ignore Claude Code even if detected
#   TCL_LSP_NO_CODEX    - set to 1 to ignore Codex even if detected
#   TCL_LSP_ASSUME_YES  - set to 1 to answer "yes" non-interactively
#   TCL_LSP_ASSUME_NO   - set to 1 to answer "no" non-interactively

set -eu

REPO="${TCL_LSP_REPO:-bitwisecook/tcl-lsp}"
VERSION="${TCL_LSP_VERSION:-latest}"
ONLY="${TCL_LSP_ONLY:-both}"

# Tri-state: if TCL_LSP_PREFIX is set in env we honour it without
# prompting; otherwise choose_prefix() may run interactively.
if [ -n "${TCL_LSP_PREFIX:-}" ]; then
    PREFIX="$TCL_LSP_PREFIX"
    PREFIX_EXPLICIT=1
else
    PREFIX="$HOME/.local/bin"
    PREFIX_EXPLICIT=0
fi

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
    # Non-interactive (piped stdin, e.g. `curl … | sh`): default to NO
    # so we don't silently mutate rc files. Pass TCL_LSP_ASSUME_YES=1
    # for full automation, or TCL_LSP_ASSUME_NO=1 to suppress prompts.
    if [ "${TCL_LSP_ASSUME_YES:-0}" = "1" ]; then
        return 0
    fi
    if [ "${TCL_LSP_ASSUME_NO:-0}" = "1" ] || [ ! -t 0 ]; then
        return 1
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
        rhel)
            # RHEL 9 / Rocky 9 / Alma 9 ship Python 3.9 as `python3`.
            # Prefer a versioned 3.10+ interpreter (AppStream module).
            PM="yum"; have dnf && PM="dnf"
            run_root "$PM" install -y ca-certificates curl
            if   run_root "$PM" install -y python3.12 2>/dev/null; then :
            elif run_root "$PM" install -y python3.11 2>/dev/null; then :
            else run_root "$PM" install -y python3
            fi
            ;;
        fedora)
            PM="yum"; have dnf && PM="dnf"
            run_root "$PM" install -y python3 ca-certificates curl
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

resolve_latest_tag() {
    # Follow the redirect from /releases/latest (HTML) to /releases/tag/vX.Y.Z.
    # Avoids the api.github.com 60/hr unauthenticated rate limit.
    redirect_url="https://github.com/$REPO/releases/latest"
    final_url=""
    if [ "$DOWNLOADER" = "wget" ]; then
        # wget prints the resolved Location on the "Location:" lines of -S.
        final_url="$(wget -qS --max-redirect=10 -O /dev/null "$redirect_url" 2>&1 \
            | awk '/^  Location:/ {loc=$2} END {print loc}' | tr -d '\r')"
    else
        final_url="$(curl -fsSLI -o /dev/null -w '%{url_effective}' "$redirect_url")"
    fi
    case "$final_url" in
        *"/releases/tag/"*) printf '%s\n' "${final_url##*/tag/}" ;;
        *) return 1 ;;
    esac
}

RESOLVED_TAG=""
ensure_tag() {
    # Lazy + memoised. Sets RESOLVED_TAG and VER_NO_V.
    if [ -n "$RESOLVED_TAG" ]; then return; fi
    if [ "$VERSION" = "latest" ]; then
        RESOLVED_TAG="$(resolve_latest_tag)" \
            || die "could not resolve latest release tag from https://github.com/$REPO/releases/latest. Set TCL_LSP_VERSION=vX.Y.Z to bypass."
    else
        RESOLVED_TAG="$VERSION"
    fi
    VER_NO_V="${RESOLVED_TAG#v}"
}

asset_url() {
    # asset_url ASSET_NAME
    ensure_tag
    printf 'https://github.com/%s/releases/download/%s/%s\n' \
        "$REPO" "$RESOLVED_TAG" "$1"
}

install_cli() {
    # install_cli NAME (one of "tcl" or "f5")
    name="$1"
    ensure_tag
    asset="${name}-${VER_NO_V}.pyz"
    url="$(asset_url "$asset")"
    log "resolved $name -> $asset (tag $RESOLVED_TAG)"

    # BSD/macOS mktemp requires the X-template at the end of the name.
    tmpfile="$(mktemp "${TMPDIR:-/tmp}/${name}-install.XXXXXX")"
    download "$url" "$tmpfile"
    write_target "$tmpfile" "$PREFIX/$name"
    log "installed $name -> $PREFIX/$name"
}

path_contains() {
    case ":${PATH}:" in
        *":$1:"*) return 0 ;;
        *) return 1 ;;
    esac
}

dir_writable() {
    # Returns 0 if $1 is a writable directory, or if the deepest
    # existing ancestor of $1 is writable (so we can mkdir into it).
    d="$1"
    while [ -n "$d" ] && [ "$d" != "/" ]; do
        if [ -d "$d" ]; then
            [ -w "$d" ]
            return $?
        fi
        d="$(dirname "$d")"
    done
    return 1
}

annotate_candidate() {
    # Print a parenthesised tag describing PATH-membership and writability.
    d="$1"
    on_path="not on PATH"
    if path_contains "$d"; then on_path="on PATH"; fi
    if dir_writable "$d"; then
        if [ -d "$d" ]; then perms="writable"; else perms="will create"; fi
    else
        perms="needs sudo"
    fi
    printf '(%s, %s)' "$on_path" "$perms"
}

collect_install_candidates() {
    # Emit one candidate path per line. Order: user-owned dirs already
    # on PATH, then promoted defaults, then well-known system dirs.
    seen=""
    emit() {
        case "$seen" in *":$1:"*) return ;; esac
        seen="$seen:$1:"
        printf '%s\n' "$1"
    }
    OLD_IFS="$IFS"; IFS=:
    # shellcheck disable=SC2086
    set -- $PATH
    IFS="$OLD_IFS"
    for d in "$@"; do
        case "$d" in
            "$HOME"/*) emit "$d" ;;
        esac
    done
    emit "$HOME/.local/bin"
    emit "$HOME/bin"
    for d in /usr/local/bin /opt/homebrew/bin /opt/local/bin; do
        [ -d "$d" ] && emit "$d"
    done
}

choose_prefix() {
    # Skip if the user pinned a location, or if we can't interact.
    if [ "$PREFIX_EXPLICIT" = "1" ]; then
        log "install location (TCL_LSP_PREFIX): $PREFIX"
        return
    fi
    if [ ! -t 0 ] || [ ! -t 1 ]; then
        log "install location (default): $PREFIX"
        return
    fi
    if [ "${TCL_LSP_ASSUME_YES:-0}" = "1" ] || [ "${TCL_LSP_ASSUME_NO:-0}" = "1" ]; then
        log "install location (default): $PREFIX"
        return
    fi

    printf '\n%sChoose install location:%s\n' "$BOLD" "$RESET"
    i=0
    cands="$(collect_install_candidates)"
    OLD_IFS="$IFS"; IFS='
'
    for c in $cands; do
        i=$((i + 1))
        annot="$(annotate_candidate "$c")"
        marker=" "
        [ "$c" = "$PREFIX" ] && marker="*"
        printf '  %s %d) %-32s %s\n' "$marker" "$i" "$c" "$annot"
    done
    IFS="$OLD_IFS"
    other_idx=$((i + 1))
    printf '    %d) Other (enter a path)\n' "$other_idx"
    printf '\nSelection [%s]: ' "$PREFIX"
    read -r ans || ans=""

    if [ -z "$ans" ]; then
        log "install location: $PREFIX"
        return
    fi
    # Numeric pick?
    case "$ans" in
        ''|*[!0-9]*)
            # Treat as a literal path
            PREFIX="$ans"
            ;;
        *)
            if [ "$ans" = "$other_idx" ]; then
                printf 'Path: '
                read -r custom
                [ -n "$custom" ] && PREFIX="$custom"
            else
                i=0
                IFS='
'
                for c in $cands; do
                    i=$((i + 1))
                    if [ "$i" = "$ans" ]; then PREFIX="$c"; break; fi
                done
                IFS="$OLD_IFS"
            fi
            ;;
    esac
    log "install location: $PREFIX"
}

write_target() {
    # write_target SRC DST — move SRC to DST, escalating to sudo when
    # the destination directory is not user-writable.
    src="$1"; dst="$2"
    target_dir="$(dirname "$dst")"
    if dir_writable "$target_dir"; then
        mkdir -p "$target_dir"
        mv "$src" "$dst"
        chmod 0755 "$dst"
    else
        log "$target_dir not writable — installing with sudo"
        run_root mkdir -p "$target_dir"
        run_root mv "$src" "$dst"
        run_root chmod 0755 "$dst"
    fi
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
            # $PATH must be written verbatim — expanded by the user's
            # shell at startup, not by us right now.
            # shellcheck disable=SC2016
            printf '\n# Added by tcl-lsp installer\nexport PATH="%s:$PATH"\n' "$PREFIX" >> "$RC"
            ;;
    esac
    log "appended PATH entry to $RC"
}

install_completion() {
    # install_completion CLI_NAME
    name="$1"
    if [ "${TCL_LSP_NO_COMP:-0}" = "1" ]; then return; fi
    if ! ask "Install $name shell completion for $SHELL_NAME? [Y/n]"; then
        if [ ! -t 0 ] && [ "${TCL_LSP_ASSUME_YES:-0}" != "1" ]; then
            log "skipped $name completion (non-interactive). Install later with:"
            log "  $name completion $SHELL_NAME  # see INSTALL-cli.md for paths"
        fi
        return
    fi

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

have_claude_cli() { have claude; }
have_codex_cli()  { have codex; }
has_claude_dir()  { [ -d "$HOME/.claude" ]; }
has_codex_dir()   { [ -d "$HOME/.codex" ]; }

detect_ai_clients() {
    HAS_CLAUDE=0
    HAS_CODEX=0
    if [ "${TCL_LSP_NO_CLAUDE:-0}" != "1" ]; then
        if have_claude_cli || has_claude_dir; then HAS_CLAUDE=1; fi
    fi
    if [ "${TCL_LSP_NO_CODEX:-0}" != "1" ]; then
        if have_codex_cli || has_codex_dir; then HAS_CODEX=1; fi
    fi
    if [ "$HAS_CLAUDE" = "1" ] || [ "$HAS_CODEX" = "1" ]; then
        msg=""
        [ "$HAS_CLAUDE" = "1" ] && msg="${msg}Claude Code "
        [ "$HAS_CODEX"  = "1" ] && msg="${msg}Codex "
        log "detected AI client(s): ${msg}"
    fi
}

ensure_unzip() {
    if have unzip; then return 0; fi
    case "$OS" in
        macos)  warn "unzip missing on macOS — install Xcode Command Line Tools"; return 1 ;;
        debian) run_root apt-get install -y unzip ;;
        rhel|fedora)
            if have dnf; then run_root dnf install -y unzip
            else run_root yum install -y unzip; fi ;;
        arch)   run_root pacman -Sy --noconfirm unzip ;;
        alpine) run_root apk add --no-cache unzip ;;
        *) warn "unzip not found and cannot auto-install"; return 1 ;;
    esac
    have unzip
}

MCP_PATH=""
install_mcp_zipapp() {
    # Download tcl-lsp-mcp-server-<ver>.pyz to $PREFIX. Idempotent —
    # safe to call once even when both Claude Code and Codex want it.
    if [ -n "$MCP_PATH" ] && [ -x "$MCP_PATH" ]; then return 0; fi
    ensure_tag
    asset="tcl-lsp-mcp-server-${VER_NO_V}.pyz"
    url="$(asset_url "$asset")"
    log "downloading $asset"
    tmpfile="$(mktemp "${TMPDIR:-/tmp}/mcp-install.XXXXXX")"
    download "$url" "$tmpfile"
    MCP_PATH="$PREFIX/tcl-lsp-mcp-server.pyz"
    write_target "$tmpfile" "$MCP_PATH"
    log "installed MCP server -> $MCP_PATH"
}

register_mcp_claude() {
    # Use `claude mcp add` when the CLI is on PATH (idempotent: remove
    # any prior registration first, then add fresh). Otherwise instruct
    # the user — we don't auto-mutate ~/.claude.json (its schema can
    # change between Claude Code versions).
    if ! have_claude_cli; then
        warn "claude CLI not on PATH — add the MCP server manually:"
        warn "  claude mcp add tcl-lsp -- $PYTHON $MCP_PATH"
        return
    fi
    # `claude mcp remove` exits non-zero when the server is absent; ignore.
    claude mcp remove tcl-lsp >/dev/null 2>&1 || true
    if claude mcp add tcl-lsp -- "$PYTHON" "$MCP_PATH" >/dev/null 2>&1; then
        log "registered MCP server with Claude Code (tcl-lsp)"
    else
        warn "claude mcp add failed — register manually:"
        warn "  claude mcp add tcl-lsp -- $PYTHON $MCP_PATH"
    fi
}

register_mcp_codex() {
    # Codex CLI reads ~/.codex/config.toml; append an [mcp_servers.tcl_lsp]
    # block, with a one-time backup of the existing file. Skip if a
    # block of the same name already exists.
    cfg="$HOME/.codex/config.toml"
    mkdir -p "$HOME/.codex"
    touch "$cfg"
    if grep -q '^\[mcp_servers\.tcl_lsp\]' "$cfg" 2>/dev/null; then
        log "Codex already has [mcp_servers.tcl_lsp] in $cfg — leaving as-is"
        return
    fi
    cp "$cfg" "${cfg}.bak.$(date +%Y%m%d%H%M%S)" 2>/dev/null || true
    {
        printf '\n[mcp_servers.tcl_lsp]\n'
        printf 'command = "%s"\n' "$PYTHON"
        printf 'args = ["%s"]\n'  "$MCP_PATH"
    } >> "$cfg"
    log "registered MCP server with Codex in $cfg"
}

install_claude_skills() {
    # Download tcl-lsp-claude-skills-<ver>.zip and extract into ~/.claude/.
    # The zip ships skills/, prompts/, and tcl-ai.pyz — the SKILL.md files
    # already reference ~/.claude/tcl-ai.pyz and ~/.claude/prompts/.
    ensure_unzip || { warn "unzip unavailable — skipping skills install"; return 1; }
    ensure_tag
    asset="tcl-lsp-claude-skills-${VER_NO_V}.zip"
    url="$(asset_url "$asset")"
    log "downloading $asset"
    tmpzip="$(mktemp "${TMPDIR:-/tmp}/claude-skills-install.XXXXXX")"
    download "$url" "$tmpzip"
    tmpdir="$(mktemp -d "${TMPDIR:-/tmp}/claude-skills-extract.XXXXXX")"
    unzip -q -o "$tmpzip" -d "$tmpdir"
    rm -f "$tmpzip"
    # The archive's top-level directory is tcl-lsp-claude-skills-<ver>/.
    inner="$(find "$tmpdir" -mindepth 1 -maxdepth 1 -type d | head -n1)"
    if [ -z "$inner" ]; then
        warn "could not locate extracted skill payload in $tmpdir"
        rm -rf "$tmpdir"
        return 1
    fi
    mkdir -p "$HOME/.claude"
    # Merge skills/, prompts/, and the tcl-ai.pyz; existing entries are
    # overwritten (this is an update path) but unrelated entries in
    # ~/.claude are untouched.
    [ -d "$inner/skills" ]  && cp -R "$inner/skills"  "$HOME/.claude/"
    [ -d "$inner/prompts" ] && cp -R "$inner/prompts" "$HOME/.claude/"
    [ -f "$inner/tcl-ai.pyz" ] && cp "$inner/tcl-ai.pyz" "$HOME/.claude/tcl-ai.pyz"
    chmod 0755 "$HOME/.claude/tcl-ai.pyz" 2>/dev/null || true
    rm -rf "$tmpdir"
    n="$(find "$HOME/.claude/skills" -mindepth 1 -maxdepth 1 -type d 2>/dev/null | wc -l | tr -d ' ')"
    log "installed Claude Code skills -> $HOME/.claude/skills/ ($n skills)"
}

install_ai_integrations() {
    detect_ai_clients
    if [ "$HAS_CLAUDE" != "1" ] && [ "$HAS_CODEX" != "1" ]; then return; fi
    if [ "${TCL_LSP_NO_MCP:-0}" = "1" ] && [ "${TCL_LSP_NO_SKILLS:-0}" = "1" ]; then
        return
    fi

    if [ "${TCL_LSP_NO_MCP:-0}" != "1" ] && ask "Install the tcl-lsp MCP server for detected AI client(s)? [Y/n]"; then
        install_mcp_zipapp
        [ "$HAS_CLAUDE" = "1" ] && register_mcp_claude
        [ "$HAS_CODEX"  = "1" ] && register_mcp_codex
    fi

    if [ "$HAS_CLAUDE" = "1" ] && [ "${TCL_LSP_NO_SKILLS:-0}" != "1" ] \
       && ask "Install Claude Code skills (irule-*, tcl-*, tk-*) into ~/.claude/? [Y/n]"; then
        install_claude_skills
    fi
}

main() {
    detect_os
    detect_shell
    ensure_curl

    if ! find_python; then
        install_python
    fi
    log "using Python: $PYTHON"

    choose_prefix

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

    install_ai_integrations

    printf '\n%sInstall complete.%s\n' "$BOLD" "$RESET"
    if ! path_contains "$PREFIX"; then
        # $PATH here is the instruction string shown to the user — they
        # paste it into their shell, where it expands.
        # shellcheck disable=SC2016
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
