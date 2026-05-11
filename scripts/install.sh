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
#   TCL_LSP_NO_VERIFY            - 1 to skip SHA256SUMS verification
#   TCL_LSP_REQUIRE_VERIFY       - 1 to fail when SHA256SUMS is missing
#   TCL_LSP_REQUIRE_COSIGN       - 1 to fail when cosign signature is missing/invalid
#   TCL_LSP_ALLOW_INSECURE_WGET  - 1 to allow wget without --https-only (DANGEROUS)
#   TCL_LSP_ASSUME_YES      - 1 to answer "yes" non-interactively
#   TCL_LSP_ASSUME_NO       - 1 to answer "no" non-interactively
#   TCL_LSP_NO_TUI          - 1 to force text prompts even when whiptail/dialog is present
#   TCL_LSP_SUFFIX          - suffix for installed binaries (e.g. "-lsp"; default: empty)

set -eu
# Defence-in-depth: a poisoned IFS in the caller's env can split values
# in places we don't expect. Force IFS to a single newline so unquoted
# expansions only split on line boundaries — the POSIX default
# (<space><tab><newline>) is intentionally tighter here. Any loop that
# needs whitespace splitting saves and restores IFS locally.
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
    # Prefer curl when present — it can pin redirects to HTTPS via
    # --proto-redir, which some `wget` builds (notably BusyBox) cannot.
    # If neither is present we install curl via the package manager.
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

# ---------------------------------------------------------------------
# Optional dependency install — offered as a single batch with default-NO
# ---------------------------------------------------------------------

pkg_name_for() {
    # pkg_name_for OS CMD — print the OS-package name for a given binary.
    # Empty output means "no equivalent on this OS" (usually preinstalled
    # or distributed as part of something else).
    os="$1"; cmd="$2"
    case "$os:$cmd" in
        # whiptail (newt on RHEL/Fedora, libnewt on Arch, none on macOS)
        rhel:whiptail|fedora:whiptail) printf 'newt' ;;
        arch:whiptail)                 printf 'libnewt' ;;
        macos:whiptail)                printf '' ;;

        # OpenSSH client (ships scp alongside ssh)
        debian:ssh)                    printf 'openssh-client' ;;
        rhel:ssh|fedora:ssh)           printf 'openssh-clients' ;;
        arch:ssh)                      printf 'openssh' ;;
        alpine:ssh)                    printf 'openssh-client' ;;
        macos:ssh)                     printf '' ;;

        # tshark — packaged as wireshark-cli on RPM-style distros
        rhel:tshark|fedora:tshark)     printf 'wireshark-cli' ;;
        arch:tshark)                   printf 'wireshark-cli' ;;
        macos:tshark)                  printf 'wireshark' ;;

        # tclsh (system Tcl interpreter)
        macos:tclsh)                   printf 'tcl-tk' ;;
        debian:tclsh|alpine:tclsh|arch:tclsh|rhel:tclsh|fedora:tclsh)
                                       printf 'tcl' ;;

        # default — package name = command name
        *:*)                           printf '%s' "$cmd" ;;
    esac
}

install_pkg() {
    # install_pkg CMD — best-effort install of the OS package providing CMD.
    cmd="$1"
    pkg="$(pkg_name_for "$OS" "$cmd")"
    if [ -z "$pkg" ]; then
        warn "no $cmd package mapping for $OS — install manually"
        return 1
    fi
    case "$OS" in
        macos)
            if ! have brew; then
                warn "brew not found — install $pkg manually"
                return 1
            fi
            if brew install "$pkg" >/dev/null 2>&1; then
                log "installed $cmd via brew"
            else
                warn "brew install $pkg failed"
            fi
            ;;
        debian)
            confirm_root_action "apt-get install -y $pkg"
            if run_root apt-get install -y "$pkg" >/dev/null 2>&1; then
                log "installed $cmd via apt-get"
            else
                warn "apt install $pkg failed"
            fi
            ;;
        rhel|fedora)
            PM="yum"; have dnf && PM="dnf"
            confirm_root_action "$PM install -y $pkg"
            if run_root "$PM" install -y "$pkg" >/dev/null 2>&1; then
                log "installed $cmd via $PM"
            else
                warn "$PM install $pkg failed"
            fi
            ;;
        arch)
            confirm_root_action "pacman -Sy --noconfirm $pkg"
            if run_root pacman -Sy --noconfirm "$pkg" >/dev/null 2>&1; then
                log "installed $cmd via pacman"
            else
                warn "pacman install $pkg failed"
            fi
            ;;
        alpine)
            confirm_root_action "apk add --no-cache $pkg"
            if run_root apk add --no-cache "$pkg" >/dev/null 2>&1; then
                log "installed $cmd via apk"
            else
                warn "apk install $pkg failed"
            fi
            ;;
        *)
            warn "no package manager known for $OS — install $pkg manually" ;;
    esac
}

check_cli_runtime_dependencies() {
    # Survey runtime tools the installed `tcl` / `f5` CLIs shell out to.
    # The CLIs themselves are self-contained Python zipapps; these only
    # affect verbs that explicitly need an external binary.
    #
    # Prints a one-shot list of what's missing and why, then asks once
    # (default NO) whether to install the lot via the OS package manager.
    [ "${TCL_LSP_NO_DEPS:-0}" = "1" ] && return

    needs_tclsh=0      # tcl pkg / tcl venv / `tcl explore` against real tclsh
    needs_ssh=0        # f5 fetch (required for that verb's SSH transport)
    needs_sshpass=0    # f5 fetch password auth (optional fallback)
    needs_tshark=0     # f5 explain-flow --tshark, enrich-pcapng, pcap-remap

    case "$ONLY" in
        tcl|both)
            if ! have tclsh && ! have tclsh9.0 && ! have tclsh8.6 && ! have tclsh8.5; then
                needs_tclsh=1
            fi
            ;;
    esac
    case "$ONLY" in
        f5|both)
            have ssh     || needs_ssh=1
            have sshpass || needs_sshpass=1
            have tshark  || needs_tshark=1
            ;;
    esac

    total=$((needs_tclsh + needs_ssh + needs_sshpass + needs_tshark))
    [ "$total" = 0 ] && return

    log "$ONLY CLI runtime dependencies missing ($total):"
    [ "$needs_tclsh"   = 1 ] && log "  tclsh    — tcl pkg / tcl venv / tcl explore against the real interpreter"
    [ "$needs_ssh"     = 1 ] && log "  openssh  — f5 fetch over SSH (required for that verb)"
    [ "$needs_sshpass" = 1 ] && log "  sshpass  — f5 fetch with password auth (optional fallback to keys)"
    [ "$needs_tshark"  = 1 ] && log "  tshark   — f5 explain-flow / enrich-pcapng / pcap-remap libpcap support"

    if ! ask_default_no "Install missing CLI runtime dependencies via the package manager? [y/N]"; then
        log "skipping CLI runtime dependency install — features above will be unavailable"
        return
    fi

    [ "$needs_tclsh"   = 1 ] && install_pkg tclsh
    [ "$needs_ssh"     = 1 ] && install_pkg ssh
    [ "$needs_sshpass" = 1 ] && install_pkg sshpass
    [ "$needs_tshark"  = 1 ] && install_pkg tshark
}

WGET_HAS_HTTPS_ONLY=""
require_wget_https_only() {
    # Memoised probe. Aborts the install when wget lacks --https-only —
    # a follow-on http:// redirect from a MITM would otherwise compromise
    # both the artefact download and the SHA256SUMS verification (the
    # attacker could simply serve their own SUMS over plaintext).
    #
    # Escape hatch: TCL_LSP_ALLOW_INSECURE_WGET=1 explicitly opts in to a
    # wget without redirect-protocol pinning. Don't set this unless you
    # know your network path is trustworthy end-to-end.
    case "$WGET_HAS_HTTPS_ONLY" in
        1) return 0 ;;
        0) return 1 ;;
    esac
    if wget --help 2>&1 | grep -q -- --https-only; then
        WGET_HAS_HTTPS_ONLY=1
        return 0
    fi
    WGET_HAS_HTTPS_ONLY=0
    if [ "${TCL_LSP_ALLOW_INSECURE_WGET:-0}" = "1" ]; then
        warn "wget here does not support --https-only — HTTPS redirect pinning OFF"
        warn "(TCL_LSP_ALLOW_INSECURE_WGET=1 set, proceeding anyway)"
        return 1
    fi
    die "wget on this host does not support --https-only.
A MITM that redirected an HTTPS request to http:// could replace
the downloaded artefact AND the SHA256SUMS file in the same response.
Options:
  - install curl (\`apt-get install curl\` / \`brew install curl\` / etc.) and re-run
  - upgrade wget to >= 1.14
  - set TCL_LSP_ALLOW_INSECURE_WGET=1 to bypass (do NOT do this on an
    untrusted network)"
}

# ---------------------------------------------------------------------
# Downloads (HTTPS-only)
# ---------------------------------------------------------------------

download() {
    # download URL OUTPUT — pins TLS, refuses HTTP redirects.
    url="$1"; out="$2"
    log "fetching $(basename "$out")"
    if [ "$DOWNLOADER" = "wget" ]; then
        if require_wget_https_only; then
            wget --https-only -qO "$out" "$url"
        else
            # Reached only when TCL_LSP_ALLOW_INSECURE_WGET=1.
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
        if require_wget_https_only; then
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
    # REQUIRE_COSIGN implies REQUIRE_VERIFY — you can't verify a signature
    # on a SUMS file you couldn't download.
    if [ "${TCL_LSP_REQUIRE_VERIFY:-0}" = "1" ] || [ "${TCL_LSP_REQUIRE_COSIGN:-0}" = "1" ]; then
        die "SHA256SUMS not found at $(asset_url SHA256SUMS) — TCL_LSP_REQUIRE_VERIFY=1 or TCL_LSP_REQUIRE_COSIGN=1, refusing to proceed."
    fi
    warn "no SHA256SUMS file in release $RESOLVED_TAG — installing without integrity verification"
    warn "(set TCL_LSP_REQUIRE_VERIFY=1 to fail on missing SUMS, or TCL_LSP_NO_VERIFY=1 to silence this)"
    SUMS_STATE=absent
    return 1
}

verify_sums_signature() {
    # Verify the SUMS file against a cosign keyless signature when both
    # cosign and a SHA256SUMS.cosign.bundle are available.
    #
    # TCL_LSP_REQUIRE_COSIGN=1 promotes "missing cosign" or "missing
    # bundle" from a silent downgrade to a hard failure. This catches a
    # network adversary stripping the bundle to coerce a signature-
    # verified install down to hash-only.
    sums="$1"
    if ! have cosign; then
        if [ "${TCL_LSP_REQUIRE_COSIGN:-0}" = "1" ]; then
            die "cosign not installed but TCL_LSP_REQUIRE_COSIGN=1.
Install cosign (\`brew install cosign\` / \`apt-get install cosign\` / etc.) and retry."
        fi
        return 0
    fi
    bundle="$WORKDIR/SHA256SUMS.cosign.bundle"
    if ! download "$(asset_url SHA256SUMS.cosign.bundle)" "$bundle" 2>/dev/null; then
        if [ "${TCL_LSP_REQUIRE_COSIGN:-0}" = "1" ]; then
            die "SHA256SUMS.cosign.bundle not published for $RESOLVED_TAG
and TCL_LSP_REQUIRE_COSIGN=1. Refusing to proceed with hash-only verification.
Older releases that predate the publish-checksums job may need the bundle
backfilled — see scripts/backfill-sums.sh --sign."
        fi
        return 0
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
    # Match either `<hash> NAME` (GNU text mode) or `<hash> *NAME` (binary).
    # Assumes filenames contain no whitespace — true for every release
    # artefact we publish; would break for `foo bar.pyz`.
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
    # Two-tier identity check:
    #
    # Tier 1 — cheap fingerprint: Python shebang + ZIP local-file-header
    # signature in the first 2KB. Catches "is it a Python zipapp at all".
    #
    # Tier 2 — deep peek for tcl-lsp markers via whichever tool is
    # available:
    #   a) `unzip -l` / `unzip -p __main__.py` (fast)
    #   b) `python3 zipfile` (always present — Python is anyway
    #      required to run any of our zipapps, and the installer
    #      ensures Python 3.10+ before this function is ever called)
    #
    # A raw-grep fallback (looking for 'lsp/_build_info.py' as a literal
    # in the file) was considered but is too easy to spoof: an attacker
    # could embed the marker string anywhere in an arbitrary executable.
    # When both unzip AND Python are missing we instead refuse to
    # confirm the file as ours — the caller falls back to "treat as
    # unrelated", which surfaces the right prompts (overwrite y/N,
    # picker for fresh install).
    f="$1"
    [ -r "$f" ] || return 1

    first="$(head -c 200 "$f" 2>/dev/null)"
    case "$first" in
        '#!'*python*) : ;;
        *) return 1 ;;
    esac
    head -c 2048 "$f" 2>/dev/null | grep -aq 'PK' || return 1

    # Tier 2a — unzip
    if have unzip; then
        if unzip -l -- "$f" 2>/dev/null | grep -q 'lsp/_build_info.py'; then
            return 0
        fi
        if unzip -p -- "$f" __main__.py 2>/dev/null \
            | grep -qE 'explorer\.(tcl|f5|wasm)_cli|ai\.mcp\.tcl_mcp_server|lsp\._build_info|lsp\.server'; then
            return 0
        fi
        return 1
    fi

    # Tier 2b — Python zipfile
    if [ -n "${PYTHON:-}" ]; then
        if "$PYTHON" - "$f" >/dev/null 2>&1 <<'PY'
import sys, zipfile
try:
    with zipfile.ZipFile(sys.argv[1]) as z:
        if 'lsp/_build_info.py' in set(z.namelist()):
            sys.exit(0)
        try:
            main = z.read('__main__.py').decode('utf-8', errors='replace')
        except KeyError:
            sys.exit(1)
        for m in (
            'explorer.tcl_cli', 'explorer.f5_cli', 'explorer.wasm_cli',
            'ai.mcp.tcl_mcp_server', 'lsp._build_info', 'lsp.server',
        ):
            if m in main:
                sys.exit(0)
except (zipfile.BadZipFile, FileNotFoundError, OSError):
    pass
sys.exit(1)
PY
        then
            return 0
        fi
        return 1
    fi

    # Neither unzip nor Python — can't confirm. Treat as not-ours so
    # downstream logic surfaces the safer "overwrite anyway?" prompt.
    return 1
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

    found_dir=""
    case "$ONLY" in
        tcl)  propose_update_one tcl  || return ;;
        f5)   propose_update_one f5   || return ;;
        both) propose_update_one tcl  || return
              propose_update_one f5   || return ;;
        *)    return ;;
    esac

    if [ -n "$found_dir" ]; then
        set_prefix "$found_dir"
        PREFIX_EXPLICIT=1
        PREFIX_FROM_UPDATE=1
        log "updating in place: $PREFIX"
    fi
}

propose_update_one() {
    # propose_update_one NAME — invoked once per CLI from
    # propose_update_install. Returns non-zero when the caller should
    # abort the in-place-update flow entirely (user declined, or PATH
    # has a non-zipapp at this name).
    n="$1"
    path="$(find_on_path "$n")" || return 0
    if looks_like_our_zipapp "$path"; then
        found_dir="$(dirname "$path")"
        log "found existing $n at $path"
        if ! ask_optout "Update existing $n at $path (in place)? [Y/n]"; then
            log "leaving $path alone; will pick a fresh install location"
            return 1
        fi
        return 0
    fi
    warn "found '$n' at $path but it doesn't look like our zipapp"
    warn "(missing python shebang or ZIP signature) — picker will run as usual"
    return 1
}

# ---------------------------------------------------------------------
# Detect PATH conflicts and offer to rename our install
# ---------------------------------------------------------------------

# Suffix applied to our binaries (e.g. "-lsp"). Set by detect_conflicts
# when the user picks "rename" or via the TCL_LSP_SUFFIX env override.
INSTALL_SUFFIX="${TCL_LSP_SUFFIX:-}"

alias_in_rc() {
    # Cheap, best-effort grep for an alias/abbr that would shadow $1.
    # Catches the common forms; doesn't follow source/sourced rc fragments.
    n="$1"
    for rc in "$RC" "$HOME/.profile" "$HOME/.bashrc" "$HOME/.bash_profile" "$HOME/.zshrc" "${XDG_CONFIG_HOME:-$HOME/.config}/fish/config.fish"; do
        [ -f "$rc" ] || continue
        if grep -qE "^[[:space:]]*(alias[[:space:]]+|abbr([[:space:]]+--add)?[[:space:]]+)${n}[[:space:]=]" "$rc" 2>/dev/null; then
            printf '%s\n' "$rc"
            return 0
        fi
    done
    return 1
}

count_conflicts_for() {
    # Increments the parent's $conflicts when name $1 would be shadowed
    # by something earlier on PATH or by a shell alias. No subshell —
    # we share scope with detect_conflicts to keep the $conflicts
    # tally simple and avoid stdout-capture acrobatics.
    n="$1"
    if other="$(find_on_path "$n")" && [ "$other" != "$PREFIX/$n" ]; then
        if looks_like_our_zipapp "$other"; then
            warn "another tcl-lsp '$n' is already at $other (will shadow $PREFIX/$n)"
        else
            warn "an unrelated '$n' exists at $other (will shadow $PREFIX/$n)"
        fi
        conflicts=$((conflicts + 1))
    fi
    if rc_with_alias="$(alias_in_rc "$n")"; then
        warn "shell alias for '$n' in $rc_with_alias may shadow our install"
        conflicts=$((conflicts + 1))
    fi
}

detect_conflicts() {
    # For each CLI we're about to install, check whether something on
    # PATH (or an alias) will shadow it once we drop our binary in
    # $PREFIX. Offer the user three options: keep the name, rename to
    # <name>-lsp, or abort. Skipped during update-in-place (the existing
    # layout is already what the user wants).
    [ "${PREFIX_FROM_UPDATE:-0}" = "1" ] && return

    conflicts=0
    case "$ONLY" in
        tcl)  count_conflicts_for tcl ;;
        f5)   count_conflicts_for f5  ;;
        both) count_conflicts_for tcl; count_conflicts_for f5 ;;
        *)    return ;;
    esac
    [ "$conflicts" = 0 ] && return

    if [ -n "$INSTALL_SUFFIX" ]; then
        log "applying TCL_LSP_SUFFIX: binaries will install as <name>${INSTALL_SUFFIX}"
        return
    fi
    if [ ! -t 0 ] || [ ! -t 1 ]; then
        warn "(set TCL_LSP_SUFFIX=-lsp to install as <name>-lsp and avoid the shadow)"
        return
    fi

    case "$UI" in
        whiptail|dialog)
            sel="$(tui_menu "Resolve naming conflict" \
                "keep"   "Install with original name (may be shadowed)" \
                "rename" "Install as <name>-lsp instead" \
                "abort"  "Cancel install")" \
                || die "aborted: naming conflict not resolved"
            ;;
        *)
            printf '\nHow to proceed? [keep / rename / abort] (default: keep): '
            read -r sel || sel=keep
            : "${sel:=keep}"
            ;;
    esac
    case "$sel" in
        rename)
            INSTALL_SUFFIX="-lsp"
            log "binaries will install as <name>-lsp"
            ;;
        abort)
            die "aborted: naming conflict not resolved"
            ;;
        keep|*)
            log "keeping original names — be aware existing shadows remain in effect"
            ;;
    esac
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
    # Two safety prompts:
    #   - If DST exists but isn't one of our zipapps, prompt (default NO)
    #     before clobbering — refuses to silently overwrite an unrelated
    #     file that happens to share the name.
    #   - If the target directory isn't user-writable, prompt (default NO)
    #     before escalating to sudo.
    src="$1"; dst="$2"
    if [ -e "$dst" ] && ! looks_like_our_zipapp "$dst"; then
        warn "$dst already exists and is not a tcl-lsp zipapp"
        warn "(no Python shebang or ZIP signature in first 2KB)"
        if ! ask_default_no "Overwrite $dst anyway? [y/N]"; then
            die "aborted: existing $dst is unrelated and overwrite declined.
Re-run with TCL_LSP_PREFIX=/other/dir, or remove $dst first."
        fi
    fi
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
    # install_cli NAME (one of "tcl" or "f5"). INSTALL_SUFFIX may rename
    # the on-disk binary (e.g. "tcl" → "tcl-lsp") to avoid PATH conflicts.
    name="$1"
    final_name="${name}${INSTALL_SUFFIX}"
    ensure_tag
    asset="${name}-${VER_NO_V}.pyz"
    url="$(asset_url "$asset")"
    log "resolved $name -> $asset (tag $RESOLVED_TAG)"

    tmpfile="$WORKDIR/$asset"
    download "$url" "$tmpfile"
    verify_artefact "$asset" "$tmpfile"
    write_target "$tmpfile" "$PREFIX/$final_name"
    log "installed $name -> $PREFIX/$final_name"
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
    final_name="${name}${INSTALL_SUFFIX}"
    if [ "${TCL_LSP_NO_COMP:-0}" = "1" ]; then return; fi
    if [ -n "$INSTALL_SUFFIX" ]; then
        # The bundled completion script registers handlers for the
        # original name (`tcl`/`f5`). When we rename the binary the
        # script won't match — skip rather than install a dead handler.
        log "skipping $name completion ($final_name has no matching completion script)"
        return
    fi
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
    # Idempotent re-registration: capture the prior entry (when present)
    # so a failed `mcp add` after a successful `mcp remove` doesn't leave
    # the user worse off than they started.
    if ! have_claude_cli; then
        warn "claude CLI not on PATH — add the MCP server manually:"
        warn "  claude mcp add tcl-lsp -- $PYTHON $MCP_PATH"
        return
    fi
    prior=""
    if claude mcp list 2>/dev/null | awk '{print $1}' | grep -qx 'tcl-lsp'; then
        # The exact `mcp list` format isn't a stable contract; capture
        # the full record so we can echo it back as restore guidance.
        prior="$(claude mcp list 2>/dev/null | awk '$1=="tcl-lsp" {sub(/^tcl-lsp[[:space:]]+/,""); print; exit}')"
        claude mcp remove tcl-lsp >/dev/null 2>&1 || true
    fi
    if claude mcp add tcl-lsp -- "$PYTHON" "$MCP_PATH" >/dev/null 2>&1; then
        log "registered MCP server with Claude Code (tcl-lsp)"
        return
    fi
    warn "claude mcp add failed"
    if [ -n "$prior" ]; then
        warn "prior registration was: $prior"
        warn "restore it manually if needed"
    fi
    warn "or register fresh with: claude mcp add tcl-lsp -- $PYTHON $MCP_PATH"
}

register_mcp_codex() {
    cfg="$HOME/.codex/config.toml"
    mkdir -p "$HOME/.codex"
    touch "$cfg"
    # Allow leading whitespace — TOML accepts indented section headers
    # and a hand-edited config may use them.
    if grep -qE '^[[:space:]]*\[mcp_servers\.tcl_lsp\]' "$cfg" 2>/dev/null; then
        log "Codex already has [mcp_servers.tcl_lsp] in $cfg — leaving as-is"
        return
    fi
    cp "$cfg" "${cfg}.bak.$(date +%Y%m%d%H%M%S)" 2>/dev/null || true
    # Defensive escape — Unix paths can contain " or \ and the TOML
    # parser would mis-tokenise either. set_prefix already validates
    # $MCP_PATH; we still guard $PYTHON which comes from `command -v`.
    py_escaped="$(toml_basic_escape "$PYTHON")"
    mcp_escaped="$(toml_basic_escape "$MCP_PATH")"
    {
        printf '\n[mcp_servers.tcl_lsp]\n'
        printf 'command = "%s"\n' "$py_escaped"
        printf 'args = ["%s"]\n'  "$mcp_escaped"
    } >> "$cfg"
    log "registered MCP server with Codex in $cfg"
}

toml_basic_escape() {
    # Escape \ and " for a TOML basic string ("…"). Leaves printables
    # alone; control chars in paths are pathological enough we let TOML
    # reject them at load time.
    printf '%s' "$1" | sed 's/\\/\\\\/g; s/"/\\"/g'
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

    # Snapshot existing skills / prompts before we overwrite them. The
    # archive ships under canonical skill dirs (irule-*, tcl-*, tk-*) and
    # if the user has hand-edited any of those, the cp -R below replaces
    # them silently. A timestamped tarball makes the swap reversible.
    #
    # We only back up if at least one of the target trees has actual
    # content. Backups live next to the targets and rotate per timestamp.
    if [ -d "$HOME/.claude/skills" ] || [ -d "$HOME/.claude/prompts" ]; then
        bak_stamp="$(date +%Y%m%d%H%M%S)"
        bak_dir="$HOME/.claude/.tcl-lsp-bak-${bak_stamp}"
        mkdir -p "$bak_dir"
        [ -d "$HOME/.claude/skills"  ] && cp -R "$HOME/.claude/skills"  "$bak_dir/" 2>/dev/null
        [ -d "$HOME/.claude/prompts" ] && cp -R "$HOME/.claude/prompts" "$bak_dir/" 2>/dev/null
        [ -f "$HOME/.claude/tcl-ai.pyz" ] && cp "$HOME/.claude/tcl-ai.pyz" "$bak_dir/" 2>/dev/null
        # Drop the backup dir entirely if nothing was actually staged.
        if [ -z "$(ls -A "$bak_dir" 2>/dev/null)" ]; then
            rmdir "$bak_dir" 2>/dev/null
        else
            log "backed up prior ~/.claude/{skills,prompts,tcl-ai.pyz} → $bak_dir"
        fi
    fi

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
    detect_conflicts

    case "$ONLY" in
        tcl)  install_cli tcl ;;
        f5)   install_cli f5 ;;
        both) install_cli tcl; install_cli f5 ;;
        *) die "invalid TCL_LSP_ONLY: $ONLY (expected tcl|f5|both)" ;;
    esac

    check_cli_runtime_dependencies

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
    s="$INSTALL_SUFFIX"
    case "$ONLY" in
        tcl)  printf 'Verify:  %stcl%s --help%s\n' "$BOLD" "$s" "$RESET" ;;
        f5)   printf 'Verify:  %sf5%s --help%s\n'  "$BOLD" "$s" "$RESET" ;;
        both) printf 'Verify:  %stcl%s --help && f5%s --help%s\n' "$BOLD" "$s" "$s" "$RESET" ;;
    esac
}

main "$@"
