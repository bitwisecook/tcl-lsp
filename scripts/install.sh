#!/usr/bin/env sh
# tcl-lsp installer for the released `tcl` / `f5` CLIs, the MCP server,
# and the Claude Code skills bundle.
#
# Detects the host OS (macOS, Debian/Ubuntu, RHEL/CentOS/Fedora, Arch,
# Alpine), checks for a usable Python 3.10+, installs dependencies
# through the native package manager when missing (after confirming),
# prompts for an install directory (scanning $PATH for writable
# candidates), and optionally writes shell completion.
#
# Before prompting, looks for an existing `tcl` / `f5` on $PATH and
# offers to update in place. When the chosen install directory is not
# user-writable, asks explicitly (default NO) before escalating to sudo.
#
# If the `claude` or `codex` CLI is detected (or ~/.claude / ~/.codex
# exists), offers to install the MCP server and — for Claude Code —
# the skills zip from the same GitHub release.
#
# Downloaded release artefacts are verified against the release's
# SHA256SUMS file (and, if `cosign` is installed and the release
# publishes one, the SHA256SUMS.cosign.bundle). Missing SUMS is a
# warning by default; set TCL_LSP_REQUIRE_VERIFY=1 to fail instead.
#
# Usage:
#   curl -fsSL https://github.com/bitwisecook/tcl-lsp/releases/latest/download/install.sh | sh
#
# Environment overrides:
#   TCL_LSP_VERSION         - release tag to install (default: latest)
#   TCL_LSP_PREFIX          - install dir (default: prompt; non-interactive: ~/.local/bin)
#   TCL_LSP_REPO            - GitHub owner/repo (default: bitwisecook/tcl-lsp)
#   TCL_LSP_ONLY            - "tcl", "f5", or "both" (default: both)
#   TCL_LSP_OS              - bypass /etc/os-release: debian|rhel|fedora|arch|alpine|macos
#   TCL_LSP_NO_DEPS         - 1 to skip OS-package install (Python, curl, unzip)
#   TCL_LSP_NO_PATH         - 1 to skip PATH/rc modification
#   TCL_LSP_NO_COMP         - 1 to skip shell completion
#   TCL_LSP_NO_MCP          - 1 to skip MCP-server install for AI clients
#   TCL_LSP_NO_SKILLS       - 1 to skip Claude Code skills install
#   TCL_LSP_NO_CLAUDE       - 1 to ignore Claude Code even if detected
#   TCL_LSP_NO_CODEX        - 1 to ignore Codex even if detected
#   TCL_LSP_NO_VERIFY       - 1 to skip SHA256SUMS verification
#   TCL_LSP_REQUIRE_VERIFY  - 1 to fail when SHA256SUMS is missing
#   TCL_LSP_ASSUME_YES      - 1 to answer "yes" non-interactively
#   TCL_LSP_ASSUME_NO       - 1 to answer "no" non-interactively
#   TCL_LSP_NO_TUI          - 1 to force text prompts even when whiptail/dialog is present

set -eu
# Defence-in-depth: a poisoned IFS in the caller's env can split values
# in places we don't expect. Reset to the POSIX default for the run.
IFS='
'

DEFAULT_REPO="bitwisecook/tcl-lsp"
REPO="${TCL_LSP_REPO:-$DEFAULT_REPO}"
VERSION="${TCL_LSP_VERSION:-latest}"
# ONLY is set from TCL_LSP_ONLY (env wins) or filled in interactively by
# choose_clis(). When choose_clis runs it sets ONLY_EXPLICIT=1 to mark the
# value as user-confirmed; otherwise main() expects the env default.
if [ -n "${TCL_LSP_ONLY:-}" ]; then
    ONLY="$TCL_LSP_ONLY"
    ONLY_EXPLICIT=1
else
    ONLY=both
    ONLY_EXPLICIT=0
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

# ---------------------------------------------------------------------
# TUI (whiptail / dialog) — used opportunistically when present
# ---------------------------------------------------------------------

UI=text
ensure_ui() {
    # Probe whiptail/dialog and only enable when we have a real TTY.
    # Set TCL_LSP_NO_TUI=1 to force the text path. Always returns 0 —
    # the absence of a TTY is a normal mode, not an error.
    [ "${TCL_LSP_NO_TUI:-0}" = "1" ] && return 0
    if [ ! -t 0 ] || [ ! -t 1 ]; then return 0; fi
    if command -v whiptail >/dev/null 2>&1; then UI=whiptail
    elif command -v dialog >/dev/null 2>&1; then UI=dialog
    fi
    return 0
}

TUI_TITLE='tcl-lsp installer'

tui_yesno() {
    # tui_yesno "prompt" — 0 = yes, 1 = no. Default highlight is "Yes".
    case "$UI" in
        whiptail|dialog)
            "$UI" --title "$TUI_TITLE" --yesno "$1" 10 70
            ;;
        *)
            printf '%s [Y/n] ' "$1"
            read -r reply || return 1
            case "$reply" in
                ''|y|Y|yes|YES|Yes) return 0 ;;
                *) return 1 ;;
            esac
            ;;
    esac
}

tui_menu() {
    # tui_menu "prompt" tag1 desc1 tag2 desc2 ...   (must come in pairs)
    # Prints the chosen tag to stdout; non-zero on cancel.
    prompt="$1"; shift
    case "$UI" in
        whiptail|dialog)
            # Count tag/desc pairs to size the menu list.
            count=$(($# / 2))
            "$UI" --title "$TUI_TITLE" --menu "$prompt" \
                  $((count + 8)) 70 "$count" "$@" 3>&1 1>&2 2>&3
            ;;
        *)
            printf '\n%s\n' "$prompt"
            i=0
            while [ $# -gt 0 ]; do
                i=$((i + 1))
                tag="$1"; desc="$2"; shift 2
                eval "tag_$i=\"\$tag\""
                printf '  %d) %-32s %s\n' "$i" "$tag" "$desc"
            done
            printf 'Selection [1]: '
            read -r ans || ans=1
            : "${ans:=1}"
            case "$ans" in
                ''|*[!0-9]*) return 1 ;;
            esac
            eval "echo \"\${tag_$ans:-}\""
            ;;
    esac
}

tui_checklist() {
    # tui_checklist "prompt" tag1 desc1 on/off  tag2 desc2 on/off  ...
    # Prints one tag per line for the boxes the user ticked.
    prompt="$1"; shift
    case "$UI" in
        whiptail|dialog)
            count=$(($# / 3))
            "$UI" --title "$TUI_TITLE" --separate-output \
                  --checklist "$prompt" \
                  $((count + 8)) 70 "$count" "$@" 3>&1 1>&2 2>&3
            ;;
        *)
            # Text fallback: echo every "on" tag, then offer the user a
            # comma-separated edit. Keeps the flow simple in plain shells.
            defaults=""
            args=""
            while [ $# -gt 0 ]; do
                tag="$1"; desc="$2"; state="$3"; shift 3
                args="${args}${tag}|${desc}\n"
                [ "$state" = ON ] && defaults="${defaults}${defaults:+,}${tag}"
            done
            printf '\n%s\n' "$prompt"
            printf '%b' "$args" | awk -F'|' '{ printf "  %-24s %s\n", $1, $2 }'
            printf 'Enable [%s]: ' "$defaults"
            read -r ans || ans=""
            : "${ans:=$defaults}"
            # Normalise "tcl, f5" → "tcl\nf5"
            printf '%s\n' "$ans" | tr ',' '\n' | sed 's/^ *//; s/ *$//; /^$/d'
            ;;
    esac
}

usage() {
    # Print the leading comment block (lines 2 onward, until the first
    # non-comment line — which is `set -eu`).
    awk 'NR > 1 && /^[^#]/ {exit} NR > 1 {sub(/^# ?/, ""); print}' "$0"
    exit 0
}

# Minimal CLI: --help / -h, --version / -V.
for arg in "$@"; do
    case "$arg" in
        -h|--help)    usage ;;
        -V|--version) printf 'tcl-lsp installer (in-tree script)\n'; exit 0 ;;
        --) break ;;
        -*) die "unknown flag: $arg (try --help)" ;;
    esac
done

# ---------------------------------------------------------------------
# REPO validation + non-default warning
# ---------------------------------------------------------------------

case "$REPO" in
    */*) : ;;
    *)   die "TCL_LSP_REPO must be in 'owner/repo' form: $REPO" ;;
esac
case "$REPO" in
    *[!A-Za-z0-9._/-]*) die "TCL_LSP_REPO contains invalid characters: $REPO" ;;
esac
if [ "$REPO" != "$DEFAULT_REPO" ]; then
    warn "Using non-default release source: github.com/$REPO"
    warn "(default is github.com/$DEFAULT_REPO)"
fi

# ---------------------------------------------------------------------
# Prompts
# ---------------------------------------------------------------------

ask() {
    # Opt-in prompt: 0 = yes, 1 = no. Default-no when piped so we don't
    # silently mutate rc files. Routes through tui_yesno when whiptail
    # or dialog is available.
    if [ "${TCL_LSP_ASSUME_YES:-0}" = "1" ]; then return 0; fi
    if [ "${TCL_LSP_ASSUME_NO:-0}"  = "1" ] || [ ! -t 0 ]; then return 1; fi
    tui_yesno "$1"
}

ask_optout() {
    # Like ask() but defaults yes when piped — for prompts where the
    # action is presumed-on because some upstream signal (detected AI
    # client, existing rc entry) was the actual opt-in.
    if [ "${TCL_LSP_ASSUME_NO:-0}" = "1" ]; then return 1; fi
    if [ "${TCL_LSP_ASSUME_YES:-0}" = "1" ] || [ ! -t 0 ]; then return 0; fi
    tui_yesno "$1"
}

ask_default_no() {
    # Strict default-NO prompt — empty input, piped stdin, and dialog
    # cancel all count as "no". Used for privilege escalation and any
    # other destructive choice the user must explicitly opt into.
    # TCL_LSP_ASSUME_YES overrides the piped-stdin default.
    if [ "${TCL_LSP_ASSUME_YES:-0}" = "1" ]; then return 0; fi
    if [ "${TCL_LSP_ASSUME_NO:-0}" = "1" ] || [ ! -t 0 ]; then return 1; fi
    case "$UI" in
        whiptail|dialog)
            "$UI" --title "$TUI_TITLE" --defaultno --yesno "$1" 10 70
            ;;
        *)
            printf '%s ' "$1"
            read -r reply || return 1
            case "$reply" in
                y|Y|yes|YES|Yes) return 0 ;;
                *) return 1 ;;
            esac
            ;;
    esac
}

confirm_root_action() {
    # confirm_root_action "human description of the privileged action"
    # Surface every sudo/doas escalation. Bypassed by TCL_LSP_ASSUME_YES.
    if [ "$(id -u)" = 0 ] || [ "${TCL_LSP_ASSUME_YES:-0}" = "1" ]; then
        return 0
    fi
    warn "Next step needs root: $1"
    ask "Proceed? [Y/n]" || die "aborted: root step declined"
}

# ---------------------------------------------------------------------
# $PREFIX validation — central choke point for any assignment
# ---------------------------------------------------------------------

set_prefix() {
    # Validate a candidate install location and assign $PREFIX.
    # Rejects anything that would let the value escape the rc-file
    # `export PATH="…:$PATH"` write, or that isn't an absolute path.
    p="$1"
    case "$p" in
        '')
            die "install location cannot be empty" ;;
        [!/]*)
            die "install location must be an absolute path: $p" ;;
        *[!A-Za-z0-9._/+~:-]*)
            die "install location contains unsupported characters: $p
Allowed: A-Z a-z 0-9 . _ / + ~ : - (no quotes, spaces, dollars, newlines)" ;;
    esac
    PREFIX="$p"
}

# Tri-state: explicit env var => use as-is; otherwise the picker may
# overwrite $PREFIX from a numbered choice or free-form path.
if [ -n "${TCL_LSP_PREFIX:-}" ]; then
    set_prefix "$TCL_LSP_PREFIX"
    PREFIX_EXPLICIT=1
else
    set_prefix "$HOME/.local/bin"
    PREFIX_EXPLICIT=0
fi

# ---------------------------------------------------------------------
# OS detection (with /etc/os-release safety check)
# ---------------------------------------------------------------------

read_os_release_safe() {
    # Refuse to source /etc/os-release unless it is owned by uid 0 and
    # not world-writable. Returns 0 if the file was sourced.
    f=/etc/os-release
    [ -e "$f" ] || return 1
    # `find -L` follows symlinks before checking metadata, so a
    # root-owned symlink targeting a world-writable file is rejected.
    # `! -perm -002` matches files without the world-write bit.
    safe="$(find -L "$f" -maxdepth 0 -uid 0 ! -perm -002 -print 2>/dev/null)"
    if [ -z "$safe" ]; then
        die "$f is not root-owned or is world-writable — refusing to source it.
Re-run with TCL_LSP_OS=<debian|rhel|fedora|arch|alpine|macos> to bypass detection."
    fi
    # shellcheck disable=SC1090,SC1091
    . "$f"
}

detect_os() {
    UNAME="$(uname -s 2>/dev/null || echo unknown)"
    if [ -n "${TCL_LSP_OS:-}" ]; then
        OS="$TCL_LSP_OS"
        log "OS override (TCL_LSP_OS): $OS"
        return
    fi
    case "$UNAME" in
        Darwin) OS=macos ;;
        Linux)
            if read_os_release_safe; then
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
    # script (we are running under /bin/sh). Trust it for rc-file paths.
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

# ---------------------------------------------------------------------
# Tool probing
# ---------------------------------------------------------------------

have() { command -v "$1" >/dev/null 2>&1; }

python_ok() {
    # Returns 0 if $1 resolves to a Python >= 3.10 interpreter.
    [ -n "${1:-}" ] || return 1
    have "$1" || return 1
    "$1" - <<'PY' 2>/dev/null
import sys
sys.exit(0 if sys.version_info >= (3, 10) else 1)
PY
}

find_python() {
    # Probe well-known absolute paths first so a poisoned $PATH cannot
    # silently shadow the system interpreter we're about to use.
    PYTHON=""
    for candidate in \
        /opt/homebrew/bin/python3.14 /opt/homebrew/bin/python3.13 /opt/homebrew/bin/python3.12 \
        /opt/homebrew/bin/python3.11 /opt/homebrew/bin/python3.10 /opt/homebrew/bin/python3 \
        /usr/local/bin/python3.14 /usr/local/bin/python3.13 /usr/local/bin/python3.12 \
        /usr/local/bin/python3.11 /usr/local/bin/python3.10 /usr/local/bin/python3 \
        /usr/bin/python3.14 /usr/bin/python3.13 /usr/bin/python3.12 \
        /usr/bin/python3.11 /usr/bin/python3.10 /usr/bin/python3 \
        python3.14 python3.13 python3.12 python3.11 python3.10 \
        python3 python; do
        if python_ok "$candidate"; then
            PYTHON="$(command -v "$candidate")"
            return 0
        fi
    done
    return 1
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
            confirm_root_action "apt-get update && apt-get install -y python3 ca-certificates curl"
            run_root apt-get update
            run_root apt-get install -y python3 ca-certificates curl
            ;;
        rhel)
            # RHEL 9 / Rocky 9 / Alma 9 ship Python 3.9 as `python3`;
            # try a versioned 3.10+ interpreter from AppStream first.
            PM="yum"; have dnf && PM="dnf"
            confirm_root_action "$PM install -y ca-certificates curl"
            run_root "$PM" install -y ca-certificates curl
            installed=0
            for v in python3.12 python3.11; do
                # Probe availability quietly, then install loudly.
                if "$PM" list --available "$v" >/dev/null 2>&1 \
                   || "$PM" info "$v" >/dev/null 2>&1; then
                    confirm_root_action "$PM install -y $v"
                    if run_root "$PM" install -y "$v"; then installed=1; break; fi
                fi
            done
            if [ "$installed" = 0 ]; then
                confirm_root_action "$PM install -y python3"
                run_root "$PM" install -y python3
            fi
            ;;
        fedora)
            PM="yum"; have dnf && PM="dnf"
            confirm_root_action "$PM install -y python3 ca-certificates curl"
            run_root "$PM" install -y python3 ca-certificates curl
            ;;
        arch)
            confirm_root_action "pacman -Sy --noconfirm python ca-certificates curl"
            run_root pacman -Sy --noconfirm python ca-certificates curl
            ;;
        alpine)
            confirm_root_action "apk add --no-cache python3 ca-certificates curl"
            run_root apk add --no-cache python3 ca-certificates curl
            ;;
        *)
            die "Cannot auto-install Python on this OS. Install Python 3.10+ manually and retry." ;;
    esac

    find_python || die "Python install completed but no python3.10+ found on PATH."
}

ensure_curl() {
    if have curl; then DOWNLOADER="curl"; return; fi
    if have wget; then DOWNLOADER="wget"; return; fi
    if [ "${TCL_LSP_NO_DEPS:-0}" = "1" ]; then
        die "Neither curl nor wget found and TCL_LSP_NO_DEPS=1 — install one and retry."
    fi
    case "$OS" in
        macos)  die "curl missing on macOS — install Xcode Command Line Tools and retry." ;;
        debian) confirm_root_action "apt-get install -y curl"; run_root apt-get install -y curl ;;
        rhel|fedora)
            if have dnf; then
                confirm_root_action "dnf install -y curl"; run_root dnf install -y curl
            else
                confirm_root_action "yum install -y curl"; run_root yum install -y curl
            fi ;;
        arch)   confirm_root_action "pacman -Sy --noconfirm curl"; run_root pacman -Sy --noconfirm curl ;;
        alpine) confirm_root_action "apk add --no-cache curl";     run_root apk add --no-cache curl ;;
        *)      die "Need curl or wget to download release artefacts." ;;
    esac
    have curl || die "curl install failed."
    DOWNLOADER="curl"
}

WGET_HAS_HTTPS_ONLY=""
wget_supports_https_only() {
    [ -n "$WGET_HAS_HTTPS_ONLY" ] && { [ "$WGET_HAS_HTTPS_ONLY" = "1" ]; return; }
    if wget --help 2>&1 | grep -q -- --https-only; then
        WGET_HAS_HTTPS_ONLY=1
    else
        WGET_HAS_HTTPS_ONLY=0
        warn "wget on this host does not support --https-only — HTTPS-only redirects cannot be enforced"
    fi
    [ "$WGET_HAS_HTTPS_ONLY" = "1" ]
}

# ---------------------------------------------------------------------
# Downloads (HTTPS-only)
# ---------------------------------------------------------------------

download() {
    # download URL OUTPUT — pins TLS, refuses HTTP redirects.
    url="$1"; out="$2"
    log "fetching $(basename "$out")"
    if [ "$DOWNLOADER" = "wget" ]; then
        if wget_supports_https_only; then
            wget --https-only -qO "$out" "$url"
        else
            wget -qO "$out" "$url"
        fi
    else
        curl --proto '=https' --proto-redir '=https' -fsSL -o "$out" "$url"
    fi
}

resolve_latest_tag() {
    # Follow the redirect from /releases/latest to /releases/tag/vX.Y.Z.
    # Avoids the api.github.com 60/hr unauthenticated rate limit.
    redirect_url="https://github.com/$REPO/releases/latest"
    final_url=""
    if [ "$DOWNLOADER" = "wget" ]; then
        if wget_supports_https_only; then
            final_url="$(wget --https-only -qS --max-redirect=10 -O /dev/null "$redirect_url" 2>&1 \
                | awk '/^  Location:/ {loc=$2} END {print loc}' | tr -d '\r')"
        else
            final_url="$(wget -qS --max-redirect=10 -O /dev/null "$redirect_url" 2>&1 \
                | awk '/^  Location:/ {loc=$2} END {print loc}' | tr -d '\r')"
        fi
    else
        final_url="$(curl --proto '=https' --proto-redir '=https' \
            -fsSLI -o /dev/null -w '%{url_effective}' "$redirect_url")"
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
    ensure_tag
    printf 'https://github.com/%s/releases/download/%s/%s\n' \
        "$REPO" "$RESOLVED_TAG" "$1"
}

# ---------------------------------------------------------------------
# Workdir + SHA256SUMS verification
# ---------------------------------------------------------------------

WORKDIR=""
init_workdir() {
    WORKDIR="$(mktemp -d "${TMPDIR:-/tmp}/tcl-lsp-install.XXXXXX")"
    trap 'rm -rf -- "$WORKDIR"' EXIT INT TERM HUP
}

SUMS_PATH=""
SUMS_STATE=""   # "present" | "absent" | ""
ensure_sums() {
    # Download SHA256SUMS once per run. Memoises in SUMS_STATE.
    # TCL_LSP_NO_VERIFY=1     → skip entirely (returns 1 silently).
    # TCL_LSP_REQUIRE_VERIFY=1 → die if missing.
    [ "${TCL_LSP_NO_VERIFY:-0}" = "1" ] && return 1
    case "$SUMS_STATE" in
        present) return 0 ;;
        absent)  return 1 ;;
    esac
    ensure_tag
    sums_tmp="$WORKDIR/SHA256SUMS"
    if download "$(asset_url SHA256SUMS)" "$sums_tmp" 2>/dev/null; then
        SUMS_PATH="$sums_tmp"
        SUMS_STATE=present
        verify_sums_signature "$SUMS_PATH" || die "cosign verification of SHA256SUMS failed"
        return 0
    fi
    if [ "${TCL_LSP_REQUIRE_VERIFY:-0}" = "1" ]; then
        die "SHA256SUMS not found at $(asset_url SHA256SUMS) — TCL_LSP_REQUIRE_VERIFY=1, refusing to proceed."
    fi
    warn "no SHA256SUMS file in release $RESOLVED_TAG — installing without integrity verification"
    warn "(set TCL_LSP_REQUIRE_VERIFY=1 to fail on missing SUMS, or TCL_LSP_NO_VERIFY=1 to silence this)"
    SUMS_STATE=absent
    return 1
}

verify_sums_signature() {
    # Verify the SUMS file against cosign keyless signature, if both
    # cosign and a SHA256SUMS.cosign.bundle are available.
    sums="$1"
    have cosign || return 0
    bundle="$WORKDIR/SHA256SUMS.cosign.bundle"
    if ! download "$(asset_url SHA256SUMS.cosign.bundle)" "$bundle" 2>/dev/null; then
        return 0   # bundle not published — nothing to verify
    fi
    if cosign verify-blob \
        --bundle "$bundle" \
        --certificate-identity-regexp "^https://github.com/$REPO/\\.github/workflows/.+@refs/tags/" \
        --certificate-oidc-issuer "https://token.actions.githubusercontent.com" \
        "$sums" >/dev/null 2>&1; then
        log "SHA256SUMS cosign signature verified"
        return 0
    fi
    return 1
}

verify_artefact() {
    # verify_artefact ASSET_NAME LOCAL_PATH
    # No-op when SUMS unavailable and not required (warning was already emitted).
    asset="$1"; local="$2"
    ensure_sums || return 0
    expected="$(awk -v a="$asset" '$2 == a || $2 == "*"a {print $1; exit}' "$SUMS_PATH")"
    [ -n "$expected" ] || die "no SHA256 entry for $asset in SHA256SUMS"
    if have sha256sum; then
        actual="$(sha256sum "$local" | awk '{print $1}')"
    elif have shasum; then
        actual="$(shasum -a 256 "$local" | awk '{print $1}')"
    else
        warn "neither sha256sum nor shasum available — skipping verify for $asset"
        return 0
    fi
    [ "$expected" = "$actual" ] \
        || die "checksum mismatch for $asset
expected: $expected
actual:   $actual"
    log "verified $asset (sha256)"
}

# ---------------------------------------------------------------------
# Install destination picker
# ---------------------------------------------------------------------

path_contains() {
    case ":${PATH}:" in
        *":$1:"*) return 0 ;;
        *) return 1 ;;
    esac
}

dir_writable() {
    # Returns 0 if $1 is a writable directory, or if the deepest
    # existing ancestor of $1 is writable. Decorative — the actual
    # write attempt is what enforces; the annotation can race.
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
    # Emit one candidate path per line. Walks $PATH without clobbering
    # the caller's $@. Skips hidden dirs from the PATH-scan (poisoned
    # ~/.evil/bin won't appear in the picker; ~/.local/bin is re-added
    # via the promoted defaults below).
    seen=""
    emit() {
        case "$seen" in *":$1:"*) return ;; esac
        seen="$seen:$1:"
        printf '%s\n' "$1"
    }
    OLD_IFS="$IFS"
    IFS=:
    # shellcheck disable=SC2086
    for d in $PATH; do
        IFS="$OLD_IFS"
        case "$d" in
            # Literal-prefix match — `${HOME}/` (with trailing slash)
            # cannot be glob-confused even if $HOME has metachars.
            "${HOME}/"*)
                case "$d" in
                    */.*) IFS=:; continue ;;
                esac
                emit "$d"
                ;;
        esac
        IFS=:
    done
    IFS="$OLD_IFS"
    emit "$HOME/.local/bin"
    emit "$HOME/bin"
    for d in /usr/local/bin /opt/homebrew/bin /opt/local/bin; do
        [ -d "$d" ] && emit "$d"
    done
}

choose_clis() {
    # Ask which CLIs to install (tcl, f5, both). TCL_LSP_ONLY in the env
    # bypasses the prompt entirely. ASSUME_YES / ASSUME_NO / piped stdin
    # all fall through to the env default ("both").
    if [ "$ONLY_EXPLICIT" = "1" ]; then
        log "CLIs (TCL_LSP_ONLY): $ONLY"
        return
    fi
    if [ ! -t 0 ] || [ ! -t 1 ]; then
        log "CLIs (default): $ONLY"
        return
    fi
    if [ "${TCL_LSP_ASSUME_YES:-0}" = "1" ] || [ "${TCL_LSP_ASSUME_NO:-0}" = "1" ]; then
        log "CLIs (default): $ONLY"
        return
    fi

    case "$UI" in
        whiptail|dialog)
            sel="$(tui_checklist "Which CLIs to install?" \
                tcl "Unified Tcl tools (format, lint, opt, ...)" ON \
                f5  "F5 BIG-IP tools (cleanup, irule, redact, ...)" ON \
                )" || die "aborted at CLI selection"
            # tui_checklist emits one tag per line.
            has_tcl=0; has_f5=0
            for t in $sel; do
                case "$t" in tcl) has_tcl=1 ;; f5) has_f5=1 ;; esac
            done
            if [ "$has_tcl" = 1 ] && [ "$has_f5" = 1 ]; then ONLY=both
            elif [ "$has_tcl" = 1 ]; then ONLY=tcl
            elif [ "$has_f5"  = 1 ]; then ONLY=f5
            else die "no CLI selected — at least one of tcl/f5 is required"
            fi
            ;;
        *)
            printf '\nWhich CLIs to install? [both/tcl/f5] (default: both): '
            read -r ans || ans=""
            : "${ans:=both}"
            case "$ans" in
                both|b) ONLY=both ;;
                tcl|t)  ONLY=tcl ;;
                f5|F5)  ONLY=f5 ;;
                *) die "invalid choice: $ans (expected both/tcl/f5)" ;;
            esac
            ;;
    esac
    log "CLIs to install: $ONLY"
}

# ---------------------------------------------------------------------
# Detect an existing install and offer to update in place
# ---------------------------------------------------------------------

find_on_path() {
    # Print the first executable named $1 found on $PATH.
    name="$1"
    OLD_IFS="$IFS"; IFS=:
    # shellcheck disable=SC2086
    for d in $PATH; do
        if [ -x "$d/$name" ] && [ -f "$d/$name" ]; then
            IFS="$OLD_IFS"
            printf '%s\n' "$d/$name"
            return 0
        fi
    done
    IFS="$OLD_IFS"
    return 1
}

looks_like_our_zipapp() {
    # Heuristic: file starts with #! python shebang and contains the
    # ZIP local-file-header signature `PK\x03\x04` within the first 2KB.
    # Cheap, no execution required.
    f="$1"
    [ -r "$f" ] || return 1
    first="$(head -c 200 "$f" 2>/dev/null)"
    case "$first" in
        '#!'*python*) : ;;
        *) return 1 ;;
    esac
    head -c 2048 "$f" 2>/dev/null | grep -q 'PK' || return 1
    return 0
}

propose_update_install() {
    # For each CLI the user selected, look for an existing copy on PATH.
    # If found AND it looks like one of our zipapps, ask whether to
    # update in place — point $PREFIX at its directory and mark explicit
    # so choose_prefix() skips.
    #
    # If multiple selected CLIs live in the same directory, one prompt
    # covers both. If they live in different dirs, prompt per-CLI and
    # take the first agreed-on dir (an unusual layout — the picker
    # always installs them side by side).
    [ "$PREFIX_EXPLICIT" = "1" ] && return

    case "$ONLY" in
        tcl)  targets="tcl" ;;
        f5)   targets="f5" ;;
        both) targets="tcl f5" ;;
        *)    return ;;
    esac

    found_dir=""
    for name in $targets; do
        if path="$(find_on_path "$name")" && looks_like_our_zipapp "$path"; then
            found_dir="$(dirname "$path")"
            log "found existing $name at $path"
            if ! ask_optout "Update existing $name at $path (in place)? [Y/n]"; then
                # User declined for this one — fall back to the normal
                # picker for everything. (We don't try to split prefix
                # per-CLI; it complicates rc-file / completion logic.)
                log "leaving $path alone; will pick a fresh install location"
                return
            fi
        elif [ -n "$path" ] && [ -x "$path" ]; then
            warn "found '$name' at $path but it doesn't look like our zipapp"
            warn "(missing python shebang or ZIP signature) — picker will run as usual"
            return
        fi
    done

    if [ -n "$found_dir" ]; then
        set_prefix "$found_dir"
        PREFIX_EXPLICIT=1
        PREFIX_FROM_UPDATE=1
        log "updating in place: $PREFIX"
    fi
}

choose_prefix() {
    if [ "$PREFIX_EXPLICIT" = "1" ]; then
        if [ "${PREFIX_FROM_UPDATE:-0}" != "1" ]; then
            log "install location (TCL_LSP_PREFIX): $PREFIX"
        fi
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

    cands="$(collect_install_candidates)"

    case "$UI" in
        whiptail|dialog)
            # Build the tag/desc argv: each candidate gets one entry,
            # plus a final "Other" entry that prompts for a free-form path.
            set --
            OLD_IFS="$IFS"; IFS='
'
            for c in $cands; do
                set -- "$@" "$c" "$(annotate_candidate "$c")"
            done
            IFS="$OLD_IFS"
            set -- "$@" "Other" "Enter a path manually"

            chosen="$(tui_menu "Choose install location" "$@")" \
                || { log "install location: $PREFIX"; return; }
            if [ "$chosen" = "Other" ]; then
                # whiptail/dialog inputbox: same redirect dance as menu.
                custom="$("$UI" --title "$TUI_TITLE" --inputbox \
                    "Path:" 8 70 "$PREFIX" 3>&1 1>&2 2>&3)" \
                    || { log "install location: $PREFIX"; return; }
                [ -n "$custom" ] && set_prefix "$custom"
            else
                set_prefix "$chosen"
            fi
            ;;
        *)
            printf '\n%sChoose install location:%s\n' "$BOLD" "$RESET"
            i=0
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
            case "$ans" in
                *[!0-9]*)
                    set_prefix "$ans"
                    ;;
                *)
                    if [ "$ans" = "$other_idx" ]; then
                        printf 'Path: '
                        read -r custom
                        if [ -n "$custom" ]; then set_prefix "$custom"; fi
                    else
                        i=0
                        IFS='
'
                        for c in $cands; do
                            i=$((i + 1))
                            if [ "$i" = "$ans" ]; then set_prefix "$c"; break; fi
                        done
                        IFS="$OLD_IFS"
                    fi
                    ;;
            esac
            ;;
    esac
    log "install location: $PREFIX"
}

# ---------------------------------------------------------------------
# Privileged file install
# ---------------------------------------------------------------------

write_target() {
    # write_target SRC DST — install SRC to DST atomically with mode 0755.
    # `install -m 0755` replaces mv+chmod with a single tool that does
    # the right thing on cross-filesystem copies.
    #
    # When the target directory is not user-writable, prompt for sudo
    # explicitly with default-NO. Declining aborts the install with a
    # clear next-step.
    src="$1"; dst="$2"
    target_dir="$(dirname "$dst")"
    if dir_writable "$target_dir"; then
        mkdir -p "$target_dir"
        install -m 0755 "$src" "$dst"
        return
    fi
    warn "Target directory not writable by $(id -un): $target_dir"
    if ! ask_default_no "Use sudo to install $(basename "$dst") into $target_dir? [y/N]"; then
        die "aborted: $target_dir not writable and sudo declined.
Re-run with TCL_LSP_PREFIX=/path/you/can/write/to, or accept the sudo prompt."
    fi
    run_root mkdir -p "$target_dir"
    run_root install -m 0755 "$src" "$dst"
}

# ---------------------------------------------------------------------
# CLI install
# ---------------------------------------------------------------------

install_cli() {
    # install_cli NAME (one of "tcl" or "f5")
    name="$1"
    ensure_tag
    asset="${name}-${VER_NO_V}.pyz"
    url="$(asset_url "$asset")"
    log "resolved $name -> $asset (tag $RESOLVED_TAG)"

    tmpfile="$WORKDIR/$asset"
    download "$url" "$tmpfile"
    verify_artefact "$asset" "$tmpfile"
    write_target "$tmpfile" "$PREFIX/$name"
    log "installed $name -> $PREFIX/$name"
}

# ---------------------------------------------------------------------
# rc-file PATH entry (idempotent)
# ---------------------------------------------------------------------

PATH_MARKER='# Added by tcl-lsp installer'

ensure_path() {
    if path_contains "$PREFIX"; then return; fi
    if [ "${TCL_LSP_NO_PATH:-0}" = "1" ]; then
        warn "$PREFIX is not on PATH — TCL_LSP_NO_PATH=1, skipping rc update."
        return
    fi
    if [ -f "$RC" ] && grep -qF "$PATH_MARKER" "$RC" 2>/dev/null; then
        log "PATH entry already present in $RC (skipping append)"
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
            printf '\n%s\nfish_add_path %s\n' "$PATH_MARKER" "$PREFIX" >> "$RC"
            ;;
        *)
            # $PATH must be written verbatim — the user's shell expands
            # it at login, not us. set_prefix() already rejected anything
            # that could escape the double quotes below.
            # shellcheck disable=SC2016
            printf '\n%s\nexport PATH="%s:$PATH"\n' "$PATH_MARKER" "$PREFIX" >> "$RC"
            ;;
    esac
    log "appended PATH entry to $RC"
}

# ---------------------------------------------------------------------
# Shell completion
# ---------------------------------------------------------------------

install_completion() {
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
            comp_install "$bin" "$dir/$name" bash
            log "bash completion -> $dir/$name"
            ;;
        zsh)
            dir="${ZDOTDIR:-$HOME}/.zsh/completions"
            comp_install "$bin" "$dir/_$name" zsh
            log "zsh completion -> $dir/_$name"
            log "ensure your .zshrc has: fpath=($dir \$fpath) && autoload -Uz compinit && compinit"
            ;;
        fish)
            dir="${XDG_CONFIG_HOME:-$HOME/.config}/fish/completions"
            comp_install "$bin" "$dir/$name.fish" fish
            log "fish completion -> $dir/$name.fish"
            ;;
        *)
            warn "no completion template for $SHELL_NAME — skipping"
            ;;
    esac
}

comp_install() {
    # comp_install BIN OUT_PATH SHELL — write completion script, prompting
    # before overwrite when run interactively.
    cbin="$1"; cout="$2"; cshell="$3"
    if [ -e "$cout" ] && [ -t 0 ] && [ "${TCL_LSP_ASSUME_YES:-0}" != "1" ]; then
        if ! ask "$cout already exists — overwrite? [Y/n]"; then
            log "kept existing $cout"
            return
        fi
    fi
    mkdir -p "$(dirname "$cout")"
    "$cbin" completion "$cshell" > "$cout" \
        || warn "$(basename "$cbin") completion failed"
}

# ---------------------------------------------------------------------
# AI client integration (Claude Code / Codex)
# ---------------------------------------------------------------------

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
    if [ "${TCL_LSP_NO_DEPS:-0}" = "1" ]; then
        warn "unzip missing and TCL_LSP_NO_DEPS=1 — skipping skills install"
        return 1
    fi
    case "$OS" in
        macos)  warn "unzip missing on macOS — install Xcode Command Line Tools"; return 1 ;;
        debian) confirm_root_action "apt-get install -y unzip"; run_root apt-get install -y unzip ;;
        rhel|fedora)
            if have dnf; then
                confirm_root_action "dnf install -y unzip"; run_root dnf install -y unzip
            else
                confirm_root_action "yum install -y unzip"; run_root yum install -y unzip
            fi ;;
        arch)   confirm_root_action "pacman -Sy --noconfirm unzip"; run_root pacman -Sy --noconfirm unzip ;;
        alpine) confirm_root_action "apk add --no-cache unzip";     run_root apk add --no-cache unzip ;;
        *) warn "unzip not found and cannot auto-install"; return 1 ;;
    esac
    have unzip
}

MCP_PATH=""
install_mcp_zipapp() {
    if [ -n "$MCP_PATH" ] && [ -x "$MCP_PATH" ]; then return 0; fi
    ensure_tag
    asset="tcl-lsp-mcp-server-${VER_NO_V}.pyz"
    url="$(asset_url "$asset")"
    log "downloading $asset"
    tmpfile="$WORKDIR/$asset"
    download "$url" "$tmpfile"
    verify_artefact "$asset" "$tmpfile"
    MCP_PATH="$PREFIX/tcl-lsp-mcp-server.pyz"
    write_target "$tmpfile" "$MCP_PATH"
    log "installed MCP server -> $MCP_PATH"
}

register_mcp_claude() {
    if ! have_claude_cli; then
        warn "claude CLI not on PATH — add the MCP server manually:"
        warn "  claude mcp add tcl-lsp -- $PYTHON $MCP_PATH"
        return
    fi
    claude mcp remove tcl-lsp >/dev/null 2>&1 || true
    if claude mcp add tcl-lsp -- "$PYTHON" "$MCP_PATH" >/dev/null 2>&1; then
        log "registered MCP server with Claude Code (tcl-lsp)"
    else
        warn "claude mcp add failed — register manually:"
        warn "  claude mcp add tcl-lsp -- $PYTHON $MCP_PATH"
    fi
}

register_mcp_codex() {
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
    ensure_unzip || { warn "unzip unavailable — skipping skills install"; return 1; }
    ensure_tag
    asset="tcl-lsp-claude-skills-${VER_NO_V}.zip"
    url="$(asset_url "$asset")"
    log "downloading $asset"
    tmpzip="$WORKDIR/$asset"
    download "$url" "$tmpzip"
    verify_artefact "$asset" "$tmpzip"
    extract_dir="$WORKDIR/claude-skills"
    mkdir -p "$extract_dir"
    unzip -q -o "$tmpzip" -d "$extract_dir"
    inner="$(find "$extract_dir" -mindepth 1 -maxdepth 1 -type d | head -n1)"
    if [ -z "$inner" ]; then
        warn "could not locate extracted skill payload in $extract_dir"
        return 1
    fi
    mkdir -p "$HOME/.claude"
    [ -d "$inner/skills" ]  && cp -R "$inner/skills"  "$HOME/.claude/"
    [ -d "$inner/prompts" ] && cp -R "$inner/prompts" "$HOME/.claude/"
    [ -f "$inner/tcl-ai.pyz" ] && cp "$inner/tcl-ai.pyz" "$HOME/.claude/tcl-ai.pyz"
    chmod 0755 "$HOME/.claude/tcl-ai.pyz" 2>/dev/null || true
    n="$(find "$HOME/.claude/skills" -mindepth 1 -maxdepth 1 -type d 2>/dev/null | wc -l | tr -d ' ')"
    log "installed Claude Code skills -> $HOME/.claude/skills/ ($n skills)"
}

install_ai_integrations() {
    detect_ai_clients
    if [ "$HAS_CLAUDE" != "1" ] && [ "$HAS_CODEX" != "1" ]; then return; fi
    if [ "${TCL_LSP_NO_MCP:-0}" = "1" ] && [ "${TCL_LSP_NO_SKILLS:-0}" = "1" ]; then
        return
    fi

    if [ "${TCL_LSP_NO_MCP:-0}" != "1" ] && ask_optout "Install the tcl-lsp MCP server for detected AI client(s)? [Y/n]"; then
        install_mcp_zipapp
        [ "$HAS_CLAUDE" = "1" ] && register_mcp_claude
        [ "$HAS_CODEX"  = "1" ] && register_mcp_codex
    fi

    if [ "$HAS_CLAUDE" = "1" ] && [ "${TCL_LSP_NO_SKILLS:-0}" != "1" ] \
       && ask_optout "Install Claude Code skills (irule-*, tcl-*, tk-*) into ~/.claude/? [Y/n]"; then
        install_claude_skills
    fi
}

# ---------------------------------------------------------------------
# main
# ---------------------------------------------------------------------

main() {
    init_workdir
    ensure_ui
    detect_os
    detect_shell
    ensure_curl

    if ! find_python; then
        install_python
    fi
    log "using Python: $PYTHON"

    choose_clis
    propose_update_install
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
        # Instruction text shown to the user; they paste this into their
        # shell where it expands.
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
